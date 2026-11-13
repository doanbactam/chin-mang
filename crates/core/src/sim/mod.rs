//! Vòng mô phỏng: điều phối tick, luồng màn (bắt đầu / qua / thất bại), bộ đếm giờ.

mod combat;
mod loot;
mod progress;

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::*;

/// Trần hàng đợi sự kiện nếu view/bot không lấy (tránh phình bộ nhớ).
pub const EVENT_CAP: usize = 4096;

impl Game {
    pub(crate) fn emit(&mut self, e: Event) {
        if self.ev.len() < EVENT_CAP {
            self.ev.push(e);
        }
    }

    pub(crate) fn catnip_aspd(&self) -> f64 {
        if self.s.catnip_active > 0.0 {
            1.0 + CATNIP_ASPD
        } else {
            1.0
        }
    }

    pub(crate) fn party_speed(&self) -> f64 {
        PARTY_SPEED
            * if self.s.catnip_active > 0.0 {
                1.0 + CATNIP_SPEED
            } else {
                1.0
            }
    }

    /// Một bước DT.
    pub(crate) fn step(&mut self) {
        self.s.tick += 1;
        self.s.time += DT;
        self.s.stats.play_secs += DT;
        self.timers();
        match self.s.phase {
            Phase::Fight => {
                self.combat();
                self.check_fight_end();
            }
            Phase::Collect => {
                self.walk();
                if self.s.drops.is_empty() {
                    self.clear_stage();
                }
            }
            Phase::Failed { t } => {
                let t = t + DT;
                if t >= FAIL_WAIT {
                    self.after_fail();
                } else {
                    self.s.phase = Phase::Failed { t };
                }
            }
        }
        self.update_projectiles();
        self.update_drops();
        self.update_corpses();
        self.update_box();
        self.update_cat_anims();
        self.update_income();
        if self.s.tick.is_multiple_of(TICKS_PER_SEC as u64) {
            self.check_progress();
        }
    }

    fn timers(&mut self) {
        let s = &mut self.s;
        s.catnip_cd = (s.catnip_cd - DT).max(0.0);
        if s.catnip_active > 0.0 {
            s.catnip_active -= DT;
            if s.catnip_active <= 0.0 {
                s.catnip_active = 0.0;
                self.emit(Event::CatnipEnded);
            }
        }
        let s = &mut self.s;
        s.wake_buff = (s.wake_buff - DT).max(0.0);
        s.taunt = (s.taunt - DT).max(0.0);
        if s.farming {
            s.farm_t += DT;
        }
        if s.soul_rank(SoulUpgrade::AutoCatnip) > 0
            && s.catnip_cd <= 0.0
            && s.catnip_active <= 0.0
            && s.phase == Phase::Fight
            && s.enemies.iter().any(|e| !e.dead)
        {
            self.activate_catnip();
        }
        if let Some(t) = self.s.boss_timer
            && self.s.phase == Phase::Fight
        {
            self.s.boss_timer = Some((t - DT).max(0.0));
        }
    }

    pub(crate) fn activate_catnip(&mut self) {
        self.s.catnip_active = CATNIP_SECS;
        self.s.catnip_cd = CATNIP_CD;
        self.s.stats.catnip_used += 1;
        self.emit(Event::CatnipStarted);
    }

    fn check_fight_end(&mut self) {
        let s = &self.s;
        if s.phase != Phase::Fight {
            return;
        }
        let any_cat = s.cats.iter().any(|c| c.unlocked && c.alive);
        if !any_cat || s.boss_timer.is_some_and(|t| t <= 0.0) {
            self.fail_stage();
            return;
        }
        if s.enemies.iter().all(|e| e.dead) {
            self.s.phase = Phase::Collect;
        }
    }

    /// Bắt đầu (lại) màn hiện tại: sinh đợt quái phía trước đội.
    pub(crate) fn start_stage(&mut self) {
        let gs = self.s.global_stage();
        self.s.max_gs = self.s.max_gs.max(gs);
        self.s.best_gs = self.s.best_gs.max(gs);
        self.s.enemies.retain(|e| e.dead);
        self.s.projs.retain(|p| !p.hostile);
        self.s.wave_killed = 0;
        self.s.boss_timer = None;
        self.s.boss_entity = 0;
        self.s.phase = Phase::Fight;
        let zone = self.s.zone;
        let kinds = zone_enemies(zone);
        let x0 = self.s.anchor + SPAWN_DIST;
        if self.s.is_boss_stage() {
            for i in 0..BOSS_ESCORTS {
                let k = kinds[self.s.rng.index(kinds.len())];
                self.spawn_enemy(k, x0 + i as f64 * ENEMY_GAP, gs);
            }
            let boss = zone_boss(zone);
            let e = self.spawn_enemy(boss, x0 + BOSS_ESCORTS as f64 * ENEMY_GAP + 12.0, gs);
            self.s.boss_entity = e;
            self.s.wave_total = BOSS_ESCORTS + 1;
        } else {
            let n = (3 + self.s.stage / 3).min(MAX_WAVE);
            for i in 0..n {
                let k = kinds[self.s.rng.index(kinds.len())];
                self.spawn_enemy(k, x0 + i as f64 * ENEMY_GAP, gs);
            }
            self.s.wave_total = n;
        }
    }

