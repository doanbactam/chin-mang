//! Chiến đấu: mèo, quái, kỹ năng, đòn đặc biệt của boss, đạn, sát thương.

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::state::*;

/// Độ cao (để đặt số sát thương) của thực thể.
fn enemy_top(k: EnemyKind) -> f64 {
    let base = if k.flies() { FLY_HEIGHT } else { 0.0 };
    base + if k.is_boss() { 28.0 } else { 14.0 }
}

fn enemy_y(k: EnemyKind) -> f64 {
    if k.flies() { FLY_HEIGHT } else { 0.0 }
}

impl Game {
    pub(crate) fn combat(&mut self) {
        if !self.s.enemies.iter().any(|e| !e.dead) {
            self.walk();
            return;
        }
        let mut front = f64::MIN;
        for c in CatId::ALL {
            if self.s.cat(c).unlocked && self.s.cat(c).alive {
                self.cat_act(c);
                front = front.max(self.s.cat(c).x);
            }
        }
        if front > self.s.anchor {
            self.s.anchor = front;
        }
        let n = self.s.enemies.len();
        for i in 0..n {
            self.enemy_act(i);
        }
        // boss xuất hiện → bắt đầu đồng hồ
        if self.s.boss_timer.is_none() && self.s.boss_entity != 0 {
            let be = self.s.boss_entity;
            if let Some(b) = self.s.enemies.iter().find(|e| e.entity == be)
                && b.x - self.s.anchor <= BOSS_APPEAR_DIST
            {
                let kind = b.kind;
                self.s.boss_timer = Some(BOSS_TIME);
                self.emit(Event::BossAppear { kind });
            }
        }
    }

    /// Quái còn sống gần `x` nhất: (index, khoảng cách có dấu).
    fn nearest_enemy(&self, x: f64) -> Option<(usize, f64)> {
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.s.enemies.iter().enumerate() {
            if e.dead {
                continue;
            }
            let dx = e.x - x;
            if best.is_none_or(|(_, b)| dx.abs() < b.abs()) {
                best = Some((i, dx));
            }
        }
        best
    }

    fn enemy_index(&self, entity: EntityId) -> Option<usize> {
        self.s
            .enemies
            .iter()
            .position(|e| e.entity == entity && !e.dead)
    }

    fn roll_crit(&mut self, cat: CatId) -> bool {
        let p = self.s.cat(cat).d.crit_pct / 100.0;
        p > 0.0 && self.s.rng.chance(p)
    }

    fn set_cat_anim(&mut self, c: CatId, anim: Anim, lock: f64) {
        let cs = self.s.cat_mut(c);
        cs.anim = anim;
        cs.anim_t = 0.0;
        cs.anim_lock = lock;
        cs.idle_t = 0.0;
    }

