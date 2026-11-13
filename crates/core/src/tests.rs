//! Test lõi: tất định, save/load, công thức, lệnh, boss, thành tựu, vắng mặt, animation.

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::*;

// ───────────────────────────── tiện ích ─────────────────────────────

fn ticks(g: &mut Game, n: u32) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..n {
        g.tick();
        all.extend(g.drain_events());
    }
    all
}

/// Chạy tới khi `pred` đúng với một sự kiện (tối đa `max` tick). Trả các sự kiện đã thấy.
fn run_until(g: &mut Game, max: u32, pred: impl Fn(&Event) -> bool) -> Option<Vec<Event>> {
    let mut all = Vec::new();
    for _ in 0..max {
        g.tick();
        let ev = g.drain_events();
        let hit = ev.iter().any(&pred);
        all.extend(ev);
        if hit {
            return Some(all);
        }
    }
    None
}

/// Chỉ còn đúng một quái `kind` đứng cạnh Mướp; trận ở trạng thái Fight.
fn lone_enemy(g: &mut Game, kind: EnemyKind, dx: f64) -> usize {
    g.s.enemies.clear();
    g.s.projs.clear();
    g.s.drops.clear();
    let gs = g.s.global_stage();
    let x = g.s.cat(CatId::Muop).x + dx;
    g.spawn_enemy(kind, x, gs);
    g.s.phase = Phase::Fight;
    g.s.wave_total = 1;
    g.s.wave_killed = 0;
    g.s.boss_entity = 0;
    g.s.boss_timer = None;
    g.s.enemies.len() - 1
}

/// Mèo không tự đánh (để quan sát đòn của quái).
fn pacify_cats(g: &mut Game) {
    for cs in g.s.cats.iter_mut() {
        cs.atk_cd = 1e9;
        cs.skill_cd = 1e9;
    }
}

fn give_giant_collars(g: &mut Game) {
    for _ in 0..4 {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Collar,
            rarity: Rarity::Legendary,
            ilvl: 10_000_000,
            upgrade: 0,
        });
    }
    g.command(Command::EquipBest { cat: None }).unwrap();
    for cs in g.s.cats.iter_mut().filter(|c| c.unlocked) {
        cs.hp = cs.d.hp_max;
    }
}

fn first_item(g: &Game) -> ItemId {
    g.inventory()[0].id
}

// ───────────────────────────── tất định / lưu ─────────────────────────────

/// Bot nhỏ dùng chung cho test tất định.
fn scripted(g: &mut Game, t: u32) {
    if t.is_multiple_of(150) {
        for c in CatId::ALL {
            let _ = g.command(Command::Unlock { cat: c });
            let _ = g.command(Command::LevelUp {
                cat: c,
                mode: BuyMode::Max,
            });
        }
        let _ = g.command(Command::EquipBest { cat: None });
        let _ = g.command(Command::OpenBox);
        let _ = g.command(Command::Catnip);
    }
}

#[test]
fn deterministic_same_seed_same_commands() {
    let mut a = Game::new(42);
    let mut b = Game::new(42);
    for t in 0..10_000 {
        scripted(&mut a, t);
        scripted(&mut b, t);
        a.tick();
        b.tick();
        if t % 1000 == 0 {
            assert_eq!(a.state_hash(), b.state_hash(), "lệch ở tick {t}");
        }
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.drain_events(), b.drain_events());
    let mut c = Game::new(43);
    for t in 0..10_000 {
        scripted(&mut c, t);
        c.tick();
    }
    assert_ne!(a.state_hash(), c.state_hash());
}

#[test]
fn save_load_roundtrip() {
    let mut g = Game::new(9);
    for t in 0..6000 {
        scripted(&mut g, t);
        g.tick();
    }
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Epic,
        ilvl: 7,
        upgrade: 3,
    });
    g.cheat(Cheat::SpawnBox);
    let json = g.save(1_700_000_000);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["version"], SAVE_VERSION);
    assert_eq!(v["saved_at"], 1_700_000_000u64);
    let mut h = Game::load(&json).unwrap();
    assert_eq!(h.saved_at(), 1_700_000_000);
    assert_eq!(g.state_hash(), h.state_hash());
    assert_eq!(g.hud(), h.hud());
    assert_eq!(g.inventory(), h.inventory());
    // tiếp tục chạy cùng lệnh → vẫn trùng
    for t in 6000..9000 {
        scripted(&mut g, t);
        scripted(&mut h, t);
        g.tick();
        h.tick();
    }
    assert_eq!(g.state_hash(), h.state_hash());
    assert_eq!(Game::new(1).saved_at(), 0);
}

#[test]
fn load_errors() {
    assert!(matches!(
        Game::load("không phải json"),
        Err(LoadError::Parse(_))
    ));
    assert!(matches!(Game::load("{}"), Err(LoadError::Parse(_))));
    let g = Game::new(1);
    let json = g.save(5);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["version"] = serde_json::json!(SAVE_VERSION + 1);
    assert_eq!(
        Game::load(&v.to_string()).err(),
        Some(LoadError::Version(SAVE_VERSION + 1))
    );
    // trường thiếu trong state → giá trị mặc định (migrate mềm)
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["state"].as_object_mut().unwrap().remove("stats");
    assert!(Game::load(&v.to_string()).is_ok());
}

// ───────────────────────────── công thức ─────────────────────────────

#[test]
fn level_price_formula_via_api() {
    let mut g = Game::new(1);
    let q = g.level_quote(CatId::Muop, BuyMode::One);
    assert_eq!(q.levels, 1);
    assert!((q.cost - 10.0).abs() < 1e-9);
    assert!(!q.affordable);
    let ten = g.level_quote(CatId::Muop, BuyMode::Ten);
    let expect: f64 = (0..10).map(|i| 10.0 * 1.13f64.powi(i)).sum();
    assert!((ten.cost - expect).abs() < 1e-6);
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 100.0,
    });
    let max = g.level_quote(CatId::Muop, BuyMode::Max);
    assert!(max.affordable);
    assert!(max.cost <= 100.0);
    assert!(rules::level_cost_n(CatId::Muop, 1, max.levels + 1) > 100.0);
    g.command(Command::LevelUp {
        cat: CatId::Muop,
        mode: BuyMode::Max,
    })
    .unwrap();
    assert_eq!(g.cat_info(CatId::Muop).level, 1 + max.levels);
    // chỉ số theo cấp: base * 1.10^(level-1)
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 11,
    });
    let info = g.cat_info(CatId::Muop);
    assert!((info.dmg - 8.0 * 1.1f64.powi(10)).abs() < 1e-9);
    assert!((info.hp_max - 100.0 * 1.1f64.powi(10)).abs() < 1e-9);
    assert_eq!(info.talent_points, 1);
    assert!(info.dps > 0.0);
}

#[test]
fn gear_formula_via_api_and_multipliers() {
    let mut g = Game::new(1);
    g.cheat(Cheat::GiveItem {
        slot: Slot::Claws,
        rarity: Rarity::Rare,
        ilvl: 20,
        upgrade: 2,
    });
    let it = g.inventory()[0].clone();
    let st = g.item_stats(&it);
    let pct = (4.0 + 20.0 * 0.5) * 1.6 * 1.2;
    assert!((st.dmg_pct - pct).abs() < 1e-9);
    assert!((st.power_score - pct).abs() < 1e-9);
    assert_eq!(g.upgrade_cost(it.id), Some((5.0f64 * 9.0 * 1.6).round()));
    assert_eq!(g.salvage_value(it.id), Some((3.0f64 * 1.6 * 3.0).round()));
    let before = g.cat_info(CatId::Muop).dmg;
    g.command(Command::Equip {
        cat: CatId::Muop,
        item: it.id,
    })
    .unwrap();
    let after = g.cat_info(CatId::Muop).dmg;
    assert!((after / before - (1.0 + pct / 100.0)).abs() < 1e-9);
    // chí mạng trần 60%
    for _ in 0..3 {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Bell,
            rarity: Rarity::Legendary,
            ilvl: 0,
            upgrade: 10,
        });
    }
    g.command(Command::EquipBest {
        cat: Some(CatId::Muop),
    })
    .unwrap();
    let bell = g.cat_info(CatId::Muop).equipped[Slot::Bell.index()]
        .clone()
        .unwrap();
    assert!((g.item_stats(&bell).crit_pct - 2.0 * 4.0 * 2.0).abs() < 1e-9);
    g.s.cat_mut(CatId::Muop).equipped[1] = Some(Item {
        id: 999,
        slot: Slot::Bell,
        rarity: Rarity::Legendary,
        ilvl: 0,
        upgrade: 200,
    });
    rules::recalc(&mut g.s);
    assert_eq!(g.cat_info(CatId::Muop).crit_pct, CRIT_CAP);
}

