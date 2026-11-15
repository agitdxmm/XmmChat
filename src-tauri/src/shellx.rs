//! 远程指令执行。
//!
//! 权限模型（重要）：这里**不做**任何权限判断，`granted` 由前端在收到对端
//! 的「协助授权」之后传入。也就是说，指令能被执行的唯一前提是
//! **对方主动邀请并同意**，而不是「我知道你地址」。
//!
//! 平台差异：
//!   * Windows —— 直接调 `cmd /C`，无需额外权限。
//!   * Android —— 应用本身没有 shell 身份，必须借助 Shizuku 或无障碍服务；
//!     这部分在 Kotlin 侧实现，Rust 返回 unsupported 让前端去走插件。

use serde::Serialize;

#[derive(Serialize)]
pub struct ShellProbe {
    /// "cmd" | "none"
    backend: String,
    /// 人类可读的能力说明，前端直接显示
    detail: String,
}

#[derive(Serialize)]
pub struct ShellResult {
    code: i32,
    stdout: String,
    stderr: String,
    /// 超时被强杀
    timed_out: bool,
}

/// 探测本机执行能力。
#[tauri::command]
pub fn shell_probe() -> ShellProbe {
    #[cfg(windows)]
    {
        ShellProbe {
            backend: "cmd".into(),
            detail: "cmd.exe · 本地执行".into(),
        }
    }
    #[cfg(not(windows))]
    {
        ShellProbe {
            backend: "none".into(),
            detail: "没有Shell权限的终端".into(),
        }
    }
}

/// 执行一条指令。
#[tauri::command]
pub async fn shell_run(cmd: String, granted: bool, timeout_ms: Option<u64>) -> Result<ShellResult, String> {
    if !granted {
        return Err("对方尚未授予协助权限".into());
    }
    let budget = timeout_ms.unwrap_or(20_000).clamp(1_000, 120_000);
    tauri::async_runtime::spawn_blocking(move || run_blocking(cmd, budget))
        .await
        .map_err(|e| format!("执行线程异常：{e}"))?
}

#[cfg(windows)]
fn run_blocking(cmd: String, timeout_ms: u64) -> Result<ShellResult, String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // chcp 65001 把控制台切到 UTF-8，否则中文输出是 GBK，前端会乱码
    let full = format!("chcp 65001>nul & {cmd}");
    let mut child = Command::new("cmd")
        .args(["/D", "/S", "/C", &full])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 cmd：{e}"))?;

    let started = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() > Duration::from_millis(timeout_ms) {
                    let _ = child.kill();
                    timed_out = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(60));
            }
            Err(e) => return Err(format!("等待进程失败：{e}")),
        }
    }

    let status = child.wait().map_err(|e| format!("回收进程失败：{e}"))?;
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut s) = child.stdout.take() {
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf);
        out = String::from_utf8_lossy(&buf).to_string();
    }
    if let Some(mut s) = child.stderr.take() {
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf);
        err = String::from_utf8_lossy(&buf).to_string();
    }

    Ok(ShellResult {
        code: status.code().unwrap_or(-1),
        stdout: out,
        stderr: err,
        timed_out,
    })
}

#[cfg(not(windows))]
fn run_blocking(_cmd: String, _timeout_ms: u64) -> Result<ShellResult, String> {
    Err("没有Shell权限的终端".into())
}
