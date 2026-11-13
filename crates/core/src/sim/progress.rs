//! Thành tựu (suy ra từ trạng thái) và hướng dẫn lần đầu.

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::state::*;

/// Điều kiện của mỗi thành tựu, suy ra hoàn toàn từ trạng thái.
pub(crate) fn ach_met(s: &State, id: AchId) -> bool {
    use AchId::*;
    let max_level = s
        .cats
        .iter()
        .filter(|c| c.unlocked)
        .map(|c| c.level)
        .max()
        .unwrap_or(1);
    match id {
        Zone2 => s.best_gs >= STAGES,
        Zone3 => s.best_gs >= 2 * STAGES,
        Zone4 => s.best_gs >= 3 * STAGES,
        Zone5 => s.best_gs >= 4 * STAGES,
        Cycle1 => s.best_gs >= ZONES * STAGES,
        BossRatKing => s.bosses_beaten[0],
        BossCrowLord => s.bosses_beaten[1],
        BossEel => s.bosses_beaten[2],
        BossMastiff => s.bosses_beaten[3],
        BossVacuumTitan => s.bosses_beaten[4],
        UnlockXiem => s.cat(CatId::Xiem).unlocked,
        UnlockMap => s.cat(CatId::Map).unlocked,
        UnlockMun => s.cat(CatId::Mun).unlocked,
        Level25 => max_level >= 25,
        Level50 => max_level >= 50,
        Level100 => max_level >= 100,
        FirstLegendary => s.got_legendary,
        MaxUpgrade => s.got_max_upgrade,
        Lives2 => s.lives_used >= 2,
        Lives5 => s.lives_used >= 5,
        Lives9 => s.lives_used >= 9,
        Boxes10 => s.stats.boxes_opened >= 10,
        Boxes100 => s.stats.boxes_opened >= 100,
        Catnip50 => s.stats.catnip_used >= 50,
        Fish1M => s.stats.fish_earned >= 1e6,
        Fish1B => s.stats.fish_earned >= 1e9,
    }
}

impl Game {
    /// Kiểm tra thành tựu + hướng dẫn (mỗi giây và sau mỗi lệnh/cheat).
    pub(crate) fn check_progress(&mut self) {
        for (k, id) in AchId::ALL.iter().enumerate() {
            if self.s.achievements[k].is_none() && ach_met(&self.s, *id) {
                self.unlock_achievement(*id);
            }
        }
        // hướng dẫn 1: lần đầu có 10 cá
        if self.s.tutorial[0] == 0 && self.s.fish >= 10.0 {
            self.s.tutorial[0] = 1;
            self.emit(Event::Tutorial {
                id: TutorialId::OpenBag,
            });
        }
        // hướng dẫn 2: đủ điều kiện mở Xiêm
        let x = cat_def(CatId::Xiem);
        if self.s.tutorial[1] == 0
            && !self.s.cat(CatId::Xiem).unlocked
            && self.s.best_gs >= x.unlock_gs
            && self.s.fish >= x.unlock_cost
        {
            self.s.tutorial[1] = 1;
            self.emit(Event::Tutorial {
                id: TutorialId::UnlockXiem,
            });
        }
        if self.s.tutorial[1] == 0 && self.s.cat(CatId::Xiem).unlocked {
            // đã mở trước khi gợi ý kịp hiện → không cần gợi ý nữa
            self.s.tutorial[1] = 2;
        }
    }

    pub(crate) fn unlock_achievement(&mut self, id: AchId) {
        let k = id as usize;
        if self.s.achievements[k].is_none() {
            self.s.achievements[k] = Some(self.s.time);
            self.emit(Event::Achievement { id });
        }
    }
}
