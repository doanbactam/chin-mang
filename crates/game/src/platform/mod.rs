//! Cửa sổ overlay (SPEC §6): trong suốt, không viền, luôn trên cùng, bám mép dưới vùng làm việc,
//! cao thêm khi mở bảng (mép dưới đứng yên). Nhịp khung hình tiết kiệm (SPEC §5).
//!
//! - Windows: work area qua `windows-sys`, Ghost mode, tự khởi động (registry HKCU Run) — `windows.rs`.
//! - Linux X11: đặt vị trí theo monitor (mép dưới màn hình).
//! - Wayland: client không tự đặt vị trí được; README có window rule Hyprland.
//! - Web: canvas cố định 1280×600, dải ở đáy canvas.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::windows_locale;

use crate::layout::{self, LayoutIn, WorkArea};
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::{CurrentLayout, View};
use crate::world::fx::Fx;
use bevy::prelude::*;
use bevy::window::{
    Monitor, PrimaryMonitor, PrimaryWindow, WindowLevel, WindowOccluded, WindowPosition,
    WindowResolution,
};
use bevy::winit::{UpdateMode, WinitSettings};
use std::time::Duration;

/// Kích thước canvas bản web.
pub const WEB_W: u32 = 1280;
pub const WEB_H: u32 = 600;

/// Khả năng của nền tảng (ẩn mục cài đặt không áp dụng).
#[derive(Resource, Clone, Debug, Default)]
pub struct Caps {
    pub monitors: Vec<String>,
    pub ghost: bool,
    pub autostart: bool,
    pub always_on_top: bool,
    pub quit: bool,
}

pub fn initial_caps() -> Caps {
    let native = !cfg!(target_arch = "wasm32");
    Caps {
        monitors: Vec::new(),
        ghost: cfg!(windows),
        autostart: cfg!(windows),
        always_on_top: native,
        quit: native,
    }
}

/// Cửa sổ chính lúc tạo.
pub fn primary_window(settings: &Settings) -> Window {
    #[allow(unused_mut)]
    let mut w = Window {
        title: "Chín Mạng".into(),
        name: Some("chin-mang".into()),
        transparent: true,
        decorations: false,
        resizable: false,
        focused: false,
        skip_taskbar: true,
        window_level: if settings.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        },
        resolution: WindowResolution::new(1280, 96).with_scale_factor_override(1.0),
        ..default()
    };
    #[cfg(target_os = "linux")]
    {
        w.composite_alpha_mode = bevy::window::CompositeAlphaMode::PreMultiplied;
    }
    #[cfg(target_os = "macos")]
    {
        w.composite_alpha_mode = bevy::window::CompositeAlphaMode::PostMultiplied;
    }
    #[cfg(target_arch = "wasm32")]
    {
        w.canvas = Some("#bevy".into());
        w.fit_canvas_to_parent = false;
        w.prevent_default_event_handling = true;
        w.resolution = WindowResolution::new(WEB_W, WEB_H).with_scale_factor_override(1.0);
        w.focused = true;
    }
    w
}

#[derive(Resource, Default)]
struct Occluded(bool);

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(initial_caps())
            .init_resource::<Occluded>()
            .add_systems(PreUpdate, (update_layout, apply_window).chain())
            .add_systems(Update, frame_pacing);
        #[cfg(windows)]
        app.add_plugins(windows::WindowsPlugin);
    }
}

/// Vùng làm việc của các màn hình (màn chính trước), px vật lý.
fn work_areas(monitors: &Query<(&Monitor, Has<PrimaryMonitor>)>) -> Vec<(String, WorkArea)> {
    let mut v: Vec<(bool, String, WorkArea)> = monitors
        .iter()
        .map(|(m, primary)| {
            let full = WorkArea {
                x: m.physical_position.x,
                y: m.physical_position.y,
                w: m.physical_width,
                h: m.physical_height,
                dpi: m.scale_factor as f32,
            };
            #[cfg(windows)]
            let full = windows::work_area(full).unwrap_or(full);
            (primary, m.name.clone().unwrap_or_default(), full)
        })
        .collect();
    v.sort_by_key(|(p, _, a)| (!*p, a.x, a.y));
    v.into_iter().map(|(_, n, a)| (n, a)).collect()
}

