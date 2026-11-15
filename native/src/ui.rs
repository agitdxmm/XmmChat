//! 界面层。
//!
//! 全部由 GPU 直接绘制，没有 HTML/CSS/JS。
//! 背景是系统合成的云母，这里只画半透明的内容层。

use eframe::egui::{
    self, Align, Color32, Frame, Layout, Margin, RichText, Rounding, ScrollArea, Sense, Stroke,
    Vec2, ViewportCommand,
};

use crate::net::{Body, Evt, LinkState, Msg, Net};
use crate::proto::{self, DEFAULT_PORT};

pub struct State {
    net: Net,
    link: LinkState,
    detail: String,
    peer: String,
    msgs: Vec<Msg>,
    input: String,
    addr_input: String,
    my_name: String,
    addresses: Vec<String>,
    port: u16,
    /// 未连接时显示的小提示
    notice: Option<String>,
}

impl State {
    pub fn new(ctx: &egui::Context) -> Self {
        let port = DEFAULT_PORT;
        Self {
            net: Net::start(ctx.clone(), port),
            link: LinkState::Idle,
            detail: String::new(),
            peer: String::new(),
            msgs: Vec::new(),
            input: String::new(),
            addr_input: String::new(),
            my_name: proto::host_name(),
            addresses: proto::local_addresses(),
            port,
            notice: None,
        }
    }

    fn pump(&mut self) {
        while let Ok(evt) = self.net.evt_rx.try_recv() {
            match evt {
                Evt::Link(state, detail) => {
                    self.link = state;
                    self.detail = detail;
                }
                Evt::Peer(name) => {
                    self.peer = name;
                    self.msgs.push(Msg {
                        mid: proto::new_id(),
                        mine: false,
                        body: Body::Sys("对方已连接".into()),
                    });
                }
                Evt::Msg(m) => {
                    self.msgs.push(m);
                }
                Evt::Progress { mid, sent, total } => {
                    for m in self.msgs.iter_mut() {
                        if m.mid == mid {
                            if let Body::File {
                                sent: s, size, ..
                            } = &mut m.body
                            {
                                *s = sent;
                                *size = total;
                            }
                        }
                    }
                }
                Evt::Saved { mid, path } => {
                    for m in self.msgs.iter_mut() {
                        if m.mid == mid {
                            if let Body::File { done, saved, .. } = &mut m.body {
                                *done = true;
                                *saved = Some(path.clone());
                            }
                        }
                    }
                }
                Evt::Err(e) => {
                    self.notice = Some(e);
                }
            }
        }
    }
}

pub fn draw(ctx: &egui::Context, _frame: &mut eframe::Frame, st: &mut State) {
    st.pump();

    titlebar(ctx, st);
    sidebar(ctx, st);
    main_area(ctx, st);
}

// ---------------------------------------------------------------- 标题栏

fn titlebar(ctx: &egui::Context, st: &State) {
    egui::TopBottomPanel::top("titlebar")
        .exact_height(46.0)
        .frame(Frame::none().fill(Color32::TRANSPARENT))
        .show(ctx, |ui| {
            let rect = ui.max_rect();

            // 整条可拖拽
            let resp = ui.interact(rect, ui.id().with("drag"), Sense::click_and_drag());
            if resp.dragged() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);

                // 标记
                let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                ui.painter()
                    .circle_filled(r.center(), 11.0, crate::accent_soft(230));
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "咪",
                    egui::FontId::proportional(12.0),
                    Color32::BLACK,
                );

                ui.add_space(9.0);
                ui.label(RichText::new("咪的天").size(13.0).color(Color32::from_gray(235)));
                ui.label(
                    RichText::new("XMMCHAT · NATIVE")
                        .size(9.5)
                        .color(crate::dim_text()),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(10.0);
                    if win_button(ui, "✕", true) {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    if win_button(ui, "—", false) {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }

                    ui.add_space(8.0);
                    let (dot, text) = match st.link {
                        LinkState::Online => (Color32::from_rgb(74, 222, 128), "已连接"),
                        LinkState::Connecting => (Color32::from_rgb(251, 191, 36), "连接中"),
                        LinkState::Listening => (Color32::from_rgb(148, 163, 184), "等待对方"),
                        _ => (Color32::from_rgb(110, 110, 125), "未连接"),
                    };
                    ui.label(RichText::new(text).size(10.5).color(crate::dim_text()));
                    let (d, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                    ui.painter().circle_filled(d.center(), 3.5, dot);
                });
            });
        });
}

