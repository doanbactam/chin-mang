//! Cài đặt người chơi (SPEC §7.5). Lưu riêng với save game (`settings.json` / localStorage).

use crate::i18n::Lang;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Edge {
    #[default]
    Bottom,
    Top,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    /// 0 = tự động, 1..=4 = hệ số cố định.
    pub scale: u32,
    /// 50 / 75 / 100 (% chiều rộng màn hình).
    pub width_pct: u32,
    pub edge: Edge,
    /// Chỉ số màn hình trong danh sách (0 = màn chính).
    pub monitor: usize,
    /// 40..=100 (%).
    pub opacity: u32,
    pub ghost: bool,
    pub always_on_top: bool,
    /// 0 = thường (×1.0), 1 = lớn (×1.25), 2 = rất lớn (×1.5).
    pub text_size: u32,
    /// 0..=100
    pub vol_master: u32,
    pub vol_music: u32,
    pub vol_sfx: u32,
    /// 10 / 20 / 30
    pub fps_limit: u32,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: Lang::Vi,
            scale: 0,
            width_pct: 100,
            edge: Edge::Bottom,
            monitor: 0,
            opacity: 100,
            ghost: false,
            always_on_top: true,
            text_size: 0,
            vol_master: 30,
            vol_music: 0,
            vol_sfx: 40,
            fps_limit: 30,
            autostart: false,
        }
    }
}

pub const SCALE_OPTIONS: [u32; 5] = [0, 1, 2, 3, 4];
pub const WIDTH_OPTIONS: [u32; 3] = [50, 75, 100];
pub const FPS_OPTIONS: [u32; 3] = [10, 20, 30];
pub const OPACITY_STEP: u32 = 10;
pub const VOLUME_STEP: u32 = 10;

impl Settings {
    /// Cài đặt mặc định với ngôn ngữ theo locale hệ thống.
    pub fn with_locale(locale: &str) -> Settings {
        Settings {
            lang: Lang::from_locale(locale),
            ..Default::default()
        }
    }

    /// Hệ số chữ: 1.0 / 1.25 / 1.5.
    pub fn text_scale(&self) -> f32 {
        match self.text_size {
            1 => 1.25,
            2 => 1.5,
            _ => 1.0,
        }
    }

    /// Đưa mọi giá trị về miền hợp lệ (save cũ / sửa tay).
    pub fn sanitize(&mut self) {
        if self.scale > 4 {
            self.scale = 0;
        }
        if !WIDTH_OPTIONS.contains(&self.width_pct) {
            self.width_pct = 100;
        }
        self.opacity = (self.opacity / OPACITY_STEP * OPACITY_STEP).clamp(40, 100);
        self.text_size = self.text_size.min(2);
        self.vol_master = self.vol_master.min(100);
        self.vol_music = self.vol_music.min(100);
        self.vol_sfx = self.vol_sfx.min(100);
        if !FPS_OPTIONS.contains(&self.fps_limit) {
            self.fps_limit = 30;
        }
    }

    pub fn from_json(s: &str) -> Option<Settings> {
        let mut v: Settings = serde_json::from_str(s).ok()?;
        v.sanitize();
        Some(v)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Âm lượng tuyến tính 0..1 cho hiệu ứng / nhạc.
    pub fn sfx_gain(&self) -> f32 {
        (self.vol_master as f32 / 100.0) * (self.vol_sfx as f32 / 100.0)
    }
    pub fn music_gain(&self) -> f32 {
        (self.vol_master as f32 / 100.0) * (self.vol_music as f32 / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!((s.vol_master, s.vol_music, s.vol_sfx), (30, 0, 40));
        assert_eq!(s.music_gain(), 0.0);
        assert!(s.always_on_top);
        assert_eq!(s.fps_limit, 30);
    }
    #[test]
    fn json_roundtrip_and_missing_fields() {
        let s = Settings {
            scale: 3,
            opacity: 70,
            ..Default::default()
        };
        assert_eq!(Settings::from_json(&s.to_json()), Some(s));
        let partial =
            Settings::from_json(r#"{"scale": 9, "fps_limit": 17, "opacity": 5}"#).unwrap();
        assert_eq!(partial.scale, 0);
        assert_eq!(partial.fps_limit, 30);
        assert_eq!(partial.opacity, 40);
        assert_eq!(partial.vol_sfx, 40);
    }
}
