use std::path::Path;

use rust_anything::analyze::{Bridge, Report, Status, UnitReport, analyze};
use rust_anything::risks::Risk;

fn mini() -> Report {
    analyze(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini")).unwrap()
}

fn unit<'r>(report: &'r Report, id: &str) -> &'r UnitReport {
    report
        .units
        .iter()
        .find(|u| u.id == id)
        .unwrap_or_else(|| panic!("no unit {id}"))
}

#[test]
fn detects_manifestless_typescript_modules_and_defaults_to_typescript_when_empty() {
    use rust_anything::lang::Lang;
    let root = std::env::temp_dir().join(format!(
        "rust-anything-detect-modules-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let empty = Lang::detect(&root);
    std::fs::create_dir(root.join("generated.java")).unwrap();
    std::fs::write(root.join("main.mts"), "export function main() {}").unwrap();
    std::fs::write(root.join("util.cts"), "export function util() {}").unwrap();
    let modules = analyze(&root);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(empty, Lang::Ts);
    assert_eq!(modules.unwrap().summary.source_files, 2);
}

#[test]
fn groups_files_into_units_and_skips_fixtures() {
    let report = mini();
    let ids: Vec<&str> = report.units.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "packages/a/src/fs.ts",
            "packages/a/src/parse/",
            "packages/a/src/svelte.ts",
            "packages/a/src/util.ts",
            "packages/a/src/view/",
            "packages/b/src/index.ts",
        ]
    );
    assert_eq!(unit(&report, "packages/a/src/parse/").files.len(), 2);
    assert_eq!(report.summary.test_files, 1);
}

#[test]
fn resolves_imports_into_waves_and_credits_tests() {
    let report = mini();
    let util = unit(&report, "packages/a/src/util.ts");
    assert_eq!(util.wave, 0);
    assert_eq!(util.direct_tests, 1);
    assert_eq!(util.fan_in, 2);
    let fs = unit(&report, "packages/a/src/fs.ts");
    assert_eq!(fs.wave, 1);
    assert_eq!(fs.internal_deps, ["packages/a/src/util.ts"]);
    assert_eq!(
        unit(&report, "packages/b/src/index.ts").workspace_deps,
        ["a"]
    );
}

#[test]
fn finds_risks_with_locations() {
    let report = mini();
    let util = unit(&report, "packages/a/src/util.ts");
    let regex = util
        .risks
        .iter()
        .find(|r| r.risk == Risk::RegexSyntax)
        .unwrap();
    assert_eq!(regex.samples, ["packages/a/src/util.ts:1"]);
    assert!(util.risks.iter().any(|r| r.risk == Risk::StringOffsets));
}

#[test]
fn picks_status_and_bridge() {
    let report = mini();
    let fs = unit(&report, "packages/a/src/fs.ts");
    assert_eq!((fs.status, fs.bridge), (Status::Ready, Bridge::NapiRs));
    let view = unit(&report, "packages/a/src/view/");
    assert_eq!((view.status, view.bridge), (Status::Keep, Bridge::KeepInJs));
    assert_eq!(
        unit(&report, "packages/a/src/svelte.ts").status,
        Status::Blocked
    );
    // `b` uses no Node API, so its code can run anywhere.
    assert_eq!(
        unit(&report, "packages/b/src/index.ts").bridge,
        Bridge::WasmBindgen
    );
    let order: Vec<(&str, usize)> = report
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p.order))
        .collect();
    assert_eq!(order, [("a", 1), ("b", 2)]);
    assert!(!report.start_here.is_empty());
    assert!(
        !report
            .start_here
            .iter()
            .any(|id| id.ends_with("svelte.ts") || id.ends_with("view/"))
    );
}

