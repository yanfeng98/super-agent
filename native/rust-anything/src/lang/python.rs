//! Python: packages, imports, risks, the PyPI-to-crate table, and exported functions and
//! classes with type hints mapped into the TS-like type dialect.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use anyhow::Result;
use tree_sitter::Node;

use super::syntax::{self, named_children, text};
use crate::risks::{Risk, regex_needs_fancy};
use crate::ts::deps::CrateMatch;
use crate::ts::discover::{FileKind, Package, SourceFile, Workspace};
use crate::ts::exports::{ExportedClass, ExportedFn, Param};
use crate::ts::scan::{FileFacts, Import};

fn language() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "venv",
    ".venv",
    "env",
    "__pycache__",
    "build",
    "dist",
    "site-packages",
    ".tox",
    ".nox",
    "target",
    "fixtures",
    "examples",
    "docs",
];

// ---- discovery -------------------------------------------------------------------------------

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut manifests = Vec::new();
    let mut files = Vec::new();
    let walker = crate::walk::filtered(root, |e| {
        !(e.file_type().is_some_and(|t| t.is_dir())
            && SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
    })
    .hidden(true)
    .require_git(false)
    .build();
    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let name = rel.rsplit('/').next().unwrap_or(&rel);
        if matches!(name, "pyproject.toml" | "setup.py" | "setup.cfg") {
            let dir = rel
                .rsplit_once('/')
                .map_or(String::new(), |(d, _)| d.to_string());
            if !manifests.contains(&dir) {
                manifests.push(dir);
            }
        } else if name.ends_with(".py") && name != "setup.py" && name != "conftest.py" {
            let is_test = name.starts_with("test_")
                || name.ends_with("_test.py")
                || rel
                    .split('/')
                    .any(|p| matches!(p, "tests" | "test" | "testing"));
            files.push((
                rel,
                if is_test {
                    FileKind::Test
                } else {
                    FileKind::Source
                },
            ));
        }
    }
    let mut packages: Vec<Package> = manifests
        .iter()
        .map(|dir| read_package(root, dir))
        .collect();
    if !packages.iter().any(|p| p.dir.is_empty()) {
        packages.push(read_package(root, ""));
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let files = files
        .into_iter()
        .map(|(path, kind)| {
            let package = packages
                .iter()
                .enumerate()
                .filter(|(_, p)| p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)))
                .max_by_key(|(_, p)| p.dir.len())
                .map_or(0, |(i, _)| i);
            SourceFile {
                path,
                package,
                kind,
            }
        })
        .collect();
    Ok(Workspace { packages, files })
}

fn read_package(root: &Path, dir: &str) -> Package {
    let join = |name: &str| {
        if dir.is_empty() {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        }
    };
    let mut name = None;
    let mut deps = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join(join("pyproject.toml")))
        && let Ok(doc) = text.parse::<toml_edit::DocumentMut>()
    {
        name = doc
            .get("project")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(normalized_name);
        if let Some(list) = doc
            .get("project")
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_array())
        {
            deps.extend(
                list.iter()
                    .filter_map(|d| d.as_str())
                    .filter_map(requirement_name),
            );
        }
        if let Some(t) = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_table_like())
        {
            deps.extend(
                t.iter()
                    .map(|(k, _)| normalized_name(k))
                    .filter(|k| k != "python"),
            );
        }
    }
    requirements_from(
        &root.join(join("requirements.txt")),
        &mut deps,
        &mut BTreeSet::new(),
    );
    let src = join("src");
    Package {
        name: name.unwrap_or_else(|| {
            if dir.is_empty() {
                root.file_name()
                    .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
            } else {
                dir.to_string()
            }
        }),
        src_root: if root.join(&src).is_dir() {
            src
        } else {
            dir.to_string()
        },
        dir: dir.to_string(),
        deps,
        dev_deps: Vec::new(),
        namespaces: Vec::new(),
    }
}

/// `requests>=2.0; python_version>"3"` -> `requests`.
/// `None` when the line points at a VCS checkout, an archive URL or a local file. Those are not
/// packages a registry serves: reading the first run of letters names `git+https://...` as `git`
/// and `./vendor/foo.whl` as `.`.
fn requirement_name(spec: &str) -> Option<String> {
    let spec = spec.trim();
    let name: String = spec
        .chars()
        .take_while(|c| c.is_alphanumeric() || "-_.".contains(*c))
        .collect();
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return None;
    }
    let rest = spec[name.len()..].trim_start();
    // A trailing `# comment` after the name leaves the name intact.
    let carried = rest.is_empty()
        || rest.starts_with(['[', '(', '=', '<', '>', '!', '~', ';', ',', '@', '#']);
    carried.then(|| normalized_name(&name))
}

/// The packages a requirements file names, following `-r` / `--requirement` includes: a file that
/// is reached twice is read once, and a cycle stops instead of recursing. `pip` resolves an include
/// against the directory of the file that names it, not the project root.
fn requirements_from(file: &Path, out: &mut Vec<String>, seen: &mut BTreeSet<std::path::PathBuf>) {
    let key = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    if !seen.insert(key) {
        return;
    }
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let dir = file.parent().unwrap_or(Path::new(""));
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("--requirement")
            .or_else(|| line.strip_prefix("-r"))
        {
            let included = rest.trim_start_matches(['=', ' ', '\t']);
            if !included.is_empty() {
                requirements_from(&dir.join(included), out, seen);
            }
            continue;
        }
        if line.is_empty() || line.starts_with(['#', '-']) {
            continue;
        }
        out.extend(requirement_name(line));
    }
}

