//! Tab Mèo: danh sách 4 hàng + chế độ ×1/×10/Max; chọn mèo → chi tiết inline
//! (chỉ số, 3 ô trang bị, 3 tài năng, kỹ năng tự động).

use super::{INNER_W, price, section};
use crate::ui::View;
use crate::ui::actions::Action;
use crate::ui::kit::*;
use crate::ui::tokens::*;
use crate::ui::vdom::{Kind, VNode};
use bevy::prelude::*;
use cm_core::{BuyMode, CatId, CatInfo, Command, Game, Slot, TALENT_MAX_RANK, UnlockReq, fmt_num};

pub fn build(cx: &Cx, game: &Game, view: &View) -> VNode {
    let modes = [
        (BuyMode::One, "buy_x1"),
        (BuyMode::Ten, "buy_x10"),
        (BuyMode::Max, "buy_max"),
    ];
    let top = hbox(S2)
        .kid(text(cx, cx.t("buy_mode"), Fs::Small, Ink::Muted))
        .kid(segmented(
            cx,
            modes
                .iter()
                .map(|(m, k)| (cx.t(k).to_string(), view.buy == *m, Action::BuyMode(*m)))
                .collect(),
        ));
    let mut v = vbox(S1).kid(top);
    match view.sel_cat {
        Some(cat) => {
            let info = game.cat_info(cat);
            v = v.kid(cat_row(cx, game, view, &info, true));
            if info.unlocked {
                v = v.kid(detail(cx, &info));
            }
        }
        None => {
            for cat in CatId::ALL {
                v = v.kid(cat_row(cx, game, view, &game.cat_info(cat), false));
            }
            v = v.kid(text(cx, cx.t("select_cat_hint"), Fs::Small, Ink::Muted));
        }
    }
    v
}

