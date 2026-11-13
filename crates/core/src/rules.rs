//! Công thức thuần (giá, chỉ số, hồn…) — không đổi trạng thái.

use crate::api::*;
use crate::data::*;
use crate::state::*;

/// Chỉ số một món sau nâng cấp (SPEC §2.6).
pub fn item_stats(item: &Item) -> ItemStats {
    let m = item.rarity.mult() * (1.0 + 0.1 * item.upgrade as f64);
    let il = item.ilvl as f64;
    let mut s = ItemStats::default();
    match item.slot {
        Slot::Collar => s.hp_pct = (5.0 + il * 0.6) * m,
        Slot::Claws => s.dmg_pct = (4.0 + il * 0.5) * m,
        Slot::Bell => {
            s.aspd_pct = (2.0 + il * 0.12) * m;
            s.crit_pct = 2.0 * m;
        }
    }
    s.power_score = s.hp_pct + s.dmg_pct + s.aspd_pct + s.crit_pct * 1.5;
    s
}

pub fn power_score(item: &Item) -> f64 {
    item_stats(item).power_score
}

/// Giá nâng món từ bậc hiện tại lên bậc kế; None nếu đã +10.
pub fn upgrade_cost(item: &Item) -> Option<f64> {
    if item.upgrade >= ITEM_MAX_UPGRADE {
        return None;
    }
    let n = item.upgrade as f64 + 1.0;
    Some((5.0 * n * n * item.rarity.mult()).round())
}

pub fn salvage_value(item: &Item) -> f64 {
    (3.0 * item.rarity.mult() * (1.0 + item.upgrade as f64)).round()
}

/// Giá lên 1 cấp từ `level`.
pub fn level_cost(cat: CatId, level: u32) -> f64 {
    cat_def(cat).base_cost * LEVEL_COST_GROWTH.powi(level as i32 - 1)
}

/// Tổng giá lên `n` cấp từ `level`.
pub fn level_cost_n(cat: CatId, level: u32, n: u32) -> f64 {
    let c0 = level_cost(cat, level);
    let g = LEVEL_COST_GROWTH;
    c0 * (g.powi(n as i32) - 1.0) / (g - 1.0)
}

pub fn level_quote(s: &State, cat: CatId, mode: BuyMode) -> LevelQuote {
    let level = s.cat(cat).level;
    match mode {
        BuyMode::One | BuyMode::Ten => {
            let n = if mode == BuyMode::One { 1 } else { 10 };
            let cost = level_cost_n(cat, level, n);
            LevelQuote {
                levels: n,
                cost,
                affordable: s.cat(cat).unlocked && s.fish >= cost,
            }
        }
        BuyMode::Max => {
            let c0 = level_cost(cat, level);
            let g = LEVEL_COST_GROWTH;
            let mut n = if s.fish < c0 {
                0
            } else {
                ((1.0 + s.fish * (g - 1.0) / c0).ln() / g.ln())
                    .floor()
                    .max(0.0) as u32
            };
            while n > 0 && level_cost_n(cat, level, n) > s.fish {
                n -= 1;
            }
            while level_cost_n(cat, level, n + 1) <= s.fish {
                n += 1;
            }
            if n == 0 {
                LevelQuote {
                    levels: 0,
                    cost: c0,
                    affordable: false,
                }
            } else {
                LevelQuote {
                    levels: n,
                    cost: level_cost_n(cat, level, n),
                    affordable: s.cat(cat).unlocked,
                }
            }
        }
    }
}

pub fn talent_points_total(level: u32) -> u32 {
    level / LEVELS_PER_TALENT_POINT
}

pub fn talent_points_free(c: &CatState) -> u32 {
    let spent: u32 = c.talents.iter().map(|&r| r as u32).sum();
    talent_points_total(c.level).saturating_sub(spent)
}

/// Hệ số của một nâng cấp hồn. Vuốt/lông/ví nhân dồn `(1 + x)^bậc` (theo kịp độ khó hàm mũ);
/// các dòng khác cộng tuyến tính.
pub fn soul_mult(s: &State, u: SoulUpgrade) -> f64 {
    let d = soul_def(u);
    let r = s.soul_rank(u) as i32;
    match u {
        SoulUpgrade::ClawSharpen | SoulUpgrade::ThickFur | SoulUpgrade::FatWallet => {
            (1.0 + d.per_rank).powi(r)
        }
        _ => 1.0 + d.per_rank * r as f64,
    }
}

