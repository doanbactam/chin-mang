//! Chín Mạng — idle auto-battler mèo trên dải trong suốt bám đáy màn hình. Chỉ dựng App.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
// Ẩn cửa sổ console trên Windows (bản release).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod audio;
mod i18n;
mod layout;
mod platform;
mod save_io;
mod settings;
#[cfg(target_arch = "wasm32")]
mod shots;
mod sim;
mod ui;
mod world;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::winit::WinitSettings;
use settings::Settings;

/// Thư mục asset: cạnh file chạy (bản phát hành) hoặc `assets/` ở gốc repo (khi chạy bằng cargo).
fn asset_path() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let p = dir.join("assets");
            if p.join("art").exists() {
                return p.display().to_string();
            }
        }
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if repo.exists() {
            return repo.display().to_string();
        }
    }
    "assets".into()
}

fn load_settings() -> Settings {
    save_io::read(save_io::SETTINGS_KEY)
        .and_then(|s| Settings::from_json(&s))
        .unwrap_or_else(|| Settings::with_locale(&save_io::system_locale()))
}

/// Lưu cài đặt khi đổi (trừ chế độ chụp ảnh).
fn persist_settings(settings: Res<Settings>, sim: Option<Res<sim::Sim>>) {
    if settings.is_changed() && !settings.is_added() && sim.is_some_and(|s| s.persist) {
        let _ = save_io::write(save_io::SETTINGS_KEY, &settings.to_json());
    }
}

fn main() {
    // Chỉ một bản chạy (2 bản sẽ ghi đè save của nhau). Giữ khóa tới khi thoát.
    #[cfg(not(target_arch = "wasm32"))]
    let _instance_lock = match save_io::acquire_single_instance() {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    #[allow(unused_mut)]
    let mut settings = load_settings();
    let mut app = App::new();

    #[cfg(target_arch = "wasm32")]
    let shot = {
        let p = shots::read_params();
        if p.shot.is_some() {
            settings = Settings::default();
        }
        if let Some(l) = p.lang {
            settings.lang = l;
        }
        if let Some(s) = p.scale {
            settings.scale = s;
        }
        p.shot
    };
    #[cfg(not(target_arch = "wasm32"))]
    let shot: Option<String> = None;

    let winit = if cfg!(target_arch = "wasm32") {
        WinitSettings::continuous()
    } else {
        WinitSettings::desktop_app()
    };

    // trạng thái ván: ảnh chụp (web) hoặc save
    match &shot {
        #[cfg(target_arch = "wasm32")]
        Some(name) => {
            shots::install(&mut app, name, &mut settings);
            app.insert_resource(audio::Sfx::muted());
        }
        _ => {
            let (sim, report, notes) = sim::load_or_new(save_io::now_unix());
            app.insert_resource(sim)
                .insert_resource(sim::Offline { report, shown: 0.0 })
                .insert_resource(sim::Notices(notes));
        }
    }

    app.insert_resource(ClearColor(Color::NONE))
        .insert_resource(winit)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(platform::primary_window(&settings)),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: asset_path(),
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .insert_resource(settings)
        .add_plugins((
            assets::AssetsPlugin,
            sim::SimPlugin,
            platform::PlatformPlugin,
            world::WorldPlugin,
            ui::UiPlugin,
            audio::AudioPlugin,
        ))
        .add_systems(Update, persist_settings)
        .run();
}
