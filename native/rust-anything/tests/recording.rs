//! `record` fingerprints the original, so a recording made before the original changed is
//! reported as stale instead of passing against old behavior.

mod common;

use std::process::Command;

use rust_anything::bridge::Manifest;
use rust_anything::recording;

#[test]
fn a_changed_original_makes_the_recording_stale() {
    if !common::have("python3") {
        return;
    }
    let root = std::env::temp_dir().join(format!("rust-anything-stale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = \"twice\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("twice.py"),
        "def twice(n: int) -> int:\n    return n * 2\n",
    )
    .unwrap();
    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let crate_dir = root.join("port");
    let run = |args: &[&std::ffi::OsStr]| {
        let out = Command::new(cli).args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
    };
    run(&[
        "bridge".as_ref(),
        root.as_os_str(),
        "twice.py".as_ref(),
        "--out".as_ref(),
        crate_dir.as_os_str(),
    ]);
    run(&["record".as_ref(), crate_dir.as_os_str()]);
    let manifest = Manifest::load(&crate_dir).unwrap();
    assert!(crate_dir.join(recording::RECORDED_FROM).is_file());
    let fresh = recording::stale(&crate_dir, &manifest);
    std::fs::write(
        root.join("twice.py"),
        "def twice(n: int) -> int:\n    return n + n\n",
    )
    .unwrap();
    let changed = recording::stale(&crate_dir, &manifest);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(fresh, None);
    assert!(changed.is_some_and(|why| why.contains("twice.py changed")));
}

fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rust-anything-from-tests-{name}-{}",
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

/// Bridges `unit`, records with `--from-tests -- <test>`, and returns what `record` printed and
/// the captured inputs.
fn from_tests(root: &std::path::Path, unit: &str, test: &[&str]) -> (String, String) {
    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let crate_dir = root.join("port");
    let bridged = Command::new(cli)
        .arg("bridge")
        .arg(root)
        .arg(unit)
        .arg("--out")
        .arg(&crate_dir)
        .output()
        .unwrap();
    assert!(bridged.status.success(), "{bridged:?}");
    let recorded = Command::new(cli)
        .arg("record")
        .arg(&crate_dir)
        .args(["--count", "5", "--from-tests", "--"])
        .args(test)
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{recorded:?}");
    (
        String::from_utf8_lossy(&recorded.stdout).into_owned(),
        std::fs::read_to_string(crate_dir.join("test-inputs.jsonl")).unwrap(),
    )
}

#[test]
fn records_the_calls_a_python_test_suite_makes() {
    if !common::have("python3") {
        return;
    }
    let root = project(
        "py",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"twice\"\nversion = \"0.1.0\"\n",
            ),
            (
                "twice.py",
                "def twice(n: float) -> float:\n    return n * 2\n",
            ),
            (
                "test_twice.py",
                "import math\nimport unittest\nfrom twice import twice\n\n\
                 class T(unittest.TestCase):\n    def test_it(self):\n        \
                 self.assertEqual(twice(21), 42)\n        self.assertEqual(twice(n=1.5), 3.0)\n        \
                 self.assertTrue(math.isnan(twice(float('nan'))))\n",
            ),
        ],
    );
    let (printed, inputs) = from_tests(&root, "twice.py", &["python3", "-m", "unittest"]);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(printed.contains("Captured 3 call(s)"), "{printed}");
    assert!(
        inputs.contains(r#"{"fn": "twice", "args": [21]}"#),
        "{inputs}"
    );
    assert!(inputs.contains(r#""args": [1.5]"#), "{inputs}");
    assert!(inputs.contains(r#"{"$num": "NaN"}"#), "{inputs}");
}

#[test]
fn records_the_calls_a_js_test_suite_makes_and_restores_the_source() {
    if !common::have("node") {
        return;
    }
    let source = "export function label(n: number): string {\n  return `${n}u`;\n}\n\nexport const unwrapped = (s: string): string => s;\n";
    let root = project(
        "ts",
        &[
            (
                "package.json",
                r#"{"name":"label","version":"0.1.0","type":"module"}"#,
            ),
            ("src/label.ts", source),
            (
                "test.ts",
                "import { label, unwrapped } from './src/label.ts'\nif (label(-0) !== '0u' || label(7) !== '7u' || unwrapped('a') !== 'a') process.exit(1)\n",
            ),
        ],
    );
    let (printed, inputs) = from_tests(&root, "src/label.ts", &["node", "test.ts"]);
    let after = std::fs::read_to_string(root.join("src/label.ts")).unwrap();
    let backups = std::fs::read_dir(root.join("src")).unwrap().count();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(after, source);
    assert_eq!(backups, 1);
    assert!(
        printed.contains("Captured 2 call(s) from the tests:"),
        "{printed}"
    );
    assert!(printed.contains("not captured: unwrapped"), "{printed}");
    assert!(
        inputs.contains(r#"{"fn":"label","args":[{"$num":"-0"}]}"#),
        "{inputs}"
    );
    assert!(inputs.contains(r#"{"fn":"label","args":[7]}"#), "{inputs}");
}

#[test]
fn record_reports_lines_of_the_original_no_case_reached() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "coverage",
        &[
            (
                "package.json",
                "{\"name\": \"sign\", \"type\": \"module\"}\n",
            ),
            (
                "sign.ts",
                "export function sign(n: number, word: string): string {\n  if (word === 'xyzzy-never-generated') {\n    return 'magic'\n  }\n  return n < 0 ? 'negative' : 'other'\n}\n",
            ),
        ],
    );
    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let crate_dir = root.join("port");
    let run = |args: &[&std::ffi::OsStr]| {
        let out = Command::new(cli).args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    run(&[
        "bridge".as_ref(),
        root.as_os_str(),
        "sign.ts".as_ref(),
        "--out".as_ref(),
        crate_dir.as_os_str(),
    ]);
    let printed = run(&["record".as_ref(), crate_dir.as_os_str()]);
    let coverage = recording::saved_coverage(&crate_dir);
    std::fs::remove_dir_all(&root).unwrap();
    let coverage = coverage.expect("record saves coverage");
    assert_eq!(coverage["lines"], 6, "{coverage}");
    assert_eq!(coverage["reached"], 4, "{coverage}");
    assert_eq!(coverage["files"][0]["missed"], "3-4", "{coverage}");
    assert!(
        printed.contains("ran 4 of the original's 6 lines"),
        "{printed}"
    );
    assert!(printed.contains("sign.ts: 3-4"), "{printed}");
}
