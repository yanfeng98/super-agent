//! Builds the migration plan: units, their dependency graph, port order and readiness.
//!
//! A *unit* is the smallest thing we suggest porting at once: a folder of source files,
//! or a single file that sits directly in a package's source root.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::lang::Lang;
use crate::risks::{Risk, Severity};
use crate::ts::deps::{self, Fit};
use crate::ts::discover::{self, FileKind, Package, Workspace};
use crate::ts::scan::{self, FileFacts, Platform};

/// Version of the JSON output. Bump on any breaking change to its shape.
pub const SCHEMA: &str = "rust-anything/analyze@1";
const SAMPLES_PER_RISK: usize = 3;
/// Units smaller than this are not suggested as a first port.
const MIN_CANDIDATE_LOC: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Good to port now.
    Ready,
    /// Portable, with risks to handle first.
    Caution,
    /// Depends on something with no Rust equivalent, or on dynamic code.
    Blocked,
    /// UI or editor code: stays in JS/TS.
    Keep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bridge {
    NapiRs,
    WasmBindgen,
    /// Keep in the source language (named for JS, the first language; applies to all).
    KeepInJs,
    Pyo3,
    /// A C ABI: drop-in `extern "C"` functions for C.
    CAbi,
    /// Go through cgo over a C ABI.
    Cgo,
    /// Ruby through Fiddle over a C ABI.
    Fiddle,
    Jni,
    /// PHP through its FFI extension over a C ABI.
    PhpFfi,
    DotnetFfi,
}

impl Bridge {
    pub fn label(self) -> &'static str {
        match self {
            Self::NapiRs => "napi-rs",
            Self::WasmBindgen => "wasm-bindgen",
            Self::KeepInJs => "keep",
            Self::Pyo3 => "PyO3",
            Self::CAbi => "C ABI",
            Self::Cgo => "cgo",
            Self::Fiddle => "Fiddle (C ABI)",
            Self::Jni => "JNI",
            Self::PhpFfi => "PHP FFI (C ABI)",
            Self::DotnetFfi => "P/Invoke (C ABI)",
        }
    }

    /// The bridge a language uses for code that can move.
    pub fn for_lang(lang: Lang) -> Self {
        match lang {
            Lang::Ts => Self::NapiRs,
            Lang::Python => Self::Pyo3,
            Lang::C => Self::CAbi,
            Lang::Go => Self::Cgo,
            Lang::Ruby => Self::Fiddle,
            Lang::Java | Lang::Kotlin => Self::Jni,
            Lang::Php => Self::PhpFfi,
            Lang::Csharp => Self::DotnetFfi,
        }
    }
}