/// A distribution name as pip compares it (PEP 503): `Core_Lib.x` -> `core-lib-x`.
fn normalized_name(name: &str) -> String {
    name.split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

// ---- scanning --------------------------------------------------------------------------------

pub fn scan(source: &str) -> FileFacts {
    let mut facts = FileFacts {
        loc: syntax::loc(source),
        ..FileFacts::default()
    };
    let Some(tree) = syntax::parse(&language(), source) else {
        return facts;
    };
    let root = tree.root_node();
    facts.parse_errors = usize::from(root.has_error());
    let add = |facts: &mut FileFacts, risk: Risk, node: Node| {
        facts
            .risks
            .entry(risk)
            .or_default()
            .push(syntax::line(node));
    };
    syntax::walk(root, |node| match node.kind() {
        "import_statement" => {
            for child in named_children(node) {
                let name = child.child_by_field_name("name").unwrap_or(child);
                facts.imports.push(Import {
                    specifier: text(name, source).to_string(),
                    type_only: false,
                    dynamic: false,
                });
            }
        }
        "import_from_statement" => {
            let Some(module) = node.child_by_field_name("module_name") else {
                return;
            };
            let module = text(module, source).to_string();
            facts.imports.push(Import {
                specifier: module.clone(),
                type_only: false,
                dynamic: false,
            });
            // `from pkg import sub` may name a submodule.
            let mut cursor = node.walk();
            for name in node.children_by_field_name("name", &mut cursor) {
                let name = name.child_by_field_name("name").unwrap_or(name);
                let sep = if module.ends_with('.') { "" } else { "." };
                facts.imports.push(Import {
                    specifier: format!("{module}{sep}{}", text(name, source)),
                    type_only: true,
                    dynamic: false,
                });
            }
        }
        "call" => {
            let Some(function) = node.child_by_field_name("function") else {
                return;
            };
            let callee = text(function, source);
            match callee {
                "eval" | "exec" | "compile" | "__import__" | "importlib.import_module" => {
                    add(&mut facts, Risk::DynamicCode, node)
                }
                "getattr" | "setattr" | "hasattr" | "delattr" | "isinstance" | "type" => {
                    add(&mut facts, Risk::DynamicTyping, node);
                }
                "globals" | "locals" | "vars" => add(&mut facts, Risk::Reflection, node),
                "time.time"
                | "time.time_ns"
                | "datetime.now"
                | "datetime.utcnow"
                | "datetime.today"
                | "date.today"
                | "datetime.datetime.now"
                | "datetime.datetime.utcnow"
                | "datetime.date.today"
                | "uuid.uuid4"
                | "os.urandom" => add(&mut facts, Risk::ClockAndRandom, node),
                _ if callee.starts_with("random.") || callee.starts_with("secrets.") => {
                    add(&mut facts, Risk::ClockAndRandom, node);
                }
                "set" | "frozenset" => add(&mut facts, Risk::UnorderedIteration, node),
                "threading.Thread"
                | "Thread"
                | "ThreadPoolExecutor"
                | "ProcessPoolExecutor"
                | "asyncio.gather" => {
                    add(&mut facts, Risk::Concurrency, node);
                }
                _ if callee.starts_with("re.") => {
                    let pattern = node
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next())
                        .filter(|a| a.kind() == "string");
                    match pattern {
                        Some(p) if regex_needs_fancy(text(p, source), "") => {
                            add(&mut facts, Risk::RegexSyntax, node)
                        }
                        Some(_) => {}
                        None => add(&mut facts, Risk::DynamicRegex, node),
                    }
                }
                _ if callee.ends_with(".find")
                    || callee.ends_with(".index")
                    || callee.ends_with(".rfind") =>
                {
                    add(&mut facts, Risk::StringOffsets, node);
                }
                _ => {}
            }
        }
        "subscript" if named_children(node).iter().any(|c| c.kind() == "slice") => {
            add(&mut facts, Risk::StringOffsets, node)
        }
        "binary_operator" | "augmented_assignment" => {
            let op = node
                .child_by_field_name("operator")
                .map_or("", |o| text(o, source));
            match op {
                "//" | "%" | "//=" | "%=" => add(&mut facts, Risk::DivisionSemantics, node),
                "**" | "**=" | "<<" | "<<=" => add(&mut facts, Risk::IntegerSemantics, node),
                "&" | "|" | "^" | ">>" => add(&mut facts, Risk::NumberSemantics, node),
                _ => {}
            }
        }
        "set" | "set_comprehension" => add(&mut facts, Risk::UnorderedIteration, node),
        "decorator" if crate::risks::compiled_by(text(node, source)).is_some() => {
            add(&mut facts, Risk::AlreadyCompiled, node)
        }
        "typed_parameter" | "typed_default_parameter"
            if node.child_by_field_name("type").is_some_and(|t| {
                let t = text(t, source);
                t.contains("ndarray") || t.contains("NDArray")
            }) =>
        {
            add(&mut facts, Risk::ArrayParameters, node)
        }
        "raise_statement" | "try_statement" => add(&mut facts, Risk::Exceptions, node),
        "yield" => add(&mut facts, Risk::Generators, node),
        "await" => add(&mut facts, Risk::Async, node),
        "for_statement"
        | "while_statement"
        | "list_comprehension"
        | "dictionary_comprehension"
        | "generator_expression" => {
            facts.loops += 1;
        }
        "class_definition" if node.child_by_field_name("superclasses").is_some() => {
            add(&mut facts, Risk::ClassInheritance, node)
        }
        "function_definition" => {
            if text(node, source).starts_with("async") {
                add(&mut facts, Risk::Async, node);
            }
            if node.child_by_field_name("parameters").is_some_and(|p| {
                named_children(p).iter().any(|c| {
                    c.kind() == "identifier"
                        && text(*c, source) != "self"
                        && text(*c, source) != "cls"
                })
            }) {
                add(&mut facts, Risk::DynamicTyping, node);
            }
        }
        "assignment" => {
            // `Class.method = ...`, `module.attr = ...`: rewriting code at runtime.
            let left = node
                .child_by_field_name("left")
                .map_or("", |l| text(l, source));
            if left.contains('.')
                && !left.starts_with("self.")
                && left.chars().next().is_some_and(char::is_uppercase)
            {
                add(&mut facts, Risk::Monkeypatching, node);
            }
        }
        _ => {}
    });
    facts
}

