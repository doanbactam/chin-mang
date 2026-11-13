//! Lưu / tải JSON có `version` + `saved_at`, migrate, hash trạng thái, vắng mặt.

use serde::{Deserialize, Serialize};

use crate::api::*;
use crate::data::*;
use crate::game::Game;
use crate::rules;
use crate::state::State;

#[derive(Serialize)]
struct SaveOut<'a> {
    version: u32,
    saved_at: u64,
    state: &'a State,
}

#[derive(Deserialize)]
struct SaveIn {
    version: u32,
    #[serde(default)]
    saved_at: u64,
    state: State,
}

pub(crate) fn save(s: &State, now_unix: u64) -> String {
    let out = SaveOut {
        version: SAVE_VERSION,
        saved_at: now_unix,
        state: s,
    };
    serde_json::to_string(&out).unwrap_or_default()
}

/// Nâng bản lưu cũ lên phiên bản hiện tại (v1 là bản đầu tiên — chưa có bước nào).
fn migrate(v: serde_json::Value, from: u32) -> Result<serde_json::Value, LoadError> {
    match from {
        SAVE_VERSION => Ok(v),
        other => Err(LoadError::Version(other)),
    }
}

pub(crate) fn load(json: &str) -> Result<State, LoadError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| LoadError::Parse(e.to_string()))?;
    let version = v
        .get("version")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| LoadError::Parse("missing version".into()))? as u32;
    if version == 0 || version > SAVE_VERSION {
        return Err(LoadError::Version(version));
    }
    let v = migrate(v, version)?;
    let sf: SaveIn = serde_json::from_value(v).map_err(|e| LoadError::Parse(e.to_string()))?;
    let _ = sf.version;
    let mut s = sf.state;
    s.saved_at = sf.saved_at;
    // công thức có thể đã đổi giữa các bản: tính lại chỉ số nếu lệch
    let stale = CatId::ALL
        .iter()
        .any(|&c| rules::derive(&s, c) != s.cat(c).d);
    if stale {
        rules::recalc(&mut s);
    }
    Ok(s)
}

/// FNV-1a 64 trên JSON của trạng thái (bỏ `saved_at`).
pub(crate) fn state_hash(s: &State) -> u64 {
    let mut c = s.clone();
    c.saved_at = 0;
    let json = serde_json::to_string(&c).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

impl Game {
    pub(crate) fn offline(&mut self, elapsed_secs: u64) -> Option<OfflineReport> {
        if elapsed_secs < OFFLINE_MIN {
            return None;
        }
        let cap = rules::offline_cap(&self.s);
        let secs = elapsed_secs.min(cap);
        let (fr, yr) = self.income_rates();
        let fish = fr * secs as f64 * OFFLINE_MULT;
        let yarn = yr * secs as f64 * OFFLINE_MULT;
        self.s.fish += fish;
        self.s.stats.fish_earned += fish;
        self.s.yarn += yarn;
        let wake = (secs as f64 / 60.0).min(WAKE_BUFF_MAX);
        self.s.wake_buff = self.s.wake_buff.max(wake);
        if fish > 0.0 {
            let x = self.s.anchor as f32;
            self.emit(Event::Loot {
                currency: Currency::Fish,
                amount: fish,
                x,
            });
        }
        if yarn > 0.0 {
            let x = self.s.anchor as f32;
            self.emit(Event::Loot {
                currency: Currency::Yarn,
                amount: yarn,
                x,
            });
        }
        self.check_progress();
        Some(OfflineReport {
            secs,
            fish,
            yarn,
            wake_buff_secs: wake as f32,
            capped: elapsed_secs > cap,
        })
    }
}
