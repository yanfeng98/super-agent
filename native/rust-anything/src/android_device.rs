//! On-device recording and diffing over adb.
//!
//! The cases are generated on the host (same generator as `record`), but the original Kotlin
//! runs under ART and the result comes back as the same `cases.jsonl`. Both sides use the
//! generated `RustAnythingDriver`: it is compiled to Java classes, dexed with d8 together with
//! the original Kotlin jar (record) or the generated JNI wrappers (diff), and started with
//! `app_process`. This covers logic units; Android UI, lifecycle and SDK-only classes stay
//! outside the profile.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::DiffOutcome;
use crate::lang::Lang;

/// A connected device and its scratch directory. The directory is removed on drop, including
/// when a step fails, so a recording does not leave jars and libraries behind on the device.
struct Device {
    serial: String,
    dir: String,
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = adb(
            Some(&self.serial),
            &["shell", "rm", "-rf", &self.dir],
            "device cleanup",
            Duration::from_secs(10),
        );
    }
}

impl Device {
    fn connect(explicit: Option<&str>) -> Result<Self> {
        let serial = select(explicit)?;
        let dir = format!("/data/local/tmp/rust-anything-{}", std::process::id());
        adb(
            Some(&serial),
            &["shell", "mkdir", "-p", &dir],
            "device scratch directory",
            Duration::from_secs(10),
        )?;
        Ok(Self { serial, dir })
    }

    fn push(&self, local: &Path, name: &str) -> Result<String> {
        let remote = format!("{}/{}", self.dir, name);
        adb(
            Some(&self.serial),
            &["push", &local.to_string_lossy(), &remote],
            &format!("push {name}"),
            Duration::from_secs(60),
        )?;
        Ok(remote)
    }

    fn pull(&self, remote: &str, local: &Path) -> Result<()> {
        adb(
            Some(&self.serial),
            &["pull", remote, &local.to_string_lossy()],
            &format!("pull {remote}"),
            Duration::from_secs(60),
        )?;
        Ok(())
    }

    fn abi(&self) -> Result<String> {
        let out = adb(
            Some(&self.serial),
            &["shell", "getprop", "ro.product.cpu.abi"],
            "device ABI",
            Duration::from_secs(10),
        )?;
        let abi = out.trim();
        if abi.is_empty() {
            bail!("the device did not report ro.product.cpu.abi");
        }
        Ok(abi.to_string())
    }

    /// Runs the dexed driver under ART. `RA_EXIT` lets us tell a driver failure (stack trace on
    /// the device) apart from an adb transport failure.
    fn driver(
        &self,
        jar: &str,
        properties: &[(&str, &str)],
        mode: &str,
        args: &[&str],
    ) -> Result<()> {
        // The env assignment must come first: `CLASSPATH=x app_process -D... /system/bin ...`.
        let mut parts = vec![format!("CLASSPATH={jar}"), "app_process".to_string()];
        parts.push("-Dfile.encoding=UTF-8".to_string());
        parts.extend(
            properties
                .iter()
                .map(|(key, value)| format!("-D{key}={value}")),
        );
        parts.extend(
            ["/system/bin", "rustanything.RustAnythingDriver", mode]
                .iter()
                .map(|s| s.to_string()),
        );
        parts.extend(args.iter().map(|s| s.to_string()));
        let script = format!("{} 2>&1; echo RA_EXIT=$?", parts.join(" "));
        let out = adb(
            Some(&self.serial),
            &["shell", &script],
            &format!("driver ({mode})"),
            crate::process::BUILD,
        )?;
        let code = out
            .lines()
            .rev()
            .find_map(|l| l.trim().strip_prefix("RA_EXIT="))
            .unwrap_or("missing");
        if code != "0" {
            bail!("the Android driver failed (exit {code}):\n{}", out.trim());
        }
        Ok(())
    }
}

fn adb(serial: Option<&str>, args: &[&str], what: &str, timeout: Duration) -> Result<String> {
    let mut cmd = Command::new("adb");
    if let Some(serial) = serial {
        cmd.args(["-s", serial]);
    }
    cmd.args(args);
    let out = crate::process::output(&mut cmd, timeout)
        .with_context(|| format!("running adb ({what}); is adb on PATH?"))?;
    if !out.success() {
        let text = if out.stderr_text().trim().is_empty() {
            out.stdout_text()
        } else {
            out.stderr_text()
        };
        bail!("adb {what} failed: {}", text.trim());
    }
    Ok(out.stdout_text())
}

