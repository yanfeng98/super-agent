//! On-device recording over adb. Skips unless a device, kotlinc, d8, android.jar and `jar` are
//! present, because CI has none of them; the same path was checked by hand on an Android 11 TV.

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::bridge;
use rust_anything::harness::{self, RecordOptions};

fn runs(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn sdk_ready() -> bool {
    let mut candidates: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    candidates.extend(rust_anything::android::sdk_folders());
    candidates.into_iter().any(|sdk| {
        let d8 = std::fs::read_dir(sdk.join("build-tools"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().join("d8").is_file());
        let platform = std::fs::read_dir(sdk.join("platforms"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().join("android.jar").is_file());
        d8 && platform
    })
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn records_a_kotlin_unit_on_a_connected_device() {
    let devices = runs("adb", &["devices"]).unwrap_or_default();
    let one_device = devices.lines().filter(|l| l.ends_with("\tdevice")).count() == 1;
    if !(std::env::var_os("ANDROID_SERIAL").is_some() || one_device)
        || runs("kotlinc", &["-version"]).is_none()
        || runs("jar", &["--version"]).is_none()
        || !sdk_ready()
    {
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kotlin");
    let dir = std::env::temp_dir().join(format!("rust-anything-device-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _cleanup = Cleanup(dir.clone());
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "src/main/kotlin/com/example/text/",
        kind: None,
        out: Some(dir.clone()),
        name: Some("text-rs".into()),
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let outcome = harness::record_android(
        &RecordOptions {
            crate_dir: &dir,
            count: 3,
            inputs: None,
            module: None,
            resolve_from: None,
            node_args: &[],
        },
        None,
    )
    .unwrap();
    assert!(outcome.cases > 20, "{outcome:?}");
    let per_function: usize = outcome.per_function.values().sum();
    assert_eq!(outcome.cases, per_function, "{outcome:?}");
    assert!(outcome.path.is_file(), "{outcome:?}");
    let text = std::fs::read_to_string(&outcome.path).unwrap();
    assert_eq!(
        text.lines().filter(|l| !l.is_empty()).count(),
        outcome.cases
    );
}
