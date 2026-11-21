//! PHP: composer.json packages and PSR-4 roots, `require`/`use` imports, risks, the
//! Composer-to-crate table, and exports (functions and public static methods) with native and
//! phpdoc types mapped into the TS-like type dialect.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use tree_sitter::Node;

use super::syntax::{self, named_children, text};
use crate::risks::{Risk, regex_needs_fancy};
use crate::ts::deps::CrateMatch;
use crate::ts::discover::{FileKind, Package, SourceFile, Workspace};
use crate::ts::exports::{ExportedFn, Param};
use crate::ts::scan::{FileFacts, Import};

fn language() -> tree_sitter::Language {
    tree_sitter_php::LANGUAGE_PHP.into()
}

const SKIP_DIRS: &[&str] = &[
    "vendor",
    "node_modules",
    "var",
    "storage",
    "cache",
    "target",
    ".phpunit.cache",
];

// ---- discovery -------------------------------------------------------------------------------

/// Composer packages that are PHP itself or its extensions, not dependencies to port.
fn is_platform(name: &str) -> bool {
    name == "php" || name.starts_with("ext-") || name.starts_with("lib-") || !name.contains('/')
}

/// `require` and `require-dev` package names, and the first PSR-4 source folder.
fn composer(text: &str) -> (Option<String>, Vec<String>, Vec<String>, Option<String>) {
    let Ok(doc) = serde_json::from_str::<Value>(text) else {
        return (None, Vec::new(), Vec::new(), None);
    };
    let names = |key: &str| -> Vec<String> {
        doc[key]
            .as_object()
            .map(|o| o.keys().filter(|k| !is_platform(k)).cloned().collect())
            .unwrap_or_default()
    };
    let src = doc["autoload"]["psr-4"].as_object().and_then(|o| {
        o.values().find_map(|v| match v {
            Value::String(s) => Some(s.trim_end_matches('/').to_string()),
            Value::Array(a) => a
                .first()
                .and_then(Value::as_str)
                .map(|s| s.trim_end_matches('/').to_string()),
            _ => None,
        })
    });
    (
        doc["name"].as_str().map(str::to_string),
        names("require"),
        names("require-dev"),
        src,
    )
}

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut packages: Vec<Package> = Vec::new();
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
        let dir = rel
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| d.to_string());
        if name == "composer.json" {
            let (pkg_name, deps, dev, src) =
                composer(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            packages.push(Package {
                name: pkg_name.unwrap_or_else(|| {
                    if dir.is_empty() {
                        root.file_name()
                            .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
                    } else {
                        dir.clone()
                    }
                }),
                src_root: match src {
                    Some(s) if !s.is_empty() => join(&dir, &s),
                    _ => dir.clone(),
                },
                dir,
                deps,
                dev_deps: dev,
            });
        } else if name.ends_with(".php") {
            let is_test = name.ends_with("Test.php")
                || name.ends_with("_test.php")
                || rel
                    .split('/')
                    .any(|p| matches!(p, "tests" | "test" | "Tests" | "spec"));
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
    if !packages.iter().any(|p| p.dir.is_empty()) {
        packages.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: if root.join("src").is_dir() {
                "src".into()
            } else {
                String::new()
            },
            deps: Vec::new(),
            dev_deps: Vec::new(),
        });
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

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

// ---- risks and imports -----------------------------------------------------------------------

/// The pattern inside a PCRE string (`'/\w+/u'`, `"#a#i"`, `'~x~'`), without delimiters and flags.
fn pcre_body(literal: &str) -> Option<&str> {
    let s = literal.trim_matches(|c| c == '\'' || c == '"');
    let open = s.chars().next()?;
    let close = match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        c if !c.is_alphanumeric() && c != '\\' && !c.is_whitespace() => c,
        _ => return None,
    };
    let end = s.rfind(close)?;
    (end > 0).then(|| &s[open.len_utf8()..end])
}

