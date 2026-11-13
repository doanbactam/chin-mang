//! Tính bố cục cửa sổ / dải / bảng (thuần, test được). SPEC §5, §6.
//!
//! Quy ước: cửa sổ đặt `scale_factor_override = 1` ⇒ 1 px logic của Bevy = 1 px vật lý.
//! Tọa độ trong cửa sổ: gốc trên-trái, y hướng xuống.

use crate::settings::Edge;
use crate::ui::tokens::{PANEL_GAP, PANEL_H, PANEL_W};
use cm_core::STRIP_H;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

/// Vùng làm việc của màn hình (px vật lý, tọa độ desktop) — hoặc canvas trên web.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// hệ số DPI của màn hình (1.0 = 100%)
    pub dpi: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutIn {
    pub work: WorkArea,
    /// 0 = tự động
    pub scale_setting: u32,
    pub width_pct: u32,
    pub edge: Edge,
    pub panel_open: bool,
    /// true: cửa sổ = canvas cố định (web), không dời/đổi cỡ cửa sổ.
    pub fixed_window: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOut {
    /// hệ số phóng thế giới (nguyên 1..=4)
    pub scale: u32,
    /// UiScale (nguyên ≥ 1)
    pub ui_scale: f32,
    /// vị trí (desktop) + cỡ cửa sổ, px vật lý
    pub window_pos: (i32, i32),
    pub window_size: (u32, u32),
    /// dải trong cửa sổ
    pub strip: Rect,
    /// bảng trong cửa sổ (khi mở)
    pub panel: Option<Rect>,
    /// rộng texture thế giới (px logic)
    pub world_w: u32,
}

/// Hệ số tự động: 2× ở 100% DPI, làm tròn về số nguyên gần nhất (125% → 2.5 → 2, 150% → 3).
pub fn auto_scale(dpi: f32) -> u32 {
    let v = 2.0 * dpi.max(0.5);
    // 2.5 (125%) → 2, 3.0 (150%) → 3: làm tròn "nửa xuống" để dải không quá dày
    let r = if (v - v.floor() - 0.5).abs() < 1e-3 {
        v.floor()
    } else {
        v.round()
    };
    (r as u32).clamp(1, 4)
}

pub fn ui_scale_for(dpi: f32) -> f32 {
    dpi.round().max(1.0)
}

pub fn compute(i: &LayoutIn) -> LayoutOut {
    let scale = if i.scale_setting == 0 {
        auto_scale(i.work.dpi)
    } else {
        i.scale_setting.clamp(1, 4)
    };
    let ui = ui_scale_for(i.work.dpi);
    let strip_h = (STRIP_H * scale) as f32;
    let strip_w = ((i.work.w as f32) * i.width_pct.clamp(10, 100) as f32 / 100.0).floor();
    let panel_h = (PANEL_H + PANEL_GAP) * ui;
    let extra = if i.panel_open { panel_h } else { 0.0 };

    let (window_pos, window_size, strip) = if i.fixed_window {
        let x = ((i.work.w as f32 - strip_w) / 2.0).floor();
        let y = match i.edge {
            Edge::Bottom => i.work.h as f32 - strip_h,
            Edge::Top => 0.0,
        };
        (
            (i.work.x, i.work.y),
            (i.work.w, i.work.h),
            Rect {
                x,
                y,
                w: strip_w,
                h: strip_h,
            },
        )
    } else {
        let win_h = strip_h + extra;
        let x = i.work.x + ((i.work.w as f32 - strip_w) / 2.0).floor() as i32;
        let (y, strip_y) = match i.edge {
            Edge::Bottom => (i.work.y + i.work.h as i32 - win_h as i32, extra),
            Edge::Top => (i.work.y, 0.0),
        };
        (
            (x, y),
            (strip_w as u32, win_h as u32),
            Rect {
                x: 0.0,
                y: strip_y,
                w: strip_w,
                h: strip_h,
            },
        )
    };

    let panel = i.panel_open.then(|| {
        let w = PANEL_W * ui;
        let h = PANEL_H * ui;
        // neo theo cụm nút phải của HUD (nút túi)
        let x = (strip.right() - w - 8.0 * ui).max(strip.x).floor();
        let y = match i.edge {
            Edge::Bottom => strip.y - PANEL_GAP * ui - h,
            Edge::Top => strip.bottom() + PANEL_GAP * ui,
        };
        Rect { x, y, w, h }
    });

    LayoutOut {
        scale,
        ui_scale: ui,
        window_pos,
        window_size,
        strip,
        panel,
        world_w: (strip_w / scale as f32).ceil() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work() -> WorkArea {
        WorkArea {
            x: 0,
            y: 0,
            w: 1920,
            h: 1040,
            dpi: 1.0,
        }
    }

    #[test]
    fn auto_scale_rounds_dpi() {
        assert_eq!(auto_scale(1.0), 2);
        assert_eq!(auto_scale(1.25), 2);
        assert_eq!(auto_scale(1.5), 3);
        assert_eq!(auto_scale(1.75), 3);
        assert_eq!(auto_scale(2.0), 4);
        assert_eq!(auto_scale(3.0), 4);
        assert_eq!(auto_scale(0.5), 1);
    }

    #[test]
    fn strip_sits_on_work_area_bottom_and_grows_up() {
        let mut i = LayoutIn {
            work: work(),
            scale_setting: 0,
            width_pct: 100,
            edge: Edge::Bottom,
            panel_open: false,
            fixed_window: false,
        };
        let a = compute(&i);
        assert_eq!(a.scale, 2);
        assert_eq!(a.window_size, (1920, 96));
        assert_eq!(a.window_pos, (0, 1040 - 96));
        assert_eq!(a.world_w, 960);
        i.panel_open = true;
        let b = compute(&i);
        let grow = (PANEL_H + PANEL_GAP) as u32;
        assert_eq!(b.window_size.1, 96 + grow);
        // mép dưới đứng yên
        assert_eq!(b.window_pos.1 + b.window_size.1 as i32, 1040);
        assert_eq!(b.strip.y, grow as f32);
        let p = b.panel.unwrap();
        assert_eq!(p.bottom() + PANEL_GAP, b.strip.y);
        assert!(p.x >= 0.0 && p.right() <= 1920.0);
    }

    #[test]
    fn top_edge_and_width() {
        let i = LayoutIn {
            work: WorkArea {
                x: 1920,
                y: 30,
                w: 2560,
                h: 1410,
                dpi: 1.5,
            },
            scale_setting: 0,
            width_pct: 50,
            edge: Edge::Top,
            panel_open: true,
            fixed_window: false,
        };
        let a = compute(&i);
        assert_eq!(a.scale, 3);
        assert_eq!(a.ui_scale, 2.0);
        assert_eq!(a.window_pos, (1920 + 640, 30));
        assert_eq!(a.strip.y, 0.0);
        assert!(a.panel.unwrap().y > a.strip.bottom());
    }

    #[test]
    fn web_canvas_is_fixed() {
        let i = LayoutIn {
            work: WorkArea {
                x: 0,
                y: 0,
                w: 1280,
                h: 600,
                dpi: 1.0,
            },
            scale_setting: 3,
            width_pct: 100,
            edge: Edge::Bottom,
            panel_open: true,
            fixed_window: true,
        };
        let a = compute(&i);
        assert_eq!(a.window_size, (1280, 600));
        assert_eq!(
            a.strip,
            Rect {
                x: 0.0,
                y: 600.0 - 144.0,
                w: 1280.0,
                h: 144.0
            }
        );
        let p = a.panel.unwrap();
        assert!(p.y >= 0.0);
        assert_eq!(a.world_w, 427);
    }
}
