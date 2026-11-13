//! Ảnh chụp tất định cho bản web (SPEC §10.2): `?shot=<tên>&lang=vi|en&scale=1..4&bg=…`.
//! Dựng trạng thái bằng `Game::cheat`, dừng mô phỏng, đặt `window.__CM_READY = true` sau ≥ 20 frame
//! kể từ khi asset đã nạp và UI đã dựng. Nền desktop giả do harness vẽ dưới canvas trong suốt.

use crate::assets::AssetsReady;
use crate::i18n::{Lang, t};
use crate::settings::Settings;
use crate::sim::{Offline, Sim};
use crate::ui::{Tab, View};
use bevy::prelude::*;
use cm_core::{AchId, BuyMode, CatId, Cheat, Currency, Game, Rarity, Slot};

pub const SHOTS: [&str; 18] = [
    "strip-z1",
    "strip-z3-boss",
    "strip-z5",
    "strip-white",
    "strip-black",
    "panel-cats",
    "panel-cat-detail",
    "panel-gear",
    "panel-souls",
    "panel-achievements",
    "panel-settings",
    "offline-summary",
    "tutorial-bubble",
    "box",
    "vi-diacritics",
    "en",
    "scale1",
    "scale3",
];

/// Tham số URL.
#[derive(Resource, Clone, Debug, Default)]
pub struct ShotParams {
    pub shot: Option<String>,
    pub lang: Option<Lang>,
    pub scale: Option<u32>,
}

pub fn read_params() -> ShotParams {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let Ok(q) = web_sys::UrlSearchParams::new_with_str(&search) else {
        return ShotParams::default();
    };
    ShotParams {
        shot: q.get("shot").filter(|s| !s.is_empty()),
        lang: q
            .get("lang")
            .map(|l| if l == "en" { Lang::En } else { Lang::Vi }),
        scale: q
            .get("scale")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|s| (1..=4).contains(s)),
    }
}

fn base_game(zone: u32, stage: u32) -> Game {
    let mut g = Game::new(7);
    g.cheat(Cheat::UnlockAll);
    for (c, lv) in [
        (CatId::Muop, 34),
        (CatId::Xiem, 28),
        (CatId::Map, 22),
        (CatId::Mun, 17),
    ] {
        g.cheat(Cheat::SetLevel { cat: c, level: lv });
    }
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 48_250.0,
    });
    g.cheat(Cheat::Give {
        currency: Currency::Yarn,
        amount: 640.0,
    });
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone,
        stage,
    });
    // tắt hướng dẫn để ảnh không bị che (trừ ảnh hướng dẫn)
    for id in [
        cm_core::TutorialId::OpenBag,
        cm_core::TutorialId::UnlockXiem,
        cm_core::TutorialId::FirstBox,
    ] {
        let _ = g.command(cm_core::Command::DismissTutorial { id });
    }
    g
}

/// Chạy tới khi có quái trong tầm nhìn (tối đa `max` tick).
fn advance_until_fight(g: &mut Game, max: u32) {
    let mut n = 0;
    while n < max {
        g.cheat(Cheat::Advance { ticks: 15 });
        n += 15;
        let party = g.hud().party_x;
        if g.enemies().iter().any(|e| (e.x - party).abs() < 60.0) {
            g.cheat(Cheat::Advance { ticks: 20 });
            return;
        }
    }
}

fn give_gear(g: &mut Game) {
    let list = [
        (Slot::Claws, Rarity::Legendary, 24, 6),
        (Slot::Bell, Rarity::Epic, 22, 3),
        (Slot::Collar, Rarity::Rare, 20, 2),
        (Slot::Claws, Rarity::Rare, 18, 0),
        (Slot::Collar, Rarity::Common, 12, 0),
        (Slot::Bell, Rarity::Common, 15, 1),
        (Slot::Collar, Rarity::Epic, 23, 0),
        (Slot::Bell, Rarity::Rare, 19, 4),
        (Slot::Claws, Rarity::Common, 9, 0),
        (Slot::Collar, Rarity::Legendary, 25, 0),
        (Slot::Claws, Rarity::Epic, 21, 2),
        (Slot::Bell, Rarity::Common, 6, 0),
    ];
    for (slot, rarity, ilvl, upgrade) in list {
        g.cheat(Cheat::GiveItem {
            slot,
            rarity,
            ilvl,
            upgrade,
        });
    }
}