#[test]
fn enemy_scaling_and_wave_size() {
    let g = Game::new(1);
    assert_eq!(g.s.wave_total, 3);
    let e = &g.s.enemies[0];
    let def = enemy_def(e.kind);
    assert!((e.hp_max - def.hp).abs() < 1e-9);
    let mut g = Game::new(1);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 2,
        stage: 9,
    });
    assert_eq!(g.s.wave_total, 6);
    let e = &g.s.enemies[0];
    assert!(zone_enemies(2).contains(&e.kind));
    assert!((e.hp_max - enemy_def(e.kind).hp * ENEMY_SCALE.powi(18)).abs() < 1e-6);
    assert!((e.fish - enemy_def(e.kind).fish * REWARD_SCALE.powi(18)).abs() < 1e-6);
    g.cheat(Cheat::Goto {
        cycle: 1,
        zone: 3,
        stage: 10,
    });
    assert_eq!(g.hud().global_stage, 79);
    assert_eq!(g.s.wave_total, 3);
    assert!(g.s.enemies.iter().any(|e| e.kind == EnemyKind::Eel));
    // tối đa 7 quái
    for st in 1..=9 {
        g.cheat(Cheat::Goto {
            cycle: 0,
            zone: 1,
            stage: st,
        });
        assert_eq!(g.s.wave_total, (3 + st / 3).min(MAX_WAVE));
    }
}

// ───────────────────────────── luồng màn ─────────────────────────────

#[test]
fn stages_advance_and_zone_changes() {
    let mut g = Game::new(5);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 60,
    });
    let ev = run_until(&mut g, 30 * 60, |e| matches!(e, Event::StageCleared { .. })).unwrap();
    assert!(ev.iter().any(|e| matches!(e, Event::EnemyDied { .. })));
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::Loot {
            currency: Currency::Fish,
            ..
        }
    )));
    assert_eq!(g.hud().stage, 2);
    assert!(g.stats().enemies_killed >= 3);
    run_until(&mut g, 30 * 600, |e| {
        matches!(e, Event::ZoneEntered { zone: 2, .. })
    })
    .unwrap();
    assert_eq!(g.hud().zone, 2);
    assert_eq!(g.hud().stage, 1);
    assert!(g.achievements()[AchId::BossRatKing as usize].unlocked);
}

#[test]
fn party_wipe_goes_back_one_stage_and_revives() {
    let mut g = Game::new(5);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 3,
        stage: 5,
    });
    let ev = run_until(&mut g, 30 * 120, |e| matches!(e, Event::StageFailed { .. })).unwrap();
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::CatDown { cat: CatId::Muop }))
    );
    assert!(ev.contains(&Event::StageFailed { zone: 3, stage: 5 }));
    assert_eq!(g.hud().stage, 4);
    assert!(!g.hud().farming);
    // ngủ trong lúc chờ, rồi hồi sinh đầy máu sau 3 s
    let c = g.cats()[0];
    assert!(!c.alive);
    let ev = ticks(&mut g, (FAIL_WAIT * 30.0) as u32 + 2);
    assert!(ev.contains(&Event::CatRevive { cat: CatId::Muop }));
    let c = g.cats()[0];
    assert!(c.alive);
    assert!((c.hp - c.hp_max).abs() < c.hp_max * 0.05);
    // không lùi dưới màn 1
    let mut g = Game::new(5);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 4,
        stage: 1,
    });
    run_until(&mut g, 30 * 120, |e| matches!(e, Event::StageFailed { .. })).unwrap();
    assert_eq!(g.hud().stage, 1);
    assert_eq!(g.hud().zone, 4);
}

#[test]
fn boss_timeout_falls_back_and_farms_then_retries() {
    let mut g = Game::new(11);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 10,
    });
    give_giant_collars(&mut g);
    // giảm sát thương của Mướp để không thể hạ boss trong 30 s
    g.s.cat_mut(CatId::Muop).d.dmg = 0.5;
    let ev = run_until(&mut g, 30 * 80, |e| matches!(e, Event::StageFailed { .. })).unwrap();
    assert!(ev.contains(&Event::BossAppear {
        kind: EnemyKind::RatKing
    }));
    assert!(
        !ev.iter().any(|e| matches!(e, Event::CatDown { .. })),
        "phải là hết giờ, không phải chết"
    );
    assert!(ev.contains(&Event::StageFailed { zone: 1, stage: 10 }));
    let hud = g.hud();
    assert_eq!(hud.stage, 9);
    assert!(hud.farming);
    assert!(matches!(
        g.next_goal(),
        Goal::UnlockCat {
            cat: CatId::Xiem,
            ..
        }
    ));
    // cày màn 9: qua màn 9 vẫn ở màn 9
    rules::recalc(&mut g.s);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 40,
    });
    run_until(&mut g, 30 * 40, |e| {
        *e == Event::StageCleared { zone: 1, stage: 9 }
    })
    .unwrap();
    assert_eq!(g.hud().stage, 9);
    assert!(g.hud().farming);
    // tự thử lại sau 60 s
    let ev = run_until(&mut g, 30 * 120, |e| matches!(e, Event::BossAppear { .. })).unwrap();
    assert!(!ev.is_empty());
    assert_eq!(g.hud().stage, 10);
    assert!(!g.hud().farming);
}

#[test]
fn boss_hud_and_timer() {
    let mut g = Game::new(2);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 2,
        stage: 10,
    });
    let b = g.hud().boss.unwrap();
    assert_eq!(b.kind, EnemyKind::CrowLord);
    assert_eq!(b.time_left, BOSS_TIME as f32);
    give_giant_collars(&mut g);
    run_until(&mut g, 30 * 30, |e| matches!(e, Event::BossAppear { .. })).unwrap();
    ticks(&mut g, 60);
    let b = g.hud().boss.unwrap();
    assert!(b.time_left < BOSS_TIME as f32 - 1.5);
    assert!(b.hp <= b.hp_max);
}

// ───────────────────────────── boss: đòn đặc biệt ─────────────────────────────

fn boss_scenario(zone: u32) -> (Game, Vec<Event>) {
    let mut g = Game::new(100 + zone as u64);
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone,
        stage: 10,
    });
    give_giant_collars(&mut g);
    let kind = zone_boss(zone);
    let ev = run_until(
        &mut g,
        30 * 40,
        |e| matches!(e, Event::BossSpecial { kind: k, .. } if *k == kind),
    )
    .unwrap_or_else(|| panic!("boss {kind:?} không dùng đòn đặc biệt"));
    (g, ev)
}

fn special_batch(ev: &[Event]) -> &[Event] {
    let i = ev
        .iter()
        .rposition(|e| matches!(e, Event::BossSpecial { .. }))
        .unwrap();
    &ev[i..]
}

#[test]
fn boss_special_rat_king_summons() {
    let (g, _) = boss_scenario(1);
    assert_eq!(g.s.wave_total, BOSS_ESCORTS + 1 + RAT_KING_SUMMONS);
    assert!(
        g.enemies()
            .iter()
            .filter(|e| e.kind == EnemyKind::Rat
                && !g.s.enemies.iter().any(|x| x.entity == e.entity && x.dead))
            .count()
            >= 2
    );
    let boss = g
        .enemies()
        .into_iter()
        .find(|e| e.kind == EnemyKind::RatKing)
        .unwrap();
    assert_eq!(boss.anim, Anim::Special);
}