/// The Rust counterpart of a dependency, from the language's table.
pub fn crate_for(lang: Lang, name: &str) -> Option<deps::CrateMatch> {
    match lang {
        Lang::Ts => deps::lookup(name),
        Lang::Python => crate::lang::python::lookup(name),
        _ => crate::lang::lookup(lang, name),
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub language: Lang,
    pub summary: Summary,
    pub packages: Vec<PackageReport>,
    pub units: Vec<UnitReport>,
    /// Unit ids suggested as the first ports, best first.
    pub start_here: Vec<String>,
    pub dependencies: Vec<DependencyReport>,
    /// `analyze --whole-app`: entry points, every dependency, pieces and decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whole_app: Option<crate::whole_app::WholeApp>,
    /// Problems with the input itself, such as every source file being excluded by ignore rules.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub packages: usize,
    pub source_files: usize,
    pub test_files: usize,
    pub ui_files: usize,
    pub loc: usize,
    pub units: usize,
    pub ready: usize,
    pub caution: usize,
    pub blocked: usize,
    pub keep: usize,
    pub parse_errors: usize,
}

#[derive(Debug, Serialize)]
pub struct PackageReport {
    pub name: String,
    pub dir: String,
    /// Position in dependency order: packages come after the workspace packages they use.
    pub order: usize,
    /// `node` when any of its code uses Node APIs, else `portable`.
    pub runtime: &'static str,
    pub workspace_deps: Vec<String>,
    pub test_files: usize,
    pub units: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnitReport {
    pub id: String,
    pub package: String,
    pub files: Vec<String>,
    pub loc: usize,
    /// Port order inside the repo: a unit's dependencies are in earlier waves.
    pub wave: usize,
    /// 0-100: how safe and easy a port looks from the code alone.
    pub readiness: u32,
    pub status: Status,
    pub bridge: Bridge,
    pub platforms: Vec<Platform>,
    pub risks: Vec<RiskReport>,
    pub internal_deps: Vec<String>,
    pub workspace_deps: Vec<String>,
    pub external_deps: Vec<String>,
    /// Test files that import this unit directly.
    pub direct_tests: usize,
    /// Units that import this one.
    pub fan_in: usize,
    /// Loops and string-offset operations per 100 lines: a rough hint of compute-heavy code.
    pub density: f64,
    /// 0-100: readiness balanced with value (compute-heavy, tested, big enough to matter).
    /// `start_here` is ordered by it.
    pub priority: u32,
    /// Exported functions and classes, and how many of them can cross a bridge unchanged.
    pub exports: usize,
    pub bridgeable: usize,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskReport {
    pub risk: Risk,
    pub severity: Severity,
    pub count: usize,
    /// `path:line` of the first occurrences.
    pub samples: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DependencyReport {
    pub name: String,
    pub used_by: usize,
    pub crates: Option<&'static str>,
    /// `good`, `partial`, `none`, or `unknown` when the table has no entry.
    pub fit: &'static str,
    pub note: &'static str,
}

/// What an import specifier points at.
enum Target {
    File(usize),
    Workspace(String),
    External(String),
    Platform,
    Unresolved,
}

#[derive(Default)]
struct UnitBuild {
    id: String,
    package: usize,
    files: Vec<usize>,
    ui: bool,
    loc: usize,
    loops: usize,
    risks: BTreeMap<Risk, (usize, Vec<String>)>,
    platforms: BTreeSet<Platform>,
    deps: BTreeSet<usize>,
    workspace_deps: BTreeSet<String>,
    external_deps: BTreeSet<String>,
    unresolved: usize,
    parse_errors: usize,
    direct_tests: usize,
    fan_in: usize,
}

pub fn analyze(root: &Path) -> Result<Report> {
    analyze_lang(root, Lang::detect(root))
}

/// Compiled or packaged files: what someone may point `analyze` at instead of source.
const COMPILED: [&str; 17] = [
    "apk", "aab", "ipa", "xapk", "jar", "war", "aar", "class", "dex", "exe", "dll", "so", "dylib",
    "wasm", "pyc", "msi", "appimage",
];

fn compiled_hint() -> &'static str {
    "rust-anything ports source code: it parses functions, records what they do, and checks the \
     Rust port against the running original. A compiled app (an APK, a JAR, an .exe) has no \
     source to port and no way to record it function by function. Point it at the app's source \
     folder instead; for an app you only have as a binary, writing a new Rust app from its \
     visible behavior is a separate project"
}

pub fn analyze_lang(root: &Path, lang: Lang) -> Result<Report> {
    if root.is_file() {
        let ext = root
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if COMPILED.contains(&ext.as_str()) {
            bail!(
                "{} is a compiled .{ext} file. {}",
                root.display(),
                compiled_hint()
            );
        }
        bail!(
            "{} is a file; `analyze` takes the folder of a project's source code",
            root.display()
        );
    }
    let ws = match lang {
        Lang::Ts => discover::discover(root)?,
        _ => crate::lang::discover(lang, root)?,
    };
    let facts: Vec<Option<FileFacts>> = ws
        .files
        .iter()
        .map(|file| {
            let text = std::fs::read_to_string(root.join(&file.path)).ok()?;
            if lang == Lang::C {
                let mut facts = crate::lang::scan(lang, &file.path, &text);
                if let Some(header) = own_header(root, &file.path) {
                    facts
                        .imports
                        .extend(crate::lang::scan(lang, &header.0, &header.1).imports);
                }
                return Some(facts);
            }
            if lang != Lang::Ts {
                return Some(crate::lang::scan(lang, &file.path, &text));
            }
            let code = crate::ts::script_of(&file.path, &text);
            let is_sfc = [".vue", ".svelte", ".astro"]
                .iter()
                .any(|e| file.path.ends_with(e));
            let parse_as = if is_sfc { "sfc.ts" } else { file.path.as_str() };
            Some(scan::scan(parse_as, &code))
        })
        .collect();
    let mut report = build(root, &ws, &facts, lang);
    check_bridgeable(root, &mut report, lang);
    Ok(report)
}

fn own_header(root: &Path, unit: &str) -> Option<(String, String)> {
    let stem = unit.strip_suffix(".c")?;
    let name = stem.rsplit('/').next().unwrap_or(stem);
    [format!("{stem}.h"), format!("include/{name}.h")]
        .into_iter()
        .find_map(|h| Some((h.clone(), std::fs::read_to_string(root.join(&h)).ok()?)))
}

/// Counts each unit's exports that can cross a bridge, with the rules `bridge` applies. A unit
/// whose exports are all JS-shaped (plugin objects, callbacks) cannot be ported without an API
/// change: it is marked `caution` and left out of `start_here`.
fn check_bridgeable(root: &Path, report: &mut Report, lang: Lang) {
    use crate::ts::exports::exported_classes;
    if lang == Lang::C {
        crate::lang::c::use_typedefs(root);
    }
    for unit in &mut report.units {
        if matches!(unit.status, Status::Keep | Status::Blocked) {
            continue;
        }
        let (mut exports, mut bridgeable) = (0, 0);
        for file in &unit.files {
            let Ok(text) = std::fs::read_to_string(root.join(file)) else {
                continue;
            };
            let types = crate::bridge::types_in_scope(root, file, &text);
            if lang != Lang::Ts {
                let (functions, classes) = crate::lang::exports(lang, file, &text);
                exports += functions.len() + classes.len();
                bridgeable += functions
                    .iter()
                    .filter(|f| crate::bridge::js_only_reason(f, &[]).is_none())
                    .count();
                bridgeable += classes
                    .iter()
                    .filter(|c| crate::bridge::class_reason(c, &[], false).is_none())
                    .count();
                continue;
            }
            for f in crate::bridge::ts_functions(root, file, &text, &types) {
                exports += 1;
                bridgeable += usize::from(crate::bridge::js_only_reason(&f, &types).is_none());
            }
            for class in exported_classes(file, &text) {
                exports += 1;
                bridgeable +=
                    usize::from(crate::bridge::class_reason(&class, &types, true).is_none());
            }
        }
        unit.exports = exports;
        unit.bridgeable = bridgeable;
        if exports > 0 && bridgeable == 0 {
            unit.status = Status::Caution;
            unit.notes.insert(
                0,
                if lang == Lang::C {
                    "No export can cross the C ABI bridge (contexts, function pointers, structs by \
                     value, pointers it cannot pair with a length): bridge a wrapper with plain \
                     arguments, an API change."
                } else {
                    "Every export is JS-shaped (functions, plugin objects, iterators): porting the \
                     logic means adding a data-only function, an API change."
                }
                .into(),
            );
        }
    }
    let summary = &mut report.summary;
    (
        summary.ready,
        summary.caution,
        summary.blocked,
        summary.keep,
    ) = (0, 0, 0, 0);
    for u in &report.units {
        match u.status {
            Status::Ready => summary.ready += 1,
            Status::Caution => summary.caution += 1,
            Status::Blocked => summary.blocked += 1,
            Status::Keep => summary.keep += 1,
        }
    }
    report.start_here = start_here(&report.units);
}

fn build(root: &Path, ws: &Workspace, facts: &[Option<FileFacts>], lang: Lang) -> Report {
    let file_index: HashMap<&str, usize> = ws
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| (f.path.as_str(), i))
        .collect();
    let workspace_names: HashSet<&str> = ws.packages.iter().map(|p| p.name.as_str()).collect();
    let alias_cache: std::cell::RefCell<HashMap<String, crate::ts::tsconfig::PathAliases>> =
        std::cell::RefCell::new(HashMap::new());
    let classify = |from: &str, spec: &str| -> Target {
        if lang != Lang::Ts {
            return match crate::lang::resolve(lang, from, spec, &file_index, ws) {
                crate::lang::Resolution::File(i) => Target::File(i),
                crate::lang::Resolution::External(name) => Target::External(name),
                crate::lang::Resolution::Stdlib => Target::Platform,
                crate::lang::Resolution::Unresolved => Target::Unresolved,
            };
        }
        if spec.starts_with("./") || spec.starts_with("../") || spec == "." || spec == ".." {
            return resolve_relative(from, spec, &file_index)
                .map_or(Target::Unresolved, Target::File);
        }
        if scan::platform_of(spec).is_some() {
            return Target::Platform;
        }
        // tsconfig `paths` (`~/utils`, `@/lib`), from the tsconfig nearest the importing file.
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        let aliases = alias_cache
            .borrow_mut()
            .entry(dir.to_string())
            .or_insert_with(|| crate::ts::tsconfig::PathAliases::nearest(root, dir))
            .clone();
        let candidates = aliases.candidates(spec);
        if !candidates.is_empty() {
            return candidates
                .iter()
                .find_map(|base| resolve_base(base, &file_index))
                .map_or(Target::Unresolved, Target::File);
        }
        if spec.starts_with('#')
            || spec.starts_with('~')
            || spec.starts_with("@/")
            || spec.contains(':')
        {
            return Target::Unresolved;
        }
        let name = deps::package_name(spec);
        if workspace_names.contains(name) {
            Target::Workspace(name.to_string())
        } else {
            Target::External(name.to_string())
        }
    };

    // Group source and UI files into units.
    let mut units: Vec<UnitBuild> = Vec::new();
    let mut unit_ids: HashMap<String, usize> = HashMap::new();
    let mut unit_of: Vec<Option<usize>> = vec![None; ws.files.len()];
    for (i, file) in ws.files.iter().enumerate() {
        if file.kind == FileKind::Test {
            continue;
        }
        let id = unit_id(&ws.packages[file.package], &file.path);
        let u = *unit_ids.entry(id.clone()).or_insert_with(|| {
            units.push(UnitBuild {
                id,
                package: file.package,
                ..UnitBuild::default()
            });
            units.len() - 1
        });
        units[u].files.push(i);
        units[u].ui |= file.kind == FileKind::Ui;
        unit_of[i] = Some(u);
    }

    // Aggregate facts and imports per unit; credit tests to what they import.
    let mut package_tests = vec![0usize; ws.packages.len()];
    let mut summary = Summary {
        packages: ws.packages.len(),
        ..Summary::default()
    };
    for (i, file) in ws.files.iter().enumerate() {
        match file.kind {
            FileKind::Source => summary.source_files += 1,
            FileKind::Test => summary.test_files += 1,
            FileKind::Ui => summary.ui_files += 1,
        }
        let Some(f) = &facts[i] else { continue };
        summary.parse_errors += usize::from(f.parse_errors > 0);
        if file.kind == FileKind::Test {
            package_tests[file.package] += 1;
            let mut hit_units = BTreeSet::new();
            let mut hit_packages = BTreeSet::new();
            for import in &f.imports {
                match classify(&file.path, &import.specifier) {
                    Target::File(j) => hit_units.extend(unit_of[j]),
                    Target::Workspace(name) => {
                        hit_packages.extend(ws.packages.iter().position(|p| p.name == name));
                    }
                    _ => {}
                }
            }
            for u in hit_units {
                units[u].direct_tests += 1;
            }
            for p in hit_packages.into_iter().filter(|&p| p != file.package) {
                package_tests[p] += 1;
            }
            continue;
        }
        let Some(u) = unit_of[i] else { continue };
        summary.loc += f.loc;
        let unit = &mut units[u];
        unit.loc += f.loc;
        unit.loops += f.loops;
        unit.parse_errors += f.parse_errors;
        unit.platforms.extend(f.platforms.iter().copied());
        if lang == Lang::C && crate::lang::c::in_arch_dir(&file.path) {
            let entry = unit.risks.entry(Risk::PlatformSpecific).or_default();
            entry.0 += 1;
            if entry.1.len() < SAMPLES_PER_RISK {
                entry.1.push(format!("{}:1", file.path));
            }
        }
        for (risk, lines) in &f.risks {
            let entry = unit.risks.entry(*risk).or_default();
            entry.0 += lines.len();
            for line in lines {
                if entry.1.len() < SAMPLES_PER_RISK {
                    entry.1.push(format!("{}:{line}", file.path));
                }
            }
        }
        for import in &f.imports {
            match classify(&file.path, &import.specifier) {
                Target::File(j) => {
                    if let Some(v) = unit_of[j].filter(|&v| v != u) {
                        units[u].deps.insert(v);
                    }
                }
                Target::Workspace(name) => {
                    if name != ws.packages[units[u].package].name {
                        units[u].workspace_deps.insert(name);
                    }
                }
                Target::External(name) if !import.type_only => {
                    units[u].external_deps.insert(name);
                }
                Target::Unresolved if !import.type_only => units[u].unresolved += 1,
                Target::Unresolved => {}
                Target::External(_) | Target::Platform => {}
            }
        }
    }
    for u in 0..units.len() {
        for v in units[u].deps.clone() {
            units[v].fan_in += 1;
        }
    }

    let edges: Vec<Vec<usize>> = units
        .iter()
        .map(|u| u.deps.iter().copied().collect())
        .collect();
    let (waves, cycles) = waves(&edges);
    // A unit needs Node when it, or anything it imports, uses Node APIs.
    let node_reach = reaches(
        &edges,
        &units
            .iter()
            .map(|u| u.platforms.contains(&Platform::Node))
            .collect::<Vec<_>>(),
    );
    // A package is Node-only when its source root needs Node; build scripts beside it do not count.
    let node_packages: BTreeSet<usize> = units
        .iter()
        .enumerate()
        .filter(|(u, unit)| {
            let src = &ws.packages[unit.package].src_root;
            node_reach[*u] && (src.is_empty() || unit.id.starts_with(&format!("{src}/")))
        })
        .map(|(_, unit)| unit.package)
        .collect();
    let ctx = Ctx {
        lang,
        ws,
        units: &units,
        package_tests: &package_tests,
        node_packages: &node_packages,
        node_reach: &node_reach,
        waves: &waves,
        cycles: &cycles,
    };
    let mut reports: Vec<UnitReport> = (0..units.len()).map(|u| score(u, &ctx)).collect();
    reports.sort_by(|a, b| a.id.cmp(&b.id));
    for r in &reports {
        match r.status {
            Status::Ready => summary.ready += 1,
            Status::Caution => summary.caution += 1,
            Status::Blocked => summary.blocked += 1,
            Status::Keep => summary.keep += 1,
        }
    }
    summary.units = reports.len();

    let start_here = start_here(&reports);
    let packages = package_reports(ws, &reports, &package_tests, &node_packages);
    let dependencies = dependency_reports(&reports, lang);
    let mut warnings = Vec::new();
    if ws.files.is_empty() {
        let hidden = crate::walk::ignored_files(root, lang.extensions());
        let compiled = crate::walk::builder(root)
            .standard_filters(false)
            .max_depth(Some(4))
            .build()
            .flatten()
            .filter(|e| {
                e.path().extension().is_some_and(|x| {
                    COMPILED.contains(&x.to_string_lossy().to_lowercase().as_str())
                })
            })
            .count();
        if hidden == 0 && compiled > 0 {
            warnings.push(format!(
                "no source files were found, only {compiled} compiled or packaged files. {}",
                compiled_hint()
            ));
        }
        if hidden > 0 {
            warnings.push(format!(
                "no source files were analyzed, but {hidden} {} files under {} are excluded by \
                 ignore rules (.gitignore or .ignore files in this repository); the plan is empty \
                 because of them, not because there is nothing to port",
                lang.name(),
                root.display()
            ));
        }
    }
    Report {
        schema: SCHEMA,
        language: lang,
        summary,
        packages,
        units: reports,
        start_here,
        dependencies,
        whole_app: None,
        warnings,
    }
}

struct Ctx<'a> {
    lang: Lang,
    ws: &'a Workspace,
    units: &'a [UnitBuild],
    package_tests: &'a [usize],
    node_packages: &'a BTreeSet<usize>,
    node_reach: &'a [bool],
    waves: &'a [usize],
    cycles: &'a [Vec<usize>],
}

fn score(u: usize, ctx: &Ctx) -> UnitReport {
    let unit = &ctx.units[u];
    let all = ctx.units;
    let ws = ctx.ws;
    let packages = &ws.packages;
    let package_tests = ctx.package_tests[unit.package];
    let wave = ctx.waves[u];
    let cycle = &ctx.cycles[u];
    let mut notes = Vec::new();
    let keep = unit.ui
        || unit.platforms.contains(&Platform::Ui)
        || unit.platforms.contains(&Platform::Editor);
    let blocking_deps: Vec<&String> = unit
        .external_deps
        .iter()
        .filter(|d| crate_for(ctx.lang, d).is_some_and(|m| m.fit == Fit::None))
        .collect();
    let unknown_deps = unit
        .external_deps
        .iter()
        .filter(|d| crate_for(ctx.lang, d).is_none())
        .count();

    let risk_points: f64 = unit
        .risks
        .iter()
        .map(|(risk, (count, _))| risk.severity().weight() * (1.0 + *count as f64).ln())
        .sum();
    let size_penalty = unit.loc as f64 / 40.0;
    let fan_in_penalty = (unit.fan_in as f64 * 1.5).min(15.0);
    let dep_penalty = blocking_deps.len() as f64 * 12.0 + unknown_deps as f64 * 4.0;
    let test_bonus = if unit.direct_tests > 0 {
        10.0
    } else if package_tests > 0 {
        4.0
    } else {
        0.0
    };
    let readiness = (100.0 - risk_points - size_penalty - fan_in_penalty - dep_penalty
        + test_bonus)
        .clamp(0.0, 100.0);

    let platform = unit.risks.contains_key(&Risk::PlatformSpecific);
    let status = if platform {
        notes.push(
            "Depends on the target (hardware registers, memory layout, inline assembly): keep in \
             C, since a recording on this machine cannot vouch for the target."
                .to_string(),
        );
        Status::Keep
    } else if keep {
        notes.push(if unit.platforms.contains(&Platform::Editor) {
            "Runs in the editor's extension host: keep in TS.".to_string()
        } else {
            "UI code: keep in the original language.".to_string()
        });
        Status::Keep
    } else if !blocking_deps.is_empty() || unit.risks.contains_key(&Risk::DynamicCode) {
        for dep in &blocking_deps {
            notes.push(format!("`{dep}` has no Rust equivalent."));
        }
        if unit.risks.contains_key(&Risk::DynamicCode) {
            notes.push("Uses dynamic code (eval, new Function or computed imports).".into());
        }
        Status::Blocked
    } else if readiness < 50.0 || unit.parse_errors > 0 {
        Status::Caution
    } else {
        Status::Ready
    };
    if unit.parse_errors > 0 {
        notes.push(format!(
            "{} parse errors: results for this unit are partial.",
            unit.parse_errors
        ));
    }
    if unknown_deps > 0 {
        notes.push(format!(
            "{unknown_deps} {} dependencies not in the crate table: check them by hand.",
            ctx.lang.registry()
        ));
    }
    if unit.unresolved > 0 {
        notes.push(format!(
            "{} imports use aliases or virtual modules and were not followed.",
            unit.unresolved
        ));
    }
    if cycle.len() > 1 {
        let others: Vec<&str> = cycle
            .iter()
            .map(|&v| all[v].id.as_str())
            .filter(|id| *id != unit.id)
            .collect();
        notes.push(format!(
            "Import cycle with {}: port them together.",
            others.join(", ")
        ));
    }
    if unit.deps.is_empty() && !keep {
        if unit.workspace_deps.is_empty() {
            notes.push("Leaf: imports no other unit.".into());
        } else {
            let names: Vec<String> = unit
                .workspace_deps
                .iter()
                .map(|d| format!("`{d}`"))
                .collect();
            notes.push(format!(
                "Imports {} from another workspace package: port {} first.",
                names.join(", "),
                if names.len() == 1 {
                    "that package"
                } else {
                    "those packages"
                }
            ));
        }
    }
    if unit.direct_tests > 0 {
        notes.push(if unit.direct_tests == 1 {
            "1 test file imports it directly.".to_string()
        } else {
            format!("{} test files import it directly.", unit.direct_tests)
        });
    } else if package_tests == 0 && !keep {
        notes.push(
            "No tests found: write characterization tests against the original code first.".into(),
        );
    }

    let string_ops = unit.risks.get(&Risk::StringOffsets).map_or(0, |r| r.0);
    let density = if unit.loc == 0 {
        0.0
    } else {
        ((unit.loops + string_ops) as f64 * 100.0 / unit.loc as f64 * 10.0).round() / 10.0
    };
    let bridge = if keep {
        Bridge::KeepInJs
    } else if ctx.lang != Lang::Ts {
        Bridge::for_lang(ctx.lang)
    } else if ctx.node_reach[u] || ctx.node_packages.contains(&unit.package) {
        Bridge::NapiRs
    } else {
        Bridge::WasmBindgen
    };
    if bridge == Bridge::WasmBindgen {
        notes.push(
            "Portable code: wasm-bindgen keeps it running in browsers and Node. It needs the \
             wasm32-unknown-unknown target and wasm-bindgen; `bridge --kind napi` runs with what \
             Node already has."
                .into(),
        );
    }

    let mut files: Vec<String> = unit
        .files
        .iter()
        .map(|&i| ws.files[i].path.clone())
        .collect();
    files.sort();
    let mut risks: Vec<RiskReport> = unit
        .risks
        .iter()
        .map(|(risk, (count, samples))| RiskReport {
            risk: *risk,
            severity: risk.severity(),
            count: *count,
            samples: samples.clone(),
        })
        .collect();
    risks.sort_by(|a, b| b.severity.cmp(&a.severity).then(b.count.cmp(&a.count)));

    UnitReport {
        id: unit.id.clone(),
        package: packages[unit.package].name.clone(),
        files,
        loc: unit.loc,
        wave,
        readiness: readiness.round() as u32,
        status,
        bridge,
        platforms: unit.platforms.iter().copied().collect(),
        risks,
        internal_deps: unit.deps.iter().map(|&v| all[v].id.clone()).collect(),
        workspace_deps: unit.workspace_deps.iter().cloned().collect(),
        external_deps: unit.external_deps.iter().cloned().collect(),
        direct_tests: unit.direct_tests,
        fan_in: unit.fan_in,
        density,
        priority: priority(
            readiness,
            density,
            unit.loc,
            unit.direct_tests,
            package_tests,
        ),
        exports: 0,
        bridgeable: 0,
        notes,
    }
}

fn start_here(units: &[UnitReport]) -> Vec<String> {
    let pick = |statuses: &[Status], min_loc: usize| -> Vec<&UnitReport> {
        let mut list: Vec<&UnitReport> = units
            .iter()
            .filter(|u| statuses.contains(&u.status) && u.loc >= min_loc && u.bridgeable > 0)
            .collect();
        list.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.readiness.cmp(&a.readiness))
                .then(a.wave.cmp(&b.wave))
                .then(b.density.total_cmp(&a.density))
                .then(a.id.cmp(&b.id))
        });
        list
    };
    let mut list = pick(&[Status::Ready], MIN_CANDIDATE_LOC);
    if list.len() < 3 {
        list = pick(&[Status::Ready, Status::Caution], MIN_CANDIDATE_LOC);
    }
    if list.is_empty() {
        // Small repos: every unit is under the size floor.
        list = pick(&[Status::Ready, Status::Caution], 1);
    }
    list.into_iter().take(10).map(|u| u.id.clone()).collect()
}

