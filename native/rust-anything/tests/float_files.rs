mod common;

use std::process::Command;

const WAVE: &str = r#"import math, struct, sys
n, out = int(sys.argv[1]), sys.argv[2]
shift = float(__import__("os").environ.get("SHIFT", "0"))
xs = [math.exp(-(4.0 * i / (n - 1))) * (1 + shift) for i in range(n)]
open(out, "wb").write(struct.pack(f"<{n}f", *xs))
print("wrote", n)
"#;

#[test]
fn float_files_match_within_the_tolerance_and_report_real_differences() {
    let Some(python) = common::python() else {
        return;
    };
    let repo = std::env::temp_dir().join(format!("rust-anything-floats-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("wave.py"), WAVE).unwrap();
    std::fs::write(
        repo.join("inputs.jsonl"),
        "{\"name\": \"wave\", \"args\": [\"1000\", \"out.f32\"]}\n",
    )
    .unwrap();
    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let recorded = Command::new(cli)
        .args([
            "app",
            "record",
            ".",
            "--inputs",
            "inputs.jsonl",
            "--no-traps",
        ])
        .args(["--float-tolerance", "1e-4", "--float-file", "out.f32=f32le"])
        .args(["--", python, "wave.py"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{recorded:?}");

    let replay = |shift: &str| {
        Command::new(cli)
            .args(["app", "replay", ".", "--build"])
            .arg(format!("{python} wave.py"))
            .env("SHIFT", shift)
            .current_dir(&repo)
            .output()
            .unwrap()
    };
    let rounded = replay("1e-7");
    let broken = replay("1e-2");
    let report =
        std::fs::read_to_string(repo.join("rust/app-cases/replay-report.md")).unwrap_or_default();
    std::fs::remove_dir_all(&repo).unwrap();
    assert!(rounded.status.success(), "{rounded:?}");
    assert!(!broken.status.success(), "{broken:?}");
    assert!(
        report.contains("values differ beyond the tolerance"),
        "{report}"
    );
}