#[test]
fn python_packages_match_dependencies_by_normalized_name() {
    let root =
        std::env::temp_dir().join(format!("rust-anything-python-names-{}", std::process::id()));
    let package = |dir: &str, toml: &str| {
        std::fs::create_dir_all(root.join(dir).join(dir)).unwrap();
        std::fs::write(root.join(dir).join("pyproject.toml"), toml).unwrap();
        std::fs::write(
            root.join(dir).join(dir).join("__init__.py"),
            "def f():\n    return 1\n",
        )
        .unwrap();
    };
    package(
        "app",
        "[project]\nname = \"app\"\ndependencies = [\"core_lib>=1\"]\n",
    );
    package("core", "[project]\nname = \"Core_Lib\"\n");
    let report = analyze(&root);
    std::fs::remove_dir_all(&root).unwrap();
    let packages: Vec<(&str, Vec<String>, usize)> = report
        .as_ref()
        .unwrap()
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p.workspace_deps.clone(), p.order))
        .collect();
    assert_eq!(
        packages,
        [
            ("core-lib", vec![], 1),
            ("app", vec!["core-lib".to_string()], 2)
        ]
    );
}

#[test]
fn numpy_array_parameters_are_flagged() {
    let root = std::env::temp_dir().join(format!("rust-anything-ndarray-{}", std::process::id()));
    std::fs::create_dir_all(root.join("arrkit")).unwrap();
    std::fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = \"arrkit\"\ndependencies = [\"numpy\"]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("arrkit/__init__.py"),
        "import numpy as np\n\ndef scale(a: np.ndarray, k: float) -> np.ndarray:\n    return a * k\n",
    )
    .unwrap();
    let report = analyze(&root);
    std::fs::remove_dir_all(&root).unwrap();
    let json = serde_json::to_string(&report.unwrap()).unwrap();
    assert!(json.contains("array-parameters"), "{json}");
}

#[test]
fn a_c_unit_depends_on_what_its_own_header_includes() {
    let root = std::env::temp_dir().join(format!("rust-anything-c-header-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    for (name, text) in [
        ("src/a.c", "#include \"a.h\"\nint a(void) { return b(); }\n"),
        ("src/a.h", "#pragma once\n#include \"b.h\"\nint a(void);\n"),
        ("src/b.c", "#include \"b.h\"\nint b(void) { return 42; }\n"),
        ("src/b.h", "#pragma once\nint b(void);\n"),
    ] {
        std::fs::write(root.join(name), text).unwrap();
    }
    let report = rust_anything::analyze::analyze_lang(&root, rust_anything::lang::Lang::C).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let a = unit(&report, "src/a.c");
    assert_eq!(a.internal_deps, ["src/b.c"], "{:?}", a.internal_deps);
    assert!(a.wave > unit(&report, "src/b.c").wave);
}

#[test]
fn json_output_is_versioned() {
    let json = serde_json::to_value(mini()).unwrap();
    assert_eq!(json["schema"], "rust-anything/analyze@1");
    assert_eq!(json["units"][0]["status"], "ready");
    assert_eq!(json["units"][0]["bridge"], "napi-rs");
}

fn whole(dir: &str) -> rust_anything::whole_app::WholeApp {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dir);
    let report = analyze(&root).unwrap();
    rust_anything::whole_app::whole_app(&root, &report)
}

#[test]
fn whole_app_finds_the_shape_and_entry_points_in_every_language() {
    let cases = [
        ("cli/ts", "cli", "tcli.js"),
        ("cli/python", "cli", "tcli.py"),
        ("cli/c", "cli", "tcli.c"),
        ("cli/go", "cli", "main.go"),
        ("cli/ruby", "cli", "tcli.rb"),
        ("cli/java", "cli", "Tcli.java"),
        ("cli/php", "cli", "tcli.php"),
        ("cli/csharp", "cli", "tcli"),
        ("http/ts", "server", "server.js"),
        ("http/python", "server", "server.py"),
        ("http/c", "server", "server.c"),
        ("http/go", "server", "main.go"),
        ("http/ruby", "server", "server.rb"),
        ("http/java", "server", "Server.java"),
        ("http/php", "server", "server.php"),
        ("http/csharp", "server", "server"),
        ("go", "cli", "cmd/strkit"),
        ("c", "cli", "src/main.c"),
        ("mini", "library", "a"),
        ("python", "library", "textkit"),
        ("ruby", "library", "textkit"),
        ("java", "library", "textkit"),
        ("php", "library", "acme/textkit"),
        ("csharp", "library", "Acme.Textkit"),
    ];
    for (dir, shape, path) in cases {
        let w = whole(dir);
        assert_eq!(w.shape, shape, "{dir}: {:?}", w.entry_points);
        assert!(
            w.entry_points
                .iter()
                .any(|e| e.path.ends_with(path) || e.name.as_deref() == Some(path)),
            "{dir}: {:?}",
            w.entry_points
        );
        assert_eq!(w.decisions[0].topic, "shape");
        assert!(
            w.dependencies.iter().all(|d| d.fit != "unknown"),
            "{dir}: {:?}",
            w.dependencies
        );
    }
}