fn package_reports(
    ws: &Workspace,
    units: &[UnitReport],
    package_tests: &[usize],
    node_packages: &BTreeSet<usize>,
) -> Vec<PackageReport> {
    let names: HashMap<&str, usize> = ws
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();
    let deps_of: Vec<Vec<usize>> = ws
        .packages
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut d: Vec<usize> = p
                .deps
                .iter()
                .chain(&p.dev_deps)
                .filter_map(|n| names.get(n.as_str()).copied())
                .filter(|&j| j != i)
                .collect();
            d.sort_unstable();
            d.dedup();
            d
        })
        .collect();
    let order = topological(&deps_of);
    let mut reports: Vec<PackageReport> = order
        .iter()
        .map(|&i| {
            let p = &ws.packages[i];
            PackageReport {
                name: p.name.clone(),
                dir: p.dir.clone(),
                order: 0,
                runtime: if node_packages.contains(&i) {
                    "node"
                } else {
                    "portable"
                },
                workspace_deps: deps_of[i]
                    .iter()
                    .map(|&j| ws.packages[j].name.clone())
                    .collect(),
                test_files: package_tests[i],
                units: units
                    .iter()
                    .filter(|u| u.package == p.name)
                    .map(|u| u.id.clone())
                    .collect(),
            }
        })
        .filter(|p| !p.units.is_empty())
        .collect();
    for (position, report) in reports.iter_mut().enumerate() {
        report.order = position + 1;
    }
    reports
}