    fn cat_act(&mut self, c: CatId) {
        let def = cat_def(c);
        let aspd_mult = self.catnip_aspd();
        let speed = self.party_speed();
        {
            let cs = self.s.cat_mut(c);
            cs.atk_cd = (cs.atk_cd - DT).max(0.0);
            cs.skill_cd = (cs.skill_cd - DT).max(0.0);
        }
        // độc
        if self.s.cat(c).poison_t > 0.0 {
            let cs = self.s.cat_mut(c);
            cs.poison_t -= DT;
            let dmg = cs.poison_dps * DT;
            self.damage_cat(c, dmg, None, false);
            if !self.s.cat(c).alive {
                return;
            }
        }
        if self.s.cat(c).stun > 0.0 {
            let cs = self.s.cat_mut(c);
            cs.stun -= DT;
            return;
        }
        // chuỗi cào liên hoàn
        if self.s.cat(c).combo_left > 0 {
            let cs = self.s.cat_mut(c);
            cs.combo_t -= DT;
            if cs.combo_t <= 0.0 {
                cs.combo_left -= 1;
                cs.combo_t = MUOP_COMBO_GAP;
                let tgt = cs.combo_target;
                let x = cs.x;
                let idx = self.enemy_index(tgt).or_else(|| {
                    self.nearest_enemy(x)
                        .filter(|&(_, dx)| dx.abs() <= def.range + 4.0)
                        .map(|(i, _)| i)
                });
                if let Some(i) = idx {
                    let crit = self.roll_crit(c);
                    let dmg = self.s.cat(c).d.dmg * MUOP_COMBO_MULT;
                    self.hit_enemy(i, dmg, crit, Some(c));
                } else {
                    self.s.cat_mut(c).combo_left = 0;
                }
            }
            return;
        }
        let x = self.s.cat(c).x;
        let Some((ti, dx)) = self.nearest_enemy(x) else {
            return;
        };
        // không vượt mèo đứng trước trong đội hình
        let my = def.order;
        let ahead = CatId::ALL
            .iter()
            .filter(|&&o| {
                let oc = self.s.cat(o);
                oc.unlocked && oc.alive && cat_def(o).order < my
            })
            .map(|&o| self.s.cat(o).x)
            .fold(f64::MAX, f64::min);
        let cs = self.s.cat_mut(c);
        cs.facing = if dx >= 0.0 { 1 } else { -1 };
        let mut moved = false;
        if dx.abs() > def.engage {
            let mut step = (dx.abs() - def.engage).min(speed * DT) * dx.signum();
            if step > 0.0 && ahead < f64::MAX {
                step = step.min((ahead - 2.0 - cs.x).max(0.0));
            }
            if step.abs() > 1e-9 {
                cs.x += step;
                moved = true;
                cs.idle_t = 0.0;
                if cs.anim_lock <= 0.0 && cs.anim != Anim::Walk {
                    cs.anim = Anim::Walk;
                    cs.anim_t = 0.0;
                }
            }
        }
        let dx = self.s.enemies[ti].x - self.s.cat(c).x;
        if dx.abs() > def.range {
            if !moved {
                let cs = self.s.cat_mut(c);
                if cs.anim_lock <= 0.0 && cs.anim == Anim::Walk {
                    cs.anim = Anim::Idle;
                    cs.anim_t = 0.0;
                }
            }
            return;
        }
        let cs = self.s.cat_mut(c);
        if !moved && cs.anim_lock <= 0.0 && cs.anim == Anim::Walk {
            cs.anim = Anim::Idle;
            cs.anim_t = 0.0;
        }
        // kỹ năng tự động
        if self.s.cat(c).skill_cd <= 0.0 && self.use_skill(c, ti) {
            self.s.cat_mut(c).skill_cd = def.skill_cd;
            return;
        }
        if self.s.cat(c).atk_cd <= 0.0 {
            let aspd = self.s.cat(c).d.aspd * aspd_mult;
            self.s.cat_mut(c).atk_cd = 1.0 / aspd;
            let lock = (0.8 / aspd).min(ANIM_ATTACK_MAX);
            self.set_cat_anim(c, Anim::Attack, lock);
            self.emit(Event::CatAttack { cat: c });
            let crit = self.roll_crit(c);
            let dmg = self.s.cat(c).d.dmg;
            match c {
                CatId::Muop | CatId::Map => self.hit_enemy(ti, dmg, crit, Some(c)),
                CatId::Xiem => {
                    let pierce = (self.s.cat(c).talents[2] as u32).div_ceil(2);
                    self.spawn_cat_proj(c, ProjKind::Fishbone, ti, dmg, crit, pierce, 0.0);
                }
                CatId::Mun => {
                    let hex = self.s.cat(c).talents[2] as f64;
                    let dmg = dmg * (1.0 + TALENT_HEX_AOE * hex);
                    self.spawn_cat_proj(c, ProjKind::Curse, ti, dmg, crit, 0, MUN_AOE_RADIUS);
                }
            }
        }
    }