/// Dựng trạng thái cho ảnh `name`. Trả về (game, view, settings đã chỉnh, báo cáo vắng mặt).
pub fn build(name: &str, settings: &mut Settings) -> (Game, View, Option<cm_core::OfflineReport>) {
    let mut view = View {
        shot_mode: true,
        ..View::default()
    };
    let mut report = None;
    let game = match name {
        "strip-z3-boss" => {
            let mut g = base_game(3, 10);
            for c in CatId::ALL {
                g.cheat(Cheat::SetLevel { cat: c, level: 60 });
            }
            let mut n = 0;
            while g.hud().boss.is_none() && n < 900 {
                g.cheat(Cheat::Advance { ticks: 15 });
                n += 15;
            }
            advance_until_fight(&mut g, 600);
            g.cheat(Cheat::Advance { ticks: 30 });
            g
        }
        "strip-z5" => {
            let mut g = base_game(5, 4);
            for c in CatId::ALL {
                g.cheat(Cheat::SetLevel { cat: c, level: 80 });
            }
            advance_until_fight(&mut g, 600);
            g
        }
        "strip-white" | "strip-black" | "scale1" | "scale3" | "en" => {
            let mut g = base_game(2, 5);
            advance_until_fight(&mut g, 600);
            g
        }
        "panel-cats" | "panel-cat-detail" | "panel-gear" | "vi-diacritics" => {
            let mut g = base_game(2, 3);
            give_gear(&mut g);
            let _ = g.command(cm_core::Command::EquipBest {
                cat: Some(CatId::Muop),
            });
            g.cheat(Cheat::Advance { ticks: 60 });
            view.panel = Some(match name {
                "panel-gear" => Tab::Gear,
                _ => Tab::Cats,
            });
            if name == "panel-cat-detail" {
                view.sel_cat = Some(CatId::Muop);
            }
            if name == "panel-gear" {
                view.sel_cat = Some(CatId::Muop);
                view.sel_item = g
                    .inventory()
                    .iter()
                    .find(|i| i.slot == Slot::Claws)
                    .map(|i| i.id);
            }
            view.buy = if name == "panel-cats" {
                BuyMode::Ten
            } else {
                BuyMode::One
            };
            g
        }
        "panel-souls" => {
            let mut g = base_game(3, 7);
            g.cheat(Cheat::Give {
                currency: Currency::Soul,
                amount: 14.0,
            });
            for u in [
                cm_core::SoulUpgrade::ClawSharpen,
                cm_core::SoulUpgrade::FatWallet,
                cm_core::SoulUpgrade::ThickFur,
            ] {
                let _ = g.command(cm_core::Command::BuySoul { upgrade: u });
            }
            g.cheat(Cheat::Advance { ticks: 30 });
            view.panel = Some(Tab::Souls);
            g
        }
        "panel-achievements" => {
            let mut g = base_game(2, 6);
            for id in [
                AchId::Zone2,
                AchId::BossRatKing,
                AchId::UnlockXiem,
                AchId::UnlockMap,
                AchId::Level25,
                AchId::Boxes10,
                AchId::FirstLegendary,
            ] {
                g.cheat(Cheat::UnlockAchievement { id });
            }
            g.cheat(Cheat::Advance { ticks: 30 });
            view.panel = Some(Tab::Achievements);
            g
        }
        "panel-settings" => {
            let mut g = base_game(1, 6);
            g.cheat(Cheat::Advance { ticks: 30 });
            view.panel = Some(Tab::Settings);
            g
        }
        "offline-summary" => {
            let mut g = base_game(1, 8);
            g.cheat(Cheat::Advance { ticks: 90 });
            report = g.apply_offline(3 * 3600 + 12 * 60);
            g
        }
        "tutorial-bubble" => {
            // ván mới thật: hướng dẫn "mở túi" xuất hiện khi có 10 cá lần đầu
            let mut g = Game::new(7);
            let mut n = 0;
            while g.pending_tutorial().is_none() && n < 9000 {
                g.cheat(Cheat::Advance { ticks: 30 });
                n += 30;
            }
            g
        }
        "box" => {
            let mut g = base_game(1, 4);
            g.cheat(Cheat::Advance { ticks: 30 });
            g.cheat(Cheat::SpawnBox);
            g.cheat(Cheat::Advance { ticks: 45 });
            g
        }
        // "strip-z1" và mặc định
        _ => {
            let mut g = base_game(1, 3);
            advance_until_fight(&mut g, 600);
            g
        }
    };
    if name == "vi-diacritics" {
        settings.lang = Lang::Vi;
        let l = Lang::Vi;
        view.forced_tip = Some((
            vec![
                t(l, "hdr_compare").to_string(),
                format!(
                    "{} · {} · {}",
                    t(l, "tut_header"),
                    t(l, "role_map"),
                    t(l, "ach_boxes_10")
                ),
            ],
            Vec2::new(24.0, 150.0),
        ));
    }
    if name == "en" {
        settings.lang = Lang::En;
        view.panel = Some(Tab::Cats);
    }
    match name {
        "scale1" => settings.scale = 1,
        "scale3" => settings.scale = 3,
        _ => {}
    }
    (game, view, report)
}

#[derive(Resource, Default)]
struct ReadyFrames(u32);

pub struct ShotsPlugin;

impl Plugin for ShotsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReadyFrames>()
            .add_systems(Last, mark_ready);
    }
}

/// Áp ảnh chụp vào app trước khi chạy.
pub fn install(app: &mut App, name: &str, settings: &mut Settings) {
    if !SHOTS.contains(&name) {
        error!("shot không hợp lệ: {name}");
    }
    let (game, view, report) = build(name, settings);
    let mut sim = Sim::new(game);
    sim.paused = true;
    sim.persist = false;
    app.insert_resource(sim)
        .insert_resource(view)
        .insert_resource(Offline { report, shown: 0.0 });
    app.add_plugins(ShotsPlugin);
}

fn mark_ready(
    mut frames: ResMut<ReadyFrames>,
    ready: Res<AssetsReady>,
    layout: Option<Res<crate::ui::CurrentLayout>>,
) {
    if !ready.0 || layout.is_none() || frames.0 > 20 {
        return;
    }
    frames.0 += 1;
    if frames.0 == 21 {
        let _ = js_sys::Reflect::set(&js_sys::global(), &"__CM_READY".into(), &true.into());
        info!("__CM_READY");
    }
}
