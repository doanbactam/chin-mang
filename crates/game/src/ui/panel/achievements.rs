//! Tab Thành tựu: lưới 24 icon (khóa = mờ + ổ khóa), tooltip tên + mô tả; thống kê tổng.

use super::section;
use crate::i18n::duration;
use crate::ui::kit::*;
use crate::ui::tokens::*;
use crate::ui::vdom::VNode;
use bevy::prelude::*;
use cm_core::{Game, fmt_num};

pub fn build(cx: &Cx, game: &Game) -> VNode {
    let achs = game.achievements();
    let got = achs.iter().filter(|a| a.unlocked).count();
    let mut grid = VNode::boxed(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(S1),
        row_gap: px(S1),
        width: px(8.0 * 36.0 + 7.0 * S1),
        ..default()
    });
    for a in &achs {
        let key = a.id.key();
        let tint = if a.unlocked {
            Color::WHITE
        } else {
            Color::srgba(0.55, 0.55, 0.65, 0.35)
        };
        let mut cell = VNode::boxed(Node {
            width: px(36.0),
            height: px(36.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .skin(
            if a.unlocked {
                "ui_slot_legendary"
            } else {
                "ui_slot"
            },
            1.0,
        )
        .kid(icon_tint(format!("ach_{}", a.id.group()), 2.0, tint));
        if !a.unlocked {
            cell = cell.kid(
                VNode::boxed(Node {
                    position_type: PositionType::Absolute,
                    right: px(1.0),
                    bottom: px(1.0),
                    ..default()
                })
                .kid(icon_k("ic_lock", 1.0)),
            );
        }
        let status = if a.unlocked { "" } else { cx.t("ach_locked") };
        let tip = if status.is_empty() {
            format!(
                "{}\n{}",
                cx.t(&format!("ach_{key}")),
                cx.t(&format!("ach_{key}_desc"))
            )
        } else {
            format!(
                "{}\n{}\n{status}",
                cx.t(&format!("ach_{key}")),
                cx.t(&format!("ach_{key}_desc"))
            )
        };
        grid = grid.kid(cell.tip(tip));
    }
    let st = game.stats();
    let line = |k: &str, v: String| {
        hbox(S2)
            .kid(text(cx, cx.t(k), Fs::Small, Ink::Muted))
            .kid(spacer())
            .kid(text(cx, v, Fs::Small, Ink::Text))
    };
    let mut totals = vbox(0.0).kids([
        line("stat_play", duration(cx.lang, st.play_secs as u64)),
        line("stat_kills", fmt_num(st.enemies_killed as f64)),
        line("stat_bosses", fmt_num(st.bosses_killed as f64)),
        line("stat_fish_total", fmt_num(st.fish_earned)),
        line("stat_boxes", fmt_num(st.boxes_opened as f64)),
    ]);
    totals.p.node.width = px(220.0);
    vbox(S2)
        .kid(hbox(S2).kid(icon(cx, "ic_trophy")).kid(title(
            cx,
            cx.tf("ach_count", &[&got.to_string(), &achs.len().to_string()]),
        )))
        .kid(grid)
        .kid(section(cx, "hdr_totals"))
        .kid(totals)
}
