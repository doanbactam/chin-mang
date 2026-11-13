//! Đọc/ghi dữ liệu bền: save game + cài đặt.
//! - Windows: `%APPDATA%\ChinMang\`
//! - Linux/khác: `$XDG_DATA_HOME/chin-mang/` (mặc định `~/.local/share/chin-mang/`)
//! - wasm: `localStorage` khóa `chin-mang.<tên>`
//!
//! Ghi file nguyên tử: ghi `<tên>.tmp` + fsync rồi `rename` đè (thử lại 5×50 ms); save giữ thêm `save.bak`
//! (bản tốt gần nhất), save hỏng được cách ly thành `save.corrupt-<ts>.json`.

pub const SAVE_KEY: &str = "save";
pub const SETTINGS_KEY: &str = "settings";
pub const EXPORT_KEY: &str = "export";
/// Bản save tốt gần nhất trước bản hiện tại (xoay vòng mỗi lần lưu thành công).
pub const BAK_KEY: &str = "save.bak";
/// Bản save ngay trước khi nhập save khác.
pub const BEFORE_IMPORT_KEY: &str = "save.before-import";
/// Khóa chạy một bản (cạnh save).
#[cfg(not(target_arch = "wasm32"))]
pub const LOCK_KEY: &str = "chin-mang.lock";

#[cfg(not(target_arch = "wasm32"))]
/// Số lần thử lại thao tác file (rename bị chặn tạm thời trên Windows khi antivirus/indexer giữ file).
pub const RETRIES: u32 = 5;
#[cfg(not(target_arch = "wasm32"))]
pub const RETRY_MS: u64 = 50;

#[cfg(not(target_arch = "wasm32"))]
/// Chạy `f` tối đa `tries` lần, nghỉ `wait` giữa các lần; trả lỗi cuối cùng.
pub fn retry<T, E>(
    tries: u32,
    mut wait: impl FnMut(),
    mut f: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut last = f();
    for _ in 1..tries.max(1) {
        if last.is_ok() {
            break;
        }
        wait();
        last = f();
    }
    last
}

/// Giây unix hiện tại.
pub fn now_unix() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Locale hệ thống (để chọn ngôn ngữ mặc định).
pub fn system_locale() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w.navigator(), &"language".into()).ok())
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var)
                && !v.is_empty()
            {
                return v;
            }
        }
        #[cfg(windows)]
        {
            return crate::platform::windows_locale();
        }
        #[allow(unreachable_code)]
        String::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::path::{Path, PathBuf};

    /// Thư mục dữ liệu theo nền tảng từ biến môi trường (tách ra để test được).
    pub fn data_dir_from(
        appdata: Option<&str>,
        xdg: Option<&str>,
        home: Option<&str>,
        windows: bool,
    ) -> Option<PathBuf> {
        if windows {
            return appdata
                .filter(|s| !s.is_empty())
                .map(|a| Path::new(a).join("ChinMang"));
        }
        if let Some(x) = xdg.filter(|s| !s.is_empty()) {
            return Some(Path::new(x).join("chin-mang"));
        }
        home.filter(|s| !s.is_empty())
            .map(|h| Path::new(h).join(".local/share/chin-mang"))
    }

    pub fn data_dir() -> Option<PathBuf> {
        let appdata = std::env::var("APPDATA").ok();
        let xdg = std::env::var("XDG_DATA_HOME").ok();
        let home = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok());
        data_dir_from(
            appdata.as_deref(),
            xdg.as_deref(),
            home.as_deref(),
            cfg!(windows),
        )
    }

    pub fn file_name(key: &str) -> String {
        match key {
            super::EXPORT_KEY => "export.txt".into(),
            super::BAK_KEY | super::LOCK_KEY => key.into(),
            _ => format!("{key}.json"),
        }
    }

    pub fn path_for(key: &str) -> Option<PathBuf> {
        data_dir().map(|d| d.join(file_name(key)))
    }

    fn rename_retry(from: &Path, to: &Path) -> std::io::Result<()> {
        super::retry(
            super::RETRIES,
            || std::thread::sleep(std::time::Duration::from_millis(super::RETRY_MS)),
            || std::fs::rename(from, to),
        )
    }

    /// Ghi `tmp` + fsync, trả đường dẫn tmp.
    fn write_tmp(path: &Path, data: &str) -> std::io::Result<PathBuf> {
        use std::io::Write;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data.as_bytes())?;
        f.sync_all()?;
        Ok(tmp)
    }

    /// fsync thư mục để tên mới bền qua mất điện (Unix; Windows không mở được thư mục như file).
    fn sync_dir(path: &Path) {
        #[cfg(unix)]
        if let Some(dir) = path.parent()
            && let Ok(d) = std::fs::File::open(dir)
        {
            let _ = d.sync_all();
        }
        #[cfg(not(unix))]
        let _ = path;
    }

    /// Ghi nguyên tử: tmp + fsync + rename (thử lại) + fsync thư mục.
    pub fn write_atomic(path: &Path, data: &str) -> std::io::Result<()> {
        let tmp = write_tmp(path, data)?;
        rename_retry(&tmp, path)?;
        sync_dir(path);
        Ok(())
    }

    /// Ghi save mới, bản cũ (đã từng ghi thành công) thành `bak`.
    pub fn write_rotating(path: &Path, bak: &Path, data: &str) -> std::io::Result<()> {
        let tmp = write_tmp(path, data)?;
        if path.exists() {
            rename_retry(path, bak)?;
        }
        rename_retry(&tmp, path)?;
        sync_dir(path);
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
use native::data_dir_from;

/// Đọc dữ liệu theo khóa.
pub fn read(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()?.local_storage().ok()??;
        storage.get_item(&format!("chin-mang.{key}")).ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(native::path_for(key)?).ok()
    }
}

