//! i18n (SPEC §7.8): mọi chuỗi hiển thị nằm trong `vi.rs` / `en.rs`, key giống nhau.
//! Mẫu có tham số dùng `{0}`, `{1}`… (thay bằng [`tf`]).

mod en;
mod vi;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Vi,
    En,
}

impl Lang {
    /// Đoán ngôn ngữ từ chuỗi locale hệ thống (`vi_VN.UTF-8`, `vi-VN`, `en-US`…).
    pub fn from_locale(locale: &str) -> Lang {
        if locale.to_ascii_lowercase().starts_with("vi") {
            Lang::Vi
        } else {
            Lang::En
        }
    }
    fn table(self) -> &'static HashMap<&'static str, &'static str> {
        static VI: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
        static EN: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
        match self {
            Lang::Vi => VI.get_or_init(|| vi::STRINGS.iter().copied().collect()),
            Lang::En => EN.get_or_init(|| en::STRINGS.iter().copied().collect()),
        }
    }
}

/// Chuỗi theo key. Key thiếu ⇒ trả lại chính key (và test bắt lỗi trước khi phát hành).
pub fn t(lang: Lang, key: &str) -> &'static str {
    match lang.table().get(key) {
        Some(s) => s,
        None => {
            // Trả về key dưới dạng chuỗi tĩnh tìm trong bảng kia nếu có, để không cấp phát.
            Lang::Vi.table().get(key).copied().unwrap_or("?")
        }
    }
}

/// Chuỗi có tham số `{0}`, `{1}`…
pub fn tf(lang: Lang, key: &str, args: &[&str]) -> String {
    fill(t(lang, key), args)
}

pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(j) = after.find('}')
            && let Ok(n) = after[..j].parse::<usize>()
        {
            out.push_str(args.get(n).copied().unwrap_or(""));
            rest = &after[j + 1..];
            continue;
        }
        out.push('{');
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Thời lượng dạng "3 giờ 12 phút" / "45 phút" / "30 s".
pub fn duration(lang: Lang, secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        tf(lang, "dur_hm", &[&h.to_string(), &m.to_string()])
    } else if m > 0 {
        tf(lang, "dur_m", &[&m.to_string()])
    } else {
        tf(lang, "dur_s", &[&secs.to_string()])
    }
}

/// Tất cả chuỗi của một ngôn ngữ (cho test font).
#[cfg(test)]
pub fn all_strings(lang: Lang) -> Vec<&'static str> {
    match lang {
        Lang::Vi => vi::STRINGS.iter().map(|(_, v)| *v).collect(),
        Lang::En => en::STRINGS.iter().map(|(_, v)| *v).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn template_params(s: &str) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            let after = &rest[i + 1..];
            if let Some(j) = after.find('}')
                && let Ok(n) = after[..j].parse::<usize>()
            {
                out.insert(n);
            }
            rest = after;
        }
        out
    }

    #[test]
    fn keys_match_between_languages() {
        let vi: BTreeSet<_> = vi::STRINGS.iter().map(|(k, _)| *k).collect();
        let en: BTreeSet<_> = en::STRINGS.iter().map(|(k, _)| *k).collect();
        let only_vi: Vec<_> = vi.difference(&en).collect();
        let only_en: Vec<_> = en.difference(&vi).collect();
        assert!(
            only_vi.is_empty() && only_en.is_empty(),
            "chỉ có ở vi: {only_vi:?}; chỉ có ở en: {only_en:?}"
        );
        assert_eq!(vi.len(), vi::STRINGS.len(), "key trùng trong vi.rs");
        assert_eq!(en.len(), en::STRINGS.len(), "key trùng trong en.rs");
    }

    #[test]
    fn template_params_match_and_not_empty() {
        for (k, v) in vi::STRINGS {
            assert!(!v.is_empty(), "vi `{k}` rỗng");
            let e = t(Lang::En, k);
            assert!(!e.is_empty(), "en `{k}` rỗng");
            assert_eq!(
                template_params(v),
                template_params(e),
                "tham số lệch ở `{k}`"
            );
        }
    }

    #[test]
    fn fill_replaces_args() {
        assert_eq!(fill("Khu {0} · {1}/10", &["2", "7"]), "Khu 2 · 7/10");
        assert_eq!(fill("{x} {0}", &["a"]), "{x} a");
        assert_eq!(tf(Lang::Vi, "hud_zone", &["2", "7"]), "Khu 2 · 7/10");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(Lang::Vi, 3 * 3600 + 12 * 60 + 5), "3 giờ 12 phút");
        assert_eq!(duration(Lang::En, 45 * 60), "45 min");
    }

    #[test]
    fn locale_detection() {
        assert_eq!(Lang::from_locale("vi_VN.UTF-8"), Lang::Vi);
        assert_eq!(Lang::from_locale("en-US"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    /// SPEC §8.4: font UI phải chứa mọi ký tự xuất hiện trong chuỗi vi/en (+ chữ số, ký hiệu do code ghép).
    #[test]
    fn font_has_all_glyphs() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/ui.ttf");
        let data = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("không đọc được {}: {e}", path.display()));
        let face = ttf_parser::Face::parse(&data, 0).expect("ui.ttf hỏng");
        let mut chars: BTreeSet<char> = BTreeSet::new();
        for lang in [Lang::Vi, Lang::En] {
            for s in all_strings(lang) {
                chars.extend(s.chars());
            }
        }
        // ký tự do code ghép: số rút gọn, phần trăm, ký hiệu độ hiếm, mũi tên so sánh
        chars.extend("0123456789.,%+-×/·:()KMBTabcdefghijklmnopqrstuvwxyz◆★▲▼−→=·—≤".chars());
        let missing: String = chars
            .into_iter()
            .filter(|c| !c.is_whitespace() && face.glyph_index(*c).is_none())
            .collect();
        assert!(missing.is_empty(), "ui.ttf thiếu glyph: {missing:?}");
    }
}
