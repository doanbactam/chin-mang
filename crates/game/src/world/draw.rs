//! Dựng danh sách lệnh vẽ của thế giới (tọa độ logic nguyên, y hướng lên, anchor theo atlas).
//! Mặt đất y = 0; đáy texture ở y = `GROUND_BOTTOM`.

use super::anim::{oneshot_index, sprite_name};
use super::fx::Fx;
use crate::assets::{AtlasDef, Frame};
use crate::sim::Sim;
use bevy::prelude::*;
use cm_core::{Anim, DropKind, Game, STRIP_H};

/// Đáy dải (texture) so với mặt đất.
pub const GROUND_BOTTOM: i32 = -5;
pub const TILE_W: i32 = 16;
pub const DECO_SPACING: i32 = 72;
/// Đội đứng ở ~35% bề rộng.
pub const PARTY_FRAC: f32 = 0.35;

#[derive(Clone, Debug, PartialEq)]
pub struct DrawCmd {
    pub name: String,
    /// góc trên-trái (px logic, y hướng lên)
    pub left: i32,
    pub top: i32,
    pub z: f32,
    pub flip: bool,
    pub tint: Color,
    /// cắt một phần frame: (x, y, w, h) tính từ góc frame
    pub sub: Option<(u32, u32, u32, u32)>,
    /// kích thước vẽ (px logic) nếu khác frame
    pub size: Option<(u32, u32)>,
}

impl DrawCmd {
    fn new(name: String, left: i32, top: i32, z: f32) -> DrawCmd {
        DrawCmd {
            name,
            left,
            top,
            z,
            flip: false,
            tint: Color::WHITE,
            sub: None,
            size: None,
        }
    }
}

/// Góc trên-trái của sprite đặt anchor tại (x, y). `flip` lật quanh anchor.
pub fn place(f: &Frame, x: i32, y: i32, flip: bool) -> (i32, i32) {
    let ax = if flip { f.w as i32 - f.ax } else { f.ax };
    (x - ax, y + f.ay)
}

