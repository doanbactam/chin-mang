//! Tab Đồ: lưới 8×5 (ô 32 px), lọc ô / độ hiếm, sắp xếp; chọn món → so sánh với món đang mặc
//! của mèo đích (mũi tên xanh/đỏ + số), Mặc / Nâng (giá len) / Rã; "Mặc đồ tốt nhất (cả đội)"; tự rã.

use super::{INNER_W, price, section};
use crate::ui::actions::Action;
use crate::ui::kit::*;
use crate::ui::tokens::*;
use crate::ui::vdom::VNode;
use crate::ui::{GearSort, View};
use bevy::prelude::*;
use cm_core::{CatId, Command, Game, Item, Rarity, Slot, fmt_num};

pub const COLS: usize = 8;
/// Chiều cao vùng chi tiết món (cố định để lưới không nhảy).
pub const DETAIL_H: f32 = 108.0;

/// Lọc + sắp xếp kho (thuần).
pub fn filtered(
    items: &[Item],
    slot: Option<Slot>,
    rarity: Option<Rarity>,
    sort: GearSort,
    score: impl Fn(&Item) -> f64,
) -> Vec<Item> {
    let mut v: Vec<Item> = items
        .iter()
        .filter(|i| slot.is_none_or(|s| i.slot == s) && rarity.is_none_or(|r| i.rarity == r))
        .cloned()
        .collect();
    match sort {
        GearSort::Power => v.sort_by(|a, b| score(b).total_cmp(&score(a)).then(b.id.cmp(&a.id))),
        GearSort::Rarity => {
            v.sort_by(|a, b| b.rarity.cmp(&a.rarity).then(score(b).total_cmp(&score(a))))
        }
        GearSort::Newest => v.sort_by_key(|i| std::cmp::Reverse(i.id)),
    }
    v
}

/// Mèo đích để mặc / so sánh: mèo đang chọn (nếu đã mở) hoặc mèo đã mở đầu tiên.
pub fn target_cat(game: &Game, view: &View) -> CatId {
    view.sel_cat
        .filter(|c| game.cat_info(*c).unlocked)
        .unwrap_or_else(|| {
            CatId::ALL
                .into_iter()
                .find(|c| game.cat_info(*c).unlocked)
                .unwrap_or(CatId::Muop)
        })
}

fn item_title(cx: &Cx, it: &Item) -> String {
    let mut s = format!(
        "{} {}",
        rarity_glyph(it.rarity),
        cx.tf(
            "item_name",
            &[
                cx.t(&format!("rarity_{}", it.rarity.key())),
                cx.t(&format!("slot_{}", it.slot.key()))
            ]
        )
    );
    if it.upgrade > 0 {
        s.push(' ');
        s.push_str(&cx.tf("item_up", &[&it.upgrade.to_string()]));
    }
    s
}

