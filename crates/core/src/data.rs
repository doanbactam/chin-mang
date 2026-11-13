//! Bảng số cân bằng (SPEC §2). Mọi thay đổi số so với SPEC ghi ở `docs/DECISIONS.md`.

use crate::api::{CatId, EnemyKind, SoulUpgrade};

// ── thế giới ──
pub const PARTY_SPEED: f64 = 24.0;
pub const SPAWN_DIST: f64 = 220.0;
pub const ENEMY_GAP: f64 = 18.0;
pub const CAT_SPACING: f64 = 14.0;
pub const ZONES: u32 = 5;
pub const STAGES: u32 = 10;
pub const BOSS_TIME: f64 = 30.0;
/// Đồng hồ boss bắt đầu khi boss vào trong khoảng này tính từ đầu đội.
pub const BOSS_APPEAR_DIST: f64 = 130.0;
pub const FAIL_WAIT: f64 = 3.0;
pub const FARM_RETRY: f64 = 60.0;
pub const COLLECT_TIME: f64 = 0.6;
pub const HEAL_BETWEEN: f64 = 0.25;
pub const REVIVE_HP: f64 = 0.5;
pub const MAX_WAVE: u32 = 7;
/// Mèo đứng Idle quá lâu thì ngủ.
pub const IDLE_TO_SLEEP: f64 = 8.0;

// ── animation (giây) ──
pub const ANIM_HURT: f64 = 0.15;
pub const ANIM_ATTACK_MAX: f64 = 0.36;
pub const ANIM_DEATH_CAT: f64 = 0.6;
pub const ANIM_DEATH_ENEMY: f64 = 0.45;
pub const ANIM_DEATH_BOSS: f64 = 0.8;
pub const ANIM_SKILL: f64 = 0.45;
pub const ANIM_SPECIAL: f64 = 0.6;

// ── mèo ──
pub const LEVEL_COST_GROWTH: f64 = 1.13;
pub const STAT_GROWTH: f64 = 1.10;
pub const LEVELS_PER_TALENT_POINT: u32 = 10;

pub struct CatDef {
    pub range: f64,
    /// khoảng cách mèo tiến tới quái gần nhất rồi dừng (≤ tầm; giữ thứ tự đội hình)
    pub engage: f64,
    pub hp: f64,
    pub dmg: f64,
    pub aspd: f64,
    pub base_cost: f64,
    pub unlock_gs: u32,
    pub unlock_cost: f64,
    pub skill_cd: f64,
    /// thứ tự đội hình (0 = đứng đầu)
    pub order: usize,
}

pub fn cat_def(c: CatId) -> &'static CatDef {
    match c {
        CatId::Muop => &CAT_MUOP,
        CatId::Xiem => &CAT_XIEM,
        CatId::Map => &CAT_MAP,
        CatId::Mun => &CAT_MUN,
    }
}

const CAT_MUOP: CatDef = CatDef {
    range: 14.0,
    engage: 14.0,
    hp: 100.0,
    dmg: 8.0,
    aspd: 1.25,
    base_cost: 10.0,
    unlock_gs: 0,
    unlock_cost: 0.0,
    skill_cd: 6.0,
    order: 1,
};
const CAT_XIEM: CatDef = CatDef {
    range: 90.0,
    engage: 48.0,
    hp: 65.0,
    dmg: 7.0,
    aspd: 1.0,
    base_cost: 20.0,
    unlock_gs: 4,
    unlock_cost: 50.0,
    skill_cd: 9.0,
    order: 2,
};
const CAT_MAP: CatDef = CatDef {
    range: 12.0,
    engage: 12.0,
    hp: 260.0,
    dmg: 4.0,
    aspd: 0.7,
    base_cost: 40.0,
    unlock_gs: 10,
    unlock_cost: 600.0,
    skill_cd: 10.0,
    order: 0,
};
const CAT_MUN: CatDef = CatDef {
    range: 75.0,
    engage: 66.0,
    hp: 60.0,
    dmg: 11.0,
    aspd: 0.5,
    base_cost: 60.0,
    unlock_gs: 20,
    unlock_cost: 2000.0,
    skill_cd: 8.0,
    order: 3,
};

