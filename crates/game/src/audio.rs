//! Âm thanh: phát sfx theo `Event` của lõi + tiếng UI, nhạc nền (tắt mặc định), âm lượng theo cài đặt,
//! giới hạn số âm cùng lúc và chống lặp cùng một âm trong thời gian ngắn.

use crate::assets::Sounds;
use crate::settings::Settings;
use crate::sim::Sim;
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use cm_core::{Currency, Event as CoreEvent, Rarity};
use std::collections::HashMap;

/// Tối đa số sfx đang phát cùng lúc.
pub const MAX_VOICES: usize = 6;
/// Khoảng tối thiểu giữa 2 lần phát cùng một âm (giây).
pub const MIN_GAP: f32 = 0.06;

/// Hàng đợi tên âm cần phát trong frame này.
#[derive(Resource, Default)]
pub struct Sfx {
    queue: Vec<&'static str>,
    pub muted: bool,
}

impl Sfx {
    /// Hàng đợi tắt tiếng (chế độ chụp ảnh).
    #[cfg(target_arch = "wasm32")]
    pub fn muted() -> Sfx {
        Sfx {
            queue: Vec::new(),
            muted: true,
        }
    }
    pub fn play(&mut self, name: &'static str) {
        if !self.muted && !self.queue.contains(&name) {
            self.queue.push(name);
        }
    }
}

#[derive(Component)]
struct SfxVoice;
#[derive(Component)]
struct Music;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sfx>()
            .add_systems(Update, (map_events, play_queue, music).chain());
    }
}

/// Âm cho một sự kiện lõi (thuần, test được). `n` xoay vòng biến thể hit/meow.
pub fn sound_for(ev: &CoreEvent, n: usize) -> Option<&'static str> {
    const HITS: [&str; 3] = ["hit_1", "hit_2", "hit_3"];
    const MEOWS: [&str; 4] = ["meow_1", "meow_2", "meow_3", "meow_4"];
    Some(match ev {
        CoreEvent::Damage { crit: true, .. } => "crit",
        CoreEvent::Damage { .. } => HITS[n % 3],
        CoreEvent::CatAttack { cat } => match cat.key() {
            "muop" => "cat_attack_muop",
            "xiem" => "cat_attack_xiem",
            "map" => "cat_attack_map",
            _ => "cat_attack_mun",
        },
        CoreEvent::CatSkill { cat, .. } => match cat.key() {
            "muop" => "skill_muop",
            "xiem" => "skill_xiem",
            "map" => "skill_map",
            _ => "skill_mun",
        },
        CoreEvent::CatDown { .. } => "cat_down",
        CoreEvent::CatRevive { .. } => MEOWS[n % 4],
        CoreEvent::EnemyDied { .. } => "enemy_die",
        CoreEvent::BossAppear { .. } => "boss_appear",
        CoreEvent::BossDefeated { .. } => "boss_die",
        CoreEvent::StageCleared { .. } => "stage_clear",
        CoreEvent::Loot {
            currency: Currency::Fish,
            ..
        } => "coin",
        CoreEvent::Loot {
            currency: Currency::Yarn,
            ..
        } => "yarn",
        CoreEvent::GearDropped {
            rarity: Rarity::Legendary,
            ..
        } => "gear_legendary",
        CoreEvent::GearDropped { .. } => "gear_drop",
        CoreEvent::LevelUp { .. } => "level_up",
        CoreEvent::CatUnlocked { .. } => "unlock",
        CoreEvent::ItemUpgraded { .. } => "level_up",
        CoreEvent::ItemSalvaged { .. } => "yarn",
        CoreEvent::BoxSpawned { .. } => "box_land",
        CoreEvent::BoxOpened { .. } => "box_open",
        CoreEvent::CatnipStarted => "catnip",
        CoreEvent::Prestiged { .. } => "prestige",
        CoreEvent::SoulBought { .. } => "unlock",
        CoreEvent::Achievement { .. } => "achievement",
        CoreEvent::Tutorial { .. } => MEOWS[n % 4],
        _ => return None,
    })
}

