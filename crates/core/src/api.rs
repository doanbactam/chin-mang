//! HỢP ĐỒNG giữa lõi (`cm-core`) và view (`chin-mang`). Xem SPEC.md §3.
//!
//! - View chỉ đọc qua các hàm truy vấn của [`Game`] và chỉ thay đổi trạng thái qua [`Game::command`].
//! - Đổi bất kỳ thứ gì công khai trong file này ⇒ cập nhật `docs/CONTRACT.md` cùng lúc.
//! - Tọa độ: pixel logic. `x` tăng sang phải, `y` = độ cao so với mặt đất (0 = đứng trên đất, dương = lên trên).
//! - Mọi chuỗi hiển thị do view dịch (i18n) từ `key()` của enum; lõi không chứa câu chữ hiển thị.

use serde::{Deserialize, Serialize};

/// Bước mô phỏng cố định (giây).
pub const DT: f64 = 1.0 / 30.0;
pub const TICKS_PER_SEC: u32 = 30;
/// Chiều cao dải theo pixel logic.
pub const STRIP_H: u32 = 48;
pub const SAVE_VERSION: u32 = 1;

/// Id thực thể ổn định trong suốt vòng đời của thực thể (view dùng để map sprite).
pub type EntityId = u32;
pub type ItemId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CatId {
    Muop,
    Xiem,
    Map,
    Mun,
}
impl CatId {
    pub const ALL: [CatId; 4] = [CatId::Muop, CatId::Xiem, CatId::Map, CatId::Mun];
    /// "muop" | "xiem" | "map" | "mun" — trùng tên sprite `cat_{key}_…`.
    pub fn key(self) -> &'static str {
        match self {
            CatId::Muop => "muop",
            CatId::Xiem => "xiem",
            CatId::Map => "map",
            CatId::Mun => "mun",
        }
    }
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnemyKind {
    Rat,
    Roach,
    Crow,
    Snake,
    Owl,
    Dog,
    Tick,
    Vacuum,
    RatKing,
    CrowLord,
    Eel,
    Mastiff,
    VacuumTitan,
}
impl EnemyKind {
    pub const ALL: [EnemyKind; 13] = [
        EnemyKind::Rat,
        EnemyKind::Roach,
        EnemyKind::Crow,
        EnemyKind::Snake,
        EnemyKind::Owl,
        EnemyKind::Dog,
        EnemyKind::Tick,
        EnemyKind::Vacuum,
        EnemyKind::RatKing,
        EnemyKind::CrowLord,
        EnemyKind::Eel,
        EnemyKind::Mastiff,
        EnemyKind::VacuumTitan,
    ];
    /// Trùng tên sprite: quái thường `en_{key}_…`, boss `boss_{key}_…`.
    pub fn key(self) -> &'static str {
        match self {
            EnemyKind::Rat => "rat",
            EnemyKind::Roach => "roach",
            EnemyKind::Crow => "crow",
            EnemyKind::Snake => "snake",
            EnemyKind::Owl => "owl",
            EnemyKind::Dog => "dog",
            EnemyKind::Tick => "tick",
            EnemyKind::Vacuum => "vacuum",
            EnemyKind::RatKing => "rat_king",
            EnemyKind::CrowLord => "crow_lord",
            EnemyKind::Eel => "eel",
            EnemyKind::Mastiff => "mastiff",
            EnemyKind::VacuumTitan => "vacuum_titan",
        }
    }
    pub fn is_boss(self) -> bool {
        matches!(
            self,
            EnemyKind::RatKing
                | EnemyKind::CrowLord
                | EnemyKind::Eel
                | EnemyKind::Mastiff
                | EnemyKind::VacuumTitan
        )
    }
    /// Quái bay (crow, crow_lord): view vẽ cao hơn; lõi đã cộng vào `y`.
    pub fn flies(self) -> bool {
        matches!(self, EnemyKind::Crow | EnemyKind::CrowLord)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Slot {
    Collar,
    Bell,
    Claws,
}
impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Collar, Slot::Bell, Slot::Claws];
    /// "collar" | "bell" | "claws" — trùng icon `item_{key}_{rarity}`.
    pub fn key(self) -> &'static str {
        match self {
            Slot::Collar => "collar",
            Slot::Bell => "bell",
            Slot::Claws => "claws",
        }
    }
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}
impl Rarity {
    pub const ALL: [Rarity; 4] = [
        Rarity::Common,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
            Rarity::Legendary => "legendary",
        }
    }
    /// 1 / 1.6 / 2.5 / 4 (SPEC §2.6).
    pub fn mult(self) -> f64 {
        match self {
            Rarity::Common => 1.0,
            Rarity::Rare => 1.6,
            Rarity::Epic => 2.5,
            Rarity::Legendary => 4.0,
        }
    }
}