    /// Dùng kỹ năng; trả false nếu không có mục tiêu hợp lệ.
    fn use_skill(&mut self, c: CatId, ti: usize) -> bool {
        let x = self.s.cat(c).x;
        match c {
            CatId::Muop => {
                let tgt = self.s.enemies[ti].entity;
                let cs = self.s.cat_mut(c);
                cs.combo_left = MUOP_COMBO_HITS;
                cs.combo_t = 0.0;
                cs.combo_target = tgt;
            }
            CatId::Xiem => {
                let tx = self.s.enemies[ti].x;
                let dmg = self.s.cat(c).d.dmg * XIEM_RAIN_MULT;
                for _ in 0..XIEM_RAIN_BONES {
                    let bx = tx
                        + self
                            .s
                            .rng
                            .range(-XIEM_RAIN_AREA / 2.0, XIEM_RAIN_AREA / 2.0);
                    let crit = self.roll_crit(c);
                    let entity = self.s.new_entity();
                    self.s.projs.push(Proj {
                        entity,
                        kind: ProjKind::BoneRain,
                        x: bx,
                        y: BONE_RAIN_HEIGHT + self.s.rng.range(0.0, 12.0),
                        vx: 0.0,
                        vy: BONE_RAIN_VY,
                        age: 0.0,
                        dmg,
                        crit,
                        hostile: false,
                        target: 0,
                        pierce_left: 0,
                        hits: Vec::new(),
                        aoe: 0.0,
                        source: 0,
                    });
                }
            }
            CatId::Map => {
                let mut any = false;
                for i in 0..self.s.enemies.len() {
                    let e = &mut self.s.enemies[i];
                    if !e.dead && (e.x - x).abs() <= MAP_SIT_RADIUS {
                        e.stun = e.stun.max(MAP_SIT_STUN);
                        any = true;
                        let (t, ex, k) = (e.entity, e.x, e.kind);
                        self.emit(Event::Stunned {
                            target: t,
                            x: ex as f32,
                            y: enemy_top(k) as f32,
                        });
                    }
                }
                if !any {
                    return false;
                }
                self.s.taunt = MAP_TAUNT;
            }
            CatId::Mun => {
                let hex = self.s.cat(c).talents[2] as f64;
                let e = &mut self.s.enemies[ti];
                e.curse = e.curse.max(MUN_CURSE_SECS + TALENT_HEX_SECS * hex);
            }
        }
        self.set_cat_anim(c, Anim::Skill, ANIM_SKILL);
        self.emit(Event::CatSkill {
            cat: c,
            x: x as f32,
        });
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_cat_proj(
        &mut self,
        c: CatId,
        kind: ProjKind,
        ti: usize,
        dmg: f64,
        crit: bool,
        pierce: u32,
        aoe: f64,
    ) {
        let cs = self.s.cat(c);
        let tx = self.s.enemies[ti].x;
        let dir = if tx >= cs.x { 1.0 } else { -1.0 };
        let x = cs.x + 4.0 * dir;
        let target = self.s.enemies[ti].entity;
        let entity = self.s.new_entity();
        self.s.projs.push(Proj {
            entity,
            kind,
            x,
            y: 8.0,
            vx: PROJ_SPEED * dir,
            vy: 0.0,
            age: 0.0,
            dmg,
            crit,
            hostile: false,
            target,
            pierce_left: pierce,
            hits: Vec::new(),
            aoe,
            source: 0,
        });
    }

    /// Mèo gây sát thương lên quái `i`.
    pub(crate) fn hit_enemy(&mut self, i: usize, raw: f64, crit: bool, _by: Option<CatId>) {
        let e = &mut self.s.enemies[i];
        if e.dead {
            return;
        }
        let mut amt = raw * if crit { CRIT_MULT } else { 1.0 };
        if e.curse > 0.0 {
            amt *= 1.0 + MUN_CURSE_BONUS;
        }
        e.hp -= amt;
        if e.anim_lock <= 0.0 {
            e.anim = Anim::Hurt;
            e.anim_t = 0.0;
            e.anim_lock = ANIM_HURT;
        }
        let (target, x, y) = (e.entity, e.x, enemy_top(e.kind));
        let dead = e.hp <= 0.0;
        self.emit(Event::Damage {
            target,
            x: x as f32,
            y: y as f32,
            amount: amt,
            crit,
            to_cat: false,
        });
        if dead {
            self.kill_enemy(i);
        }
    }

    fn kill_enemy(&mut self, i: usize) {
        let e = &mut self.s.enemies[i];
        e.hp = 0.0;
        e.dead = true;
        let boss = e.kind.is_boss();
        e.dying = if boss {
            ANIM_DEATH_BOSS
        } else {
            ANIM_DEATH_ENEMY
        };
        e.anim = Anim::Death;
        e.anim_t = 0.0;
        e.anim_lock = 0.0;
        let (entity, kind, x) = (e.entity, e.kind, e.x);
        self.s.wave_killed += 1;
        self.s.stats.enemies_killed += 1;
        self.emit(Event::EnemyDied {
            entity,
            kind,
            x: x as f32,
            y: enemy_y(kind) as f32,
        });
        if boss {
            self.s.stats.bosses_killed += 1;
            if let Some(b) = boss_index(kind) {
                self.s.bosses_beaten[b] = true;
            }
            self.emit(Event::BossDefeated { kind });
        }
        self.spawn_loot(i);
    }

    /// Quái chọn mục tiêu: Mập (khiêu khích / trong tầm ưu tiên) hoặc mèo gần nhất.
    fn enemy_target(&self, i: usize) -> Option<CatId> {
        let e = &self.s.enemies[i];
        let range = enemy_def(e.kind).range;
        let map = self.s.cat(CatId::Map);
        if map.unlocked
            && map.alive
            && (self.s.taunt > 0.0 || (map.x - e.x).abs() <= range + MAP_AGGRO_BONUS)
        {
            return Some(CatId::Map);
        }
        let mut best: Option<(CatId, f64)> = None;
        for c in CatId::ALL {
            let cs = self.s.cat(c);
            if !cs.unlocked || !cs.alive {
                continue;
            }
            let d = (cs.x - e.x).abs();
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((c, d));
            }
        }
        best.map(|(c, _)| c)
    }