pub fn build(cx: &Cx, game: &Game, view: &View) -> VNode {
    let inv = game.inventory();
    let items = filtered(&inv, view.filter_slot, view.filter_rarity, view.sort, |i| {
        game.item_stats(i).power_score
    });

    // ── lọc / sắp xếp ──
    let mut slot_seg: Vec<(String, bool, Action)> = vec![(
        cx.t("filter_all").into(),
        view.filter_slot.is_none(),
        Action::FilterSlot(None),
    )];
    for s in Slot::ALL {
        slot_seg.push((
            cx.t(&format!("slot_short_{}", s.key())).into(),
            view.filter_slot == Some(s),
            Action::FilterSlot(Some(s)),
        ));
    }
    let next_rarity = match view.filter_rarity {
        None => Some(Rarity::Common),
        Some(Rarity::Legendary) => None,
        Some(r) => Rarity::ALL
            .get(Rarity::ALL.iter().position(|x| *x == r).unwrap_or(0) + 1)
            .copied(),
    };
    let rar_label = match view.filter_rarity {
        None => cx.t("filter_all").to_string(),
        Some(r) => rarity_glyph(r).to_string(),
    };
    let next_sort = match view.sort {
        GearSort::Power => GearSort::Rarity,
        GearSort::Rarity => GearSort::Newest,
        GearSort::Newest => GearSort::Power,
    };
    let sort_label = cx.t(match view.sort {
        GearSort::Power => "sort_power",
        GearSort::Rarity => "sort_rarity",
        GearSort::Newest => "sort_new",
    });
    let cap = game.inventory_cap();
    let filters = hbox(S1)
        .kid(segmented(cx, slot_seg))
        .kid(spacer())
        .kid(
            button_with(
                cx,
                vec![icon(&cx.with_k(1.0), "ic_filter")],
                rar_label,
                Action::FilterRarity(next_rarity),
                true,
                BTN_H_SMALL,
            )
            .tip(format!(
                "{}\n{}",
                cx.t("filter_rarity"),
                cx.t("filter_rarity_hint")
            )),
        )
        .kid(
            button_with(
                cx,
                vec![icon(&cx.with_k(1.0), "ic_sort")],
                "",
                Action::Sort(next_sort),
                true,
                BTN_H_SMALL,
            )
            .tip(format!("{}: {sort_label}", cx.t("sort_label"))),
        )
        .kid(
            text(
                cx,
                format!("{}/{}", inv.len(), cap),
                Fs::Small,
                if inv.len() >= cap {
                    Ink::Bad
                } else {
                    Ink::Muted
                },
            )
            .tip(cx.tf("inv_count", &[&inv.len().to_string(), &cap.to_string()])),
        );

    // ── lưới 8×5 ──
    let mut grid = VNode::boxed(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(S1),
        row_gap: px(S1),
        width: px(COLS as f32 * SLOT + (COLS as f32 - 1.0) * S1),
        flex_shrink: 0.0,
        ..default()
    });
    for i in 0..cap.min(40) {
        grid = grid.kid(match items.get(i) {
            Some(it) => slot(
                cx,
                Some(it.rarity),
                Some(&format!("item_{}_{}", it.slot.key(), it.rarity.key())),
                (it.upgrade > 0).then(|| format!("+{}", it.upgrade)),
                view.sel_item == Some(it.id),
            )
            .action(Action::SelectItem(if view.sel_item == Some(it.id) {
                None
            } else {
                Some(it.id)
            }))
            .tip(item_title(cx, it)),
            None => slot(cx, None, None, None, false),
        });
    }

    let target = target_cat(game, view);
    let detail = match view.sel_item.and_then(|id| game.item(id)) {
        Some(it) => item_detail(cx, game, &it, target),
        None => {
            let hint = if inv.is_empty() {
                cx.t("gear_empty")
            } else {
                cx.t("select_item_hint")
            };
            vbox(S1).kid(para(cx, hint, Fs::Small, Ink::Muted, INNER_W))
        }
    };

    // ── đáy: mặc tốt nhất + tự rã ──
    let auto = game.auto_salvage();
    let bottom = hbox(S2)
        .kid(button_small(
            cx,
            cx.t("btn_equip_best_all"),
            Action::Cmd(Command::EquipBest { cat: None }),
            true,
        ))
        .kid(spacer())
        .kid(text(cx, cx.t("autosalv"), Fs::Small, Ink::Muted))
        .kid(segmented(
            cx,
            vec![
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
            ],
        ));
    let mut detail_box = VNode::boxed(Node {
        height: px(DETAIL_H),
        width: px(INNER_W),
        ..default()
    })
    .kid(detail);
    detail_box.p.node.flex_direction = FlexDirection::Column;
    vbox(S1).kid(filters).kid(grid).kid(detail_box).kid(bottom)
}