/// Where a Python import points.
pub enum Resolution {
    File(usize),
    External(String),
    Stdlib,
    Unresolved,
}

/// Resolves `pkg.mod`, `.mod` and `..mod` against the files of the workspace.
pub fn resolve(from: &str, spec: &str, index: &HashMap<&str, usize>, ws: &Workspace) -> Resolution {
    let try_module = |base: &str| -> Option<usize> {
        let base = base.trim_matches('/');
        [format!("{base}.py"), format!("{base}/__init__.py")]
            .iter()
            .find_map(|c| index.get(c.as_str()).copied())
    };
    if spec.starts_with('.') {
        let dots = spec.chars().take_while(|c| *c == '.').count();
        let mut dir: Vec<&str> = from.split('/').collect();
        dir.pop();
        for _ in 1..dots {
            dir.pop();
        }
        let rest = spec[dots..].replace('.', "/");
        let base = if rest.is_empty() {
            dir.join("/")
        } else {
            format!("{}/{rest}", dir.join("/"))
        };
        return try_module(&base).map_or(Resolution::Unresolved, Resolution::File);
    }
    let path = spec.replace('.', "/");
    let roots: BTreeSet<&str> = ws
        .packages
        .iter()
        .map(|p| p.src_root.as_str())
        .chain([""])
        .collect();
    for root in roots {
        let base = if root.is_empty() {
            path.clone()
        } else {
            format!("{root}/{path}")
        };
        if let Some(i) = try_module(&base) {
            return Resolution::File(i);
        }
    }
    let segments: Vec<&str> = spec.split('.').collect();
    if let Some(known) = (2..=segments.len())
        .rev()
        .map(|k| segments[..k].join("."))
        .find(|name| lookup(name).is_some())
    {
        return Resolution::External(known);
    }
    let top = segments[0];
    if STDLIB.contains(&top) {
        Resolution::Stdlib
    } else if index
        .keys()
        .any(|p| p.starts_with(&format!("{top}/")) || p.contains(&format!("/{top}/")))
    {
        // A local package imported by a shorter path than its files: still local.
        Resolution::Unresolved
    } else {
        Resolution::External(top.to_lowercase().replace('_', "-"))
    }
}

// The version-agnostic list includes Python 3.12 modules removed in 3.13;
// imports of their backports can therefore also resolve as stdlib.
const STDLIB: &[&str] = &[
    "__future__",
    "__main__",
    "_abc",
    "_aix_support",
    "_ast",
    "_asyncio",
    "_bisect",
    "_blake2",
    "_bz2",
    "_codecs",
    "_codecs_cn",
    "_codecs_hk",
    "_codecs_iso2022",
    "_codecs_jp",
    "_codecs_kr",
    "_codecs_tw",
    "_collections",
    "_collections_abc",
    "_compat_pickle",
    "_compression",
    "_contextvars",
    "_crypt",
    "_csv",
    "_ctypes",
    "_curses",
    "_curses_panel",
    "_datetime",
    "_dbm",
    "_decimal",
    "_elementtree",
    "_frozen_importlib",
    "_frozen_importlib_external",
    "_functools",
    "_gdbm",
    "_hashlib",
    "_heapq",
    "_imp",
    "_io",
    "_json",
    "_locale",
    "_lsprof",
    "_lzma",
    "_markupbase",
    "_md5",
    "_msi",
    "_multibytecodec",
    "_multiprocessing",
    "_opcode",
    "_operator",
    "_osx_support",
    "_overlapped",
    "_pickle",
    "_posixshmem",
    "_posixsubprocess",
    "_py_abc",
    "_pydatetime",
    "_pydecimal",
    "_pyio",
    "_pylong",
    "_queue",
    "_random",
    "_scproxy",
    "_sha1",
    "_sha2",
    "_sha3",
    "_signal",
    "_sitebuiltins",
    "_socket",
    "_sqlite3",
    "_sre",
    "_ssl",
    "_stat",
    "_statistics",
    "_string",
    "_strptime",
    "_struct",
    "_symtable",
    "_thread",
    "_threading_local",
    "_tkinter",
    "_tokenize",
    "_tracemalloc",
    "_typing",
    "_uuid",
    "_warnings",
    "_weakref",
    "_weakrefset",
    "_winapi",
    "_wmi",
    "_zoneinfo",
    "abc",
    "aifc",
    "antigravity",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "audioop",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cgi",
    "cgitb",
    "chunk",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "crypt",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "genericpath",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "grp",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "idlelib",
    "imaplib",
    "imghdr",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "mailcap",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "msilib",
    "msvcrt",
    "multiprocessing",
    "netrc",
    "nis",
    "nntplib",
    "nt",
    "ntpath",
    "nturl2path",
    "numbers",
    "opcode",
    "operator",
    "optparse",
    "os",
    "ossaudiodev",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pipes",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "posix",
    "posixpath",
    "pprint",
    "profile",
    "pstats",
    "pty",
    "pwd",
    "py_compile",
    "pyclbr",
    "pydoc",
    "pydoc_data",
    "pyexpat",
    "queue",
    "quopri",
    "random",
    "re",
    "readline",
    "reprlib",
    "resource",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "sndhdr",
    "socket",
    "socketserver",
    "spwd",
    "sqlite3",
    "sre_compile",
    "sre_constants",
    "sre_parse",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "sunau",
    "symtable",
    "sys",
    "sysconfig",
    "syslog",
    "tabnanny",
    "tarfile",
    "telnetlib",
    "tempfile",
    "termios",
    "test",
    "textwrap",
    "this",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "tomllib",
    "trace",
    "traceback",
    "tracemalloc",
    "tty",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "typing_extensions",
    "unicodedata",
    "unittest",
    "urllib",
    "uu",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "winreg",
    "winsound",
    "wsgiref",
    "xdrlib",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];