/// Ghi dữ liệu theo khóa. Trả về nơi đã ghi (đường dẫn file hoặc khóa localStorage).
pub fn write(key: &str, data: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let full = format!("chin-mang.{key}");
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or_else(|| "localStorage không khả dụng".to_string())?;
        storage
            .set_item(&full, data)
            .map_err(|_| "localStorage đầy".to_string())?;
        Ok(full)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path =
            native::path_for(key).ok_or_else(|| "không tìm thấy thư mục dữ liệu".to_string())?;
        native::write_atomic(&path, data).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    }
}

/// Lưu save game: bản đang có thành `save.bak`, bản mới thành `save`.
pub fn write_save(data: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(old) = read(SAVE_KEY) {
            let _ = write(BAK_KEY, &old);
        }
        write(SAVE_KEY, data)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (Some(path), Some(bak)) = (native::path_for(SAVE_KEY), native::path_for(BAK_KEY))
        else {
            return Err("không tìm thấy thư mục dữ liệu".into());
        };
        native::write_rotating(&path, &bak, data).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    }
}

/// Cách ly một save hỏng thành `save.corrupt-<ts>.json` để không bị ghi đè. Trả về nơi đã cất.
pub fn quarantine(key: &str, ts: u64) -> Option<String> {
    let dest = format!("{key}.corrupt-{ts}");
    #[cfg(target_arch = "wasm32")]
    {
        let data = read(key)?;
        write(&dest, &data).ok()?;
        remove(key);
        Some(dest)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let from = native::path_for(key)?;
        let to = native::path_for(&dest)?;
        std::fs::rename(&from, &to).ok()?;
        Some(to.display().to_string())
    }
}

