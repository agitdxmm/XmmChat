//! 网络层：一个 tokio runtime + 一条 WebSocket 会话。
//!
//! 行为与 Web 版一致：始终监听 19779，同时允许主动拨号。
//! 会话建立后把写端注册到 `Shared.out`，界面发消息时直接投递过去。

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message, WebSocketStream,
};

use crate::proto::{self, Envelope, CHUNK};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    Idle,
    Listening,
    Connecting,
    Online,
    Closed,
}

impl LinkState {
    pub fn text(self) -> &'static str {
        match self {
            LinkState::Idle => "未连接",
            LinkState::Listening => "等待对方",
            LinkState::Connecting => "连接中",
            LinkState::Online => "已连接",
            LinkState::Closed => "已断开",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Msg {
    pub mid: String,
    pub mine: bool,
    pub body: Body,
}

#[derive(Debug, Clone)]
pub enum Body {
    Text(String),
    File {
        name: String,
        size: u64,
        sent: u64,
        done: bool,
        saved: Option<PathBuf>,
    },
    Sys(String),
}

impl Body {
    pub fn summary(&self) -> String {
        match self {
            Body::Text(t) => {
                let n = t.chars().count();
                if n > 42 {
                    format!("{}…", t.chars().take(42).collect::<String>())
                } else {
                    t.clone()
                }
            }
            Body::File { name, .. } => format!("📎 {name}"),
            Body::Sys(t) => t.clone(),
        }
    }
}

/// 网络 → 界面
#[derive(Debug)]
pub enum Evt {
    Link(LinkState, String),
    Peer(String),
    Msg(Msg),
    Progress {
        mid: String,
        sent: u64,
        total: u64,
    },
    Saved {
        mid: String,
        path: PathBuf,
    },
    Err(String),
}

/// 写端任务收到的任务
enum Out {
    Text(String),
    File(PathBuf, String),
}

// ---------------------------------------------------------------- 共享状态

struct Shared {
    evt_tx: std_mpsc::Sender<Evt>,
    ctx: egui::Context,
    /// 会话建立后填充；断开时清空
    out: Mutex<Option<tokio_mpsc::UnboundedSender<Out>>>,
}

impl Shared {
    fn emit(&self, e: Evt) {
        let _ = self.evt_tx.send(e);
        self.ctx.request_repaint();
    }

    fn attach(&self, tx: tokio_mpsc::UnboundedSender<Out>) {
        if let Ok(mut guard) = self.out.lock() {
            *guard = Some(tx);
        }
    }

    fn detach(&self) {
        if let Ok(mut guard) = self.out.lock() {
            *guard = None;
        }
    }

    fn submit(&self, job: Out) -> bool {
        let guard = match self.out.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        match guard.as_ref() {
            Some(tx) => tx.send(job).is_ok(),
            None => false,
        }
    }
}

// ---------------------------------------------------------------- 对外接口

pub struct Net {
    shared: Arc<Shared>,
    cmd_tx: tokio_mpsc::UnboundedSender<Cmd>,
    pub evt_rx: std_mpsc::Receiver<Evt>,
}

impl Net {
    pub fn start(ctx: egui::Context, port: u16) -> Self {
        let (cmd_tx, mut cmd_rx) = tokio_mpsc::unbounded_channel::<Cmd>();
        let (evt_tx, evt_rx) = std_mpsc::channel::<Evt>();

        let shared = Arc::new(Shared {
            evt_tx,
            ctx: ctx.clone(),
            out: Mutex::new(None),
        });

        let s = shared.clone();
        std::thread::Builder::new()
            .name("xmm-net".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                else {
                    return;
                };

                rt.block_on(async move {
                    let sh = s.clone();

                    // --- 监听：IPv4 与 IPv6 都要绑 ---
                    let mut bound: Vec<(&str, TcpListener)> = Vec::new();
                    if let Ok(l) = TcpListener::bind(("0.0.0.0", port)).await {
                        bound.push(("0.0.0.0", l));
                    }
                    if let Ok(l) = TcpListener::bind(("::", port)).await {
                        bound.push(("[::]", l));
                    }

                    if bound.is_empty() {
                        sh.emit(Evt::Err(format!(
                            "{port} 端口不可用，只能主动连接对方"
                        )));
                    } else {
                        let names: Vec<&str> = bound.iter().map(|(n, _)| *n).collect();
                        sh.emit(Evt::Link(
                            LinkState::Listening,
                            format!("{}:{port}", names.join(" + ")),
                        ));
                        for (_, listener) in bound {
                            let sh2 = sh.clone();
                            tokio::spawn(async move { accept_loop(listener, sh2).await });
                        }
                    }

                    // --- 命令 ---
                    while let Some(cmd) = cmd_rx.recv().await {
                        let sh4 = sh.clone();
                        match cmd {
                            Cmd::Dial(raw) => {
                                tokio::spawn(async move {
                                    let url = match proto::dial_url(&raw) {
                                        Ok(u) => u,
                                        Err(e) => {
                                            sh4.emit(Evt::Err(e));
                                            return;
                                        }
                                    };
                                    sh4.emit(Evt::Link(LinkState::Connecting, url.clone()));
                                    match connect_async(&url).await {
                                        Ok((ws, _)) => {
                                            sh4.emit(Evt::Link(LinkState::Online, url));
                                            session(ws, sh4).await;
                                        }
                                        Err(e) => {
                                            sh4.emit(Evt::Link(
                                                LinkState::Closed,
                                                format!("连不上：{e}"),
                                            ));
                                        }
                                    }
                                });
                            }
                            // 这两个由界面直接调 submit()，这里兜底
                            Cmd::SendText(t) => {
                                sh4.submit(Out::Text(t));
                            }
                            Cmd::SendFile(p) => {
                                let name = file_name_of(&p);
                                sh4.submit(Out::File(p, name));
                            }
                        }
                    }
                });
            })
            .ok();

        Self {
            shared,
            cmd_tx,
            evt_rx,
        }
    }

    pub fn dial(&self, addr: String) {
        let _ = self.cmd_tx.send(Cmd::Dial(addr));
    }

    /// 发一条文字。返回 false 表示当前没连上。
    pub fn send_text(&self, text: String) -> bool {
        self.shared.submit(Out::Text(text))
    }

    /// 发一个文件。返回 false 表示当前没连上。
    pub fn send_file(&self, path: PathBuf) -> bool {
        let name = file_name_of(&path);
        self.shared.submit(Out::File(path, name))
    }

    pub fn is_online(&self) -> bool {
        self.shared
            .out
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }
}

fn file_name_of(p: &PathBuf) -> String {
    p.file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into())
}

