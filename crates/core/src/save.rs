//! Lưu / tải JSON có `version` + `saved_at`, migrate, hash trạng thái, vắng mặt.

use serde::{Deserialize, Serialize};

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::{Phase, State, global_stage};

#[derive(Serialize)]
struct SaveOut<'a> {
    version: u32,
    saved_at: u64,
    state: &'a State,
}

#[derive(Deserialize)]
struct SaveIn {
    version: u32,
    #[serde(default)]
    saved_at: u64,
    state: State,
}

pub(crate) fn save(s: &State, now_unix: u64) -> String {
    // không bao giờ ghi NaN/inf (serde_json sẽ ghi `null` → bản lưu hỏng)
    let mut clean;
    let state = if is_sane(s) {
        s
    } else {
        clean = s.clone();
        sanitize(&mut clean);
        &clean
    };
    let out = SaveOut {
        version: SAVE_VERSION,
        saved_at: now_unix,
        state,
    };
    serde_json::to_string(&out).unwrap_or_default()
}

/// Nâng bản lưu cũ lên phiên bản hiện tại (v1 là bản đầu tiên — chưa có bước nào).
fn migrate(v: serde_json::Value, from: u32) -> Result<serde_json::Value, LoadError> {
    match from {
        SAVE_VERSION => Ok(v),
        other => Err(LoadError::Version(other)),
    }
}

pub(crate) fn load(json: &str) -> Result<State, LoadError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| LoadError::Parse(e.to_string()))?;
    let version = v
        .get("version")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| LoadError::Parse("missing version".into()))? as u32;
    if version == 0 || version > SAVE_VERSION {
        return Err(LoadError::Version(version));
    }
    let v = migrate(v, version)?;
    let sf: SaveIn = serde_json::from_value(v).map_err(|e| LoadError::Parse(e.to_string()))?;
    let _ = sf.version;
    let mut s = sf.state;
    s.saved_at = sf.saved_at;
    sanitize(&mut s);
    // bản lưu có ít thành tựu hơn (trước khi thêm fish_1m/fish_1b) → nới mảng
    s.achievements.resize(AchId::ALL.len(), None);
    Ok(s)
}

fn fin(x: &mut f64) {
    if !x.is_finite() {
        *x = 0.0;
    }
}

/// Hữu hạn và không âm (tài nguyên, bộ đếm giờ, máu…).
fn nonneg(x: &mut f64) {
    if !x.is_finite() || *x < 0.0 {
        *x = 0.0;
    }
}