#[test]
fn boss_special_crow_lord_dashes_to_rearmost() {
    let (g, ev) = boss_scenario(2);
    let batch = special_batch(&ev);
    assert!(
        batch
            .iter()
            .any(|e| matches!(e, Event::Damage { to_cat: true, .. }))
    );
    let boss =
        g.s.enemies
            .iter()
            .find(|e| e.kind == EnemyKind::CrowLord)
            .unwrap();
    let rear =
        g.s.cats
            .iter()
            .filter(|c| c.unlocked && c.alive)
            .map(|c| c.x)
            .fold(f64::MAX, f64::min);
    assert!(
        (boss.x - rear).abs() <= 12.0,
        "boss {} rear {}",
        boss.x,
        rear
    );
}

#[test]
fn boss_special_eel_shocks_and_stuns() {
    let (g, ev) = boss_scenario(3);
    let batch = special_batch(&ev);
    assert!(
        batch
            .iter()
            .any(|e| matches!(e, Event::Stunned { target, .. } if *target <= 4))
    );
    assert!(g.cats().iter().any(|c| c.stunned));
}

#[test]
fn boss_special_mastiff_roars() {
    let (g, _) = boss_scenario(4);
    let boss =
        g.s.enemies
            .iter()
            .find(|e| e.kind == EnemyKind::Mastiff)
            .unwrap();
    assert!(boss.enrage > 0.0);
}

#[test]
fn boss_special_titan_pulls_and_stuns() {
    let mut g = Game::new(105);
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 5,
        stage: 10,
    });
    give_giant_collars(&mut g);
    let mut prev: Vec<f64> = g.s.cats.iter().map(|c| c.x).collect();
    let mut seen = false;
    for _ in 0..30 * 40 {
        prev = g.s.cats.iter().map(|c| c.x).collect();
        g.tick();
        let ev = g.drain_events();
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::BossSpecial {
                    kind: EnemyKind::VacuumTitan,
                    ..
                }
            )
        }) {
            seen = true;
            assert!(
                ev.iter()
                    .any(|e| matches!(e, Event::Stunned { target, .. } if *target <= 4))
            );
            break;
        }
    }
    assert!(seen);
    let boss_x =
        g.s.enemies
            .iter()
            .find(|e| e.kind == EnemyKind::VacuumTitan)
            .unwrap()
            .x;
    // ít nhất một mèo ở xa bị kéo lại gần boss
    let pulled =
        g.s.cats
            .iter()
            .zip(prev.iter())
            .any(|(c, &p)| (boss_x - c.x).abs() < (boss_x - p).abs() - 5.0);
    assert!(pulled);
    assert!(g.cats().iter().all(|c| c.stunned));
}

// ───────────────────────────── hiệu ứng trạng thái & kỹ năng ─────────────────────────────

#[test]
fn snake_poisons() {
    let mut g = Game::new(1);
    give_giant_collars(&mut g);
    pacify_cats(&mut g);
    lone_enemy(&mut g, EnemyKind::Snake, 8.0);
    ticks(&mut g, 10);
    assert!(g.s.cat(CatId::Muop).poison_t > 0.0);
    let hp = g.s.cat(CatId::Muop).hp;
    ticks(&mut g, 3);
    assert!(g.s.cat(CatId::Muop).hp < hp);
}

#[test]
fn tick_lifesteals() {
    let mut g = Game::new(1);
    pacify_cats(&mut g);
    let i = lone_enemy(&mut g, EnemyKind::Tick, 6.0);
    g.s.enemies[i].hp = g.s.enemies[i].hp_max * 0.5;
    let ev = run_until(&mut g, 60, |e| matches!(e, Event::Heal { .. })).unwrap();
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Damage { to_cat: true, .. }))
    );
    assert!(g.s.enemies[0].hp > g.s.enemies[0].hp_max * 0.5);
}

#[test]
fn owl_shoots_feathers() {
    let mut g = Game::new(1);
    pacify_cats(&mut g);
    lone_enemy(&mut g, EnemyKind::Owl, 60.0);
    run_until(&mut g, 60, |e| {
        matches!(
            e,
            Event::EnemyAttack {
                kind: EnemyKind::Owl,
                ..
            }
        )
    })
    .unwrap();
    assert!(g.projectiles().iter().any(|p| p.kind == ProjKind::Feather));
    run_until(&mut g, 60, |e| {
        matches!(e, Event::Damage { to_cat: true, .. })
    })
    .unwrap();
}

#[test]
fn map_sits_stuns_and_taunts_and_thorns_reflect() {
    let mut g = Game::new(1);
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Map,
        level: 50,
    });
    for _ in 0..5 {
        g.command(Command::TalentUp {
            cat: CatId::Map,
            slot: 2,
        })
        .unwrap();
    }
    pacify_cats(&mut g);
    let mx = g.s.cat(CatId::Map).x;
    g.s.enemies.clear();
    let gs = g.s.global_stage();
    g.spawn_enemy(EnemyKind::Rat, mx + 8.0, gs);
    g.s.phase = Phase::Fight;
    // phản đòn: chuột đánh Mập → chuột mất máu dù mèo không đánh
    run_until(&mut g, 60, |e| matches!(e, Event::EnemyAttack { .. })).unwrap();
    assert!(g.s.enemies[0].hp < g.s.enemies[0].hp_max);
    // kỹ năng Ngồi Đè
    g.s.cat_mut(CatId::Map).skill_cd = 0.0;
    let ev = run_until(&mut g, 10, |e| {
        matches!(
            e,
            Event::CatSkill {
                cat: CatId::Map,
                ..
            }
        )
    })
    .unwrap();
    assert!(ev.iter().any(|e| matches!(e, Event::Stunned { .. })));
    assert!(g.enemies()[0].stunned);
    assert!(g.s.taunt > 0.0);
    assert_eq!(
        g.cats().iter().find(|c| c.cat == CatId::Map).unwrap().anim,
        Anim::Skill
    );
}

#[test]
fn muop_combo_three_hits() {
    let mut g = Game::new(1);
    let i = lone_enemy(&mut g, EnemyKind::Vacuum, 10.0);
    g.s.enemies[i].hp = 1e9;
    g.s.enemies[i].hp_max = 1e9;
    g.s.cat_mut(CatId::Muop).atk_cd = 1e9;
    let mut ev = run_until(&mut g, 30, |e| {
        matches!(
            e,
            Event::CatSkill {
                cat: CatId::Muop,
                ..
            }
        )
    })
    .unwrap();
    ev.extend(ticks(&mut g, 20));
    let hits = ev
        .iter()
        .filter(|e| matches!(e, Event::Damage { to_cat: false, .. }))
        .count();
    assert_eq!(hits as u32, MUOP_COMBO_HITS);
}

#[test]
fn xiem_bone_rain_and_pierce() {
    let mut g = Game::new(1);
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Xiem,
        level: 50,
    });
    for _ in 0..3 {
        g.command(Command::TalentUp {
            cat: CatId::Xiem,
            slot: 2,
        })
        .unwrap();
    }
    pacify_cats(&mut g);
    let xx = g.s.cat(CatId::Xiem).x;
    g.s.enemies.clear();
    let gs = g.s.global_stage();
    for k in 0..3 {
        g.spawn_enemy(EnemyKind::Vacuum, xx + 60.0 + 3.0 * k as f64, gs);
    }
    for e in g.s.enemies.iter_mut() {
        e.hp = 1e9;
        e.hp_max = 1e9;
        e.stun = 1e9;
    }
    g.s.phase = Phase::Fight;
    // đạn xuyên: bậc 3 → xuyên ceil(3/2) = 2 quái thêm
    g.s.cat_mut(CatId::Xiem).atk_cd = 0.0;
    ticks(&mut g, 1);
    let p =
        g.s.projs
            .iter()
            .find(|p| p.kind == ProjKind::Fishbone)
            .unwrap();
    assert_eq!(p.pierce_left, 2);
    ticks(&mut g, 30);
    assert_eq!(g.s.enemies.iter().filter(|e| e.hp < 1e9).count(), 3);
    // Mưa Xương Cá
    g.s.cat_mut(CatId::Xiem).skill_cd = 0.0;
    g.s.cat_mut(CatId::Xiem).atk_cd = 1e9;
    ticks(&mut g, 1);
    assert_eq!(
        g.projectiles()
            .iter()
            .filter(|p| p.kind == ProjKind::BoneRain)
            .count() as u32,
        XIEM_RAIN_BONES
    );
}

