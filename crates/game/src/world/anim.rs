//! Chọn frame theo `anim` + `anim_t` (thuần, test được).

use cm_core::Anim;

/// Tốc độ (frame/giây) và có lặp không.
pub fn anim_rate(anim: Anim) -> (f32, bool) {
    match anim {
        Anim::Idle => (2.5, true),
        Anim::Walk => (8.0, true),
        Anim::Attack => (10.0, false),
        Anim::Hurt => (8.0, false),
        Anim::Death => (8.0, false),
        Anim::Sleep => (1.5, true),
        Anim::Skill => (10.0, false),
        Anim::Special => (8.0, false),
    }
}

/// Chỉ số frame trong `0..n`. Animation không lặp giữ frame cuối (death nằm yên).
pub fn frame_index(anim: Anim, t: f32, n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let (fps, looping) = anim_rate(anim);
    let i = (t.max(0.0) * fps).floor() as usize;
    if looping { i % n } else { i.min(n - 1) }
}

/// Thứ tự thử khi sprite thiếu animation (vd quái không có `idle`/`sleep`/`skill`).
fn fallbacks(anim: Anim) -> &'static [Anim] {
    match anim {
        Anim::Idle => &[Anim::Idle, Anim::Walk],
        Anim::Walk => &[Anim::Walk, Anim::Idle],
        Anim::Attack => &[Anim::Attack, Anim::Walk],
        Anim::Hurt => &[Anim::Hurt, Anim::Walk],
        Anim::Death => &[Anim::Death, Anim::Hurt],
        Anim::Sleep => &[Anim::Sleep, Anim::Death, Anim::Idle, Anim::Walk],
        Anim::Skill => &[Anim::Skill, Anim::Special, Anim::Attack],
        Anim::Special => &[Anim::Special, Anim::Skill, Anim::Attack],
    }
}

/// Tên sprite cho `prefix` (vd `cat_muop`, `en_rat`, `boss_eel`) ở animation `anim` thời điểm `t`.
/// `len(base)` trả số frame có trong atlas cho `base` (0 nếu không có).
pub fn sprite_name(
    prefix: &str,
    anim: Anim,
    t: f32,
    len: impl Fn(&str) -> usize,
) -> Option<String> {
    for &a in fallbacks(anim) {
        let base = format!("{prefix}_{}", a.key());
        let n = len(&base);
        if n > 0 {
            // ngủ thay bằng frame chết cuối cùng (mèo nằm), giữ yên
            let i = if a != anim && a == Anim::Death {
                n - 1
            } else {
                frame_index(a, t, n)
            };
            return Some(format!("{base}_{i}"));
        }
    }
    None
}

/// Frame cho hiệu ứng chạy một lần trong `dur` giây (None khi đã hết).
pub fn oneshot_index(t: f32, dur: f32, n: usize) -> Option<usize> {
    if t < 0.0 || t >= dur || n == 0 {
        return None;
    }
    Some(((t / dur) * n as f32).floor() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loops_and_holds() {
        assert_eq!(frame_index(Anim::Walk, 0.0, 4), 0);
        assert_eq!(frame_index(Anim::Walk, 0.13, 4), 1);
        assert_eq!(frame_index(Anim::Walk, 0.5, 4), 0); // 4 frame @8fps → vòng lại
        assert_eq!(frame_index(Anim::Death, 5.0, 3), 2); // giữ frame cuối
        assert_eq!(frame_index(Anim::Attack, 0.25, 3), 2);
        assert_eq!(frame_index(Anim::Hurt, 3.0, 1), 0);
        assert_eq!(frame_index(Anim::Idle, -1.0, 2), 0);
    }

    #[test]
    fn falls_back_for_missing_anims() {
        let have = |b: &str| match b {
            "en_rat_walk" => 4,
            "en_rat_death" => 3,
            "en_rat_attack" => 2,
            _ => 0,
        };
        assert_eq!(
            sprite_name("en_rat", Anim::Idle, 0.0, have).as_deref(),
            Some("en_rat_walk_0")
        );
        assert_eq!(
            sprite_name("en_rat", Anim::Sleep, 0.0, have).as_deref(),
            Some("en_rat_death_2")
        );
        assert_eq!(
            sprite_name("en_rat", Anim::Skill, 1.0, have).as_deref(),
            Some("en_rat_attack_1")
        );
        assert_eq!(sprite_name("en_none", Anim::Walk, 0.0, have), None);
    }

    #[test]
    fn oneshot() {
        assert_eq!(oneshot_index(0.0, 0.4, 4), Some(0));
        assert_eq!(oneshot_index(0.39, 0.4, 4), Some(3));
        assert_eq!(oneshot_index(0.4, 0.4, 4), None);
    }
}
