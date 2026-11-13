//! Đồ rơi, trang bị, tự rã, hộp carton.

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::*;

impl Game {
    pub(crate) fn wake_mult(&self) -> f64 {
        if self.s.wake_buff > 0.0 {
            WAKE_FISH_MULT
        } else {
            1.0
        }
    }

    pub(crate) fn credit_fish(&mut self, a: f64) {
        self.s.fish += a;
        self.s.stats.fish_earned += a;
        self.s.income_cur_fish += a;
    }

    pub(crate) fn credit_yarn(&mut self, a: f64) {
        self.s.yarn += a;
        self.s.income_cur_yarn += a;
    }

    fn push_drop(&mut self, kind: DropKind, x: f64, y: f64, amount: f64, item: Option<Item>) {
        let entity = self.s.new_entity();
        self.s.drops.push(Drop {
            entity,
            kind,
            x0: x,
            y0: y,
            t: 0.0,
            amount,
            item,
        });
    }

    /// Phần thưởng khi quái `i` chết.
    pub(crate) fn spawn_loot(&mut self, i: usize) {
        let (kind, x, fish, gs) = {
            let e = &self.s.enemies[i];
            (e.kind, e.x, e.fish, e.gs)
        };
        let boss = kind.is_boss();
        let y = if kind.flies() { FLY_HEIGHT } else { 2.0 };
        let fish = fish * rules::fish_mult(&self.s) * self.wake_mult();
        self.push_drop(DropKind::Fish, x, y, fish, None);
        if boss {
            let yarn = rules::yarn_drop_amount(gs) * BOSS_YARN_MULT;
            self.push_drop(DropKind::Yarn, x + 4.0, y, yarn, None);
        } else if self.s.rng.chance(YARN_DROP_CHANCE) {
            self.push_drop(
                DropKind::Yarn,
                x + 3.0,
                y,
                rules::yarn_drop_amount(gs),
                None,
            );
        }
        let gear = boss
            || self
                .s
                .rng
                .chance(GEAR_DROP_CHANCE * rules::gear_chance_mult(&self.s));
        if gear {
            let item = self.roll_item(gs, boss);
            self.emit(Event::GearDropped {
                item: item.id,
                rarity: item.rarity,
                x: x as f32,
            });
            self.push_drop(DropKind::Gear(item.rarity), x - 3.0, y, 0.0, Some(item));
        }
    }

    pub(crate) fn roll_rarity(&mut self, min_rare: bool) -> Rarity {
        let start = if min_rare { 1 } else { 0 };
        let total: f64 = RARITY_WEIGHTS[start..].iter().sum();
        let mut r = self.s.rng.f64() * total;
        for (k, w) in RARITY_WEIGHTS.iter().enumerate().skip(start) {
            if r < *w {
                return Rarity::ALL[k];
            }
            r -= w;
        }
        Rarity::Legendary
    }

    pub(crate) fn make_item(&mut self, slot: Slot, rarity: Rarity, ilvl: u32, upgrade: u8) -> Item {
        let id = self.s.next_item;
        self.s.next_item += 1;
        Item {
            id,
            slot,
            rarity,
            ilvl,
            upgrade: upgrade.min(ITEM_MAX_UPGRADE),
        }
    }

    pub(crate) fn roll_item(&mut self, ilvl: u32, min_rare: bool) -> Item {
        let slot = Slot::ALL[self.s.rng.index(3)];
        let rarity = self.roll_rarity(min_rare);
        self.make_item(slot, rarity, ilvl, 0)
    }

    /// Nhận món vào kho (tự rã nếu kho đầy hoặc theo cài đặt).
    pub(crate) fn gain_item(&mut self, item: Item) {
        if item.rarity == Rarity::Legendary {
            self.s.got_legendary = true;
        }
        if item.upgrade >= ITEM_MAX_UPGRADE {
            self.s.got_max_upgrade = true;
        }
        let auto = self.s.auto_salvage.is_some_and(|m| item.rarity <= m);
        if auto || self.s.inventory.len() >= INVENTORY_CAP {
            let y = rules::salvage_value(&item);
            self.credit_yarn(y);
            self.emit(Event::ItemSalvaged {
                item: item.id,
                yarn: y,
            });
        } else {
            self.s.inventory.push(item);
        }
    }

