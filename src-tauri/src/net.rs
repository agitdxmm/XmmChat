//! 传输层：WebSocket，端口 19779。
//!
//! 同一份代码既能当服务端（accept）也能当客户端（connect），
//! 所以 Windows 与 Android 共用完全相同的信道实现。
//! 业务协议（JSON 信封）由前端解析，Rust 只搬字节。

use std::sync::Arc;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message, WebSocketStream};

pub const PORT: u16 = 19779;

/// 当前链路状态。串行化在一把锁里，避免两条连接互相踩。
#[derive(Default)]
pub struct Hub {
    inner: Mutex<Link>,
}

#[derive(Default)]
struct Link {
    tx: Option<mpsc::UnboundedSender<Message>>,
    peer: String,
    role: String,
}

pub fn new_hub() -> Arc<Hub> {
    Arc::new(Hub::default())
}

#[derive(Clone, Serialize)]
struct LinkEvent {
    state: String,
    peer: String,
    role: String,
    msg: String,
}

#[derive(Clone, Serialize)]
struct DataEvent {
    text: String,
}

#[derive(Clone, Serialize)]
struct BinEvent {
    b64: String,
}

fn emit_link(app: &AppHandle, state: &str, peer: &str, role: &str, msg: &str) {
    let _ = app.emit(
        "xmm://link",
        LinkEvent {
            state: state.into(),
            peer: peer.into(),
            role: role.into(),
            msg: msg.into(),
        },
    );
}

/// 拿到 Hub 的强引用（`manage` 存的就是 Arc<Hub>）。
fn hub_of(app: &AppHandle) -> Arc<Hub> {
    let st = app.state::<Arc<Hub>>();
    Arc::clone(&st)
}

/// 服务端：同时监听 IPv4 与 IPv6。
///
/// `bind(0.0.0.0)` 拿到的永远是**纯 IPv4** socket —— 即使系统 `bindv6only=0`，
/// 内核也不会让它接管 v6 流量。所以只绑它，界面里那些 `2408:...` 地址
/// 对方看得见、却连不进来。
///
/// 双栈能力只来自 `bind([::])`：
/// - Windows 默认 `IPV6_V6ONLY=1` → v4/v6 是两个独立 socket，都能绑成功，各管各的；
/// - Linux/Android 默认 `IPV6_V6ONLY=0` → `[::]` 本身就能收 v4（v4-mapped 地址），
///   它已经把端口占住，随后 `0.0.0.0` 会 EADDRINUSE，跳过即可。
///
/// 两种平台最终都能覆盖 v4 + v6，区别只是背后是 1 个 socket 还是 2 个。
///
/// **顺序很重要，必须先绑 `[::]`**：
/// - Linux 上 `[::]` 是双栈，会同时占住 v4 端口。若先绑了 `0.0.0.0`，
///   再绑 `[::]` 就会 EADDRINUSE —— 结果只剩 v4，v6 又漏了。
/// - Windows 上两者互不干扰，先后无所谓，但统一的顺序更好维护。
pub async fn serve(app: AppHandle, port: u16) -> Result<(), String> {
    let mut bound: Vec<(&'static str, TcpListener)> = Vec::new();
    let mut errs: Vec<String> = Vec::new();

    // 先 v6（Linux 上是双栈，会顺手覆盖 v4）
    match TcpListener::bind(("::", port)).await {
        Ok(l) => bound.push(("[::]", l)),
        Err(e) => errs.push(format!("IPv6 {e}")),
    }

    // 再 v4；如果上面已经双栈接管，这里会 EADDRINUSE，跳过即可
    match TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => bound.push(("0.0.0.0", l)),
        Err(e) => errs.push(format!("IPv4 {e}")),
    }

    if bound.is_empty() {
        return Err(format!("端口 {port} 监听失败：{}", errs.join("；")));
    }

    let labels: Vec<&str> = bound.iter().map(|(l, _)| *l).collect();
    emit_link(
        &app,
        "listening",
        &format!("{}:{port}", labels.join(" + ")),
        "host",
        "",
    );

    for (_label, listener) in bound {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let (stream, addr) = match listener.accept().await {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let app3 = app2.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = run_server_conn(app3.clone(), stream, addr.to_string()).await {
                        emit_link(&app3, "error", &addr.to_string(), "host", &e);
                    }
                });
            }
        });
    }

    // accept 循环都在后台任务里，这里挂住不让 serve 返回
    std::future::pending::<()>().await;
    Ok(())
}

async fn run_server_conn(app: AppHandle, stream: TcpStream, peer: String) -> Result<(), String> {
    let hub = hub_of(&app);
    {
        let l = hub.inner.lock().await;
        if l.tx.is_some() {
            return Err("已有连接，本次握手被拒绝".into());
        }
    }
    let ws = accept_async(stream)
        .await
        .map_err(|e| format!("握手失败：{e}"))?;
    attach(app, ws, peer, "host").await
}

