//! Android NDK builds: the JNI `cdylib` for each ABI, installed into `jniLibs/`.
//!
//! The JVM records and diffs on the host; an Android app needs the same library built for
//! its ABIs instead. This links with the NDK's clang directly, so `cargo-ndk` is not needed:
//! the only moving parts are the linker path and the Rust target.
//!
//! The recording step still runs on a desktop JVM (phase 3, on-device recording, is not
//! implemented). What this adds is a library Gradle can package: `<crate>/jniLibs/<abi>/lib*.so`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::bridge::{Kind, Manifest};
use crate::lang::Lang;

/// The ABIs built when `--abi` is not given: phones (64 and 32 bit) and emulators.
pub const DEFAULT_ABIS: &[Abi] = &[Abi::Arm64, Abi::Armv7, Abi::X86_64];

/// The generated Java runtime uses `java.util.Optional` and `java.util.function` (API 24) and
/// `ThreadLocal.withInitial` (API 26), so libraries built for a lower API would load but fail in
/// the wrapper before the first call reaches Rust. API 26 is the honest floor.
pub const MIN_API: u32 = 26;

/// The API level the libraries target when `--api` is not given.
pub const DEFAULT_API: u32 = MIN_API;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abi {
    Arm64,
    Armv7,
    X86_64,
    X86,
}

impl Abi {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "arm64-v8a" => Ok(Self::Arm64),
            "armeabi-v7a" => Ok(Self::Armv7),
            "x86_64" => Ok(Self::X86_64),
            "x86" => Ok(Self::X86),
            other => bail!(
                "unknown Android ABI `{other}`; use one of arm64-v8a, armeabi-v7a, x86_64, x86"
            ),
        }
    }

    /// The name Gradle uses for the ABI directory.
    pub fn name(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64-v8a",
            Self::Armv7 => "armeabi-v7a",
            Self::X86_64 => "x86_64",
            Self::X86 => "x86",
        }
    }

    /// The Rust target triple.
    pub fn triple(self) -> &'static str {
        match self {
            Self::Arm64 => "aarch64-linux-android",
            Self::Armv7 => "armv7-linux-androideabi",
            Self::X86_64 => "x86_64-linux-android",
            Self::X86 => "i686-linux-android",
        }
    }

    /// Android's 16 KB page-size requirement applies to 64-bit libraries.
    fn is_64_bit(self) -> bool {
        matches!(self, Self::Arm64 | Self::X86_64)
    }

    /// The NDK clang driver's prefix; the API level is appended (`aarch64-linux-android26-clang`).
    fn clang_prefix(self) -> &'static str {
        match self {
            Self::Arm64 => "aarch64-linux-android",
            Self::Armv7 => "armv7a-linux-androideabi",
            Self::X86_64 => "x86_64-linux-android",
            Self::X86 => "i686-linux-android",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Library {
    pub abi: &'static str,
    pub path: PathBuf,
    /// JNI entry points checked against the library's dynamic symbols.
    pub entry_points: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub ndk: PathBuf,
    pub api: u32,
    pub libraries: Vec<Library>,
}

pub struct Options {
    /// ABIs to build; `DEFAULT_ABIS` when empty.
    pub abis: Vec<String>,
    pub api: u32,
    /// NDK root; found from the environment and the SDK when `None`.
    pub ndk: Option<PathBuf>,
    /// Where to install the libraries; `<crate>/jniLibs` when `None`.
    pub out: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            abis: Vec::new(),
            api: DEFAULT_API,
            ndk: None,
            out: None,
        }
    }
}