    pub(crate) fn update_drops(&mut self) {
        let mut k = 0;
        while k < self.s.drops.len() {
            self.s.drops[k].t += DT;
            if self.s.drops[k].t < COLLECT_TIME {
                k += 1;
                continue;
            }
            let d = self.s.drops.swap_remove(k);
            let x = self.s.anchor as f32;
            match d.kind {
                DropKind::Fish => {
                    self.credit_fish(d.amount);
                    self.emit(Event::Loot {
                        currency: Currency::Fish,
                        amount: d.amount,
                        x,
                    });
                }
                DropKind::Yarn => {
                    self.credit_yarn(d.amount);
                    self.emit(Event::Loot {
                        currency: Currency::Yarn,
                        amount: d.amount,
                        x,
                    });
                }
                DropKind::Gear(_) => {
                    if let Some(it) = d.item {
                        self.gain_item(it);
                    }
                }
            }
        }
    }

    // ── hộp carton ──

    pub(crate) fn spawn_box(&mut self) {
        let entity = self.s.new_entity();
        self.s.carton = Some(Carton {
            entity,
            y: BOX_FALL_FROM,
            time_left: BOX_LIFE,
            opening: None,
        });
        let x = (self.s.anchor + BOX_AHEAD) as f32;
        self.emit(Event::BoxSpawned { x });
        if self.s.tutorial[2] == 0 {
            self.s.tutorial[2] = 1;
            self.emit(Event::Tutorial {
                id: TutorialId::FirstBox,
            });
        }
    }

    pub(crate) fn update_box(&mut self) {
        let Some(b) = self.s.carton.as_mut() else {
            self.s.box_timer -= DT;
            if self.s.box_timer <= 0.0 {
                self.s.box_timer = self.s.rng.range(BOX_MIN, BOX_MAX);
                self.spawn_box();
            }
            return;
        };
        b.y = (b.y - BOX_FALL_SPEED * DT).max(0.0);
        if let Some(t) = b.opening {
            let t = t + DT / BOX_OPEN_TIME;
            if t >= 1.0 {
                self.s.carton = None;
            } else {
                b.opening = Some(t);
            }
            return;
        }
        b.time_left -= DT;
        if b.time_left <= 0.0 {
            self.s.carton = None;
            self.emit(Event::BoxExpired);
        }
    }

    /// Mở hộp: 50% cá (60 s thu nhập), 25% len, 15% trang bị rare+, 10% +1 catnip.
    pub(crate) fn open_box(&mut self) -> Result<(), CmdError> {
        match &self.s.carton {
            Some(b) if b.opening.is_none() => {}
            _ => return Err(CmdError::NothingToDo),
        }
        if let Some(b) = self.s.carton.as_mut() {
            b.opening = Some(0.0);
        }
        self.s.stats.boxes_opened += 1;
        let x = (self.s.anchor + BOX_AHEAD) as f32;
        self.emit(Event::BoxOpened { x });
        let r = self.s.rng.f64();
        let gs = self.s.global_stage();
        if r < 0.50 {
            let (rate, _) = self.income_rates();
            let fish = (rate * BOX_FISH_SECS)
                .max(rules::estimate_fish_rate(&self.s) * BOX_FISH_SECS * 0.5);
            self.credit_fish(fish);
            self.emit(Event::Loot {
                currency: Currency::Fish,
                amount: fish,
                x,
            });
        } else if r < 0.75 {
            let yarn = 10.0 + gs as f64;
            self.credit_yarn(yarn);
            self.emit(Event::Loot {
                currency: Currency::Yarn,
                amount: yarn,
                x,
            });
        } else if r < 0.90 {
            let item = self.roll_item(gs, true);
            self.emit(Event::GearDropped {
                item: item.id,
                rarity: item.rarity,
                x,
            });
            self.gain_item(item);
        } else {
            self.s.catnip_charges += 1;
            self.emit(Event::Loot {
                currency: Currency::Catnip,
                amount: 1.0,
                x,
            });
        }
        Ok(())
    }
}
