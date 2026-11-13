//! Đọc/ghi dữ liệu bền: save game + cài đặt.
//! - Windows: `%APPDATA%\ChinMang\`
//! - Linux/khác: `$XDG_DATA_HOME/chin-mang/` (mặc định `~/.local/share/chin-mang/`)
//! - wasm: `localStorage` khóa `chin-mang.<tên>`
//!
//! Ghi file nguyên tử: ghi `<tên>.tmp` rồi `rename` đè.

pub const SAVE_KEY: &str = "save";
pub const SETTINGS_KEY: &str = "settings";
pub const EXPORT_KEY: &str = "export";

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

    pub fn path_for(key: &str) -> Option<PathBuf> {
        let ext = if key == super::EXPORT_KEY {
            "txt"
        } else {
            "json"
        };
        data_dir().map(|d| d.join(format!("{key}.{ext}")))
    }

    /// Ghi nguyên tử: tmp + rename.
    pub fn write_atomic(path: &Path, data: &str) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, path)
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
    fn export_roundtrip() {
        let json = "{\"version\":1,\"x\":\"Mướp\"}";
        let enc = encode_export(json);
        assert_eq!(decode_import(&enc).as_deref(), Some(json));
        assert_eq!(decode_import(&format!("  {enc}\n")).as_deref(), Some(json));
        assert_eq!(decode_import(json).as_deref(), Some(json));
        assert_eq!(decode_import("%%%"), None);
    }
}