#[test]
fn mun_curse_and_aoe() {
    let mut g = Game::new(1);
    g.cheat(Cheat::UnlockAll);
    pacify_cats(&mut g);
    let mx = g.s.cat(CatId::Mun).x;
    g.s.enemies.clear();
    let gs = g.s.global_stage();
    g.spawn_enemy(EnemyKind::Vacuum, mx + 50.0, gs);
    g.spawn_enemy(EnemyKind::Vacuum, mx + 60.0, gs);
    for e in g.s.enemies.iter_mut() {
        e.hp = 1e9;
        e.hp_max = 1e9;
        e.stun = 1e9;
    }
    g.s.phase = Phase::Fight;
    g.s.cat_mut(CatId::Mun).skill_cd = 0.0;
    let ev = run_until(&mut g, 5, |e| {
        matches!(
            e,
            Event::CatSkill {
                cat: CatId::Mun,
                ..
            }
        )
    })
    .unwrap();
    assert!(!ev.is_empty());
    assert!(g.enemies()[0].cursed);
    // đạn nổ vùng trúng cả 2 quái, quái bị nguyền nhận thêm 30%
    g.s.cat_mut(CatId::Mun).atk_cd = 0.0;
    let ev = run_until(&mut g, 60, |e| {
        matches!(e, Event::Damage { to_cat: false, .. })
    })
    .unwrap();
    let dmg: Vec<f64> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Damage {
                to_cat: false,
                amount,
                crit: false,
                ..
            } => Some(*amount),
            _ => None,
        })
        .collect();
    assert_eq!(dmg.len(), 2);
    assert!((dmg[0] / dmg[1] - 1.3).abs() < 1e-9 || (dmg[1] / dmg[0] - 1.3).abs() < 1e-9);
}

// ───────────────────────────── animation ─────────────────────────────

#[test]
fn animations_follow_actions() {
    let mut g = Game::new(3);
    ticks(&mut g, 5);
    assert_eq!(g.cats()[0].anim, Anim::Walk);
    let e0 = g.enemies()[0];
    assert_eq!(e0.anim, Anim::Walk);
    assert_eq!(e0.facing, -1);
    // tấn công
    let mut saw_attack = false;
    let mut saw_enemy_death = false;
    let mut saw_enemy_attack = false;
    for _ in 0..30 * 20 {
        g.tick();
        g.drain_events();
        saw_attack |= g.cats()[0].anim == Anim::Attack;
        saw_enemy_death |= g.enemies().iter().any(|e| e.anim == Anim::Death);
        saw_enemy_attack |= g.enemies().iter().any(|e| e.anim == Anim::Attack);
    }
    assert!(saw_attack && saw_enemy_death && saw_enemy_attack);
    // trúng đòn → hurt; chết → death rồi sleep
    let mut g = Game::new(3);
    pacify_cats(&mut g);
    lone_enemy(&mut g, EnemyKind::Dog, 8.0);
    run_until(&mut g, 60, |e| {
        matches!(e, Event::Damage { to_cat: true, .. })
    })
    .unwrap();
    assert_eq!(g.cats()[0].anim, Anim::Hurt);
    g.s.cat_mut(CatId::Muop).hp = 0.1;
    g.s.cat_mut(CatId::Muop).anim_lock = 0.0;
    run_until(&mut g, 120, |e| matches!(e, Event::CatDown { .. })).unwrap();
    assert_eq!(g.cats()[0].anim, Anim::Death);
    ticks(&mut g, (ANIM_DEATH_CAT * 30.0) as u32 + 2);
    assert_eq!(g.cats()[0].anim, Anim::Sleep);
    assert!(g.cats()[0].anim_t >= 0.0);
}

#[test]
fn idle_cats_fall_asleep() {
    // đội đứng yên (chờ sau thất bại) → mèo còn thức nằm ngủ
    let mut g = Game::new(3);
    g.s.enemies.clear();
    g.s.phase = Phase::Failed { t: -100.0 };
    let cs = g.s.cat_mut(CatId::Muop);
    cs.anim = Anim::Idle;
    cs.anim_lock = 0.0;
    ticks(&mut g, 40);
    assert_eq!(g.cats()[0].anim, Anim::Sleep);
    // đứng Idle quá IDLE_TO_SLEEP giây → ngủ
    let mut g = Game::new(3);
    let cs = g.s.cat_mut(CatId::Muop);
    cs.anim = Anim::Idle;
    cs.idle_t = IDLE_TO_SLEEP - 0.01;
    g.update_cat_anims();
    assert_eq!(g.cats()[0].anim, Anim::Sleep);
}

// ───────────────────────────── lệnh: thành công + lỗi ─────────────────────────────

#[test]
fn cmd_level_up() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::LevelUp {
            cat: CatId::Muop,
            mode: BuyMode::One
        }),
        Err(CmdError::NotEnough(Currency::Fish))
    );
    assert_eq!(
        g.command(Command::LevelUp {
            cat: CatId::Xiem,
            mode: BuyMode::One
        }),
        Err(CmdError::Locked)
    );
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 1000.0,
    });
    g.command(Command::LevelUp {
        cat: CatId::Muop,
        mode: BuyMode::One,
    })
    .unwrap();
    assert!(g.drain_events().contains(&Event::LevelUp {
        cat: CatId::Muop,
        level: 2
    }));
    g.command(Command::LevelUp {
        cat: CatId::Muop,
        mode: BuyMode::Ten,
    })
    .unwrap();
    assert_eq!(g.cat_info(CatId::Muop).level, 12);
    assert!(
        (g.hud().fish - (1000.0 - 10.0 - rules::level_cost_n(CatId::Muop, 2, 10))).abs() < 1e-6
    );
}

#[test]
fn cmd_unlock() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::Unlock { cat: CatId::Muop }),
        Err(CmdError::AlreadyUnlocked)
    );
    assert_eq!(
        g.command(Command::Unlock { cat: CatId::Xiem }),
        Err(CmdError::Locked)
    );
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 5,
    });
    assert!(g.cat_info(CatId::Xiem).unlock_reachable);
    assert_eq!(
        g.command(Command::Unlock { cat: CatId::Xiem }),
        Err(CmdError::NotEnough(Currency::Fish))
    );
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 50.0,
    });
    g.command(Command::Unlock { cat: CatId::Xiem }).unwrap();
    assert!(g.cat_info(CatId::Xiem).unlocked);
    assert_eq!(g.cats().len(), 2);
    assert!(g.achievements()[AchId::UnlockXiem as usize].unlocked);
}

#[test]
fn cmd_talent_up() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::TalentUp {
            cat: CatId::Muop,
            slot: 0
        }),
        Err(CmdError::NoTalentPoints)
    );
    assert_eq!(
        g.command(Command::TalentUp {
            cat: CatId::Mun,
            slot: 0
        }),
        Err(CmdError::Locked)
    );
    assert_eq!(
        g.command(Command::TalentUp {
            cat: CatId::Muop,
            slot: 3
        }),
        Err(CmdError::NotAvailable)
    );
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 100,
    });
    let aspd = g.cat_info(CatId::Muop).aspd;
    for _ in 0..5 {
        g.command(Command::TalentUp {
            cat: CatId::Muop,
            slot: 2,
        })
        .unwrap();
    }
    assert_eq!(
        g.command(Command::TalentUp {
            cat: CatId::Muop,
            slot: 2
        }),
        Err(CmdError::MaxRank)
    );
    let info = g.cat_info(CatId::Muop);
    assert_eq!(info.talents[2], (Talent::Frenzy, 5));
    assert!((info.aspd / aspd - 1.30).abs() < 1e-9);
    assert_eq!(info.talent_points, 5);
    let dmg = info.dmg;
    g.command(Command::TalentUp {
        cat: CatId::Muop,
        slot: 0,
    })
    .unwrap();
    assert!((g.cat_info(CatId::Muop).dmg / dmg - 1.08).abs() < 1e-9);
}

