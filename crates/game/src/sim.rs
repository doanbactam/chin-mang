//! Lõi mô phỏng trong Bevy: `Resource` bọc `cm_core::Game`, `FixedUpdate` 30 Hz,
//! bù tick khi frame thưa (max_delta của đồng hồ ảo nâng lên 5 s), nội suy vị trí giữa 2 tick,
//! nạp / tự lưu 30 s / lưu khi thoát, áp thưởng vắng mặt.

use crate::save_io;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use cm_core::{Event as CoreEvent, Game, OfflineReport, TICKS_PER_SEC};
use std::collections::HashMap;
use std::time::Duration;

/// Tự lưu mỗi 10 s (tắt máy / log off Windows không qua đường thoát của game, xem DECISIONS).
pub const AUTOSAVE_SECS: f32 = 10.0;
/// Khoảng trống đồng hồ thực giữa hai frame coi như máy đã ngủ (SPEC §2.9: < 60 s bỏ qua).
pub const SLEEP_GAP_SECS: u64 = 60;

#[derive(Resource)]
pub struct Sim {
    pub game: Game,
    /// Vị trí trước tick gần nhất (để nội suy).
    pub prev: HashMap<u32, Vec2>,
    /// Sự kiện từ lõi trong frame này (xóa ở cuối frame).
    pub events: Vec<CoreEvent>,
    /// Dừng mô phỏng (ảnh chụp tất định).
    pub paused: bool,
    /// Không đọc/ghi save (chế độ chụp ảnh).
    pub persist: bool,
    /// Lỗi lưu gần nhất chưa báo cho người chơi.
    pub save_error: Option<String>,
}

impl Sim {
    pub fn new(game: Game) -> Sim {
        Sim {
            game,
            prev: HashMap::new(),
            events: Vec::new(),
            paused: false,
            persist: true,
            save_error: None,
        }
    }

    /// Nội suy vị trí thực thể `id` (giá trị hiện tại `cur`) theo phần dư của bước cố định.
    pub fn lerp(&self, id: u32, cur: Vec2, alpha: f32) -> Vec2 {
        match self.prev.get(&id) {
            Some(p) if p.distance_squared(cur) < 64.0 * 64.0 => p.lerp(cur, alpha.clamp(0.0, 1.0)),
            _ => cur,
        }
    }

    /// Thay ván (nhập save / xóa tiến trình).
    pub fn replace(&mut self, game: Game) {
        self.game = game;
        self.prev.clear();
        self.events.clear();
    }

    /// Lưu ngay (xoay vòng `save.bak`). Lỗi được giữ lại để UI báo "Không lưu được".
    pub fn save_now(&mut self) {
        if !self.persist {
            return;
        }
        let json = self.game.save(save_io::now_unix());
        if let Err(e) = save_io::write_save(&json) {
            warn!("không lưu được: {e}");
            self.save_error = Some(e);
        }
    }
}

/// Thông báo phát sinh trước khi UI sẵn sàng (vd lúc nạp save) — khóa i18n, UI hiện thành toast.
#[derive(Resource, Default)]
pub struct Notices(pub Vec<&'static str>);

/// Kết quả chọn bản save để nạp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavePick {
    /// không có save: ván mới
    Fresh,
    /// save chính đọc được
    Main(String),
    /// save chính hỏng / thiếu, khôi phục từ `save.bak`; `main_corrupt` = bản chính có nhưng hỏng
    Backup { json: String, main_corrupt: bool },
    /// cả hai đều hỏng / thiếu nhưng có ít nhất một bản hỏng: ván mới
    Corrupt {
        main_corrupt: bool,
        bak_corrupt: bool,
    },
}

/// Chọn bản save hợp lệ (thuần, test được). `ok(json)` = tải được.
pub fn pick_save(main: Option<String>, bak: Option<String>, ok: impl Fn(&str) -> bool) -> SavePick {
    let main_corrupt = match main {
        Some(m) if ok(&m) => return SavePick::Main(m),
        Some(_) => true,
        None => false,
    };
    match bak {
        Some(b) if ok(&b) => SavePick::Backup {
            json: b,
            main_corrupt,
        },
        Some(_) => SavePick::Corrupt {
            main_corrupt,
            bak_corrupt: true,
        },
        None if main_corrupt => SavePick::Corrupt {
            main_corrupt,
            bak_corrupt: false,
        },
        None => SavePick::Fresh,
    }
}

