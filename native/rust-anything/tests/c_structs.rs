//! C functions that take and return structs of scalars by value (`AVRational av_mul_q(AVRational,
//! AVRational)`), and that set a `const char **` / `char **` to a position inside a string
//! argument (`av_strstart`, `strtol`): bridged with the original signatures, recorded, ported,
//! diffed and gated, and a wrong port is caught.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::bridge;
use rust_anything::harness::{self, DiffOptions, RecordOptions};

const RATIONAL_H: &str = "#include <stdint.h>\ntypedef int32_t Int;\n\
typedef struct Rational { Int num, den; } Rational;\n\
struct point { double x; double y; };\n\
typedef struct Holder { int *p; } Holder;\n";

const Q_C: &str = r#"#include <stdint.h>
#include "rational.h"

static int64_t gcd(int64_t a, int64_t b)
{
    while (b) {
        int64_t t = a % b;
        a = b;
        b = t;
    }
    return a < 0 ? -a : a;
}

Rational mul_q(Rational b, Rational c)
{
    int64_t num = (int64_t)b.num * c.num, den = (int64_t)b.den * c.den;
    int64_t g = gcd(num, den);
    Rational r = { 0, 1 };
    if (g) {
        r.num = (Int)(num / g);
        r.den = (Int)(den / g);
    }
    if (r.den < 0) {
        r.num = -r.num;
        r.den = -r.den;
    }
    return r;
}

struct point scale(struct point p, double k)
{
    struct point r = { p.x * k, p.y * k };
    return r;
}

int strstart(const char *str, const char *pfx, const char **ptr)
{
    while (*pfx && *pfx == *str) {
        pfx++;
        str++;
    }
    if (!*pfx && ptr)
        *ptr = str;
    return !*pfx;
}

long parse_num(const char *s, char **end)
{
    long v = 0;
    const char *p = s;
    while (*p >= '0' && *p <= '9')
        v = v * 10 + (*p++ - '0');
    if (end)
        *end = (char *)p;
    return v;
}

const char *skip(const char *s, int n)
{
    return s + n;
}

int held(Holder h)
{
    return h.p != 0;
}
"#;

const PORT: &str = r#"
use std::ffi::{c_int, c_long};

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
}

pub fn mul_q(b: crate::Rational, c: crate::Rational) -> crate::Rational {
    let num = i64::from(b.num) * i64::from(c.num);
    let den = i64::from(b.den) * i64::from(c.den);
    let g = gcd(num, den);
    let mut r = crate::Rational { num: 0, den: 1 };
    if g != 0 {
        r.num = (num / g) as i32;
        r.den = (den / g) as i32;
    }
    if r.den < 0 {
        r.num = -r.num;
        r.den = -r.den;
    }
    r
}

pub fn scale(p: crate::point, k: f64) -> crate::point {
    crate::point {
        x: p.x * k,
        y: p.y * k,
    }
}

pub fn strstart(
    str: Option<&[u8]>,
    pfx: Option<&[u8]>,
    ptr: Option<&mut Option<usize>>,
) -> c_int {
    let (Some(s), Some(pfx)) = (str, pfx) else {
        return 0;
    };
    if !s.starts_with(pfx) {
        return 0;
    }
    if let Some(ptr) = ptr {
        *ptr = Some(pfx.len());
    }
    1
}

pub fn skip(s: Option<&[u8]>, n: c_int) -> Option<usize> {
    let s = s?;
    usize::try_from(n).ok().filter(|&n| n <= s.len() + 1)
}

pub fn parse_num(s: Option<&[u8]>, end: Option<&mut Option<usize>>) -> c_long {
    let s = s.unwrap_or_default();
    let digits = s.iter().take_while(|b| b.is_ascii_digit()).count();
    if let Some(end) = end {
        *end = Some(digits);
    }
    s[..digits]
        .iter()
        .fold(0, |v: c_long, &b| v.wrapping_mul(10).wrapping_add(c_long::from(b - b'0')))
}
"#;