#[test]
fn cmd_equip_unequip() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::Equip {
            cat: CatId::Muop,
            item: 77
        }),
        Err(CmdError::NoSuchItem)
    );
    g.cheat(Cheat::GiveItem {
        slot: Slot::Collar,
        rarity: Rarity::Rare,
        ilvl: 3,
        upgrade: 0,
    });
    let id = first_item(&g);
    assert_eq!(
        g.command(Command::Equip {
            cat: CatId::Xiem,
            item: id
        }),
        Err(CmdError::Locked)
    );
    g.command(Command::Equip {
        cat: CatId::Muop,
        item: id,
    })
    .unwrap();
    assert!(g.inventory().is_empty());
    assert_eq!(
        g.cat_info(CatId::Muop).equipped[0].as_ref().map(|i| i.id),
        Some(id)
    );
    assert_eq!(
        g.command(Command::Equip {
            cat: CatId::Muop,
            item: id
        }),
        Err(CmdError::NothingToDo)
    );
    // chuyển món sang mèo khác
    g.cheat(Cheat::UnlockAll);
    g.command(Command::Equip {
        cat: CatId::Map,
        item: id,
    })
    .unwrap();
    assert!(g.cat_info(CatId::Muop).equipped[0].is_none());
    assert_eq!(
        g.command(Command::Unequip {
            cat: CatId::Muop,
            slot: Slot::Collar
        }),
        Err(CmdError::NothingToDo)
    );
    g.command(Command::Unequip {
        cat: CatId::Map,
        slot: Slot::Collar,
    })
    .unwrap();
    assert_eq!(g.inventory().len(), 1);
    // kho đầy → không tháo được
    g.command(Command::Equip {
        cat: CatId::Map,
        item: id,
    })
    .unwrap();
    for _ in 0..INVENTORY_CAP {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Bell,
            rarity: Rarity::Common,
            ilvl: 0,
            upgrade: 0,
        });
    }
    assert_eq!(g.inventory().len(), INVENTORY_CAP);
    assert_eq!(g.inventory_cap(), INVENTORY_CAP);
    assert_eq!(
        g.command(Command::Unequip {
            cat: CatId::Map,
            slot: Slot::Collar
        }),
        Err(CmdError::NotAvailable)
    );
}

#[test]
fn cmd_equip_best() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::EquipBest { cat: None }),
        Err(CmdError::NothingToDo)
    );
    assert_eq!(
        g.command(Command::EquipBest {
            cat: Some(CatId::Mun)
        }),
        Err(CmdError::Locked)
    );
    for (r, il) in [(Rarity::Common, 50), (Rarity::Epic, 10), (Rarity::Rare, 10)] {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Claws,
            rarity: r,
            ilvl: il,
            upgrade: 0,
        });
    }
    g.command(Command::EquipBest {
        cat: Some(CatId::Muop),
    })
    .unwrap();
    let worn = g.cat_info(CatId::Muop).equipped[Slot::Claws.index()]
        .clone()
        .unwrap();
    // common ilvl50: (4+25)*1 = 29; epic ilvl10: 9*2.5 = 22.5 → common thắng theo power_score
    assert_eq!(worn.rarity, Rarity::Common);
    assert_eq!(
        g.command(Command::EquipBest {
            cat: Some(CatId::Muop)
        }),
        Err(CmdError::NothingToDo)
    );
    assert!(!g.hud().any_affordable || g.hud().fish >= 10.0);
}

#[test]
fn cmd_upgrade_item() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::UpgradeItem { item: 5 }),
        Err(CmdError::NoSuchItem)
    );
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Rare,
        ilvl: 1,
        upgrade: 9,
    });
    let id = first_item(&g);
    assert_eq!(
        g.command(Command::UpgradeItem { item: id }),
        Err(CmdError::NotEnough(Currency::Yarn))
    );
    let cost = g.upgrade_cost(id).unwrap();
    assert_eq!(cost, (5.0f64 * 100.0 * 1.6).round());
    g.cheat(Cheat::Give {
        currency: Currency::Yarn,
        amount: cost,
    });
    g.command(Command::UpgradeItem { item: id }).unwrap();
    assert_eq!(g.item(id).unwrap().upgrade, 10);
    assert_eq!(g.hud().yarn, 0.0);
    assert!(g.drain_events().contains(&Event::ItemUpgraded {
        item: id,
        upgrade: 10
    }));
    assert_eq!(
        g.command(Command::UpgradeItem { item: id }),
        Err(CmdError::MaxRank)
    );
    assert_eq!(g.upgrade_cost(id), None);
    assert!(g.achievements()[AchId::MaxUpgrade as usize].unlocked);
}

#[test]
fn cmd_salvage_and_auto_salvage() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::Salvage { item: 3 }),
        Err(CmdError::NoSuchItem)
    );
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Epic,
        ilvl: 1,
        upgrade: 2,
    });
    let id = first_item(&g);
    g.command(Command::Salvage { item: id }).unwrap();
    assert_eq!(g.hud().yarn, (3.0f64 * 2.5 * 3.0).round());
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Rare,
        ilvl: 1,
        upgrade: 0,
    });
    let worn = first_item(&g);
    g.command(Command::Equip {
        cat: CatId::Muop,
        item: worn,
    })
    .unwrap();
    assert_eq!(
        g.command(Command::Salvage { item: worn }),
        Err(CmdError::NotAvailable)
    );
    // tự rã
    assert_eq!(g.auto_salvage(), None);
    assert_eq!(
        g.command(Command::SetAutoSalvage { max: None }),
        Err(CmdError::NothingToDo)
    );
    for r in [Rarity::Common, Rarity::Rare, Rarity::Epic] {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Collar,
            rarity: r,
            ilvl: 1,
            upgrade: 0,
        });
    }
    g.command(Command::SetAutoSalvage {
        max: Some(Rarity::Rare),
    })
    .unwrap();
    assert_eq!(g.auto_salvage(), Some(Rarity::Rare));
    let inv = g.inventory();
    assert_eq!(inv.len(), 1);
    assert_eq!(inv[0].rarity, Rarity::Epic);
    // món rơi mới cũng tự rã
    let y = g.hud().yarn;
    let it = g.make_item(Slot::Claws, Rarity::Common, 1, 0);
    g.gain_item(it);
    assert_eq!(g.inventory().len(), 1);
    assert!(g.hud().yarn > y);
    // kho đầy → tự rã
    g.command(Command::SetAutoSalvage { max: None }).unwrap();
    while g.inventory().len() < INVENTORY_CAP {
        g.cheat(Cheat::GiveItem {
            slot: Slot::Collar,
            rarity: Rarity::Legendary,
            ilvl: 1,
            upgrade: 0,
        });
    }
    let y = g.hud().yarn;
    let it = g.make_item(Slot::Claws, Rarity::Legendary, 1, 0);
    g.gain_item(it);
    assert_eq!(g.inventory().len(), INVENTORY_CAP);
    assert!(g.hud().yarn > y);
}

#[test]
fn cmd_catnip() {
    let mut g = Game::new(1);
    let aspd_tick = |g: &Game| g.catnip_aspd();
    g.command(Command::Catnip).unwrap();
    assert!(g.drain_events().contains(&Event::CatnipStarted));
    assert_eq!(g.hud().catnip_active, CATNIP_SECS as f32);
    assert_eq!(g.hud().catnip_cd, 1.0);
    assert!((aspd_tick(&g) - 1.5).abs() < 1e-12);
    assert!((g.party_speed() - PARTY_SPEED * 1.4).abs() < 1e-9);
    assert_eq!(g.command(Command::Catnip), Err(CmdError::OnCooldown));
    g.cheat(Cheat::Give {
        currency: Currency::Catnip,
        amount: 1.0,
    });
    g.command(Command::Catnip).unwrap();
    assert_eq!(g.hud().catnip_charges, 0);
    let ev = ticks(&mut g, (CATNIP_SECS * 30.0) as u32 + 2);
    assert!(ev.contains(&Event::CatnipEnded));
    assert_eq!(g.stats().catnip_used, 2);
}

#[test]
fn auto_catnip_from_soul_shop() {
    let mut g = Game::new(1);
    g.cheat(Cheat::Give {
        currency: Currency::Soul,
        amount: 5.0,
    });
    g.command(Command::BuySoul {
        upgrade: SoulUpgrade::AutoCatnip,
    })
    .unwrap();
    let ev = ticks(&mut g, 30);
    assert!(ev.contains(&Event::CatnipStarted));
    assert_eq!(
        g.command(Command::BuySoul {
            upgrade: SoulUpgrade::AutoCatnip
        }),
        Err(CmdError::MaxRank)
    );
}