fn select(explicit: Option<&str>) -> Result<String> {
    if let Some(serial) = explicit {
        return Ok(serial.to_string());
    }
    if let Ok(serial) = std::env::var("ANDROID_SERIAL")
        && !serial.trim().is_empty()
    {
        return Ok(serial);
    }
    let list = adb(None, &["devices"], "device list", Duration::from_secs(10))?;
    let devices: Vec<&str> = list
        .lines()
        .filter_map(|l| l.strip_suffix("\tdevice"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    match devices.as_slice() {
        [one] => Ok((*one).to_string()),
        [] => bail!("no connected Android device; connect one or pass --device <serial>"),
        many => bail!(
            "{} Android devices connected ({}); pass --device <serial>",
            many.len(),
            many.join(", ")
        ),
    }
}

/// d8 from the newest installed build-tools and the newest platform's `android.jar`.
fn sdk_tools() -> Result<(PathBuf, PathBuf)> {
    let mut candidates: Vec<PathBuf> = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    candidates.extend(crate::android::sdk_folders());
    // ANDROID_HOME often points at an old or deleted SDK: pick the first real one.
    let sdk = candidates
        .into_iter()
        .find(|p| p.join("build-tools").is_dir() && p.join("platforms").is_dir())
        .context("Android SDK not found; set ANDROID_HOME")?;
    let newest = |dir: PathBuf, file: &str| -> Option<PathBuf> {
        let mut versions: Vec<(Vec<u32>, PathBuf)> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join(file).is_file())
            .map(|p| {
                let key = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
                    .split(|c: char| !c.is_ascii_digit())
                    .filter_map(|part| part.parse().ok())
                    .collect();
                (key, p)
            })
            .collect();
        versions.sort_by(|a, b| a.0.cmp(&b.0));
        versions.pop().map(|(_, p)| p)
    };
    let d8 = newest(sdk.join("build-tools"), "d8")
        .context("no d8 in the SDK's build-tools; install build-tools with sdkmanager")?
        .join("d8");
    let jar = newest(sdk.join("platforms"), "android.jar")
        .context("no android.jar in the SDK's platforms; install a platform with sdkmanager")?
        .join("android.jar");
    Ok((d8, jar))
}

/// Compiles Java classes (and optionally the original Kotlin fat jar) into one dexed jar ART can
/// put on its classpath.
fn package(classes: &[PathBuf], work: &Path) -> Result<PathBuf> {
    let (d8, android_jar) = sdk_tools()?;
    let dex = work.join("dex");
    if dex.exists() {
        std::fs::remove_dir_all(&dex)?;
    }
    std::fs::create_dir_all(&dex)?;
    let mut cmd = Command::new(&d8);
    cmd.arg("--min-api")
        .arg(crate::android::DEFAULT_API.to_string())
        .arg("--lib")
        .arg(&android_jar)
        .arg("--output")
        .arg(&dex)
        .args(classes);
    tool(&mut cmd, "d8 (dex driver)")?;
    let jar = work.join("driver.jar");
    let mut cmd = Command::new("jar");
    cmd.arg("cf")
        .arg(&jar)
        .arg("-C")
        .arg(&dex)
        .arg("classes.dex");
    tool(&mut cmd, "jar (package driver)")?;
    Ok(jar)
}

fn tool(cmd: &mut Command, what: &str) -> Result<()> {
    let out = crate::process::output(cmd, crate::process::BUILD)
        .with_context(|| format!("running {what}"))?;
    if !out.success() {
        bail!(
            "{what} failed: {}",
            if out.stderr_text().trim().is_empty() {
                out.stdout_text()
            } else {
                out.stderr_text()
            }
            .trim()
        );
    }
    Ok(())
}

fn work_dir(crate_dir: &Path, mode: &str) -> Result<PathBuf> {
    let dir = crate_dir
        .join(crate::harness::WORK_DIR)
        .join("android-device")
        .join(mode);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn check_language(m: &Manifest) -> Result<()> {
    if !matches!(m.language, Lang::Java | Lang::Kotlin) {
        bail!(
            "on-device recording needs a JVM crate (Java or Kotlin), not {}",
            m.language.name()
        );
    }
    Ok(())
}

fn targets(m: &Manifest, work: &Path) -> Result<PathBuf> {
    let path = work.join("targets.json");
    std::fs::write(&path, crate::codegen_kotlin::targets(m).to_string())?;
    Ok(path)
}

/// Records the original JVM unit on the device; returns the host's `cases.jsonl`.
pub fn record(
    crate_dir: &Path,
    m: &Manifest,
    cases: &[Value],
    device: Option<&str>,
) -> Result<PathBuf> {
    check_language(m)?;
    let device = Device::connect(device)?;
    let work = work_dir(crate_dir, "record")?;
    let classes = if m.language == Lang::Java {
        crate::harness_java::android_classes(crate_dir, m, false)?
    } else {
        crate::harness_kotlin::android_classes(crate_dir, m, false)?
    };
    let jar = package(&classes, &work)?;
    let targets = targets(m, &work)?;
    let inputs = work.join("cases.json");
    // The driver's record mode takes one JSON array, as it does on the host.
    std::fs::write(&inputs, serde_json::to_string(cases)?)?;
    let remote_jar = device.push(&jar, "driver.jar")?;
    let remote_targets = device.push(&targets, "targets.json")?;
    let remote_inputs = device.push(&inputs, "cases.json")?;
    let remote_out = format!("{}/out.jsonl", device.dir);
    device.driver(
        &remote_jar,
        &[],
        "record",
        &[&remote_targets, &remote_inputs, &remote_out, "0"],
    )?;
    let local = work.join("out.jsonl");
    device.pull(&remote_out, &local)?;
    let out = crate_dir.join(crate::harness::CASES);
    crate::harness::install(&local, &out)?;
    Ok(out)
}

/// Diffs the Android `.so` built for the connected device's ABI against the recording.
pub fn diff(
    crate_dir: &Path,
    m: &Manifest,
    device: Option<&str>,
    jni_libs: Option<&Path>,
) -> Result<DiffOutcome> {
    check_language(m)?;
    let cases = crate_dir.join(crate::harness::CASES);
    if !cases.is_file() {
        bail!(
            "no {} in {} (run `rust-anything record --profile android` first)",
            crate::harness::CASES,
            crate_dir.display()
        );
    }
    let device = Device::connect(device)?;
    let abi = device.abi()?;
    let base = jni_libs
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate_dir.join("jniLibs"));
    let lib = base.join(&abi).join(format!("lib{}.so", m.lib_name()));
    if !lib.is_file() {
        bail!(
            "no {} for the device ABI {abi}; run `rust-anything build {crate} --profile android --abi {abi}` first",
            lib.display(),
            crate = crate_dir.display()
        );
    }
    if crate::harness::sources_newer_than(crate_dir, &lib) {
        bail!(
            "{} is older than the crate's sources, so diff would test the previous port; run \
             `rust-anything build {crate} --profile android --abi {abi}` first",
            lib.display(),
            crate = crate_dir.display()
        );
    }
    let work = work_dir(crate_dir, "diff")?;
    let classes = if m.language == Lang::Java {
        crate::harness_java::android_classes(crate_dir, m, true)?
    } else {
        crate::harness_kotlin::android_classes(crate_dir, m, true)?
    };
    let jar = package(&classes, &work)?;
    let targets = targets(m, &work)?;
    let remote_jar = device.push(&jar, "driver.jar")?;
    let remote_targets = device.push(&targets, "targets.json")?;
    let remote_cases = device.push(&cases, "cases.jsonl")?;
    let remote_lib = device.push(&lib, "port.so")?;
    let remote_out = format!("{}/diff.json", device.dir);
    device.driver(
        &remote_jar,
        &[(
            &format!("rust-anything.lib.{}", m.lib_name()),
            remote_lib.as_str(),
        )],
        "diff",
        &[&remote_targets, &remote_cases, &remote_out, "0"],
    )?;
    let local = work.join("diff.json");
    device.pull(&remote_out, &local)?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&local)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_an_explicit_serial_without_adb() {
        assert_eq!(select(Some("emulator-5554")).unwrap(), "emulator-5554");
    }
}