/// Gộp hai báo cáo vắng mặt (máy ngủ khi thẻ cũ chưa nhận).
pub fn merge_reports(a: Option<OfflineReport>, b: OfflineReport) -> OfflineReport {
    match a {
        None => b,
        Some(a) => OfflineReport {
            secs: a.secs + b.secs,
            fish: a.fish + b.fish,
            yarn: a.yarn + b.yarn,
            wake_buff_secs: a.wake_buff_secs.max(b.wake_buff_secs),
            capped: a.capped || b.capped,
        },
    }
}

/// Khoảng trống đồng hồ thực (giây) đủ dài để tính là máy đã ngủ.
pub fn sleep_gap(last_unix: u64, now_unix: u64) -> Option<u64> {
    let gap = now_unix.saturating_sub(last_unix);
    (last_unix > 0 && gap >= SLEEP_GAP_SECS).then_some(gap)
}

/// Báo cáo vắng mặt chờ người chơi nhận (SPEC §7.4).
#[derive(Resource, Default)]
pub struct Offline {
    pub report: Option<OfflineReport>,
    /// giây đã hiển thị (tự nhận sau 10 s)
    pub shown: f32,
}

#[derive(Resource)]
struct Autosave(Timer);

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let mut virt = Time::<Virtual>::default();
        virt.set_max_delta(Duration::from_secs(5));
        app.insert_resource(virt)
            .insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SEC as f64))
            .init_resource::<Offline>()
            .insert_resource(Autosave(Timer::from_seconds(
                AUTOSAVE_SECS,
                TimerMode::Repeating,
            )))
            .add_systems(FixedUpdate, step)
            .init_resource::<Notices>()
            .init_resource::<WallClock>()
            .add_systems(Update, (wake_from_sleep, autosave).chain())
            .add_systems(Last, (save_on_exit, clear_events).chain());
    }
}

/// Tạo `Sim` từ save (thử `save.bak` khi bản chính hỏng; bản hỏng được cách ly) và áp thưởng vắng mặt.
pub fn load_or_new(seed: u64) -> (Sim, Option<OfflineReport>, Vec<&'static str>) {
    let mut notes = Vec::new();
    let now = save_io::now_unix();
    let pick = pick_save(
        save_io::read(save_io::SAVE_KEY),
        save_io::read(save_io::BAK_KEY),
        |j| Game::load(j).is_ok(),
    );
    let (json, main_corrupt) = match pick {
        SavePick::Fresh => (None, false),
        SavePick::Main(j) => (Some(j), false),
        SavePick::Backup { json, main_corrupt } => {
            notes.push("toast_save_restored");
            (Some(json), main_corrupt)
        }
        SavePick::Corrupt {
            main_corrupt,
            bak_corrupt,
        } => {
            notes.push("toast_save_corrupt");
            if bak_corrupt {
                save_io::quarantine(save_io::BAK_KEY, now);
            }
            (None, main_corrupt)
        }
    };
    if main_corrupt {
        save_io::quarantine(save_io::SAVE_KEY, now);
    }
    let mut report = None;
    let game = match json.map(|j| Game::load(&j)) {
        Some(Ok(mut g)) => {
            let saved = g.saved_at();
            if saved > 0 && now > saved {
                report = g.apply_offline(now - saved);
            }
            g
        }
        _ => Game::new(seed),
    };
    (Sim::new(game), report, notes)
}

/// Mốc đồng hồ thực của frame trước (phát hiện máy ngủ).
#[derive(Resource, Default)]
struct WallClock(u64);

/// Máy ngủ/treo khi game đang mở: khoảng trống ≥ 60 s ⇒ thưởng vắng mặt (chỉ thưởng) + thẻ tóm tắt + lưu.
fn wake_from_sleep(
    mut clock: ResMut<WallClock>,
    sim: Option<ResMut<Sim>>,
    mut offline: ResMut<Offline>,
) {
    let now = save_io::now_unix();
    let last = std::mem::replace(&mut clock.0, now);
    let Some(mut sim) = sim else { return };
    if sim.paused || !sim.persist {
        return;
    }
    if let Some(gap) = sleep_gap(last, now)
        && let Some(r) = sim.game.apply_offline(gap)
    {
        offline.report = Some(merge_reports(offline.report.take(), r));
        offline.shown = 0.0;
        sim.save_now();
    }
}