/// The path a `require`/`include` names: `'x.php'` or `__DIR__ . '/x.php'`.
fn include_target(arg: Node, source: &str) -> Option<String> {
    let literal = |n: Node| -> Option<String> {
        matches!(n.kind(), "string" | "encapsed_string")
            .then(|| {
                text(n, source)
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string()
            })
            .filter(|s| !s.contains('$'))
    };
    if let Some(s) = literal(arg) {
        return Some(if s.starts_with('/') || s.starts_with('.') {
            s
        } else {
            format!("./{s}")
        });
    }
    if arg.kind() == "binary_expression" {
        let left = arg.child_by_field_name("left")?;
        let right = arg.child_by_field_name("right")?;
        if text(left, source) == "__DIR__" {
            return literal(right).map(|s| format!(".{s}"));
        }
    }
    None
}

fn operator<'a>(node: Node, source: &'a str) -> &'a str {
    node.child_by_field_name("operator").map_or_else(
        || {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|c| !c.is_named())
                .map_or("", |c| text(c, source))
        },
        |o| text(o, source),
    )
}

const REFLECTION: &[&str] = &[
    "method_exists",
    "property_exists",
    "function_exists",
    "class_exists",
    "get_class",
    "get_object_vars",
    "call_user_func",
    "call_user_func_array",
    "is_callable",
    "func_get_args",
    "compact",
    "extract",
];

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
            .push(syntax::line(node))
    };
    syntax::walk(root, |node| match node.kind() {
        "require_expression"
        | "require_once_expression"
        | "include_expression"
        | "include_once_expression" => {
            match named_children(node)
                .into_iter()
                .next()
                .and_then(|a| include_target(a, source))
            {
                Some(spec) => facts.imports.push(Import {
                    specifier: spec,
                    type_only: false,
                    dynamic: false,
                }),
                None => add(&mut facts, Risk::DynamicCode, node),
            }
        }
        "namespace_use_clause" => {
            let name = node
                .named_child(0)
                .map_or_else(|| text(node, source), |n| text(n, source));
            let prefix = node
                .parent()
                .filter(|p| p.kind() == "namespace_use_group")
                .and_then(|g| g.parent())
                .and_then(|d| {
                    named_children(d)
                        .into_iter()
                        .find(|n| n.kind() == "namespace_name")
                })
                .map(|p| format!("{}\\", text(p, source)))
                .unwrap_or_default();
            facts.imports.push(Import {
                specifier: format!("{prefix}{}", name.trim_start_matches('\\')),
                type_only: false,
                dynamic: false,
            });
        }
        "function_call_expression" => {
            let callee = node
                .child_by_field_name("function")
                .map_or("", |f| text(f, source))
                .trim_start_matches('\\');
            match callee {
                "eval" | "create_function" | "assert" => add(&mut facts, Risk::DynamicCode, node),
                c if REFLECTION.contains(&c) => add(&mut facts, Risk::Reflection, node),
                c if c.starts_with("preg_") => {
                    let pattern = node
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next())
                        .and_then(|a| a.named_child(0));
                    match pattern.filter(|p| matches!(p.kind(), "string" | "encapsed_string")) {
                        Some(p) if text(p, source).contains('$') => {
                            add(&mut facts, Risk::DynamicRegex, node)
                        }
                        Some(p) => {
                            if pcre_body(text(p, source)).is_some_and(|b| {
                                regex_needs_fancy(b, "") || b.contains("(?>") || b.contains("\\K")
                            }) {
                                add(&mut facts, Risk::RegexSyntax, node);
                            }
                        }
                        None => add(&mut facts, Risk::DynamicRegex, node),
                    }
                }
                "substr" | "strpos" | "strrpos" | "strlen" | "str_pad" | "substr_replace"
                | "mb_substr" | "mb_strpos" | "mb_strlen" | "ucfirst" | "strtoupper"
                | "strtolower" | "str_split" | "wordwrap" => {
                    add(&mut facts, Risk::StringOffsets, node)
                }
                "intdiv" | "fmod" => add(&mut facts, Risk::DivisionSemantics, node),
                "json_encode" | "json_decode" | "serialize" | "unserialize" => {
                    add(&mut facts, Risk::Json, node)
                }
                "pcntl_fork" | "proc_open" | "curl_multi_exec" => {
                    add(&mut facts, Risk::Concurrency, node)
                }
                "class_alias" | "runkit_function_redefine" | "uopz_set_return" => {
                    add(&mut facts, Risk::Monkeypatching, node)
                }
                "array_map" | "array_filter" | "array_walk" | "array_reduce" | "usort"
                | "uasort" | "uksort" => {
                    facts.loops += 1;
                    add(&mut facts, Risk::Callbacks, node);
                }
                "sort" | "asort" | "ksort" | "array_unique" | "array_keys" | "array_values" => {
                    add(&mut facts, Risk::OrderedCollections, node)
                }
                _ => {}
            }
        }
        "object_creation_expression" => {
            let class = text(node, source);
            if class.contains("Reflection") {
                add(&mut facts, Risk::Reflection, node);
            } else if class.contains("Fiber") || class.contains("parallel\\") {
                add(&mut facts, Risk::Concurrency, node);
            }
        }
        "dynamic_variable_name" => add(&mut facts, Risk::Reflection, node),
        "method_declaration" => {
            let name = node
                .child_by_field_name("name")
                .map_or("", |n| text(n, source));
            if matches!(
                name,
                "__call" | "__callStatic" | "__get" | "__set" | "__isset" | "__unset"
            ) {
                add(&mut facts, Risk::Reflection, node);
            }
        }
        "class_declaration" | "interface_declaration" | "trait_declaration" => {
            if named_children(node)
                .iter()
                .any(|c| c.kind() == "base_clause")
            {
                add(&mut facts, Risk::ClassInheritance, node);
            }
        }
        "use_declaration" => add(&mut facts, Risk::ClassInheritance, node),
        "binary_expression" => match operator(node, source) {
            "/" => add(&mut facts, Risk::NumberSemantics, node),
            "%" => add(&mut facts, Risk::DivisionSemantics, node),
            "**" | "<<" | "*" => add(&mut facts, Risk::IntegerSemantics, node),
            "==" | "!=" | "<>" | "<=>" => add(&mut facts, Risk::DynamicTyping, node),
            _ => {}
        },
        "array_creation_expression" => add(&mut facts, Risk::OrderedCollections, node),
        "throw_expression" | "try_statement" => add(&mut facts, Risk::Exceptions, node),
        "yield_expression" => add(&mut facts, Risk::Generators, node),
        "anonymous_function" | "anonymous_function_creation_expression" | "arrow_function" => {
            add(&mut facts, Risk::Callbacks, node)
        }
        "null" => add(&mut facts, Risk::NullReferences, node),
        "foreach_statement" | "for_statement" | "while_statement" | "do_statement" => {
            facts.loops += 1
        }
        _ => {}
    });
    facts
}