fn update_layout(
    mut commands: Commands,
    settings: Res<Settings>,
    view: Res<View>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
    mut caps: ResMut<Caps>,
    current: Option<Res<CurrentLayout>>,
) {
    let fixed = cfg!(target_arch = "wasm32");
    let work = if fixed {
        WorkArea {
            x: 0,
            y: 0,
            w: WEB_W,
            h: WEB_H,
            dpi: 1.0,
        }
    } else {
        let areas = work_areas(&monitors);
        let names: Vec<String> = areas.iter().map(|(n, _)| n.clone()).collect();
        if caps.monitors != names {
            caps.monitors = names;
        }
        match areas.get(settings.monitor).or(areas.first()) {
            Some((_, a)) => *a,
            // chưa có thông tin màn hình (frame đầu): giả định 1920×1080
            None => WorkArea {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
                dpi: 1.0,
            },
        }
    };
    let out = layout::compute(&LayoutIn {
        work,
        scale_setting: settings.scale,
        width_pct: settings.width_pct,
        edge: settings.edge,
        panel_open: view.panel.is_some(),
        fixed_window: fixed,
    });
    if current.is_none_or(|c| c.0 != out) {
        commands.insert_resource(CurrentLayout(out));
    }
}

fn apply_window(
    layout: Option<Res<CurrentLayout>>,
    settings: Res<Settings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let Some(layout) = layout else { return };
    let l = layout.0;
    if !cfg!(target_arch = "wasm32") {
        let pos = WindowPosition::At(IVec2::new(l.window_pos.0, l.window_pos.1));
        if window.position != pos {
            window.position = pos;
        }
        let (w, h) = l.window_size;
        if window.resolution.physical_width() != w || window.resolution.physical_height() != h {
            window.resolution.set_physical_resolution(w, h);
        }
        let level = if settings.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        if window.window_level != level {
            window.window_level = level;
        }
    }
}

/// Số khung hình/giây mong muốn (thuần): bị che → 2; có hoạt động → giới hạn cài đặt; chỉ đi đường → 10.
pub fn target_fps(limit: u32, occluded: bool, active: bool) -> u32 {
    if occluded {
        2
    } else if active {
        limit.max(10)
    } else {
        10.min(limit)
    }
}

fn frame_pacing(
    mut winit: ResMut<WinitSettings>,
    mut occ: ResMut<Occluded>,
    mut events: MessageReader<WindowOccluded>,
    settings: Res<Settings>,
    view: Res<View>,
    sim: Option<Res<Sim>>,
    fx: Res<Fx>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    for e in events.read() {
        occ.0 = e.occluded;
    }
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let busy = sim.is_some_and(|s| {
        let h = s.game.hud();
        !s.game.enemies().is_empty()
            || s.game.carton().is_some()
            || h.catnip_active > 0.0
            || !s.game.drops().is_empty()
    });
    let active = busy
        || view.panel.is_some()
        || !fx.effects.is_empty()
        || !fx.numbers.is_empty()
        || window.cursor_position().is_some();
    let fps = target_fps(settings.fps_limit, occ.0, active);
    let mode = UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / fps as f64));
    if winit.focused_mode != mode {
        winit.focused_mode = mode;
        winit.unfocused_mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fps_policy() {
        assert_eq!(target_fps(30, false, true), 30);
        assert_eq!(target_fps(30, false, false), 10);
        assert_eq!(target_fps(20, true, true), 2);
        assert_eq!(target_fps(10, false, true), 10);
    }
}
