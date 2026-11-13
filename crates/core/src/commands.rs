//! Lệnh của người chơi và cheat.

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::*;

/// Vị trí của một món: trong kho (index) hoặc đang mặc (mèo, ô).
enum Loc {
    Inv(usize),
    Worn(CatId, usize),
}

impl Game {
    pub(crate) fn exec(&mut self, cmd: Command) -> Result<(), CmdError> {
        match cmd {
            Command::LevelUp { cat, mode } => self.cmd_level_up(cat, mode),
            Command::Unlock { cat } => self.cmd_unlock(cat),
            Command::TalentUp { cat, slot } => self.cmd_talent_up(cat, slot),
            Command::Equip { cat, item } => self.cmd_equip(cat, item),
            Command::Unequip { cat, slot } => self.cmd_unequip(cat, slot),
            Command::EquipBest { cat } => self.cmd_equip_best(cat),
            Command::UpgradeItem { item } => self.cmd_upgrade_item(item),
            Command::Salvage { item } => self.cmd_salvage(item),
            Command::SetAutoSalvage { max } => self.cmd_set_auto_salvage(max),
            Command::Catnip => self.cmd_catnip(),
            Command::OpenBox => self.open_box(),
            Command::BuySoul { upgrade } => self.cmd_buy_soul(upgrade),
            Command::Prestige => self.cmd_prestige(),
            Command::RetryBoss => self.cmd_retry_boss(),
            Command::DismissTutorial { id } => self.cmd_dismiss_tutorial(id),
        }
    }

    fn cmd_level_up(&mut self, cat: CatId, mode: BuyMode) -> Result<(), CmdError> {
        if !self.s.cat(cat).unlocked {
            return Err(CmdError::Locked);
        }
        let q = rules::level_quote(&self.s, cat, mode);
        if !q.affordable || q.levels == 0 {
            return Err(CmdError::NotEnough(Currency::Fish));
        }
        self.s.fish = (self.s.fish - q.cost).max(0.0);
        let cs = self.s.cat_mut(cat);
        cs.level += q.levels;
        let level = cs.level;
        rules::recalc(&mut self.s);
        self.emit(Event::LevelUp { cat, level });
        Ok(())
    }

    fn cmd_unlock(&mut self, cat: CatId) -> Result<(), CmdError> {
        if self.s.cat(cat).unlocked {
            return Err(CmdError::AlreadyUnlocked);
        }
        let def = cat_def(cat);
        if self.s.best_gs < def.unlock_gs {
            return Err(CmdError::Locked);
        }
        if self.s.fish < def.unlock_cost {
            return Err(CmdError::NotEnough(Currency::Fish));
        }
        self.s.fish -= def.unlock_cost;
        self.unlock_cat(cat);
        Ok(())
    }

    pub(crate) fn unlock_cat(&mut self, cat: CatId) {
        let cs = self.s.cat_mut(cat);
        cs.unlocked = true;
        cs.alive = true;
        cs.hp = 1.0;
        cs.d.hp_max = 1.0;
        rules::recalc(&mut self.s);
        let slot = rules::formation_slot(&self.s, cat);
        let x = self.s.anchor - slot as f64 * CAT_SPACING;
        let cs = self.s.cat_mut(cat);
        cs.x = x;
        cs.hp = cs.d.hp_max;
        cs.anim = Anim::Idle;
        cs.anim_t = 0.0;
        if self.s.tutorial[1] == 1 && cat == CatId::Xiem {
            self.s.tutorial[1] = 2;
        }
        self.emit(Event::CatUnlocked { cat });
    }