#[test]
fn cmd_open_box_and_box_lifecycle() {
    let mut g = Game::new(1);
    assert!(g.carton().is_none());
    assert_eq!(g.command(Command::OpenBox), Err(CmdError::NothingToDo));
    g.cheat(Cheat::SpawnBox);
    let b = g.carton().unwrap();
    assert!(b.y > 0.0);
    assert_eq!(g.pending_tutorial(), Some(TutorialId::FirstBox));
    ticks(&mut g, 30);
    assert_eq!(g.carton().unwrap().y, 0.0);
    let res_before = (
        g.hud().fish,
        g.hud().yarn,
        g.hud().catnip_charges,
        g.inventory().len(),
    );
    g.command(Command::OpenBox).unwrap();
    let res_after = (
        g.hud().fish,
        g.hud().yarn,
        g.hud().catnip_charges,
        g.inventory().len(),
    );
    assert_ne!(res_before, res_after, "hộp phải cho thưởng");
    assert_eq!(g.command(Command::OpenBox), Err(CmdError::NothingToDo));
    assert!(g.carton().unwrap().opening.is_some());
    ticks(&mut g, 20);
    assert!(g.carton().is_none());
    assert_eq!(g.stats().boxes_opened, 1);
    // hộp hết hạn sau 90 s, không phạt
    g.cheat(Cheat::SpawnBox);
    let ev = ticks(&mut g, (BOX_LIFE * 30.0) as u32 + 5);
    assert!(ev.contains(&Event::BoxExpired));
    // hộp tự rơi trong 180..360 s
    let mut g = Game::new(8);
    let ev = run_until(&mut g, (BOX_MAX * 30.0) as u32 + 30, |e| {
        matches!(e, Event::BoxSpawned { .. })
    })
    .unwrap();
    assert!(g.hud().time >= BOX_MIN - 1.0);
    assert!(!ev.is_empty());
}

#[test]
fn box_reward_distribution() {
    let mut g = Game::new(4);
    let mut kinds = [0u32; 4];
    for _ in 0..400 {
        g.cheat(Cheat::SpawnBox);
        let before = (
            g.hud().fish,
            g.hud().yarn,
            g.hud().catnip_charges,
            g.s.next_item,
        );
        g.command(Command::OpenBox).unwrap();
        let h = g.hud();
        if h.fish > before.0 {
            kinds[0] += 1;
        } else if h.yarn > before.1 {
            kinds[1] += 1;
        } else if g.s.next_item > before.3 {
            kinds[2] += 1;
        } else if h.catnip_charges > before.2 {
            kinds[3] += 1;
        }
        g.s.carton = None;
        g.drain_events();
    }
    assert_eq!(kinds.iter().sum::<u32>(), 400);
    assert!(
        kinds[0] > 150 && kinds[1] > 60 && kinds[2] > 30 && kinds[3] > 15,
        "{kinds:?}"
    );
}

#[test]
fn cmd_buy_soul() {
    let mut g = Game::new(1);
    assert_eq!(
        g.command(Command::BuySoul {
            upgrade: SoulUpgrade::ClawSharpen
        }),
        Err(CmdError::NotEnough(Currency::Soul))
    );
    g.cheat(Cheat::Give {
        currency: Currency::Soul,
        amount: 100.0,
    });
    let dmg = g.cat_info(CatId::Muop).dmg;
    g.command(Command::BuySoul {
        upgrade: SoulUpgrade::ClawSharpen,
    })
    .unwrap();
    assert!((g.cat_info(CatId::Muop).dmg / dmg - 1.15).abs() < 1e-9);
    let row = g.soul_shop()[0];
    assert_eq!(row.rank, 1);
    assert_eq!(row.cost, Some((1.35f64).round()));
    for _ in 0..4 {
        g.command(Command::BuySoul {
            upgrade: SoulUpgrade::LongNap,
        })
        .unwrap();
    }
    assert_eq!(
        g.command(Command::BuySoul {
            upgrade: SoulUpgrade::LongNap
        }),
        Err(CmdError::MaxRank)
    );
    assert_eq!(g.soul_shop()[SoulUpgrade::LongNap as usize].cost, None);
    assert_eq!(rules::offline_cap(&g.s), 16 * 3600);
}

#[test]
fn cmd_retry_boss() {
    let mut g = Game::new(1);
    assert_eq!(g.command(Command::RetryBoss), Err(CmdError::NotAvailable));
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 9,
    });
    g.s.farming = true;
    g.command(Command::RetryBoss).unwrap();
    assert_eq!(g.hud().stage, 10);
    assert!(!g.hud().farming);
    assert!(g.hud().boss.is_some());
}

#[test]
fn cmd_dismiss_tutorial() {
    let mut g = Game::new(1);
    assert_eq!(g.pending_tutorial(), None);
    assert_eq!(
        g.command(Command::DismissTutorial {
            id: TutorialId::OpenBag
        }),
        Err(CmdError::NothingToDo)
    );
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 10.0,
    });
    assert_eq!(g.pending_tutorial(), Some(TutorialId::OpenBag));
    assert!(g.drain_events().contains(&Event::Tutorial {
        id: TutorialId::OpenBag
    }));
    g.command(Command::DismissTutorial {
        id: TutorialId::OpenBag,
    })
    .unwrap();
    assert_eq!(g.pending_tutorial(), None);
    // mỗi gợi ý chỉ một lần
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 10.0,
    });
    assert_eq!(g.pending_tutorial(), None);
    // gợi ý mở Xiêm
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 5,
    });
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 50.0,
    });
    assert_eq!(g.pending_tutorial(), Some(TutorialId::UnlockXiem));
}

// ───────────────────────────── Chín Mạng ─────────────────────────────

#[test]
fn cmd_prestige_resets_exactly() {
    let mut g = Game::new(1);
    assert_eq!(g.command(Command::Prestige), Err(CmdError::NotAvailable));
    assert!(!g.prestige_preview().available);
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 4,
        stage: 1,
    });
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 40,
    });
    g.command(Command::TalentUp {
        cat: CatId::Muop,
        slot: 0,
    })
    .unwrap();
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 5000.0,
    });
    g.cheat(Cheat::Give {
        currency: Currency::Yarn,
        amount: 1000.0,
    });
    g.cheat(Cheat::Give {
        currency: Currency::Soul,
        amount: 2.0,
    });
    g.cheat(Cheat::Give {
        currency: Currency::Catnip,
        amount: 2.0,
    });
    g.command(Command::BuySoul {
        upgrade: SoulUpgrade::ThickFur,
    })
    .unwrap();
    g.command(Command::SetAutoSalvage {
        max: Some(Rarity::Common),
    })
    .unwrap();
    // món: huyền thoại đang mặc (giữ), sử thi đang mặc (mất), huyền thoại trong kho (mất)
    g.cheat(Cheat::GiveItem {
        slot: Slot::Claws,
        rarity: Rarity::Legendary,
        ilvl: 5,
        upgrade: 4,
    });
    let leg = first_item(&g);
    g.command(Command::Equip {
        cat: CatId::Xiem,
        item: leg,
    })
    .unwrap();
    g.cheat(Cheat::GiveItem {
        slot: Slot::Collar,
        rarity: Rarity::Epic,
        ilvl: 5,
        upgrade: 0,
    });
    let epic = first_item(&g);
    g.command(Command::Equip {
        cat: CatId::Xiem,
        item: epic,
    })
    .unwrap();
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Legendary,
        ilvl: 5,
        upgrade: 0,
    });
    let stats_before = g.stats();
    let ach_before = g.achievements();
    let pp = g.prestige_preview();
    assert!(pp.available);
    assert_eq!(pp.required_global_stage, PRESTIGE_MIN_GS);
    assert_eq!(pp.souls_gain, rules::prestige_gain(30));
    let soul_before = g.hud().soul;

    g.command(Command::Prestige).unwrap();
    let ev = g.drain_events();
    assert!(ev.contains(&Event::Prestiged {
        souls: pp.souls_gain,
        lives_used: 1
    }));
    let h = g.hud();
    assert_eq!((h.cycle, h.zone, h.stage), (0, 1, 1));
    assert_eq!(h.fish, 0.0);
    assert_eq!(h.yarn, 250.0);
    assert_eq!(h.soul, soul_before + pp.souls_gain);
    assert_eq!(h.catnip_charges, 2);
    assert_eq!(h.max_global_stage, 0);
    for c in CatId::ALL {
        let i = g.cat_info(c);
        assert!(i.unlocked, "mèo đã mở giữ nguyên");
        assert_eq!(i.level, 1);
        assert_eq!(i.talents.iter().map(|t| t.1).sum::<u8>(), 0);
        assert_eq!(i.talent_points, 0);
    }
    assert!(g.inventory().is_empty());
    let xiem = g.cat_info(CatId::Xiem);
    assert_eq!(
        xiem.equipped[Slot::Claws.index()].as_ref().map(|i| i.id),
        Some(leg)
    );
    assert!(xiem.equipped[Slot::Collar.index()].is_none());
    assert_eq!(g.soul_shop()[SoulUpgrade::ThickFur as usize].rank, 1);
    assert_eq!(g.auto_salvage(), Some(Rarity::Common));
    assert_eq!(g.stats(), stats_before);
    for (a, b) in ach_before.iter().zip(g.achievements().iter()) {
        assert!(!a.unlocked || b.unlocked);
    }
    assert!(g.life_marks()[0]);
    assert!(!g.life_marks()[1]);
    assert!((rules::fish_mult(&g.s) - 1.05).abs() < 1e-12);
}

