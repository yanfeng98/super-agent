//! Per-file facts from the oxc AST: imports, platform use, migration risks.

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AwaitExpression, BinaryExpression, CallExpression, Class,
    DoWhileStatement, ExportAllDeclaration, ExportFromDeclaration, Expression, ForInStatement,
    ForOfStatement, ForStatement, Function, IdentifierReference, ImportDeclaration,
    ImportExpression, NewExpression, RegExpLiteral, StaticMemberExpression, TSAnyKeyword,
    TSFunctionType, TSMethodSignature, ThrowStatement, TryStatement, WhileStatement,
    YieldExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxc_syntax::scope::ScopeFlags;
use serde::Serialize;

use crate::risks::{Risk, regex_folds_differently, regex_needs_fancy};

/// A runtime a file depends on directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    Node,
    Browser,
    Editor,
    Ui,
}

#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub specifier: String,
    pub type_only: bool,
    pub dynamic: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct FileFacts {
    pub loc: usize,
    pub imports: Vec<Import>,
    /// Lines (1-based) where each risk shows up.
    pub risks: BTreeMap<Risk, Vec<u32>>,
    pub platforms: BTreeSet<Platform>,
    pub loops: usize,
    pub parse_errors: usize,
}

const STRING_OFFSET_METHODS: &[&str] = &[
    "slice",
    "substring",
    "substr",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "indexOf",
    "lastIndexOf",
    "padStart",
    "padEnd",
];
const BROWSER_GLOBALS: &[&str] = &[
    "window",
    "document",
    "navigator",
    "localStorage",
    "sessionStorage",
    "location",
    "HTMLElement",
];
const NODE_GLOBALS: &[&str] = &["process", "Buffer", "__dirname", "__filename"];
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
];
const UI_PACKAGES: &[&str] = &[
    "vue",
    "react",
    "react-dom",
    "preact",
    "solid-js",
    "svelte",
    "nuxt",
    "#app",
    "#imports",
    "vue-router",
];

/// The platform an import specifier pins a file to, if any. A Node subpath is the same builtin
/// (`stream/web`, `util/types`, `fs/promises`), so the builtin list is matched on the package root;
/// a subpath of anything else stays what its root says it is.
pub fn platform_of(specifier: &str) -> Option<Platform> {
    let root = specifier.split('/').next().unwrap_or(specifier);
    if specifier.starts_with("node:") || NODE_BUILTINS.contains(&root) {
        Some(Platform::Node)
    } else if specifier == "vscode" {
        Some(Platform::Editor)
    } else if UI_PACKAGES.contains(&specifier) {
        Some(Platform::Ui)
    } else {
        None
    }
}

/// Parses one file and collects its facts. Never fails: syntax errors are counted.
pub fn scan(path: &str, source: &str) -> FileFacts {
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let mut collector = Collector {
        line_starts: std::iter::once(0)
            .chain(
                source
                    .match_indices('\n')
                    .map(|(i, _)| u32::try_from(i + 1).unwrap_or(u32::MAX)),
            )
            .collect(),
        facts: FileFacts {
            loc: source.lines().filter(|l| !l.trim().is_empty()).count(),
            parse_errors: ret.diagnostics.len(),
            ..FileFacts::default()
        },
    };
    collector.visit_program(&ret.program);
    for import in &collector.facts.imports {
        if let Some(platform) = platform_of(&import.specifier) {
            collector.facts.platforms.insert(platform);
        }
    }
    collector.facts
}

struct Collector {
    line_starts: Vec<u32>,
    facts: FileFacts,
}

impl Collector {
    fn line(&self, span: Span) -> u32 {
        u32::try_from(self.line_starts.partition_point(|&s| s <= span.start)).unwrap_or(u32::MAX)
    }

    fn add(&mut self, risk: Risk, span: Span) {
        let line = self.line(span);
        self.facts.risks.entry(risk).or_default().push(line);
    }

    fn import(&mut self, specifier: String, type_only: bool, dynamic: bool) {
        self.facts.imports.push(Import {
            specifier,
            type_only,
            dynamic,
        });
    }

    fn check_regex(&mut self, pattern: &str, flags: &str, span: Span) {
        if regex_needs_fancy(pattern, flags) || regex_folds_differently(pattern, flags) {
            self.add(Risk::RegexSyntax, span);
        }
        if flags.contains('d') {
            self.add(Risk::StringOffsets, span);
        }
    }
}