fn cat_row(cx: &Cx, game: &Game, view: &View, info: &CatInfo, selected: bool) -> VNode {
    let cat = info.cat;
    let name = cx.t(&format!("cat_{}", cat.key()));
    let dim = if info.unlocked {
        Color::WHITE
    } else {
        Color::srgba(0.45, 0.45, 0.55, 1.0)
    };
    // vùng chọn mèo (portrait + tên) là một nút phẳng riêng để không “nuốt” click của nút Lên cấp
    let mut who = VNode::boxed(Node {
        column_gap: px(S2),
        align_items: AlignItems::Center,
        padding: UiRect::all(px(2.0)),
        ..row(S2)
    })
    .kid(
        VNode::boxed(Node {
            width: px(36.0),
            height: px(36.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .skin("ui_slot", 1.0)
        .kid(icon_tint(format!("portrait_{}", cat.key()), 2.0, dim)),
    )
    .kid({
        let mut c = vbox(0.0).kid(
            hbox(S1)
                .kid(text_full(cx, name, Fs::Body, Ink::Text, true))
                .kid(text(
                    cx,
                    cx.t(&format!("role_{}", cat.key())),
                    Fs::Small,
                    Ink::Accent,
                )),
        );
        if info.unlocked {
            c = c.kid(text(
                cx,
                format!(
                    "{} · {}",
                    cx.tf("level_short", &[&info.level.to_string()]),
                    cx.tf("dps", &[&fmt_num(info.dps)])
                ),
                Fs::Small,
                Ink::Muted,
            ));
        } else {
            let UnlockReq::Stage { global_stage, .. } = info.unlock;
            let (z, s) = zone_stage(global_stage);
            c = c.kid(text(
                cx,
                cx.tf("locked_req", &[&z.to_string(), &s.to_string()]),
                Fs::Small,
                if info.unlock_reachable {
                    Ink::Good
                } else {
                    Ink::Muted
                },
            ));
        }
        c
    });
    if info.unlocked {
        who = who.action(Action::SelectCat(if selected { None } else { Some(cat) }));
        if selected {
            who = who.skin("ui_tab_active", cx.k);
        }
    }
    let right = if info.unlocked {
        let q = game.level_quote(cat, view.buy);
        let label = if q.levels > 1 {
            cx.tf("btn_level_up_n", &[&q.levels.to_string()])
        } else {
            cx.t("btn_level_up").to_string()
        };
        let mut b = button_with(
            cx,
            vec![],
            label,
            Action::Cmd(Command::LevelUp {
                cat,
                mode: view.buy,
            }),
            q.affordable && q.levels > 0,
            BTN_H,
        );
        b.p.node.min_width = px(108.0);
        b = b.kids(price(cx, "ic_fish", q.cost, q.affordable));
        b
    } else {
        let UnlockReq::Stage { cost, .. } = info.unlock;
        let can = info.unlock_reachable && game.hud().fish >= cost;
        let mut b = button_with(
            cx,
            vec![icon(&cx.with_k(1.0), "ic_lock")],
            cx.t("btn_unlock"),
            Action::Cmd(Command::Unlock { cat }),
            can,
            BTN_H,
        );
        b.p.node.min_width = px(108.0);
        b.kids(price(cx, "ic_fish", cost, can))
    };
    let mut r = hbox(S2).kid(who).kid(spacer());
    if selected {
        r = r.kid(
            VNode::boxed(Node {
                width: px(BTN_H),
                height: px(BTN_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            })
            .skin("ui_btn", cx.k)
            .action(Action::SelectCat(None))
            .tip(cx.t("btn_back"))
            .kid(icon_k("ic_back", 1.0)),
        );
    }
    let mut row = r.kid(right);
    row.p.node.width = px(INNER_W);
    row.p.node.height = px(ROW_H + S1);
    row
}

fn zone_stage(global_stage: u32) -> (u32, u32) {
    let z = (global_stage / 10) % 5 + 1;
    (z, global_stage % 10 + 1)
}

fn stat(cx: &Cx, key: &str, v: String) -> VNode {
    let mut r = hbox(S1)
        .kid(text(cx, cx.t(key), Fs::Small, Ink::Muted))
        .kid(spacer())
        .kid(text(cx, v, Fs::Small, Ink::Text));
    r.p.node.width = px(76.0);
    r
}

fn detail(cx: &Cx, info: &CatInfo) -> VNode {
    let cat = info.cat;
    let half = (INNER_W - S3) / 2.0;
    // ── trái: chỉ số + trang bị ──
    let stats = VNode::boxed(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(S2),
        row_gap: px(0.0),
        width: px(half),
        ..row(S2)
    })
    .kids([
        stat(cx, "stat_hp", fmt_num(info.hp_max.ceil())),
        stat(cx, "stat_dmg", fmt_num(info.dmg.ceil())),
        stat(cx, "stat_aspd", format!("{:.2}", info.aspd)),
        stat(cx, "stat_crit", format!("{:.0}%", info.crit_pct)),
        stat(cx, "stat_range", format!("{:.0}", info.range)),
    ]);
    let mut slots = hbox(S1);
    for s in Slot::ALL {
        let item = info.equipped[s.index()].clone();
        let (rar, ic, badge, tip) = match &item {
            Some(it) => (
                Some(it.rarity),
                format!("item_{}_{}", s.key(), it.rarity.key()),
                (it.upgrade > 0).then(|| format!("+{}", it.upgrade)),
                format!(
                    "{} {}\n{}\n{}",
                    rarity_glyph(it.rarity),
                    cx.tf(
                        "item_name",
                        &[
                            cx.t(&format!("rarity_{}", it.rarity.key())),
                            cx.t(&format!("slot_{}", s.key()))
                        ]
                    ),
                    cx.tf("item_ilvl", &[&it.ilvl.to_string()]),
                    cx.t("tip_unequip")
                ),
            ),
            None => (
                None,
                format!("item_{}_common", s.key()),
                None,
                format!(
                    "{}\n{}",
                    cx.t(&format!("slot_{}", s.key())),
                    cx.t("empty_slot")
                ),
            ),
        };
        let mut sv = slot(cx, rar, Some(&ic), badge, false).tip(tip);
        if let Some(it) = item {
            sv = sv.action(Action::Cmd(Command::Unequip { cat, slot: it.slot }));
        } else if let Some(Kind::Icon(i)) = sv.kids.first_mut().map(|k| &mut k.p.kind) {
            i.tint = Color::srgba(1.0, 1.0, 1.0, 0.25);
        }
        slots = slots.kid(sv);
    }
    let left = vbox(S1)
        .kid(section(cx, "hdr_stats"))
        .kid(stats)
        .kid(section(cx, "hdr_gear"))
        .kid(slots)
        .kid(button_small(
            cx,
            cx.t("btn_equip_best"),
            Action::Cmd(Command::EquipBest { cat: Some(cat) }),
            true,
        ));
    // ── phải: tài năng + kỹ năng ──
    let mut talents = vbox(2.0).kid(
        hbox(S1).kid(section(cx, "hdr_talents")).kid(spacer()).kid(
            text(
                cx,
                cx.tf("talent_points", &[&info.talent_points.to_string()]),
                Fs::Small,
                if info.talent_points > 0 {
                    Ink::Good
                } else {
                    Ink::Muted
                },
            )
            .tip(cx.t("talent_points_hint")),
        ),
    );
    for (i, (t, rank)) in info.talents.iter().enumerate() {
        let can = info.talent_points > 0 && *rank < TALENT_MAX_RANK;
        let mut r = hbox(S1)
            .kid(icon(&cx.with_k(1.0), format!("talent_{}", t.key())))
            .kid(text(
                cx,
                cx.t(&format!("talent_{}", t.key())),
                Fs::Small,
                Ink::Text,
            ))
            .kid(spacer())
            .kid(text(
                cx,
                cx.tf("talent_rank", &[&rank.to_string()]),
                Fs::Small,
                if *rank >= TALENT_MAX_RANK {
                    Ink::Gold
                } else {
                    Ink::Muted
                },
            ))
            .kid(
                button_small(
                    cx,
                    "+",
                    Action::Cmd(Command::TalentUp { cat, slot: i as u8 }),
                    can,
                )
                .tip(cx.t("tip_talent_up")),
            );
        r.p.node.width = px(half);
        talents = talents.kid(r.tip(cx.t(&format!("talent_{}_desc", t.key()))));
    }
    let skill = vbox(2.0)
        .kid(section(cx, "hdr_skill"))
        .kid(
            hbox(S2)
                .kid(
                    VNode::boxed(Node {
                        width: px(36.0),
                        height: px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .skin("ui_slot", 1.0)
                    .kid(icon_k(format!("skill_{}", cat.key()), 2.0)),
                )
                .kid(
                    vbox(0.0)
                        .kid(text_full(
                            cx,
                            cx.t(&format!("skill_{}_name", cat.key())),
                            Fs::Small,
                            Ink::Text,
                            true,
                        ))
                        .kid(text(
                            cx,
                            cx.tf("skill_cd", &[&format!("{:.0}", info.skill_cd_secs)]),
                            Fs::Small,
                            Ink::Muted,
                        )),
                ),
        )
        .kid(para(
            cx,
            cx.t(&format!("skill_{}_desc", cat.key())),
            Fs::Small,
            Ink::Muted,
            half,
        ));
    let right = vbox(S2).kid(talents).kid(skill);
    let mut d = hbox(S3).kid(left).kid(right);
    d.p.node.align_items = AlignItems::FlexStart;
    d
}