    fn cmd_talent_up(&mut self, cat: CatId, slot: u8) -> Result<(), CmdError> {
        if slot > 2 {
            return Err(CmdError::NotAvailable);
        }
        let cs = self.s.cat(cat);
        if !cs.unlocked {
            return Err(CmdError::Locked);
        }
        if cs.talents[slot as usize] >= TALENT_MAX_RANK {
            return Err(CmdError::MaxRank);
        }
        if rules::talent_points_free(cs) == 0 {
            return Err(CmdError::NoTalentPoints);
        }
        self.s.cat_mut(cat).talents[slot as usize] += 1;
        rules::recalc(&mut self.s);
        Ok(())
    }

    /// Tìm món trong kho hoặc đang mặc.
    fn locate_item(&self, id: ItemId) -> Option<Loc> {
        if let Some(i) = self.s.inventory.iter().position(|it| it.id == id) {
            return Some(Loc::Inv(i));
        }
        for c in CatId::ALL {
            for (k, it) in self.s.cat(c).equipped.iter().enumerate() {
                if it.as_ref().is_some_and(|it| it.id == id) {
                    return Some(Loc::Worn(c, k));
                }
            }
        }
        None
    }

    fn cmd_equip(&mut self, cat: CatId, id: ItemId) -> Result<(), CmdError> {
        if !self.s.cat(cat).unlocked {
            return Err(CmdError::Locked);
        }
        let item = match self.locate_item(id) {
            None => return Err(CmdError::NoSuchItem),
            Some(Loc::Inv(i)) => self.s.inventory.remove(i),
            Some(Loc::Worn(c, k)) => {
                if c == cat {
                    return Err(CmdError::NothingToDo);
                }
                self.s.cat_mut(c).equipped[k]
                    .take()
                    .ok_or(CmdError::NoSuchItem)?
            }
        };
        let slot = item.slot.index();
        if let Some(old) = self.s.cat_mut(cat).equipped[slot].replace(item) {
            self.s.inventory.push(old);
        }
        rules::recalc(&mut self.s);
        Ok(())
    }

    fn cmd_unequip(&mut self, cat: CatId, slot: Slot) -> Result<(), CmdError> {
        if !self.s.cat(cat).unlocked {
            return Err(CmdError::Locked);
        }
        if self.s.cat(cat).equipped[slot.index()].is_none() {
            return Err(CmdError::NothingToDo);
        }
        if self.s.inventory.len() >= INVENTORY_CAP {
            return Err(CmdError::NotAvailable);
        }
        if let Some(it) = self.s.cat_mut(cat).equipped[slot.index()].take() {
            self.s.inventory.push(it);
        }
        rules::recalc(&mut self.s);
        Ok(())
    }

