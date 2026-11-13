//! Bộ widget riêng (SPEC §5): Panel, Button, IconButton, Tab, Tooltip, Bar, NumberLabel, Toggle, Slider, Slot.
//! Mọi khoảng cách / màu / cỡ chữ lấy từ `tokens.rs`; nền là 9-slice từ atlas phóng hệ số nguyên.

use super::actions::Action;
use super::tokens::*;
use super::vdom::{IconSpec, Kind, TextSpec, VNode};
use crate::i18n::Lang;
use bevy::prelude::*;
use cm_core::Rarity;

/// Ngữ cảnh dựng UI.
#[derive(Clone, Copy)]
pub struct Cx {
    pub lang: Lang,
    /// hệ số chữ (Chữ lớn × HUD)
    pub ts: f32,
    /// hệ số phóng art (9-slice, icon), nguyên
    pub k: f32,
    /// true: đang vẽ trên nền sáng (khung bảng kem, nút, ô) ⇒ chữ tối, không bóng
    pub light: bool,
}

impl Cx {
    pub fn t(&self, key: &str) -> &'static str {
        crate::i18n::t(self.lang, key)
    }
    pub fn tf(&self, key: &str, args: &[&str]) -> String {
        crate::i18n::tf(self.lang, key, args)
    }
    pub fn fs(&self, f: Fs) -> f32 {
        (f.px() * self.ts).round()
    }
    /// Ngữ cảnh chữ trên nền sáng.
    pub fn lit(&self) -> Cx {
        Cx {
            light: true,
            ..*self
        }
    }
    pub fn ink(&self, ink: Ink) -> Color {
        if self.light {
            ink.on_light()
        } else {
            ink.color()
        }
    }
    pub fn with_k(&self, k: f32) -> Cx {
        Cx { k, ..*self }
    }
}

// ── bố cục ──

pub fn row(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(gap),
        ..default()
    }
}
pub fn col(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(gap),
        ..default()
    }
}
pub fn hbox(gap: f32) -> VNode {
    VNode::boxed(row(gap))
}
pub fn vbox(gap: f32) -> VNode {
    VNode::boxed(col(gap))
}
pub fn spacer() -> VNode {
    VNode::boxed(Node {
        flex_grow: 1.0,
        ..default()
    })
}
pub fn absolute(x: f32, y: f32, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(w),
        height: px(h),
        ..default()
    }
}

// ── chữ ──

pub fn text_full(cx: &Cx, s: impl Into<String>, size: Fs, ink: Ink, bold: bool) -> VNode {
    VNode::new(
        Kind::Text(TextSpec {
            text: s.into(),
            size: cx.fs(size),
            color: cx.ink(ink),
            bold,
            justify: Justify::Left,
            wrap: false,
            shadow: !cx.light,
        }),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    )
}
pub fn text(cx: &Cx, s: impl Into<String>, size: Fs, ink: Ink) -> VNode {
    text_full(cx, s, size, ink, false)
}
pub fn title(cx: &Cx, s: impl Into<String>) -> VNode {
    text_full(cx, s, Fs::Title, Ink::Text, true)
}
/// Tiêu đề mục nhỏ in hoa.
pub fn header(cx: &Cx, s: impl Into<String>) -> VNode {
    text_full(cx, s, Fs::Small, Ink::Accent, true)
}
/// Đoạn văn tự xuống dòng trong bề rộng `w`.
pub fn para(cx: &Cx, s: impl Into<String>, size: Fs, ink: Ink, w: f32) -> VNode {
    let mut v = text(cx, s, size, ink);
    if let Kind::Text(t) = &mut v.p.kind {
        t.wrap = true;
    }
    v.p.node.max_width = px(w);
    v
}

// ── ảnh ──