fn dependency_reports(units: &[UnitReport], lang: Lang) -> Vec<DependencyReport> {
    let mut used: BTreeMap<&str, usize> = BTreeMap::new();
    for unit in units.iter().filter(|u| u.status != Status::Keep) {
        for dep in &unit.external_deps {
            *used.entry(dep).or_default() += 1;
        }
    }
    let mut out: Vec<DependencyReport> = used
        .into_iter()
        .map(|(name, used_by)| {
            let found = crate_for(lang, name);
            DependencyReport {
                name: name.to_string(),
                used_by,
                crates: found.map(|m| m.crates),
                fit: match found.map(|m| m.fit) {
                    Some(Fit::Good) => "good",
                    Some(Fit::Partial) => "partial",
                    Some(Fit::None) => "none",
                    None => "unknown",
                },
                note: found.map_or("", |m| m.note),
            }
        })
        .collect();
    let rank = |fit: &str| match fit {
        "none" => 0,
        "unknown" => 1,
        "partial" => 2,
        _ => 3,
    };
    out.sort_by(|a, b| {
        rank(a.fit)
            .cmp(&rank(b.fit))
            .then(b.used_by.cmp(&a.used_by))
    });
    out
}

/// Readiness balanced with how much a port would gain.
fn priority(
    readiness: f64,
    density: f64,
    loc: usize,
    direct_tests: usize,
    package_tests: usize,
) -> u32 {
    let compute = (density * 4.0).min(50.0);
    let tests = if direct_tests > 0 {
        30.0
    } else if package_tests > 0 {
        15.0
    } else {
        0.0
    };
    // Below ~80 lines a port rarely pays for its bridge; past ~600 it is too big for a first step.
    let size = match loc {
        0..80 => loc as f64 / 4.0,
        80..600 => 20.0,
        _ => 10.0,
    };
    let value = compute + tests + size;
    (readiness * 0.5 + value * 0.5).round().clamp(0.0, 100.0) as u32
}