fn item_detail(cx: &Cx, game: &Game, it: &Item, target: CatId) -> VNode {
    let st = game.item_stats(it);
    let info = game.cat_info(target);
    let worn = info.equipped[it.slot.index()].clone();
    let worn_st = worn
        .as_ref()
        .map(|w| game.item_stats(w))
        .unwrap_or_default();
    let cat_name = cx.t(&format!("cat_{}", target.key()));

    // trái: món đang chọn
    let left = vbox(0.0)
        .kid(section(cx, "hdr_item"))
        .kid(text_full(
            cx,
            item_title(cx, it),
            Fs::Body,
            rarity_ink(it.rarity),
            true,
        ))
        .kid(text(
            cx,
            cx.tf("item_ilvl", &[&it.ilvl.to_string()]),
            Fs::Small,
            Ink::Muted,
        ));
    // giữa: so sánh với món đang mặc của mèo đích
    let mut cmp = vbox(0.0).kid(section(cx, "hdr_compare"));
    match &worn {
        Some(w) => {
            cmp = cmp.kid(text(
                cx,
                format!("{cat_name}: {}", item_title(cx, w)),
                Fs::Small,
                rarity_ink(w.rarity),
            ));
        }
        None => {
            cmp = cmp.kid(text(
                cx,
                format!("{cat_name}: {}", cx.t("compare_none")),
                Fs::Small,
                Ink::Muted,
            ))
        }
    }
    let pairs = [
        ("istat_hp", st.hp_pct, worn_st.hp_pct),
        ("istat_dmg", st.dmg_pct, worn_st.dmg_pct),
        ("istat_aspd", st.aspd_pct, worn_st.aspd_pct),
        ("istat_crit", st.crit_pct, worn_st.crit_pct),
        ("istat_power", st.power_score, worn_st.power_score),
    ];
    for (k, a, b) in pairs {
        if a == 0.0 && b == 0.0 {
            continue;
        }
        let d = a - b;
        let (arrow, ink) = if d > 0.05 {
            ("▲", Ink::Good)
        } else if d < -0.05 {
            ("▼", Ink::Bad)
        } else {
            ("=", Ink::Muted)
        };
        let label = if k == "istat_power" {
            cx.tf(k, &[&format!("{a:.0}")])
        } else {
            cx.tf(k, &[&format!("{a:.1}")])
        };
        cmp = cmp.kid(
            hbox(S1)
                .kid(text(cx, label, Fs::Small, Ink::Text))
                .kid(text(cx, format!("{arrow} {:+.1}", d), Fs::Small, ink)),
        );
    }
    // phải: hành động
    let up_cost = game.upgrade_cost(it.id);
    let yarn = game.hud().yarn;
    let up = match up_cost {
        Some(c) => button_with(
            cx,
            vec![],
            cx.t("btn_upgrade"),
            Action::Cmd(Command::UpgradeItem { item: it.id }),
            yarn >= c,
            BTN_H_SMALL,
        )
        .kids(price(cx, "ic_yarn", c, yarn >= c)),
        None => button_small(
            cx,
            cx.t("upgrade_max"),
            Action::Cmd(Command::UpgradeItem { item: it.id }),
            false,
        ),
    };
    let salv = game.salvage_value(it.id).unwrap_or(0.0);
    let next_cat = next_unlocked(game, target);
    let mut actions = hbox(S1)
        .kid(button_small(
            cx,
            format!("{} → {cat_name}", cx.t("btn_equip")),
            Action::Cmd(Command::Equip {
                cat: target,
                item: it.id,
            }),
            true,
        ))
        .kid(up)
        .kid(
            button_small(
                cx,
                cx.t("btn_salvage"),
                Action::Cmd(Command::Salvage { item: it.id }),
                true,
            )
            .tip(cx.tf("salvage_gain", &[&fmt_num(salv)])),
        );
    if next_cat != target {
        actions = actions.kid(button_small(
            cx,
            format!("{} →", cx.t("target_cat")),
            Action::SelectCat(Some(next_cat)),
            true,
        ));
    }
    let mut top = hbox(S3).kid(left).kid(cmp);
    top.p.node.align_items = AlignItems::FlexStart;
    vbox(S1).kid(top).kid(actions)
}

fn next_unlocked(game: &Game, cur: CatId) -> CatId {
    let start = cur.index();
    for d in 1..=4 {
        let c = CatId::ALL[(start + d) % 4];
        if game.cat_info(c).unlocked {
            return c;
        }
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;
    fn it(id: u64, slot: Slot, rarity: Rarity) -> Item {
        Item {
            id,
            slot,
            rarity,
            ilvl: 1,
            upgrade: 0,
        }
    }
    #[test]
    fn filter_and_sort() {
        let items = vec![
            it(1, Slot::Bell, Rarity::Common),
            it(2, Slot::Claws, Rarity::Epic),
            it(3, Slot::Bell, Rarity::Rare),
        ];
        let score = |i: &Item| i.rarity.mult();
        let v = filtered(&items, Some(Slot::Bell), None, GearSort::Power, score);
        assert_eq!(v.iter().map(|i| i.id).collect::<Vec<_>>(), vec![3, 1]);
        let v = filtered(&items, None, None, GearSort::Newest, score);
        assert_eq!(v[0].id, 3);
        let v = filtered(&items, None, Some(Rarity::Epic), GearSort::Rarity, score);
        assert_eq!(v.len(), 1);
    }
}