/// Builds the JNI library for each ABI and installs it under `<out>/<abi>/`.
pub fn build(crate_dir: &Path, manifest: &Manifest, opts: &Options) -> Result<Report> {
    if !matches!(manifest.language, Lang::Java | Lang::Kotlin) {
        bail!(
            "an Android build needs a JNI crate (Java or Kotlin), not {}",
            manifest.language.name()
        );
    }
    if manifest.kind != Kind::Ffi {
        bail!(
            "an Android build needs an `ffi` crate, not {:?}",
            manifest.kind
        );
    }
    check_api(opts.api)?;
    let ndk = ndk_root(opts.ndk.as_deref())?;
    let bin = ndk
        .join("toolchains/llvm/prebuilt")
        .join(host_tag())
        .join("bin");
    if !bin.is_dir() {
        bail!("no NDK toolchain in {}", bin.display());
    }
    let mut abis = Vec::new();
    if opts.abis.is_empty() {
        abis.extend_from_slice(DEFAULT_ABIS);
    } else {
        for name in &opts.abis {
            let abi = Abi::parse(name)?;
            if !abis.contains(&abi) {
                abis.push(abi);
            }
        }
    }
    check_targets(crate_dir, &abis)?;
    let out = opts
        .out
        .clone()
        .unwrap_or_else(|| crate_dir.join("jniLibs"));
    let lib = manifest.lib_name();
    let target_dir = crate::cache::target_for(crate_dir)?.dir;
    let mut libraries = Vec::new();
    for abi in abis {
        let linker = bin.join(format!("{}{}-clang", abi.clang_prefix(), opts.api));
        if !linker.is_file() {
            bail!(
                "no NDK linker for API {} at {}; this NDK does not ship that API level, lower `--api` (minimum {})",
                opts.api,
                linker.display(),
                MIN_API
            );
        }
        let mut cmd = Command::new("cargo");
        cmd.args([
            "build",
            "--release",
            "--target",
            abi.triple(),
            "--manifest-path",
        ])
        .arg(crate_dir.join("Cargo.toml"))
        // Cargo config and rust-toolchain.toml belong to the crate, not to whoever called us.
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env(linker_var(abi.triple()), &linker)
        // `cc`-based build scripts (none today, but crates bring them) read these.
        .env(format!("CC_{}", abi.triple().replace('-', "_")), &linker)
        .env(
            format!("AR_{}", abi.triple().replace('-', "_")),
            bin.join("llvm-ar"),
        )
        // Android requires 16 KB ELF alignment on 64-bit devices (Play enforces it for new
        // apps since 2025). NDK r28+ does this by default; the flags are harmless on newer NDKs.
        .env(
            rustflags_var(abi.triple()),
            "-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384",
        );
        run(&mut cmd, &format!("cargo build --target {}", abi.triple()))?;
        let built = target_dir
            .join(abi.triple())
            .join("release")
            .join(format!("lib{lib}.so"));
        if !built.is_file() {
            bail!("no library at {}", built.display());
        }
        let entry_points = check_symbols(crate_dir, abi, &bin.join("llvm-nm"), &built)?;
        check_page_size(abi, &bin.join("llvm-readelf"), &built)?;
        let dest = out.join(abi.name()).join(format!("lib{lib}.so"));
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::harness::install(&built, &dest)?;
        libraries.push(Library {
            abi: abi.name(),
            path: dest,
            entry_points,
        });
    }
    Ok(Report {
        ndk,
        api: opts.api,
        libraries,
    })
}

/// The generated Java runtime needs API 26+ even though the NDK itself can link for API 21.
pub fn check_api(api: u32) -> Result<()> {
    if api < MIN_API {
        bail!(
            "--api {api} is too low: the generated Java runtime needs API {MIN_API}+ \
             (java.util.Optional, java.util.function, ThreadLocal.withInitial)"
        );
    }
    Ok(())
}

/// The NDK root: `--ndk`, then `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`, then the newest complete
/// NDK under `ANDROID_HOME`/`ANDROID_SDK_ROOT`, then the usual SDK folders.
pub fn ndk_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return valid_ndk(path);
    }
    for var in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"] {
        if let Some(path) = std::env::var_os(var) {
            return valid_ndk(Path::new(&path));
        }
    }
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(sdk) = std::env::var_os(var)
            && let Some(ndk) = newest_ndk(Path::new(&sdk).join("ndk"))
        {
            return Ok(ndk);
        }
    }
    for sdk in sdk_folders() {
        if let Some(ndk) = newest_ndk(sdk.join("ndk")) {
            return Ok(ndk);
        }
    }
    bail!(
        "Android NDK not found: set ANDROID_NDK_HOME, pass `--ndk <path>`, or install one with \
         `sdkmanager \"ndk;28.0.13004108\"`"
    )
}

/// A usable NDK needs its clang toolchain for this host, not just a `source.properties` file:
/// an interrupted `sdkmanager` download often leaves an empty newer directory behind.
fn valid_ndk(path: &Path) -> Result<PathBuf> {
    if path
        .join("toolchains/llvm/prebuilt")
        .join(host_tag())
        .join("bin")
        .is_dir()
    {
        Ok(path.to_path_buf())
    } else {
        bail!(
            "{} is not a usable Android NDK (no toolchains/llvm/prebuilt/{}/bin)",
            path.display(),
            host_tag()
        )
    }
}

fn newest_ndk(dir: PathBuf) -> Option<PathBuf> {
    let mut found: Vec<(Vec<u32>, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("source.properties").is_file() && valid_ndk(p).is_ok())
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
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.pop().map(|(_, p)| p)
}