/// For each node, whether it or anything reachable from it is flagged.
fn reaches(edges: &[Vec<usize>], flagged: &[bool]) -> Vec<bool> {
    let mut out = flagged.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for v in 0..edges.len() {
            if !out[v] && edges[v].iter().any(|&w| out[w]) {
                out[v] = true;
                changed = true;
            }
        }
    }
    out
}

/// The unit id for a file: its folder, or the file itself when it sits at the source root.
fn unit_id(package: &Package, path: &str) -> String {
    let base =
        if !package.src_root.is_empty() && path.starts_with(&format!("{}/", package.src_root)) {
            &package.src_root
        } else {
            &package.dir
        };
    let rel = if base.is_empty() {
        path
    } else {
        &path[base.len() + 1..]
    };
    match rel.rsplit_once('/') {
        Some((parent, _)) => format!("{}/", discover::join(base, parent)),
        None => path.to_string(),
    }
}

/// Resolves `./x`, `../x.js` and folder imports the way TS bundler resolution does.
fn resolve_relative(from: &str, spec: &str, index: &HashMap<&str, usize>) -> Option<usize> {
    let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
    resolve_base(&normalize(&discover::join(dir, spec))?, index)
}

/// A root-relative import target, with the extensions and index files bundlers try.
fn resolve_base(base: &str, index: &HashMap<&str, usize>) -> Option<usize> {
    let base = base.to_string();
    let mut candidates = vec![base.clone()];
    for (js, ts) in [
        (".js", ".ts"),
        (".js", ".tsx"),
        (".jsx", ".tsx"),
        (".mjs", ".mts"),
        (".cjs", ".cts"),
    ] {
        if let Some(stem) = base.strip_suffix(js) {
            candidates.push(format!("{stem}{ts}"));
        }
    }
    for ext in [
        "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue", "svelte", "astro",
    ] {
        candidates.push(format!("{base}.{ext}"));
        candidates.push(format!("{base}/index.{ext}"));
    }
    candidates
        .iter()
        .find_map(|c| index.get(c.as_str()).copied())
}

