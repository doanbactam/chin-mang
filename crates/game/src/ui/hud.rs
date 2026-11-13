//! Dải chính (SPEC §7.2): tiền tệ (trái), khu/màn + tiến độ đợt hoặc máu boss (giữa-trên),
//! nút catnip / túi / cài đặt (phải).

use super::actions::Action;
use super::kit::*;
use super::tokens::*;
use super::vdom::VNode;
use super::{Tab, View};
use crate::layout::Rect;
use crate::settings::Settings;
use bevy::prelude::*;
use cm_core::{CatId, Command, Game, Hud, fmt_num};

/// Bề rộng cụm nút phải (để biết vùng “nút HUD” cho Ghost mode): 3 nút + khoảng cách.
pub fn right_cluster_w(k: f32) -> f32 {
    3.0 * btn_size(k) + 2.0 * S2 + S2
}
/// Tối thiểu 24 px để dễ bấm bằng chuột ở HUD ×1 (icon 12 px + lề 6 px mỗi bên).
pub fn btn_size(k: f32) -> f32 {
    (20.0 * k).max(24.0)
}
/// Hệ số art của HUD theo bố cục: một nửa hệ số thế giới (làm tròn lên) để thế giới là nhân vật chính
/// (×1, ×2 → HUD ×1; ×3, ×4 → HUD ×2).
pub fn hud_k(l: &crate::layout::LayoutOut) -> f32 {
    hud_k_for(l.scale as f32 / l.ui_scale)
}
pub fn hud_k_for(world_k: f32) -> f32 {
    (world_k.max(1.0).round() / 2.0).ceil().max(1.0)
}

