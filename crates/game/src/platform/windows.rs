//! Windows: work area (`SPI_GETWORKAREA` / `GetMonitorInfoW`), Ghost mode (§7.6: chỉ đọc *vị trí*
//! con trỏ mỗi 100 ms), tự khởi động (HKCU\Software\Microsoft\Windows\CurrentVersion\Run).

use crate::layout::WorkArea;
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::CurrentLayout;
use crate::ui::vdom::{HoldButton, UiAction};
use crate::world::{WorldCam, window_to_logic};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{CursorOptions, PrimaryWindow};
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Vùng làm việc (trừ taskbar) của màn hình chứa tâm `full`.
pub fn work_area(full: WorkArea) -> Option<WorkArea> {
    unsafe {
        let center = POINT {
            x: full.x + full.w as i32 / 2,
            y: full.y + full.h as i32 / 2,
        };
        let mon = MonitorFromPoint(center, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut info) == 0 {
            return None;
        }
        let r: RECT = info.rcWork;
        Some(WorkArea {
            x: r.left,
            y: r.top,
            w: (r.right - r.left) as u32,
            h: (r.bottom - r.top) as u32,
            dpi: full.dpi,
        })
    }
}

/// Locale người dùng (vd "vi-VN").
pub fn windows_locale() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..(n as usize - 1)])
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "ChinMang";

/// Bật/tắt tự khởi động cùng Windows (ghi đường dẫn exe hiện tại; khóa luôn được đóng).
pub fn set_autostart(on: bool) -> Result<(), String> {
    // chuẩn bị mọi thứ có thể lỗi TRƯỚC khi mở khóa registry
    let val = if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Some(wide(&format!("\"{}\"", exe.display())))
    } else {
        None
    };
    let path = wide(RUN_KEY);
    let name = wide(RUN_NAME);
    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let opened = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if opened != 0 {
            return Err(format!("không mở được khóa Run ({opened})"));
        }
        let res = match &val {
            Some(v) => RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                v.as_ptr() as *const u8,
                (v.len() * 2) as u32,
            ),
            None => {
                let r = RegDeleteValueW(key, name.as_ptr());
                if r == 2 { 0 } else { r } // ERROR_FILE_NOT_FOUND: đã tắt sẵn
            }
        };
        RegCloseKey(key);
        if res == 0 {
            Ok(())
        } else {
            Err(format!("registry lỗi {res}"))
        }
    }
}

#[derive(Resource)]
struct GhostTimer(Timer);

pub struct WindowsPlugin;

impl Plugin for WindowsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GhostTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .add_systems(Update, (ghost_mode, autostart_sync));
    }
}

/// Ghost mode: cả dải click-xuyên-qua; bật lại hit test khi con trỏ ở trên một nút (HUD, Thử lại boss,
/// Nhận ở thẻ vắng mặt, Đã hiểu ở bong bóng hướng dẫn…) hoặc trên hộp carton.
fn ghost_mode(
    time: Res<Time<Real>>,
    mut timer: ResMut<GhostTimer>,
    settings: Res<Settings>,
    layout: Option<Res<CurrentLayout>>,
    view: Res<crate::ui::View>,
    sim: Option<Res<Sim>>,
    cam: Res<WorldCam>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform), Or<(With<UiAction>, With<HoldButton>)>>,
    mut q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut cursor) = q.single_mut() else {
        return;
    };
    if !settings.ghost || view.panel.is_some() {
        if !cursor.hit_test {
            cursor.hit_test = true;
        }
        return;
    }
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let Some(layout) = layout else { return };
    let mut p = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut p) } == 0 {
        return;
    }
    let l = layout.0;
    let (wx, wy) = l.window_pos;
    let local = Vec2::new((p.x - wx) as f32, (p.y - wy) as f32);
    let rects: Vec<(Vec2, Vec2)> = buttons
        .iter()
        .map(|(n, t)| (t.translation, n.size()))
        .collect();
    let mut inside = super::point_in_any(local, &rects);
    if !inside
        && let (Some(sim), Some(lp)) = (sim, window_to_logic(local, l.strip, l.scale, cam.left))
        && let Some(b) = sim.game.carton()
    {
        inside = b.opening.is_none()
            && (lp.x - b.x).abs() <= 9.0
            && lp.y >= b.y - 1.0
            && lp.y <= b.y + 14.0;
    }
    if cursor.hit_test != inside {
        cursor.hit_test = inside;
    }
}

/// Đồng bộ registry: lúc khởi động (nếu đang bật thì ghi lại đường dẫn exe hiện tại) và mỗi khi đổi cài đặt.
fn autostart_sync(settings: Res<Settings>, mut last: Local<Option<bool>>) {
    if *last != Some(settings.autostart) {
        if (last.is_some() || settings.autostart)
            && let Err(e) = set_autostart(settings.autostart)
        {
            warn!("tự khởi động: {e}");
        }
        *last = Some(settings.autostart);
    }
}
