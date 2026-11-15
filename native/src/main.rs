#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! 咪的天 · 原生版
//!
//! 纯 Rust + GPU 直接绘制，没有 WebView、没有 JS 运行时。
//! 背景用 Windows 自带的合成器做云母/亚克力，不占 CPU。

use eframe::egui;
use egui::{Color32, Rounding, Stroke, Vec2};

mod net;
mod proto;
mod ui;

const APP_TITLE: &str = "咪的天";

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size([1160.0, 760.0])
            .with_min_inner_size([760.0, 540.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new(cc)))
        }),
    )
}

/// egui 自带字体没有中文，得从系统里借一份。
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhl.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];

    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}

pub struct App {
    pub state: ui::State,
    /// 是否已经给窗口挂上系统材质
    backdrop_done: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();

        // 整体透明，让系统合成的云母透出来
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::from_rgba_unmultiplied(24, 24, 30, 200);
        visuals.window_rounding = Rounding::same(14.0);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 14));
        visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(139, 124, 246, 70);

        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.set_pixels_per_point(1.25);

        Self {
            state: ui::State::new(&cc.egui_ctx),
            backdrop_done: false,
        }
    }
}

impl eframe::App for App {
    /// 每帧清成全透明，避免上一帧残影叠在亚克力上。
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.backdrop_done {
            self.backdrop_done = true;
            apply_backdrop(frame);
        }

        ui::draw(ctx, frame, &mut self.state);
    }
}

// ---------------------------------------------------------------- 系统材质

/// Windows 11 用 Mica，Win10 用亚克力。
/// 这两个都由 DWM 在 GPU 上合成，不是我们自己画的模糊 —— 所以几乎不耗 CPU。
fn apply_backdrop(frame: &eframe::Frame) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

        let Some(title) = to_wide(APP_TITLE) else { return };
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if hwnd == 0 {
            return;
        }

        // 2 = Mica, 3 = Acrylic, 4 = Tabbed
        let preference: u32 = 2;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &preference as *const u32 as *const _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    #[cfg(not(windows))]
    {
        let _ = frame;
    }
}

#[cfg(windows)]
fn to_wide(s: &str) -> Option<Vec<u16>> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    Some(v)
}

/// 给一个圆角矩形的填充色，界面里到处用。
pub fn card_fill(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(28, 28, 36, alpha)
}

pub fn hairline() -> Stroke {
    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18))
}

pub const ACCENT: Color32 = Color32::from_rgb(167, 139, 250);

pub fn accent_soft(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(167, 139, 250, alpha)
}

pub fn dim_text() -> Color32 {
    Color32::from_rgb(150, 148, 165)
}

pub fn body_size() -> f32 {
    13.0
}

pub fn small_size() -> f32 {
    11.0
}

pub fn pad(v: f32) -> Vec2 {
    Vec2::splat(v)
}
