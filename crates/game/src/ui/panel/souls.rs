//! Tab Hồn: số hồn, nút "Dùng một mạng" (giữ 2 s) + số hồn sẽ nhận / điều kiện, dải 9 dấu ấn mạng,
//! cửa hàng hồn.

use super::{INNER_W, price, section};
use crate::ui::actions::Action;
use crate::ui::kit::*;
use crate::ui::tokens::*;
use crate::ui::vdom::VNode;
use bevy::prelude::*;
use cm_core::{Command, Game, fmt_num};

pub const PRESTIGE_HOLD: f32 = 2.0;

pub fn build(cx: &Cx, game: &Game) -> VNode {
    let hud = game.hud();
    let pre = game.prestige_preview();
    let head = hbox(S2)
        .kid(icon(cx, "ic_soul"))
        .kid(title(cx, cx.tf("souls_have", &[&fmt_num(hud.soul)])))
        .kid(spacer())
        .kid(text(
            cx,
            cx.tf("lives_used", &[&pre.lives_used.to_string()]),
            Fs::Small,
            Ink::Muted,
        ));

    let info = if pre.available {
        text(
            cx,
            cx.tf("prestige_gain", &[&fmt_num(pre.souls_gain)]),
            Fs::Body,
            Ink::Epic,
        )
    } else {
        let z = (pre.required_global_stage / 10) % 5 + 1;
        let s = pre.required_global_stage % 10 + 1;
        text(
            cx,
            cx.tf("prestige_req", &[&z.to_string(), &s.to_string()]),
            Fs::Small,
            Ink::Muted,
        )
    };
    let prestige = hbox(S2)
        .kid(
            hold_button(
                cx,
                cx.t("btn_prestige"),
                Action::Cmd(Command::Prestige),
                pre.available,
                PRESTIGE_HOLD,
                132.0,
            )
            .tip(format!(
                "{}\n{}\n{}",
                cx.t("btn_prestige"),
                cx.t("prestige_desc"),
                cx.t("tip_hold_2")
            )),
        )
        .kid(info);

    // dấu ấn mạng
    let marks = game.life_marks();
    let mut life = hbox(S1);
    for (i, got) in marks.iter().enumerate() {
        let tint = if *got {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.25)
        };
        life = life.kid(
            VNode::boxed(Node {
                width: px(24.0),
                height: px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .skin(if *got { "ui_slot_epic" } else { "ui_slot" }, 1.0)
            .kid(icon_tint(format!("life_{}", i + 1), 2.0, tint))
            .tip(format!(
                "{}\n{}",
                cx.t(&format!("life_{}", i + 1)),
                if *got {
                    cx.t("life_mark_desc")
                } else {
                    cx.t("life_mark_locked")
                }
            )),
        );
    }

    // cửa hàng
    let mut shop = vbox(2.0);
    for row in game.soul_shop() {
        let key = row.upgrade.key();
        let rank = match row.max_rank {
            Some(m) => cx.tf("shop_rank_max", &[&row.rank.to_string(), &m.to_string()]),
            None => cx.tf("shop_rank", &[&row.rank.to_string()]),
        };
        let buy = match row.cost {
            Some(c) => {
                let mut b = button_with(
                    cx,
                    vec![],
                    cx.t("btn_buy"),
                    Action::Cmd(Command::BuySoul {
                        upgrade: row.upgrade,
                    }),
                    row.affordable,
                    BTN_H_SMALL,
                )
                .kids(price(cx, "ic_soul", c, row.affordable));
                b.p.node.min_width = px(76.0);
                b
            }
            None => text(cx, cx.t("shop_maxed"), Fs::Small, Ink::Gold),
        };
        let mut r = hbox(S2)
            .kid(icon(cx, format!("soul_{key}")))
            .kid(text_full(
                cx,
                cx.t(&format!("soul_{key}")),
                Fs::Small,
                Ink::Text,
                true,
            ))
            .kid(text(cx, rank, Fs::Small, Ink::Muted))
            .kid(spacer())
            .kid(buy)
            .tip(format!(
                "{}\n{}",
                cx.t(&format!("soul_{key}")),
                cx.t(&format!("soul_{key}_desc"))
            ));
        r.p.node.width = px(INNER_W);
        shop = shop.kid(r);
    }

    vbox(S1)
        .kid(head)
        .kid(prestige)
        .kid(section(cx, "hdr_life_marks"))
        .kid(life)
        .kid(section(cx, "hdr_shop"))
        .kid(shop)
}
