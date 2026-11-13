//! Bảng (SPEC §7.3): mở ngay trên dải, rộng 360, cao ≤ 420, neo theo nút túi.
//! Thanh tiêu đề 4 tab + cài đặt + nút đóng; thanh "Mục tiêu kế" ở đáy.

mod achievements;
mod cats;
mod gear;
mod settings;
mod souls;

use super::actions::Action;
use super::kit::*;
use super::tokens::*;
use super::vdom::VNode;
use super::{Tab, View};
use crate::layout::Rect;
use crate::platform::Caps;
use crate::settings::Settings;
use bevy::prelude::*;
use cm_core::{Game, Goal, fmt_num};

/// Bề rộng nội dung bên trong khung bảng.
pub const INNER_W: f32 = PANEL_W - 2.0 * S3;
/// Chiều cao vùng nội dung (giữa thanh tab và thanh mục tiêu).
pub const BODY_H: f32 = PANEL_H - 2.0 * S3 - TAB_H - GOAL_H - 2.0 * S1;

pub fn build(
    cx: &Cx,
    game: &Game,
    view: &View,
    settings: &Settings,
    caps: &Caps,
    tab: Tab,
    r: Rect,
) -> VNode {
    let tabs = [
        (Tab::Cats, "ic_cats", "tab_cats"),
        (Tab::Gear, "ic_gear", "tab_gear"),
        (Tab::Souls, "ic_soul", "tab_souls"),
        (Tab::Achievements, "ic_trophy", "tab_ach"),
    ];
    let mut bar = hbox(2.0);
    bar.p.node.height = px(TAB_H);
    for (t, ic, key) in tabs {
        bar = bar.kid(super::kit::tab(
            cx,
            ic,
            cx.t(key),
            tab == t,
            Action::OpenTab(t),
        ));
    }
    bar = bar
        .kid(
            VNode::boxed(Node {
                width: px(TAB_H - S1),
                height: px(TAB_H - S1),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .skin(
                if tab == Tab::Settings {
                    "ui_tab_active"
                } else {
                    "ui_tab"
                },
                cx.k,
            )
            .action(Action::OpenTab(Tab::Settings))
            .tip(cx.t("tab_settings"))
            .kid(icon(cx, "ic_settings")),
        )
        .kid(spacer())
        .kid(icon_button(cx, "ic_close", Action::ClosePanel, TAB_H - S1).tip(cx.t("tip_close")));

    let body = match tab {
        Tab::Cats => cats::build(cx, game, view),
        Tab::Gear => gear::build(cx, game, view),
        Tab::Souls => souls::build(cx, game),
        Tab::Achievements => achievements::build(cx, game),
        Tab::Settings => settings::build(cx, game, settings, caps),
    };
    let body = VNode::boxed(Node {
        width: px(INNER_W),
        height: px(BODY_H),
        flex_direction: FlexDirection::Column,
        row_gap: px(S1),
        overflow: Overflow::clip(),
        ..default()
    })
    .kid(body);

    panel(
        cx,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(S1),
            ..absolute(r.x, r.y, r.w, r.h)
        },
    )
    .kid(bar)
    .kid(body)
    .kid(goal_bar(cx, game))
}

/// Câu cho mục tiêu kế.
pub fn goal_text(cx: &Cx, goal: Goal) -> String {
    let cat_name = |c: cm_core::CatId| cx.t(&format!("cat_{}", c.key()));
    match goal {
        Goal::UnlockCat { cat, fish_needed } if fish_needed <= 0.0 => {
            cx.tf("goal_unlock_cat_ready", &[cat_name(cat)])
        }
        Goal::UnlockCat { cat, fish_needed } => cx.tf(
            "goal_unlock_cat",
            &[cat_name(cat), &fmt_num(fish_needed.ceil())],
        ),
        Goal::ReachStage { zone, stage } => {
            cx.tf("goal_reach_stage", &[&zone.to_string(), &stage.to_string()])
        }
        Goal::DefeatBoss { kind, zone } => cx.tf(
            "goal_defeat_boss",
            &[cx.t(&format!("enemy_{}", kind.key())), &zone.to_string()],
        ),
        Goal::LevelCat { cat, fish_needed } if fish_needed <= 0.0 => {
            cx.tf("goal_level_cat_ready", &[cat_name(cat)])
        }
        Goal::LevelCat { cat, fish_needed } => cx.tf(
            "goal_level_cat",
            &[cat_name(cat), &fmt_num(fish_needed.ceil())],
        ),
        Goal::Prestige { souls } => cx.tf("goal_prestige", &[&fmt_num(souls)]),
    }
}

fn goal_bar(cx: &Cx, game: &Game) -> VNode {
    plate(
        cx,
        Node {
            height: px(GOAL_H),
            width: px(INNER_W),
            column_gap: px(S2),
            align_items: AlignItems::Center,
            ..default()
        },
    )
    .kid(icon(&cx.with_k(1.0), "ic_up"))
    .kid(text(cx, cx.t("goal_label"), Fs::Small, Ink::Accent))
    .kid(text(
        cx,
        goal_text(cx, game.next_goal()),
        Fs::Small,
        Ink::Text,
    ))
}

/// Hàng tiêu đề mục.
pub fn section(cx: &Cx, key: &str) -> VNode {
    header(cx, cx.t(key))
}

/// Giá: icon tiền + số.
pub fn price(cx: &Cx, icon_name: &str, amount: f64, ok: bool) -> Vec<VNode> {
    vec![
        icon(&cx.with_k(1.0), icon_name),
        text(
            cx,
            fmt_num(amount.ceil()),
            Fs::Small,
            if ok { Ink::Text } else { Ink::Bad },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_fits_grid() {
        const { assert!(BODY_H > 300.0) };
        assert_eq!(INNER_W % 4.0, 0.0);
    }
}