// kỹ năng
pub const MUOP_COMBO_HITS: u32 = 3;
pub const MUOP_COMBO_MULT: f64 = 1.2;
pub const MUOP_COMBO_GAP: f64 = 0.15;
pub const XIEM_RAIN_BONES: u32 = 5;
pub const XIEM_RAIN_MULT: f64 = 0.8;
pub const XIEM_RAIN_AREA: f64 = 40.0;
pub const XIEM_RAIN_HIT: f64 = 10.0;
pub const MAP_SIT_RADIUS: f64 = 20.0;
pub const MAP_SIT_STUN: f64 = 1.5;
pub const MAP_TAUNT: f64 = 3.0;
/// Quái ưu tiên Mập nếu Mập nằm trong tầm + khoảng này.
pub const MAP_AGGRO_BONUS: f64 = 24.0;
pub const MUN_CURSE_BONUS: f64 = 0.30;
pub const MUN_CURSE_SECS: f64 = 5.0;
pub const MUN_AOE_RADIUS: f64 = 18.0;

// tài năng (mỗi bậc)
pub const TALENT_POWER: f64 = 0.08;
pub const TALENT_VIGOR: f64 = 0.10;
pub const TALENT_FRENZY: f64 = 0.06;
pub const TALENT_THORNS: f64 = 0.06;
pub const TALENT_HEX_AOE: f64 = 0.06;
pub const TALENT_HEX_SECS: f64 = 0.5;

// đạn
pub const PROJ_SPEED: f64 = 160.0;
pub const PROJ_ENEMY_SPEED: f64 = 120.0;
pub const PROJ_MAX_AGE: f64 = 2.5;
pub const PROJ_HIT_DIST: f64 = 5.0;
pub const BONE_RAIN_HEIGHT: f64 = 40.0;
pub const BONE_RAIN_VY: f64 = -120.0;

// ── quái ──
pub const ENEMY_SCALE: f64 = 1.15;
/// Sau màn này độ khó tăng chậm hơn (`ENEMY_SCALE_LATE`/màn) để cấp + đồ + hồn theo kịp.
pub const ENEMY_SCALE_KNEE: u32 = 40;
pub const ENEMY_SCALE_LATE: f64 = 1.125;
pub const REWARD_SCALE: f64 = 1.10;
pub const FLY_HEIGHT: f64 = 10.0;
pub const POISON_SECS: f64 = 2.0;
pub const POISON_DPS_MULT: f64 = 0.4;
pub const LIFESTEAL: f64 = 0.5;
pub const BOSS_SPECIAL_EVERY: f64 = 6.0;
/// Boss chỉ đếm hồi đòn đặc biệt khi có mèo trong khoảng này.
pub const BOSS_ENGAGE_DIST: f64 = 80.0;

pub struct EnemyDef {
    pub hp: f64,
    pub dmg: f64,
    pub aspd: f64,
    pub speed: f64,
    pub range: f64,
    pub fish: f64,
}