// ---- dependencies ----------------------------------------------------------------------------

/// The Rust counterpart of a PyPI package (or import name).
pub fn lookup(package: &str) -> Option<CrateMatch> {
    crate::crates::find("python", package)
}

// ---- exports ---------------------------------------------------------------------------------

/// Decorators that keep a function's behavior as seen by callers.
const SAFE_DECORATORS: &[&str] = &[
    "cache",
    "lru_cache",
    "functools.cache",
    "functools.lru_cache",
    "staticmethod",
];

/// Public module-level functions and classes, honoring `__all__`.
pub fn exports(path: &str, source: &str) -> (Vec<ExportedFn>, Vec<ExportedClass>) {
    let Some(tree) = syntax::parse(&language(), source) else {
        return (Vec::new(), Vec::new());
    };
    let root = tree.root_node();
    let all = dunder_all(root, source);
    let public = |name: &str| {
        all.as_ref().map_or(!name.starts_with('_'), |names| {
            names.iter().any(|n| n == name)
        })
    };
    let entry = entry_calls(root, source, path.ends_with("__main__.py"));
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    for item in named_children(root) {
        let (decorators, definition) = unwrap_decorated(item, source);
        match definition.kind() {
            "function_definition" => {
                let Some(mut f) = function(path, definition, source, &decorators, false) else {
                    continue;
                };
                let parser = definition
                    .child_by_field_name("return_type")
                    .is_some_and(|t| text(t, source).ends_with("ArgumentParser"));
                if parser || entry.iter().any(|n| *n == f.name) {
                    f.blocker = Some(
                        "a command-line entry point: it stays in Python, and `app front-door` \
                         takes the command line over when the whole app moves"
                            .into(),
                    );
                }
                if public(&f.name) {
                    functions.push(f);
                }
            }
            "class_definition" => {
                let Some(c) = class(path, definition, source, &decorators) else {
                    continue;
                };
                if public(&c.name) {
                    classes.push(c);
                }
            }
            _ => {}
        }
    }
    (functions, classes)
}

/// Dataclasses declared at the top level of a file, as types `record` can build values of:
/// `{ $object: "Name"; attrs: { field: type; defaulted?: type } }`, which the Python driver
/// turns into an instance of the class.
pub fn dataclass_types(source: &str) -> Vec<crate::ts::exports::LocalType> {
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in named_children(tree.root_node()) {
        let (decorators, definition) = unwrap_decorated(item, source);
        if definition.kind() != "class_definition"
            || !decorators.iter().any(|d| d.contains("dataclass"))
        {
            continue;
        }
        let Some(name) = definition.child_by_field_name("name") else {
            continue;
        };
        let mut fields = Vec::new();
        if let Some(body) = definition.child_by_field_name("body") {
            for statement in named_children(body) {
                let Some(assignment) = named_children(statement)
                    .into_iter()
                    .find(|n| n.kind() == "assignment")
                else {
                    continue;
                };
                let (Some(left), Some(ty)) = (
                    assignment.child_by_field_name("left"),
                    assignment.child_by_field_name("type"),
                ) else {
                    continue;
                };
                let hint = text(ty, source);
                if hint.starts_with("ClassVar") || hint.starts_with("typing.ClassVar") {
                    continue;
                }
                let optional = if assignment.child_by_field_name("right").is_some() {
                    "?"
                } else {
                    ""
                };
                fields.push(format!(
                    "{}{optional}: {}",
                    text(left, source),
                    to_ts_type(hint)
                ));
            }
        }
        let name = text(name, source).to_string();
        out.push(crate::ts::exports::LocalType {
            text: format!(
                "{{ $object: {name:?}; attrs: {{ {} }} }}",
                fields.join("; ")
            ),
            name,
            alias: true,
        });
    }
    out
}

/// Functions the file runs as a program: the calls inside `if __name__ == "__main__":`, or
/// every top-level call in a package's `__main__.py`.
fn entry_calls<'s>(root: Node, source: &'s str, main_module: bool) -> Vec<&'s str> {
    let mut out = Vec::new();
    let mut collect = |node: Node| {
        syntax::walk(node, |n| {
            if n.kind() == "call"
                && let Some(f) = n.child_by_field_name("function")
                && f.kind() == "identifier"
            {
                out.push(text(f, source));
            }
        });
    };
    for statement in named_children(root) {
        let guard = statement.kind() == "if_statement"
            && statement.child_by_field_name("condition").is_some_and(|c| {
                let c = text(c, source).replace(' ', "").replace('\'', "\"");
                c == "__name__==\"__main__\"" || c == "\"__main__\"==__name__"
            });
        if guard {
            if let Some(body) = statement.child_by_field_name("consequence") {
                collect(body);
            }
        } else if main_module && statement.kind() == "expression_statement" {
            collect(statement);
        }
    }
    out
}