#[test]
fn php_units_resolve_psr4_imports_and_map_composer_packages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/php");
    let report = analyze(&root).unwrap();
    assert_eq!(report.language, rust_anything::lang::Lang::Php);
    let slug = report
        .units
        .iter()
        .find(|u| u.files.iter().any(|f| f.ends_with("Slug.php")))
        .unwrap();
    assert_eq!(slug.bridge, Bridge::PhpFfi);
    let ascii = report
        .units
        .iter()
        .find(|u| u.files.iter().any(|f| f.ends_with("Ascii.php")))
        .unwrap();
    assert!(slug.wave > ascii.wave, "Slug imports Ascii: {slug:?}");
    let w = rust_anything::whole_app::whole_app(&root, &report);
    let carbon = w
        .dependencies
        .iter()
        .find(|d| d.name == "nesbot/carbon")
        .unwrap();
    assert!(carbon.declared);
    assert_eq!(carbon.fit, "good");
    assert!(
        w.dependencies
            .iter()
            .all(|d| d.name != "php" && !d.name.starts_with("ext-"))
    );
}

#[test]
fn whole_app_lists_declared_dependencies_keeps_and_decisions() {
    let python = whole("python");
    let requests = python
        .dependencies
        .iter()
        .find(|d| d.name == "requests")
        .unwrap();
    assert!(requests.declared);
    assert_eq!(requests.fit, "good");

    let mini = whole("mini");
    let topics: Vec<&str> = mini.decisions.iter().map(|d| d.topic.as_str()).collect();
    assert!(topics.contains(&"keep packages/a/src/view/"), "{topics:?}");
    assert!(
        topics.contains(&"blocked packages/a/src/svelte.ts"),
        "{topics:?}"
    );
    assert!(topics.contains(&"dependency svelte"), "{topics:?}");
    assert!(mini.decisions.iter().all(|d| !d.recommendation.is_empty()));
    let ids: Vec<&str> = mini.decisions.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids[..3], ["D1", "D2", "D3"]);
}