/// Khóa chạy một bản: giữ file khóa (độc quyền) cạnh save suốt vòng đời tiến trình.
/// `Ok(None)` = không có thư mục dữ liệu (bỏ qua); `Err` = đã có bản khác đang chạy.
#[cfg(not(target_arch = "wasm32"))]
pub fn acquire_single_instance() -> Result<Option<std::fs::File>, String> {
    use std::io::Write;
    let Some(path) = native::path_for(LOCK_KEY) else {
        return Ok(None);
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    match f.try_lock() {
        Ok(()) => {
            let _ = f.set_len(0);
            let _ = write!(f, "{}", std::process::id());
            Ok(Some(f))
        }
        Err(std::fs::TryLockError::WouldBlock) => Err(format!(
            "đã có bản Chín Mạng khác đang chạy (khóa {})",
            path.display()
        )),
        Err(std::fs::TryLockError::Error(e)) => {
            // hệ thống file không hỗ trợ khóa: chạy tiếp, không chặn người chơi
            eprintln!("không khóa được {}: {e}", path.display());
            Ok(Some(f))
        }
    }
}

pub fn remove(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = s.remove_item(&format!("chin-mang.{key}"));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(p) = native::path_for(key) {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Save → chuỗi base64 để xuất.
pub fn encode_export(json: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(json.as_bytes())
}

/// Chuỗi base64 (hoặc JSON thô) → JSON save.
pub fn decode_import(text: &str) -> Option<String> {
    use base64::Engine;
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(compact)
        .ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn data_dirs_per_platform() {
        assert_eq!(
            data_dir_from(Some(r"C:\Users\a\AppData\Roaming"), None, None, true),
            Some(PathBuf::from(r"C:\Users\a\AppData\Roaming").join("ChinMang"))
        );
        assert_eq!(
            data_dir_from(None, Some("/x/data"), Some("/home/a"), false),
            Some(PathBuf::from("/x/data/chin-mang"))
        );
        assert_eq!(
            data_dir_from(None, Some(""), Some("/home/a"), false),
            Some(PathBuf::from("/home/a/.local/share/chin-mang"))
        );
        assert_eq!(data_dir_from(None, None, None, true), None);
    }

    #[test]
    fn atomic_write_replaces_file() {
        let dir = std::env::temp_dir().join(format!("chin-mang-test-{}", std::process::id()));
        let p = dir.join("save.json");
        native::write_atomic(&p, "{\"a\":1}").unwrap();
        native::write_atomic(&p, "{\"a\":2}").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{\"a\":2}");
        assert!(!p.with_extension("tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rotating_write_keeps_previous_as_bak() {
        let dir = std::env::temp_dir().join(format!("chin-mang-rot-{}", std::process::id()));
        let p = dir.join("save.json");
        let b = dir.join("save.bak");
        native::write_rotating(&p, &b, "A").unwrap();
        assert!(!b.exists());
        native::write_rotating(&p, &b, "B").unwrap();
        native::write_rotating(&p, &b, "C").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "C");
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "B");
        assert!(!p.with_extension("tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn retry_gives_up_after_n_tries() {
        let mut n = 0;
        let mut waits = 0;
        let r: Result<(), ()> = retry(
            5,
            || waits += 1,
            || {
                n += 1;
                Err(())
            },
        );
        assert!(r.is_err());
        assert_eq!((n, waits), (5, 4));
        let mut m = 0;
        let r: Result<u32, ()> = retry(
            5,
            || {},
            || {
                m += 1;
                if m < 3 { Err(()) } else { Ok(m) }
            },
        );
        assert_eq!(r, Ok(3));
    }

    #[test]
    fn file_names() {
        assert_eq!(native::file_name(SAVE_KEY), "save.json");
        assert_eq!(native::file_name(BAK_KEY), "save.bak");
        assert_eq!(native::file_name("save.corrupt-12"), "save.corrupt-12.json");
        assert_eq!(
            native::file_name(BEFORE_IMPORT_KEY),
            "save.before-import.json"
        );
        assert_eq!(native::file_name(EXPORT_KEY), "export.txt");
    }

    #[test]
    fn export_roundtrip() {
        let json = "{\"version\":1,\"x\":\"Mướp\"}";
        let enc = encode_export(json);
        assert_eq!(decode_import(&enc).as_deref(), Some(json));
        assert_eq!(decode_import(&format!("  {enc}\n")).as_deref(), Some(json));
        assert_eq!(decode_import(json).as_deref(), Some(json));
        assert_eq!(decode_import("%%%"), None);
    }
}