/// The names a module's `__all__` lists, when its top level sets it from string literals only:
/// `__all__ = [...]` (with or without a type; the last one wins) and `__all__ += [...]`. `None`
/// when there is no `__all__`, or when it is built or changed any other way (`+ other.__all__`,
/// `sorted(...)`, `__all__.extend(...)`, inside an `if` or `try` block), so the caller falls
/// back to the names without a leading underscore instead of trusting a partial list.
fn dunder_all(root: Node, source: &str) -> Option<Vec<String>> {
    let mut names: Option<Vec<String>> = None;
    for statement in named_children(root) {
        match statement.kind() {
            "expression_statement" => {}
            // Their bodies are not the module's top level.
            "function_definition" | "class_definition" | "decorated_definition" => continue,
            // A change inside a block (`if sys.platform == ...: __all__ += [...]`) cannot be
            // followed.
            _ if mentions_all(statement, source) => return None,
            _ => continue,
        }
        for node in named_children(statement) {
            let left = node.child_by_field_name("left").map(|l| text(l, source));
            match node.kind() {
                "assignment" | "augmented_assignment" if left == Some("__all__") => {
                    // `__all__: list[str]` alone declares it without a value.
                    let Some(right) = node.child_by_field_name("right") else {
                        continue;
                    };
                    let listed = string_literals(right, source)?;
                    match node
                        .child_by_field_name("operator")
                        .map(|o| text(o, source))
                    {
                        None => names = Some(listed),
                        Some("+=") => names.get_or_insert_with(Vec::new).extend(listed),
                        Some(_) => return None,
                    }
                }
                _ if mentions_all(node, source) => return None,
                _ => {}
            }
        }
    }
    names
}

