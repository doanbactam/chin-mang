//! `Game`: điểm vào duy nhất của view. Chữ ký các hàm công khai ở đây là HỢP ĐỒNG (docs/CONTRACT.md).
//! Thân hàm do lõi hiện thực (chia nhỏ vào các module sim/*, data.rs, save.rs…).
#![allow(unused_variables)]
use crate::api::*;

pub struct Game {}

impl Game {
    /// Ván mới (mèo Mướp cấp 1, khu 1 màn 1).
    pub fn new(seed: u64) -> Game { todo!() }
    /// Tiến 1 bước DT.
    pub fn tick(&mut self) { todo!() }
    /// Lệnh của người chơi; có hiệu lực ngay (trước tick kế).
    pub fn command(&mut self, cmd: Command) -> Result<(), CmdError> { todo!() }
    /// Lấy và xóa hàng đợi sự kiện kể từ lần gọi trước.
    pub fn drain_events(&mut self) -> Vec<Event> { todo!() }
    pub fn cheat(&mut self, cheat: Cheat) { todo!() }

    // ── truy vấn cho view (rẻ, gọi mỗi frame được) ──
    pub fn hud(&self) -> Hud { todo!() }
    pub fn cats(&self) -> Vec<CatView> { todo!() }
    pub fn enemies(&self) -> Vec<EnemyView> { todo!() }
    pub fn projectiles(&self) -> Vec<ProjectileView> { todo!() }
    pub fn drops(&self) -> Vec<DropView> { todo!() }
    pub fn carton(&self) -> Option<BoxView> { todo!() }

    // ── truy vấn cho bảng ──
    pub fn cat_info(&self, cat: CatId) -> CatInfo { todo!() }
    pub fn level_quote(&self, cat: CatId, mode: BuyMode) -> LevelQuote { todo!() }
    pub fn inventory(&self) -> Vec<Item> { todo!() }
    pub fn item(&self, id: ItemId) -> Option<Item> { todo!() }
    pub fn item_stats(&self, item: &Item) -> ItemStats { todo!() }
    pub fn upgrade_cost(&self, id: ItemId) -> Option<f64> { todo!() }
    pub fn salvage_value(&self, id: ItemId) -> Option<f64> { todo!() }
    pub fn inventory_cap(&self) -> usize { todo!() }
    pub fn auto_salvage(&self) -> Option<Rarity> { todo!() }
    pub fn soul_shop(&self) -> Vec<SoulShopRow> { todo!() }
    pub fn prestige_preview(&self) -> PrestigePreview { todo!() }
    /// 9 phần tử: dấu ấn mạng 1..9 đã đạt chưa.
    pub fn life_marks(&self) -> [bool; 9] { todo!() }
    pub fn achievements(&self) -> Vec<AchView> { todo!() }
    pub fn next_goal(&self) -> Goal { todo!() }
    pub fn pending_tutorial(&self) -> Option<TutorialId> { todo!() }
    pub fn stats(&self) -> Stats { todo!() }

    // ── lưu / tải / vắng mặt ──
    /// JSON có `version` và `saved_at = now_unix`.
    pub fn save(&self, now_unix: u64) -> String { todo!() }
    pub fn load(json: &str) -> Result<Game, LoadError> { todo!() }
    /// `saved_at` của bản lưu đã tải (0 nếu ván mới).
    pub fn saved_at(&self) -> u64 { todo!() }
    /// Gọi 1 lần sau `load` với `now_unix - saved_at`. None nếu < 60 s.
    pub fn apply_offline(&mut self, elapsed_secs: u64) -> Option<OfflineReport> { todo!() }
    /// Hash tất định của toàn bộ trạng thái (test).
    pub fn state_hash(&self) -> u64 { todo!() }
}
