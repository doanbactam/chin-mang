//! Toàn bộ trạng thái mô phỏng (được serialize nguyên vẹn vào save).

use serde::{Deserialize, Serialize};

use crate::api::*;
use crate::data::*;
use crate::rng::Rng;

/// Chỉ số đã tính sẵn của một mèo (tính lại khi đổi cấp / đồ / tài năng / hồn).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Derived {
    pub hp_max: f64,
    pub dmg: f64,
    pub aspd: f64,
    pub crit_pct: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatState {
    pub unlocked: bool,
    pub level: u32,
    pub talents: [u8; 3],
    pub equipped: [Option<Item>; 3],
    pub x: f64,
    pub hp: f64,
    pub alive: bool,
    pub atk_cd: f64,
    pub skill_cd: f64,
    pub stun: f64,
    pub poison_t: f64,
    pub poison_dps: f64,
    pub anim: Anim,
    pub anim_t: f64,
    /// thời gian còn lại của animation một-lần (attack/hurt/skill/death); hết thì về trạng thái nền
    pub anim_lock: f64,
    pub combo_left: u32,
    pub combo_t: f64,
    pub combo_target: EntityId,
    pub facing: i8,
    pub idle_t: f64,
    pub last_hit_t: f64,
    pub d: Derived,
}