/// Namespace roots of common Composer packages: `use GuzzleHttp\Client` needs `guzzlehttp/guzzle`.
const NAMESPACE_PACKAGES: &[(&str, &str)] = &[
    ("GuzzleHttp", "guzzlehttp/guzzle"),
    ("Monolog", "monolog/monolog"),
    ("Carbon", "nesbot/carbon"),
    ("Ramsey\\Uuid", "ramsey/uuid"),
    ("Illuminate", "laravel/framework"),
    ("Doctrine\\ORM", "doctrine/orm"),
    ("Doctrine\\DBAL", "doctrine/dbal"),
    ("Twig", "twig/twig"),
    ("League\\CommonMark", "league/commonmark"),
    ("League\\Csv", "league/csv"),
    ("League\\Flysystem", "league/flysystem"),
    ("Dotenv", "vlucas/phpdotenv"),
    ("Firebase\\JWT", "firebase/php-jwt"),
    ("Predis", "predis/predis"),
    ("Slim", "slim/slim"),
    ("PHPUnit", "phpunit/phpunit"),
    ("Psr\\Log", "psr/log"),
    ("Psr\\Http\\Message", "psr/http-message"),
    ("Respect\\Validation", "respect/validation"),
    ("Brick\\Math", "brick/math"),
];