/// 客户端：连过去。
#[tauri::command]
pub async fn net_dial(app: AppHandle, addr: String) -> Result<String, String> {
    let url = normalize_url(&addr)?;

    let hub = hub_of(&app);
    {
        let l = hub.inner.lock().await;
        if l.tx.is_some() {
            return Err("已经在通信中了，先挂断".into());
        }
    }

    emit_link(&app, "connecting", &url, "guest", "");
    let (ws, _) = connect_async(url.as_str())
        .await
        .map_err(|e| format!("连不上 {url} —— {e}"))?;

    let peer = url.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = attach(app2.clone(), ws, peer.clone(), "guest").await {
            emit_link(&app2, "error", &peer, "guest", &e);
        }
    });
    Ok(url)
}

/// 把一条已经握手的 WebSocket 挂到 Hub 上，双向搬运。
async fn attach<S>(app: AppHandle, ws: WebSocketStream<S>, peer: String, role: &str) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut source) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let hub = hub_of(&app);
    {
        let mut l = hub.inner.lock().await;
        l.tx = Some(tx);
        l.peer = peer.clone();
        l.role = role.to_string();
    }
    emit_link(&app, "online", &peer, role, "");

    // 出站任务
    let outbound = tauri::async_runtime::spawn(async move {
        while let Some(m) = rx.recv().await {
            let closing = matches!(m, Message::Close(_));
            if sink.send(m).await.is_err() {
                break;
            }
            if closing {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // 入站循环
    let app_in = app.clone();
    while let Some(item) = source.next().await {
        match item {
            Ok(Message::Text(t)) => {
                let _ = app_in.emit("xmm://data", DataEvent { text: t });
            }
            Ok(Message::Binary(b)) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&b);
                let _ = app_in.emit("xmm://bin", BinEvent { b64 });
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    outbound.abort();
    {
        let mut l = hub.inner.lock().await;
        l.tx = None;
        l.peer.clear();
    }
    emit_link(&app, "closed", &peer, role, "");
    Ok(())
}

/// 发文本帧（JSON 信封）。
#[tauri::command]
pub async fn net_send(app: AppHandle, text: String) -> Result<(), String> {
    let hub = hub_of(&app);
    let l = hub.inner.lock().await;
    let tx = l.tx.as_ref().ok_or("尚未建立连接")?;
    tx.send(Message::Text(text))
        .map_err(|_| "发送失败：链路已断开".to_string())
}

/// 发二进制帧（base64 传入，解码后原样发出）。大文件/语音走这里。
#[tauri::command]
pub async fn net_send_bin(app: AppHandle, b64: String) -> Result<(), String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("二进制解码失败：{e}"))?;
    let hub = hub_of(&app);
    let l = hub.inner.lock().await;
    let tx = l.tx.as_ref().ok_or("尚未建立连接")?;
    tx.send(Message::Binary(bytes))
        .map_err(|_| "发送失败：链路已断开".to_string())
}

/// 主动挂断。
#[tauri::command]
pub async fn net_hangup(app: AppHandle) -> Result<(), String> {
    let hub = hub_of(&app);
    let l = hub.inner.lock().await;
    if let Some(tx) = l.tx.as_ref() {
        let _ = tx.send(Message::Close(None));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct LinkState {
    online: bool,
    peer: String,
    role: String,
}

#[tauri::command]
pub async fn net_state(app: AppHandle) -> Result<LinkState, String> {
    let hub = hub_of(&app);
    let l = hub.inner.lock().await;
    Ok(LinkState {
        online: l.tx.is_some(),
        peer: l.peer.clone(),
        role: l.role.clone(),
    })
}

/// 地址归一化：允许填 IPv4 / IPv6 / 域名 / 局域网名，允许自带端口。
fn normalize_url(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("地址不能为空".into());
    }
    if s.starts_with("ws://") || s.starts_with("wss://") {
        return Ok(s.to_string());
    }

    let v6_bracket = s.starts_with('[');
    let colons = s.matches(':').count();

    // 已经写了端口
    let has_port = (v6_bracket && s.contains("]:")) || (!v6_bracket && colons == 1);
    if has_port {
        return Ok(format!("ws://{s}"));
    }

    // 裸 IPv6 字面量（::1 / fe80::...）需要方括号
    if colons >= 2 {
        return Ok(format!("ws://[{s}]:{PORT}"));
    }

    Ok(format!("ws://{s}:{PORT}"))
}

#[cfg(test)]
mod tests {
    use super::normalize_url;

    #[test]
    fn normalizes_addresses() {
        assert_eq!(normalize_url("192.168.1.9").unwrap(), "ws://192.168.1.9:19779");
        assert_eq!(normalize_url("pc.local").unwrap(), "ws://pc.local:19779");
        assert_eq!(normalize_url("192.168.1.9:9000").unwrap(), "ws://192.168.1.9:9000");
        assert_eq!(normalize_url("::1").unwrap(), "ws://[::1]:19779");
        assert_eq!(normalize_url("[::1]:9000").unwrap(), "ws://[::1]:9000");
        assert_eq!(normalize_url(" ws://a.b ").unwrap(), "ws://a.b");
        assert!(normalize_url("   ").is_err());
    }
}