    fn enemy_act(&mut self, i: usize) {
        if self.s.enemies[i].dead {
            return;
        }
        {
            let e = &mut self.s.enemies[i];
            e.atk_cd = (e.atk_cd - DT).max(0.0);
            e.curse = (e.curse - DT).max(0.0);
            e.enrage = (e.enrage - DT).max(0.0);
            e.anim_t += DT;
            if e.anim_lock > 0.0 {
                e.anim_lock -= DT;
                // hurt / special là animation một lần: hết khóa thì về tư thế nền (walk)
                if e.anim_lock <= 0.0 && matches!(e.anim, Anim::Hurt | Anim::Special) {
                    e.anim_lock = 0.0;
                    e.anim = Anim::Walk;
                    e.anim_t = 0.0;
                }
            }
        }
        if self.s.enemies[i].poison_t > 0.0 {
            let e = &mut self.s.enemies[i];
            e.poison_t -= DT;
            let d = e.poison_dps * DT;
            self.hit_enemy(i, d, false, None);
            if self.s.enemies[i].dead {
                return;
            }
        }
        if self.s.enemies[i].stun > 0.0 {
            self.s.enemies[i].stun -= DT;
            return;
        }
        let Some(tc) = self.enemy_target(i) else {
            return;
        };
        let kind = self.s.enemies[i].kind;
        let def = enemy_def(kind);
        let cx = self.s.cat(tc).x;
        // boss: đếm hồi đòn đặc biệt khi đang giao chiến
        if kind.is_boss() {
            let ex = self.s.enemies[i].x;
            let engaged = self
                .s
                .cats
                .iter()
                .any(|c| c.unlocked && c.alive && (c.x - ex).abs() <= BOSS_ENGAGE_DIST);
            if engaged {
                self.s.enemies[i].special_t += DT;
                if self.s.enemies[i].special_t >= BOSS_SPECIAL_EVERY {
                    self.s.enemies[i].special_t = 0.0;
                    self.boss_special(i);
                    return;
                }
            }
        }
        let e = &mut self.s.enemies[i];
        let dx = cx - e.x;
        e.facing = if dx >= 0.0 { 1 } else { -1 };
        if dx.abs() > def.range {
            let step = (dx.abs() - def.range).min(def.speed * DT);
            e.x += step * dx.signum();
            if e.anim_lock <= 0.0 && e.anim != Anim::Walk {
                e.anim = Anim::Walk;
                e.anim_t = 0.0;
            }
            return;
        }
        if e.atk_cd > 0.0 {
            return;
        }
        let aspd = def.aspd
            * if e.enrage > 0.0 {
                1.0 + MASTIFF_ROAR_ASPD
            } else {
                1.0
            };
        e.atk_cd = 1.0 / aspd;
        if e.anim_lock <= 0.0 || e.anim == Anim::Hurt {
            e.anim = Anim::Attack;
            e.anim_t = 0.0;
            e.anim_lock = (0.8 / aspd).min(0.4);
        }
        let (entity, dmg, ex) = (e.entity, e.dmg, e.x);
        self.emit(Event::EnemyAttack { entity, kind });
        match kind {
            EnemyKind::Owl | EnemyKind::Eel => {
                let pk = if kind == EnemyKind::Owl {
                    ProjKind::Feather
                } else {
                    ProjKind::Spark
                };
                let dir = if dx >= 0.0 { 1.0 } else { -1.0 };
                let pe = self.s.new_entity();
                self.s.projs.push(Proj {
                    entity: pe,
                    kind: pk,
                    x: ex + 4.0 * dir,
                    y: 10.0,
                    vx: PROJ_ENEMY_SPEED * dir,
                    vy: 0.0,
                    age: 0.0,
                    dmg,
                    crit: false,
                    hostile: true,
                    target: crate::state::cat_entity(tc),
                    pierce_left: 0,
                    hits: Vec::new(),
                    aoe: 0.0,
                    source: entity,
                });
            }
            _ => self.enemy_hits_cat(i, tc, dmg),
        }
    }