#[test]
fn prestige_head_start_and_life_marks() {
    let mut g = Game::new(1);
    g.cheat(Cheat::Give {
        currency: Currency::Soul,
        amount: 100.0,
    });
    g.command(Command::BuySoul {
        upgrade: SoulUpgrade::HeadStart,
    })
    .unwrap();
    g.command(Command::BuySoul {
        upgrade: SoulUpgrade::HeadStart,
    })
    .unwrap();
    for n in 1..=10u32 {
        g.cheat(Cheat::Goto {
            cycle: 0,
            zone: 3,
            stage: 6,
        });
        g.command(Command::Prestige).unwrap();
        let h = g.hud();
        assert_eq!((h.zone, h.stage), (2, 1), "head_start bậc 2 → màn +10");
        assert_eq!(h.max_global_stage, 10);
        assert_eq!(g.prestige_preview().lives_used, n);
    }
    assert!(g.life_marks().iter().all(|&m| m));
    // sau mạng 9 vẫn prestige được nhưng không thêm dấu ấn
    assert!((rules::fish_mult(&g.s) - 1.45).abs() < 1e-12);
}

// ───────────────────────────── vắng mặt ─────────────────────────────

#[test]
fn offline_rewards_capped_and_no_penalty() {
    let mut g = Game::new(6);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 30,
    });
    ticks(&mut g, 30 * 400);
    assert_eq!(g.apply_offline(59), None);
    let before = (g.hud(), g.cat_info(CatId::Muop).level, g.inventory());
    let json = g.save(1000);
    let mut h = Game::load(&json).unwrap();
    let rep = h.apply_offline(100 * 3600).unwrap();
    assert!(rep.capped);
    assert_eq!(rep.secs, OFFLINE_BASE_CAP);
    assert!(rep.fish > 0.0);
    assert!(rep.yarn >= 0.0);
    assert_eq!(rep.wake_buff_secs, WAKE_BUFF_MAX as f32);
    let after = h.hud();
    assert!(after.fish >= before.0.fish + rep.fish - 1e-6);
    assert!(after.yarn >= before.0.yarn);
    assert_eq!(
        (after.zone, after.stage),
        (before.0.zone, before.0.stage),
        "không tiến màn khi offline"
    );
    assert_eq!(h.cat_info(CatId::Muop).level, before.1);
    assert_eq!(h.inventory(), before.2);
    assert_eq!(after.wake_buff, WAKE_BUFF_MAX as f32);
    // công thức: thu nhập trung bình 300 s gần nhất × elapsed × 0.6
    let mut k = Game::load(&json).unwrap();
    let (fr, _) = k.income_rates();
    let r = k.apply_offline(7200).unwrap();
    assert!(!r.capped);
    assert!((r.fish - fr * 7200.0 * OFFLINE_MULT).abs() < 1e-6);
    assert_eq!(r.wake_buff_secs, 120.0);
    // long_nap +2 h trần
    let mut k = Game::load(&json).unwrap();
    k.cheat(Cheat::Give {
        currency: Currency::Soul,
        amount: 3.0,
    });
    k.command(Command::BuySoul {
        upgrade: SoulUpgrade::LongNap,
    })
    .unwrap();
    let r = k.apply_offline(100 * 3600).unwrap();
    assert_eq!(r.secs, OFFLINE_BASE_CAP + 7200);
    // buff vừa ngủ dậy: x2 cá
    let fish_mult_normal = g.wake_mult();
    assert_eq!(fish_mult_normal, 1.0);
    assert_eq!(h.wake_mult(), WAKE_FISH_MULT);
}

#[test]
fn offline_without_income_history_uses_estimate() {
    let mut g = Game::new(6);
    let r = g.apply_offline(3600).unwrap();
    assert!(r.fish > 0.0);
    assert!((r.fish - rules::estimate_fish_rate(&g.s) * 3600.0 * OFFLINE_MULT).abs() < 1e-6);
}

// ───────────────────────────── thành tựu ─────────────────────────────

#[test]
fn every_achievement_reachable() {
    let mut g = Game::new(21);
    let unlocked = |g: &Game, id: AchId| g.achievements()[id as usize].unlocked;
    g.cheat(Cheat::UnlockAll);
    for id in [AchId::UnlockXiem, AchId::UnlockMap, AchId::UnlockMun] {
        assert!(unlocked(&g, id));
    }
    g.cheat(Cheat::SetLevel {
        cat: CatId::Map,
        level: 100,
    });
    for id in [AchId::Level25, AchId::Level50, AchId::Level100] {
        assert!(unlocked(&g, id));
    }
    // hạ từng boss (cấp rất cao)
    for c in CatId::ALL {
        g.cheat(Cheat::SetLevel { cat: c, level: 400 });
    }
    for z in 1..=5 {
        g.cheat(Cheat::Goto {
            cycle: 0,
            zone: z,
            stage: 10,
        });
        let kind = zone_boss(z);
        run_until(&mut g, 30 * 60, |e| *e == Event::BossDefeated { kind })
            .unwrap_or_else(|| panic!("không hạ được {kind:?}"));
        run_until(&mut g, 30 * 5, |e| matches!(e, Event::ZoneEntered { .. })).unwrap();
    }
    for id in [
        AchId::BossRatKing,
        AchId::BossCrowLord,
        AchId::BossEel,
        AchId::BossMastiff,
        AchId::BossVacuumTitan,
        AchId::Zone2,
        AchId::Zone3,
        AchId::Zone4,
        AchId::Zone5,
        AchId::Cycle1,
    ] {
        assert!(unlocked(&g, id), "{id:?}");
    }
    assert_eq!(g.hud().cycle, 1);
    // đồ
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Legendary,
        ilvl: 1,
        upgrade: 9,
    });
    assert!(unlocked(&g, AchId::FirstLegendary));
    let id = g.inventory().last().unwrap().id;
    g.cheat(Cheat::Give {
        currency: Currency::Yarn,
        amount: 1e6,
    });
    g.command(Command::UpgradeItem { item: id }).unwrap();
    assert!(unlocked(&g, AchId::MaxUpgrade));
    // hộp
    for _ in 0..100 {
        g.cheat(Cheat::SpawnBox);
        g.command(Command::OpenBox).unwrap();
    }
    assert!(unlocked(&g, AchId::Boxes10) && unlocked(&g, AchId::Boxes100));
    // catnip
    g.cheat(Cheat::Give {
        currency: Currency::Catnip,
        amount: 60.0,
    });
    for _ in 0..50 {
        g.command(Command::Catnip).unwrap();
    }
    assert!(unlocked(&g, AchId::Catnip50));
    // giàu: tổng cá kiếm được (Give là cheat, không tính) → kiếm thật qua hộp/offline
    g.s.stats.fish_earned = 0.0;
    g.credit_fish(1e6);
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 0.0,
    });
    assert!(unlocked(&g, AchId::Fish1M) && !unlocked(&g, AchId::Fish1B));
    g.credit_fish(1e9);
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 0.0,
    });
    assert!(unlocked(&g, AchId::Fish1B));
    assert_eq!(AchId::Fish1M.key(), "fish_1m");
    assert_eq!(AchId::Fish1B.group(), "wealth");
    // mạng
    for _ in 0..9 {
        g.cheat(Cheat::Goto {
            cycle: 0,
            zone: 3,
            stage: 6,
        });
        g.command(Command::Prestige).unwrap();
    }
    for id in [AchId::Lives2, AchId::Lives5, AchId::Lives9] {
        assert!(unlocked(&g, id));
    }
    let all = g.achievements();
    assert_eq!(all.len(), 26);
    assert!(all.iter().all(|a| a.unlocked && a.at.is_some()), "{all:?}");
    // đã đạt rồi thì không phát lại
    g.drain_events();
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 1.0,
    });
    assert!(
        !g.drain_events()
            .iter()
            .any(|e| matches!(e, Event::Achievement { .. }))
    );
}