/// Collapses `.` and `..` segments; `None` when the path leaves the root.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// Port waves from strongly connected components: a unit comes after everything it imports.
/// Returns each node's wave and the members of its component.
pub(crate) fn waves(edges: &[Vec<usize>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    struct Tarjan<'e> {
        edges: &'e [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        components: Vec<Vec<usize>>,
    }
    fn visit(t: &mut Tarjan, v: usize) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on_stack[v] = true;
        let edges = t.edges;
        for &w in &edges[v] {
            match t.index[w] {
                None => {
                    visit(t, w);
                    t.low[v] = t.low[v].min(t.low[w]);
                }
                Some(iw) if t.on_stack[w] => t.low[v] = t.low[v].min(iw),
                Some(_) => {}
            }
        }
        if t.index[v] == Some(t.low[v]) {
            let mut component = Vec::new();
            while let Some(w) = t.stack.pop() {
                t.on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            t.components.push(component);
        }
    }
    let n = edges.len();
    let mut t = Tarjan {
        edges,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        components: Vec::new(),
    };
    for v in 0..n {
        if t.index[v].is_none() {
            visit(&mut t, v);
        }
    }
    // Tarjan emits components after everything they reach, so dependencies come first.
    let mut wave = vec![0usize; n];
    let mut members = vec![Vec::new(); n];
    for component in &t.components {
        let w = component
            .iter()
            .flat_map(|&v| edges[v].iter())
            .filter(|w| !component.contains(w))
            .map(|&w| wave[w] + 1)
            .max()
            .unwrap_or(0);
        for &v in component {
            wave[v] = w;
            members[v] = component.clone();
        }
    }
    (wave, members)
}