fn win_button(ui: &mut egui::Ui, label: &str, danger: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 26.0), Sense::click());
    let hovered = resp.hovered();
    if hovered {
        let c = if danger {
            Color32::from_rgb(220, 60, 60)
        } else {
            Color32::from_white_alpha(24)
        };
        ui.painter().rect_filled(rect, Rounding::same(7.0), c);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        if hovered {
            Color32::WHITE
        } else {
            Color32::from_gray(170)
        },
    );
    resp.clicked()
}

// ---------------------------------------------------------------- 侧栏

fn sidebar(ctx: &egui::Context, st: &State) {
    egui::SidePanel::left("side")
        .exact_width(268.0)
        .resizable(false)
        .frame(
            Frame::none()
                .fill(crate::card_fill(150))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            // 头像
            let (r, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
            ui.painter()
                .circle_filled(r.center(), 23.0, crate::accent_soft(210));
            let initial = st.my_name.chars().next().unwrap_or('喵');
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional(19.0),
                Color32::BLACK,
            );

            ui.add_space(10.0);
            ui.label(
                RichText::new(&st.my_name)
                    .size(14.0)
                    .color(Color32::from_gray(240)),
            );
            ui.label(
                RichText::new(format!("端口 {}", st.port))
                    .size(10.5)
                    .color(crate::dim_text()),
            );

            ui.add_space(18.0);
            ui.label(
                RichText::new("本机地址")
                    .size(10.0)
                    .color(crate::dim_text()),
            );
            ui.add_space(6.0);

            for addr in &st.addresses {
                let text = format!("{}:{}", addr, st.port);
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
                let bg = if resp.hovered() {
                    Color32::from_white_alpha(14)
                } else {
                    Color32::from_white_alpha(7)
                };
                ui.painter().rect_filled(rect, Rounding::same(7.0), bg);
                ui.painter().text(
                    rect.left_center() + Vec2::new(9.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &text,
                    egui::FontId::monospace(11.0),
                    Color32::from_gray(205),
                );
                if resp.clicked() {
                    ui.ctx().copy_text(text);
                }
                ui.add_space(4.0);
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(
                RichText::new("对方")
                    .size(10.0)
                    .color(crate::dim_text()),
            );
            ui.add_space(4.0);
            if st.peer.is_empty() {
                ui.label(
                    RichText::new("还没有人连接")
                        .size(11.5)
                        .color(Color32::from_gray(120)),
                );
            } else {
                ui.label(
                    RichText::new(&st.peer)
                        .size(12.5)
                        .color(Color32::from_gray(235)),
                );
            }
        });
}

// ---------------------------------------------------------------- 主区

fn main_area(ctx: &egui::Context, st: &mut State) {
    egui::CentralPanel::default()
        .frame(Frame::none().inner_margin(Margin::same(18)))
        .show(ctx, |ui| {
            if st.link == LinkState::Online || !st.msgs.is_empty() {
                chat_view(ui, st);
            } else {
                connect_view(ui, st);
            }
        });
}

fn connect_view(ui: &mut egui::Ui, st: &mut State) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);

        ui.label(
            RichText::new("填对方地址，直接连")
                .size(22.0)
                .color(Color32::from_gray(238)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("点对点，不经过任何服务器")
                .size(12.0)
                .color(crate::dim_text()),
        );

        ui.add_space(26.0);

        let width = 420.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 46.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, Rounding::same(12.0), crate::card_fill(120));
        ui.painter()
            .rect_stroke(rect, Rounding::same(12.0), crate::hairline());

        let mut child = ui.child_ui(
            rect.shrink2(Vec2::new(12.0, 0.0)),
            Layout::left_to_right(Align::Center),
            None,
        );
        child.add_sized(
            [width - 130.0, 24.0],
            egui::TextEdit::singleline(&mut st.addr_input)
                .hint_text("192.168.1.20  或  ::1  或  pc.local")
                .frame(false)
                .desired_width(width - 130.0),
        );
        if child.button("连接").clicked() && !st.addr_input.trim().is_empty() {
            st.net.dial(st.addr_input.clone());
        }

        ui.add_space(14.0);
        if let Some(n) = &st.notice {
            ui.label(RichText::new(n).size(11.0).color(Color32::from_rgb(248, 180, 120)));
        }

        ui.add_space(20.0);
        ui.label(
            RichText::new(format!("或者让对方连过来：{}:{}", st.addresses.first().cloned().unwrap_or_default(), st.port))
                .size(10.5)
                .color(crate::dim_text()),
        );
    });
}