pub fn derive(s: &State, cat: CatId) -> Derived {
    let def = cat_def(cat);
    let c = s.cat(cat);
    let lm = STAT_GROWTH.powi(c.level as i32 - 1);
    let mut hp_pct = 0.0;
    let mut dmg_pct = 0.0;
    let mut aspd_pct = 0.0;
    let mut crit = 0.0;
    for it in c.equipped.iter().flatten() {
        let st = item_stats(it);
        hp_pct += st.hp_pct;
        dmg_pct += st.dmg_pct;
        aspd_pct += st.aspd_pct;
        crit += st.crit_pct;
    }
    let t = c.talents;
    let frenzy = if cat == CatId::Muop {
        1.0 + TALENT_FRENZY * t[2] as f64
    } else {
        1.0
    };
    Derived {
        hp_max: def.hp
            * lm
            * (1.0 + hp_pct / 100.0)
            * (1.0 + TALENT_VIGOR * t[1] as f64)
            * soul_mult(s, SoulUpgrade::ThickFur),
        dmg: def.dmg
            * lm
            * (1.0 + dmg_pct / 100.0)
            * (1.0 + TALENT_POWER * t[0] as f64)
            * soul_mult(s, SoulUpgrade::ClawSharpen),
        aspd: def.aspd * (1.0 + aspd_pct / 100.0) * frenzy,
        crit_pct: crit.min(CRIT_CAP),
    }
}

/// Tính lại chỉ số mọi mèo; giữ tỉ lệ máu hiện tại.
pub fn recalc(s: &mut State) {
    for c in CatId::ALL {
        let d = derive(s, c);
        let cs = s.cat_mut(c);
        let ratio = if cs.d.hp_max > 0.0 {
            (cs.hp / cs.d.hp_max).clamp(0.0, 1.0)
        } else {
            1.0
        };
        cs.d = d;
        cs.hp = if cs.alive {
            (ratio * d.hp_max).max(1e-9)
        } else {
            0.0
        };
    }
}

/// DPS ước tính của mèo (đòn thường + crit + kỹ năng trung bình).
pub fn est_dps(cat: CatId, d: &Derived, talents: [u8; 3]) -> f64 {
    let base = d.dmg * d.aspd * (1.0 + d.crit_pct / 100.0 * (CRIT_MULT - 1.0));
    let cd = cat_def(cat).skill_cd;
    let skill = match cat {
        CatId::Muop => d.dmg * MUOP_COMBO_HITS as f64 * MUOP_COMBO_MULT / cd,
        CatId::Xiem => d.dmg * XIEM_RAIN_BONES as f64 * XIEM_RAIN_MULT / cd,
        CatId::Map => 0.0,
        CatId::Mun => base * TALENT_HEX_AOE * talents[2] as f64,
    };
    base + skill
}

/// Hệ số cá toàn cục (không gồm buff vừa ngủ dậy).
pub fn fish_mult(s: &State) -> f64 {
    let marks = s.lives_used.min(LIFE_MARKS) as f64;
    soul_mult(s, SoulUpgrade::FatWallet) * (1.0 + LIFE_MARK_FISH * marks)
}

pub fn gear_chance_mult(s: &State) -> f64 {
    soul_mult(s, SoulUpgrade::LuckyPaw)
}

/// Hệ số HP/DMG quái: `ENEMY_SCALE^gs` tới màn `ENEMY_SCALE_KNEE`, sau đó `ENEMY_SCALE_LATE`/màn.
pub fn enemy_scale(gs: u32) -> f64 {
    let early = gs.min(ENEMY_SCALE_KNEE) as i32;
    let late = gs.saturating_sub(ENEMY_SCALE_KNEE) as i32;
    ENEMY_SCALE.powi(early) * ENEMY_SCALE_LATE.powi(late)
}

pub fn reward_scale(gs: u32) -> f64 {
    REWARD_SCALE.powi(gs as i32)
}

pub fn yarn_drop_amount(gs: u32) -> f64 {
    ((1 + gs / 10) as f64 * YARN_GROWTH.powi(gs as i32)).round()
}

/// Hồn theo công thức (không xét điều kiện tiến bộ của mạng): 0 nếu < PRESTIGE_MIN_GS.
pub fn prestige_gain(max_gs: u32) -> f64 {
    if max_gs < PRESTIGE_MIN_GS {
        return 0.0;
    }
    let x = (max_gs as f64 - PRESTIGE_BASE_GS).max(0.0);
    (x.powf(PRESTIGE_EXP) / PRESTIGE_DIV).floor().max(1.0)
}

/// global_stage cần đạt trong mạng này để được prestige:
/// `max(PRESTIGE_MIN_GS, màn xuất phát + PRESTIGE_MIN_PROGRESS)`.
pub fn prestige_required(s: &State) -> u32 {
    PRESTIGE_MIN_GS.max(s.life_start_gs.saturating_add(PRESTIGE_MIN_PROGRESS))
}

pub fn prestige_available(s: &State) -> bool {
    s.max_gs >= prestige_required(s)
}

/// Hồn thực nhận nếu prestige ngay (0 khi chưa đủ điều kiện).
pub fn prestige_souls(s: &State) -> f64 {
    if prestige_available(s) {
        prestige_gain(s.max_gs)
    } else {
        0.0
    }
}

pub fn soul_cost(u: SoulUpgrade, rank: u32) -> Option<f64> {
    let d = soul_def(u);
    if let Some(m) = d.max_rank
        && rank >= m
    {
        return None;
    }
    Some((d.base * d.growth.powi(rank as i32)).round().max(1.0))
}

pub fn offline_cap(s: &State) -> u64 {
    OFFLINE_BASE_CAP
        + (soul_def(SoulUpgrade::LongNap).per_rank as u64)
            * s.soul_rank(SoulUpgrade::LongNap) as u64
}

