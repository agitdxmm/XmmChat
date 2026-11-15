//! 本机信息：主机名、平台、局域网地址。用于「把这段地址发给对方」。

use serde::Serialize;

#[derive(Serialize)]
pub struct HostInfo {
    hostname: String,
    platform: String,
    arch: String,
    lan_ips: Vec<String>,
    port: u16,
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into())
}

#[tauri::command]
pub fn host_info() -> HostInfo {
    let mut lan_ips = Vec::new();
    if let Ok(ifaces) = local_ip_address::list_afinet_netifas() {
        for (_, ip) in ifaces {
            match ip {
                std::net::IpAddr::V4(v4) => {
                    if !v4.is_loopback() && !v4.is_link_local() {
                        lan_ips.push(v4.to_string());
                    }
                }
                std::net::IpAddr::V6(v6) => {
                    // 只保留全局单播，链路本地 fe80:: 对方多半连不上
                    if !v6.is_loopback() && !v6.is_unicast_link_local() {
                        lan_ips.push(format!("[{v6}]"));
                    }
                }
            }
        }
    }
    lan_ips.sort();
    lan_ips.dedup();

    HostInfo {
        hostname: hostname(),
        platform: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        lan_ips,
        port: crate::net::PORT,
    }
}