impl Default for CatState {
    fn default() -> Self {
        CatState {
            unlocked: false,
            level: 1,
            talents: [0; 3],
            equipped: [None, None, None],
            x: 0.0,
            hp: 1.0,
            alive: true,
            atk_cd: 0.0,
            skill_cd: 0.0,
            stun: 0.0,
            poison_t: 0.0,
            poison_dps: 0.0,
            anim: Anim::Idle,
            anim_t: 0.0,
            anim_lock: 0.0,
            combo_left: 0,
            combo_t: 0.0,
            combo_target: 0,
            facing: 1,
            idle_t: 0.0,
            last_hit_t: -100.0,
            d: Derived::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Enemy {
    pub entity: EntityId,
    pub kind: EnemyKind,
    pub x: f64,
    pub hp: f64,
    pub hp_max: f64,
    pub dmg: f64,
    pub fish: f64,
    pub atk_cd: f64,
    pub stun: f64,
    pub curse: f64,
    pub poison_t: f64,
    pub poison_dps: f64,
    pub special_t: f64,
    pub enrage: f64,
    pub anim: Anim,
    pub anim_t: f64,
    pub anim_lock: f64,
    pub facing: i8,
    /// > 0: đang chết (chạy animation death), không còn là mục tiêu
    pub dying: f64,
    pub dead: bool,
    pub gs: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proj {
    pub entity: EntityId,
    pub kind: ProjKind,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub age: f64,
    pub dmg: f64,
    pub crit: bool,
    /// true: đạn của quái bắn mèo
    pub hostile: bool,
    pub target: EntityId,
    pub pierce_left: u32,
    pub hits: Vec<EntityId>,
    pub aoe: f64,
    /// quái bắn (để tính hút máu/phản đòn)
    pub source: EntityId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drop {
    pub entity: EntityId,
    pub kind: DropKind,
    pub x0: f64,
    pub y0: f64,
    pub t: f64,
    pub amount: f64,
    pub item: Option<Item>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Carton {
    pub entity: EntityId,
    pub y: f64,
    pub time_left: f64,
    pub opening: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    Fight,
    Collect,
    Failed { t: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub seed: u64,
    pub rng: Rng,
    pub tick: u64,
    pub time: f64,
    pub saved_at: u64,

    // vị trí
    pub cycle: u32,
    pub zone: u32,
    pub stage: u32,
    /// max global_stage trong mạng hiện tại (dùng cho công thức hồn)
    pub max_gs: u32,
    /// max global_stage mọi thời đại (thành tựu)
    pub best_gs: u32,
    pub farming: bool,
    pub farm_t: f64,
    pub phase: Phase,
    pub anchor: f64,
    pub wave_total: u32,
    pub wave_killed: u32,
    pub boss_entity: EntityId,
    /// None = chưa xuất hiện; Some(t) = giây còn lại
    pub boss_timer: Option<f64>,
    pub taunt: f64,

    pub cats: [CatState; 4],
    pub enemies: Vec<Enemy>,
    pub projs: Vec<Proj>,
    pub drops: Vec<Drop>,
    pub carton: Option<Carton>,
    pub box_timer: f64,
    pub next_entity: EntityId,
    pub next_item: ItemId,

    // tài nguyên
    pub fish: f64,
    pub yarn: f64,
    pub soul: f64,
    pub souls_earned: f64,
    pub catnip_charges: u32,
    pub catnip_cd: f64,
    pub catnip_active: f64,
    pub wake_buff: f64,

    pub inventory: Vec<Item>,
    pub auto_salvage: Option<Rarity>,
    pub soul_ranks: [u32; 7],
    pub lives_used: u32,

    pub achievements: [Option<f64>; 24],
    pub bosses_beaten: [bool; 5],
    pub got_legendary: bool,
    pub got_max_upgrade: bool,
    /// 0 = chưa, 1 = đang chờ, 2 = xong
    pub tutorial: [u8; 3],
    pub stats: Stats,

    // thu nhập 300 s gần nhất
    pub income_fish: Vec<f64>,
    pub income_yarn: Vec<f64>,
    pub income_pos: usize,
    pub income_count: u32,
    pub income_cur_fish: f64,
    pub income_cur_yarn: f64,
}

impl Default for State {
    fn default() -> Self {
        State {
            seed: 0,
            rng: Rng::new(0),
            tick: 0,
            time: 0.0,
            saved_at: 0,
            cycle: 0,
            zone: 1,
            stage: 1,
            max_gs: 0,
            best_gs: 0,
            farming: false,
            farm_t: 0.0,
            phase: Phase::Fight,
            anchor: 0.0,
            wave_total: 0,
            wave_killed: 0,
            boss_entity: 0,
            boss_timer: None,
            taunt: 0.0,
            cats: Default::default(),
            enemies: Vec::new(),
            projs: Vec::new(),
            drops: Vec::new(),
            carton: None,
            box_timer: BOX_MIN,
            next_entity: 100,
            next_item: 1,
            fish: 0.0,
            yarn: 0.0,
            soul: 0.0,
            souls_earned: 0.0,
            catnip_charges: 0,
            catnip_cd: 0.0,
            catnip_active: 0.0,
            wake_buff: 0.0,
            inventory: Vec::new(),
            auto_salvage: None,
            soul_ranks: [0; 7],
            lives_used: 0,
            achievements: [None; 24],
            bosses_beaten: [false; 5],
            got_legendary: false,
            got_max_upgrade: false,
            tutorial: [0; 3],
            stats: Stats::default(),
            income_fish: vec![0.0; INCOME_WINDOW],
            income_yarn: vec![0.0; INCOME_WINDOW],
            income_pos: 0,
            income_count: 0,
            income_cur_fish: 0.0,
            income_cur_yarn: 0.0,
        }
    }
}

impl State {
    pub fn global_stage(&self) -> u32 {
        global_stage(self.cycle, self.zone, self.stage)
    }

    pub fn new_entity(&mut self) -> EntityId {
        let e = self.next_entity;
        self.next_entity = self.next_entity.wrapping_add(1).max(100);
        e
    }

    pub fn cat(&self, c: CatId) -> &CatState {
        &self.cats[c.index()]
    }

    pub fn cat_mut(&mut self, c: CatId) -> &mut CatState {
        &mut self.cats[c.index()]
    }

    pub fn soul_rank(&self, u: SoulUpgrade) -> u32 {
        self.soul_ranks[soul_index(u)]
    }

    pub fn is_boss_stage(&self) -> bool {
        self.stage == STAGES
    }
}

pub fn global_stage(cycle: u32, zone: u32, stage: u32) -> u32 {
    (cycle * ZONES + (zone - 1)) * STAGES + (stage - 1)
}

/// (cycle, zone, stage) từ global_stage.
pub fn split_gs(gs: u32) -> (u32, u32, u32) {
    let stage = gs % STAGES + 1;
    let z = gs / STAGES;
    (z / ZONES, z % ZONES + 1, stage)
}

/// Entity id cố định của mèo.
pub fn cat_entity(c: CatId) -> EntityId {
    c.index() as EntityId + 1
}