pub fn enemy_def(k: EnemyKind) -> &'static EnemyDef {
    use EnemyKind::*;
    match k {
        Rat => &EnemyDef {
            hp: 22.0,
            dmg: 3.0,
            aspd: 1.0,
            speed: 20.0,
            range: 10.0,
            fish: 2.0,
        },
        Roach => &EnemyDef {
            hp: 12.0,
            dmg: 2.0,
            aspd: 1.4,
            speed: 34.0,
            range: 8.0,
            fish: 1.0,
        },
        Crow => &EnemyDef {
            hp: 18.0,
            dmg: 4.0,
            aspd: 0.9,
            speed: 28.0,
            range: 10.0,
            fish: 3.0,
        },
        Snake => &EnemyDef {
            hp: 30.0,
            dmg: 3.0,
            aspd: 0.8,
            speed: 16.0,
            range: 12.0,
            fish: 3.0,
        },
        Owl => &EnemyDef {
            hp: 20.0,
            dmg: 5.0,
            aspd: 0.6,
            speed: 14.0,
            range: 70.0,
            fish: 4.0,
        },
        Dog => &EnemyDef {
            hp: 70.0,
            dmg: 6.0,
            aspd: 0.8,
            speed: 18.0,
            range: 12.0,
            fish: 6.0,
        },
        Tick => &EnemyDef {
            hp: 26.0,
            dmg: 4.0,
            aspd: 1.0,
            speed: 22.0,
            range: 8.0,
            fish: 4.0,
        },
        Vacuum => &EnemyDef {
            hp: 140.0,
            dmg: 7.0,
            aspd: 0.5,
            speed: 10.0,
            range: 14.0,
            fish: 10.0,
        },
        RatKing => &EnemyDef {
            hp: 480.0,
            dmg: 9.0,
            aspd: 0.8,
            speed: 14.0,
            range: 16.0,
            fish: 60.0,
        },
        CrowLord => &EnemyDef {
            hp: 720.0,
            dmg: 11.0,
            aspd: 0.9,
            speed: 18.0,
            range: 16.0,
            fish: 90.0,
        },
        Eel => &EnemyDef {
            hp: 560.0,
            dmg: 10.0,
            aspd: 0.7,
            speed: 12.0,
            range: 30.0,
            fish: 120.0,
        },
        Mastiff => &EnemyDef {
            hp: 900.0,
            dmg: 14.0,
            aspd: 0.7,
            speed: 14.0,
            range: 16.0,
            fish: 160.0,
        },
        VacuumTitan => &EnemyDef {
            hp: 1200.0,
            dmg: 16.0,
            aspd: 0.6,
            speed: 10.0,
            range: 20.0,
            fish: 220.0,
        },
    }
}

pub fn zone_enemies(zone: u32) -> &'static [EnemyKind] {
    use EnemyKind::*;
    match zone {
        1 => &[Rat, Roach],
        2 => &[Crow, Roach, Rat],
        3 => &[Snake, Rat, Owl],
        4 => &[Dog, Owl, Tick],
        _ => &[Vacuum, Tick, Dog],
    }
}

pub fn zone_boss(zone: u32) -> EnemyKind {
    use EnemyKind::*;
    match zone {
        1 => RatKing,
        2 => CrowLord,
        3 => Eel,
        4 => Mastiff,
        _ => VacuumTitan,
    }
}

pub fn boss_index(k: EnemyKind) -> Option<usize> {
    use EnemyKind::*;
    match k {
        RatKing => Some(0),
        CrowLord => Some(1),
        Eel => Some(2),
        Mastiff => Some(3),
        VacuumTitan => Some(4),
        _ => None,
    }
}

pub const BOSS_ESCORTS: u32 = 2;
/// Hệ số nhân HP/DMG boss (cân bằng; SPEC gốc = 1.0).
pub const BOSS_HP_MULT: f64 = 0.3;
pub const BOSS_DMG_MULT: f64 = 1.0;
pub const RAT_KING_SUMMONS: u32 = 2;
pub const CROW_LORD_DASH_MULT: f64 = 2.0;
pub const EEL_SHOCK_RADIUS: f64 = 40.0;
pub const EEL_SHOCK_STUN: f64 = 1.0;
pub const EEL_SHOCK_MULT: f64 = 1.0;
pub const MASTIFF_ROAR_ASPD: f64 = 0.30;
pub const MASTIFF_ROAR_SECS: f64 = 4.0;
pub const TITAN_PULL: f64 = 20.0;
pub const TITAN_STUN: f64 = 0.8;

// ── tài nguyên / đồ rơi ──
pub const YARN_DROP_CHANCE: f64 = 0.20;
/// Len mỗi lần rơi = `(1 + floor(gs/10)) * YARN_GROWTH^gs` (SPEC gốc: không có hệ số mũ).
pub const YARN_GROWTH: f64 = 1.10;
pub const BOSS_YARN_MULT: f64 = 5.0;
pub const GEAR_DROP_CHANCE: f64 = 0.03;
pub const RARITY_WEIGHTS: [f64; 4] = [70.0, 22.0, 7.0, 1.0];
pub const INVENTORY_CAP: usize = 40;
pub const ITEM_MAX_UPGRADE: u8 = 10;
pub const CRIT_CAP: f64 = 60.0;
pub const CRIT_MULT: f64 = 2.0;