#[test]
fn whole_app_ports_units_that_import_each_other_as_one_piece() {
    let root = std::env::temp_dir().join(format!("rust-anything-tangled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("package.json"), r#"{"name":"tangled"}"#).unwrap();
    std::fs::write(
        root.join("src/ping.ts"),
        "import { pong } from './pong'\nexport function ping(n: number): number { return n > 0 ? pong(n - 1) : 0 }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/pong.ts"),
        "import { ping } from './ping'\nexport function pong(n: number): number { return n > 0 ? ping(n - 1) : 1 }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/leaf.ts"),
        "export function leaf(s: string): string { return s.trim() }\n",
    )
    .unwrap();
    let report = analyze(&root).unwrap();
    let w = rust_anything::whole_app::whole_app(&root, &report);
    let tangled: Vec<&Vec<String>> = w
        .pieces
        .iter()
        .filter(|p| p.tangled)
        .map(|p| &p.units)
        .collect();
    assert_eq!(
        tangled,
        [&vec!["src/ping.ts".to_string(), "src/pong.ts".to_string()]]
    );
    assert_eq!(w.pieces.len(), 2);
    assert!(w.decisions.iter().any(|d| d.topic == "tangled units"));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn ignore_rules_above_the_repository_are_skipped_and_hiding_everything_warns() {
    let tmp = std::env::temp_dir().join(format!("ra-ignore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let repo = tmp.join("home/proj");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(tmp.join("home/.gitignore"), "*\n").unwrap();
    std::fs::write(repo.join("package.json"), r#"{"name":"x"}"#).unwrap();
    std::fs::write(
        repo.join("src/a.ts"),
        "export function f(t: string): string { return t; }\n",
    )
    .unwrap();

    let report = analyze(&repo).unwrap();
    assert_eq!(report.summary.source_files, 1);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);

    std::fs::create_dir(repo.join(".git")).unwrap();
    assert_eq!(analyze(&repo).unwrap().summary.source_files, 1);

    std::fs::write(repo.join(".gitignore"), "*\n").unwrap();
    let report = analyze(&repo).unwrap();
    assert_eq!(report.summary.source_files, 0);
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("1 TypeScript/JavaScript files"));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn platform_specific_c_stays_in_c_and_compiled_apps_are_refused() {
    let root = std::env::temp_dir().join(format!("ra-platform-c-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let write = |path: &str, text: &str| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "hw/regs.c",
        "#include <stdint.h>\n\
         struct flags { unsigned ready : 1; unsigned mode : 3; };\n\
         void led_on(void) { *(volatile uint32_t *)0x40021000 = 1; }\n\
         void wait(void) { __asm__ volatile (\"nop\"); }\n",
    );
    write(
        "util/sum.c",
        "int sum(const int *xs, int n) { int t = 0; for (int i = 0; i < n; i++) t += xs[i]; return t; }\n",
    );
    let report = analyze(&root).unwrap();
    let hw = report.units.iter().find(|u| u.id.contains("hw")).unwrap();
    assert_eq!(hw.status, Status::Keep, "{hw:?}");
    assert!(
        hw.risks.iter().any(|r| r.risk == Risk::PlatformSpecific),
        "{hw:?}"
    );
    let util = report.units.iter().find(|u| u.id.contains("util")).unwrap();
    assert_ne!(util.status, Status::Keep, "{util:?}");

    write("RealCalc.apk", "PK");
    let err = analyze(&root.join("RealCalc.apk")).unwrap_err();
    assert!(err.to_string().contains("compiled .apk"), "{err}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_c_library_folder_reads_like_ffmpegs_libavutil() {
    let root = std::env::temp_dir()
        .join(format!("ra-libavutil-{}", std::process::id()))
        .join("libavutil");
    let _ = std::fs::remove_dir_all(&root);
    let write = |path: &str, text: &str| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "base64.h",
        "#include <stdint.h>\nint b64_decode(uint8_t *out, const char *in, int out_size);\n",
    );
    write(
        "base64.c",
        "#include \"base64.h\"\n\
         int b64_decode(uint8_t *out, const char *in, int out_size) {\n\
         \x20   if (!out) goto check;\n    return out_size;\ncheck:\n    return in[0] == 0 ? 0 : -1;\n}\n",
    );
    write(
        "tests/base64.c",
        "#include \"libavutil/base64.h\"\nint main(void) { return b64_decode(0, \"\", 0); }\n",
    );
    write("x86/cpu.c", "int cpu_flags(void) { return 1; }\n");
    let report = analyze(&root).unwrap();
    let ids: Vec<&str> = report.units.iter().map(|u| u.id.as_str()).collect();
    let base64 = report
        .units
        .iter()
        .find(|u| u.id == "base64.c")
        .unwrap_or_else(|| panic!("{ids:?}"));
    assert_ne!(
        base64.status,
        Status::Blocked,
        "goto is control flow: {base64:?}"
    );
    assert_eq!(
        base64.direct_tests, 1,
        "`libavutil/base64.h` from inside libavutil: {base64:?}"
    );
    assert_eq!(base64.bridgeable, 1, "{base64:?}");
    let cpu = report
        .units
        .iter()
        .find(|u| u.id.starts_with("x86/"))
        .unwrap_or_else(|| panic!("{ids:?}"));
    assert_eq!(cpu.status, Status::Keep, "{cpu:?}");
    std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
}

#[test]
fn electron_apps_get_a_main_process_decision() {
    let root = std::env::temp_dir().join(format!("ra-electron-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"charts","main":"src/main.ts","devDependencies":{"electron":"^30.0.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("src/main.ts"),
        "export function sma(xs: number[], n: number): number[] { return xs.slice(n - 1).map((_, i) => xs.slice(i, i + n).reduce((a, b) => a + b, 0) / n) }\n",
    )
    .unwrap();
    let report = analyze(&root).unwrap();
    let app = rust_anything::whole_app::whole_app(&root, &report);
    std::fs::remove_dir_all(&root).unwrap();
    let shell = app
        .decisions
        .iter()
        .find(|d| d.topic == "desktop shell")
        .unwrap_or_else(|| panic!("{:#?}", app.decisions));
    assert!(shell.recommendation.contains("Tauri"), "{shell:?}");
}

#[test]
fn csharp_units_resolve_namespaces_and_map_nuget_packages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/csharp");
    let report = analyze(&root).unwrap();
    assert_eq!(report.language, rust_anything::lang::Lang::Csharp);
    assert_eq!(report.summary.test_files, 1);
    let text = unit(&report, "src/Text/");
    let util = unit(&report, "src/Util/");
    assert_eq!(text.internal_deps, ["src/Util/"]);
    assert_eq!((util.wave, text.wave), (0, 1));
    assert_eq!(text.bridge, Bridge::DotnetFfi);
    assert!(text.risks.iter().any(|r| r.risk == Risk::RegexSyntax));
    assert_eq!(util.external_deps, ["Newtonsoft.Json"]);
    let json = report
        .dependencies
        .iter()
        .find(|d| d.name == "Newtonsoft.Json")
        .unwrap();
    assert_eq!(json.crates, Some("serde_json"));
    let app = rust_anything::whole_app::whole_app(&root, &report);
    assert!(
        app.dependencies.iter().all(|d| d.name != "xunit"),
        "test packages are not runtime dependencies: {:?}",
        app.dependencies
    );
}

#[test]
fn kotlin_units_export_jvm_statics_and_map_gradle_packages() {
    use rust_anything::lang::{Lang, exports};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kotlin");
    let report = analyze(&root).unwrap();
    assert_eq!(report.language, Lang::Kotlin);

    let source =
        std::fs::read_to_string(root.join("src/main/kotlin/com/example/text/Strings.kt")).unwrap();
    let (fns, _) = exports(
        Lang::Kotlin,
        "src/main/kotlin/com/example/text/Strings.kt",
        &source,
    );
    let by_name = |name: &str| fns.iter().find(|f| f.name == name).unwrap();
    assert_eq!(
        rust_anything::lang::kotlin::receiver(by_name("slugify")).0,
        "com.example.text.StringsKt"
    );
    assert_eq!(
        rust_anything::lang::kotlin::receiver(by_name("gcd")).0,
        "com.example.text.Nums"
    );
    assert!(
        by_name("slugify").blocker.is_none(),
        "{:?}",
        by_name("slugify")
    );
    assert_eq!(by_name("words").returns, "string[]");

    let w = rust_anything::whole_app::whole_app(&root, &report);
    let gson = w
        .dependencies
        .iter()
        .find(|d| d.name == "com.google.code.gson:gson")
        .unwrap();
    assert!(gson.declared);
    assert_eq!(gson.fit, "good");
    assert!(
        w.dependencies
            .iter()
            .all(|d| d.name != "com.example:legacy"),
        "a commented-out dependency is not declared: {:?}",
        w.dependencies
    );
}

#[test]
fn a_comment_inside_a_require_block_is_not_a_module() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go-reqs");
    let declared = rust_anything::whole_app::declared(&root, Lang::Go);
    for name in [
        "github.com/spf13/cobra",
        "github.com/google/uuid",
        "github.com/mattn/go-sqlite3",
    ] {
        assert!(declared.contains(name), "{name}: {declared:?}");
    }
    for bogus in ["//", "pinned", "until"] {
        assert!(
            !declared.contains(bogus),
            "{bogus} is not a module: {declared:?}"
        );
    }
}

#[test]
fn a_gem_group_ends_at_its_own_end() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ruby-gems");
    let declared = rust_anything::whole_app::declared(&root, Lang::Ruby);
    assert!(
        declared.contains("rack"),
        "a Gemfile gem is a runtime dependency: {declared:?}"
    );
    assert!(
        !declared.contains("rspec"),
        "a gem inside `group :test do` is not: {declared:?}"
    );
    assert!(
        !declared.contains("debug"),
        "a gem inside a nested block in a group is not: {declared:?}"
    );
}

#[test]
fn a_commented_out_maven_dependency_is_not_declared() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/java-pom");
    let declared = rust_anything::whole_app::declared(&root, Lang::Java);
    assert!(
        declared.contains("com.google.guava:guava"),
        "a live dependency is declared: {declared:?}"
    );
    for commented in ["com.example:legacy", "com.example:older"] {
        assert!(
            !declared.contains(commented),
            "{commented} is commented out: {declared:?}"
        );
    }
    assert!(
        !declared.iter().any(|d| d.contains("junit")),
        "a test-scope dependency is not a runtime one: {declared:?}"
    );
}

#[test]
fn a_commented_out_test_package_does_not_hide_a_csharp_project() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/csharp-comments");
    let text = std::fs::read_to_string(root.join("App.csproj")).unwrap();
    assert!(
        !rust_anything::lang::csharp::is_test_project(&text),
        "a project whose test package is commented out is not a test project"
    );
    let declared = rust_anything::whole_app::declared(&root, Lang::Csharp);
    for name in ["Newtonsoft.Json", "Serilog"] {
        assert!(declared.contains(name), "{name}: {declared:?}");
    }
    assert!(
        !declared.iter().any(|d| d.to_lowercase().contains("xunit")),
        "a commented-out package is not declared: {declared:?}"
    );
}