fn step(sim: Option<ResMut<Sim>>) {
    let Some(mut sim) = sim else { return };
    if sim.paused {
        return;
    }
    let Sim {
        game, prev, events, ..
    } = &mut *sim;
    prev.clear();
    for c in game.cats() {
        prev.insert(c.entity, Vec2::new(c.x, c.y));
    }
    for e in game.enemies() {
        prev.insert(e.entity, Vec2::new(e.x, e.y));
    }
    for p in game.projectiles() {
        prev.insert(p.entity, Vec2::new(p.x, p.y));
    }
    for d in game.drops() {
        prev.insert(d.entity, Vec2::new(d.x, d.y));
    }
    if let Some(b) = game.carton() {
        prev.insert(b.entity, Vec2::new(b.x, b.y));
    }
    prev.insert(u32::MAX, Vec2::new(game.hud().party_x, 0.0));
    game.tick();
    events.extend(game.drain_events());
}

fn autosave(time: Res<Time<Real>>, mut timer: ResMut<Autosave>, sim: Option<ResMut<Sim>>) {
    if timer.0.tick(time.delta()).just_finished()
        && let Some(mut sim) = sim
    {
        sim.save_now();
    }
}

fn save_on_exit(
    mut exit: MessageReader<AppExit>,
    mut close: MessageReader<WindowCloseRequested>,
    sim: Option<ResMut<Sim>>,
) {
    let exiting = exit.read().count() + close.read().count() > 0;
    if exiting && let Some(mut sim) = sim {
        sim.save_now();
    }
}

fn clear_events(sim: Option<ResMut<Sim>>) {
    if let Some(mut sim) = sim
        && !sim.events.is_empty()
    {
        sim.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(j: &str) -> bool {
        j.starts_with('{')
    }

    #[test]
    fn picks_main_then_backup() {
        assert_eq!(pick_save(None, None, ok), SavePick::Fresh);
        assert_eq!(
            pick_save(Some("{a".into()), Some("{b".into()), ok),
            SavePick::Main("{a".into())
        );
        assert_eq!(
            pick_save(Some("xx".into()), Some("{b".into()), ok),
            SavePick::Backup {
                json: "{b".into(),
                main_corrupt: true
            }
        );
        assert_eq!(
            pick_save(None, Some("{b".into()), ok),
            SavePick::Backup {
                json: "{b".into(),
                main_corrupt: false
            }
        );
        assert_eq!(
            pick_save(Some("xx".into()), None, ok),
            SavePick::Corrupt {
                main_corrupt: true,
                bak_corrupt: false
            }
        );
        assert_eq!(
            pick_save(Some("xx".into()), Some("yy".into()), ok),
            SavePick::Corrupt {
                main_corrupt: true,
                bak_corrupt: true
            }
        );
    }

    #[test]
    fn sleep_gap_threshold() {
        assert_eq!(sleep_gap(0, 1000), None);
        assert_eq!(sleep_gap(1000, 1059), None);
        assert_eq!(sleep_gap(1000, 1060), Some(60));
        assert_eq!(sleep_gap(1000, 900), None);
    }

    #[test]
    fn reports_merge_rewards_only() {
        let a = OfflineReport {
            secs: 100,
            fish: 5.0,
            yarn: 1.0,
            wake_buff_secs: 30.0,
            capped: false,
        };
        let b = OfflineReport {
            secs: 200,
            fish: 7.0,
            yarn: 2.0,
            wake_buff_secs: 10.0,
            capped: true,
        };
        let m = merge_reports(Some(a), b);
        assert_eq!(
            (m.secs, m.fish, m.yarn, m.wake_buff_secs, m.capped),
            (300, 12.0, 3.0, 30.0, true)
        );
        assert_eq!(merge_reports(None, b), b);
    }
}