/// Kahn's algorithm; nodes in cycles are appended in index order.
fn topological(deps_of: &[Vec<usize>]) -> Vec<usize> {
    let n = deps_of.len();
    let mut remaining: Vec<usize> = deps_of.iter().map(Vec::len).collect();
    let mut order = Vec::with_capacity(n);
    let mut done = vec![false; n];
    while let Some(next) = (0..n).find(|&i| !done[i] && remaining[i] == 0) {
        done[next] = true;
        order.push(next);
        for (i, deps) in deps_of.iter().enumerate() {
            if deps.contains(&next) {
                remaining[i] -= 1;
            }
        }
    }
    order.extend((0..n).filter(|&i| !done[i]));
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize("a/./b/../c").as_deref(), Some("a/c"));
        assert_eq!(normalize("../x"), None);
    }

    #[test]
    fn orders_waves_and_cycles() {
        // 0 -> 1 -> 2, 3 <-> 4 -> 2
        let edges = vec![vec![1], vec![2], vec![], vec![4], vec![3, 2]];
        let (wave, members) = waves(&edges);
        assert_eq!(wave, [2, 1, 0, 1, 1]);
        assert_eq!(members[3].len(), 2);
    }

    #[test]
    fn sorts_packages_topologically() {
        assert_eq!(topological(&[vec![1], vec![], vec![0, 1]]), [1, 0, 2]);
    }
}