#[test]
fn a_requirements_file_names_packages_not_urls() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/python-reqs");
    let declared = rust_anything::whole_app::declared(&root, Lang::Python);
    for name in ["requests", "flask", "myapp", "six"] {
        assert!(declared.contains(name), "{name}: {declared:?}");
    }
    for bogus in ["git", ".", "foo"] {
        assert!(
            !declared.contains(bogus),
            "{bogus} is not a package a registry serves: {declared:?}"
        );
    }
}

#[test]
fn a_requirements_include_is_followed_once() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/python-includes");
    let declared = rust_anything::whole_app::declared(&root, Lang::Python);
    for name in ["flask", "six", "pytest", "ruff"] {
        assert!(declared.contains(name), "{name}: {declared:?}");
    }
}

#[test]
fn a_java_import_names_the_library_the_table_knows() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/java-guava");
    let report = analyze(&root).unwrap();
    assert_eq!(report.language, Lang::Java);
    let unit = &report.units[0];
    for library in ["com.google.common", "org.apache.commons"] {
        assert!(
            unit.external_deps.contains(&library.to_string()),
            "{library}: {:?}",
            unit.external_deps
        );
    }
    for prefix in ["com.google", "org.apache"] {
        assert!(
            !unit.external_deps.contains(&prefix.to_string()),
            "a two-segment prefix is not a library: {:?}",
            unit.external_deps
        );
    }
}