pub fn sdk_folders() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join("Android/Sdk"));
        out.push(home.join("Library/Android/sdk"));
        out.push(home.join("Documents/Android/Sdk"));
    }
    if cfg!(windows)
        && let Some(local) = std::env::var_os("LOCALAPPDATA")
    {
        out.push(PathBuf::from(local).join("Android/Sdk"));
    }
    out.push(PathBuf::from("/usr/lib/android-sdk"));
    out.push(PathBuf::from("/opt/android-sdk"));
    out
}

/// NDK prebuilts are x86_64 even on Linux machines with an arm64 CPU; say so when they are absent.
fn host_tag() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else if cfg!(windows) {
        "windows-x86_64"
    } else {
        // NDK prebuilts are x86_64 even on Linux hosts with an arm64 CPU.
        "linux-x86_64"
    }
}

fn linker_var(triple: &str) -> String {
    format!(
        "CARGO_TARGET_{}_LINKER",
        triple.replace('-', "_").to_uppercase()
    )
}

fn rustflags_var(triple: &str) -> String {
    format!(
        "CARGO_TARGET_{}_RUSTFLAGS",
        triple.replace('-', "_").to_uppercase()
    )
}

/// Fails before cargo does, listing every target to install at once.
fn check_targets(crate_dir: &Path, abis: &[Abi]) -> Result<()> {
    let Some(installed) = installed_targets(crate_dir) else {
        return Ok(());
    };
    let missing: Vec<&str> = abis
        .iter()
        .map(|a| a.triple())
        .filter(|t| !installed.iter().any(|i| i == t))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    bail!(
        "Rust targets for Android are not installed: {}",
        format_args!("rustup target add {}", missing.join(" "))
    )
}

fn installed_targets(crate_dir: &Path) -> Option<Vec<String>> {
    let out = crate::process::output(
        Command::new("rustup")
            .args(["target", "list", "--installed"])
            .current_dir(crate_dir),
        crate::process::QUICK,
    )
    .ok()?;
    out.success().then(|| {
        out.stdout_text()
            .lines()
            .map(str::trim)
            .map(String::from)
            .collect()
    })
}