/// The Composer package a namespace belongs to: `Symfony\Component\Yaml\Yaml` is
/// `symfony/yaml`; otherwise from [`NAMESPACE_PACKAGES`], else the namespace root.
fn package_of(spec: &str) -> String {
    if let Some(rest) = spec.strip_prefix("Symfony\\Component\\") {
        let component = rest.split('\\').next().unwrap_or(rest);
        let mut kebab = String::new();
        for (i, c) in component.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                kebab.push('-');
            }
            kebab.extend(c.to_lowercase());
        }
        return format!("symfony/{kebab}");
    }
    NAMESPACE_PACKAGES
        .iter()
        .find(|(ns, _)| spec == *ns || spec.starts_with(&format!("{ns}\\")))
        .map_or_else(
            || spec.split('\\').next().unwrap_or(spec).to_string(),
            |(_, p)| (*p).to_string(),
        )
}

pub fn resolve(
    from: &str,
    spec: &str,
    index: &HashMap<&str, usize>,
    ws: &Workspace,
) -> super::Resolution {
    if spec.starts_with("./") || spec.starts_with("../") || spec.ends_with(".php") {
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        let rel = spec.trim_start_matches("./");
        if let Some(path) = normalize(&join(dir, rel))
            && let Some(i) = index.get(path.as_str())
        {
            return super::Resolution::File(*i);
        }
        return super::Resolution::Unresolved;
    }
    // PSR-4: `App\Text\Slug` is `<src>/Text/Slug.php` for the `App\` prefix; the prefix is
    // unknown here, so try each tail of the namespace under each package's source root.
    let parts: Vec<&str> = spec.split('\\').filter(|p| !p.is_empty()).collect();
    for p in &ws.packages {
        for skip in 0..parts.len() {
            let candidate = join(&p.src_root, &format!("{}.php", parts[skip..].join("/")));
            if let Some(i) = index.get(candidate.as_str()) {
                return super::Resolution::File(*i);
            }
        }
    }
    if parts.len() <= 1 {
        return super::Resolution::Stdlib;
    }
    super::Resolution::External(package_of(spec))
}

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

/// Rust counterparts of common Composer packages.
pub fn lookup(package: &str) -> Option<CrateMatch> {
    crate::crates::find("php", package)
}

// ---- exports ---------------------------------------------------------------------------------

/// A PHP type (native or phpdoc: `?int`, `string|null`, `int[]`, `array<string, int>`,
/// `list<string>`) in the dialect.
pub fn php_to_ts(t: &str) -> String {
    let t = t.trim().trim_start_matches('\\');
    if let Some(inner) = t.strip_prefix('?') {
        return format!("{} | undefined", php_to_ts(inner));
    }
    let parts = split_top(t, '|');
    if parts.len() > 1 {
        let mapped: Vec<String> = parts.iter().map(|p| php_to_ts(p)).collect();
        return mapped.join(" | ");
    }
    if let Some(inner) = t.strip_suffix("[]") {
        let inner = php_to_ts(inner);
        return if inner.contains(' ') {
            format!("({inner})[]")
        } else {
            format!("{inner}[]")
        };
    }
    for generic in [
        "list<",
        "array<",
        "iterable<",
        "non-empty-list<",
        "non-empty-array<",
    ] {
        if let Some(inner) = t.strip_prefix(generic).and_then(|r| r.strip_suffix('>')) {
            let args = split_top(inner, ',');
            return match args.as_slice() {
                [v] => format!("{}[]", php_to_ts(v)),
                [k, v] if php_to_ts(k) == "string" => format!("Record<string, {}>", php_to_ts(v)),
                [k, v] if php_to_ts(k) == "int" => format!("{}[]", php_to_ts(v)),
                _ => "unknown".into(),
            };
        }
    }
    match t.to_ascii_lowercase().as_str() {
        "string" | "non-empty-string" | "class-string" | "numeric-string" => "string".into(),
        "int" | "integer" | "positive-int" | "negative-int" | "non-negative-int" => "int".into(),
        "float" | "double" => "number".into(),
        "bool" | "boolean" | "true" | "false" => "boolean".into(),
        "null" | "void" => "undefined".into(),
        "array" | "iterable" => "unknown[]".into(),
        _ => "unknown".into(),
    }
}