fn chat_view(ui: &mut egui::Ui, st: &mut State) {
    // 消息区
    let footer_h = 62.0;
    let avail = ui.available_height() - footer_h;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(avail)
        .show(ui, |ui| {
            for m in &st.msgs {
                bubble(ui, m);
                ui.add_space(8.0);
            }
        });

    ui.add_space(6.0);

    // 输入区
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 46.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, Rounding::same(12.0), crate::card_fill(140));
    ui.painter()
        .rect_stroke(rect, Rounding::same(12.0), crate::hairline());

    let mut child = ui.child_ui(
        rect.shrink2(Vec2::new(12.0, 0.0)),
        Layout::left_to_right(Align::Center),
        None,
    );
    let send_w = 62.0;
    let resp = child.add_sized(
        [rect.width() - send_w - 24.0, 24.0],
        egui::TextEdit::singleline(&mut st.input)
            .hint_text("说点什么，回车发送")
            .frame(false)
            .desired_width(rect.width() - send_w - 24.0),
    );

    let entered =
        resp.lost_focus() && child.input(|i| i.key_pressed(egui::Key::Enter));

    let clicked = child.button("发送").clicked();
    if (entered || clicked) && !st.input.trim().is_empty() {
        let text = st.input.trim().to_string();
        st.input.clear();
        if st.net.send_text(text.clone()) {
            st.msgs.push(Msg {
                mid: proto::new_id(),
                mine: true,
                body: Body::Text(text),
            });
        } else {
            st.notice = Some("还没连上，发不出去".into());
        }
        resp.request_focus();
    }
}

fn bubble(ui: &mut egui::Ui, m: &Msg) {
    let max_w = (ui.available_width() * 0.68).max(220.0);
    let text = m.body.summary();

    let galley = ui.painter().layout(
        text,
        egui::FontId::proportional(12.5),
        if m.mine {
            Color32::from_gray(20)
        } else {
            Color32::from_gray(232)
        },
        max_w - 28.0,
    );

    let pad = Vec2::new(14.0, 10.0);
    let size = Vec2::new(galley.size().x + pad.x * 2.0, galley.size().y + pad.y * 2.0);

    let full_w = ui.available_width();
    let align_right = m.mine;

    ui.horizontal(|ui| {
        if align_right {
            ui.add_space((full_w - size.x - 4.0).max(0.0));
        } else {
            ui.add_space(4.0);
        }

        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

        let (fill, stroke) = if m.mine {
            (crate::accent_soft(210), Stroke::new(1.0, Color32::from_white_alpha(40)))
        } else {
            (
                Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                Stroke::new(1.0, Color32::from_white_alpha(26)),
            )
        };

        ui.painter().rect_filled(rect, Rounding::same(14.0), fill);
        ui.painter().rect_stroke(rect, Rounding::same(14.0), stroke);

        ui.painter().galley(
            rect.min + pad,
            galley,
            if m.mine {
                Color32::from_gray(20)
            } else {
                Color32::from_gray(232)
            },
        );
    });
}
