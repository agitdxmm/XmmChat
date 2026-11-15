//! 线协议。
//!
//! 和 Web 版（`src/core/protocol.ts`）完全一致：一条 JSON 一个 WebSocket 文本帧，
//! 所以原生版和 Web 版可以互连 —— 不是两套东西。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTO_VERSION: u8 = 1;
pub const DEFAULT_PORT: u16 = 19779;

/// 信封。所有消息都套这一层。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u8,
    /// 消息类型，如 `chat` / `file-chunk` / `hello`
    #[serde(rename = "type")]
    pub kind: String,
    /// 消息 id，用于回执和去重
    pub mid: String,
    /// 毫秒时间戳
    pub ts: u64,
    pub data: Value,
}

impl Envelope {
    pub fn new(kind: &str, data: Value) -> Self {
        Self {
            v: PROTO_VERSION,
            kind: kind.to_string(),
            mid: new_id(),
            ts: now_ms(),
            data,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn parse(text: &str) -> Option<Self> {
        serde_json::from_str::<Envelope>(text).ok()
    }
}

/// 时间工具：毫秒时间戳
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 短 id：时间 + 计数器，够用且不会撞。
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:04x}", now_ms(), n & 0xffff)
}

// ---------------------------------------------------------------- 常用消息

pub fn hello(name: &str, platform: &str) -> Envelope {
    Envelope::new(
        "hello",
        json!({ "name": name, "platform": platform, "ver": env!("CARGO_PKG_VERSION") }),
    )
}

pub fn chat_text(text: &str) -> Envelope {
    Envelope::new("chat", json!({ "kind": "text", "text": text }))
}

pub fn typing(on: bool) -> Envelope {
    Envelope::new("typing", json!({ "on": on }))
}

pub fn file_meta(name: &str, size: u64, mime: &str) -> Envelope {
    Envelope::new(
        "file-meta",
        json!({ "name": name, "size": size, "mime": mime }),
    )
}

pub fn file_chunk(mid: &str, seq: u64, b64: &str, last: bool) -> Envelope {
    let mut e = Envelope::new(
        "file-chunk",
        json!({ "seq": seq, "data": b64, "last": last }),
    );
    e.mid = mid.to_string();
    e
}

/// 文件分块大小。64 KB 是 base64 后约 85 KB 一帧，够快也不会把内存顶爆。
pub const CHUNK: usize = 64 * 1024;

// ---------------------------------------------------------------- 地址解析

/// 把用户输入变成可拨号的 URL。
///
/// 支持：`1.2.3.4` / `::1` / `host.local` / `1.2.3.4:19779` / `ws://...`
pub fn dial_url(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("地址是空的".into());
    }

    if s.starts_with("ws://") || s.starts_with("wss://") {
        return Ok(with_port_if_missing(s));
    }

    // 裸 IPv6，形如 ::1 或 fe80::1
    if s.contains(':') && s.matches(':').count() >= 2 && !s.contains('[') {
        return Ok(format!("ws://[{}]:{}", s, DEFAULT_PORT));
    }

    Ok(with_port_if_missing(&format!("ws://{}", s)))
}

fn with_port_if_missing(url: &str) -> String {
    let after_scheme = url.splitn(2, "://").nth(1).unwrap_or("");

    // [v6]:port 已经有端口
    if let Some(end) = after_scheme.rfind(']') {
        let rest = &after_scheme[end + 1..];
        if rest.starts_with(':') && rest.len() > 1 {
            return url.to_string();
        }
        return format!("{}:{}", url, DEFAULT_PORT);
    }

    if after_scheme.matches(':').count() == 1 {
        return url.to_string();
    }

    format!("{}:{}", url, DEFAULT_PORT)
}

/// 本机所有可被对方拨到的地址。
pub fn local_addresses() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(ips) = local_ip_address::list_afinet_netifas() {
        for (_, ip) in ips {
            match ip {
                std::net::IpAddr::V4(v4) => {
                    if !v4.is_loopback() {
                        out.push(v4.to_string());
                    }
                }
                std::net::IpAddr::V6(v6) => {
                    // 只留全局地址，fe80 链接本地对方拨不到
                    if !v6.is_loopback() && !v6.is_unicast_link_local() {
                        out.push(format!("[{}]", v6));
                    }
                }
            }
        }
    }
    if out.is_empty() {
        out.push("127.0.0.1".into());
    }
    out
}

pub fn host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "未知设备".into())
}