fn split_top(text: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '<' | '{' | '(' | '[' => depth += 1,
            '>' | '}' | ')' | ']' => depth -= 1,
            _ if c == sep && depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// The phpdoc block right above `node`, one tag per entry (`@param string $s`).
fn phpdoc(node: Node, source: &str) -> Vec<String> {
    let Some(prev) = node.prev_named_sibling().filter(|p| p.kind() == "comment") else {
        return Vec::new();
    };
    let doc = text(prev, source);
    if !doc.starts_with("/**") {
        return Vec::new();
    }
    doc.trim_start_matches("/**")
        .trim_end_matches("*/")
        .split('@')
        .skip(1)
        .map(|tag| {
            format!(
                "@{}",
                tag.lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
            )
        })
        .collect()
}

/// The dialect type of a literal default (`'-'`, `0`, `1.5`, `true`, `[]`).
fn literal_type(value: &str) -> Option<&'static str> {
    let v = value.trim();
    if v.starts_with('"') || v.starts_with('\'') {
        Some("string")
    } else if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false") {
        Some("boolean")
    } else if v.parse::<i64>().is_ok() {
        Some("int")
    } else if v.parse::<f64>().is_ok() {
        Some("number")
    } else if v.starts_with('[') {
        Some("unknown[]")
    } else {
        None
    }
}

fn function_fn(
    path: &str,
    receiver: &str,
    sep: &str,
    node: Node,
    source: &str,
) -> Option<ExportedFn> {
    let name = text(node.child_by_field_name("name")?, source).to_string();
    let doc = phpdoc(node, source);
    let doc_param = |p: &str| {
        doc.iter().find_map(|tag| {
            let rest = tag.strip_prefix("@param ")?;
            let mut words = rest.split_whitespace();
            let t = words.next()?;
            (words.next()? == format!("${p}")).then(|| php_to_ts(t))
        })
    };
    let returns = node
        .child_by_field_name("return_type")
        .map(|t| php_to_ts(text(t, source)))
        .filter(|t| t != "unknown")
        .or_else(|| {
            doc.iter().find_map(|tag| {
                let t = tag.strip_prefix("@return ")?.split_whitespace().next()?;
                Some(php_to_ts(t))
            })
        })
        .unwrap_or_else(|| "unknown".into());
    let mut blocker = None;
    if name.starts_with("__") {
        blocker = Some(format!("`{name}` is a magic method"));
    }
    let mut params = Vec::new();
    let mut signature = String::new();
    if let Some(list) = node.child_by_field_name("parameters") {
        signature = text(list, source)
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_string();
        for p in named_children(list) {
            let pname = p
                .child_by_field_name("name")
                .map(|n| text(n, source).trim_start_matches('$').to_string())
                .unwrap_or_default();
            match p.kind() {
                "simple_parameter" => {}
                "variadic_parameter" => {
                    blocker.get_or_insert_with(|| "takes `...$args`".into());
                    continue;
                }
                _ => {
                    blocker.get_or_insert_with(|| format!("parameter `{}`", text(p, source)));
                    continue;
                }
            }
            if named_children(p)
                .iter()
                .any(|c| c.kind() == "reference_modifier")
                || text(p, source).contains("&$")
            {
                blocker.get_or_insert_with(|| format!("takes `${pname}` by reference"));
            }
            let declared = p.child_by_field_name("type").map(|t| text(t, source));
            if declared.is_some_and(|t| t.contains("callable") || t.contains("Closure")) {
                blocker.get_or_insert_with(|| format!("takes a callable `${pname}`"));
            }
            let value = p
                .child_by_field_name("default_value")
                .map(|v| text(v, source).to_string());
            let mut ts_type = declared
                .map(php_to_ts)
                .filter(|t| t != "unknown")
                .or_else(|| doc_param(&pname))
                .or_else(|| value.as_deref().and_then(literal_type).map(str::to_string))
                .unwrap_or_else(|| "unknown".into());
            if value
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case("null"))
                && !ts_type.contains("undefined")
            {
                ts_type = format!("{ts_type} | undefined");
            }
            params.push(Param {
                name: pname,
                ts_type,
                optional: value.is_some(),
                default: value,
                ..Default::default()
            });
        }
    }
    if let Some(body) = node.child_by_field_name("body") {
        let mut yields = false;
        syntax::walk(body, |n| yields |= n.kind() == "yield_expression");
        if yields {
            blocker.get_or_insert_with(|| "is a generator (`yield`)".into());
        }
    }
    let prefix = if receiver.is_empty() {
        String::new()
    } else {
        format!("{receiver}{sep}")
    };
    Some(ExportedFn {
        name: name.clone(),
        file: path.to_string(),
        line: syntax::line(node),
        params,
        returns,
        prototype: Some(format!("{prefix}{name}({signature})")),
        blocker,
        ..Default::default()
    })
}