/// 界面 → 网络（拨号单独走一条通道，避免和写端混在一起）
#[derive(Debug)]
pub enum Cmd {
    Dial(String),
    SendText(String),
    SendFile(PathBuf),
}

// ---------------------------------------------------------------- 会话

async fn accept_loop(listener: TcpListener, sh: Arc<Shared>) {
    loop {
        let Ok((stream, addr)) = listener.accept().await else {
            continue;
        };
        let sh2 = sh.clone();
        tokio::spawn(async move {
            if let Ok(ws) = accept_async(stream).await {
                sh2.emit(Evt::Link(LinkState::Online, addr.to_string()));
                session(ws, sh2).await;
            }
        });
    }
}

async fn session<S>(ws: WebSocketStream<S>, sh: Arc<Shared>)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();

    // 先打招呼 —— 这之后 sink 就交给写任务了
    let hello = proto::hello(&whoami(), "windows");
    let _ = sink.send(Message::Text(hello.to_json().into())).await;

    // 注册写端
    let (out_tx, mut out_rx) = tokio_mpsc::unbounded_channel::<Out>();
    sh.attach(out_tx);

    // 写任务
    let writer = tokio::spawn(async move {
        while let Some(job) = out_rx.recv().await {
            match job {
                Out::Text(t) => {
                    let e = proto::chat_text(&t);
                    if sink.send(Message::Text(e.to_json().into())).await.is_err() {
                        break;
                    }
                }
                Out::File(path, name) => {
                    if send_file_inner(&mut sink, &path, &name).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut incoming: Option<(String, String, u64, u64, tokio::fs::File)> = None;

    while let Some(Ok(frame)) = stream.next().await {
        let Message::Text(text) = frame else { continue };
        let Some(env) = Envelope::parse(&text) else { continue };

        match env.kind.as_str() {
            "hello" => {
                let name = env
                    .data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("对方")
                    .to_string();
                sh.emit(Evt::Peer(name));
            }
            "chat" => {
                let text = env
                    .data
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                sh.emit(Evt::Msg(Msg {
                    mid: env.mid,
                    mine: false,
                    body: Body::Text(text),
                }));
            }
            "file-meta" => {
                let name = env
                    .data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("file")
                    .to_string();
                let size = env.data.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                let dir = received_dir();
                let _ = tokio::fs::create_dir_all(&dir).await;
                if let Ok(f) = tokio::fs::File::create(dir.join(&name)).await {
                    incoming = Some((env.mid.clone(), name.clone(), size, 0, f));
                    sh.emit(Evt::Msg(Msg {
                        mid: env.mid,
                        mine: false,
                        body: Body::File {
                            name,
                            size,
                            sent: 0,
                            done: false,
                            saved: None,
                        },
                    }));
                }
            }
            "file-chunk" => {
                let Some((mid, name, total, got, file)) = incoming.as_mut() else {
                    continue;
                };
                let b64 = env.data.get("data").and_then(|v| v.as_str()).unwrap_or("");
                let Ok(bytes) = B64.decode(b64) else { continue };
                if file.write_all(&bytes).await.is_err() {
                    continue;
                }
                *got += bytes.len() as u64;
                sh.emit(Evt::Progress {
                    mid: mid.clone(),
                    sent: *got,
                    total: *total,
                });

                if env
                    .data
                    .get("last")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    let _ = file.flush().await;
                    sh.emit(Evt::Saved {
                        mid: mid.clone(),
                        path: received_dir().join(name.clone()),
                    });
                    incoming = None;
                }
            }
            _ => {}
        }
    }

    writer.abort();
    sh.detach();
    sh.emit(Evt::Link(LinkState::Closed, "对方已断开".into()));
}

async fn send_file_inner<S>(
    sink: &mut futures_util::stream::SplitSink<WebSocketStream<S>, Message>,
    path: &PathBuf,
    name: &str,
) -> Result<(), ()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut f = tokio::fs::File::open(path).await.map_err(|_| ())?;
    let size = f.metadata().await.map(|m| m.len()).unwrap_or(0);

    let meta = proto::file_meta(name, size, "application/octet-stream");
    sink.send(Message::Text(meta.to_json().into()))
        .await
        .map_err(|_| ())?;

    let mut buf = vec![0u8; CHUNK];
    let mut seq = 0u64;
    let mid = proto::new_id();

    loop {
        let n = f.read(&mut buf).await.map_err(|_| ())?;
        if n == 0 {
            break;
        }
        let last = n < CHUNK;
        let b64 = B64.encode(&buf[..n]);
        let e = proto::file_chunk(&mid, seq, &b64, last);
        sink.send(Message::Text(e.to_json().into()))
            .await
            .map_err(|_| ())?;
        seq += 1;
        tokio::time::sleep(Duration::from_millis(1)).await;
        if last {
            break;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------- 工具

fn whoami() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "小喵咪".into())
}

fn received_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Downloads").join("咪的天"))
        .unwrap_or_else(|_| PathBuf::from("."))
}