const INPUTS: &str = r#"{"fn": "mul_q", "args": [{"num": 1, "den": 2}, {"num": 2, "den": 3}]}
{"fn": "mul_q", "args": [{"num": -3, "den": 4}, {"num": 4, "den": -9}]}
{"fn": "mul_q", "args": [{"num": 0, "den": 0}, {"num": 5}]}
{"fn": "scale", "args": [{"x": 1.5, "y": -2}, 2.0]}
{"fn": "scale", "args": [{"x": 0.1, "y": 3}, -0.5]}
{"fn": "strstart", "args": ["prefix=value", "prefix=", true]}
{"fn": "strstart", "args": ["prefix=value", "nope", true]}
{"fn": "strstart", "args": ["prefix=value", "nope", 3]}
{"fn": "strstart", "args": ["prefix=value", "pre", null]}
{"fn": "strstart", "args": ["abc", "", 2]}
{"fn": "parse_num", "args": ["123abc", true]}
{"fn": "parse_num", "args": ["xyz", true]}
{"fn": "parse_num", "args": ["42", null]}
{"fn": "skip", "args": ["abc", 2]}
{"fn": "skip", "args": ["abc", 100]}
"#;

fn project() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rust-anything-c-structs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/rational.h"), RATIONAL_H).unwrap();
    std::fs::write(dir.join("src/q.c"), Q_C).unwrap();
    dir
}