fn identifier<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    match expr {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn string_arg(arg: Option<&Argument>) -> Option<String> {
    match arg {
        Some(Argument::StringLiteral(s)) => Some(s.value.display().to_string()),
        _ => None,
    }
}

impl<'a> Visit<'a> for Collector {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.import(
            it.source.value.display().to_string(),
            it.import_kind.is_type(),
            false,
        );
        walk::walk_import_declaration(self, it);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.import(
            it.source.value.display().to_string(),
            it.export_kind.is_type(),
            false,
        );
        walk::walk_export_from_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.import(
            it.source.value.display().to_string(),
            it.export_kind.is_type(),
            false,
        );
        walk::walk_export_all_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match &it.source {
            Expression::StringLiteral(s) => self.import(s.value.display().to_string(), false, true),
            _ => self.add(Risk::DynamicCode, it.span),
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        match &it.callee {
            Expression::StaticMemberExpression(member) => {
                let property = member.property.name.as_str();
                match (identifier(&member.object), property) {
                    (Some("JSON"), _) => self.add(Risk::Json, it.span),
                    (Some("Object"), "keys" | "values" | "entries" | "fromEntries") => {
                        self.add(Risk::OrderedCollections, it.span);
                    }
                    (
                        Some("Object"),
                        "defineProperty" | "getPrototypeOf" | "setPrototypeOf" | "freeze",
                    )
                    | (Some("Reflect"), _) => self.add(Risk::Reflection, it.span),
                    (Some("Promise"), _) => self.add(Risk::Async, it.span),
                    (Some("Math"), "random")
                    | (Some("Date" | "performance"), "now")
                    | (Some("crypto"), "randomUUID" | "getRandomValues") => {
                        self.add(Risk::ClockAndRandom, it.span);
                    }
                    (Some("Math"), "round") | (_, "toFixed") => {
                        self.add(Risk::NumberSemantics, it.span);
                    }
                    (_, "matchAll" | "exec") => self.add(Risk::StringOffsets, it.span),
                    (_, method) if STRING_OFFSET_METHODS.contains(&method) => {
                        self.add(Risk::StringOffsets, it.span);
                    }
                    _ => {}
                }
            }
            Expression::Identifier(id) => match id.name.as_str() {
                "eval" => self.add(Risk::DynamicCode, it.span),
                "Date" => self.add(Risk::ClockAndRandom, it.span),
                "parseInt" | "parseFloat" => self.add(Risk::NumberSemantics, it.span),
                "require" => match string_arg(it.arguments.first()) {
                    Some(specifier) => self.import(specifier, false, true),
                    None => self.add(Risk::DynamicCode, it.span),
                },
                "RegExp" => {
                    self.regexp_constructor(it.arguments.first(), it.arguments.get(1), it.span)
                }
                _ => {}
            },
            _ => {}
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        match identifier(&it.callee) {
            Some("RegExp") => {
                self.regexp_constructor(it.arguments.first(), it.arguments.get(1), it.span);
            }
            Some("Map" | "Set") => self.add(Risk::OrderedCollections, it.span),
            Some("Date") if it.arguments.is_empty() => self.add(Risk::ClockAndRandom, it.span),
            Some("Function") => self.add(Risk::DynamicCode, it.span),
            Some("Proxy" | "WeakRef" | "FinalizationRegistry") => {
                self.add(Risk::Reflection, it.span);
            }
            Some("Promise") => self.add(Risk::Async, it.span),
            _ => {}
        }
        walk::walk_new_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if matches!(it.property.name.as_str(), "indices" | "lastIndex") {
            self.add(Risk::StringOffsets, it.span);
        }
        walk::walk_static_member_expression(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        let flags = it.regex.flags.to_string();
        self.check_regex(it.regex.pattern.text.as_str(), &flags, it.span);
    }

    fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
        self.add(Risk::Exceptions, it.span);
        walk::walk_throw_statement(self, it);
    }

    fn visit_try_statement(&mut self, it: &TryStatement<'a>) {
        self.add(Risk::Exceptions, it.span);
        walk::walk_try_statement(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.add(Risk::Async, it.span);
        walk::walk_await_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.generator {
            self.add(Risk::Generators, it.span);
        }
        if it.r#async {
            self.add(Risk::Async, it.span);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if it.r#async {
            self.add(Risk::Async, it.span);
        }
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.add(Risk::Generators, it.span);
        walk::walk_yield_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if it.heritage.is_some() {
            self.add(Risk::ClassInheritance, it.span);
        }
        walk::walk_class(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.facts.loops += 1;
        self.add(Risk::OrderedCollections, it.span);
        walk::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.facts.loops += 1;
        walk::walk_for_of_statement(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.facts.loops += 1;
        walk::walk_for_statement(self, it);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.facts.loops += 1;
        walk::walk_while_statement(self, it);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.facts.loops += 1;
        walk::walk_do_while_statement(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_bitwise() {
            self.add(Risk::NumberSemantics, it.span);
        } else if it.operator.is_in() {
            self.add(Risk::DynamicTyping, it.span);
        }
        walk::walk_binary_expression(self, it);
    }

    fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
        self.add(Risk::DynamicTyping, it.span);
    }

    fn visit_ts_function_type(&mut self, it: &TSFunctionType<'a>) {
        self.add(Risk::Callbacks, it.span);
        walk::walk_ts_function_type(self, it);
    }

    fn visit_ts_method_signature(&mut self, it: &TSMethodSignature<'a>) {
        self.add(Risk::Callbacks, it.span);
        walk::walk_ts_method_signature(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if BROWSER_GLOBALS.contains(&name) {
            self.facts.platforms.insert(Platform::Browser);
        } else if NODE_GLOBALS.contains(&name) {
            self.facts.platforms.insert(Platform::Node);
        } else if name == "Symbol" {
            self.add(Risk::Reflection, it.span);
        }
    }
}

impl Collector {
    fn regexp_constructor(
        &mut self,
        pattern: Option<&Argument>,
        flags: Option<&Argument>,
        span: Span,
    ) {
        match string_arg(pattern) {
            Some(pattern) => {
                let flags = string_arg(flags).unwrap_or_default();
                self.check_regex(&pattern, &flags, span);
            }
            None => self.add(Risk::DynamicRegex, span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_imports_platforms_and_risks() {
        let facts = scan(
            "a.ts",
            "import { readFile } from 'node:fs'\n\
             import type { X } from './types'\n\
             export * from './util.js'\n\
             const re = /(?<=a)b/g\n\
             const dyn = new RegExp(input)\n\
             export function f(s: string) { return s.slice(1) }\n\
             const m = new Map()\n",
        );
        let specifiers: Vec<_> = facts.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(specifiers, ["node:fs", "./types", "./util.js"]);
        assert!(facts.imports[1].type_only);
        assert!(facts.platforms.contains(&Platform::Node));
        assert_eq!(facts.risks[&Risk::RegexSyntax], [4]);
        assert_eq!(facts.risks[&Risk::DynamicRegex], [5]);
        assert_eq!(facts.risks[&Risk::StringOffsets], [6]);
        assert_eq!(facts.risks[&Risk::OrderedCollections], [7]);
        assert_eq!(facts.parse_errors, 0);
    }

    #[test]
    fn flags_rounding_and_ascii_case_folds() {
        let facts = scan(
            "ms.ts",
            "const re = /^(\\d+) *(secs?|ms)$/i\n\
             export const r = (x: number) => Math.round(x)\n\
             export const f = (x: number) => x.toFixed(2)\n",
        );
        assert_eq!(facts.risks[&Risk::RegexSyntax], [1]);
        assert_eq!(facts.risks[&Risk::NumberSemantics], [2, 3]);
    }

    #[test]
    fn a_subpath_names_the_same_builtin() {
        // A Node subpath is the same module, and every builtin counts, not the ones a fixture
        // happened to import.
        for spec in [
            "stream/web",
            "util/types",
            "dns/promises",
            "fs/promises",
            "node:fs/promises",
            "tls",
            "http2",
            "querystring",
            "string_decoder",
            "wasi",
        ] {
            assert_eq!(platform_of(spec), Some(Platform::Node), "{spec}");
        }
        // A subpath of a third-party or UI package stays what its root says it is.
        assert_eq!(platform_of("react/jsx-runtime"), None);
        assert_eq!(platform_of("@scope/pkg/sub"), None);
        assert_eq!(platform_of("path-to-regexp"), None);
        assert_eq!(platform_of("left-pad"), None);
        assert_eq!(platform_of("vscode"), Some(Platform::Editor));
        assert_eq!(platform_of("svelte/store"), None);
    }
}