    pub(crate) fn spawn_enemy(&mut self, kind: EnemyKind, x: f64, gs: u32) -> EntityId {
        let def = enemy_def(kind);
        let sc = rules::enemy_scale(gs);
        let (hm, dm) = if kind.is_boss() {
            (BOSS_HP_MULT, BOSS_DMG_MULT)
        } else {
            (1.0, 1.0)
        };
        let entity = self.s.new_entity();
        self.s.enemies.push(Enemy {
            entity,
            kind,
            x,
            hp: def.hp * sc * hm,
            hp_max: def.hp * sc * hm,
            dmg: def.dmg * sc * dm,
            fish: def.fish * rules::reward_scale(gs),
            atk_cd: 0.0,
            stun: 0.0,
            curse: 0.0,
            poison_t: 0.0,
            poison_dps: 0.0,
            special_t: 0.0,
            enrage: 0.0,
            anim: Anim::Walk,
            anim_t: 0.0,
            anim_lock: 0.0,
            facing: -1,
            dying: 0.0,
            dead: false,
            gs,
        });
        entity
    }

    fn clear_stage(&mut self) {
        let (zone, stage) = (self.s.zone, self.s.stage);
        self.emit(Event::StageCleared { zone, stage });
        self.heal_between_stages();
        if self.s.is_boss_stage() {
            self.s.farming = false;
            self.s.farm_t = 0.0;
            self.s.stage = 1;
            if self.s.zone >= ZONES {
                self.s.zone = 1;
                // trần an toàn chống tràn số (không thể đạt khi chơi thật)
                self.s.cycle = (self.s.cycle + 1).min(MAX_CYCLE);
            } else {
                self.s.zone += 1;
            }
            let (zone, cycle) = (self.s.zone, self.s.cycle);
            self.emit(Event::ZoneEntered { zone, cycle });
        } else if self.s.farming && self.s.stage == STAGES - 1 {
            if self.s.farm_t >= FARM_RETRY {
                self.s.farming = false;
                self.s.farm_t = 0.0;
                self.s.stage = STAGES;
            }
        } else {
            self.s.stage += 1;
        }
        self.start_stage();
        self.check_progress();
    }

    fn heal_between_stages(&mut self) {
        for c in CatId::ALL {
            let cs = self.s.cat_mut(c);
            if !cs.unlocked {
                continue;
            }
            if cs.alive {
                cs.hp = (cs.hp + HEAL_BETWEEN * cs.d.hp_max).min(cs.d.hp_max);
            } else {
                self.revive(c, REVIVE_HP);
            }
        }
    }

    pub(crate) fn revive(&mut self, c: CatId, frac: f64) {
        let cs = self.s.cat_mut(c);
        let was_dead = !cs.alive;
        cs.alive = true;
        cs.hp = cs.d.hp_max * frac;
        cs.stun = 0.0;
        cs.poison_t = 0.0;
        cs.combo_left = 0;
        cs.anim = Anim::Idle;
        cs.anim_t = 0.0;
        cs.anim_lock = 0.0;
        cs.idle_t = 0.0;
        if was_dead {
            self.emit(Event::CatRevive { cat: c });
        }
    }

    pub(crate) fn fail_stage(&mut self) {
        let (zone, stage) = (self.s.zone, self.s.stage);
        self.emit(Event::StageFailed { zone, stage });
        self.s.enemies.clear();
        self.s.projs.clear();
        self.s.boss_timer = None;
        self.s.boss_entity = 0;
        if self.s.is_boss_stage() {
            self.s.farming = true;
            self.s.farm_t = 0.0;
            self.s.stage = STAGES - 1;
        } else {
            self.s.stage = (self.s.stage - 1).max(1);
        }
        for cs in self.s.cats.iter_mut().filter(|c| c.unlocked && c.alive) {
            cs.combo_left = 0;
            cs.anim = Anim::Idle;
            cs.anim_t = 0.0;
            cs.anim_lock = 0.0;
        }
        self.s.phase = Phase::Failed { t: 0.0 };
    }

    fn after_fail(&mut self) {
        for c in CatId::ALL {
            if self.s.cat(c).unlocked {
                self.revive(c, 1.0);
            }
        }
        self.start_stage();
    }