pub fn build(
    cx: &Cx,
    game: &Game,
    strip: Rect,
    view: &View,
    _settings: &Settings,
    now: f32,
) -> VNode {
    let hud = game.hud();
    let k = cx.k;
    let pad = S1 * k.min(2.0);
    let mut root = VNode::boxed(absolute(strip.x, strip.y, strip.w, strip.h));

    // ── trái: tiền tệ ──
    let mut cur = vbox(0.0)
        .kid(
            number_label(cx, "ic_fish", &fmt_num(hud.fish), Ink::Gold, 28.0 * cx.ts)
                .tip(cx.t("tip_fish")),
        )
        .kid({
            let mut r = hbox(S2).kid(
                number_label(cx, "ic_yarn", &fmt_num(hud.yarn), Ink::Text, 20.0 * cx.ts)
                    .tip(cx.t("tip_yarn")),
            );
            if hud.soul > 0.0 {
                r = r.kid(
                    number_label(cx, "ic_soul", &fmt_num(hud.soul), Ink::Epic, 12.0)
                        .tip(cx.t("tip_soul")),
                );
            }
            r
        });
    if hud.wake_buff > 0.0 {
        let dur = crate::i18n::duration(cx.lang, hud.wake_buff.ceil() as u64);
        cur = cur.kid(text(cx, "×2", Fs::Small, Ink::Good).tip(cx.tf("tip_wake", &[&dur])));
    }
    root = root.kid(
        VNode::boxed(Node {
            position_type: PositionType::Absolute,
            left: px(pad),
            top: px(pad),
            ..default()
        })
        .kid(plate(cx, Node::default()).kid(cur)),
    );

    // ── giữa-trên: khu / màn / tiến độ ──
    root = root.kid(
        VNode::boxed(Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            right: px(0.0),
            top: px(pad),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .kid(center_block(cx, &hud, now)),
    );

    // ── phải: nút ──
    let b = btn_size(k);
    let catnip = catnip_button(cx, &hud, b);
    let bag = {
        let mut v = icon_button(cx, "ic_bag", Action::TogglePanel(Tab::Cats), b);
        let mut tip = cx.t("tip_bag").to_string();
        if hud.any_affordable {
            v = v.kid(red_dot(4.0 * k + 2.0));
            tip = format!("{tip}\n{}", cx.t("tip_bag_dot"));
        }
        if view.panel.is_some() {
            v = v.skin("ui_tab_active", k);
        }
        v.tip(tip)
    };
    let gear =
        icon_button(cx, "ic_settings", Action::OpenTab(Tab::Settings), b).tip(cx.t("tip_settings"));
    root.kid(
        VNode::boxed(Node {
            position_type: PositionType::Absolute,
            right: px(pad + S1),
            top: px(((strip.h - b) / 2.0).floor().max(0.0)),
            ..row(S2)
        })
        .kids([catnip, bag, gear]),
    )
}

fn center_block(cx: &Cx, hud: &Hud, now: f32) -> VNode {
    let k = cx.k;
    let zone_name = cx.t(&format!("zone_{}", hud.zone));
    let mut label = cx.tf("hud_zone", &[&hud.zone.to_string(), &hud.stage.to_string()]);
    if hud.cycle > 0 {
        label = format!(
            "{} · {label}",
            cx.tf("hud_cycle", &[&(hud.cycle + 1).to_string()])
        );
    }
    let mut col = vbox(2.0).kid(hbox(S1).kid(text(cx, label, Fs::Body, Ink::Text)).kid(text(
        cx,
        format!("· {zone_name}"),
        Fs::Small,
        Ink::Muted,
    )));
    col.p.node.align_items = AlignItems::Center;
    let bar_w = 120.0 * (k / 2.0).max(0.75);
    if let Some(boss) = &hud.boss {
        let name = cx.t(&format!("enemy_{}", boss.kind.key()));
        let secs = boss.time_left.max(0.0).ceil() as u32;
        // đồng hồ nhấp nháy khi còn ≤ 10 s
        let urgent = secs <= 10 && (now * 2.0).fract() < 0.5;
        col = col
            .kid(
                hbox(S1)
                    .kid(text(cx, cx.t("hud_boss"), Fs::Small, Ink::Bad))
                    .kid(bar(
                        cx,
                        (boss.hp / boss.hp_max.max(1.0)) as f32,
                        "ui_bar_boss",
                        bar_w,
                        4.0 * k.min(2.0),
                    ))
                    .kid(text(
                        cx,
                        format!("{}:{:02}", secs / 60, secs % 60),
                        Fs::Small,
                        if urgent { Ink::Bad } else { Ink::Text },
                    )),
            )
            .kid(text(cx, name, Fs::Small, Ink::Muted));
    } else {
        // thanh tiến độ đợt mảnh (2 px logic)
        col = col.kid(bar(cx, hud.wave_progress, "ui_bar_xp", bar_w, 2.0 * k));
    }
    let mut v = plate(
        cx,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
    )
    .kid(col);
    if hud.farming {
        v = hbox(S2).kid(v).kid(
            plate(cx, Node::default())
                .kid(text(cx, cx.t("hud_farming"), Fs::Small, Ink::Muted))
                .kid(button_small(
                    cx,
                    cx.t("btn_retry_boss"),
                    Action::Cmd(Command::RetryBoss),
                    true,
                )),
        );
    }
    v
}

fn catnip_button(cx: &Cx, hud: &Hud, b: f32) -> VNode {
    let ready = hud.catnip_cd <= 0.0;
    let usable = ready || hud.catnip_charges > 0;
    let mut v = icon_button(cx, "ic_catnip", Action::Cmd(Command::Catnip), b);
    if hud.catnip_active > 0.0 {
        v = v.skin("ui_tab_active", cx.k);
    }
    // vòng hồi chiêu: lớp tối phủ từ trên xuống theo phần thời gian còn lại
    if !ready {
        v = v.kid(
            VNode::boxed(Node {
                position_type: PositionType::Absolute,
                left: px(2.0),
                right: px(2.0),
                bottom: px(2.0),
                height: Val::Percent((hud.catnip_cd.clamp(0.0, 1.0) * 92.0).round()),
                ..default()
            })
            .bg(Color::srgba(0.08, 0.08, 0.16, 0.6)),
        );
    }
    if hud.catnip_charges > 0 {
        v = v.kid(
            VNode::boxed(Node {
                position_type: PositionType::Absolute,
                right: px(2.0),
                bottom: px(-2.0),
                ..default()
            })
            .kid(text(
                cx,
                hud.catnip_charges.to_string(),
                Fs::Small,
                Ink::Gold,
            )),
        );
    }
    let mut tip = cx.t("tip_catnip").to_string();
    if hud.catnip_active > 0.0 {
        tip = format!(
            "{tip}\n{}",
            cx.tf(
                "tip_catnip_active",
                &[&(hud.catnip_active.ceil() as u32).to_string()]
            )
        );
    }
    if hud.catnip_charges > 0 {
        tip = format!(
            "{tip}\n{}",
            cx.tf("tip_catnip_charges", &[&hud.catnip_charges.to_string()])
        );
    }
    v.disabled(!usable).tip(tip)
}

/// Tooltip khi rê chuột lên mèo (tên, cấp, HP, sát thương).
pub fn cat_tooltip(cx: &Cx, game: &Game, cat: CatId) -> Vec<VNode> {
    let info = game.cat_info(cat);
    let hp = game
        .cats()
        .into_iter()
        .find(|c| c.cat == cat)
        .map(|c| c.hp)
        .unwrap_or(info.hp_max);
    let name = cx.t(&format!("cat_{}", cat.key()));
    vec![
        text_full(
            cx,
            cx.tf("cat_tip", &[name, &info.level.to_string()]),
            Fs::Body,
            Ink::Text,
            true,
        ),
        text(
            cx,
            cx.t(&format!("role_{}", cat.key())),
            Fs::Small,
            Ink::Accent,
        ),
        text(
            cx,
            cx.tf(
                "cat_tip_stats",
                &[
                    &fmt_num(hp.ceil()),
                    &fmt_num(info.hp_max.ceil()),
                    &fmt_num(info.dmg.ceil()),
                ],
            ),
            Fs::Small,
            Ink::Text,
        ),
        text(cx, cx.t("cat_tip_hint"), Fs::Small, Ink::Muted),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn hud_is_half_world_scale() {
        assert_eq!(super::hud_k_for(1.0), 1.0);
        assert_eq!(super::hud_k_for(2.0), 1.0);
        assert_eq!(super::hud_k_for(3.0), 2.0);
        assert_eq!(super::hud_k_for(4.0), 2.0);
    }

    #[test]
    fn hud_buttons_stay_clickable() {
        assert_eq!(super::btn_size(1.0), 24.0);
        assert_eq!(super::btn_size(2.0), 40.0);
    }
}