/// Functions (`Ns\name(...)`) and public static methods of classes (`Ns\Class::name(...)`).
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".php") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut namespace = String::new();
    let visit = |items: Vec<Node>, namespace: &mut String, out: &mut Vec<ExportedFn>| {
        for item in items {
            match item.kind() {
                "namespace_definition" => {
                    *namespace = item
                        .child_by_field_name("name")
                        .map(|n| text(n, source).to_string())
                        .unwrap_or_default();
                }
                "function_definition" => {
                    out.extend(function_fn(path, namespace, "\\", item, source));
                }
                "class_declaration" => {
                    if named_children(item)
                        .iter()
                        .any(|c| c.kind() == "abstract_modifier")
                    {
                        continue;
                    }
                    let Some(class) = item.child_by_field_name("name") else {
                        continue;
                    };
                    let class = text(class, source);
                    let receiver = if namespace.is_empty() {
                        class.to_string()
                    } else {
                        format!("{namespace}\\{class}")
                    };
                    for method in item
                        .child_by_field_name("body")
                        .map(named_children)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|m| m.kind() == "method_declaration")
                    {
                        let mods: Vec<&str> = named_children(method)
                            .iter()
                            .filter(|c| c.kind().ends_with("_modifier"))
                            .map(|c| text(*c, source))
                            .collect();
                        let public = !mods.iter().any(|m| matches!(*m, "private" | "protected"));
                        if public && mods.contains(&"static") {
                            out.extend(function_fn(path, &receiver, "::", method, source));
                        }
                    }
                }
                _ => {}
            }
        }
    };
    let root = tree.root_node();
    for item in named_children(root) {
        if item.kind() == "namespace_definition"
            && let Some(body) = item.child_by_field_name("body")
        {
            namespace = item
                .child_by_field_name("name")
                .map(|n| text(n, source).to_string())
                .unwrap_or_default();
            visit(named_children(body), &mut namespace, &mut out);
        } else {
            visit(vec![item], &mut namespace, &mut out);
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"<?php
declare(strict_types=1);
namespace App\Text;
use App\Util\Str;
use GuzzleHttp\Client;
use Symfony\Component\Yaml\Yaml;
require_once __DIR__ . '/helpers.php';

/**
 * @param string $s
 * @param list<string> $stop
 * @return string
 */
function slugify(string $s, $stop = [], string $sep = '-', ?int $max = null): string {
    if ($s === '') { throw new \InvalidArgumentException("empty"); }
    $x = intdiv(7, 2) % 3;
    return strtolower(preg_replace('/(?<=a)b/u', $sep, $s));
}

function words($text) { foreach (explode(' ', $text) as $w) { yield $w; } }

final class Words extends Base {
    public static function count(string $s, int ...$rest): int { return str_word_count($s); }
    public static function scale($x, float $factor = 1.5) { return $x * $factor; }
    private static function hidden() {}
    public function instance() {}
}
eval('1;');
"#;

    #[test]
    fn maps_php_types() {
        assert_eq!(php_to_ts("?int"), "int | undefined");
        assert_eq!(php_to_ts("string|null"), "string | undefined");
        assert_eq!(php_to_ts("int[]"), "int[]");
        assert_eq!(php_to_ts("array<string, int>"), "Record<string, int>");
        assert_eq!(php_to_ts("list<float>"), "number[]");
        assert_eq!(php_to_ts("\\DateTime"), "unknown");
    }

    #[test]
    fn finds_functions_and_static_methods() {
        let fns = exports("src/Text/slug.php", SRC);
        let protos: Vec<&str> = fns
            .iter()
            .map(|f| f.prototype.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(
            protos,
            [
                "App\\Text\\slugify(string $s, $stop = [], string $sep = '-', ?int $max = null)",
                "App\\Text\\words($text)",
                "App\\Text\\Words::count(string $s, int ...$rest)",
                "App\\Text\\Words::scale($x, float $factor = 1.5)",
            ]
        );
        let slug = &fns[0];
        let types: Vec<&str> = slug.params.iter().map(|p| p.ts_type.as_str()).collect();
        assert_eq!(types, ["string", "string[]", "string", "int | undefined"]);
        assert_eq!(slug.returns, "string");
        assert!(slug.blocker.is_none());
        assert!(fns[1].blocker.as_deref().unwrap().contains("generator"));
        assert!(fns[2].blocker.as_deref().unwrap().contains("...$args"));
        assert_eq!(fns[3].params[1].ts_type, "number");
    }

    #[test]
    fn resolves_import_names_from_a_group_use() {
        let facts = scan(
            r"<?php
namespace App;
use Vendor\Library\{One, Two as Alias, function helpers\trim_value};
use Other\Three;
",
        );
        assert_eq!(facts.parse_errors, 0);
        assert_eq!(
            facts
                .imports
                .iter()
                .map(|i| i.specifier.as_str())
                .collect::<Vec<_>>(),
            [
                "Vendor\\Library\\One",
                "Vendor\\Library\\Two",
                "Vendor\\Library\\helpers\\trim_value",
                "Other\\Three"
            ]
        );
    }

    #[test]
    fn scans_imports_and_risks() {
        let facts = scan(SRC);
        let specs: Vec<&str> = facts.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(
            specs,
            [
                "App\\Util\\Str",
                "GuzzleHttp\\Client",
                "Symfony\\Component\\Yaml\\Yaml",
                "./helpers.php"
            ]
        );
        for risk in [
            Risk::Exceptions,
            Risk::RegexSyntax,
            Risk::DivisionSemantics,
            Risk::Generators,
            Risk::ClassInheritance,
            Risk::DynamicCode,
            Risk::StringOffsets,
        ] {
            assert!(
                facts.risks.contains_key(&risk),
                "{risk:?}: {:?}",
                facts.risks
            );
        }
        assert_eq!(package_of("GuzzleHttp\\Client"), "guzzlehttp/guzzle");
        assert_eq!(
            package_of("Symfony\\Component\\HttpFoundation\\Request"),
            "symfony/http-foundation"
        );
        assert_eq!(pcre_body("'#a+#i'"), Some("a+"));
        assert_eq!(pcre_body("'{x}u'"), Some("x"));
    }

    #[test]
    fn reads_composer() {
        let (name, deps, dev, src) = composer(
            r#"{"name": "acme/tool", "require": {"php": ">=8.1", "ext-json": "*", "guzzlehttp/guzzle": "^7"},
               "require-dev": {"phpunit/phpunit": "^10"}, "autoload": {"psr-4": {"Acme\\": "src/"}}}"#,
        );
        assert_eq!(name.as_deref(), Some("acme/tool"));
        assert_eq!(deps, ["guzzlehttp/guzzle"]);
        assert_eq!(dev, ["phpunit/phpunit"]);
        assert_eq!(src.as_deref(), Some("src"));
    }
}