fn all_finite(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Kiểm nhanh: state có hợp lệ không (để `save()` khỏi phải clone khi không cần).
fn is_sane(s: &State) -> bool {
    let mut c = s.clone();
    sanitize(&mut c);
    c == *s
}

/// Đưa state (thường từ save sửa tay / nhập) về miền hợp lệ: vị trí, cấp, bậc trong biên;
/// mọi f64 không hữu hạn → 0, tài nguyên/bộ đếm âm → 0; thực thể trận hỏng bị bỏ.
pub(crate) fn sanitize(s: &mut State) {
    s.cycle = s.cycle.min(MAX_CYCLE);
    s.zone = s.zone.clamp(1, ZONES);
    s.stage = s.stage.clamp(1, STAGES);
    let gs_cap = global_stage(MAX_CYCLE, ZONES, STAGES);
    s.max_gs = s.max_gs.min(gs_cap).max(s.global_stage());
    s.best_gs = s.best_gs.min(gs_cap).max(s.max_gs);
    s.life_start_gs = s.life_start_gs.min(s.max_gs);
    s.wave_killed = s.wave_killed.min(s.wave_total);
    for x in [
        &mut s.time,
        &mut s.farm_t,
        &mut s.taunt,
        &mut s.box_timer,
        &mut s.fish,
        &mut s.yarn,
        &mut s.soul,
        &mut s.souls_earned,
        &mut s.catnip_cd,
        &mut s.catnip_active,
        &mut s.wake_buff,
        &mut s.income_cur_fish,
        &mut s.income_cur_yarn,
        &mut s.stats.play_secs,
        &mut s.stats.fish_earned,
    ] {
        nonneg(x);
    }
    s.catnip_cd = s.catnip_cd.min(CATNIP_CD);
    s.catnip_active = s.catnip_active.min(CATNIP_SECS);
    s.wake_buff = s.wake_buff.min(WAKE_BUFF_MAX);
    s.box_timer = s.box_timer.min(BOX_MAX);
    fin(&mut s.anchor);
    if let Some(t) = s.boss_timer.as_mut() {
        nonneg(t);
        *t = t.min(BOSS_TIME);
    }
    if let Phase::Failed { t } = &mut s.phase {
        fin(t);
    }
    for (i, r) in s.soul_ranks.iter_mut().enumerate() {
        if let Some(m) = soul_def(SoulUpgrade::ALL[i]).max_rank {
            *r = (*r).min(m);
        }
    }
    s.tutorial.iter_mut().for_each(|t| *t = (*t).min(2));
    s.achievements.resize(AchId::ALL.len(), None);
    for a in s.achievements.iter_mut() {
        if a.is_some_and(|t| !t.is_finite()) {
            *a = Some(0.0);
        }
    }
    if s.income_fish.len() != INCOME_WINDOW || s.income_yarn.len() != INCOME_WINDOW {
        s.income_fish = vec![0.0; INCOME_WINDOW];
        s.income_yarn = vec![0.0; INCOME_WINDOW];
        s.income_count = 0;
    }
    s.income_fish.iter_mut().for_each(nonneg);
    s.income_yarn.iter_mut().for_each(nonneg);
    s.income_pos %= INCOME_WINDOW;
    s.income_count = s.income_count.min(INCOME_WINDOW as u32);
    // đồ
    let fix_item = |it: &mut Item| it.upgrade = it.upgrade.min(ITEM_MAX_UPGRADE);
    s.inventory.truncate(INVENTORY_CAP);
    s.inventory.iter_mut().for_each(fix_item);
    let mut max_id = 0;
    for cs in s.cats.iter_mut() {
        for (k, e) in cs.equipped.iter_mut().enumerate() {
            if e.as_ref().is_some_and(|it| it.slot.index() != k) {
                *e = None;
            }
            if let Some(it) = e.as_mut() {
                fix_item(it);
                max_id = max_id.max(it.id);
            }
        }
    }
    for it in &s.inventory {
        max_id = max_id.max(it.id);
    }
    s.next_item = s.next_item.max(max_id + 1);
    // mèo
    let muop = CatId::Muop.index();
    s.cats[muop].unlocked = true;
    for cs in s.cats.iter_mut() {
        cs.level = cs.level.clamp(1, MAX_LEVEL);
        for t in cs.talents.iter_mut() {
            *t = (*t).min(TALENT_MAX_RANK);
        }
        let spent: u32 = cs.talents.iter().map(|&r| r as u32).sum();
        if spent > rules::talent_points_total(cs.level) {
            cs.talents = [0; 3];
        }
        fin(&mut cs.x);
        for x in [
            &mut cs.hp,
            &mut cs.atk_cd,
            &mut cs.skill_cd,
            &mut cs.stun,
            &mut cs.poison_t,
            &mut cs.poison_dps,
            &mut cs.anim_t,
            &mut cs.anim_lock,
            &mut cs.combo_t,
            &mut cs.idle_t,
        ] {
            nonneg(x);
        }
        fin(&mut cs.last_hit_t);
    }
    // thực thể trận: bỏ cái hỏng
    s.enemies.retain(|e| {
        all_finite(&[
            e.x,
            e.hp,
            e.hp_max,
            e.dmg,
            e.fish,
            e.atk_cd,
            e.stun,
            e.curse,
            e.poison_t,
            e.poison_dps,
            e.special_t,
            e.enrage,
            e.anim_t,
            e.anim_lock,
            e.dying,
        ]) && e.hp_max > 0.0
    });
    s.projs
        .retain(|p| all_finite(&[p.x, p.y, p.vx, p.vy, p.age, p.dmg, p.aoe]));
    s.drops
        .retain(|d| all_finite(&[d.x0, d.y0, d.t, d.amount]) && d.amount >= 0.0);
    if s.carton
        .as_ref()
        .is_some_and(|b| !all_finite(&[b.y, b.time_left, b.opening.unwrap_or(0.0)]))
    {
        s.carton = None;
    }
    // chỉ số suy ra: tính lại nếu lệch / hỏng
    let stale = CatId::ALL
        .iter()
        .any(|&c| rules::derive(s, c) != s.cat(c).d);
    if stale {
        for c in CatId::ALL {
            let d = rules::derive(s, c);
            let cs = s.cat_mut(c);
            cs.d = d;
            cs.hp = if cs.alive { cs.hp.min(d.hp_max) } else { 0.0 };
        }
    }
}

/// FNV-1a 64 trên JSON của trạng thái (bỏ `saved_at`).
pub(crate) fn state_hash(s: &State) -> u64 {
    let mut c = s.clone();
    c.saved_at = 0;
    let json = serde_json::to_string(&c).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

impl Game {
    pub(crate) fn offline(&mut self, elapsed_secs: u64) -> Option<OfflineReport> {
        if elapsed_secs < OFFLINE_MIN {
            return None;
        }
        let cap = rules::offline_cap(&self.s);
        let secs = elapsed_secs.min(cap);
        let (fr, yr) = self.income_rates();
        let fish = fr * secs as f64 * OFFLINE_MULT;
        let yarn = yr * secs as f64 * OFFLINE_MULT;
        self.s.fish += fish;
        self.s.stats.fish_earned += fish;
        self.s.yarn += yarn;
        let wake = (secs as f64 / 60.0).min(WAKE_BUFF_MAX);
        self.s.wake_buff = self.s.wake_buff.max(wake);
        if fish > 0.0 {
            let x = self.s.anchor as f32;
            self.emit(Event::Loot {
                currency: Currency::Fish,
                amount: fish,
                x,
            });
        }
        if yarn > 0.0 {
            let x = self.s.anchor as f32;
            self.emit(Event::Loot {
                currency: Currency::Yarn,
                amount: yarn,
                x,
            });
        }
        self.check_progress();
        Some(OfflineReport {
            secs,
            fish,
            yarn,
            wake_buff_secs: wake as f32,
            capped: elapsed_secs > cap,
        })
    }
}