#[test]
fn a_function_import_does_not_name_a_composer_package() {
    use rust_anything::lang::Lang;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/php-namespace");
    let report = analyze(&root).unwrap();
    assert_eq!(report.language, Lang::Php);
    let user = report
        .units
        .iter()
        .find(|u| u.id == "src/User.php")
        .unwrap_or_else(|| {
            panic!(
                "units: {:?}",
                report.units.iter().map(|u| &u.id).collect::<Vec<_>>()
            )
        });
    assert!(
        user.external_deps.contains(&"monolog/monolog".to_string()),
        "{:?}",
        user.external_deps
    );
    assert!(
        !user.external_deps.contains(&"Acme".to_string()),
        "the project's own namespace is not a Composer package: {:?}",
        user.external_deps
    );
    assert!(
        user.external_deps.contains(&"Stripe".to_string()),
        "a vendor namespace the table cannot place stays external: {:?}",
        user.external_deps
    );
    assert!(
        user.internal_deps.contains(&"src/Greeter.php".to_string()),
        "{:?}",
        user.internal_deps
    );
}

fn scratch(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ra-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

#[test]
fn the_plugin_and_whole_app_read_a_manifest_the_same_way() {
    use rust_anything::lang::{Lang, discover};
    let fixture = |dir: &str| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(dir)
    };

    let ws = discover(Lang::Ruby, &fixture("ruby-gems")).unwrap();
    let gemfile = ws.packages.iter().find(|p| p.dir.is_empty()).unwrap();
    assert_eq!(gemfile.deps, ["rack"]);
    assert_eq!(gemfile.dev_deps, ["rspec", "byebug", "debug"]);
    let declared: Vec<String> =
        rust_anything::whole_app::declared(&fixture("ruby-gems"), Lang::Ruby)
            .into_iter()
            .collect();
    assert_eq!(declared, gemfile.deps);

    let ws = discover(Lang::Python, &fixture("python-includes")).unwrap();
    let mut deps = ws.packages[0].deps.clone();
    deps.sort();
    deps.dedup();
    assert_eq!(deps, ["flask", "pytest", "ruff", "six"]);

    let ws = discover(Lang::Java, &fixture("java-pom")).unwrap();
    assert!(
        ws.packages[0]
            .deps
            .iter()
            .all(|d| !d.starts_with("com.example:")),
        "a commented-out dependency is not one: {:?}",
        ws.packages[0].deps
    );
}