    /// Đội đi theo đội hình khi không có quái.
    pub(crate) fn walk(&mut self) {
        let speed = self.party_speed();
        self.s.anchor += speed * DT;
        for c in CatId::ALL {
            if !self.s.cat(c).unlocked {
                continue;
            }
            let slot = rules::formation_slot(&self.s, c);
            let anchor = self.s.anchor;
            let cs = self.s.cat_mut(c);
            if !cs.alive {
                continue;
            }
            let target = anchor - slot as f64 * CAT_SPACING;
            let dx = target - cs.x;
            if dx > 0.01 {
                let catch = if dx > 40.0 { 3.0 } else { 1.6 };
                cs.x += dx.min(speed * catch * DT);
                cs.facing = 1;
                cs.idle_t = 0.0;
                if cs.anim_lock <= 0.0 && cs.anim != Anim::Walk {
                    cs.anim = Anim::Walk;
                    cs.anim_t = 0.0;
                }
            } else if cs.anim_lock <= 0.0 && cs.anim == Anim::Walk {
                cs.anim = Anim::Idle;
                cs.anim_t = 0.0;
            }
        }
    }

    pub(crate) fn update_cat_anims(&mut self) {
        let failed = matches!(self.s.phase, Phase::Failed { .. });
        for cs in self.s.cats.iter_mut().filter(|c| c.unlocked) {
            cs.anim_t += DT;
            if cs.anim_lock > 0.0 {
                cs.anim_lock -= DT;
                if cs.anim_lock <= 0.0 {
                    cs.anim_lock = 0.0;
                    let next = if !cs.alive { Anim::Sleep } else { Anim::Idle };
                    cs.anim = next;
                    cs.anim_t = 0.0;
                }
                continue;
            }
            if !cs.alive {
                if cs.anim != Anim::Sleep && cs.anim != Anim::Death {
                    cs.anim = Anim::Sleep;
                    cs.anim_t = 0.0;
                }
                continue;
            }
            if cs.anim == Anim::Idle {
                cs.idle_t += DT;
                if cs.idle_t >= IDLE_TO_SLEEP || failed && cs.idle_t >= 1.0 {
                    cs.anim = Anim::Sleep;
                    cs.anim_t = 0.0;
                }
            } else if cs.anim != Anim::Sleep {
                cs.idle_t = 0.0;
            }
        }
    }

    fn update_income(&mut self) {
        if !self.s.tick.is_multiple_of(TICKS_PER_SEC as u64) {
            return;
        }
        let s = &mut self.s;
        if s.income_fish.len() != INCOME_WINDOW {
            s.income_fish = vec![0.0; INCOME_WINDOW];
            s.income_yarn = vec![0.0; INCOME_WINDOW];
            s.income_pos = 0;
            s.income_count = 0;
        }
        let p = s.income_pos % INCOME_WINDOW;
        s.income_fish[p] = s.income_cur_fish;
        s.income_yarn[p] = s.income_cur_yarn;
        s.income_pos = (p + 1) % INCOME_WINDOW;
        s.income_count = (s.income_count + 1).min(INCOME_WINDOW as u32);
        s.income_cur_fish = 0.0;
        s.income_cur_yarn = 0.0;
    }

    /// (cá/giây, len/giây) trung bình 300 s gần nhất (hoặc ước tính).
    pub(crate) fn income_rates(&self) -> (f64, f64) {
        let s = &self.s;
        if s.income_count < INCOME_MIN_SAMPLES {
            return (rules::estimate_fish_rate(s), rules::estimate_yarn_rate(s));
        }
        let n = s.income_count as f64;
        let f: f64 = s.income_fish.iter().sum();
        let y: f64 = s.income_yarn.iter().sum();
        (f / n, y / n)
    }

    /// Đặt vị trí mới (cheat / prestige): xóa trận hiện tại và bắt đầu màn.
    pub(crate) fn goto_stage(&mut self, cycle: u32, zone: u32, stage: u32) {
        let old_zone = (self.s.cycle, self.s.zone);
        self.s.cycle = cycle.min(MAX_CYCLE);
        self.s.zone = zone.clamp(1, ZONES);
        self.s.stage = stage.clamp(1, STAGES);
        self.s.farming = false;
        self.s.farm_t = 0.0;
        self.s.enemies.clear();
        self.s.projs.clear();
        for c in CatId::ALL {
            if self.s.cat(c).unlocked {
                let slot = rules::formation_slot(&self.s, c);
                let a = self.s.anchor;
                let cs = self.s.cat_mut(c);
                cs.x = a - slot as f64 * CAT_SPACING;
                cs.combo_left = 0;
            }
        }
        if old_zone != (self.s.cycle, self.s.zone) {
            let (zone, cycle) = (self.s.zone, self.s.cycle);
            self.emit(Event::ZoneEntered { zone, cycle });
        }
        self.start_stage();
    }
}