fn map_events(sim: Res<Sim>, mut sfx: ResMut<Sfx>, mut n: Local<usize>) {
    for ev in &sim.events {
        // sát thương lên mèo không phát hit (tránh ồn): chỉ khi mèo đánh trúng
        if matches!(ev, CoreEvent::Damage { to_cat: true, .. }) {
            continue;
        }
        if let Some(s) = sound_for(ev, *n) {
            *n = n.wrapping_add(1);
            sfx.play(s);
        }
    }
}

fn play_queue(
    mut commands: Commands,
    mut sfx: ResMut<Sfx>,
    sounds: Option<Res<Sounds>>,
    settings: Res<Settings>,
    voices: Query<(), With<SfxVoice>>,
    time: Res<Time<Real>>,
    mut last: Local<HashMap<&'static str, f32>>,
) {
    let queue = std::mem::take(&mut sfx.queue);
    let Some(sounds) = sounds else { return };
    let gain = settings.sfx_gain();
    if gain <= 0.0 || sfx.muted {
        return;
    }
    let now = time.elapsed_secs();
    let mut live = voices.iter().count();
    for name in queue {
        if live >= MAX_VOICES {
            break;
        }
        if last.get(name).is_some_and(|t| now - t < MIN_GAP) {
            continue;
        }
        let Some(def) = sounds.map.get(name) else {
            continue;
        };
        last.insert(name, now);
        live += 1;
        commands.spawn((
            AudioPlayer(def.handle.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(gain * def.gain),
                ..default()
            },
            SfxVoice,
        ));
    }
}

fn music(
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
    settings: Res<Settings>,
    sfx: Res<Sfx>,
    mut q: Query<(Entity, Option<&mut AudioSink>), With<Music>>,
) {
    let Some(sounds) = sounds else { return };
    let gain = if sfx.muted {
        0.0
    } else {
        settings.music_gain()
    };
    let Some(def) = sounds.map.get("music_calm") else {
        return;
    };
    match q.single_mut() {
        Ok((e, sink)) => {
            if gain <= 0.0 {
                commands.entity(e).despawn();
            } else if let Some(mut sink) = sink {
                let v = Volume::Linear(gain * def.gain);
                if sink.volume() != v {
                    sink.set_volume(v);
                }
            }
        }
        Err(_) => {
            if gain > 0.0 {
                commands.spawn((
                    AudioPlayer(def.handle.clone()),
                    PlaybackSettings {
                        mode: PlaybackMode::Loop,
                        volume: Volume::Linear(gain * def.gain),
                        ..default()
                    },
                    Music,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::CatId;
    #[test]
    fn events_map_to_spec_sounds() {
        assert_eq!(
            sound_for(
                &CoreEvent::CatSkill {
                    cat: CatId::Map,
                    x: 0.0
                },
                0
            ),
            Some("skill_map")
        );
        assert_eq!(
            sound_for(
                &CoreEvent::Damage {
                    target: 1,
                    x: 0.0,
                    y: 0.0,
                    amount: 1.0,
                    crit: true,
                    to_cat: false
                },
                0
            ),
            Some("crit")
        );
        assert_eq!(
            sound_for(
                &CoreEvent::Damage {
                    target: 1,
                    x: 0.0,
                    y: 0.0,
                    amount: 1.0,
                    crit: false,
                    to_cat: false
                },
                4
            ),
            Some("hit_2")
        );
        assert_eq!(sound_for(&CoreEvent::BoxExpired, 0), None);
    }
    #[test]
    fn all_mapped_names_exist_in_audio_manifest() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/audio/audio.json");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let names: Vec<String> = v["sounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect();
        for n in [
            "hit_1",
            "hit_2",
            "hit_3",
            "crit",
            "cat_attack_muop",
            "skill_mun",
            "enemy_die",
            "boss_appear",
            "boss_die",
            "cat_down",
            "level_up",
            "unlock",
            "coin",
            "yarn",
            "gear_drop",
            "gear_legendary",
            "box_land",
            "box_open",
            "catnip",
            "ui_click",
            "ui_tab",
            "ui_error",
            "prestige",
            "achievement",
            "stage_clear",
            "meow_1",
            "music_calm",
        ] {
            assert!(names.iter().any(|x| x == n), "audio.json thiếu {n}");
        }
    }
}