#[test]
fn a_python_library_imported_as_yaml_and_declared_as_pyyaml_is_one_dependency() {
    let root = scratch(
        "yaml-join",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"conf\"\ndependencies = [\"PyYAML>=6\", \"orjson\"]\n",
            ),
            (
                "conf/load.py",
                "import yaml\nimport left_pad_but_new\n\ndef load(text: str) -> dict:\n    return yaml.safe_load(text)\n",
            ),
        ],
    );
    let report = analyze(&root).unwrap();
    let w = rust_anything::whole_app::whole_app(&root, &report);
    std::fs::remove_dir_all(&root).unwrap();
    let yaml: Vec<_> = w
        .dependencies
        .iter()
        .filter(|d| d.names.iter().any(|n| n == "yaml" || n == "pyyaml"))
        .collect();
    assert_eq!(yaml.len(), 1, "{:?}", w.dependencies);
    assert_eq!(yaml[0].names, ["pyyaml", "yaml"]);
    assert_eq!(yaml[0].name, "pyyaml");
    assert!(yaml[0].declared);
    assert_eq!(yaml[0].used_by, 1);
    let unknown = w
        .dependencies
        .iter()
        .find(|d| d.name == "left-pad-but-new")
        .unwrap_or_else(|| panic!("{:?}", w.dependencies));
    assert_eq!((unknown.names.len(), unknown.declared), (1, false));
    let json = w.dependencies.iter().find(|d| d.name == "orjson").unwrap();
    assert_eq!((json.used_by, json.declared), (0, true));
}

