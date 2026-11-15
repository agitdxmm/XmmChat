//! 咪的天 XmmChat — Rust 内核
//!
//! 设计要点：
//!   * 传输层用 tokio-tungstenite，**同一份代码既是服务端也是客户端**，
//!     所以 Windows 和 Android 共用同一套信道实现，不需要平台分支。
//!   * 协议本身（JSON 信封）放在前端，Rust 只负责搬字节 + 提供原生能力。
//!     这样安卓端不需要重复实现一遍聊天逻辑。
//!   * 远程能力（执行指令 / 抓屏）**必须**在对端显式授予后才会响应，
//!     授予状态由前端持有并通过 `shell_run` 的 `granted` 参数传入。

mod cap;
mod net;
mod shellx;
mod sysinfo;

use tauri::Manager;

/// 高 DPI 小屏上，配置里的窗口尺寸可能超过监视器工作区。
/// 按实际可用区域收一下，然后把窗口居中。
///
/// 注意：`current_monitor()` / `outer_size()` 这类 getter 内部是
/// 「把闭包投递到事件循环 + 同步等结果」。在 `setup` 回调里直接调会永久阻塞
/// （事件循环还没开始跑），表现为「进程活着但一个窗口都没有」。
/// 所以这里必须放到独立线程、等事件循环起来之后再调。
///
/// 只在桌面平台编译 —— 安卓没有 `center()`，也不需要收边。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn fit_window(win: &tauri::WebviewWindow) {
    let (Ok(Some(mon)), Ok(outer), Ok(inner)) =
        (win.current_monitor(), win.outer_size(), win.inner_size())
    else {
        return;
    };

    let sf = mon.scale_factor();
    let area = mon.work_area().size.to_logical::<f64>(sf);
    let cur = inner.to_logical::<f64>(sf);
    let deco_w = outer.width.saturating_sub(inner.width) as f64 / sf;
    let deco_h = outer.height.saturating_sub(inner.height) as f64 / sf;

    let w = cur.width.min(area.width - deco_w - 12.0).max(640.0);
    let h = cur.height.min(area.height - deco_h - 12.0).max(480.0);

    if (w - cur.width).abs() > 1.0 || (h - cur.height).abs() > 1.0 {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        let _ = win.center();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(net::new_hub())
        .invoke_handler(tauri::generate_handler![
            net::net_dial,
            net::net_send,
            net::net_send_bin,
            net::net_hangup,
            net::net_state,
            shellx::shell_run,
            shellx::shell_probe,
            cap::screen_grab,
            sysinfo::host_info,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // 自动监听 19779：对端填了本机地址就能连进来。
            // 端口被占用时只是静默失败，不影响主动外连。
            tauri::async_runtime::spawn(async move {
                let _ = net::serve(handle, 19779).await;
            });

            // 收边放到独立线程，等事件循环跑起来再动手（仅桌面）。
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let h2 = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    if let Some(win) = h2.get_webview_window("main") {
                        fit_window(&win);
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("咪的天 启动失败");
}
