//! Thẻ tóm tắt vắng mặt (SPEC §7.4): thẻ nhỏ trên dải, chỉ có thưởng; nút "Nhận" (tự nhận sau 10 s).

use super::actions::Action;
use super::kit::*;
use super::tokens::*;
use super::vdom::VNode;
use crate::i18n::duration;
use crate::layout::Rect;
use crate::sim::Offline;
use bevy::prelude::*;
use cm_core::fmt_num;

pub const AUTO_CLAIM_SECS: f32 = 10.0;

pub fn build(cx: &Cx, offline: &Offline, strip: Rect) -> Vec<VNode> {
    // thẻ dùng khung kem: chữ tối
    let cx = &cx.lit();
    let Some(r) = offline.report else {
        return vec![];
    };
    let mut gains = hbox(S2)
        .kid(icon(&cx.with_k(1.0), "ic_fish"))
        .kid(text(
            cx,
            cx.tf("off_fish", &[&fmt_num(r.fish)]),
            Fs::Small,
            Ink::Gold,
        ))
        .kid(icon(&cx.with_k(1.0), "ic_yarn"))
        .kid(text(
            cx,
            cx.tf("off_yarn", &[&fmt_num(r.yarn)]),
            Fs::Small,
            Ink::Text,
        ));
    if r.wake_buff_secs > 0.0 {
        gains = gains.kid(text(
            cx,
            cx.tf("off_wake", &[&duration(cx.lang, r.wake_buff_secs as u64)]),
            Fs::Small,
            Ink::Good,
        ));
    }
    let mut info = vbox(0.0)
        .kid(text_full(
            cx,
            cx.tf("off_title", &[&duration(cx.lang, r.secs)]),
            Fs::Body,
            Ink::Text,
            true,
        ))
        .kid(gains);
    if r.capped {
        info = info.kid(text(cx, cx.t("off_capped"), Fs::Small, Ink::Muted));
    }
    let left = (AUTO_CLAIM_SECS - offline.shown).ceil().max(0.0) as u32;
    let claim = vbox(0.0)
        .kid(button(cx, cx.t("btn_claim"), Action::ClaimOffline, true))
        .kid(text(
            cx,
            cx.tf("off_auto", &[&left.to_string()]),
            Fs::Small,
            Ink::Muted,
        ));
    let card = VNode::boxed(Node {
        padding: UiRect::axes(px(S3), px(S1)),
        column_gap: px(S3),
        align_items: AlignItems::Center,
        ..row(S3)
    })
    .skin("ui_panel", cx.k.min(2.0))
    .solid()
    .kid(icon_k("ic_lives", cx.k))
    .kid(info)
    .kid(claim);
    vec![
        VNode::boxed(Node {
            position_type: PositionType::Absolute,
            left: px(strip.x),
            width: px(strip.w),
            top: px(strip.y),
            height: px(strip.h),
            justify_content: JustifyContent::Center,
            // đáy dải: không che nhãn khu/màn ở giữa-trên
            align_items: AlignItems::FlexEnd,
            padding: UiRect::bottom(px(2.0)),
            ..default()
        })
        .kid(card),
    ]
}