pub fn icon(cx: &Cx, name: impl Into<String>) -> VNode {
    icon_k(name, cx.k)
}
pub fn icon_k(name: impl Into<String>, k: f32) -> VNode {
    VNode::new(
        Kind::Icon(IconSpec {
            name: name.into(),
            k,
            tint: Color::WHITE,
            flip: false,
        }),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    )
}
pub fn icon_tint(name: impl Into<String>, k: f32, tint: Color) -> VNode {
    VNode::new(
        Kind::Icon(IconSpec {
            name: name.into(),
            k,
            tint,
            flip: false,
        }),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    )
}

// ── khung ──

/// Panel: khung 9-slice `ui_panel`.
pub fn panel(cx: &Cx, node: Node) -> VNode {
    VNode::boxed(Node {
        padding: UiRect::all(px(S3)),
        ..node
    })
    .skin("ui_panel", cx.k)
    .solid()
}
/// Nền lót bán trong suốt cho chữ trên dải.
pub fn plate(cx: &Cx, node: Node) -> VNode {
    VNode::boxed(Node {
        padding: UiRect::axes(px(S2), px(S1)),
        ..node
    })
    .skin("ui_plate", cx.k)
}
pub fn tooltip_box(cx: &Cx, lines: Vec<VNode>) -> VNode {
    VNode::boxed(Node {
        flex_direction: FlexDirection::Column,
        padding: UiRect::axes(px(S2), px(S1 + 2.0)),
        row_gap: px(2.0),
        max_width: px(TOOLTIP_MAX_W),
        ..default()
    })
    .skin("ui_tooltip", cx.k.min(2.0))
    .kids(lines)
}

// ── nút ──

