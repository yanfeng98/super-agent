//! Bridges built for real: compile the generated crate, load it from the original language and
//! replay the recording. Run on every CI platform, since linking differs between them.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::bridge::{self, Kind};
use rust_anything::harness::{self, DiffOptions, RecordOptions};

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rust-anything-native-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// Bridge `unit`, record the original, port `name` with `body`, build, and replay.
fn round_trip(root: &Path, unit: &str, kind: Kind, name: &str, body: &str) {
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root,
        unit,
        kind: Some(kind),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let recorded = harness::record(&RecordOptions {
        crate_dir: &dir,
        count: 30,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    assert!(recorded.cases > 0, "{recorded:?}");

    let port_path = dir.join("src/port.rs");
    let port = std::fs::read_to_string(&port_path).unwrap();
    let start = port
        .find(&format!("pub fn {name}("))
        .unwrap_or_else(|| panic!("no {name} in\n{port}"));
    let open = start + port[start..].find('{').unwrap();
    let close = open + port[open..].find("\n}").unwrap();
    let ported = format!("{}{{\n    {body}{}", &port[..open], &port[close..]);
    std::fs::write(&port_path, ported).unwrap();

    harness::build(&dir).unwrap();
    let outcome = harness::diff(&DiffOptions {
        crate_dir: &dir,
        module: None,
        ignore_key_order: false,
        node_args: &[],
        float_tolerance: None,
    })
    .unwrap();
    assert_eq!(
        (outcome.failed, outcome.passed),
        (0, outcome.total),
        "{:#?}",
        outcome.mismatches
    );

    std::fs::write(&port_path, std::fs::read(&port_path).unwrap()).unwrap();
    let stale = harness::diff(&DiffOptions {
        crate_dir: &dir,
        module: None,
        ignore_key_order: false,
        node_args: &[],
        float_tolerance: None,
    })
    .unwrap_err();
    assert!(
        format!("{stale:#}").contains("rust-anything build"),
        "{stale:#}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn napi_addon_builds_loads_and_matches_in_space_and_unicode_path() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "napi path with spaces ünïcøde",
        &[
            ("package.json", r#"{"name":"shout","version":"0.1.0"}"#),
            (
                "src/shout.ts",
                "export function shout(text: string): string {\n  return text + \"!\";\n}\n",
            ),
        ],
    );
    round_trip(
        &root,
        "src/shout.ts",
        Kind::Napi,
        "shout",
        "Ok(format!(\"{text}!\"))",
    );
}

#[test]
fn pyo3_extension_builds_imports_and_matches() {
    if common::python().is_none() {
        return;
    }
    let root = project(
        "pyo3",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"shout\"\nversion = \"0.1.0\"\n",
            ),
            (
                "shout/__init__.py",
                "def shout(text: str) -> str:\n    return text + \"!\"\n",
            ),
        ],
    );
    round_trip(
        &root,
        "shout/",
        Kind::Pyo3,
        "shout",
        "Ok(format!(\"{text}!\"))",
    );
}

/// A freshly bridged crate is all `unimplemented!` stubs: replaying it must pass no case, even
/// where the original throws the same kind of error a panic turns into.
fn stubs_fail(root: &Path, unit: &str, kind: Option<Kind>, program: &str) {
    if !common::have(program) {
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "rust-anything-stubs-{}-{}",
        root.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    bridge::bridge(&bridge::Options {
        root,
        unit,
        kind,
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let recorded = harness::record(&RecordOptions {
        crate_dir: &dir,
        count: 20,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    assert!(
        recorded.errors > 0,
        "{}: no recorded case throws, so this proves nothing",
        root.display()
    );
    harness::build(&dir).unwrap();
    let outcome = harness::diff(&DiffOptions {
        crate_dir: &dir,
        module: None,
        ignore_key_order: false,
        node_args: &[],
        float_tolerance: None,
    })
    .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(outcome.passed, 0, "a stub matched {outcome:#?}");
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn unported_stubs_never_match_in_typescript() {
    stubs_fail(
        &fixture("classes"),
        "src/stats.ts",
        Some(Kind::Napi),
        "node",
    );
}

#[test]
fn unported_stubs_never_match_in_python() {
    let Some(python) = common::python() else {
        return;
    };
    stubs_fail(&fixture("python"), "src/textkit/", None, python);
}

#[test]
fn unported_stubs_never_match_a_ruby_runtime_error() {
    if cfg!(windows) {
        eprintln!(
            "skipped: RubyInstaller's Ruby is built with MinGW, and an MSVC Rust extension cannot load into it"
        );
        return;
    }
    if !common::have_ruby_3() {
        return;
    }
    let root = project(
        "stubs-ruby",
        &[
            ("shout.gemspec", ""),
            (
                "lib/shout.rb",
                "module Shout\n  def self.shout(s)\n    raise \"empty\" if s.empty?\n    s + \"!\"\n  end\nend\n",
            ),
        ],
    );
    stubs_fail(&root, "lib/shout.rb", None, "ruby");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unported_stubs_never_match_a_java_runtime_exception() {
    let root = project(
        "stubs-java",
        &[
            (
                "pom.xml",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>com.example</groupId>\n  <artifactId>shout</artifactId>\n  <version>0.1.0</version>\n</project>\n",
            ),
            (
                "src/main/java/com/example/shout/Shout.java",
                "package com.example.shout;\n\npublic final class Shout {\n    public static String shout(String s) {\n        if (s.isEmpty()) {\n            throw new RuntimeException(\"empty\");\n        }\n        return s + \"!\";\n    }\n}\n",
            ),
        ],
    );
    stubs_fail(&root, "src/main/java/com/example/shout/", None, "java");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unported_stubs_never_match_a_go_panic() {
    let root = project(
        "stubs-go",
        &[
            ("go.mod", "module example.com/shout\n\ngo 1.22\n"),
            (
                "shout/shout.go",
                "package shout\n\nfunc Shout(s string) string {\n\tif s == \"\" {\n\t\tpanic(\"empty\")\n\t}\n\treturn s + \"!\"\n}\n",
            ),
        ],
    );
    stubs_fail(&root, "shout/", None, "go");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn napi_errors_keep_their_class_and_async_exports_reject() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "napi-errors",
        &[
            ("package.json", r#"{"name":"guard","version":"0.1.0"}"#),
            (
                "src/guard.ts",
                "export function check(text: string): string {\n  if (text === '') throw new TypeError('empty');\n  return text;\n}\n\
                 export async function later(text: string): Promise<string> {\n  return text;\n}\n",
            ),
        ],
    );
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/guard.ts",
        kind: Some(Kind::Napi),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    let port = port
        .replacen(
            "unimplemented!(\"port `check` from src/guard.ts\")",
            "if text.is_empty() { return Err(\"TypeError: empty\".into()); }\n    Ok(text)",
            1,
        )
        .replacen(
            "unimplemented!(\"port `later` from src/guard.ts\")",
            "panic!(\"boom\")",
            1,
        );
    std::fs::write(dir.join("src/port.rs"), &port).unwrap();
    harness::build(&dir).unwrap();
    let script = r#"
const m = require(process.argv[1])
const out = []
try { m.check('') } catch (e) { out.push(e instanceof TypeError, e.message) }
try { m.check('a\uD800') } catch (e) { out.push(e instanceof TypeError && /lone surrogate/.test(e.message)) }
let threw = false
const p = (() => { try { return m.later(5) } catch { threw = true } })()
out.push(!threw && p instanceof Promise)
;(p ? p.catch(e => e.name) : Promise.resolve(null)).then(name => {
  out.push(name)
  return m.later('x').catch(e => e.code)
}).then(code => { out.push(code); console.log(JSON.stringify(out)) })
"#;
    let output = Command::new("node")
        .args(["-e", script])
        .arg(dir.join("index.cjs"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        stdout.trim(),
        r#"[true,"empty",true,true,"TypeError","RUST_PANIC"]"#,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn napi_carries_bytes_maps_sets_dates_and_bigints() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "napi-types",
        &[
            ("package.json", r#"{"name":"types","version":"0.1.0"}"#),
            (
                "src/types.ts",
                "export function size(m: Map<string, number>): number { return m.size }\n\
                 export function echo(b: Uint8Array): Uint8Array { return b }\n\
                 export function big(n: bigint): bigint { return n * 2n }\n\
                 export function day(d: Date): Date { return d }\n\
                 export function keys(s: Set<string>): Set<string> { return s }\n",
            ),
        ],
    );
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/types.ts",
        kind: Some(Kind::Napi),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let mut port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    for (name, body) in [
        (
            "size",
            "Ok(m[\"$map\"].as_array().map_or(0, Vec::len) as f64)",
        ),
        ("echo", "Ok(b)"),
        (
            "big",
            "let n: i128 = n[\"$bigint\"].as_str().ok_or(\"not a bigint\")?.parse().map_err(|e| format!(\"{e}\"))?;\n    Ok(serde_json::json!({ \"$bigint\": (n * 2).to_string() }))",
        ),
        ("day", "Ok(d)"),
        ("keys", "Ok(s)"),
    ] {
        let stub = format!("unimplemented!(\"port `{name}` from src/types.ts\")");
        assert!(port.contains(&stub), "{port}");
        port = port.replacen(&stub, body, 1);
    }
    std::fs::write(dir.join("src/port.rs"), &port).unwrap();
    harness::build(&dir).unwrap();
    let script = r#"
const m = require(process.argv[1])
const bytes = m.echo(new Uint8Array([0, 7, 255]))
const buffer = m.echo(Buffer.from('hi'))
const when = new Date('2024-02-29T12:00:00.000Z')
console.log(JSON.stringify([
  m.size(new Map([['a', 1], ['b', 2]])),
  bytes instanceof Uint8Array && [...bytes].join(','),
  Buffer.isBuffer(buffer) && buffer.toString(),
  String(m.big(9007199254740993n)),
  m.day(when) instanceof Date && m.day(when).getTime() === when.getTime(),
  [...m.keys(new Set(['x', 'y']))].join(','),
]))
"#;
    let output = Command::new("node")
        .args(["-e", script])
        .arg(dir.join("index.cjs"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"[2,"0,7,255","hi","18014398509481986",true,"x,y"]"#,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn java_records_with_the_bridge_crate_inside_the_project() {
    if !common::have("javac") {
        return;
    }
    let root = project(
        "java-default-out",
        &[
            (
                "pom.xml",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>com.example</groupId>\n  <artifactId>smoke</artifactId>\n  <version>0.1.0</version>\n</project>\n",
            ),
            (
                "src/main/java/com/example/smoke/TextKit.java",
                "package com.example.smoke;\n\npublic final class TextKit {\n    public static int wordCount(String s) {\n        return s.isBlank() ? 0 : s.trim().split(\"\\\\s+\").length;\n    }\n}\n",
            ),
        ],
    );
    let outcome = bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/main/java/com/example/smoke/",
        kind: None,
        out: None,
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    assert!(outcome.crate_dir.ends_with("rust/smoke-rs"), "{outcome:?}");
    let recorded = harness::record(&RecordOptions {
        crate_dir: &outcome.crate_dir,
        count: 10,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(recorded.cases > 0, "{recorded:?}");
}

fn bridge_python(root: &Path, unit: &str) -> anyhow::Result<bridge::Outcome> {
    bridge::bridge(&bridge::Options {
        root,
        unit,
        kind: None,
        out: Some(root.join("port")),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
}

#[test]
fn a_name_defined_twice_in_a_unit_is_refused() {
    let root = project(
        "twice",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"hp\"\nversion = \"0.1.0\"\n",
            ),
            (
                "hp/__init__.py",
                "from . import verdict as _v\n\ndef verdict(*args, **options):\n    return _v.verdict(*args, **options)\n",
            ),
            (
                "hp/verdict.py",
                "def verdict(score: int) -> str:\n    return \"ok\" if score > 0 else \"no\"\n",
            ),
        ],
    );
    let err = bridge_python(&root, "hp/").unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    let text = format!("{err:#}");
    assert!(text.contains("`verdict` is defined twice"), "{text}");
    assert!(
        text.contains("hp/__init__.py") && text.contains("hp/verdict.py"),
        "{text}"
    );
}

#[test]
fn cli_entry_points_stay_in_python() {
    let root = project(
        "entry",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"tool\"\nversion = \"0.1.0\"\n",
            ),
            (
                "tool/cli.py",
                "import argparse\n\n\
                 def build_parser() -> argparse.ArgumentParser:\n    return argparse.ArgumentParser()\n\n\
                 def shout(text: str) -> str:\n    return text.upper()\n\n\
                 def main() -> int:\n    build_parser().parse_args()\n    return 0\n\n\
                 if __name__ == \"__main__\":\n    raise SystemExit(main())\n",
            ),
        ],
    );
    let outcome = bridge_python(&root, "tool/").unwrap();
    let manifest = rust_anything::bridge::Manifest::load(&outcome.crate_dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(outcome.exports, ["shout"]);
    let kept: Vec<&str> = manifest.js_only.iter().map(|j| j.name.as_str()).collect();
    assert_eq!(kept, ["build_parser", "main"]);
    assert!(
        manifest.js_only[0]
            .reason
            .contains("command-line entry point")
    );
}

#[test]
fn dataclass_parameters_get_real_instances() {
    if !common::have("python3") {
        return;
    }
    let root = project(
        "dataclass",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"clip\"\nversion = \"0.1.0\"\n",
            ),
            ("clip/__init__.py", ""),
            (
                "clip/models.py",
                "from dataclasses import dataclass\n\n\
                 @dataclass\nclass Limits:\n    low: int\n    high: int = 10\n",
            ),
            (
                "clip/ops.py",
                "from .models import Limits\n\n\
                 def clamp(value: int, limits: Limits) -> int:\n    return max(limits.low, min(value, limits.high))\n",
            ),
        ],
    );
    let outcome = bridge_python(&root, "clip/").unwrap();
    harness::record(&RecordOptions {
        crate_dir: &outcome.crate_dir,
        count: 40,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    let cases = std::fs::read_to_string(outcome.crate_dir.join("cases.jsonl")).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let with_instance = cases
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|c| c["fn"] == "clamp")
        .filter(|c| c["args"][1]["$object"] == "Limits" && c.get("ok").is_some())
        .count();
    assert!(
        with_instance >= 5,
        "only {with_instance} clamp cases ran with a real Limits instance:\n{cases}"
    );
}

#[test]
fn one_file_of_a_unit_can_be_bridged_alone() {
    if !common::have("python3") {
        return;
    }
    let root = project(
        "one-file",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"pkg\"\nversion = \"0.1.0\"\n",
            ),
            ("pkg/__init__.py", ""),
            (
                "pkg/core.py",
                "def score(text: str) -> int:\n    return len(text.split())\n",
            ),
            (
                "pkg/tool.py",
                "def helper(path: str) -> str:\n    return open(path).read()\n",
            ),
        ],
    );
    let outcome = bridge_python(&root, "pkg/core.py").unwrap();
    assert_eq!(outcome.exports, ["score"]);
    let manifest = rust_anything::bridge::Manifest::load(&outcome.crate_dir).unwrap();
    assert_eq!(manifest.unit, "pkg/core.py");
    let recorded = harness::record(&RecordOptions {
        crate_dir: &outcome.crate_dir,
        count: 10,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap();
    let missing = bridge_python(&root, "pkg/nope.py").unwrap_err();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(recorded.cases > 0, "{recorded:?}");
    assert!(format!("{missing:#}").contains("no unit or source file"));
}

#[test]
fn napi_batches_take_structured_parameters() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "batch-options",
        &[
            ("package.json", r#"{"name":"fmt","version":"0.1.0"}"#),
            (
                "src/fmt.ts",
                "export function label(n: number, options?: { long?: boolean }): string {\n  return options?.long ? `${n} units` : `${n}u`;\n}\n",
            ),
        ],
    );
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/fmt.ts",
        kind: Some(Kind::Napi),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: vec!["label:n".into()],
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    let port = port.replacen(
        "unimplemented!(\"port `label` from src/fmt.ts\")",
        "let long = options\n        .as_ref()\n        .and_then(|o| o.get(\"long\"))\n        .and_then(serde_json::Value::as_bool)\n        .unwrap_or(false);\n    Ok(if long { format!(\"{n} units\") } else { format!(\"{n}u\") })",
        1,
    );
    std::fs::write(dir.join("src/port.rs"), &port).unwrap();
    harness::build(&dir).unwrap();
    let script = r#"
const { LabelBatch } = require(process.argv[1])
const batch = new LabelBatch([1, 2])
const refused = [() => new LabelBatch(3), () => new LabelBatch([1, '2'])].map(f => { try { f(); return 'no' } catch (e) { return e.name } })
console.log(JSON.stringify([batch.run(), batch.run({ long: true }), batch instanceof LabelBatch, refused]))
"#;
    let output = Command::new("node")
        .args(["-e", script])
        .arg(dir.join("index.cjs"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"[["1u","2u"],["1 units","2 units"],true,["TypeError","TypeError"]]"#,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn napi_string_batches_cross_joined() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "batch-strings",
        &[
            ("package.json", r#"{"name":"wrap","version":"0.1.0"}"#),
            (
                "src/wrap.ts",
                "export function wrap(s: string): string {\n  return `<${s}>`;\n}\n",
            ),
        ],
    );
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/wrap.ts",
        kind: Some(Kind::Napi),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: vec!["wrap:s".into()],
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    let port = port.replacen(
        "unimplemented!(\"port `wrap` from src/wrap.ts\")",
        "Ok(format!(\"<{s}>\"))",
        1,
    );
    std::fs::write(dir.join("src/port.rs"), &port).unwrap();
    harness::build(&dir).unwrap();
    let script = r#"
const { WrapBatch } = require(process.argv[1])
const items = ['', 'a', 'é', '😀x', '日本', 'b\uD800c', 'ascii only']
const batch = new WrapBatch(items)
const refused = [() => new WrapBatch('ab'), () => new WrapBatch(['a', 1])].map(f => { try { f(); return 'no' } catch (e) { return e.name } })
console.log(JSON.stringify([batch.run(), batch instanceof WrapBatch, refused, new WrapBatch([]).run()]))
"#;
    let output = Command::new("node")
        .args(["-e", script])
        .arg(dir.join("index.cjs"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"[["<>","<a>","<é>","<😀x>","<日本>","<b�c>","<ascii only>"],true,["TypeError","TypeError"],[]]"#,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn python_batches_build_and_run_on_rusty_alloc() {
    if !common::have("python3") {
        return;
    }
    let root = project(
        "rusty-alloc",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"tally\"\nversion = \"0.1.0\"\n",
            ),
            ("tally/__init__.py", ""),
            (
                "tally/words.py",
                "def shout(text: str) -> str:\n    return \" \".join(w.upper() + \"!\" for w in text.split())\n",
            ),
        ],
    );
    let dir = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "tally/",
        kind: None,
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: vec!["shout:text".into()],
        as_json: Vec::new(),
        allocator: bridge::Allocator::RustyAlloc,
    })
    .unwrap();
    let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
    let port = port.replacen(
        "unimplemented!(\"port `shout` from tally/words.py\")",
        "Ok(text\n        .split_whitespace()\n        .map(|w| format!(\"{}!\", w.to_uppercase()))\n        .collect::<Vec<_>>()\n        .join(\" \"))",
        1,
    );
    assert!(port.contains("to_uppercase"), "{port}");
    std::fs::write(dir.join("src/port.rs"), &port).unwrap();
    harness::build(&dir).unwrap();
    let script = r#"
import sys
sys.path[:0] = [sys.argv[1] + "/python", sys.argv[2]]
from tally_rs import ShoutBatch
from tally.words import shout
items = [f"item {i} of the batch, {'x' * (i % 50)}" for i in range(5000)]
print(ShoutBatch(items).run() == [shout(t) for t in items])
"#;
    let output = Command::new("python3")
        .args(["-c", script])
        .arg(&dir)
        .arg(&root)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "True",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