    /// Quái `i` đánh trúng mèo (gồm độc, hút máu, phản đòn).
    fn enemy_hits_cat(&mut self, i: usize, c: CatId, dmg: f64) {
        let kind = self.s.enemies[i].kind;
        self.damage_cat(c, dmg, Some(i), true);
        if kind == EnemyKind::Snake {
            let cs = self.s.cat_mut(c);
            if cs.alive {
                cs.poison_t = POISON_SECS;
                cs.poison_dps = POISON_DPS_MULT * dmg;
            }
        }
        if kind == EnemyKind::Tick {
            let e = &mut self.s.enemies[i];
            if !e.dead {
                let heal = (LIFESTEAL * dmg).min(e.hp_max - e.hp).max(0.0);
                e.hp += heal;
                let (target, x) = (e.entity, e.x);
                if heal > 0.0 {
                    self.emit(Event::Heal {
                        target,
                        x: x as f32,
                        y: 14.0,
                        amount: heal,
                    });
                }
            }
        }
    }

    /// Mèo nhận sát thương. `src` = quái gây ra (phản đòn của Mập).
    pub(crate) fn damage_cat(&mut self, c: CatId, amt: f64, src: Option<usize>, hurt_anim: bool) {
        let cs = self.s.cat_mut(c);
        if !cs.alive || amt <= 0.0 {
            return;
        }
        cs.hp -= amt;
        cs.last_hit_t = 0.0;
        if hurt_anim && cs.anim_lock <= 0.0 {
            cs.anim = Anim::Hurt;
            cs.anim_t = 0.0;
            cs.anim_lock = ANIM_HURT;
            cs.idle_t = 0.0;
        }
        let x = cs.x;
        let died = cs.hp <= 0.0;
        let thorns = if c == CatId::Map { cs.talents[2] } else { 0 };
        self.emit(Event::Damage {
            target: cat_entity(c),
            x: x as f32,
            y: 14.0,
            amount: amt,
            crit: false,
            to_cat: true,
        });
        if thorns > 0
            && let Some(i) = src
        {
            self.hit_enemy(i, amt * TALENT_THORNS * thorns as f64, false, Some(c));
        }
        if died {
            let cs = self.s.cat_mut(c);
            cs.hp = 0.0;
            cs.alive = false;
            cs.combo_left = 0;
            cs.poison_t = 0.0;
            cs.stun = 0.0;
            cs.anim = Anim::Death;
            cs.anim_t = 0.0;
            cs.anim_lock = ANIM_DEATH_CAT;
            self.s.stats.deaths += 1;
            self.emit(Event::CatDown { cat: c });
        }
    }