/// The values of a list or tuple made only of plain string literals (comments allowed).
fn string_literals(node: Node, source: &str) -> Option<Vec<String>> {
    if !matches!(node.kind(), "list" | "tuple" | "expression_list") {
        return None;
    }
    let mut out = Vec::new();
    for item in named_children(node) {
        match item.kind() {
            "comment" => {}
            "string" => {
                let mut content = String::new();
                for part in named_children(item) {
                    match part.kind() {
                        // Bytes are not names.
                        "string_start" if text(part, source).contains(['b', 'B']) => return None,
                        "string_start" | "string_end" => {}
                        // Escape sequences would need decoding.
                        "string_content" if part.named_child_count() > 0 => return None,
                        "string_content" => content.push_str(text(part, source)),
                        _ => return None,
                    }
                }
                out.push(content);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Whether the module's own `__all__` appears inside `node`; `other.__all__` and a keyword
/// argument named `__all__` do not count.
fn mentions_all(node: Node, source: &str) -> bool {
    match node.kind() {
        "identifier" => text(node, source) == "__all__",
        "attribute" => node
            .child_by_field_name("object")
            .is_some_and(|object| mentions_all(object, source)),
        "keyword_argument" => node
            .child_by_field_name("value")
            .is_some_and(|value| mentions_all(value, source)),
        _ => named_children(node)
            .into_iter()
            .any(|child| mentions_all(child, source)),
    }
}

fn unwrap_decorated<'t>(node: Node<'t>, source: &str) -> (Vec<String>, Node<'t>) {
    if node.kind() != "decorated_definition" {
        return (Vec::new(), node);
    }
    let decorators = named_children(node)
        .into_iter()
        .filter(|n| n.kind() == "decorator")
        .map(|d| {
            let t = text(d, source).trim_start_matches('@').trim();
            t.split('(').next().unwrap_or(t).to_string()
        })
        .collect();
    (
        decorators,
        node.child_by_field_name("definition").unwrap_or(node),
    )
}

/// Unannotated parameters the body uses as numbers (`r - g`, `x * 0.5`, `max(r, g)`, `x < 1`)
/// become `number`: generated inputs then fit what the code expects.
pub fn infer_numbers(params: &mut [Param], body: Node, source: &str) {
    let unknown: Vec<String> = params
        .iter()
        .filter(|p| p.ts_type == "unknown" && p.default.is_none())
        .map(|p| p.name.clone())
        .collect();
    if unknown.is_empty() {
        return;
    }
    let mut numeric: Vec<String> = Vec::new();
    let is_number = |n: Node| matches!(n.kind(), "integer" | "float");
    syntax::walk(body, |n| {
        let operands: Vec<Node> = match n.kind() {
            "binary_operator" => {
                let op = n
                    .child_by_field_name("operator")
                    .map_or("", |o| text(o, source));
                let (Some(l), Some(r)) = (
                    n.child_by_field_name("left"),
                    n.child_by_field_name("right"),
                ) else {
                    return;
                };
                match op {
                    "-" | "*" | "/" | "//" | "**" => vec![l, r],
                    "+" | "%" if is_number(l) || is_number(r) => vec![l, r],
                    _ => return,
                }
            }
            "comparison_operator" => {
                let parts = named_children(n);
                if !parts.iter().any(|p| is_number(*p)) {
                    return;
                }
                parts
            }
            "call" => {
                let f = n
                    .child_by_field_name("function")
                    .map_or("", |f| text(f, source));
                if !matches!(
                    f,
                    "min" | "max" | "abs" | "round" | "float" | "int" | "divmod"
                ) && !f.starts_with("math.")
                {
                    return;
                }
                n.child_by_field_name("arguments")
                    .map(named_children)
                    .unwrap_or_default()
            }
            _ => return,
        };
        for o in operands {
            if o.kind() == "identifier" {
                let name = text(o, source).to_string();
                if unknown.contains(&name) && !numeric.contains(&name) {
                    numeric.push(name);
                }
            }
        }
    });
    for p in params.iter_mut().filter(|p| numeric.contains(&p.name)) {
        p.ts_type = "number".into();
    }
}

fn function(
    path: &str,
    node: Node,
    source: &str,
    decorators: &[String],
    method: bool,
) -> Option<ExportedFn> {
    let name = text(node.child_by_field_name("name")?, source).to_string();
    let (mut params, mut blocker) =
        parameters(node.child_by_field_name("parameters")?, source, method);
    let body = node.child_by_field_name("body");
    if let Some(body) = body {
        infer_numbers(&mut params, body, source);
    }
    let mut generator = false;
    if let Some(body) = body {
        syntax::walk(body, |n| {
            if n.kind() == "yield" {
                generator = true;
            }
        });
    }
    // An untyped parameter that the body calls is a callable: it cannot cross a bridge.
    if let Some(body) = body {
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        syntax::walk(body, |n| {
            if n.kind() == "call"
                && let Some(f) = n.child_by_field_name("function")
                && f.kind() == "identifier"
                && names.contains(&text(f, source))
                && blocker.is_none()
            {
                blocker = Some(format!(
                    "calls its parameter `{}`: a callable",
                    text(f, source)
                ));
            }
        });
    }
    if let Some(d) = decorators
        .iter()
        .find(|d| !SAFE_DECORATORS.contains(&d.as_str()))
    {
        blocker = blocker.or_else(|| Some(format!("decorated with `@{d}`")));
    }
    Some(ExportedFn {
        name,
        file: path.to_string(),
        line: syntax::line(node),
        params,
        returns: node
            .child_by_field_name("return_type")
            .map_or_else(|| "unknown".into(), |r| to_ts_type(text(r, source))),
        is_async: text(node, source).starts_with("async"),
        is_generator: generator,
        blocker,
        ..Default::default()
    })
}

/// Parameters in the type dialect, and a reason when the signature cannot cross a bridge.
fn parameters(node: Node, source: &str, method: bool) -> (Vec<Param>, Option<String>) {
    let mut params = Vec::new();
    let mut blocker = None;
    let mut keyword_only = false;
    for (i, p) in named_children(node).into_iter().enumerate() {
        let name = |n: Option<Node>| n.map(|n| text(n, source).to_string()).unwrap_or_default();
        let param = match p.kind() {
            "identifier" => Param {
                name: text(p, source).into(),
                ts_type: "unknown".into(),
                ..Default::default()
            },
            "typed_parameter" => {
                let inner = named_children(p).into_iter().next();
                let ty = p
                    .child_by_field_name("type")
                    .map_or_else(|| "unknown".into(), |t| to_ts_type(text(t, source)));
                match inner.map(|n| n.kind()) {
                    Some("list_splat_pattern") => {
                        blocker = Some("takes `*args`".into());
                        continue;
                    }
                    Some("dictionary_splat_pattern") => {
                        blocker = Some("takes `**kwargs`".into());
                        continue;
                    }
                    _ => Param {
                        name: name(inner),
                        ts_type: ty,
                        ..Default::default()
                    },
                }
            }
            "default_parameter" | "typed_default_parameter" => {
                let value = p.child_by_field_name("value");
                let default = value.map(|v| text(v, source).to_string());
                let ty = match p.child_by_field_name("type") {
                    Some(t) => to_ts_type(text(t, source)),
                    None => match value.map(|v| v.kind()) {
                        Some("integer") => "int".into(),
                        Some("float") => "number".into(),
                        Some("string") => "string".into(),
                        Some("true" | "false") => "boolean".into(),
                        _ => "unknown".into(),
                    },
                };
                Param {
                    name: name(p.child_by_field_name("name")),
                    ts_type: ty,
                    optional: true,
                    default,
                    ..Default::default()
                }
            }
            "list_splat_pattern" => {
                blocker = Some("takes `*args`".into());
                continue;
            }
            "dictionary_splat_pattern" => {
                blocker = Some("takes `**kwargs`".into());
                continue;
            }
            "keyword_separator" => {
                keyword_only = true;
                continue;
            }
            _ => continue,
        };
        if method && i == 0 && (param.name == "self" || param.name == "cls") {
            continue;
        }
        params.push(Param {
            keyword_only,
            ..param
        });
    }
    (params, blocker)
}

fn class(path: &str, node: Node, source: &str, decorators: &[String]) -> Option<ExportedClass> {
    let name = text(node.child_by_field_name("name")?, source).to_string();
    let mut blockers = Vec::new();
    if let Some(bases) = node.child_by_field_name("superclasses") {
        let bases: Vec<&str> = named_children(bases)
            .into_iter()
            .map(|b| text(b, source))
            .filter(|b| *b != "object")
            .collect();
        if !bases.is_empty() {
            blockers.push(format!("inherits from {}", bases.join(", ")));
        }
    }
    if let Some(d) = decorators.first() {
        blockers.push(format!("decorated with `@{d}`"));
    }
    let mut constructor = Vec::new();
    let mut methods = Vec::new();
    for item in named_children(node.child_by_field_name("body")?) {
        let (decorators, definition) = unwrap_decorated(item, source);
        match definition.kind() {
            "function_definition" => {
                let method_name = definition
                    .child_by_field_name("name")
                    .map_or("", |n| text(n, source));
                if method_name == "__init__" {
                    let (params, blocker) =
                        parameters(definition.child_by_field_name("parameters")?, source, true);
                    constructor = params;
                    blockers.extend(blocker);
                    // `self.name = ...` with a public name: callers can read the attribute.
                    if let Some(body) = definition.child_by_field_name("body") {
                        syntax::walk(body, |n| {
                            if n.kind() == "assignment"
                                && let Some(left) = n.child_by_field_name("left")
                                && let Some(attr) = text(left, source).strip_prefix("self.")
                                && !attr.starts_with('_')
                                && !attr.contains('.')
                            {
                                blockers.push(format!("public attribute `{attr}`"));
                            }
                        });
                    }
                } else if method_name.starts_with("__") && method_name.ends_with("__") {
                    blockers.push(format!("special method `{method_name}`"));
                } else if method_name.starts_with('_') {
                } else if let Some(d) = decorators.first() {
                    blockers.push(format!("`@{d}` method `{method_name}`"));
                } else if let Some(f) = function(path, definition, source, &[], true) {
                    if f.is_async || f.is_generator {
                        blockers.push(format!("async or generator method `{}`", f.name));
                    }
                    if f.returns
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .any(|w| w == name || w == "Self")
                    {
                        blockers.push(format!("method `{}` returns an instance", f.name));
                    }
                    methods.push(f);
                }
            }
            "expression_statement" => {
                let assignment = named_children(definition)
                    .into_iter()
                    .find(|n| n.kind() == "assignment");
                if let Some(left) = assignment.and_then(|a| a.child_by_field_name("left")) {
                    let attr = text(left, source);
                    if !attr.starts_with('_') {
                        blockers.push(format!("public class attribute `{attr}`"));
                    }
                }
            }
            _ => {}
        }
    }
    blockers.sort();
    blockers.dedup();
    Some(ExportedClass {
        name,
        file: path.to_string(),
        line: syntax::line(node),
        constructor,
        methods,
        blocker: (!blockers.is_empty()).then(|| blockers.join(", ")),
    })
}

/// A Python type hint in the TS-like dialect: `list[str]` -> `string[]`, `Optional[int]` ->
/// `int | undefined`, `Callable[...]` -> a function type (which keeps the export in Python).
pub fn to_ts_type(hint: &str) -> String {
    let hint = hint.trim().trim_matches(|c| c == '"' || c == '\'');
    let hint = hint.strip_prefix("typing.").unwrap_or(hint);
    let top = split_top(hint, '|');
    if top.len() > 1 {
        return top
            .iter()
            .map(|p| to_ts_type(p))
            .collect::<Vec<_>>()
            .join(" | ");
    }
    let (head, args) = match hint.find('[') {
        Some(i) if hint.ends_with(']') => {
            (&hint[..i], split_top(&hint[i + 1..hint.len() - 1], ','))
        }
        _ => (hint, Vec::new()),
    };
    let head = head.trim().strip_prefix("typing.").unwrap_or(head.trim());
    let arg = |i: usize| {
        args.get(i)
            .map_or_else(|| "unknown".to_string(), |a| to_ts_type(a))
    };
    match head {
        "str" => "string".into(),
        "int" => "int".into(),
        "float" => "number".into(),
        "bool" => "boolean".into(),
        "None" | "NoneType" => "undefined".into(),
        "list" | "List" | "Sequence" | "Iterable" | "MutableSequence" | "Collection" => {
            format!("{}[]", wrap(&arg(0)))
        }
        "Optional" => format!("{} | undefined", arg(0)),
        "Union" => args
            .iter()
            .map(|a| to_ts_type(a))
            .collect::<Vec<_>>()
            .join(" | "),
        "dict" | "Dict" | "Mapping" | "MutableMapping" if arg(0) == "string" => {
            format!("Record<string, {}>", arg(1))
        }
        "Literal" => args
            .iter()
            .map(|a| a.trim().replace('"', "'"))
            .collect::<Vec<_>>()
            .join(" | "),
        "Callable" => "(...args: unknown[]) => unknown".into(),
        "Any" | "object" => "unknown".into(),
        // Tuples, sets, bytes, other dicts and named classes cross as JSON.
        other => other.to_string(),
    }
}

fn wrap(ts: &str) -> String {
    if ts.contains('|') {
        format!("({ts})")
    } else {
        ts.to_string()
    }
}

fn split_top(text: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 {
            out.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_type_hints() {
        assert_eq!(to_ts_type("list[str]"), "string[]");
        assert_eq!(to_ts_type("Optional[int]"), "int | undefined");
        assert_eq!(to_ts_type("str | None"), "string | undefined");
        assert_eq!(
            to_ts_type("dict[str, list[float]]"),
            "Record<string, number[]>"
        );
        assert_eq!(to_ts_type("Literal['a', \"b\"]"), "'a' | 'b'");
        assert!(to_ts_type("Callable[[int], str]").contains("=>"));
        assert_eq!(to_ts_type("tuple[int, int]"), "tuple");
    }

    #[test]
    fn unix_stdlib_is_not_external() {
        // `import fcntl` in a pure-Python package must not surface as an unknown
        // (npm) dependency: it is Unix stdlib with no PyPI counterpart to look up.
        for module in [
            "fcntl", "termios", "resource", "grp", "pwd", "posix", "syslog",
        ] {
            assert!(
                STDLIB.contains(&module),
                "{module} should resolve as stdlib"
            );
        }
    }

    #[test]
    fn standard_library_imports_resolve_without_dependencies() {
        let ws = Workspace {
            packages: vec![],
            files: vec![],
        };
        let index = HashMap::new();
        for module in [
            "faulthandler",
            "site",
            "bdb",
            "graphlib",
            "_thread",
            "telnetlib",
            "cgi",
        ] {
            assert!(
                matches!(resolve("main.py", module, &index, &ws), Resolution::Stdlib),
                "{module}"
            );
        }
        assert!(matches!(
            resolve("main.py", "requests", &index, &ws),
            Resolution::External(_)
        ));
        let local = HashMap::from([("site.py", 0)]);
        assert!(matches!(
            resolve("main.py", "site", &local, &ws),
            Resolution::File(0)
        ));
    }

    #[test]
    fn a_dotted_import_the_table_knows_keeps_its_dotted_name() {
        let ws = Workspace {
            packages: vec![],
            files: vec![],
        };
        let index = HashMap::new();
        for spec in ["google.protobuf", "google.protobuf.json_format"] {
            match resolve("main.py", spec, &index, &ws) {
                Resolution::External(name) => assert_eq!(name, "google.protobuf", "{spec}"),
                _ => panic!("{spec}: not external"),
            }
        }
        match resolve("main.py", "yaml.loader", &index, &ws) {
            Resolution::External(name) => assert_eq!(name, "yaml"),
            _ => panic!("yaml.loader: not external"),
        }
    }

    #[test]
    fn reads_exports() {
        let source = "__all__ = ['slug', 'Counter']\n\n\
            def slug(text: str, sep: str = '-', *, lower=True) -> str:\n    return text\n\n\
            def hidden(x): pass\n\n\
            @lru_cache\ndef cached(n: int) -> int:\n    return n\n\n\
            def gen(n: int):\n    yield n\n\n\
            class Counter:\n    def __init__(self, start: int = 0):\n        self._n = start\n    def add(self, k: int) -> int:\n        return k\n    def _p(self): pass\n\n\
            class Point:\n    def __init__(self, x):\n        self.x = x\n";
        let (functions, classes) = exports("m.py", source);
        assert_eq!(
            functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["slug"]
        );
        let slug = &functions[0];
        assert_eq!(slug.params[1].default.as_deref(), Some("'-'"));
        assert!(slug.params[2].keyword_only && slug.params[2].ts_type == "boolean");
        assert_eq!(slug.returns, "string");
        assert_eq!(classes.len(), 1);
        assert!(classes[0].blocker.is_none());
        assert_eq!(classes[0].methods.len(), 1);

        let (all, classes) = exports(
            "n.py",
            &source.replace("__all__ = ['slug', 'Counter']\n", ""),
        );
        let names: Vec<&str> = all.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["slug", "hidden", "cached", "gen"]);
        assert!(all[3].is_generator);
        assert!(
            classes[1]
                .blocker
                .as_deref()
                .unwrap()
                .contains("public attribute `x`")
        );
    }

    #[test]
    fn reads_all_after_a_docstring_and_imports() {
        let source = "\"\"\"Text helpers.\"\"\"\n\
            from os import path\n\
            import re\n\n\
            __all__ = ['slug', '_kept']\n\n\
            def slug(text: str) -> str:\n    return text\n\n\
            def helper(x: int) -> int:\n    return x\n\n\
            def _kept(x: int) -> int:\n    return x\n";
        let (functions, _) = exports("m.py", source);
        let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["slug", "_kept"]);
    }

    fn exported(source: &str) -> Vec<String> {
        let defs = "\ndef a(x: int) -> int:\n    return x\n\n\
            def b(x: int) -> int:\n    return x\n\n\
            def _c(x: int) -> int:\n    return x\n";
        exports("m.py", &format!("{source}{defs}"))
            .0
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    #[test]
    fn later_changes_to_all_are_followed() {
        assert_eq!(
            exported("__all__ = ['a']\n__all__ += ['_c']\n"),
            ["a", "_c"]
        );
        assert_eq!(exported("__all__ = ['a']\n__all__ = ['b']\n"), ["b"]);
    }

    #[test]
    fn annotated_prefixed_and_commented_all_is_read() {
        let source =
            "__all__: list[str]\n__all__: list[str] = [\n    r'a',  # first\n    \"_c\",\n]\n";
        assert_eq!(exported(source), ["a", "_c"]);
    }

    #[test]
    fn an_all_built_any_other_way_falls_back_to_public_names() {
        for source in [
            "import other\n__all__ = ['a'] + other.__all__\n",
            "__all__ = sorted(['a'])\n",
            "__all__ = ['a']\n__all__.extend(['_c'])\n",
            "__all__ = [f'{1}']\n",
            "import sys\n__all__ = ['a']\nif sys.platform == 'win32':\n    __all__ += ['_c']\n",
            "__all__ = ['\\x61']\n",
            "__all__ = [b'a']\n",
        ] {
            assert_eq!(exported(source), ["a", "b"], "{source}");
        }
    }

    #[test]
    fn another_module_all_and_reads_in_functions_keep_the_list() {
        let source = "import other\n__all__ = ['a']\nnames = other.__all__\n\n\
            def __dir__():\n    return __all__\n";
        assert_eq!(exported(source), ["a"]);
    }

    #[test]
    fn scans_risks_and_imports() {
        let facts = scan(
            "import os\nfrom .util import helper\nimport re\nx = 7 // 2\nre.compile(r'(?<=a)b')\nfor i in range(3): pass\n",
        );
        let specs: Vec<&str> = facts.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(
            specs.contains(&"os") && specs.contains(&".util") && specs.contains(&".util.helper")
        );
        assert!(facts.risks.contains_key(&Risk::DivisionSemantics));
        assert!(facts.risks.contains_key(&Risk::RegexSyntax));
        assert_eq!(facts.loops, 1);
    }

    #[test]
    fn infers_numbers_from_arithmetic() {
        let src = "def rgb_to_hsv(r, g, b):\n    maxc = max(r, g, b)\n    return maxc - r\n\n\
                   def greet(name):\n    return 'hi ' + name\n";
        let (fns, _) = exports("colorsys.py", src);
        let types: Vec<&str> = fns[0].params.iter().map(|p| p.ts_type.as_str()).collect();
        assert_eq!(types, ["number", "number", "number"]);
        assert_eq!(fns[1].params[0].ts_type, "unknown");
    }
}