// ── hộp carton / catnip ──
pub const BOX_MIN: f64 = 180.0;
pub const BOX_MAX: f64 = 360.0;
pub const BOX_LIFE: f64 = 90.0;
pub const BOX_FALL_FROM: f64 = 40.0;
pub const BOX_FALL_SPEED: f64 = 60.0;
pub const BOX_AHEAD: f64 = 60.0;
pub const BOX_OPEN_TIME: f64 = 0.5;
pub const BOX_FISH_SECS: f64 = 60.0;
pub const CATNIP_SECS: f64 = 12.0;
pub const CATNIP_CD: f64 = 120.0;
pub const CATNIP_ASPD: f64 = 0.5;
pub const CATNIP_SPEED: f64 = 0.4;

// ── Chín Mạng ──
pub const PRESTIGE_MIN_GS: u32 = 25;
/// Phải tiến thêm ít nhất chừng này màn so với màn xuất phát của mạng hiện tại mới được prestige.
pub const PRESTIGE_MIN_PROGRESS: u32 = 10;
/// Trần an toàn (chống tràn số khi nhập save sửa tay): vòng và cấp tối đa.
pub const MAX_CYCLE: u32 = 100;
pub const MAX_LEVEL: u32 = 5000;
pub const PRESTIGE_BASE_GS: f64 = 20.0;
pub const PRESTIGE_EXP: f64 = 1.5;
pub const PRESTIGE_DIV: f64 = 26.0;
pub const PRESTIGE_YARN_KEEP: f64 = 0.25;
pub const LIFE_MARK_FISH: f64 = 0.05;
pub const LIFE_MARKS: u32 = 9;

pub struct SoulDef {
    pub per_rank: f64,
    pub max_rank: Option<u32>,
    pub base: f64,
    pub growth: f64,
}

pub fn soul_def(u: SoulUpgrade) -> &'static SoulDef {
    use SoulUpgrade::*;
    match u {
        ClawSharpen => &SoulDef {
            per_rank: 0.15,
            max_rank: None,
            base: 1.0,
            growth: 1.35,
        },
        ThickFur => &SoulDef {
            per_rank: 0.15,
            max_rank: None,
            base: 1.0,
            growth: 1.35,
        },
        FatWallet => &SoulDef {
            per_rank: 0.20,
            max_rank: None,
            base: 2.0,
            growth: 1.4,
        },
        LongNap => &SoulDef {
            per_rank: 7200.0,
            max_rank: Some(4),
            base: 3.0,
            growth: 2.0,
        },
        AutoCatnip => &SoulDef {
            per_rank: 1.0,
            max_rank: Some(1),
            base: 5.0,
            growth: 1.0,
        },
        HeadStart => &SoulDef {
            per_rank: 5.0,
            max_rank: Some(5),
            base: 4.0,
            growth: 1.8,
        },
        LuckyPaw => &SoulDef {
            per_rank: 0.20,
            max_rank: Some(10),
            base: 2.0,
            growth: 1.5,
        },
    }
}

pub fn soul_index(u: SoulUpgrade) -> usize {
    u as usize
}

// ── vắng mặt ──
pub const OFFLINE_MIN: u64 = 60;
pub const OFFLINE_BASE_CAP: u64 = 8 * 3600;
pub const OFFLINE_MULT: f64 = 0.6;
pub const INCOME_WINDOW: usize = 300;
/// Cần ít nhất chừng này giây dữ liệu thu nhập; ít hơn thì ước tính từ màn hiện tại.
pub const INCOME_MIN_SAMPLES: u32 = 30;
pub const WAKE_BUFF_MAX: f64 = 480.0;
pub const WAKE_FISH_MULT: f64 = 2.0;
