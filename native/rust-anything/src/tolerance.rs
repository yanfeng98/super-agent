//! Float tolerance: numeric code (physics, statistics, graphics) rarely gives bit-identical
//! floats once ported, because the order of operations, fused multiply-add and the math library
//! differ. With a tolerance, two results still match when every number differs by at most
//! `tolerance` times the larger magnitude (absolute below 1). Integers, strings and structure
//! must still match exactly, and the same value written differently (`3` and `3.0`) is a
//! formatting difference, not rounding, so it never passes.

use anyhow::{Result, ensure};
use serde_json::Value;

pub fn check(tolerance: f64) -> Result<f64> {
    ensure!(
        tolerance.is_finite() && (0.0..1.0).contains(&tolerance),
        "--float-tolerance must be at least 0 and below 1 (e.g. 1e-9), not {tolerance}"
    );
    Ok(tolerance)
}

pub(crate) fn near(e: f64, a: f64, tolerance: f64) -> bool {
    (e - a).abs() <= tolerance * e.abs().max(a.abs()).max(1.0)
}

/// JSON results: numbers within the tolerance, everything else equal.
pub fn values_close(expected: &Value, actual: &Value, tolerance: f64) -> bool {
    values_close_with_order(expected, actual, tolerance, true)
}

fn values_close_with_order(
    expected: &Value,
    actual: &Value,
    tolerance: f64,
    ignore_key_order: bool,
) -> bool {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            if e == a {
                return true;
            }
            if !e.is_f64() && !a.is_f64() {
                return false;
            }
            match (e.as_f64(), a.as_f64()) {
                (Some(x), Some(y)) => x != y && near(x, y, tolerance),
                _ => false,
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            e.len() == a.len()
                && e.iter()
                    .zip(a)
                    .all(|(x, y)| values_close_with_order(x, y, tolerance, ignore_key_order))
        }
        (Value::Object(e), Value::Object(a)) => {
            e.len() == a.len()
                && (ignore_key_order || e.keys().eq(a.keys()))
                && e.iter().all(|(k, x)| {
                    a.get(k)
                        .is_some_and(|y| values_close_with_order(x, y, tolerance, ignore_key_order))
                })
        }
        _ => expected == actual,
    }
}

/// A line split into number tokens and the text between them.
pub(crate) fn split_numbers(line: &str) -> (Vec<&str>, Vec<&str>) {
    let bytes = line.as_bytes();
    let (mut numbers, mut text) = (Vec::new(), Vec::new());
    let (mut i, mut last) = (0, 0);
    while i < bytes.len() {
        let starts = bytes[i].is_ascii_digit()
            || (bytes[i] == b'-'
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()));
        if !starts || (i > 0 && (bytes[i - 1].is_ascii_alphabetic() || bytes[i - 1] == b'_')) {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            let mut j = i + 1;
            if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                j += 1;
            }
            if j < bytes.len() && bytes[j].is_ascii_digit() {
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        text.push(&line[last..start]);
        numbers.push(&line[start..i]);
        last = i;
    }
    text.push(&line[last..]);
    (numbers, text)
}

fn is_float(token: &str) -> bool {
    token.contains(['.', 'e', 'E'])
}

/// One line of output: the same text around the numbers, and each pair of numbers equal, or a
/// float within the tolerance.
pub fn line_close(expected: &str, actual: &str, tolerance: f64) -> bool {
    let (en, et) = split_numbers(expected);
    let (an, at) = split_numbers(actual);
    if et != at || en.len() != an.len() {
        return false;
    }
    en.iter().zip(&an).all(|(e, a)| {
        if e == a {
            return true;
        }
        if !is_float(e) && !is_float(a) {
            return false;
        }
        match (e.parse::<f64>(), a.parse::<f64>()) {
            (Ok(x), Ok(y)) => x != y && near(x, y, tolerance),
            _ => false,
        }
    })
}

/// A diff outcome with the mismatches that are within the tolerance counted as passed.
pub fn apply(
    mut outcome: crate::harness::DiffOutcome,
    tolerance: f64,
    ignore_key_order: bool,
) -> crate::harness::DiffOutcome {
    let before = outcome.mismatches.len();
    outcome.mismatches.retain(|m| {
        !values_close_with_order(&m["expected"], &m["actual"], tolerance, ignore_key_order)
    });
    let forgiven = before - outcome.mismatches.len();
    outcome.passed += forgiven;
    outcome.failed = outcome.failed.saturating_sub(forgiven);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_numbers_within_tolerance() {
        let t = 1e-9;
        assert!(values_close(&json!(0.30000000000000004), &json!(0.3), t));
        assert!(values_close(
            &json!({"x": [1.0000000001, 2], "n": "a"}),
            &json!({"n": "a", "x": [1.0, 2]}),
            t
        ));
        assert!(!values_close(&json!(0.31), &json!(0.3), t));
        assert!(!values_close(
            &json!(1_000_000_000_001_i64),
            &json!(1_000_000_000_000_i64),
            t
        ));
        assert!(
            !values_close(&json!(3), &json!(3.0), t),
            "formatting, not rounding"
        );
        assert!(!values_close(
            &json!("0.3"),
            &json!("0.30000000000000004"),
            t
        ));
        assert!(!values_close(&json!([1.0]), &json!([1.0, 2.0]), t));
        assert!(values_close(&json!(1e300), &json!(1.0000000000001e300), t));
    }

    #[test]
    fn output_lines_within_tolerance() {
        let t = 1e-9;
        assert!(line_close(
            "x=0.3 y=-1.5e-3",
            "x=0.30000000000000004 y=-1.5000000000001e-3",
            t
        ));
        assert!(line_close("energy 3", "energy 3.0000000000000004", t));
        assert!(!line_close("energy 3", "energy 3.0", t));
        assert!(!line_close("count 10", "count 11", t));
        assert!(!line_close("v1.2", "v1.3", t));
        assert!(!line_close("x=0.3", "y=0.3", t));
        assert!(!line_close("a 0.5", "a 0.5 0.5", t));
        assert!(line_close(
            "step-2 at 0.1",
            "step-2 at 0.10000000000000002",
            t
        ));
    }

    #[test]
    fn mismatches_within_tolerance_count_as_passed() {
        let outcome = crate::harness::DiffOutcome {
            total: 4,
            passed: 1,
            failed: 3,
            missing: Vec::new(),
            mismatches: vec![
                json!({"fn": "f", "args": [1], "expected": 0.1, "actual": 0.10000000000000002}),
                json!({"fn": "f", "args": [2], "expected": {"x": 1.5}, "actual": {"x": 1.5000000000000002}}),
                json!({"fn": "f", "args": [3], "expected": 0.1, "actual": 0.2}),
            ],
            ..Default::default()
        };
        let out = apply(outcome, 1e-9, true);
        assert_eq!((out.passed, out.failed, out.mismatches.len()), (3, 1, 1));
        assert_eq!(out.mismatches[0]["args"], json!([3]));
    }

    #[test]
    fn tolerance_is_checked() {
        assert!(check(1e-9).is_ok());
        assert!(check(-1.0).is_err());
        assert!(check(f64::NAN).is_err());
        assert!(check(1.0).is_err());
    }
}
