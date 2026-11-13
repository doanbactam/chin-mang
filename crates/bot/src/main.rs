//! cm-bot: bot chơi headless + cổng cân bằng (SPEC §4, §10.3).
//!
//! - `cm-bot sim --hours H --seed S --bot greedy|idle [--every MIN]`
//! - `cm-bot gates [--threads N]`

use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cm_core::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BotKind {
    Greedy,
    Idle,
}

/// Số liệu thu được trong một lần chạy.
#[derive(Clone, Debug, Default)]
struct Report {
    seed: u64,
    hours: f64,
    unlock_xiem: Option<f64>,
    unlock_map: Option<f64>,
    unlock_mun: Option<f64>,
    first_rat_king: Option<f64>,
    reach_zone3: Option<f64>,
    first_prestige: Option<f64>,
    lives_used: u32,
    best_gs: u32,
    /// giây đứng lâu nhất ở cùng max_global_stage trước prestige đầu tiên
    max_stall: f64,
    /// trung vị khoảng cách giữa hai lần mua trong 2 giờ đầu (giây)
    median_buy_gap: f64,
    cpu_secs: f64,
}

struct Bot {
    kind: BotKind,
    g: Game,
    t: f64,
    purchases: Vec<f64>,
    rep: Report,
    last_gs_change: f64,
    cur_max_gs: u32,
    last_idle_buy: f64,
    souls_total: f64,
    /// tắt prestige (phân tích đường cong)
    no_prestige: bool,
    /// in mỗi khi max_global_stage của mạng hiện tại tăng
    trace: bool,
}

/// Trọng số vai trò khi chọn mèo lên cấp (giá chia trọng số).
fn role_weight(c: CatId) -> f64 {
    match c {
        CatId::Muop => 1.0,
        CatId::Xiem => 1.0,
        CatId::Map => 0.8,
        CatId::Mun => 1.0,
    }
}

impl Bot {
    fn new(kind: BotKind, seed: u64) -> Bot {
        Bot {
            kind,
            g: Game::new(seed),
            t: 0.0,
            purchases: Vec::new(),
            rep: Report {
                seed,
                ..Report::default()
            },
            last_gs_change: 0.0,
            cur_max_gs: 0,
            last_idle_buy: 0.0,
            souls_total: 0.0,
            no_prestige: false,
            trace: false,
        }
    }

    fn buy(&mut self, cmd: Command) -> bool {
        let ok = self.g.command(cmd).is_ok();
        if ok {
            self.purchases.push(self.t);
        }
        ok
    }

    /// Lên cấp mèo rẻ nhất (theo trọng số vai trò) cho tới khi hết tiền.
    fn level_cheapest(&mut self) {
        for _ in 0..500 {
            let mut best: Option<(CatId, f64)> = None;
            for c in CatId::ALL {
                let q = self.g.level_quote(c, BuyMode::One);
                if !q.affordable {
                    continue;
                }
                let score = q.cost / role_weight(c);
                if best.is_none_or(|(_, b)| score < b) {
                    best = Some((c, score));
                }
            }
            let Some((c, _)) = best else { break };
            if !self.buy(Command::LevelUp {
                cat: c,
                mode: BuyMode::One,
            }) {
                break;
            }
        }
    }