#[test]
fn cheat_unlock_achievement() {
    let mut g = Game::new(1);
    assert!(!g.achievements()[AchId::Boxes100 as usize].unlocked);
    g.cheat(Cheat::UnlockAchievement {
        id: AchId::Boxes100,
    });
    assert!(g.achievements()[AchId::Boxes100 as usize].unlocked);
    assert!(g.drain_events().contains(&Event::Achievement {
        id: AchId::Boxes100
    }));
    g.cheat(Cheat::UnlockAchievement {
        id: AchId::Boxes100,
    });
    assert!(g.drain_events().is_empty());
}

// ───────────────────────────── truy vấn khác ─────────────────────────────

#[test]
fn next_goal_progression() {
    let mut g = Game::new(1);
    assert_eq!(g.next_goal(), Goal::ReachStage { zone: 1, stage: 5 });
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 5,
    });
    assert_eq!(
        g.next_goal(),
        Goal::UnlockCat {
            cat: CatId::Xiem,
            fish_needed: 50.0
        }
    );
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 50.0,
    });
    g.command(Command::Unlock { cat: CatId::Xiem }).unwrap();
    assert_eq!(g.next_goal(), Goal::ReachStage { zone: 2, stage: 1 });
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 9,
    });
    assert_eq!(
        g.next_goal(),
        Goal::DefeatBoss {
            kind: EnemyKind::RatKing,
            zone: 1
        }
    );
    g.cheat(Cheat::UnlockAll);
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 3,
        stage: 7,
    });
    assert!(matches!(g.next_goal(), Goal::Prestige { .. }));
    g.command(Command::Prestige).unwrap();
    assert!(matches!(g.next_goal(), Goal::LevelCat { .. }));
}

#[test]
fn any_affordable_flag() {
    let mut g = Game::new(1);
    assert!(!g.hud().any_affordable);
    g.cheat(Cheat::Give {
        currency: Currency::Fish,
        amount: 10.0,
    });
    assert!(g.hud().any_affordable);
    let mut g = Game::new(1);
    g.cheat(Cheat::GiveItem {
        slot: Slot::Bell,
        rarity: Rarity::Common,
        ilvl: 1,
        upgrade: 0,
    });
    assert!(g.hud().any_affordable, "món tốt hơn trong kho");
}

#[test]
fn views_are_consistent() {
    let mut g = Game::new(12);
    g.cheat(Cheat::UnlockAll);
    ticks(&mut g, 30 * 10);
    let hud = g.hud();
    assert_eq!(g.cats().len(), 4);
    // đội hình: Mập trước, Mướp, Xiêm, Mun sau
    let cats = g.cats();
    let x = |c: CatId| cats.iter().find(|v| v.cat == c).unwrap().x;
    assert!(x(CatId::Map) >= x(CatId::Muop) - 1.0);
    assert!(x(CatId::Muop) > x(CatId::Xiem));
    assert!(x(CatId::Xiem) > x(CatId::Mun));
    assert!(hud.party_x >= x(CatId::Map) - 1.0);
    assert!(g.enemies().iter().all(|e| e.x as f64 > -1e9));
    // quái bay vẽ cao hơn
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 2,
        stage: 3,
    });
    for e in g.enemies() {
        assert_eq!(e.y > 0.0, e.kind.flies());
    }
    // đồ rơi bay về đội
    let mut seen = false;
    for _ in 0..30 * 30 {
        g.tick();
        g.drain_events();
        if let Some(d) = g.drops().first() {
            assert!((0.0..=1.0).contains(&d.collect_t));
            seen = true;
        }
    }
    assert!(seen);
    let s = g.stats();
    assert!(s.play_secs > 30.0 && s.enemies_killed > 0 && s.fish_earned > 0.0);
}

#[test]
fn gear_drops_from_bosses_min_rare() {
    let mut g = Game::new(31);
    for c in CatId::ALL {
        g.cheat(Cheat::SetLevel { cat: c, level: 200 });
    }
    g.cheat(Cheat::Goto {
        cycle: 0,
        zone: 1,
        stage: 10,
    });
    let ev = run_until(&mut g, 30 * 60, |e| matches!(e, Event::BossDefeated { .. })).unwrap();
    let _ = ev;
    let ev = ticks(&mut g, 30);
    let _ = ev;
    let all: Vec<Item> = g.inventory();
    assert!(
        all.iter()
            .any(|it| it.rarity >= Rarity::Rare && it.ilvl == 9),
        "{all:?}"
    );
}

#[test]
fn rarity_roll_distribution() {
    let mut g = Game::new(77);
    let mut n = [0u32; 4];
    for _ in 0..20_000 {
        n[g.roll_rarity(false) as usize] += 1;
    }
    let p = |k: usize| n[k] as f64 / 20_000.0;
    assert!(
        (p(0) - 0.70).abs() < 0.02
            && (p(1) - 0.22).abs() < 0.02
            && (p(2) - 0.07).abs() < 0.01
            && p(3) > 0.004
    );
    for _ in 0..1000 {
        assert!(g.roll_rarity(true) >= Rarity::Rare);
    }
}

#[test]
fn event_queue_is_capped() {
    let mut g = Game::new(1);
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 80,
    });
    g.cheat(Cheat::Advance { ticks: 30 * 3600 });
    assert!(g.drain_events().len() <= crate::sim::EVENT_CAP);
    assert!(g.hud().time > 3599.0);
}

#[test]
fn game_is_send_sync_for_bevy_resource() {
    fn check<T: Send + Sync + 'static>() {}
    check::<Game>();
}

#[test]
fn enemy_hurt_and_boss_special_are_short() {
    let mut g = Game::new(3);
    pacify_cats(&mut g);
    let i = lone_enemy(&mut g, EnemyKind::Vacuum, 40.0);
    g.s.enemies[i].stun = 1e9;
    g.hit_enemy(i, 1.0, false, None);
    assert_eq!(g.enemies()[0].anim, Anim::Hurt);
    ticks(&mut g, 9);
    assert_ne!(g.enemies()[0].anim, Anim::Hurt);
    let (mut g, _) = boss_scenario(4);
    assert_eq!(
        g.s.enemies
            .iter()
            .find(|e| e.kind == EnemyKind::Mastiff)
            .unwrap()
            .anim,
        Anim::Special
    );
    ticks(&mut g, (ANIM_SPECIAL * 30.0) as u32 + 4);
    let boss =
        g.s.enemies
            .iter()
            .find(|e| e.kind == EnemyKind::Mastiff)
            .unwrap();
    assert_ne!(boss.anim, Anim::Special);
}

#[test]
fn old_save_with_24_achievements_loads() {
    let g = Game::new(1);
    let json = g.save(10);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = v["state"]["achievements"].as_array_mut().unwrap();
    arr.truncate(24);
    let h = Game::load(&v.to_string()).unwrap();
    assert_eq!(h.achievements().len(), 26);
}

#[test]
fn wealth_achievement_from_real_income() {
    let mut g = Game::new(2);
    g.s.stats.fish_earned = 999_990.0;
    g.cheat(Cheat::SetLevel {
        cat: CatId::Muop,
        level: 40,
    });
    let ev = run_until(&mut g, 30 * 120, |e| {
        *e == Event::Achievement { id: AchId::Fish1M }
    });
    assert!(ev.is_some());
}