/// Băm nguyên tất định (lát nền / trang trí).
pub fn hash(i: i64) -> u64 {
    let mut z = (i as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mép trái camera (nguyên) để đội đứng ở ~35% bề rộng.
pub fn camera_left(party_x: f32, world_w: i32) -> i32 {
    party_x.round() as i32 - (world_w as f32 * PARTY_FRAC).round() as i32
}

/// Các ô nền nhìn thấy: (x, biến thể).
pub fn ground_tiles(left: i32, world_w: i32) -> Vec<(i32, u64)> {
    let first = left.div_euclid(TILE_W) - 1;
    let last = (left + world_w).div_euclid(TILE_W) + 1;
    (first..=last)
        .map(|i| (i * TILE_W, hash(i as i64) % 4))
        .collect()
}

/// Đồ trang trí nhìn thấy: (x, biến thể).
pub fn deco_slots(left: i32, world_w: i32, zone: u32) -> Vec<(i32, u64)> {
    let first = left.div_euclid(DECO_SPACING) - 1;
    let last = (left + world_w).div_euclid(DECO_SPACING) + 1;
    (first..=last)
        .filter_map(|j| {
            let h = hash(j as i64 * 31 + zone as i64 * 7919);
            (h % 5 >= 2).then(|| (j * DECO_SPACING + (h >> 8) as i32 % 40, (h >> 16) % 4))
        })
        .collect()
}

/// Ký tự số sát thương → tên glyph `fd_*`.
pub fn digit_glyphs(s: &str) -> Vec<String> {
    s.chars()
        .filter_map(|c| {
            Some(match c {
                '0'..='9' => format!("fd_{c}"),
                '.' => "fd_dot".into(),
                'K' | 'M' | 'B' | 'T' => format!("fd_{c}"),
                '+' => "fd_plus".into(),
                '-' => "fd_minus".into(),
                _ => return None,
            })
        })
        .collect()
}

pub struct Input<'a> {
    pub game: &'a Game,
    pub sim: &'a Sim,
    pub alpha: f32,
    pub world_w: i32,
    pub atlas: &'a AtlasDef,
    /// con trỏ (tọa độ logic)
    pub cursor: Option<Vec2>,
    pub fx: &'a Fx,
    pub now: f32,
}

const Z_DECO: f32 = 1.0;
const Z_GROUND: f32 = 2.0;
const Z_SHADOW: f32 = 3.0;
const Z_DROP: f32 = 4.0;
const Z_ENEMY: f32 = 5.0;
const Z_CAT: f32 = 6.0;
const Z_PROJ: f32 = 7.0;
const Z_FX: f32 = 8.0;
const Z_BAR: f32 = 9.0;
const Z_NUM: f32 = 10.0;

fn push(
    out: &mut Vec<DrawCmd>,
    atlas: &AtlasDef,
    name: &str,
    x: i32,
    y: i32,
    flip: bool,
    z: f32,
    tint: Color,
) {
    if let Some(f) = atlas.get(name) {
        let (l, t) = place(f, x, y, flip);
        out.push(DrawCmd {
            flip,
            tint,
            ..DrawCmd::new(name.to_string(), l, t, z + out.len() as f32 * 1e-4)
        });
    }
}

fn hp_bar(
    out: &mut Vec<DrawCmd>,
    atlas: &AtlasDef,
    cx: i32,
    top: i32,
    w: u32,
    frac: f32,
    fill: &str,
) {
    // một hàng pixel giữa của sprite thanh 8×4, vẽ dài `w`
    let n = out.len() as f32 * 1e-4;
    if atlas.get("ui_bar_bg").is_some() {
        out.push(DrawCmd {
            sub: Some((1, 1, 6, 1)),
            size: Some((w + 2, 3)),
            ..DrawCmd::new(
                "ui_bar_bg".into(),
                cx - w as i32 / 2 - 1,
                top + 1,
                Z_BAR + n,
            )
        });
    }
    let fw = ((w as f32) * frac.clamp(0.0, 1.0)).round() as u32;
    if fw > 0 && atlas.get(fill).is_some() {
        out.push(DrawCmd {
            sub: Some((2, 1, 4, 1)),
            size: Some((fw, 1)),
            ..DrawCmd::new(fill.into(), cx - w as i32 / 2, top, Z_BAR + n + 1e-5)
        });
    }
}

/// Trả về (mép trái camera, lệnh vẽ).
pub fn build(i: &Input) -> (i32, Vec<DrawCmd>) {
    let a = i.atlas;
    let game = i.game;
    let hud = game.hud();
    let party = i.sim.lerp(u32::MAX, Vec2::new(hud.party_x, 0.0), i.alpha).x;
    let left = camera_left(party, i.world_w);
    let mut out: Vec<DrawCmd> = Vec::with_capacity(256);
    let zone = hud.zone.clamp(1, 5);
    let len = |b: &str| a.anim_len(b);

    // ── nền ──
    for (x, v) in deco_slots(left, i.world_w, zone) {
        push(
            &mut out,
            a,
            &format!("deco_z{zone}_{v}"),
            x,
            0,
            x % 2 == 0,
            Z_DECO,
            Color::srgb(0.86, 0.86, 0.92),
        );
    }
    for (x, v) in ground_tiles(left, i.world_w) {
        let name = format!("ground_z{zone}_{v}");
        if a.get(&name).is_some() {
            out.push(DrawCmd::new(name, x, 1, Z_GROUND));
        }
    }

    // ── đồ rơi + hộp ──
    for d in game.drops() {
        let p = i.sim.lerp(d.entity, Vec2::new(d.x, d.y), i.alpha);
        let name = match d.kind {
            DropKind::Fish => "drop_fish".to_string(),
            DropKind::Yarn => "drop_yarn".to_string(),
            DropKind::Gear(r) => format!("drop_gear_{}", r.key()),
        };
        let bob = if d.collect_t <= 0.0 {
            ((i.now * 4.0 + d.entity as f32).sin() * 1.5).round() as i32
        } else {
            0
        };
        push(
            &mut out,
            a,
            &name,
            p.x.round() as i32,
            p.y.round() as i32 + bob.max(0),
            false,
            Z_DROP,
            Color::WHITE,
        );
    }
    if let Some(b) = game.carton() {
        let p = i.sim.lerp(b.entity, Vec2::new(b.x, b.y), i.alpha);
        let (x, y) = (p.x.round() as i32, p.y.round() as i32);
        push(&mut out, a, "shadow_s", x, 0, false, Z_SHADOW, Color::WHITE);
        let blink = b.opening.is_none() && b.time_left < 10.0 && (i.now * 4.0).fract() < 0.3;
        let name = match b.opening {
            Some(t) => format!("box_open_{}", ((t * 3.0) as usize).min(2)),
            None => "box_closed".to_string(),
        };
        if !blink {
            push(&mut out, a, &name, x, y, false, Z_DROP, Color::WHITE);
        }
    }

    // ── quái ──
    for e in game.enemies() {
        let p = i.sim.lerp(e.entity, Vec2::new(e.x, e.y), i.alpha);
        let (x, y) = (p.x.round() as i32, p.y.round() as i32);
        let boss = e.kind.is_boss();
        if e.anim != Anim::Death || e.anim_t < 0.3 {
            push(
                &mut out,
                a,
                if boss { "shadow_l" } else { "shadow_s" },
                x,
                0,
                false,
                Z_SHADOW,
                Color::WHITE,
            );
        }
        let prefix = if boss {
            format!("boss_{}", e.kind.key())
        } else {
            format!("en_{}", e.kind.key())
        };
        let tint = if e.cursed {
            Color::srgb(0.85, 0.65, 1.0)
        } else if e.poisoned {
            Color::srgb(0.7, 1.0, 0.7)
        } else {
            Color::WHITE
        };
        if let Some(name) = sprite_name(&prefix, e.anim, e.anim_t, len) {
            // quái vốn quay trái: lật khi quay phải
            push(&mut out, a, &name, x, y, e.facing > 0, Z_ENEMY, tint);
        }
        let h = if boss { 32 } else { 16 };
        if e.anim != Anim::Death && e.hp < e.hp_max && !boss {
            hp_bar(
                &mut out,
                a,
                x,
                y + h + 2,
                10,
                (e.hp / e.hp_max.max(1.0)) as f32,
                "ui_bar_boss",
            );
        }
        if e.stunned {
            push(
                &mut out,
                a,
                &fx_frame(a, "stun", i.now, 0.5),
                x,
                y + h + 6,
                false,
                Z_FX,
                Color::WHITE,
            );
        }
    }

    // ── mèo ──
    for c in game.cats() {
        let p = i.sim.lerp(c.entity, Vec2::new(c.x, c.y), i.alpha);
        let (x, y) = (p.x.round() as i32, p.y.round() as i32);
        push(&mut out, a, "shadow_s", x, 0, false, Z_SHADOW, Color::WHITE);
        let mut flip = c.facing < 0;
        // nhìn theo con trỏ khi đứng/đi (chỉ cosmetic)
        if let Some(cur) = i.cursor
            && matches!(c.anim, Anim::Idle | Anim::Walk | Anim::Sleep)
            && (cur.x - p.x).abs() <= 24.0
            && (cur.x - p.x).abs() >= 2.0
        {
            flip = cur.x < p.x;
        }
        if let Some(name) = sprite_name(&format!("cat_{}", c.cat.key()), c.anim, c.anim_t, len) {
            push(&mut out, a, &name, x, y, flip, Z_CAT, Color::WHITE);
        }
        if c.alive && (c.hp < c.hp_max || c.in_combat) {
            hp_bar(
                &mut out,
                a,
                x,
                y + 17,
                12,
                (c.hp / c.hp_max.max(1.0)) as f32,
                "ui_bar_hp",
            );
        }
        if c.stunned {
            push(
                &mut out,
                a,
                &fx_frame(a, "stun", i.now, 0.5),
                x,
                y + 22,
                false,
                Z_FX,
                Color::WHITE,
            );
        }
    }

    // ── đạn ──
    for pr in game.projectiles() {
        let p = i.sim.lerp(pr.entity, Vec2::new(pr.x, pr.y), i.alpha);
        let base = format!("proj_{}", pr.kind.key());
        let name = if a.get(&base).is_some() {
            base
        } else {
            format!("{base}_{}", ((pr.age * 10.0) as usize) % len(&base).max(1))
        };
        push(
            &mut out,
            a,
            &name,
            p.x.round() as i32,
            p.y.round() as i32,
            pr.vx < 0.0,
            Z_PROJ,
            Color::WHITE,
        );
    }

    // ── hiệu ứng + số nổi ──
    for f in &i.fx.effects {
        let n = len(&format!("fx_{}", f.kind));
        if let Some(idx) = oneshot_index(f.t, f.dur, n) {
            push(
                &mut out,
                a,
                &format!("fx_{}_{idx}", f.kind),
                f.x,
                f.y,
                false,
                Z_FX,
                Color::WHITE,
            );
        }
    }
    for num in &i.fx.numbers {
        let glyphs = digit_glyphs(&num.text);
        let k = if num.crit { 2 } else { 1 };
        let adv = 4 * k;
        let total = glyphs.len() as i32 * adv;
        let rise = (num.t / num.dur * 12.0).round() as i32;
        let x0 = num.x - total / 2;
        let top = num.y + rise + 6 * k;
        let fade = 1.0 - (num.t / num.dur - 0.6).max(0.0) / 0.4;
        let tint = num.color.with_alpha(fade.clamp(0.0, 1.0));
        for (gi, g) in glyphs.iter().enumerate() {
            if a.get(g).is_some() {
                let size = (k > 1).then_some((4 * k as u32, 6 * k as u32));
                out.push(DrawCmd {
                    tint,
                    size,
                    ..DrawCmd::new(
                        g.clone(),
                        x0 + gi as i32 * adv,
                        top,
                        Z_NUM + out.len() as f32 * 1e-4,
                    )
                });
            }
        }
    }
    // không vẽ gì lọt ra ngoài texture theo chiều dọc
    out.retain(|c| c.top > GROUND_BOTTOM - 1 && c.top - 64 < GROUND_BOTTOM + STRIP_H as i32);
    (left, out)
}

/// Frame lặp của hiệu ứng `fx_{kind}` theo thời gian thực.
fn fx_frame(a: &AtlasDef, kind: &str, now: f32, period: f32) -> String {
    let n = a.anim_len(&format!("fx_{kind}")).max(1);
    format!("fx_{kind}_{}", ((now / period * n as f32) as usize) % n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32, ax: i32, ay: i32) -> Frame {
        Frame {
            x: 0,
            y: 0,
            w,
            h,
            ax,
            ay,
            slice: 0,
        }
    }

    #[test]
    fn anchor_placement_and_flip() {
        let f = frame(16, 16, 8, 16); // đáy-giữa
        assert_eq!(place(&f, 100, 0, false), (92, 16));
        let g = frame(16, 16, 7, 16);
        assert_eq!(place(&g, 100, 0, false), (93, 16));
        assert_eq!(place(&g, 100, 0, true), (91, 16));
    }

    #[test]
    fn camera_keeps_party_at_35_percent() {
        assert_eq!(camera_left(1000.4, 960), 1000 - 336);
    }

    #[test]
    fn tiles_cover_view_without_gaps() {
        let t = ground_tiles(-37, 200);
        assert!(t.first().unwrap().0 <= -37);
        assert!(t.last().unwrap().0 + TILE_W >= 163);
        for w in t.windows(2) {
            assert_eq!(w[1].0 - w[0].0, TILE_W);
        }
        // tất định
        assert_eq!(ground_tiles(-37, 200), t);
    }

    #[test]
    fn deco_is_deterministic_and_sparse() {
        let a = deco_slots(0, 960, 2);
        assert_eq!(a, deco_slots(0, 960, 2));
        assert!(!a.is_empty() && a.len() < 16);
        assert!(a.iter().all(|(_, v)| *v < 4));
    }

    #[test]
    fn digits_map_to_font_glyphs() {
        assert_eq!(digit_glyphs("1.2K"), vec!["fd_1", "fd_dot", "fd_2", "fd_K"]);
        assert_eq!(digit_glyphs("+5"), vec!["fd_plus", "fd_5"]);
    }
}
