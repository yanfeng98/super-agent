//! Step 4: a drop-in module that runs the TS original and the Rust port side by side.
//!
//! Import the generated module where the app imported the original. Every call returns the
//! original's result; the Rust result is compared and mismatches are reported. Once production
//! traffic shows no mismatches, `RUST_ANYTHING_MODE=new` switches to Rust, still without a code change.

use crate::walk::Canonical;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::bridge::Manifest;
use crate::harness::CODEC_JS;

#[derive(Debug, Serialize)]
pub struct ShadowOutcome {
    pub path: PathBuf,
    pub exports: Vec<String>,
}

pub fn shadow(crate_dir: &Path, out: Option<&Path>) -> Result<ShadowOutcome> {
    let crate_dir = crate_dir
        .canonical()
        .with_context(|| format!("resolving {}", crate_dir.display()))?;
    let manifest = Manifest::load(&crate_dir)?;
    if manifest.language == crate::lang::Lang::Python {
        return shadow_python(&crate_dir, &manifest, out);
    }
    if manifest.kind == crate::bridge::Kind::CAbi {
        return shadow_c(&crate_dir, &manifest, out);
    }
    if matches!(
        manifest.language,
        crate::lang::Lang::Java | crate::lang::Lang::Kotlin
    ) {
        let dir = out.map_or_else(|| crate_dir.join("java"), Path::to_path_buf);
        let port = std::fs::read_to_string(crate_dir.join("src/port.rs")).unwrap_or_default();
        let mut api = crate::codegen::parse_port(&port);
        crate::bridge::match_export_names(&manifest, &mut api);
        let files = crate::codegen_java::shadows(&manifest, &api);
        for (path, _) in &files {
            let target = dir.join(path.trim_start_matches("shadow/"));
            if target.exists() {
                bail!("{} exists; remove it first or pass --out", target.display());
            }
        }
        let mut first = None;
        for (path, text) in &files {
            let target = dir.join(path.trim_start_matches("shadow/"));
            std::fs::create_dir_all(target.parent().context("no folder")?)?;
            std::fs::write(&target, with_kill_switch(text, &manifest.crate_name))?;
            first.get_or_insert(target);
        }
        return Ok(ShadowOutcome {
            path: first.context("no class to shadow")?,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    if manifest.language == crate::lang::Lang::Php {
        bail!(
            "PHP cannot redefine a function once the original is loaded, so there is no shadow \
             module for PHP yet; once `diff` passes, require {} in place of the original files",
            crate_dir
                .join(crate::codegen_php::wrapper_path(&manifest))
                .display()
        );
    }
    if manifest.language == crate::lang::Lang::Csharp {
        bail!(
            "C# binds calls when it compiles, so there is no shadow module for C# yet; once `diff` \
             passes, compile {} into the project and call the `<namespace>.Rust` classes (or alias \
             them with `using`), with the built library next to the app",
            crate_dir
                .join(crate::codegen_csharp::wrapper_path(&manifest))
                .display()
        );
    }
    if manifest.language == crate::lang::Lang::Ruby {
        let path = match out {
            Some(p) => p.to_path_buf(),
            None => crate_dir
                .join("ruby")
                .join(format!("{}_shadow.rb", manifest.lib_name())),
        };
        if path.exists() {
            bail!("{} exists; remove it first or pass --out", path.display());
        }
        std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
        std::fs::write(
            &path,
            with_kill_switch(
                &crate::codegen_ruby::shadow(&manifest),
                &manifest.crate_name,
            ),
        )?;
        return Ok(ShadowOutcome {
            path,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    if manifest.language == crate::lang::Lang::Go {
        let pkg = crate::codegen_go::package_name(&manifest);
        let path = match out {
            Some(p) => p.to_path_buf(),
            None => crate_dir
                .join("go")
                .join("shadow")
                .join(format!("{pkg}.go")),
        };
        if path.exists() {
            bail!("{} exists; remove it first or pass --out", path.display());
        }
        std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
        let original = crate::harness_go::import_path(&manifest)?;
        std::fs::write(
            &path,
            with_kill_switch(
                &crate::codegen_go::shadow(&manifest, &original),
                &manifest.crate_name,
            ),
        )?;
        let _ = crate::process::output(
            std::process::Command::new("gofmt").arg("-w").arg(&path),
            crate::process::QUICK,
        );
        return Ok(ShadowOutcome {
            path,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    let mut files: Vec<&str> = manifest
        .exports
        .iter()
        .map(|f| f.file.as_str())
        .chain(manifest.js_only.iter().map(|j| j.file.as_str()))
        .chain(manifest.classes.iter().map(|c| c.file.as_str()))
        .collect();
    files.sort_unstable();
    files.dedup();
    let stem = manifest.crate_name.trim_end_matches("-rs");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}.shadow.ts")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    let dir = path.parent().context("output path has no folder")?;
    std::fs::create_dir_all(dir)?;
    let dir = dir.canonical()?;

    let mut code = String::from(
        "// Generated by rust-anything: runs the TS original and the Rust port side by side.\n\
         // RUST_ANYTHING_MODE=old: original only. shadow (default): original result, Rust compared.\n\
         // new: Rust only. Report mismatches with `onMismatch`; counts are in `shadowStats`.\n\
         /* eslint-disable */\n\
         import { createRequire } from 'node:module'\n",
    );
    for (i, file) in files.iter().enumerate() {
        code.push_str(&format!(
            "import * as legacy{i} from '{}'\n",
            relative(&dir, &manifest.root.join(file))
        ));
    }
    code.push_str(&format!(
        "\ntype Mode = 'old' | 'shadow' | 'new'\n\
         export interface Mismatch {{ fn: string, args: unknown[], expected: unknown, actual: unknown }}\n\
         const mode: Mode = ((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow') as Mode\n\
         let native: any = null\n\
         if (mode !== 'old') {{\n\
         \x20 try {{\n\
         \x20   native = createRequire(import.meta.url)('{native}')\n\
         \x20 }}\n\
         \x20 catch (error) {{\n\
         \x20   if (mode === 'new') throw error\n\
         \x20   console.warn('[rust-anything] Rust module not loaded, running the original only:', (error as Error).message)\n\
         \x20 }}\n\
         }}\n\
         export const shadowStats = {{ calls: 0, mismatches: 0 }}\n\
         let report = (m: Mismatch): void => {{\n\
         \x20 if (shadowStats.mismatches <= 10) console.warn('[rust-anything] mismatch in', m.fn, JSON.stringify(enc(m.args)), 'expected', JSON.stringify(m.expected), 'got', JSON.stringify(m.actual))\n\
         }}\n\
         /** Replace the default reporter (console.warn for the first 10 mismatches). */\n\
         export function onMismatch(handler: (m: Mismatch) => void): void {{\n\
         \x20 report = handler\n\
         }}\n\
         {codec}\n\
         function compare(name: string, args: unknown[], expected: unknown, actual: unknown): void {{\n\
         \x20 const want = {{ ok: enc(expected) }}\n\
         \x20 if (!same(want, actual)) {{\n\
         \x20   shadowStats.mismatches++\n\
         \x20   report({{ fn: name, args, expected: want, actual }})\n\
         \x20 }}\n\
         }}\n\
         function check(name: string, args: unknown[], expected: unknown, run: () => unknown): void {{\n\
         \x20 shadowStats.calls++\n\
         \x20 let result: unknown\n\
         \x20 try {{\n\
         \x20   result = run()\n\
         \x20 }}\n\
         \x20 catch (error) {{\n\
         \x20   return compare(name, args, expected, {{ error: String((error as Error)?.message ?? error) }})\n\
         \x20 }}\n\
         \x20 // An async port: compare when it settles; callers never wait for Rust.\n\
         \x20 if (result && typeof (result as any).then === 'function') {{\n\
         \x20   (result as Promise<unknown>).then(\n\
         \x20     value => compare(name, args, expected, {{ ok: enc(value) }}),\n\
         \x20     error => compare(name, args, expected, {{ error: String(error?.message ?? error) }}),\n\
         \x20   )\n\
         \x20   return\n\
         \x20 }}\n\
         \x20 compare(name, args, expected, {{ ok: enc(result) }})\n\
         }}\n\
         function call(name: string, legacy: any, args: unknown[]): any {{\n\
         \x20 if (mode === 'old' || !native) return legacy(...args)\n\
         \x20 if (mode === 'new') return native[name](...args)\n\
         \x20 // The original's result is always what callers get; it runs first and may throw.\n\
         \x20 const expected = legacy(...args)\n\
         \x20 if (expected && typeof (expected as any).then === 'function') {{\n\
         \x20   return (expected as Promise<unknown>).then((value) => {{\n\
         \x20     check(name, args, value, () => native[name](...args))\n\
         \x20     return value\n\
         \x20   }})\n\
         \x20 }}\n\
         \x20 check(name, args, expected, () => native[name](...args))\n\
         \x20 return expected\n\
         }}\n",
        native = relative(&dir, &crate_dir.join("index.cjs")),
        codec = CODEC_JS.trim().replace("const enc = (v, stack = [])", "const enc = (v: any, stack: any[] = []): any")
            .replace("const dec = (v)", "const dec = (v: any): any")
            .replace("const canon = (v, sort)", "const canon = (v: any, sort: boolean): any")
            .replace("const same = (a, b, sort = false)", "const same = (a: unknown, b: unknown, sort = false): boolean"),
    ));
    for f in &manifest.exports {
        let i = files.iter().position(|file| *file == f.file).unwrap_or(0);
        code.push_str(&format!(
            "\nexport const {name}: typeof legacy{i}.{name} = ((...args: unknown[]) => call('{name}', legacy{i}.{name}, args)) as any\n",
            name = f.name
        ));
    }
    for c in &manifest.classes {
        let i = files.iter().position(|file| *file == c.file).unwrap_or(0);
        let methods: String = c
            .methods
            .iter()
            .map(|m| format!("  {name}(...args: any[]): any {{\n    return this.#call('{name}', args)\n  }}\n", name = m.name))
            .collect();
        code.push_str(&format!(
            "\n/** `{name}`: a TS instance and a Rust instance side by side; callers get the TS results. */\n\
             export class {name} {{\n\
             \x20 #legacy: any = null\n\
             \x20 #native: any = null\n\
             \x20 constructor(...args: any[]) {{\n\
             \x20   if (mode === 'new' && native) {{\n\
             \x20     this.#native = new native.{name}(...args)\n\
             \x20     return\n\
             \x20   }}\n\
             \x20   this.#legacy = new (legacy{i} as any).{name}(...args)\n\
             \x20   if (mode === 'old' || !native) return\n\
             \x20   try {{\n\
             \x20     this.#native = new native.{name}(...args)\n\
             \x20   }}\n\
             \x20   catch (error) {{\n\
             \x20     shadowStats.mismatches++\n\
             \x20     report({{ fn: '{name} constructor', args, expected: {{ ok: true }}, actual: {{ error: String((error as Error)?.message ?? error) }} }})\n\
             \x20   }}\n\
             \x20 }}\n\
             {methods}             \x20 #call(method: string, args: unknown[]): any {{\n\
             \x20   if (!this.#legacy) return this.#native[method](...args)\n\
             \x20   if (!this.#native) return this.#legacy[method](...args)\n\
             \x20   const expected = this.#legacy[method](...args)\n\
             \x20   check(`{name}.${{method}}`, args, expected, () => this.#native[method](...args))\n\
             \x20   return expected\n\
             \x20 }}\n\
             }}\n",
            name = c.name
        ));
    }
    for j in &manifest.js_only {
        let i = files.iter().position(|file| *file == j.file).unwrap_or(0);
        code.push_str(&format!(
            "\n/** Stays in TS ({}). */\nexport const {name} = legacy{i}.{name}\n",
            j.reason,
            name = j.name
        ));
    }
    std::fs::write(&path, with_kill_switch(&code, &manifest.crate_name))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(ShadowOutcome {
        path,
        exports: manifest
            .exports
            .iter()
            .map(|f| f.name.clone())
            .chain(manifest.classes.iter().map(|c| format!("class {}", c.name)))
            .collect(),
    })
}

/// An ES import specifier for `to`, relative to the folder `from`.
/// Makes the shadow honor the unit's kill switch, `RUST_ANYTHING_UNIT_<NAME>=original`, before
/// `RUST_ANYTHING_MODE`: the original package can send one unit back on its own.
fn with_kill_switch(text: &str, crate_name: &str) -> String {
    let switch = crate::front_door::kill_switch(crate_name);
    text.replacen(
        "((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow')",
        &format!(
            "((globalThis as any).process?.env?.{switch} === 'original' ? 'old' : ((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow'))"
        ),
        1,
    )
    .replacen(
        "_MODE = os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")",
        &format!(
            "_MODE = \"old\" if os.environ.get(\"{switch}\") == \"original\" else os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")"
        ),
        1,
    )
    .replacen(
        "MODE = ENV.fetch(\"RUST_ANYTHING_MODE\", \"shadow\")",
        &format!(
            "MODE = ENV[\"{switch}\"] == \"original\" ? \"old\" : ENV.fetch(\"RUST_ANYTHING_MODE\", \"shadow\")"
        ),
        1,
    )
    .replacen(
        "var mode = func() string {\n",
        &format!(
            "var mode = func() string {{\n\tif os.Getenv(\"{switch}\") == \"original\" {{\n\t\treturn \"old\"\n\t}}\n"
        ),
        1,
    )
    .replace(
        "System.getProperty(\"rust-anything.mode\", \"shadow\")",
        &format!(
            "(\"original\".equals(System.getenv(\"{switch}\")) ? \"old\" : System.getProperty(\"rust-anything.mode\", \"shadow\"))"
        ),
    )
}

pub(crate) fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<Component> = from.components().collect();
    let to_parts: Vec<Component> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_string(), from.len() - common).collect();
    parts.extend(
        to_parts[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    let joined = parts.join("/");
    if joined.starts_with("..") {
        joined
    } else {
        format!("./{joined}")
    }
}

/// The Python shadow module: same functions and classes, original first, Rust compared.
fn shadow_python(
    crate_dir: &Path,
    manifest: &Manifest,
    out: Option<&Path>,
) -> Result<ShadowOutcome> {
    let stem = manifest
        .crate_name
        .trim_end_matches("-rs")
        .replace('-', "_");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}_shadow.py")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
    let mut files: Vec<&str> = manifest
        .exports
        .iter()
        .map(|f| f.file.as_str())
        .chain(manifest.classes.iter().map(|c| c.file.as_str()))
        .chain(manifest.js_only.iter().map(|j| j.file.as_str()))
        .collect();
    files.sort_unstable();
    files.dedup();
    let module_of = |file: &str| files.iter().position(|f| *f == file).unwrap_or(0);
    let sources = format!(
        "[{}]",
        files
            .iter()
            .map(|f| format!("{:?}", manifest.root.join(f).to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut code = format!(
        "\"\"\"Generated by rust-anything: runs the Python original and the Rust port side by side.\n\n\
         RUST_ANYTHING_MODE=old: original only. shadow (default): original result, Rust compared.\n\
         new: Rust only. Report mismatches with `on_mismatch`; counts are in `stats`.\n\"\"\"\n\
         import importlib\nimport json\nimport math\nimport os\nimport sys\n\n\
         _MODE = os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")\n\
         _SOURCES = {sources}\n\
         _PACKAGE_DIR = {package_dir:?}\n{driver_helpers}\n\
         _legacy = [load(p) for p in _SOURCES]\n\
         _native = None\n\
         if _MODE != \"old\":\n    try:\n        sys.path.insert(0, _PACKAGE_DIR)\n        _native = importlib.import_module({lib:?})\n\
         \x20   except ImportError as error:\n        if _MODE == \"new\":\n            raise\n\
         \x20       print(\"[rust-anything] Rust module not loaded, running the original only:\", error, file=sys.stderr)\n\n\
         stats = {{\"calls\": 0, \"mismatches\": 0}}\n\n\n\
         def _default_report(m):\n    if stats[\"mismatches\"] <= 10:\n        print(\"[rust-anything] mismatch in\", m[\"fn\"], m, file=sys.stderr)\n\n\n\
         _report = _default_report\n\n\n\
         def on_mismatch(handler):\n    \"\"\"Replace the default reporter (stderr for the first 10 mismatches).\"\"\"\n    global _report\n    _report = handler\n\n\n\
         def _check(name, args, expected, run):\n    stats[\"calls\"] += 1\n    try:\n        actual = {{\"ok\": enc(run())}}\n\
         \x20   except (KeyboardInterrupt, SystemExit):\n        raise\n    except BaseException as e:  # noqa: BLE001\n        actual = {{\"error\": error_text(e)}}\n\
         \x20   want = {{\"ok\": enc(expected)}}\n    if canon(want, False) != canon(actual, False):\n\
         \x20       stats[\"mismatches\"] += 1\n        _report({{\"fn\": name, \"args\": args, \"expected\": want, \"actual\": actual}})\n\n\n\
         def _call(module, name, args, kwargs):\n    legacy = getattr(_legacy[module], name)\n    if _native is None:\n        return legacy(*args, **kwargs)\n\
         \x20   if _MODE == \"new\":\n        return getattr(_native, name)(*args, **kwargs)\n    expected = legacy(*args, **kwargs)\n\
         \x20   _check(name, list(args), expected, lambda: getattr(_native, name)(*args, **kwargs))\n    return expected\n",
        sources = sources,
        package_dir = crate_dir.join("python").to_string_lossy(),
        lib = manifest.lib_name(),
        driver_helpers = PY_HELPERS,
    );
    for f in &manifest.exports {
        code.push_str(&format!(
            "\n\ndef {name}(*args, **kwargs):\n    return _call({m}, {name:?}, args, kwargs)\n",
            name = f.name,
            m = module_of(&f.file)
        ));
    }
    for c in &manifest.classes {
        let methods: String = c
            .methods
            .iter()
            .map(|m| {
                format!(
                    "\n    def {name}(self, *args, **kwargs):\n        return self._call({name:?}, args, kwargs)\n",
                    name = m.name
                )
            })
            .collect();
        code.push_str(&format!(
            "\n\nclass {name}:\n    \"\"\"A Python instance and a Rust instance side by side; callers get the Python results.\"\"\"\n\n\
             \x20   def __init__(self, *args, **kwargs):\n        self._legacy = None\n        self._native = None\n\
             \x20       if _MODE == \"new\" and _native is not None:\n            self._native = _native.{name}(*args, **kwargs)\n            return\n\
             \x20       self._legacy = getattr(_legacy[{m}], {name:?})(*args, **kwargs)\n        if _native is None or _MODE == \"old\":\n            return\n\
             \x20       try:\n            self._native = _native.{name}(*args, **kwargs)\n        except (KeyboardInterrupt, SystemExit):\n            raise\n        except BaseException as e:  # noqa: BLE001\n\
             \x20           stats[\"mismatches\"] += 1\n            _report({{\"fn\": \"{name} constructor\", \"args\": list(args), \"expected\": {{\"ok\": True}}, \"actual\": {{\"error\": error_text(e)}}}})\n\n\
             \x20   def _call(self, method, args, kwargs):\n        if self._legacy is None:\n            return getattr(self._native, method)(*args, **kwargs)\n\
             \x20       if self._native is None:\n            return getattr(self._legacy, method)(*args, **kwargs)\n\
             \x20       expected = getattr(self._legacy, method)(*args, **kwargs)\n\
             \x20       _check(f\"{name}.{{method}}\", list(args), expected, lambda: getattr(self._native, method)(*args, **kwargs))\n        return expected\n{methods}",
            name = c.name,
            m = module_of(&c.file)
        ));
    }
    for j in &manifest.js_only {
        code.push_str(&format!(
            "\n\n# Stays in Python ({}).\n{name} = _legacy[{m}].{name}\n",
            j.reason,
            name = j.name,
            m = module_of(&j.file)
        ));
    }
    std::fs::write(&path, with_kill_switch(&code, &manifest.crate_name))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(ShadowOutcome {
        path,
        exports: manifest
            .exports
            .iter()
            .map(|f| f.name.clone())
            .chain(manifest.classes.iter().map(|c| format!("class {}", c.name)))
            .collect(),
    })
}

/// `enc`, `canon`, `load` and `error_text` from the Python driver, for the shadow module.
const PY_HELPERS: &str = {
    // Kept in one place: the driver defines them first, before its entry points.
    r#"
BIG = 2 ** 53


def enc(v, stack=()):
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, int):
        return v if -BIG <= v <= BIG else {"$int": str(v)}
    if isinstance(v, float):
        if math.isnan(v):
            return {"$num": "NaN"}
        if math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and math.copysign(1, v) < 0:
            return {"$num": "-0"}
        return {"$float": repr(v)} if v.is_integer() else v
    if any(v is s for s in stack):
        return {"$cycle": True}
    stack = stack + (v,)
    if isinstance(v, list):
        return [enc(x, stack) for x in v]
    if isinstance(v, tuple):
        return {"$tuple": [enc(x, stack) for x in v]}
    if isinstance(v, (set, frozenset)):
        return {"$set": sorted((enc(x, stack) for x in v), key=json.dumps)}
    if isinstance(v, (bytes, bytearray)):
        return {"$bytes": list(v)}
    if isinstance(v, dict):
        if all(isinstance(k, str) for k in v):
            return {k: enc(x, stack) for k, x in v.items()}
        return {"$dict": [[enc(k, stack), enc(x, stack)] for k, x in v.items()]}
    if hasattr(v, "__dict__"):
        return {"$object": type(v).__name__, "attrs": enc(vars(v), stack)}
    return {"$repr": type(v).__name__}


def canon(v, sort):
    return json.dumps(v, sort_keys=sort, ensure_ascii=False)


def error_text(e):
    return f"{type(e).__name__}: {e}"


def load(path):
    path = os.path.abspath(path)
    directory, filename = os.path.split(path)
    parts = [os.path.splitext(filename)[0]]
    while os.path.exists(os.path.join(directory, "__init__.py")):
        directory, package = os.path.split(directory)
        parts.insert(0, package)
    if parts[-1] == "__init__":
        parts.pop()
    if directory not in sys.path:
        sys.path.insert(0, directory)
    return importlib.import_module(".".join(parts))
"#
};

/// C: a shadow translation unit, and the `-D` flags that rename the originals beside it.
fn shadow_c(crate_dir: &Path, manifest: &Manifest, out: Option<&Path>) -> Result<ShadowOutcome> {
    let stem = manifest
        .crate_name
        .trim_end_matches("-rs")
        .replace('-', "_");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}_shadow.c")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    let dir = path.parent().context("output path has no folder")?;
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, crate::codegen_c::shadow_c(manifest))?;
    std::fs::write(
        dir.join("flags.txt"),
        crate::codegen_c::shadow_flags(manifest),
    )?;
    Ok(ShadowOutcome {
        path,
        exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::relative;
    use std::path::Path;

    #[test]
    fn builds_relative_specifiers() {
        assert_eq!(
            relative(Path::new("/r/rust/x/shadow"), Path::new("/r/src/a.ts")),
            "../../../src/a.ts"
        );
        assert_eq!(
            relative(Path::new("/r/rust/x"), Path::new("/r/rust/x/index.cjs")),
            "./index.cjs"
        );
    }
}