    fn boss_special(&mut self, i: usize) {
        let (kind, bx, dmg, gs) = {
            let e = &mut self.s.enemies[i];
            e.anim = Anim::Special;
            e.anim_t = 0.0;
            e.anim_lock = ANIM_SPECIAL;
            (e.kind, e.x, e.dmg, e.gs)
        };
        self.emit(Event::BossSpecial { kind, x: bx as f32 });
        match kind {
            EnemyKind::RatKing => {
                for k in 0..RAT_KING_SUMMONS {
                    self.spawn_enemy(EnemyKind::Rat, bx + 8.0 + 10.0 * k as f64, gs);
                    self.s.wave_total += 1;
                }
            }
            EnemyKind::CrowLord => {
                // lao tới mèo đứng sau cùng (xa boss nhất)
                let mut tgt: Option<(CatId, f64)> = None;
                for c in CatId::ALL {
                    let cs = self.s.cat(c);
                    if cs.unlocked && cs.alive {
                        let d = (cs.x - bx).abs();
                        if tgt.is_none_or(|(_, b)| d > b) {
                            tgt = Some((c, d));
                        }
                    }
                }
                if let Some((c, _)) = tgt {
                    let cx = self.s.cat(c).x;
                    self.s.enemies[i].x = cx + 8.0;
                    self.enemy_hits_cat(i, c, dmg * CROW_LORD_DASH_MULT);
                }
            }
            EnemyKind::Eel => {
                for c in CatId::ALL {
                    let cs = self.s.cat(c);
                    if cs.unlocked && cs.alive && (cs.x - bx).abs() <= EEL_SHOCK_RADIUS {
                        let x = cs.x;
                        self.damage_cat(c, dmg * EEL_SHOCK_MULT, Some(i), true);
                        let cs = self.s.cat_mut(c);
                        if cs.alive {
                            cs.stun = cs.stun.max(EEL_SHOCK_STUN);
                            self.emit(Event::Stunned {
                                target: cat_entity(c),
                                x: x as f32,
                                y: 14.0,
                            });
                        }
                    }
                }
            }
            EnemyKind::Mastiff => {
                self.s.enemies[i].enrage = MASTIFF_ROAR_SECS;
            }
            EnemyKind::VacuumTitan => {
                for c in CatId::ALL {
                    let cs = self.s.cat_mut(c);
                    if cs.unlocked && cs.alive {
                        let dx = bx - cs.x;
                        let pull = TITAN_PULL.min((dx.abs() - 6.0).max(0.0));
                        cs.x += pull * dx.signum();
                        cs.stun = cs.stun.max(TITAN_STUN);
                        let x = cs.x;
                        self.emit(Event::Stunned {
                            target: cat_entity(c),
                            x: x as f32,
                            y: 14.0,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn update_projectiles(&mut self) {
        let mut k = 0;
        while k < self.s.projs.len() {
            if self.step_proj(k) {
                self.s.projs.swap_remove(k);
            } else {
                k += 1;
            }
        }
    }

    /// Cập nhật một viên đạn; true nếu cần xóa.
    fn step_proj(&mut self, k: usize) -> bool {
        let p = &mut self.s.projs[k];
        p.age += DT;
        if p.age > PROJ_MAX_AGE {
            return true;
        }
        if p.hostile {
            let c = CatId::ALL[(p.target as usize).saturating_sub(1).min(3)];
            let cs = &self.s.cats[c.index()];
            if !cs.alive || !cs.unlocked {
                return true;
            }
            let dx = cs.x - p.x;
            p.vx = PROJ_ENEMY_SPEED * dx.signum();
            p.x += p.vx * DT;
            if (cs.x - p.x).abs() <= PROJ_HIT_DIST {
                let (dmg, src) = (p.dmg, p.source);
                let si = self
                    .s
                    .enemies
                    .iter()
                    .position(|e| e.entity == src && !e.dead);
                match si {
                    Some(i) => self.enemy_hits_cat(i, c, dmg),
                    None => self.damage_cat(c, dmg, None, true),
                }
                return true;
            }
            return false;
        }
        match p.kind {
            ProjKind::BoneRain => {
                p.y += p.vy * DT;
                if p.y > 0.0 {
                    return false;
                }
                let (x, dmg, crit) = (p.x, p.dmg, p.crit);
                if let Some((i, dx)) = self.nearest_enemy(x)
                    && dx.abs() <= XIEM_RAIN_HIT
                {
                    self.hit_enemy(i, dmg, crit, Some(CatId::Xiem));
                }
                true
            }
            ProjKind::Curse => {
                // bám mục tiêu nếu còn sống
                let tgt = p.target;
                if let Some(e) = self.s.enemies.iter().find(|e| e.entity == tgt && !e.dead) {
                    let dx = e.x - p.x;
                    p.vx = PROJ_SPEED * dx.signum();
                }
                p.x += p.vx * DT;
                let (x, dmg, crit, aoe) = (p.x, p.dmg, p.crit, p.aoe);
                let hit = self
                    .s
                    .enemies
                    .iter()
                    .any(|e| !e.dead && (e.x - x).abs() <= PROJ_HIT_DIST);
                if !hit {
                    return false;
                }
                for i in 0..self.s.enemies.len() {
                    let e = &self.s.enemies[i];
                    if !e.dead && (e.x - x).abs() <= aoe {
                        self.hit_enemy(i, dmg, crit, Some(CatId::Mun));
                    }
                }
                true
            }
            _ => {
                p.x += p.vx * DT;
                let x = p.x;
                let mut hit_idx = None;
                for (i, e) in self.s.enemies.iter().enumerate() {
                    if !e.dead
                        && (e.x - x).abs() <= PROJ_HIT_DIST
                        && !self.s.projs[k].hits.contains(&e.entity)
                    {
                        hit_idx = Some(i);
                        break;
                    }
                }
                let Some(i) = hit_idx else {
                    return false;
                };
                let (dmg, crit) = (self.s.projs[k].dmg, self.s.projs[k].crit);
                let ent = self.s.enemies[i].entity;
                self.hit_enemy(i, dmg, crit, Some(CatId::Xiem));
                let p = &mut self.s.projs[k];
                if p.pierce_left == 0 {
                    return true;
                }
                p.pierce_left -= 1;
                p.hits.push(ent);
                false
            }
        }
    }

    /// Đếm ngược animation chết của quái rồi xóa.
    pub(crate) fn update_corpses(&mut self) {
        for e in self.s.enemies.iter_mut().filter(|e| e.dead) {
            e.dying -= DT;
            e.anim_t += DT;
        }
        self.s.enemies.retain(|e| !e.dead || e.dying > 0.0);
    }
}