/// Thứ tự đội hình của các mèo đã mở (0 = đứng đầu).
pub fn formation_slot(s: &State, cat: CatId) -> usize {
    let my = cat_def(cat).order;
    CatId::ALL
        .iter()
        .filter(|&&c| s.cat(c).unlocked && cat_def(c).order < my)
        .count()
}

/// Ước lượng cá/giây ở màn hiện tại (khi chưa có dữ liệu thu nhập thực).
pub fn estimate_fish_rate(s: &State) -> f64 {
    let gs = s.global_stage();
    let kinds = zone_enemies(s.zone);
    let avg: f64 = kinds.iter().map(|&k| enemy_def(k).fish).sum::<f64>() / kinds.len() as f64;
    let n = (3 + s.stage / 3).min(MAX_WAVE) as f64;
    avg * n * reward_scale(gs) * fish_mult(s) / 15.0
}

pub fn estimate_yarn_rate(s: &State) -> f64 {
    let gs = s.global_stage();
    let n = (3 + s.stage / 3).min(MAX_WAVE) as f64;
    n * YARN_DROP_CHANCE * yarn_drop_amount(gs) / 15.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_cost_formula() {
        assert!((level_cost(CatId::Muop, 1) - 10.0).abs() < 1e-9);
        assert!((level_cost(CatId::Xiem, 1) - 20.0).abs() < 1e-9);
        assert!((level_cost(CatId::Map, 1) - 40.0).abs() < 1e-9);
        assert!((level_cost(CatId::Mun, 1) - 60.0).abs() < 1e-9);
        assert!((level_cost(CatId::Muop, 11) - 10.0 * 1.13f64.powi(10)).abs() < 1e-9);
        let sum: f64 = (0..10).map(|i| level_cost(CatId::Muop, 5 + i)).sum();
        assert!((level_cost_n(CatId::Muop, 5, 10) - sum).abs() < 1e-6);
    }

    #[test]
    fn item_formulas() {
        let it = Item {
            id: 1,
            slot: Slot::Collar,
            rarity: Rarity::Rare,
            ilvl: 10,
            upgrade: 0,
        };
        assert!((item_stats(&it).hp_pct - (5.0 + 6.0) * 1.6).abs() < 1e-9);
        let claws = Item {
            slot: Slot::Claws,
            rarity: Rarity::Epic,
            upgrade: 2,
            ..it.clone()
        };
        assert!((item_stats(&claws).dmg_pct - (4.0 + 5.0) * 2.5 * 1.2).abs() < 1e-9);
        let bell = Item {
            slot: Slot::Bell,
            rarity: Rarity::Legendary,
            ..it.clone()
        };
        let st = item_stats(&bell);
        assert!((st.aspd_pct - (2.0 + 1.2) * 4.0).abs() < 1e-9);
        assert!((st.crit_pct - 8.0).abs() < 1e-9);
        assert!((st.power_score - (st.aspd_pct + st.crit_pct * 1.5)).abs() < 1e-9);
        // nâng cấp: 5 * (bậc+1)^2 * mult
        assert_eq!(upgrade_cost(&it), Some((5.0f64 * 1.0 * 1.6).round()));
        let it3 = Item {
            upgrade: 3,
            ..it.clone()
        };
        assert_eq!(upgrade_cost(&it3), Some((5.0f64 * 16.0 * 1.6).round()));
        let it10 = Item {
            upgrade: 10,
            ..it.clone()
        };
        assert_eq!(upgrade_cost(&it10), None);
        // rã: 3 * mult * (1 + upgrade)
        assert_eq!(salvage_value(&it3), (3.0f64 * 1.6 * 4.0).round());
    }

    #[test]
    fn prestige_formula() {
        let f = |m: f64| {
            ((m - PRESTIGE_BASE_GS).powf(PRESTIGE_EXP) / PRESTIGE_DIV)
                .floor()
                .max(1.0)
        };
        assert_eq!(prestige_gain(PRESTIGE_MIN_GS - 1), 0.0);
        assert_eq!(prestige_gain(25), f(25.0));
        assert!(prestige_gain(25) >= 1.0);
        assert_eq!(prestige_gain(60), f(60.0));
        assert_eq!(prestige_gain(140), f(140.0));
    }

    #[test]
    fn soul_costs() {
        assert_eq!(soul_cost(SoulUpgrade::AutoCatnip, 0), Some(5.0));
        assert_eq!(soul_cost(SoulUpgrade::AutoCatnip, 1), None);
        assert_eq!(soul_cost(SoulUpgrade::HeadStart, 5), None);
        assert_eq!(
            soul_cost(SoulUpgrade::FatWallet, 2),
            Some((2.0f64 * 1.4 * 1.4).round())
        );
    }

    #[test]
    fn gs_roundtrip() {
        for gs in 0..200 {
            let (c, z, st) = split_gs(gs);
            assert_eq!(global_stage(c, z, st), gs);
        }
        assert_eq!(global_stage(0, 3, 6), 25);
    }
}
