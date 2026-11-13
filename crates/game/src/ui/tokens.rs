//! Token thiết kế (SPEC §7.1): lưới 4 px, 3 cỡ chữ, màu lấy từ bảng màu §8.2 (`tools/art/pixel.mjs`).
//! Mọi khoảng cách / màu / cỡ chữ của UI lấy từ đây.

use bevy::color::Color;
use cm_core::Rarity;

// ── lưới 4 px ──
pub const S1: f32 = 4.0;
pub const S2: f32 = 8.0;
pub const S3: f32 = 12.0;
pub const S4: f32 = 16.0;

// ── cỡ chữ (px logic, nhân `text_scale`) ──
pub const FONT_SMALL: f32 = 11.0;
pub const FONT_BODY: f32 = 13.0;
pub const FONT_TITLE: f32 = 16.0;

/// Hệ số phóng của art UI trong bảng (9-slice, icon): luôn nguyên.
pub const PANEL_ART: u32 = 2;
/// Bảng: rộng 360, cao 420 (tối đa theo SPEC §7.3).
pub const PANEL_W: f32 = 360.0;
pub const PANEL_H: f32 = 420.0;
/// Khoảng hở giữa mép trên dải và đáy bảng.
pub const PANEL_GAP: f32 = 4.0;
pub const TAB_H: f32 = 28.0;
pub const GOAL_H: f32 = 24.0;
pub const SLOT: f32 = 32.0;
pub const ROW_H: f32 = 28.0;
pub const BTN_H: f32 = 24.0;
pub const BTN_H_SMALL: f32 = 20.0;
pub const TOOLTIP_MAX_W: f32 = 220.0;

/// Cỡ chữ theo 3 bậc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fs {
    Small,
    Body,
    Title,
}
impl Fs {
    pub fn px(self) -> f32 {
        match self {
            Fs::Small => FONT_SMALL,
            Fs::Body => FONT_BODY,
            Fs::Title => FONT_TITLE,
        }
    }
}

/// Bảng màu (con của bảng 34 màu trong pixel.mjs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ink {
    /// chữ chính (w)
    Text,
    /// chữ phụ (u)
    Muted,
    /// vàng (z) — số tiền, huyền thoại
    Gold,
    /// xanh lá (d) — tăng, đủ tiền
    Good,
    /// đỏ (o) — giảm, thiếu tiền
    Bad,
    /// xanh dương sáng (D) — hiếm
    Rare,
    /// tím (L) — sử thi, hồn mèo
    Epic,
    /// cam (y) — nhấn
    Accent,
    /// viền / bóng chữ (k)
    Outline,
}

const fn hex(rgb: u32) -> Color {
    Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

impl Ink {
    /// Màu trên nền sáng (khung bảng kem, nút, ô): bản tối cùng sắc độ.
    pub fn on_light(self) -> Color {
        match self {
            Ink::Text => hex(0x1b1b2f),
            Ink::Muted => hex(0x4a4e63),
            Ink::Gold => hex(0xd9742b),
            Ink::Good => hex(0x2f6b3a),
            Ink::Bad => hex(0x8c2f39),
            Ink::Rare => hex(0x2a4a8a),
            Ink::Epic => hex(0x4b2a6b),
            Ink::Accent => hex(0x8a5a3c),
            Ink::Outline => hex(0x1b1b2f),
        }
    }
    pub fn color(self) -> Color {
        match self {
            Ink::Text => hex(0xf6f2e6),
            Ink::Muted => hex(0xa3a9b8),
            Ink::Gold => hex(0xffe27a),
            Ink::Good => hex(0x7cc45a),
            Ink::Bad => hex(0xf28b6c),
            Ink::Rare => hex(0x7fd0f0),
            Ink::Epic => hex(0xd08be8),
            Ink::Accent => hex(0xf7a93b),
            Ink::Outline => hex(0x1b1b2f),
        }
    }
}

pub fn rarity_ink(r: Rarity) -> Ink {
    match r {
        Rarity::Common => Ink::Muted,
        Rarity::Rare => Ink::Rare,
        Rarity::Epic => Ink::Epic,
        Rarity::Legendary => Ink::Gold,
    }
}

/// Ký hiệu độ hiếm (không chỉ dựa vào màu, SPEC §7.1).
pub fn rarity_glyph(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "◆",
        Rarity::Rare => "◆◆",
        Rarity::Epic => "◆◆◆",
        Rarity::Legendary => "★",
    }
}

/// Khung ô theo độ hiếm: thường không viền, hiếm viền đơn, sử thi viền kép, huyền thoại viền + lấp lánh.
pub fn rarity_slot(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "ui_slot",
        Rarity::Rare => "ui_slot_rare",
        Rarity::Epic => "ui_slot_epic",
        Rarity::Legendary => "ui_slot_legendary",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spacing_on_4px_grid() {
        for v in [
            S1,
            S2,
            S3,
            S4,
            PANEL_W,
            PANEL_H,
            PANEL_GAP,
            TAB_H,
            GOAL_H,
            SLOT,
            ROW_H,
            BTN_H,
            BTN_H_SMALL,
        ] {
            assert_eq!(v % 4.0, 0.0, "{v} không theo lưới 4 px");
        }
        const { assert!(PANEL_H <= 420.0) };
    }
}