/// The JNI entry points `src/lib.rs` declares.
fn expected_symbols(crate_dir: &Path) -> Result<Vec<String>> {
    let lib = std::fs::read_to_string(crate_dir.join("src/lib.rs"))
        .with_context(|| format!("reading {}/src/lib.rs", crate_dir.display()))?;
    let symbols: Vec<String> = lib
        .lines()
        .filter_map(|line| {
            let at = line.find("extern \"system\" fn Java_")?;
            let name: String = line[at + "extern \"system\" fn ".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect();
    if symbols.is_empty() {
        bail!(
            "{} declares no JNI entry points; is this a Java or Kotlin crate?",
            crate_dir.join("src/lib.rs").display()
        );
    }
    Ok(symbols)
}

/// Runs a build tool with the same timeout and output cap as the rest of the tool.
fn run(cmd: &mut Command, what: &str) -> Result<String> {
    let out = crate::process::output(cmd, crate::process::BUILD)
        .with_context(|| format!("running {what}"))?;
    if out.success() {
        return Ok(out.stdout_text());
    }
    let stderr = out.stderr_text();
    let tail: Vec<&str> = stderr.lines().rev().take(25).collect();
    bail!(
        "{what} failed ({}):\n{}",
        out.status,
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    )
}

/// Checks that the built library exports every declared JNI entry point, which catches a library
/// that links but would fail `System.loadLibrary` with `UnsatisfiedLinkError`.
fn check_symbols(crate_dir: &Path, abi: Abi, nm: &Path, library: &Path) -> Result<usize> {
    let expected = expected_symbols(crate_dir)?;
    let out = crate::process::output(
        Command::new(nm).args(["-D", "--defined-only"]).arg(library),
        crate::process::QUICK,
    )
    .with_context(|| format!("running {}", nm.display()))?;
    if !out.success() {
        bail!("{} failed on {}", nm.display(), library.display());
    }
    let have = out.stdout_text();
    let missing: Vec<&String> = expected
        .iter()
        .filter(|s| {
            !have
                .lines()
                .any(|l| l.split_whitespace().any(|w| w == s.as_str()))
        })
        .collect();
    if !missing.is_empty() {
        bail!(
            "the {} library {} does not export {}",
            abi.name(),
            library.display(),
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(expected.len())
}

/// Every `LOAD` segment of a 64-bit Android library must be 16 KB aligned.
fn check_page_size(abi: Abi, readelf: &Path, library: &Path) -> Result<()> {
    if !abi.is_64_bit() {
        return Ok(());
    }
    let text = run(
        Command::new(readelf).arg("-l").arg(library),
        &format!("llvm-readelf -l (ABI {})", abi.name()),
    )?;
    check_alignment(&text).with_context(|| format!("checking {}", library.display()))
}

fn check_alignment(readelf: &str) -> Result<()> {
    let segments: Vec<&str> = readelf
        .lines()
        .filter(|l| l.split_whitespace().next() == Some("LOAD"))
        .collect();
    if segments.is_empty() {
        bail!("llvm-readelf listed no LOAD segments");
    }
    let misaligned: Vec<&str> = segments
        .iter()
        .copied()
        .filter(|line| {
            line.split_whitespace()
                .last()
                .and_then(|a| u64::from_str_radix(a.trim_start_matches("0x"), 16).ok())
                .is_none_or(|align| align < 0x4000)
        })
        .collect();
    if !misaligned.is_empty() {
        bail!(
            "{} LOAD segment(s) are not 16 KB aligned: {}",
            misaligned.len(),
            misaligned[0]
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gradle_abi_names() {
        assert_eq!(Abi::parse("arm64-v8a").unwrap(), Abi::Arm64);
        assert_eq!(Abi::parse("armeabi-v7a").unwrap(), Abi::Armv7);
        assert_eq!(Abi::parse("x86_64").unwrap(), Abi::X86_64);
        assert!(Abi::parse("mips").is_err());
    }

    #[test]
    fn clang_driver_names_match_the_ndk() {
        assert_eq!(
            format!("{}{}-clang", Abi::Arm64.clang_prefix(), 21),
            "aarch64-linux-android21-clang"
        );
        assert_eq!(
            format!("{}{}-clang", Abi::Armv7.clang_prefix(), 21),
            "armv7a-linux-androideabi21-clang"
        );
    }

    #[test]
    fn rejects_too_low_an_api() {
        assert!(check_api(DEFAULT_API).is_ok());
        assert!(check_api(21).unwrap_err().to_string().contains("API 26"));
    }

    #[test]
    fn the_newest_complete_ndk_wins() {
        let dir = std::env::temp_dir().join(format!("ra-ndk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for version in ["25.1.8937393", "30.0.16248370", "99.0.0"] {
            let root = dir.join(version);
            std::fs::create_dir_all(root.join("source.properties").parent().unwrap()).unwrap();
            std::fs::write(root.join("source.properties"), "").unwrap();
            // 99.0.0 is an interrupted download: no clang toolchain.
            if version != "99.0.0" {
                std::fs::create_dir_all(
                    root.join("toolchains/llvm/prebuilt")
                        .join(host_tag())
                        .join("bin"),
                )
                .unwrap();
            }
        }
        let found = newest_ndk(dir.clone()).unwrap();
        assert_eq!(found.file_name().unwrap(), "30.0.16248370");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_16kb_check_reads_elf_load_alignment() {
        let aligned = "  LOAD 0x000000 0x00000000 0x00000000 0x1000 0x1000 R E 0x4000\n\
                       LOAD 0x001000 0x00001000 0x00001000 0x0200 0x0200 R W 0x4000\n";
        assert!(check_alignment(aligned).is_ok());
        let bad = aligned.replace("0x4000\n", "0x1000\n");
        assert!(
            check_alignment(&bad)
                .unwrap_err()
                .to_string()
                .contains("16 KB")
        );
        assert!(check_alignment("no segments here").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn the_symbol_check_ignores_comments_and_reports_missing_entries() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = std::env::temp_dir().join(format!("ra-symbols-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "// fn Java_pkg_Fake_not_real()\n\
             pub extern \"system\" fn Java_pkg_Fake_call() {}\n",
        )
        .unwrap();
        let script = |name: &str, body: &str| {
            let text = dir.join(format!("{name}.txt"));
            std::fs::write(&text, body).unwrap();
            let path = dir.join(name);
            assert!(
                Command::new("cp")
                    .arg(&text)
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            );
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        let nm = script("nm", "#!/bin/sh\nexit 0\n");
        let err = check_symbols(&dir, Abi::Arm64, &nm, Path::new("libfake.so"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Java_pkg_Fake_call"), "{err}");
        assert!(!err.contains("not_real"), "{err}");
        assert!(err.contains("arm64-v8a"), "{err}");
        let nm = script("nm-found", "#!/bin/sh\necho '0000 T Java_pkg_Fake_call'\n");
        assert_eq!(
            check_symbols(&dir, Abi::Arm64, &nm, Path::new("libfake.so")).unwrap(),
            1
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
