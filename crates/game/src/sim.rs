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

pub const AUTOSAVE_SECS: f32 = 30.0;

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
}

impl Sim {
    pub fn new(game: Game) -> Sim {
        Sim {
            game,
            prev: HashMap::new(),
            events: Vec::new(),
            paused: false,
            persist: true,
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

    pub fn save_now(&self) {
        if !self.persist {
            return;
        }
        let json = self.game.save(save_io::now_unix());
        if let Err(e) = save_io::write(save_io::SAVE_KEY, &json) {
            warn!("không lưu được: {e}");
        }
    }
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
            .add_systems(Update, autosave)
            .add_systems(Last, (save_on_exit, clear_events).chain());
    }
}

/// Tạo `Sim` từ save (nếu có) và áp thưởng vắng mặt.
pub fn load_or_new(seed: u64) -> (Sim, Option<OfflineReport>) {
    let mut report = None;
    let game = match save_io::read(save_io::SAVE_KEY).map(|s| Game::load(&s)) {
        Some(Ok(mut g)) => {
            let now = save_io::now_unix();
            let saved = g.saved_at();
            if saved > 0 && now > saved {
                report = g.apply_offline(now - saved);
            }
            g
        }
        Some(Err(e)) => {
            warn!("save hỏng ({e:?}), bắt đầu ván mới");
            Game::new(seed)
        }
        None => Game::new(seed),
    };
    (Sim::new(game), report)
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

fn autosave(time: Res<Time<Real>>, mut timer: ResMut<Autosave>, sim: Option<Res<Sim>>) {
    if timer.0.tick(time.delta()).just_finished()
        && let Some(sim) = sim
    {
        sim.save_now();
    }
}

fn save_on_exit(
    mut exit: MessageReader<AppExit>,
    mut close: MessageReader<WindowCloseRequested>,
    sim: Option<Res<Sim>>,
) {
    let exiting = exit.read().count() + close.read().count() > 0;
    if exiting && let Some(sim) = sim {
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