fn bridged(repo: &Path) -> PathBuf {
    bridge::bridge(&bridge::Options {
        root: repo,
        unit: "src/q.c",
        kind: None,
        out: None,
        name: None,
        force: true,
        keep: Vec::new(),
        lang: Some(rust_anything::lang::Lang::C),
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap()
    .crate_dir
}

fn build(dir: &Path) {
    let built = Command::new(env!("CARGO_BIN_EXE_rust-anything"))
        .args(["build", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

fn diff(dir: &Path) -> harness::DiffOutcome {
    harness::diff(&DiffOptions {
        crate_dir: dir,
        module: None,
        ignore_key_order: false,
        node_args: &[],
        float_tolerance: None,
    })
    .unwrap()
}

#[test]
fn structs_by_value_and_pointers_into_strings_cross_the_c_bridge() {
    if !common::have("cc") || common::python().is_none() || !common::have("cargo") {
        return;
    }
    let repo = project();
    let dir = bridged(&repo);
    let manifest = bridge::Manifest::load(&dir).unwrap();
    let f = |name: &str| manifest.exports.iter().find(|f| f.name == name).unwrap();
    let kinds =
        |name: &str| -> Vec<String> { f(name).params.iter().map(|p| p.ts_type.clone()).collect() };
    let rational = "struct:Rational:int32_t num,int32_t den";
    assert_eq!(kinds("mul_q"), [rational, rational]);
    assert_eq!(f("mul_q").returns, rational);
    assert_eq!(f("scale").returns, "struct:struct point:double x,double y");
    assert_eq!(kinds("strstart"), ["string", "string", "outptr:0"]);
    assert_eq!(kinds("parse_num"), ["string", "outptr:0"]);
    assert!(
        manifest.exports.iter().all(|f| f.name != "held"),
        "a struct holding a pointer stays in C"
    );

    let header = std::fs::read_to_string(dir.join("include/q_rs.h")).unwrap();
    assert!(
        header.contains("typedef struct { int32_t num; int32_t den; } Rational;"),
        "{header}"
    );
    assert!(
        header.contains("struct point scale(struct point p, double k);"),
        "{header}"
    );
    let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    assert!(
        port.contains("pub fn mul_q(b: crate::Rational, c: crate::Rational) -> crate::Rational {"),
        "{port}"
    );
    assert!(port.contains("ptr: Option<&mut Option<usize>>"), "{port}");
    let lib = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("#[repr(C)]") && lib.contains("pub struct Rational {"),
        "{lib}"
    );

    let head = port.split("\n/// Port of").next().unwrap();
    std::fs::write(dir.join("src/port.rs"), format!("{head}\n{PORT}")).unwrap();
    let fmt = Command::new("cargo")
        .args(["fmt", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .status()
        .unwrap();
    assert!(fmt.success());

    harness::record(&RecordOptions {
        crate_dir: &dir,
        count: 40,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    let generated: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("cases.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let of = |name: &'static str| generated.iter().filter(move |c| c["fn"] == name);
    for c in of("mul_q").chain(of("scale")) {
        assert!(c["args"][0].is_object() && c.get("ok").is_some(), "{c}");
    }
    let dens: Vec<&serde_json::Value> = of("mul_q").map(|c| &c["args"][0]["den"]).collect();
    for edge in [0, i64::from(i32::MIN), i64::from(i32::MAX)] {
        assert!(
            dens.contains(&&serde_json::json!(edge)),
            "den {edge} in {dens:?}"
        );
    }
    let starts: Vec<&serde_json::Value> = of("strstart").map(|c| &c["args"][2]).collect();
    for start in [
        serde_json::json!(true),
        serde_json::Value::Null,
        serde_json::json!(0),
    ] {
        assert!(starts.contains(&&start), "{start} in {starts:?}");
    }
    assert!(
        generated
            .iter()
            .all(|c| c["fn"] == "skip" || c.get("ok").is_some()),
        "{:?}",
        generated
            .iter()
            .find(|c| c["fn"] != "skip" && c.get("ok").is_none())
    );

    let inputs = repo.join("inputs.jsonl");
    std::fs::write(&inputs, INPUTS).unwrap();
    harness::record(&RecordOptions {
        crate_dir: &dir,
        count: 20,
        inputs: Some(&inputs),
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    let cases = std::fs::read_to_string(dir.join("cases.jsonl")).unwrap();
    for expected in [
        r#""ok": {"num": 1, "den": 3}"#,
        r#""ok": {"num": 0, "den": 1}"#,
        r#""ok": {"x": 3.0, "y": -4.0}"#,
        r#""args": ["prefix=value", "prefix=", true], "ok": {"return": 1, "ptr": 7}"#,
        r#""args": ["prefix=value", "nope", true], "ok": {"return": 0, "ptr": null}"#,
        r#""args": ["prefix=value", "nope", 3], "ok": {"return": 0, "ptr": 3}"#,
        r#""args": ["abc", "", 2], "ok": {"return": 1, "ptr": 0}"#,
        r#""args": ["123abc", true], "ok": {"return": 123, "end": 3}"#,
        r#""args": ["abc", 2], "ok": 2"#,
        r#""args": ["abc", 100], "error": "returns a pointer outside its arguments""#,
    ] {
        assert!(cases.contains(expected), "{expected} in {cases}");
    }

    build(&dir);
    let outcome = diff(&dir);
    assert_eq!(outcome.passed, outcome.total, "{:?}", outcome.mismatches);
    assert!(outcome.total >= 13);

    let rows = harness::bench(&harness::BenchOptions {
        crate_dir: &dir,
        min_ms: 5,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    assert_eq!(rows.len(), 5, "every function benchmarked");

    let gate = rust_anything::gate::gate(&dir, &[]).unwrap();
    assert!(gate.passed, "{:?}", gate.checks);

    let wrong = PORT
        .replace("r.den = (den / g) as i32;", "r.den = den as i32;")
        .replace("*ptr = Some(pfx.len());", "*ptr = Some(pfx.len() + 1);");
    std::fs::write(dir.join("src/port.rs"), format!("{head}\n{wrong}")).unwrap();
    build(&dir);
    let outcome = diff(&dir);
    let failed: std::collections::BTreeSet<String> = outcome
        .mismatches
        .iter()
        .map(|m| m["fn"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        failed.into_iter().collect::<Vec<_>>(),
        ["mul_q", "strstart"],
        "{:?}",
        outcome.mismatches
    );
    std::fs::remove_dir_all(&repo).unwrap();
}