    fn cmd_equip_best(&mut self, cat: Option<CatId>) -> Result<(), CmdError> {
        let cats: Vec<CatId> = match cat {
            Some(c) => {
                if !self.s.cat(c).unlocked {
                    return Err(CmdError::Locked);
                }
                vec![c]
            }
            None => {
                // mèo gây sát thương chính nhận đồ trước
                let mut v: Vec<CatId> = CatId::ALL
                    .into_iter()
                    .filter(|&c| self.s.cat(c).unlocked)
                    .collect();
                v.sort_by_key(|&c| cat_def(c).order);
                v
            }
        };
        let mut changed = false;
        for c in cats {
            for slot in Slot::ALL {
                let cur = self.s.cat(c).equipped[slot.index()]
                    .as_ref()
                    .map(rules::power_score)
                    .unwrap_or(-1.0);
                let mut best: Option<(usize, f64)> = None;
                for (i, it) in self.s.inventory.iter().enumerate() {
                    if it.slot != slot {
                        continue;
                    }
                    let p = rules::power_score(it);
                    if p > cur && best.is_none_or(|(_, b)| p > b) {
                        best = Some((i, p));
                    }
                }
                if let Some((i, _)) = best {
                    let item = self.s.inventory.remove(i);
                    if let Some(old) = self.s.cat_mut(c).equipped[slot.index()].replace(item) {
                        self.s.inventory.push(old);
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            return Err(CmdError::NothingToDo);
        }
        rules::recalc(&mut self.s);
        Ok(())
    }

    fn item_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        match self.locate_item(id)? {
            Loc::Inv(i) => self.s.inventory.get_mut(i),
            Loc::Worn(c, k) => self.s.cat_mut(c).equipped[k].as_mut(),
        }
    }

    fn cmd_upgrade_item(&mut self, id: ItemId) -> Result<(), CmdError> {
        let yarn = self.s.yarn;
        let it = self.item_mut(id).ok_or(CmdError::NoSuchItem)?;
        let cost = rules::upgrade_cost(it).ok_or(CmdError::MaxRank)?;
        if yarn < cost {
            return Err(CmdError::NotEnough(Currency::Yarn));
        }
        it.upgrade += 1;
        let up = it.upgrade;
        self.s.yarn -= cost;
        if up >= ITEM_MAX_UPGRADE {
            self.s.got_max_upgrade = true;
        }
        rules::recalc(&mut self.s);
        self.emit(Event::ItemUpgraded {
            item: id,
            upgrade: up,
        });
        Ok(())
    }

    fn cmd_salvage(&mut self, id: ItemId) -> Result<(), CmdError> {
        match self.locate_item(id) {
            None => Err(CmdError::NoSuchItem),
            Some(Loc::Worn(..)) => Err(CmdError::NotAvailable),
            Some(Loc::Inv(i)) => {
                let it = self.s.inventory.remove(i);
                let y = rules::salvage_value(&it);
                self.credit_yarn(y);
                self.emit(Event::ItemSalvaged { item: id, yarn: y });
                Ok(())
            }
        }
    }

    fn cmd_set_auto_salvage(&mut self, max: Option<Rarity>) -> Result<(), CmdError> {
        if self.s.auto_salvage == max {
            return Err(CmdError::NothingToDo);
        }
        self.s.auto_salvage = max;
        if let Some(m) = max {
            let ids: Vec<ItemId> = self
                .s
                .inventory
                .iter()
                .filter(|it| it.rarity <= m)
                .map(|it| it.id)
                .collect();
            for id in ids {
                let _ = self.cmd_salvage(id);
            }
        }
        Ok(())
    }

    fn cmd_catnip(&mut self) -> Result<(), CmdError> {
        if self.s.catnip_cd > 0.0 {
            if self.s.catnip_charges == 0 {
                return Err(CmdError::OnCooldown);
            }
            self.s.catnip_charges -= 1;
        }
        self.activate_catnip();
        Ok(())
    }

    fn cmd_buy_soul(&mut self, u: SoulUpgrade) -> Result<(), CmdError> {
        let rank = self.s.soul_rank(u);
        let cost = rules::soul_cost(u, rank).ok_or(CmdError::MaxRank)?;
        if self.s.soul < cost {
            return Err(CmdError::NotEnough(Currency::Soul));
        }
        self.s.soul -= cost;
        self.s.soul_ranks[soul_index(u)] += 1;
        rules::recalc(&mut self.s);
        self.emit(Event::SoulBought {
            upgrade: u,
            rank: rank + 1,
        });
        Ok(())
    }

    fn cmd_prestige(&mut self) -> Result<(), CmdError> {
        if self.s.max_gs < PRESTIGE_MIN_GS {
            return Err(CmdError::NotAvailable);
        }
        let souls = rules::prestige_gain(self.s.max_gs);
        self.s.soul += souls;
        self.s.souls_earned += souls;
        self.s.lives_used += 1;
        // reset
        self.s.fish = 0.0;
        self.s.yarn = (self.s.yarn * PRESTIGE_YARN_KEEP).floor();
        self.s.inventory.clear();
        for cs in self.s.cats.iter_mut() {
            cs.level = 1;
            cs.talents = [0; 3];
            for e in cs.equipped.iter_mut() {
                if e.as_ref().is_some_and(|it| it.rarity != Rarity::Legendary) {
                    *e = None;
                }
            }
            cs.alive = true;
            cs.stun = 0.0;
            cs.poison_t = 0.0;
            cs.combo_left = 0;
            cs.atk_cd = 0.0;
            cs.skill_cd = 0.0;
            cs.anim = Anim::Idle;
            cs.anim_t = 0.0;
            cs.anim_lock = 0.0;
            cs.d.hp_max = 0.0;
        }
        rules::recalc(&mut self.s);
        for cs in self.s.cats.iter_mut() {
            cs.hp = cs.d.hp_max;
        }
        self.s.drops.clear();
        self.s.catnip_active = 0.0;
        let start = SoulUpgrade::HeadStart;
        let start_gs = soul_def(start).per_rank as u32 * self.s.soul_rank(start);
        let (cycle, zone, stage) = split_gs(start_gs);
        self.s.max_gs = start_gs;
        self.s.income_fish.iter_mut().for_each(|v| *v = 0.0);
        self.s.income_yarn.iter_mut().for_each(|v| *v = 0.0);
        self.s.income_count = 0;
        self.s.income_cur_fish = 0.0;
        self.s.income_cur_yarn = 0.0;
        // luôn phát ZoneEntered cho mạng mới
        self.s.zone = 0;
        self.goto_stage(cycle, zone, stage);
        let lives_used = self.s.lives_used;
        self.emit(Event::Prestiged { souls, lives_used });
        Ok(())
    }

    fn cmd_retry_boss(&mut self) -> Result<(), CmdError> {
        if !self.s.farming {
            return Err(CmdError::NotAvailable);
        }
        self.s.farming = false;
        self.s.farm_t = 0.0;
        self.s.stage = STAGES;
        if !matches!(self.s.phase, Phase::Failed { .. }) {
            self.s.enemies.retain(|e| e.dead);
            self.s.projs.clear();
            self.start_stage();
        }
        Ok(())
    }

    fn cmd_dismiss_tutorial(&mut self, id: TutorialId) -> Result<(), CmdError> {
        let k = id as usize;
        if self.s.tutorial[k] != 1 {
            return Err(CmdError::NothingToDo);
        }
        self.s.tutorial[k] = 2;
        Ok(())
    }

    // ── cheat ──

    pub(crate) fn exec_cheat(&mut self, cheat: Cheat) {
        match cheat {
            Cheat::Goto { cycle, zone, stage } => {
                self.goto_stage(cycle, zone, stage);
            }
            Cheat::Give { currency, amount } => match currency {
                Currency::Fish => self.s.fish += amount,
                Currency::Yarn => self.s.yarn += amount,
                Currency::Soul => self.s.soul += amount,
                Currency::Catnip => self.s.catnip_charges += amount.max(0.0) as u32,
            },
            Cheat::UnlockAll => {
                for c in CatId::ALL {
                    if !self.s.cat(c).unlocked {
                        self.unlock_cat(c);
                    }
                }
            }
            Cheat::SetLevel { cat, level } => {
                self.s.cat_mut(cat).level = level.max(1);
                rules::recalc(&mut self.s);
            }
            Cheat::GiveItem {
                slot,
                rarity,
                ilvl,
                upgrade,
            } => {
                let it = self.make_item(slot, rarity, ilvl, upgrade);
                if it.rarity == Rarity::Legendary {
                    self.s.got_legendary = true;
                }
                if it.upgrade >= ITEM_MAX_UPGRADE {
                    self.s.got_max_upgrade = true;
                }
                if self.s.inventory.len() < INVENTORY_CAP {
                    self.s.inventory.push(it);
                }
            }
            Cheat::Advance { ticks } => {
                for _ in 0..ticks {
                    self.step();
                }
            }
            Cheat::SpawnBox => self.spawn_box(),
            Cheat::UnlockAchievement { id } => self.unlock_achievement(id),
        }
    }
}
