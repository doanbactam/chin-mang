//! Hiệu ứng theo `Event` của lõi: fx sprite một lần, số sát thương nổi (`fd_*`), rung khi boss xuất hiện.

use super::WorldCam;
use crate::sim::Sim;
use bevy::prelude::*;
use cm_core::{CatId, Event as CoreEvent, fmt_num};

#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub kind: &'static str,
    pub x: i32,
    pub y: i32,
    pub t: f32,
    pub dur: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Number {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub t: f32,
    pub dur: f32,
    pub crit: bool,
    pub color: Color,
}

#[derive(Resource, Default)]
pub struct Fx {
    pub effects: Vec<Effect>,
    pub numbers: Vec<Number>,
}

pub const MAX_NUMBERS: usize = 24;
pub const MAX_EFFECTS: usize = 32;

impl Fx {
    pub fn advance(&mut self, dt: f32) {
        for e in &mut self.effects {
            e.t += dt;
        }
        for n in &mut self.numbers {
            n.t += dt;
        }
        self.effects.retain(|e| e.t < e.dur);
        self.numbers.retain(|n| n.t < n.dur);
    }

    pub fn effect(&mut self, kind: &'static str, x: f32, y: f32, dur: f32) {
        if self.effects.len() >= MAX_EFFECTS {
            self.effects.remove(0);
        }
        self.effects.push(Effect {
            kind,
            x: x.round() as i32,
            y: y.round() as i32,
            t: 0.0,
            dur,
        });
    }

    pub fn number(&mut self, amount: f64, x: f32, y: f32, crit: bool, to_cat: bool, heal: bool) {
        if self.numbers.len() >= MAX_NUMBERS {
            self.numbers.remove(0);
        }
        let color = if heal {
            Color::srgb_u8(0x7c, 0xc4, 0x5a)
        } else if to_cat {
            Color::srgb_u8(0xf2, 0x8b, 0x6c)
        } else if crit {
            Color::srgb_u8(0xff, 0xe2, 0x7a)
        } else {
            Color::WHITE
        };
        let text = if heal {
            format!("+{}", fmt_num(amount.ceil()))
        } else {
            fmt_num(amount.ceil())
        };
        self.numbers.push(Number {
            text,
            x: x.round() as i32,
            y: y.round() as i32,
            t: 0.0,
            dur: if crit { 1.0 } else { 0.8 },
            crit,
            color,
        });
    }

    /// Áp một sự kiện lõi (thuần, test được). `cat_pos` tra vị trí mèo.
    pub fn on_event(&mut self, ev: &CoreEvent, cat_pos: impl Fn(CatId) -> Option<Vec2>) -> bool {
        let mut shake = false;
        match *ev {
            CoreEvent::Damage {
                x,
                y,
                amount,
                crit,
                to_cat,
                ..
            } => {
                self.effect(if crit { "crit" } else { "hit" }, x, y + 8.0, 0.3);
                self.number(amount, x, y + 12.0, crit, to_cat, false);
            }
            CoreEvent::Heal { x, y, amount, .. } => {
                self.number(amount, x, y + 12.0, false, false, true)
            }
            CoreEvent::CatSkill { cat, x } => {
                let kind = match cat {
                    CatId::Muop => "slash",
                    CatId::Map => "aoe",
                    CatId::Mun => "poison",
                    CatId::Xiem => "hit",
                };
                self.effect(kind, x, 8.0, 0.45);
            }
            CoreEvent::EnemyDied { x, y, .. } => self.effect("poof", x, y + 8.0, 0.4),
            CoreEvent::BossSpecial { x, .. } => self.effect("aoe", x, 10.0, 0.5),
            CoreEvent::Stunned { x, y, .. } => self.effect("stun", x, y + 18.0, 0.6),
            CoreEvent::LevelUp { cat, .. }
            | CoreEvent::CatRevive { cat }
            | CoreEvent::CatUnlocked { cat } => {
                if let Some(p) = cat_pos(cat) {
                    self.effect("levelup", p.x, p.y + 12.0, 0.6);
                }
            }
            CoreEvent::CatnipStarted => {
                for c in CatId::ALL {
                    if let Some(p) = cat_pos(c) {
                        self.effect("catnip", p.x, p.y + 10.0, 0.6);
                    }
                }
            }
            CoreEvent::BoxOpened { x } => self.effect("poof", x, 7.0, 0.4),
            CoreEvent::BossAppear { .. } => shake = true,
            _ => {}
        }
        shake
    }
}

pub fn collect(sim: Res<Sim>, mut fx: ResMut<Fx>, mut cam: ResMut<WorldCam>) {
    if sim.events.is_empty() {
        return;
    }
    let cats = sim.game.cats();
    let pos = |c: CatId| {
        cats.iter()
            .find(|v| v.cat == c)
            .map(|v| Vec2::new(v.x, v.y))
    };
    for ev in &sim.events {
        if fx.on_event(ev, pos) {
            cam.shake = 0.4;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_spawns_number_and_hit() {
        let mut fx = Fx::default();
        let shake = fx.on_event(
            &CoreEvent::Damage {
                target: 3,
                x: 10.4,
                y: 0.0,
                amount: 1234.0,
                crit: true,
                to_cat: false,
            },
            |_| None,
        );
        assert!(!shake);
        assert_eq!(fx.numbers[0].text, "1.23K");
        assert!(fx.numbers[0].crit);
        assert_eq!(fx.effects[0].kind, "crit");
        assert!(fx.on_event(
            &CoreEvent::BossAppear {
                kind: cm_core::EnemyKind::Eel
            },
            |_| None
        ));
        fx.advance(2.0);
        assert!(fx.numbers.is_empty() && fx.effects.is_empty());
    }
    #[test]
    fn caps_counts() {
        let mut fx = Fx::default();
        for _ in 0..100 {
            fx.number(1.0, 0.0, 0.0, false, false, false);
        }
        assert_eq!(fx.numbers.len(), MAX_NUMBERS);
    }
}
