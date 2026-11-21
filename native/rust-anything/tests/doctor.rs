//! `doctor --json`: one entry per check, and the exit code follows the required checks.

use std::process::Command;

use serde_json::Value;

#[test]
fn doctor_json_lists_every_check_with_a_fix_for_each_problem() {
    let home = std::env::temp_dir().join(format!("rust-anything-doctor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rust-anything"))
        .args(["doctor", "--json"])
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!("{e}: {}", String::from_utf8_lossy(&out.stdout));
    });
    let checks = report["checks"].as_array().expect("a checks array");
    let mut all_required_ok = true;
    for c in checks {
        let status = c["status"].as_str().expect("a status");
        assert!(
            ["ok", "missing", "outdated", "foreign"].contains(&status),
            "{c}"
        );
        assert!(c["name"].is_string() && c["detail"].is_string(), "{c}");
        assert_eq!(c["fix"].is_string(), status != "ok", "{c}");
        if c["required"].as_bool().expect("required is a bool") && status != "ok" {
            all_required_ok = false;
        }
    }
    let names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();
    for name in [
        "cargo",
        "node",
        "python3",
        "go",
        "ruby",
        "java",
        "mcp server",
    ] {
        assert!(names.contains(&name), "no {name} check in {names:?}");
    }
    let mcp = checks.iter().find(|c| c["name"] == "mcp server").unwrap();
    assert_eq!(mcp["status"], "ok", "{mcp}");
    assert_eq!(out.status.success(), all_required_ok, "{report:#}");
    assert!(matches!(out.status.code(), Some(0 | 1)));
}
