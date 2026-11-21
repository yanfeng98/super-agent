/// JS `Number.prototype.toString()` (what `${n}` and `String(n)` print): `1e+21`, `5e-324`,
/// `0` for `-0`, `NaN`, `Infinity`, and the shortest digits that read back as the same number
/// (the closest of them on a tie, as V8 picks). The digits come from serde_json's printer, several
/// times faster than `format!`.
#[allow(dead_code)]
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.fract() == 0.0 && x.abs() < 9007199254740992.0 {
        return (x as i64).to_string();
    }
    let json = serde_json::to_string(&x).unwrap_or_default();
    let (sign, json) = match json.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", json.as_str()),
    };
    let (mantissa, exponent) = match json.find(['e', 'E']) {
        Some(i) => (&json[..i], json[i + 1..].parse::<i64>().unwrap_or(0)),
        None => (json, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let lead = all.len() - all.trim_start_matches('0').len();
    let digits = all[lead..].trim_end_matches('0');
    let k = digits.len() as i64;
    let n = int.len() as i64 - lead as i64 + exponent;
    let mut out = String::with_capacity(digits.len() + 8);
    out.push_str(sign);
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.abs().to_string());
    }
    out
}

/// JS `Math.round`: the nearest integer, halves toward positive infinity (`-2.5` gives `-2`,
/// where Rust's `round` gives `-3`), and `-0` for negatives that round to zero.
#[allow(dead_code)]
pub fn math_round(x: f64) -> f64 {
    if !x.is_finite() || x == x.trunc() {
        return x;
    }
    let floor = x.floor();
    let r = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

#[cfg(test)]
mod js_number_tests {
    use super::*;

    #[test]
    fn prints_numbers_as_js_does() {
        for (x, js) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-7.5, "-7.5"),
            (0.1, "0.1"),
            (0.30000000000000004, "0.30000000000000004"),
            (1e21, "1e+21"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (2.5e-7, "2.5e-7"),
            (9007199254740993.0, "9007199254740992"),
            (-123456789.0, "-123456789"),
            (1e300, "1e+300"),
            (153596401214599.62, "153596401214599.62"),
            (-25.728225708007812, "-25.728225708007812"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
        ] {
            assert_eq!(number_to_string(x), js, "{x:?}");
        }
    }

    #[test]
    fn rounds_as_math_round() {
        for (x, js) in [
            (2.5, 3.0),
            (-2.5, -2.0),
            (-2.6, -3.0),
            (0.49999999999999994, 0.0),
            (1.5, 2.0),
            (-0.4, -0.0),
            (4503599627370495.5, 4503599627370496.0),
            (-7.0, -7.0),
        ] {
            let r = math_round(x);
            assert_eq!(r.to_bits(), f64::to_bits(js), "{x}: {r}");
        }
    }
}
