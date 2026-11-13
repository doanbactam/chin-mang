//! Hướng dẫn lần đầu (SPEC §7.7): bong bóng trên dải, không chặn; mỗi gợi ý một lần (lõi lưu trong save).

use super::actions::Action;
use super::hud::right_cluster_w;
use super::kit::*;
use super::tokens::*;
use super::vdom::VNode;
use crate::layout::Rect;
use bevy::prelude::*;
use cm_core::{Game, TutorialId};

pub fn key(id: TutorialId) -> &'static str {
    match id {
        TutorialId::OpenBag => "tut_open_bag",
        TutorialId::UnlockXiem => "tut_unlock_xiem",
        TutorialId::FirstBox => "tut_first_box",
    }
}

pub fn build(cx: &Cx, game: &Game, strip: Rect, panel_open: bool) -> Vec<VNode> {
    let Some(id) = game.pending_tutorial() else {
        return vec![];
    };
    // gợi ý mở túi / mở Xiêm trỏ vào cụm nút phải; gợi ý hộp nằm giữa dải
    if panel_open && id != TutorialId::FirstBox {
        return vec![];
    }
    let bubble = VNode::boxed(Node {
        padding: UiRect::axes(px(S2), px(S1)),
        align_items: AlignItems::Center,
        ..row(S2)
    })
    .skin("ui_tooltip", cx.k.min(2.0))
    .solid()
    .kid(vbox(0.0).kid(header(cx, cx.t("tut_header"))).kid(text(
        cx,
        cx.t(key(id)),
        Fs::Body,
        Ink::Text,
    )))
    .kid(button_small(
        cx,
        cx.t("tut_ok"),
        Action::DismissTutorial(id),
        true,
    ));
    let right = right_cluster_w(cx.k) + S4;
    let mut wrap = VNode::boxed(Node {
        position_type: PositionType::Absolute,
        top: px(strip.y + (strip.h * 0.5).floor()),
        ..default()
    })
    .kid(bubble);
    match id {
        TutorialId::FirstBox => {
            // phía trên-trái đội hình: không che hộp (hộp rơi trước đội, sát mặt đất)
            wrap.p.node.left = px(strip.x + (strip.w * 0.12).floor());
            wrap.p.node.top = px(strip.y + S1);
        }
        _ => {
            wrap.p.node.right = px(right);
            wrap.p.node.top = px(strip.y + S1);
        }
    }
    vec![wrap]
}
