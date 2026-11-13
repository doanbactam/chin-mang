//! Định dạng số rút gọn (SPEC §3).

/// `< 1000` → số nguyên; rồi 1.23K, 45.6M, 789B, 1.00T, sau đó aa, ab, … (3 chữ số có nghĩa).
pub fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return "∞".into();
    }
    if v < 0.0 {
        return format!("-{}", fmt_num(-v));
    }
    if v < 1000.0 {
        return format!("{}", v.floor() as u64);
    }
    let mut exp = 0usize;
    let mut m = v;
    while m >= 1000.0 {
        m /= 1000.0;
        exp += 1;
    }
    // làm tròn xuống 3 chữ số có nghĩa để không hiện "1000K"
    let (m, digits) = if m >= 100.0 {
        (m.floor(), 0)
    } else if m >= 10.0 {
        ((m * 10.0).floor() / 10.0, 1)
    } else {
        ((m * 100.0).floor() / 100.0, 2)
    };
    format!("{:.*}{}", digits, m, suffix(exp))
}

fn suffix(exp: usize) -> String {
    match exp {
        1 => "K".into(),
        2 => "M".into(),
        3 => "B".into(),
        4 => "T".into(),
        n => {
            let i = n - 5;
            let a = (b'a' + (i / 26) as u8 % 26) as char;
            let b = (b'a' + (i % 26) as u8) as char;
            format!("{a}{b}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_num;
    #[test]
    fn formats() {
        assert_eq!(fmt_num(0.0), "0");
        assert_eq!(fmt_num(999.9), "999");
        assert_eq!(fmt_num(1234.0), "1.23K");
        assert_eq!(fmt_num(45_678_000.0), "45.6M");
        assert_eq!(fmt_num(789e9), "789B");
        assert_eq!(fmt_num(1e12), "1.00T");
        assert_eq!(fmt_num(1e15), "1.00aa");
        assert_eq!(fmt_num(2.5e18), "2.50ab");
        assert_eq!(fmt_num(999_999.0), "999K");
    }
}
