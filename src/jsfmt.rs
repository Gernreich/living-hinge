//! Number formatting that matches JavaScript exactly, so ported generators
//! write byte-identical files.

/// JavaScript `String(x)`: shortest round-trip digits, exponent form
/// outside 1e-6 <= |x| < 1e21.
pub fn num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    let neg = x < 0.0;
    let e = format!("{:e}", x.abs()); // e.g. "1.2345e-7", shortest digits
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp + 1; // decimal point position
    let body = if k <= n && n <= 21 {
        format!("{}{}", digits, "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let sign = if n - 1 >= 0 { "+" } else { "-" };
        let m = if k == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!("{}e{}{}", m, sign, (n - 1).abs())
    };
    if neg { format!("-{}", body) } else { body }
}

/// JavaScript `x.toFixed(f)`: exact decimal value, ties round up in magnitude.
pub fn fixed(x: f64, f: usize) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return num(x);
    }
    let neg = x < 0.0;
    let exact = format!("{:.1100}", x.abs()); // exact binary expansion
    let (int, frac) = exact.split_once('.').unwrap();
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes().take(f)).collect();
    if frac.as_bytes()[f] >= b'5' {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits[i] == b'9' {
                digits[i] = b'0';
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let s = String::from_utf8(digits).unwrap();
    let il = s.len() - f;
    let body = if f == 0 { s } else { format!("{}.{}", &s[..il], &s[il..]) };
    if neg { format!("-{}", body) } else { body }
}

/// JavaScript `Math.round(x)`.
pub fn round(x: f64) -> f64 {
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x.is_sign_negative() { -0.0 } else { r } // Math.round(-0.3) is -0
}

/// What `console.log` prints for a number: `String(x)`, except that -0 shows.
pub fn ins(x: f64) -> String {
    if x == 0.0 && x.is_sign_negative() { "-0".into() } else { num(x) }
}

/// JavaScript `Math.min(a, b)`: NaN wins, and -0 is below +0.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else if b < a {
        b
    } else if a.is_sign_negative() {
        a
    } else {
        b
    }
}

/// JavaScript `Math.max(a, b)`: NaN wins, and +0 is above -0.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else if b > a {
        b
    } else if a.is_sign_negative() {
        b
    } else {
        a
    }
}

/// JavaScript `Number(s)` for a decimal string, as the generators read their
/// environment: surrounding whitespace ignored, empty is 0, junk is NaN.
pub fn number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if t.is_empty() {
        return 0.0;
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'+' => (1.0, &t[1..]),
        b'-' => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    let radix = match body.get(..2) {
        Some("0x") | Some("0X") => Some(16),
        Some("0o") | Some("0O") => Some(8),
        Some("0b") | Some("0B") => Some(2),
        _ => None,
    };
    if let Some(r) = radix {
        // JavaScript allows no sign on these.
        if sign < 0.0 || t.starts_with('+') || body.len() == 2 {
            return f64::NAN;
        }
        let mut v = 0.0f64;
        for c in body[2..].chars() {
            match c.to_digit(r) {
                Some(d) => v = v * r as f64 + d as f64,
                None => return f64::NAN,
            }
        }
        return v;
    }
    // Rust's parser also takes "inf", "nan" and "infinity"; JavaScript does not.
    let ok = !body.is_empty()
        && body.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
        && body.chars().any(|c| c.is_ascii_digit());
    if !ok {
        return f64::NAN;
    }
    match t.parse::<f64>() {
        Ok(v) => v,
        Err(_) => f64::NAN,
    }
}

/// JavaScript `x.toExponential(f)`.
pub fn exponential(x: f64, f: usize) -> String {
    if !x.is_finite() {
        return num(x);
    }
    let neg = x < 0.0;
    if x == 0.0 {
        let z = if f == 0 { "0".to_string() } else { format!("0.{}", "0".repeat(f)) };
        return format!("{}e+0", z);
    }
    // Plenty of exact digits, then round half up by hand as JavaScript does.
    let s = format!("{:.80e}", x.abs());
    let (mant, exp) = s.split_once('e').unwrap();
    let mut e: i32 = exp.parse().unwrap();
    let digits: Vec<u8> = mant.bytes().filter(|b| *b != b'.').collect();
    let mut kept: Vec<u8> = digits[..=f].to_vec();
    if digits[f + 1] >= b'5' {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                kept.pop();
                e += 1;
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let k = String::from_utf8(kept).unwrap();
    let m = if f == 0 { k } else { format!("{}.{}", &k[..1], &k[1..]) };
    let sign = if neg { "-" } else { "" };
    format!("{}{}e{}{}", sign, m, if e < 0 { "-" } else { "+" }, e.abs())
}

/// JavaScript `n.toLocaleString()` (en-US) for an integer.
pub fn locale_int(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{}", out) } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_js() {
        assert_eq!(num(116.08320000000002), "116.08320000000002");
        assert_eq!(num(1e21), "1e+21");
        assert_eq!(num(1.5e-7), "1.5e-7");
        assert_eq!(num(0.000001), "0.000001");
        assert_eq!(num(4.0), "4");
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(1.005, 2), "1.00");
        assert_eq!(fixed(-0.001, 2), "-0.00");
        assert_eq!(fixed(-0.0, 2), "0.00");
        assert_eq!(num(123456789012345680000.0), "123456789012345680000");
        assert_eq!(num(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(fixed(9.995, 2), "9.99");
        assert_eq!(fixed(99.5, 0), "100");
        assert_eq!(locale_int(9435), "9,435");
        assert_eq!(round(0.49999999999999994), 0.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(2.5), 3.0);
        assert!(round(-0.3).is_sign_negative());
        assert_eq!(number(" 30 "), 30.0);
        assert_eq!(number(""), 0.0);
        assert_eq!(number("0x10"), 16.0);
        assert!(number("inf").is_nan());
        assert!(number("3mm").is_nan());
        assert_eq!(number("-Infinity"), f64::NEG_INFINITY);
        assert_eq!(number(".5"), 0.5);
        assert_eq!(number("1e3"), 1000.0);
        assert_eq!(exponential(1.2345e-10, 3), "1.234e-10");
        assert!(number("-0x10").is_nan());
        assert_eq!(number("1."), 1.0);
        assert_eq!(number("+.5e1"), 5.0);
        assert_eq!(exponential(0.0, 3), "0.000e+0");
        assert_eq!(exponential(9.9996, 3), "1.000e+1");
        assert_eq!(exponential(123.0, 3), "1.230e+2");
        assert_eq!(ins(-0.0), "-0");
    }
}