/// 3 tài năng mỗi mèo: 0 = power, 1 = vigor, 2 = tài năng riêng (frenzy/pierce/thorns/hex).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Talent {
    Power,
    Vigor,
    Frenzy,
    Pierce,
    Thorns,
    Hex,
}
impl Talent {
    pub fn key(self) -> &'static str {
        match self {
            Talent::Power => "power",
            Talent::Vigor => "vigor",
            Talent::Frenzy => "frenzy",
            Talent::Pierce => "pierce",
            Talent::Thorns => "thorns",
            Talent::Hex => "hex",
        }
    }
    /// Ba tài năng của một mèo theo thứ tự ô 0..3.
    pub fn of(cat: CatId) -> [Talent; 3] {
        let special = match cat {
            CatId::Muop => Talent::Frenzy,
            CatId::Xiem => Talent::Pierce,
            CatId::Map => Talent::Thorns,
            CatId::Mun => Talent::Hex,
        };
        [Talent::Power, Talent::Vigor, special]
    }
}
pub const TALENT_MAX_RANK: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SoulUpgrade {
    ClawSharpen,
    ThickFur,
    FatWallet,
    LongNap,
    AutoCatnip,
    HeadStart,
    LuckyPaw,
}
impl SoulUpgrade {
    pub const ALL: [SoulUpgrade; 7] = [
        SoulUpgrade::ClawSharpen,
        SoulUpgrade::ThickFur,
        SoulUpgrade::FatWallet,
        SoulUpgrade::LongNap,
        SoulUpgrade::AutoCatnip,
        SoulUpgrade::HeadStart,
        SoulUpgrade::LuckyPaw,
    ];
    /// Trùng icon `soul_{key}`.
    pub fn key(self) -> &'static str {
        match self {
            SoulUpgrade::ClawSharpen => "claw_sharpen",
            SoulUpgrade::ThickFur => "thick_fur",
            SoulUpgrade::FatWallet => "fat_wallet",
            SoulUpgrade::LongNap => "long_nap",
            SoulUpgrade::AutoCatnip => "auto_catnip",
            SoulUpgrade::HeadStart => "head_start",
            SoulUpgrade::LuckyPaw => "lucky_paw",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Currency {
    Fish,
    Yarn,
    Soul,
    Catnip,
}

/// Animation hiện tại; view chọn frame theo `anim_t` (giây kể từ khi animation bắt đầu).
/// Tên animation trùng tên sprite: idle walk attack hurt death sleep skill special.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Anim {
    Idle,
    Walk,
    Attack,
    Hurt,
    Death,
    Sleep,
    Skill,
    Special,
}
impl Anim {
    pub fn key(self) -> &'static str {
        match self {
            Anim::Idle => "idle",
            Anim::Walk => "walk",
            Anim::Attack => "attack",
            Anim::Hurt => "hurt",
            Anim::Death => "death",
            Anim::Sleep => "sleep",
            Anim::Skill => "skill",
            Anim::Special => "special",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjKind {
    Fishbone,
    BoneRain,
    Curse,
    Feather,
    Spark,
}
impl ProjKind {
    /// Trùng sprite `proj_{key}_…` (bone_rain là 1 frame `proj_bone_rain`).
    pub fn key(self) -> &'static str {
        match self {
            ProjKind::Fishbone => "fishbone",
            ProjKind::BoneRain => "bone_rain",
            ProjKind::Curse => "curse",
            ProjKind::Feather => "feather",
            ProjKind::Spark => "spark",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DropKind {
    Fish,
    Yarn,
    Gear(Rarity),
}

/// Ba chế độ mua cấp ở tab Mèo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuyMode {
    One,
    Ten,
    Max,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TutorialId {
    OpenBag,
    UnlockXiem,
    FirstBox,
}

/// 24 thành tựu (SPEC §2.10). `key()` dùng cho i18n; `group()` chọn icon `ach_{group}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AchId {
    Zone2,
    Zone3,
    Zone4,
    Zone5,
    Cycle1,
    BossRatKing,
    BossCrowLord,
    BossEel,
    BossMastiff,
    BossVacuumTitan,
    UnlockXiem,
    UnlockMap,
    UnlockMun,
    Level25,
    Level50,
    Level100,
    FirstLegendary,
    MaxUpgrade,
    Lives2,
    Lives5,
    Lives9,
    Boxes10,
    Boxes100,
    Catnip50,
}
impl AchId {
    pub const ALL: [AchId; 24] = [
        AchId::Zone2,
        AchId::Zone3,
        AchId::Zone4,
        AchId::Zone5,
        AchId::Cycle1,
        AchId::BossRatKing,
        AchId::BossCrowLord,
        AchId::BossEel,
        AchId::BossMastiff,
        AchId::BossVacuumTitan,
        AchId::UnlockXiem,
        AchId::UnlockMap,
        AchId::UnlockMun,
        AchId::Level25,
        AchId::Level50,
        AchId::Level100,
        AchId::FirstLegendary,
        AchId::MaxUpgrade,
        AchId::Lives2,
        AchId::Lives5,
        AchId::Lives9,
        AchId::Boxes10,
        AchId::Boxes100,
        AchId::Catnip50,
    ];
    pub fn key(self) -> &'static str {
        match self {
            AchId::Zone2 => "zone_2",
            AchId::Zone3 => "zone_3",
            AchId::Zone4 => "zone_4",
            AchId::Zone5 => "zone_5",
            AchId::Cycle1 => "cycle_1",
            AchId::BossRatKing => "boss_rat_king",
            AchId::BossCrowLord => "boss_crow_lord",
            AchId::BossEel => "boss_eel",
            AchId::BossMastiff => "boss_mastiff",
            AchId::BossVacuumTitan => "boss_vacuum_titan",
            AchId::UnlockXiem => "unlock_xiem",
            AchId::UnlockMap => "unlock_map",
            AchId::UnlockMun => "unlock_mun",
            AchId::Level25 => "level_25",
            AchId::Level50 => "level_50",
            AchId::Level100 => "level_100",
            AchId::FirstLegendary => "first_legendary",
            AchId::MaxUpgrade => "max_upgrade",
            AchId::Lives2 => "lives_2",
            AchId::Lives5 => "lives_5",
            AchId::Lives9 => "lives_9",
            AchId::Boxes10 => "boxes_10",
            AchId::Boxes100 => "boxes_100",
            AchId::Catnip50 => "catnip_50",
        }
    }
    /// zone | boss | cat | level | loot | lives | box | wealth  → icon `ach_{group}`.
    pub fn group(self) -> &'static str {
        use AchId::*;
        match self {
            Zone2 | Zone3 | Zone4 | Zone5 | Cycle1 => "zone",
            BossRatKing | BossCrowLord | BossEel | BossMastiff | BossVacuumTitan => "boss",
            UnlockXiem | UnlockMap | UnlockMun => "cat",
            Level25 | Level50 | Level100 => "level",
            FirstLegendary | MaxUpgrade => "loot",
            Lives2 | Lives5 | Lives9 => "lives",
            Boxes10 | Boxes100 | Catnip50 => "box",
        }
    }
}

// ───────────────────────────── Dữ liệu trạng thái cho view ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub slot: Slot,
    pub rarity: Rarity,
    /// `global_stage` lúc rơi.
    pub ilvl: u32,
    /// 0..=10
    pub upgrade: u8,
}

/// Chỉ số đã tính của một món (phần trăm, vd 12.5 = +12.5%).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItemStats {
    pub hp_pct: f64,
    pub dmg_pct: f64,
    pub aspd_pct: f64,
    pub crit_pct: f64,
    /// SPEC §2.6.1
    pub power_score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatView {
    pub cat: CatId,
    pub entity: EntityId,
    pub x: f32,
    pub y: f32,
    /// +1 nhìn phải, -1 nhìn trái.
    pub facing: i8,
    pub anim: Anim,
    pub anim_t: f32,
    pub hp: f64,
    pub hp_max: f64,
    pub alive: bool,
    pub stunned: bool,
    /// 0 = sẵn sàng, 1 = vừa dùng.
    pub skill_cd: f32,
    /// true nếu đang trong trận (thanh máu nên hiện).
    pub in_combat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyView {
    pub entity: EntityId,
    pub kind: EnemyKind,
    pub x: f32,
    pub y: f32,
    pub facing: i8,
    pub anim: Anim,
    pub anim_t: f32,
    pub hp: f64,
    pub hp_max: f64,
    pub stunned: bool,
    pub cursed: bool,
    pub poisoned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileView {
    pub entity: EntityId,
    pub kind: ProjKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropView {
    pub entity: EntityId,
    pub kind: DropKind,
    pub x: f32,
    pub y: f32,
    /// 0..1 tiến độ bay về đội.
    pub collect_t: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxView {
    pub entity: EntityId,
    pub x: f32,
    /// độ cao khi đang rơi (0 = đã chạm đất)
    pub y: f32,
    /// giây còn lại trước khi biến mất
    pub time_left: f32,
    /// Some(t) khi đang mở (0..1 tiến độ animation mở)
    pub opening: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossHud {
    pub kind: EnemyKind,
    pub hp: f64,
    pub hp_max: f64,
    pub time_left: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hud {
    pub cycle: u32,
    /// 1..=5
    pub zone: u32,
    /// 1..=10
    pub stage: u32,
    pub global_stage: u32,
    pub max_global_stage: u32,
    /// 0..1 phần quái của đợt đã bị hạ.
    pub wave_progress: f32,
    pub boss: Option<BossHud>,
    pub farming: bool,
    pub fish: f64,
    pub yarn: f64,
    pub soul: f64,
    pub catnip_charges: u32,
    /// 0 = sẵn sàng, 1 = vừa dùng.
    pub catnip_cd: f32,
    /// giây còn lại của hiệu ứng catnip (0 nếu không hoạt động)
    pub catnip_active: f32,
    /// giây còn lại của buff "vừa ngủ dậy"
    pub wake_buff: f32,
    /// true nếu có bất kỳ nâng cấp nào mua được ngay (chấm đỏ trên nút túi)
    pub any_affordable: bool,
    /// thời gian game (giây)
    pub time: f64,
    /// x để camera bám theo (đầu đội hình)
    pub party_x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnlockReq {
    /// Cần đạt `global_stage` này (đã từng) và trả `cost` cá.
    Stage { global_stage: u32, cost: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatInfo {
    pub cat: CatId,
    pub unlocked: bool,
    pub unlock: UnlockReq,
    /// đã đủ điều kiện stage (chỉ còn thiếu cá hoặc đã đủ)
    pub unlock_reachable: bool,
    pub level: u32,
    pub hp_max: f64,
    pub dmg: f64,
    pub aspd: f64,
    pub crit_pct: f64,
    pub range: f32,
    /// sát thương/giây ước tính (gồm crit, aspd, kỹ năng trung bình)
    pub dps: f64,
    pub talent_points: u32,
    pub talents: [(Talent, u8); 3],
    pub equipped: [Option<Item>; 3],
    pub skill_cd_secs: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelQuote {
    /// số cấp sẽ lên (0 nếu không đủ tiền cho dù 1 cấp; với One/Ten là số cấp yêu cầu nhưng `affordable` = false)
    pub levels: u32,
    pub cost: f64,
    pub affordable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoulShopRow {
    pub upgrade: SoulUpgrade,
    pub rank: u32,
    /// None = không giới hạn
    pub max_rank: Option<u32>,
    /// None khi đã max
    pub cost: Option<f64>,
    pub affordable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrestigePreview {
    pub available: bool,
    pub required_global_stage: u32,
    pub souls_gain: f64,
    pub lives_used: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AchView {
    pub id: AchId,
    pub unlocked: bool,
    /// thời điểm (giây game tổng) đạt được
    pub at: Option<f64>,
}

/// Một mục tiêu gần nhất hiển thị ở thanh "Mục tiêu kế" (view dịch thành câu).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    UnlockCat { cat: CatId, fish_needed: f64 },
    ReachStage { zone: u32, stage: u32 },
    DefeatBoss { kind: EnemyKind, zone: u32 },
    LevelCat { cat: CatId, fish_needed: f64 },
    Prestige { souls: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OfflineReport {
    pub secs: u64,
    pub fish: f64,
    pub yarn: f64,
    pub wake_buff_secs: f32,
    /// true nếu đã chạm trần offline
    pub capped: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub play_secs: f64,
    pub enemies_killed: u64,
    pub bosses_killed: u64,
    pub fish_earned: f64,
    pub boxes_opened: u32,
    pub catnip_used: u32,
    pub deaths: u32,
}

// ───────────────────────────── Lệnh và sự kiện ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    LevelUp {
        cat: CatId,
        mode: BuyMode,
    },
    Unlock {
        cat: CatId,
    },
    TalentUp {
        cat: CatId,
        slot: u8,
    },
    Equip {
        cat: CatId,
        item: ItemId,
    },
    Unequip {
        cat: CatId,
        slot: Slot,
    },
    /// None = cả đội
    EquipBest {
        cat: Option<CatId>,
    },
    UpgradeItem {
        item: ItemId,
    },
    Salvage {
        item: ItemId,
    },
    /// None = tắt tự rã; Some(r) = rã mọi món có độ hiếm ≤ r
    SetAutoSalvage {
        max: Option<Rarity>,
    },
    Catnip,
    OpenBox,
    BuySoul {
        upgrade: SoulUpgrade,
    },
    Prestige,
    RetryBoss,
    DismissTutorial {
        id: TutorialId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdError {
    NotEnough(Currency),
    Locked,
    AlreadyUnlocked,
    NoSuchItem,
    MaxRank,
    NoTalentPoints,
    OnCooldown,
    NothingToDo,
    NotAvailable,
}

/// Sự kiện để view phát âm thanh / hiệu ứng / số nổi. Lấy bằng [`Game::drain_events`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Damage {
        target: EntityId,
        x: f32,
        y: f32,
        amount: f64,
        crit: bool,
        to_cat: bool,
    },
    Heal {
        target: EntityId,
        x: f32,
        y: f32,
        amount: f64,
    },
    CatAttack {
        cat: CatId,
    },
    CatSkill {
        cat: CatId,
        x: f32,
    },
    CatDown {
        cat: CatId,
    },
    CatRevive {
        cat: CatId,
    },
    EnemyAttack {
        entity: EntityId,
        kind: EnemyKind,
    },
    EnemyDied {
        entity: EntityId,
        kind: EnemyKind,
        x: f32,
        y: f32,
    },
    BossAppear {
        kind: EnemyKind,
    },
    BossSpecial {
        kind: EnemyKind,
        x: f32,
    },
    BossDefeated {
        kind: EnemyKind,
    },
    Stunned {
        target: EntityId,
        x: f32,
        y: f32,
    },
    StageCleared {
        zone: u32,
        stage: u32,
    },
    StageFailed {
        zone: u32,
        stage: u32,
    },
    ZoneEntered {
        zone: u32,
        cycle: u32,
    },
    Loot {
        currency: Currency,
        amount: f64,
        x: f32,
    },
    GearDropped {
        item: ItemId,
        rarity: Rarity,
        x: f32,
    },
    LevelUp {
        cat: CatId,
        level: u32,
    },
    CatUnlocked {
        cat: CatId,
    },
    ItemUpgraded {
        item: ItemId,
        upgrade: u8,
    },
    ItemSalvaged {
        item: ItemId,
        yarn: f64,
    },
    BoxSpawned {
        x: f32,
    },
    BoxOpened {
        x: f32,
    },
    BoxExpired,
    CatnipStarted,
    CatnipEnded,
    Prestiged {
        souls: f64,
        lives_used: u32,
    },
    SoulBought {
        upgrade: SoulUpgrade,
        rank: u32,
    },
    Achievement {
        id: AchId,
    },
    Tutorial {
        id: TutorialId,
    },
}

/// Thiết lập nhanh trạng thái cho test / ảnh chụp (`?shot=`) / debug. Không dùng trong gameplay thường.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cheat {
    /// Nhảy tới vị trí (cycle, zone 1..=5, stage 1..=10) và cập nhật max_global_stage nếu cao hơn.
    Goto {
        cycle: u32,
        zone: u32,
        stage: u32,
    },
    Give {
        currency: Currency,
        amount: f64,
    },
    UnlockAll,
    SetLevel {
        cat: CatId,
        level: u32,
    },
    /// Thêm một món vào kho.
    GiveItem {
        slot: Slot,
        rarity: Rarity,
        ilvl: u32,
        upgrade: u8,
    },
    /// Chạy n tick ngay.
    Advance {
        ticks: u32,
    },
    /// Thả hộp carton ngay trước đội.
    SpawnBox,
    UnlockAchievement {
        id: AchId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    Parse(String),
    Version(u32),
}
