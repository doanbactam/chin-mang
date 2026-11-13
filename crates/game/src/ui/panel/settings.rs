//! Tab Cài đặt (SPEC §7.5). Mục chỉ có nghĩa trên một nền tảng (Ghost mode, tự khởi động, màn hình,
//! luôn trên cùng, thoát) chỉ hiện khi nền tảng hỗ trợ (`Caps`).

use super::{INNER_W, section};
use crate::i18n::Lang;
use crate::platform::Caps;
use crate::settings::{
    Edge, FPS_OPTIONS, OPACITY_STEP, SCALE_OPTIONS, Settings, VOLUME_STEP, WIDTH_OPTIONS,
};
use crate::ui::actions::{Action, SetOp, VolKind};
use crate::ui::kit::*;
use crate::ui::tokens::*;
use crate::ui::vdom::VNode;
use bevy::prelude::*;
use cm_core::{Command, Game, Rarity};

pub const RESET_HOLD: f32 = 3.0;
pub const IMPORT_HOLD: f32 = 2.0;
const COL_W: f32 = (INNER_W - S2) / 2.0;

/// Hàng 2 dòng: nhãn trên, điều khiển dưới.
fn stacked(cx: &Cx, key: &str, control: VNode) -> VNode {
    vbox(2.0)
        .kid(text(cx, cx.t(key), Fs::Small, Ink::Muted))
        .kid(control)
}
/// Hàng 1 dòng: nhãn trái, điều khiển phải.
fn inline(cx: &Cx, key: &str, control: VNode) -> VNode {
    let mut r = hbox(S1)
        .kid(text(cx, cx.t(key), Fs::Small, Ink::Muted))
        .kid(spacer())
        .kid(control);
    r.p.node.width = px(COL_W);
    r
}
pub fn build(cx: &Cx, game: &Game, s: &Settings, caps: &Caps) -> VNode {
    let seg = |items: Vec<(String, bool, Action)>| segmented(cx, items);
    // ── cột trái: hiển thị + tự rã ──
    let mut left = vbox(2.0).kid(section(cx, "hdr_display"));
    left = left.kid(stacked(
        cx,
        "set_lang",
        seg(vec![
            (
                cx.t("lang_vi").into(),
                s.lang == Lang::Vi,
                Action::Set(SetOp::Lang(Lang::Vi)),
            ),
            (
                cx.t("lang_en").into(),
                s.lang == Lang::En,
                Action::Set(SetOp::Lang(Lang::En)),
            ),
        ]),
    ));
    left = left.kid(stacked(
        cx,
        "set_scale",
        seg(SCALE_OPTIONS
            .iter()
            .map(|&v| {
                let label = if v == 0 {
                    cx.t("scale_auto").to_string()
                } else {
                    format!("{v}×")
                };
                (label, s.scale == v, Action::Set(SetOp::Scale(v)))
            })
            .collect()),
    ));
    left = left.kid(stacked(
        cx,
        "set_width",
        seg(WIDTH_OPTIONS
            .iter()
            .map(|&v| {
                (
                    format!("{v}%"),
                    s.width_pct == v,
                    Action::Set(SetOp::Width(v)),
                )
            })
            .collect()),
    ));
    left = left.kid(inline(
        cx,
        "set_edge",
        seg(vec![
            (
                cx.t("edge_bottom").into(),
                s.edge == Edge::Bottom,
                Action::Set(SetOp::Edge(Edge::Bottom)),
            ),
            (
                cx.t("edge_top").into(),
                s.edge == Edge::Top,
                Action::Set(SetOp::Edge(Edge::Top)),
            ),
        ]),
    ));
    if caps.monitors.len() > 1 {
        left = left.kid(stacked(
            cx,
            "set_monitor",
            seg((0..caps.monitors.len().min(4))
                .map(|i| {
                    (
                        cx.tf("monitor_n", &[&(i + 1).to_string()]),
                        s.monitor == i,
                        Action::Set(SetOp::Monitor(i)),
                    )
                })
                .collect()),
        ));
    }
    left = left.kid(stacked(
        cx,
        "set_text",
        seg(["text_normal", "text_large", "text_xlarge"]
            .iter()
            .enumerate()
            .map(|(i, k)| {
                (
                    cx.t(k).to_string(),
                    s.text_size == i as u32,
                    Action::Set(SetOp::Text(i as u32)),
                )
            })
            .collect()),
    ));
    left = left.kid(inline(
        cx,
        "set_opacity",
        slider(cx, s.opacity, 40, 100, OPACITY_STEP, |v| {
            Action::Set(SetOp::Opacity(v))
        }),
    ));
    let auto = game.auto_salvage();
    left = left.kid(stacked(
        cx,
        "set_autosalvage",
        seg(vec![
            (
                cx.t("autosalv_off").into(),
                auto.is_none(),
                Action::Cmd(Command::SetAutoSalvage { max: None }),
            ),
            (
                cx.t("autosalv_common").into(),
                auto == Some(Rarity::Common),
                Action::Cmd(Command::SetAutoSalvage {
                    max: Some(Rarity::Common),
                }),
            ),
            (
                cx.t("autosalv_rare").into(),
                auto == Some(Rarity::Rare),
                Action::Cmd(Command::SetAutoSalvage {
                    max: Some(Rarity::Rare),
                }),
            ),
        ]),
    ));
    left.p.node.width = px(COL_W);
    left.p.node.flex_shrink = 0.0;

    // ── cột phải: âm thanh, hệ thống, dữ liệu ──
    let mut right = vbox(2.0).kid(section(cx, "hdr_sound"));
    for (key, kind, v) in [
        ("set_vol_master", VolKind::Master, s.vol_master),
        ("set_vol_music", VolKind::Music, s.vol_music),
        ("set_vol_sfx", VolKind::Sfx, s.vol_sfx),
    ] {
        right = right.kid(inline(
            cx,
            key,
            slider(cx, v, 0, 100, VOLUME_STEP, move |x| {
                Action::Set(SetOp::Vol(kind, x))
            }),
        ));
    }
    right = right.kid(section(cx, "hdr_system"));
    right = right.kid(inline(
        cx,
        "set_fps",
        seg(FPS_OPTIONS
            .iter()
            .map(|&v| (v.to_string(), s.fps_limit == v, Action::Set(SetOp::Fps(v))))
            .collect()),
    ));
    if caps.always_on_top {
        right = right.kid(inline(
            cx,
            "set_on_top",
            toggle(
                cx,
                s.always_on_top,
                Action::Set(SetOp::OnTop(!s.always_on_top)),
            ),
        ));
    }
    if caps.ghost {
        right = right.kid(inline(
            cx,
            "set_ghost",
            toggle(cx, s.ghost, Action::Set(SetOp::Ghost(!s.ghost))),
        ));
    }
    if caps.autostart {
        right = right.kid(inline(
            cx,
            "set_autostart",
            toggle(cx, s.autostart, Action::Set(SetOp::Autostart(!s.autostart))),
        ));
    }
    right = right.kid(section(cx, "hdr_data"));
    right = right.kid(
        hbox(S1)
            .kid(button_small(cx, cx.t("set_export"), Action::Export, true).tip(cx.t("tip_export")))
            .kid(
                hold_button(
                    cx,
                    cx.t("set_import"),
                    Action::Import,
                    true,
                    IMPORT_HOLD,
                    84.0,
                )
                .tip(format!(
                    "{}\n{}\n{}",
                    cx.t("set_import"),
                    cx.t("tip_import"),
                    cx.t("tip_import_hold")
                )),
            ),
    );
    right = right.kid(
        hold_button(
            cx,
            cx.t("set_reset"),
            Action::Reset,
            true,
            RESET_HOLD,
            COL_W,
        )
        .tip(format!("{}\n{}", cx.t("set_reset"), cx.t("tip_hold_3"))),
    );
    if caps.quit {
        let mut q = button(cx, cx.t("set_quit"), Action::Quit, true);
        q.p.node.width = px(COL_W);
        right = right.kid(q);
    }
    right.p.node.width = px(COL_W);
    right.p.node.flex_shrink = 0.0;

    let mut r = hbox(S2).kid(left).kid(right);
    r.p.node.align_items = AlignItems::FlexStart;
    r
}