#[test]
fn a_kotlin_import_and_its_gradle_coordinates_are_one_dependency() {
    let root = scratch(
        "room-join",
        &[
            (
                "build.gradle.kts",
                "plugins { kotlin(\"jvm\") version \"2.0.21\" }\n\
                 dependencies { implementation(\"androidx.room:room-runtime:2.6.1\") }\n",
            ),
            (
                "src/main/kotlin/com/acme/G.kt",
                "package com.acme\nimport androidx.room.RoomDatabase\nobject G { @JvmStatic fun f(): String = \"x\" }\n",
            ),
        ],
    );
    let report = analyze(&root).unwrap();
    let w = rust_anything::whole_app::whole_app(&root, &report);
    std::fs::remove_dir_all(&root).unwrap();
    let room: Vec<_> = w
        .dependencies
        .iter()
        .filter(|d| d.names.iter().any(|n| n.starts_with("androidx.room")))
        .collect();
    assert_eq!(room.len(), 1, "{:?}", w.dependencies);
    assert_eq!(
        room[0].names,
        ["androidx.room", "androidx.room:room-runtime"]
    );
    assert_eq!(room[0].name, "androidx.room:room-runtime");
    assert!(room[0].declared);
    assert_eq!(room[0].used_by, 1);
    let json = serde_json::to_value(room[0]).unwrap();
    assert_eq!(json["name"], "androidx.room:room-runtime");
    assert_eq!(json["names"][0], "androidx.room");
    let plan = rust_anything::report::markdown(
        &rust_anything::analyze::Report {
            whole_app: Some(w),
            ..report
        },
        "room",
    );
    assert!(
        plan.contains("`androidx.room:room-runtime` (also `androidx.room`)"),
        "{plan}"
    );
}

#[test]
fn a_unit_importing_another_workspace_package_is_not_a_leaf() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/monorepo");
    let report = analyze(&root).unwrap();
    let b = report
        .units
        .iter()
        .find(|u| u.id == "packages/b/src/titles.ts")
        .unwrap();
    assert_eq!(b.workspace_deps, ["@m/a"]);
    assert!(
        !b.notes.iter().any(|n| n.starts_with("Leaf:")),
        "a workspace import is an import: {:?}",
        b.notes
    );
    assert!(
        b.notes.iter().any(|n| n.contains("@m/a")),
        "the note should name the package: {:?}",
        b.notes
    );
}

#[test]
fn map_names_a_path_that_does_not_exist() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_rust-anything"))
        .args(["map", "/no/such/rust-anything-path"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("/no/such/rust-anything-path"), "{err}");
}

#[test]
fn bridgeable_exports_are_counted_as_bridge_judges_them() {
    let root = std::env::temp_dir().join(format!("ra-analyze-dts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    let write = |path: &str, text: &str| std::fs::write(root.join(path), text).unwrap();
    write(
        "package.json",
        r#"{"name":"t","version":"1.0.0","type":"module"}"#,
    );
    write(
        "src/plug.js",
        "export function use(plugin) { return plugin(1); }\n",
    );
    write(
        "src/plug.d.ts",
        "export declare function use(plugin: (x: number) => number): number;\n",
    );
    write(
        "src/sum.js",
        "export function sum(a, b) { return a + b; }\n",
    );
    write(
        "src/sum.d.ts",
        "export declare function sum(a: number, b: number): number;\n",
    );
    write(
        "src/bus.ts",
        "export class Bus {\n  on(h: (x: number) => void): void { h(1); }\n}\n",
    );
    write(
        "src/pat.ts",
        "type Pat = RegExp;\nexport function test(p: Pat, s: string): boolean { return p.test(s); }\n",
    );
    let report = analyze(&root).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    for (id, why) in [
        (
            "src/bus.ts",
            "`bridge` keeps a class with a callback method in JS",
        ),
        (
            "src/pat.ts",
            "`bridge` resolves `Pat` to RegExp and keeps `test` in JS",
        ),
    ] {
        let u = unit(&report, id);
        assert_eq!((u.exports, u.bridgeable), (1, 0), "{why}: {u:?}");
        assert_eq!(u.status, Status::Caution, "{u:?}");
    }
    let plug = unit(&report, "src/plug.js");
    assert_eq!(
        (plug.exports, plug.bridgeable),
        (1, 0),
        "`bridge` reads plug.d.ts and keeps `use` in JS: {plug:?}"
    );
    assert_eq!(plug.status, Status::Caution, "{plug:?}");
    assert!(
        report.start_here == ["src/sum.js"],
        "{:?}",
        report.start_here
    );
    let sum = unit(&report, "src/sum.js");
    assert_eq!((sum.exports, sum.bridgeable), (1, 1), "{sum:?}");
}