fn btn_node(h: f32) -> Node {
    Node {
        height: px(h),
        padding: UiRect::axes(px(S2), px(0.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: px(S1),
        flex_shrink: 0.0,
        ..default()
    }
}

/// Button: nhãn chữ (có thể kèm icon), trạng thái hover/press/disabled.
pub fn button(cx: &Cx, label: impl Into<String>, action: Action, enabled: bool) -> VNode {
    button_with(cx, vec![], label, action, enabled, BTN_H)
}
pub fn button_small(cx: &Cx, label: impl Into<String>, action: Action, enabled: bool) -> VNode {
    button_with(cx, vec![], label, action, enabled, BTN_H_SMALL)
}
pub fn button_with(
    cx: &Cx,
    lead: Vec<VNode>,
    label: impl Into<String>,
    action: Action,
    enabled: bool,
    h: f32,
) -> VNode {
    let ink = if enabled { Ink::Text } else { Ink::Muted };
    let size = if h <= BTN_H_SMALL {
        Fs::Small
    } else {
        Fs::Body
    };
    VNode::boxed(btn_node(h))
        .skin(if enabled { "ui_btn" } else { "ui_btn_disabled" }, cx.k)
        .action(action)
        .disabled(!enabled)
        .kids(lead)
        .kid(text(&cx.lit(), label, size, ink))
}
/// IconButton vuông.
pub fn icon_button(cx: &Cx, name: &str, action: Action, size: f32) -> VNode {
    VNode::boxed(Node {
        width: px(size),
        height: px(size),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    })
    .skin("ui_btn", cx.k)
    .action(action)
    .kid(icon(cx, name))
}
/// Nút giữ-để-xác-nhận: có thanh tiến độ phủ dưới nhãn.
pub fn hold_button(
    cx: &Cx,
    label: impl Into<String>,
    action: Action,
    enabled: bool,
    secs: f32,
    w: f32,
) -> VNode {
    let ink = if enabled { Ink::Text } else { Ink::Muted };
    let fill = VNode::new(
        Kind::HoldFill,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            top: px(0.0),
            bottom: px(0.0),
            width: Val::Percent(0.0),
            ..default()
        },
    )
    .skin("ui_bar_cd", cx.k);
    let mut b = VNode::boxed(Node {
        width: px(w),
        ..btn_node(BTN_H)
    })
    .skin(if enabled { "ui_btn" } else { "ui_btn_disabled" }, cx.k)
    .kid(fill)
    .kid(text(&cx.lit(), label, Fs::Body, ink))
    .disabled(!enabled);
    if enabled {
        b = b.action(action).hold(secs);
    }
    b
}

/// Tab: icon + chữ ngắn.
pub fn tab(cx: &Cx, icon_name: &str, label: &str, active: bool, action: Action) -> VNode {
    VNode::boxed(Node {
        height: px(TAB_H - S1),
        padding: UiRect::axes(px(S2 - 2.0), px(0.0)),
        align_items: AlignItems::Center,
        column_gap: px(S1),
        ..default()
    })
    .skin(if active { "ui_tab_active" } else { "ui_tab" }, cx.k)
    .action(action)
    .kid(icon_k(icon_name, 1.0))
    .kid(text(
        &cx.lit(),
        label,
        Fs::Small,
        if active { Ink::Text } else { Ink::Muted },
    ))
}

/// Toggle (Bật/Tắt).
pub fn toggle(cx: &Cx, on: bool, action: Action) -> VNode {
    let label = if on { cx.t("on") } else { cx.t("off") };
    VNode::boxed(Node {
        width: px(56.0),
        ..btn_node(BTN_H_SMALL)
    })
    .skin(if on { "ui_tab_active" } else { "ui_btn" }, cx.k)
    .action(action)
    .kid(icon_tint(
        "ic_check",
        cx.k / 2.0,
        if on {
            Ink::Good.on_light()
        } else {
            Color::NONE
        },
    ))
    .kid(text(
        &cx.lit(),
        label,
        Fs::Small,
        if on { Ink::Good } else { Ink::Muted },
    ))
}

/// Nhóm lựa chọn (segmented): mỗi ô một hành động.
pub fn segmented(cx: &Cx, items: Vec<(String, bool, Action)>) -> VNode {
    hbox(2.0).kids(items.into_iter().map(|(label, active, a)| {
        VNode::boxed(Node {
            min_width: px(24.0),
            padding: UiRect::axes(px(6.0), px(0.0)),
            ..btn_node(BTN_H_SMALL)
        })
        .skin(if active { "ui_tab_active" } else { "ui_btn" }, cx.k)
        .action(a)
        .kid(text(
            &cx.lit(),
            label,
            Fs::Small,
            if active { Ink::Text } else { Ink::Muted },
        ))
    }))
}

/// Slider bậc: − giá trị% + (âm lượng, độ mờ); bước `step`, trong [min, max].
pub fn slider(
    cx: &Cx,
    value: u32,
    min: u32,
    max: u32,
    step: u32,
    make: impl Fn(u32) -> Action,
) -> VNode {
    let num = VNode::boxed(Node {
        width: px(32.0),
        justify_content: JustifyContent::Center,
        ..default()
    })
    .kid(text(cx, format!("{value}%"), Fs::Small, Ink::Text));
    hbox(2.0)
        .kid(button_small(
            cx,
            "−",
            make(value.saturating_sub(step).max(min)),
            value > min,
        ))
        .kid(num)
        .kid(button_small(
            cx,
            "+",
            make((value + step).min(max)),
            value < max,
        ))
}

/// Bar: nền `ui_bar_bg` + phần lấp `ui_bar_{kind}`.
pub fn bar(cx: &Cx, frac: f32, fill: &str, w: f32, h: f32) -> VNode {
    let f = frac.clamp(0.0, 1.0);
    let k = (h / 4.0).floor().max(1.0).min(cx.k);
    let mut fill_node = VNode::boxed(Node {
        width: px((w * f).round()),
        height: px(h),
        ..default()
    });
    if f > 0.0 {
        fill_node = fill_node.skin(fill, k);
    }
    VNode::boxed(Node {
        width: px(w),
        height: px(h),
        ..default()
    })
    .skin("ui_bar_bg", k)
    .kid(fill_node)
}

/// NumberLabel: icon + số căn phải (bề rộng tối thiểu để số không “nhảy”).
pub fn number_label(cx: &Cx, icon_name: &str, value: &str, ink: Ink, min_w: f32) -> VNode {
    hbox(S1).kid(icon(cx, icon_name)).kid(
        VNode::boxed(Node {
            min_width: px(min_w),
            justify_content: JustifyContent::FlexEnd,
            ..default()
        })
        .kid(text(cx, value, Fs::Body, ink)),
    )
}

/// Slot ô đồ 32 px: khung theo độ hiếm + icon + nhãn nâng cấp + ký hiệu.
pub fn slot(
    cx: &Cx,
    rarity: Option<Rarity>,
    icon_name: Option<&str>,
    badge: Option<String>,
    selected: bool,
) -> VNode {
    let frame = rarity.map(rarity_slot).unwrap_or("ui_slot");
    let mut v = VNode::boxed(Node {
        width: px(SLOT),
        height: px(SLOT),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    })
    .skin(frame, cx.k / 2.0);
    if selected {
        v = v.skin_tint(Color::srgb(1.0, 0.85, 0.45));
    }
    if let Some(n) = icon_name {
        v = v.kid(icon_k(n, 1.0));
    }
    if let Some(r) = rarity {
        // ký hiệu độ hiếm góc trên-trái (không chỉ dựa vào màu)
        v = v.kid(
            VNode::boxed(Node {
                position_type: PositionType::Absolute,
                left: px(2.0),
                top: px(-2.0),
                ..default()
            })
            .kid(text(&cx.lit(), rarity_glyph(r), Fs::Small, rarity_ink(r))),
        );
    }
    if let Some(b) = badge {
        v = v.kid(
            VNode::boxed(Node {
                position_type: PositionType::Absolute,
                right: px(2.0),
                bottom: px(-3.0),
                ..default()
            })
            .kid(text(&cx.lit(), b, Fs::Small, Ink::Text)),
        );
    }
    v
}

/// Chấm đỏ báo hiệu (nút túi).
pub fn red_dot(size: f32) -> VNode {
    VNode::boxed(Node {
        position_type: PositionType::Absolute,
        right: px(-2.0),
        top: px(-2.0),
        width: px(size),
        height: px(size),
        border: UiRect::all(px(1.0)),
        border_radius: BorderRadius::MAX,
        ..default()
    })
    .bg(Color::srgb_u8(0xd1, 0x4b, 0x4b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx() -> Cx {
        Cx {
            lang: Lang::Vi,
            ts: 1.0,
            k: 2.0,
            light: false,
        }
    }

    #[test]
    fn disabled_button_has_no_live_action_skin() {
        let c = cx();
        let b = button(&c, "Lên cấp", Action::ClosePanel, false);
        assert!(b.p.disabled);
        assert_eq!(b.p.skin.as_ref().unwrap().name, "ui_btn_disabled");
        let b = button(&c, "Lên cấp", Action::ClosePanel, true);
        assert_eq!(b.p.skin.as_ref().unwrap().name, "ui_btn");
        assert_eq!(b.p.action, Some(Action::ClosePanel));
    }

    #[test]
    fn bar_fill_width_is_integer() {
        let b = bar(&cx(), 0.333, "ui_bar_hp", 100.0, 8.0);
        assert_eq!(b.kids[0].p.node.width, Val::Px(33.0));
        let b = bar(&cx(), 0.0, "ui_bar_hp", 100.0, 8.0);
        assert!(b.kids[0].p.skin.is_none());
    }

    #[test]
    fn text_sizes_follow_tokens() {
        let big = Cx { ts: 1.5, ..cx() };
        assert_eq!(big.fs(Fs::Body), 20.0);
        assert_eq!(cx().fs(Fs::Small), 11.0);
    }
}
