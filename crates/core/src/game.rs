//! `Game`: điểm vào duy nhất của view. Chữ ký các hàm công khai ở đây là HỢP ĐỒNG (docs/CONTRACT.md).
//! Thân hàm do lõi hiện thực (chia nhỏ vào các module sim/*, data.rs, save.rs…).
use crate::api::*;
use crate::data::*;
use crate::rules;
use crate::save;
use crate::state::*;

pub struct Game {
    pub(crate) s: State,
    pub(crate) ev: Vec<Event>,
}

impl Game {
    /// Ván mới (mèo Mướp cấp 1, khu 1 màn 1).
    pub fn new(seed: u64) -> Game {
        let mut s = State {
            seed,
            rng: crate::rng::Rng::new(seed),
            ..State::default()
        };
        s.box_timer = s.rng.range(BOX_MIN, BOX_MAX);
        let mut g = Game { s, ev: Vec::new() };
        g.unlock_cat(CatId::Muop);
        g.ev.clear();
        g.start_stage();
        g
    }
    /// Tiến 1 bước DT.
    pub fn tick(&mut self) {
        self.step();
    }
    /// Lệnh của người chơi; có hiệu lực ngay (trước tick kế).
    pub fn command(&mut self, cmd: Command) -> Result<(), CmdError> {
        let r = self.exec(cmd);
        if r.is_ok() {
            self.check_progress();
        }
        r
    }
    /// Lấy và xóa hàng đợi sự kiện kể từ lần gọi trước.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.ev)
    }
    pub fn cheat(&mut self, cheat: Cheat) {
        self.exec_cheat(cheat);
        self.check_progress();
    }

    // ── truy vấn cho view (rẻ, gọi mỗi frame được) ──
    pub fn hud(&self) -> Hud {
        let s = &self.s;
        let boss = if s.boss_entity != 0 {
            s.enemies
                .iter()
                .find(|e| e.entity == s.boss_entity)
                .map(|b| BossHud {
                    kind: b.kind,
                    hp: b.hp.max(0.0),
                    hp_max: b.hp_max,
                    time_left: s.boss_timer.unwrap_or(BOSS_TIME) as f32,
                })
        } else {
            None
        };
        let wave_progress = if s.wave_total == 0 {
            0.0
        } else {
            (s.wave_killed as f32 / s.wave_total as f32).min(1.0)
        };
        Hud {
            cycle: s.cycle,
            zone: s.zone,
            stage: s.stage,
            global_stage: s.global_stage(),
            max_global_stage: s.max_gs,
            wave_progress,
            boss,
            farming: s.farming,
            fish: s.fish,
            yarn: s.yarn,
            soul: s.soul,
            catnip_charges: s.catnip_charges,
            catnip_cd: (s.catnip_cd / CATNIP_CD) as f32,
            catnip_active: s.catnip_active as f32,
            wake_buff: s.wake_buff as f32,
            any_affordable: self.any_affordable(),
            time: s.time,
            party_x: s.anchor as f32,
        }
    }
    pub fn cats(&self) -> Vec<CatView> {
        let s = &self.s;
        let near = |x: f64, r: f64| {
            s.enemies
                .iter()
                .any(|e| !e.dead && (e.x - x).abs() <= r + 30.0)
        };
        CatId::ALL
            .iter()
            .filter(|&&c| s.cat(c).unlocked)
            .map(|&c| {
                let cs = s.cat(c);
                CatView {
                    cat: c,
                    entity: cat_entity(c),
                    x: cs.x as f32,
                    y: 0.0,
                    facing: cs.facing,
                    anim: cs.anim,
                    anim_t: cs.anim_t as f32,
                    hp: cs.hp.max(0.0),
                    hp_max: cs.d.hp_max,
                    alive: cs.alive,
                    stunned: cs.stun > 0.0,
                    skill_cd: (cs.skill_cd / cat_def(c).skill_cd) as f32,
                    in_combat: cs.alive && (near(cs.x, cat_def(c).range) || cs.hp < cs.d.hp_max),
                }
            })
            .collect()
    }
    pub fn enemies(&self) -> Vec<EnemyView> {
        self.s
            .enemies
            .iter()
            .map(|e| EnemyView {
                entity: e.entity,
                kind: e.kind,
                x: e.x as f32,
                y: if e.kind.flies() {
                    FLY_HEIGHT as f32
                } else {
                    0.0
                },
                facing: e.facing,
                anim: e.anim,
                anim_t: e.anim_t as f32,
                hp: e.hp.max(0.0),
                hp_max: e.hp_max,
                stunned: e.stun > 0.0,
                cursed: e.curse > 0.0,
                poisoned: e.poison_t > 0.0,
            })
            .collect()
    }
    pub fn projectiles(&self) -> Vec<ProjectileView> {
        self.s
            .projs
            .iter()
            .map(|p| ProjectileView {
                entity: p.entity,
                kind: p.kind,
                x: p.x as f32,
                y: p.y as f32,
                vx: p.vx as f32,
                vy: p.vy as f32,
                age: p.age as f32,
            })
            .collect()
    }
    pub fn drops(&self) -> Vec<DropView> {
        let a = self.s.anchor;
        self.s
            .drops
            .iter()
            .map(|d| {
                let k = (d.t / COLLECT_TIME).clamp(0.0, 1.0);
                // nảy lên rồi bay về đầu đội
                let x = d.x0 + (a - 6.0 - d.x0) * k * k;
                let y = d.y0 * (1.0 - k) + 8.0 * k + 14.0 * (std::f64::consts::PI * k).sin();
                DropView {
                    entity: d.entity,
                    kind: d.kind,
                    x: x as f32,
                    y: y as f32,
                    collect_t: k as f32,
                }
            })
            .collect()
    }
    pub fn carton(&self) -> Option<BoxView> {
        self.s.carton.as_ref().map(|b| BoxView {
            entity: b.entity,
            x: (self.s.anchor + BOX_AHEAD) as f32,
            y: b.y as f32,
            time_left: b.time_left.max(0.0) as f32,
            opening: b.opening.map(|t| t as f32),
        })
    }

    // ── truy vấn cho bảng ──
    pub fn cat_info(&self, cat: CatId) -> CatInfo {
        let s = &self.s;
        let cs = s.cat(cat);
        let def = cat_def(cat);
        let d = if cs.unlocked {
            cs.d
        } else {
            rules::derive(s, cat)
        };
        let tal = Talent::of(cat);
        CatInfo {
            cat,
            unlocked: cs.unlocked,
            unlock: UnlockReq::Stage {
                global_stage: def.unlock_gs,
                cost: def.unlock_cost,
            },
            unlock_reachable: s.best_gs >= def.unlock_gs,
            level: cs.level,
            hp_max: d.hp_max,
            dmg: d.dmg,
            aspd: d.aspd,
            crit_pct: d.crit_pct,
            range: def.range as f32,
            dps: rules::est_dps(cat, &d, cs.talents),
            talent_points: rules::talent_points_free(cs),
            talents: [
                (tal[0], cs.talents[0]),
                (tal[1], cs.talents[1]),
                (tal[2], cs.talents[2]),
            ],
            equipped: cs.equipped.clone(),
            skill_cd_secs: def.skill_cd as f32,
        }
    }
    pub fn level_quote(&self, cat: CatId, mode: BuyMode) -> LevelQuote {
        rules::level_quote(&self.s, cat, mode)
    }
    pub fn inventory(&self) -> Vec<Item> {
        self.s.inventory.clone()
    }
    pub fn item(&self, id: ItemId) -> Option<Item> {
        self.s
            .inventory
            .iter()
            .find(|it| it.id == id)
            .cloned()
            .or_else(|| {
                self.s
                    .cats
                    .iter()
                    .flat_map(|c| c.equipped.iter().flatten())
                    .find(|it| it.id == id)
                    .cloned()
            })
    }
    pub fn item_stats(&self, item: &Item) -> ItemStats {
        rules::item_stats(item)
    }
    pub fn upgrade_cost(&self, id: ItemId) -> Option<f64> {
        self.item(id).and_then(|it| rules::upgrade_cost(&it))
    }
    pub fn salvage_value(&self, id: ItemId) -> Option<f64> {
        self.item(id).map(|it| rules::salvage_value(&it))
    }
    pub fn inventory_cap(&self) -> usize {
        INVENTORY_CAP
    }
    pub fn auto_salvage(&self) -> Option<Rarity> {
        self.s.auto_salvage
    }
    pub fn soul_shop(&self) -> Vec<SoulShopRow> {
        SoulUpgrade::ALL
            .iter()
            .map(|&u| {
                let rank = self.s.soul_rank(u);
                let cost = rules::soul_cost(u, rank);
                SoulShopRow {
                    upgrade: u,
                    rank,
                    max_rank: soul_def(u).max_rank,
                    cost,
                    affordable: cost.is_some_and(|c| self.s.soul >= c),
                }
            })
            .collect()
    }
    pub fn prestige_preview(&self) -> PrestigePreview {
        PrestigePreview {
            available: self.s.max_gs >= PRESTIGE_MIN_GS,
            required_global_stage: PRESTIGE_MIN_GS,
            souls_gain: rules::prestige_gain(self.s.max_gs),
            lives_used: self.s.lives_used,
        }
    }
    /// 9 phần tử: dấu ấn mạng 1..9 đã đạt chưa.
    pub fn life_marks(&self) -> [bool; 9] {
        let mut m = [false; 9];
        for (i, v) in m.iter_mut().enumerate() {
            *v = self.s.lives_used > i as u32;
        }
        m
    }
    pub fn achievements(&self) -> Vec<AchView> {
        AchId::ALL
            .iter()
            .enumerate()
            .map(|(k, &id)| AchView {
                id,
                unlocked: self.s.achievements[k].is_some(),
                at: self.s.achievements[k],
            })
            .collect()
    }
    pub fn next_goal(&self) -> Goal {
        let s = &self.s;
        // 1) mèo đã đủ điều kiện màn nhưng chưa mở
        for c in [CatId::Xiem, CatId::Map, CatId::Mun] {
            let def = cat_def(c);
            if !s.cat(c).unlocked && s.best_gs >= def.unlock_gs {
                return Goal::UnlockCat {
                    cat: c,
                    fish_needed: (def.unlock_cost - s.fish).max(0.0),
                };
            }
        }
        // 2) boss của khu hiện tại khi đã gần / đang cày
        if s.farming || s.stage >= STAGES - 1 {
            return Goal::DefeatBoss {
                kind: zone_boss(s.zone),
                zone: s.zone,
            };
        }
        // 3) Chín Mạng
        if s.max_gs >= PRESTIGE_MIN_GS {
            return Goal::Prestige {
                souls: rules::prestige_gain(s.max_gs),
            };
        }
        // 4) màn mở khóa mèo kế tiếp
        for c in [CatId::Xiem, CatId::Map, CatId::Mun] {
            let def = cat_def(c);
            if !s.cat(c).unlocked {
                let (_, zone, stage) = split_gs(def.unlock_gs);
                return Goal::ReachStage { zone, stage };
            }
        }
        // 5) lên cấp mèo rẻ nhất
        let mut best = (CatId::Muop, f64::MAX);
        for c in CatId::ALL {
            if s.cat(c).unlocked {
                let cost = rules::level_cost(c, s.cat(c).level);
                if cost < best.1 {
                    best = (c, cost);
                }
            }
        }
        Goal::LevelCat {
            cat: best.0,
            fish_needed: (best.1 - s.fish).max(0.0),
        }
    }
    pub fn pending_tutorial(&self) -> Option<TutorialId> {
        [
            TutorialId::OpenBag,
            TutorialId::UnlockXiem,
            TutorialId::FirstBox,
        ]
        .into_iter()
        .find(|&t| self.s.tutorial[t as usize] == 1)
    }
    pub fn stats(&self) -> Stats {
        self.s.stats
    }

    // ── lưu / tải / vắng mặt ──
    /// JSON có `version` và `saved_at = now_unix`.
    pub fn save(&self, now_unix: u64) -> String {
        save::save(&self.s, now_unix)
    }
    pub fn load(json: &str) -> Result<Game, LoadError> {
        save::load(json).map(|s| Game { s, ev: Vec::new() })
    }
    /// `saved_at` của bản lưu đã tải (0 nếu ván mới).
    pub fn saved_at(&self) -> u64 {
        self.s.saved_at
    }
    /// Gọi 1 lần sau `load` với `now_unix - saved_at`. None nếu < 60 s.
    pub fn apply_offline(&mut self, elapsed_secs: u64) -> Option<OfflineReport> {
        self.offline(elapsed_secs)
    }
    /// Hash tất định của toàn bộ trạng thái (test).
    pub fn state_hash(&self) -> u64 {
        save::state_hash(&self.s)
    }

    /// Có nâng cấp nào mua được ngay không (chấm đỏ trên nút túi).
    fn any_affordable(&self) -> bool {
        let s = &self.s;
        for c in CatId::ALL {
            let cs = s.cat(c);
            let def = cat_def(c);
            if !cs.unlocked {
                if s.best_gs >= def.unlock_gs && s.fish >= def.unlock_cost {
                    return true;
                }
                continue;
            }
            if s.fish >= rules::level_cost(c, cs.level) {
                return true;
            }
            if rules::talent_points_free(cs) > 0 && cs.talents.iter().any(|&r| r < TALENT_MAX_RANK)
            {
                return true;
            }
            for it in cs.equipped.iter().flatten() {
                if rules::upgrade_cost(it).is_some_and(|y| s.yarn >= y) {
                    return true;
                }
            }
            for slot in Slot::ALL {
                let cur = cs.equipped[slot.index()]
                    .as_ref()
                    .map(rules::power_score)
                    .unwrap_or(-1.0);
                if s.inventory
                    .iter()
                    .any(|it| it.slot == slot && rules::power_score(it) > cur)
                {
                    return true;
                }
            }
        }
        if SoulUpgrade::ALL
            .iter()
            .any(|&u| rules::soul_cost(u, s.soul_rank(u)).is_some_and(|c| s.soul >= c))
        {
            return true;
        }
        s.max_gs >= PRESTIGE_MIN_GS && rules::prestige_gain(s.max_gs) >= 3.0
    }
}