    fn act_greedy(&mut self) {
        let g = &mut self.g;
        if let Some(t) = g.pending_tutorial() {
            let _ = g.command(Command::DismissTutorial { id: t });
        }
        if g.carton().is_some_and(|b| b.opening.is_none()) {
            let _ = g.command(Command::OpenBox);
        }
        let hud = g.hud();
        if hud.catnip_active <= 0.0 && (hud.catnip_cd <= 0.0 || hud.catnip_charges > 0) {
            let _ = g.command(Command::Catnip);
        }
        if hud.farming {
            let _ = g.command(Command::RetryBoss);
        }
        // Chín Mạng: khi hồn nhận được ≥ max(3, tổng hồn đã có — gồm phần đã tiêu ở cửa hàng)
        let pp = self.g.prestige_preview();
        if !self.no_prestige
            && pp.available
            && pp.souls_gain >= (3.0f64).max(self.souls_total)
            && self.g.command(Command::Prestige).is_ok()
        {
            self.souls_total += pp.souls_gain;
            if self.rep.first_prestige.is_none() {
                self.rep.first_prestige = Some(self.t);
            }
            self.cur_max_gs = self.g.hud().max_global_stage;
            self.last_gs_change = self.t;
        }
        for c in [CatId::Xiem, CatId::Map, CatId::Mun] {
            let _ = self.buy(Command::Unlock { cat: c });
        }
        // cửa hàng hồn: mua món rẻ nhất
        loop {
            let best = self
                .g
                .soul_shop()
                .into_iter()
                .filter(|r| r.affordable)
                .min_by(|a, b| {
                    a.cost
                        .partial_cmp(&b.cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            match best {
                Some(r) if self.buy(Command::BuySoul { upgrade: r.upgrade }) => {}
                _ => break,
            }
        }
        let _ = self.g.command(Command::EquipBest { cat: None });
        // tài năng: ưu tiên power, rồi tài năng riêng, rồi vigor
        for c in CatId::ALL {
            for _ in 0..20 {
                let info = self.g.cat_info(c);
                if !info.unlocked || info.talent_points == 0 {
                    break;
                }
                let order = [0u8, 2, 1];
                let slot = order
                    .into_iter()
                    .find(|&s| info.talents[s as usize].1 < TALENT_MAX_RANK);
                match slot {
                    Some(s) if self.buy(Command::TalentUp { cat: c, slot: s }) => {}
                    _ => break,
                }
            }
        }
        // nâng đồ đang mặc khi dư len (rẻ nhất trước)
        for _ in 0..50 {
            let mut best: Option<(ItemId, f64)> = None;
            for c in CatId::ALL {
                let info = self.g.cat_info(c);
                for it in info.equipped.iter().flatten() {
                    if let Some(cost) = self.g.upgrade_cost(it.id)
                        && best.is_none_or(|(_, b)| cost < b)
                    {
                        best = Some((it.id, cost));
                    }
                }
            }
            match best {
                Some((id, cost)) if cost <= self.g.hud().yarn => {
                    if !self.buy(Command::UpgradeItem { item: id }) {
                        break;
                    }
                }
                _ => break,
            }
        }
        // kho gần đầy: rã món kém nhất
        let inv = self.g.inventory();
        if inv.len() + 4 >= self.g.inventory_cap() {
            for it in inv.iter().take(8) {
                let _ = self.g.command(Command::Salvage { item: it.id });
            }
        }
        // để dành cá cho mèo kế tiếp khi đã đủ điều kiện màn
        let saving = [CatId::Xiem, CatId::Map, CatId::Mun].iter().any(|&c| {
            let i = self.g.cat_info(c);
            !i.unlocked && i.unlock_reachable
        });
        if !saving {
            self.level_cheapest();
        }
    }

    fn act_idle(&mut self) {
        if let Some(t) = self.g.pending_tutorial() {
            let _ = self.g.command(Command::DismissTutorial { id: t });
        }
        if self.t - self.last_idle_buy >= 1800.0 {
            self.last_idle_buy = self.t;
            self.level_cheapest();
        }
    }

    fn observe(&mut self) {
        for e in self.g.drain_events() {
            match e {
                Event::CatUnlocked { cat } => {
                    let slot = match cat {
                        CatId::Xiem => &mut self.rep.unlock_xiem,
                        CatId::Map => &mut self.rep.unlock_map,
                        CatId::Mun => &mut self.rep.unlock_mun,
                        CatId::Muop => continue,
                    };
                    slot.get_or_insert(self.t);
                }
                Event::BossDefeated {
                    kind: EnemyKind::RatKing,
                } => {
                    self.rep.first_rat_king.get_or_insert(self.t);
                }
                Event::ZoneEntered { zone: 3, cycle: 0 } => {
                    self.rep.reach_zone3.get_or_insert(self.t);
                }
                _ => {}
            }
        }
        let hud = self.g.hud();
        self.rep.best_gs = self.rep.best_gs.max(hud.max_global_stage);
        if hud.max_global_stage != self.cur_max_gs {
            if self.trace {
                let lv: Vec<String> = CatId::ALL
                    .iter()
                    .map(|&c| self.g.cat_info(c).level.to_string())
                    .collect();
                println!(
                    "  t={:>6.1}m gs {:>3} (+{:.1}m) cấp {} cá {} mạng {}",
                    self.t / 60.0,
                    hud.max_global_stage,
                    (self.t - self.last_gs_change) / 60.0,
                    lv.join("/"),
                    fmt_num(hud.fish),
                    self.g.prestige_preview().lives_used
                );
            }
            self.cur_max_gs = hud.max_global_stage;
            self.last_gs_change = self.t;
        }
        if self.rep.first_prestige.is_none() {
            self.rep.max_stall = self.rep.max_stall.max(self.t - self.last_gs_change);
        }
    }

    fn run(&mut self, hours: f64, mut progress: impl FnMut(&Bot)) {
        let start = Instant::now();
        let total = (hours * 3600.0) as u64;
        for sec in 1..=total {
            for _ in 0..TICKS_PER_SEC {
                self.g.tick();
            }
            self.t = sec as f64;
            self.observe();
            match self.kind {
                BotKind::Greedy => self.act_greedy(),
                BotKind::Idle => self.act_idle(),
            }
            self.observe();
            progress(self);
        }
        self.rep.hours = hours;
        self.rep.lives_used = self.g.prestige_preview().lives_used;
        let gaps: Vec<f64> = {
            let p: Vec<f64> = self
                .purchases
                .iter()
                .copied()
                .filter(|&t| t <= 7200.0)
                .collect();
            let mut d: Vec<f64> = p
                .windows(2)
                .map(|w| w[1] - w[0])
                .filter(|&x| x > 0.0)
                .collect();
            d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            d
        };
        self.rep.median_buy_gap = if gaps.is_empty() {
            f64::INFINITY
        } else {
            gaps[gaps.len() / 2]
        };
        self.rep.cpu_secs = start.elapsed().as_secs_f64();
    }
}

fn fmt_t(t: Option<f64>) -> String {
    match t {
        Some(s) => format!("{:.1}m", s / 60.0),
        None => "—".into(),
    }
}

fn parse_args(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).cloned())
}

fn cmd_sim(args: &[String]) -> ExitCode {
    let hours: f64 = parse_args(args, "--hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);
    let seed: u64 = parse_args(args, "--seed")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let every: f64 = parse_args(args, "--every")
        .and_then(|v| v.parse().ok())
        .unwrap_or(10.0);
    let kind = match parse_args(args, "--bot").as_deref() {
        Some("idle") => BotKind::Idle,
        Some("greedy") | None => BotKind::Greedy,
        Some(other) => {
            eprintln!("bot không hợp lệ: {other} (greedy|idle)");
            return ExitCode::from(2);
        }
    };
    let mut bot = Bot::new(kind, seed);
    bot.no_prestige = args.iter().any(|a| a == "--no-prestige");
    bot.trace = args.iter().any(|a| a == "--trace");
    println!(
        "{:>7} {:>5} {:>8} {:>6} {:>9} {:>8} {:>6} {:>17} {:>5}",
        "phút", "vòng", "khu·màn", "max", "cá", "len", "hồn", "cấp mèo", "mạng"
    );
    let step = (every * 60.0).max(1.0) as u64;
    bot.run(hours, |b| {
        if (b.t as u64).is_multiple_of(step) {
            let h = b.g.hud();
            let lv: Vec<String> = CatId::ALL
                .iter()
                .map(|&c| {
                    let i = b.g.cat_info(c);
                    if i.unlocked {
                        i.level.to_string()
                    } else {
                        "-".into()
                    }
                })
                .collect();
            println!(
                "{:>7.0} {:>5} {:>5}·{:<2} {:>6} {:>9} {:>8} {:>6} {:>17} {:>5}{}",
                b.t / 60.0,
                h.cycle,
                h.zone,
                h.stage,
                h.max_global_stage,
                fmt_num(h.fish),
                fmt_num(h.yarn),
                fmt_num(h.soul),
                lv.join("/"),
                b.g.prestige_preview().lives_used,
                if h.farming { " (cày)" } else { "" }
            );
        }
    });
    let r = &bot.rep;
    println!();
    println!(
        "Xiêm {} · Mập {} · Mun {} · Vua Chuột {} · khu 3 {} · prestige đầu {} · mạng {} · max gs {} · stall {:.1}m · trung vị mua {:.0}s · CPU {:.2}s",
        fmt_t(r.unlock_xiem),
        fmt_t(r.unlock_map),
        fmt_t(r.unlock_mun),
        fmt_t(r.first_rat_king),
        fmt_t(r.reach_zone3),
        fmt_t(r.first_prestige),
        r.lives_used,
        r.best_gs,
        r.max_stall / 60.0,
        r.median_buy_gap,
        r.cpu_secs
    );
    ExitCode::SUCCESS
}

struct Gate {
    name: &'static str,
    value: String,
    pass: bool,
}

fn avg(v: &[Option<f64>]) -> Option<f64> {
    if v.iter().any(|x| x.is_none()) {
        return None;
    }
    Some(v.iter().map(|x| x.unwrap_or(0.0)).sum::<f64>() / v.len() as f64)
}

fn run_jobs(jobs: Vec<(BotKind, u64, f64)>, threads: usize) -> Vec<(BotKind, Report)> {
    let queue = Arc::new(Mutex::new(jobs.into_iter().enumerate().collect::<Vec<_>>()));
    let out = Arc::new(Mutex::new(Vec::new()));
    let mut hs = Vec::new();
    for _ in 0..threads.max(1) {
        let q = Arc::clone(&queue);
        let o = Arc::clone(&out);
        hs.push(std::thread::spawn(move || {
            loop {
                let job = q.lock().ok().and_then(|mut v| v.pop());
                let Some((idx, (kind, seed, hours))) = job else {
                    break;
                };
                let mut b = Bot::new(kind, seed);
                b.run(hours, |_| {});
                eprintln!(
                    "  xong {:?} seed {} ({} h game, {:.1} s CPU)",
                    kind, seed, hours, b.rep.cpu_secs
                );
                if let Ok(mut o) = o.lock() {
                    o.push((idx, kind, b.rep));
                }
            }
        }));
    }
    for h in hs {
        let _ = h.join();
    }
    let mut v = Arc::try_unwrap(out)
        .map(|m| m.into_inner().unwrap_or_default())
        .unwrap_or_default();
    v.sort_by_key(|(i, _, _)| *i);
    v.into_iter().map(|(_, k, r)| (k, r)).collect()
}

fn cmd_gates(args: &[String]) -> ExitCode {
    let threads: usize = parse_args(args, "--threads")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let seeds: Vec<u64> = (1..=5).collect();
    let mut jobs = Vec::new();
    for &s in &seeds {
        jobs.push((BotKind::Greedy, s, 24.0));
    }
    for &s in &seeds {
        jobs.push((BotKind::Idle, s, 8.0));
    }
    let t0 = Instant::now();
    let res = run_jobs(jobs, threads);
    let greedy: Vec<&Report> = res
        .iter()
        .filter(|(k, _)| *k == BotKind::Greedy)
        .map(|(_, r)| r)
        .collect();
    let idle: Vec<&Report> = res
        .iter()
        .filter(|(k, _)| *k == BotKind::Idle)
        .map(|(_, r)| r)
        .collect();

    println!();
    println!(
        "{:>4} {:>7} {:>7} {:>7} {:>8} {:>7} {:>9} {:>5} {:>6} {:>8} {:>8} {:>8}",
        "seed",
        "Xiêm",
        "Mập",
        "Mun",
        "VuaChuột",
        "khu3",
        "prestige",
        "mạng",
        "max gs",
        "stall",
        "mua(tv)",
        "CPU/h"
    );
    for r in &greedy {
        println!(
            "{:>4} {:>7} {:>7} {:>7} {:>8} {:>7} {:>9} {:>5} {:>6} {:>7.1}m {:>7.0}s {:>7.2}s",
            r.seed,
            fmt_t(r.unlock_xiem),
            fmt_t(r.unlock_map),
            fmt_t(r.unlock_mun),
            fmt_t(r.first_rat_king),
            fmt_t(r.reach_zone3),
            fmt_t(r.first_prestige),
            r.lives_used,
            r.best_gs,
            r.max_stall / 60.0,
            r.median_buy_gap,
            r.cpu_secs / r.hours
        );
    }
    for r in &idle {
        println!(
            "idle seed {}: max gs {} (vòng {} khu {}) sau {} h",
            r.seed,
            r.best_gs,
            r.best_gs / 50,
            r.best_gs / 10 % 5 + 1,
            r.hours
        );
    }

    let col =
        |f: &dyn Fn(&Report) -> Option<f64>| avg(&greedy.iter().map(|r| f(r)).collect::<Vec<_>>());
    let min = 60.0;
    let in_range = |v: Option<f64>, lo: f64, hi: f64| v.is_some_and(|x| x >= lo && x <= hi);
    let below = |v: Option<f64>, hi: f64| v.is_some_and(|x| x < hi);
    let show = |v: Option<f64>, unit: f64, suffix: &str| match v {
        Some(x) => format!("{:.1}{suffix}", x / unit),
        None => "không đạt".into(),
    };

    let xiem = col(&|r| r.unlock_xiem);
    let map = col(&|r| r.unlock_map);
    let mun = col(&|r| r.unlock_mun);
    let rk = col(&|r| r.first_rat_king);
    let z3 = col(&|r| r.reach_zone3);
    let pr = col(&|r| r.first_prestige);
    let lives = col(&|r| Some(r.lives_used as f64));
    let gs = col(&|r| Some(r.best_gs as f64));
    let stall = greedy.iter().map(|r| r.max_stall).fold(0.0f64, f64::max);
    let buy = col(&|r| Some(r.median_buy_gap));
    let idle_min_gs = idle.iter().map(|r| r.best_gs).min().unwrap_or(0);
    let cpu_h = res
        .iter()
        .map(|(_, r)| r.cpu_secs / r.hours)
        .fold(0.0f64, f64::max);

    let gates = vec![
        Gate {
            name: "Mở Xiêm < 3 phút",
            value: show(xiem, min, " phút"),
            pass: below(xiem, 3.0 * min),
        },
        Gate {
            name: "Mở Mập < 15 phút",
            value: show(map, min, " phút"),
            pass: below(map, 15.0 * min),
        },
        Gate {
            name: "Mở Mun < 50 phút",
            value: show(mun, min, " phút"),
            pass: below(mun, 50.0 * min),
        },
        Gate {
            name: "Hạ boss khu 1 trong 4–12 phút",
            value: show(rk, min, " phút"),
            pass: in_range(rk, 4.0 * min, 12.0 * min),
        },
        Gate {
            name: "Đạt khu 3 trong 30–75 phút",
            value: show(z3, min, " phút"),
            pass: in_range(z3, 30.0 * min, 75.0 * min),
        },
        Gate {
            name: "Prestige đầu trong 1.5–4 giờ",
            value: show(pr, 3600.0, " giờ"),
            pass: in_range(pr, 1.5 * 3600.0, 4.0 * 3600.0),
        },
        Gate {
            name: "24 h: dùng 3–7 mạng",
            value: show(lives, 1.0, ""),
            pass: in_range(lives, 3.0, 7.0),
        },
        Gate {
            name: "24 h: max_global_stage 60–160",
            value: show(gs, 1.0, ""),
            pass: in_range(gs, 60.0, 160.0),
        },
        Gate {
            name: "Không tường > 25 phút (trước prestige đầu, mọi seed)",
            value: format!("{:.1} phút", stall / 60.0),
            pass: stall <= 25.0 * min,
        },
        Gate {
            name: "Trung vị khoảng mua (2 h đầu) ≤ 4 phút",
            value: show(buy, 1.0, " s"),
            pass: below(buy, 4.0 * min + 1e-9),
        },
        Gate {
            name: "IdleBot 8 h đạt ít nhất khu 2 (mọi seed)",
            value: format!("min gs {idle_min_gs}"),
            pass: idle_min_gs >= 10,
        },
        Gate {
            name: "1 giờ game < 2 s CPU",
            value: format!("{cpu_h:.2} s"),
            pass: cpu_h < 2.0,
        },
    ];
    println!();
    println!(
        "{:<55} {:>14}  kết quả",
        "Cổng (GreedyBot seed 1..5, trung bình)", "giá trị"
    );
    let mut all = true;
    for g in &gates {
        println!(
            "{:<55} {:>14}  {}",
            g.name,
            g.value,
            if g.pass { "ĐẠT" } else { "TRƯỢT" }
        );
        all &= g.pass;
    }
    println!();
    println!("Tổng thời gian: {:.1} s", t0.elapsed().as_secs_f64());
    if all {
        println!("TẤT CẢ CỔNG ĐẠT");
        ExitCode::SUCCESS
    } else {
        println!("CÓ CỔNG TRƯỢT");
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sim") => cmd_sim(&args[1..]),
        Some("gates") => cmd_gates(&args[1..]),
        _ => {
            eprintln!(
                "dùng: cm-bot sim --hours H --seed S --bot greedy|idle [--every PHÚT]\n      cm-bot gates [--threads N]"
            );
            ExitCode::from(2)
        }
    }
}
