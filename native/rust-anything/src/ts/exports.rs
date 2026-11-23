//! Exported functions of a TS/JS file, with their parameter and return types as written.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, FormalParameters, Statement, TSTypeAnnotation,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde::{Deserialize, Serialize};

use super::jsdoc::{self, JsDoc, ParamTag};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    /// The type in the TS-like dialect; `unknown` when there is no annotation.
    pub ts_type: String,
    pub optional: bool,
    pub rest: bool,
    /// The default value as written in the source, when the language has defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Python keyword-only parameter (after `*`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyword_only: bool,
    /// The exact source type, for languages whose bridge must match it (C: `size_t`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_type: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedFn {
    pub name: String,
    /// Source file, relative to the repository root.
    pub file: String,
    pub line: u32,
    pub params: Vec<Param>,
    /// The TS return type as written; `unknown` when there is no annotation.
    pub returns: String,
    pub is_async: bool,
    #[serde(default)]
    pub is_generator: bool,
    /// Names of the function's own type parameters (`T` in `f<T>(x: T)`).
    #[serde(default)]
    pub type_params: Vec<String>,
    /// The body checks `instanceof`: class identity, which does not survive a bridge.
    #[serde(default)]
    pub uses_instanceof: bool,
    /// Why the function must stay in its language, found by the language plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
    /// The exact source return type (C: `unsigned long`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_return: Option<String>,
    /// The declaration as written (C: `int word_count(const char *text)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prototype: Option<String>,
}

/// An exported class: its constructor and public instance methods, or why it must stay in TS.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedClass {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub constructor: Vec<Param>,
    pub methods: Vec<ExportedFn>,
    /// Set when the class cannot cross a bridge as it is (inheritance, public fields, ...).
    pub blocker: Option<String>,
}

/// Top-level `export class` declarations.
pub fn exported_classes(path: &str, source: &str) -> Vec<ExportedClass> {
    use oxc_ast::ast::{ClassElement, MethodDefinitionKind, TSAccessibility};
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let text = |span: oxc_span::Span| source[span.start as usize..span.end as usize].to_string();
    let line = |offset: u32| {
        u32::try_from(source[..offset as usize].matches('\n').count() + 1).unwrap_or(u32::MAX)
    };
    let annotation = |ann: Option<&TSTypeAnnotation>| {
        ann.map_or_else(|| "unknown".to_string(), |a| text(a.type_annotation.span()))
    };
    let params_of = |params: &FormalParameters| -> Vec<Param> {
        params
            .items
            .iter()
            .enumerate()
            .map(|(i, p)| Param {
                name: binding_name(&p.pattern).unwrap_or_else(|| format!("arg{i}")),
                ts_type: annotation(p.type_annotation.as_deref()),
                optional: p.optional || p.initializer.is_some(),
                rest: false,
                ..Default::default()
            })
            .collect()
    };
    let mut out = Vec::new();
    for statement in &ret.program.body {
        let Statement::ExportDeclaration(export) = statement else {
            continue;
        };
        let Declaration::ClassDeclaration(class) = &export.declaration else {
            continue;
        };
        let Some(id) = &class.id else { continue };
        let mut blockers: Vec<String> = Vec::new();
        if class.heritage.is_some() {
            blockers.push("extends another class".into());
        }
        let mut constructor = Vec::new();
        let mut methods = Vec::new();
        for element in &class.body.body {
            match element {
                ClassElement::MethodDefinition(m) => {
                    let hidden = m.key.is_private_identifier()
                        || matches!(
                            m.accessibility,
                            Some(TSAccessibility::Private | TSAccessibility::Protected)
                        );
                    let name = m
                        .key
                        .static_name()
                        .and_then(|n| n.as_str().map(str::to_string));
                    match m.kind {
                        MethodDefinitionKind::Constructor => {
                            constructor = params_of(&m.value.params);
                            // `constructor(public data)`: a parameter property is a public field.
                            if m.value.params.items.iter().any(|p| {
                                matches!(p.accessibility, Some(TSAccessibility::Public))
                                    || (p.readonly && p.accessibility.is_none())
                            }) {
                                blockers.push("public parameter properties".into());
                            }
                        }
                        _ if hidden => {}
                        MethodDefinitionKind::Get | MethodDefinitionKind::Set => {
                            blockers.push(format!("accessor `{}`", name.unwrap_or_default()));
                        }
                        MethodDefinitionKind::Method if m.r#static => {
                            blockers.push(format!("static method `{}`", name.unwrap_or_default()));
                        }
                        MethodDefinitionKind::Method => {
                            let Some(name) = name else {
                                blockers.push("computed method name".into());
                                continue;
                            };
                            let f = &m.value;
                            methods.push(ExportedFn {
                                name,
                                file: path.to_string(),
                                line: line(m.span.start),
                                params: params_of(&f.params),
                                returns: annotation(f.return_type.as_deref()),
                                is_async: f.r#async,
                                is_generator: f.generator,
                                type_params: Vec::new(),
                                uses_instanceof: uses_instanceof(&text(m.span)),
                                ..Default::default()
                            });
                        }
                    }
                }
                ClassElement::PropertyDefinition(p) => {
                    let hidden = p.key.is_private_identifier()
                        || matches!(
                            p.accessibility,
                            Some(TSAccessibility::Private | TSAccessibility::Protected)
                        );
                    if !hidden {
                        let name = p
                            .key
                            .static_name()
                            .and_then(|n| n.as_str().map(str::to_string))
                            .unwrap_or_default();
                        blockers.push(format!("public field `{name}`"));
                    }
                }
                ClassElement::AccessorProperty(_) => blockers.push("accessor property".into()),
                ClassElement::StaticBlock(_) => blockers.push("static block".into()),
                ClassElement::TSIndexSignature(_) => {}
            }
        }
        let name = id.name.as_str().to_string();
        for m in &methods {
            if m.is_async || m.is_generator {
                blockers.push(format!("async or generator method `{}`", m.name));
            }
            let returns_self = m
                .returns
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == name || w == "this");
            if returns_self {
                blockers.push(format!("method `{}` returns an instance", m.name));
            }
        }
        out.push(ExportedClass {
            name,
            file: path.to_string(),
            line: line(class.span.start),
            constructor,
            methods,
            blocker: (!blockers.is_empty()).then(|| blockers.join(", ")),
        });
    }
    out
}

/// Top-level `export function f()` and `export const f = () => ...` declarations.
pub fn exported_functions(path: &str, source: &str) -> Vec<ExportedFn> {
    functions(path, source, false)
}

pub fn declared_functions(path: &str, source: &str) -> Vec<ExportedFn> {
    functions(path, source, true)
}

fn functions(path: &str, source: &str, declarations: bool) -> Vec<ExportedFn> {
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let text = |span: oxc_span::Span| source[span.start as usize..span.end as usize].to_string();
    let annotation = |ann: Option<&TSTypeAnnotation>| {
        ann.map_or_else(|| "unknown".to_string(), |a| text(a.type_annotation.span()))
    };
    let line = |offset: u32| {
        u32::try_from(source[..offset as usize].matches('\n').count() + 1).unwrap_or(u32::MAX)
    };
    let params = |params: &FormalParameters, doc: &JsDoc| -> Vec<Param> {
        let mut out: Vec<Param> = params
            .items
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = binding_name(&p.pattern).unwrap_or_else(|| format!("arg{i}"));
                let tag = doc.param(&name);
                let documented = tag.and_then(|t| t.ts_type.clone());
                Param {
                    ts_type: match (&p.type_annotation, documented, p.initializer.as_deref()) {
                        (Some(a), _, _) => annotation(Some(a)),
                        (None, Some(t), _) => t,
                        // `namespaced = true`: the type comes from the default value.
                        (None, None, Some(Expression::BooleanLiteral(_))) => "boolean".into(),
                        (None, None, Some(Expression::NumericLiteral(_))) => "number".into(),
                        (
                            None,
                            None,
                            Some(Expression::StringLiteral(_) | Expression::TemplateLiteral(_)),
                        ) => "string".into(),
                        (None, None, _) => "unknown".into(),
                    },
                    optional: p.optional
                        || p.initializer.is_some()
                        || (p.type_annotation.is_none() && tag.is_some_and(|t| t.optional)),
                    rest: false,
                    name,
                    ..Default::default()
                }
            })
            .collect();
        if let Some(rest) = &params.rest {
            let name = binding_name(&rest.rest.argument).unwrap_or_else(|| "rest".into());
            let ts_type = match (&rest.type_annotation, doc.param(&name)) {
                (
                    None,
                    Some(ParamTag {
                        ts_type: Some(t), ..
                    }),
                ) => t.clone(),
                (ann, _) => annotation(ann.as_deref()),
            };
            out.push(Param {
                name,
                ts_type,
                optional: true,
                rest: true,
                ..Default::default()
            });
        }
        out
    };
    let returns_of = |ann: Option<&TSTypeAnnotation>, doc: &JsDoc| match (ann, &doc.returns) {
        (None, Some(t)) => t.clone(),
        (ann, _) => annotation(ann),
    };

    let function_entry = |f: &oxc_ast::ast::Function, doc: &JsDoc, exported: Option<String>| {
        let id = f.id.as_ref()?;
        Some(ExportedFn {
            name: exported.unwrap_or_else(|| id.name.as_str().to_string()),
            file: path.to_string(),
            line: line(f.span.start),
            params: params(&f.params, doc),
            returns: returns_of(f.return_type.as_deref(), doc),
            is_async: f.r#async,
            is_generator: f.generator,
            type_params: type_params_of(f.type_parameters.as_deref(), &text),
            uses_instanceof: uses_instanceof(&text(f.span)),
            ..Default::default()
        })
    };
    let declarator_entry =
        |declarator: &oxc_ast::ast::VariableDeclarator, doc: &JsDoc, exported: Option<String>| {
            let name = binding_name(&declarator.id)?;
            let type_names =
                |tp: Option<&oxc_ast::ast::TSTypeParameterDeclaration>| type_params_of(tp, &text);
            let (fn_params, returns, is_async, is_generator, span, type_params) =
                match &declarator.init {
                    Some(Expression::ArrowFunctionExpression(f)) => (
                        &f.params,
                        f.return_type.as_deref(),
                        f.r#async,
                        false,
                        f.span,
                        type_names(f.type_parameters.as_deref()),
                    ),
                    Some(Expression::FunctionExpression(f)) => (
                        &f.params,
                        f.return_type.as_deref(),
                        f.r#async,
                        f.generator,
                        f.span,
                        type_names(f.type_parameters.as_deref()),
                    ),
                    _ => return None,
                };
            Some(ExportedFn {
                name: exported.unwrap_or(name),
                file: path.to_string(),
                line: line(span.start),
                params: params(fn_params, doc),
                returns: returns_of(returns, doc),
                is_async,
                is_generator,
                type_params,
                uses_instanceof: uses_instanceof(&text(span)),
                ..Default::default()
            })
        };

    let mut out = Vec::new();
    let mut local = std::collections::HashMap::new();
    let mut listed = Vec::new();
    for statement in &ret.program.body {
        let doc = jsdoc::leading(source, statement.span().start as usize).unwrap_or_default();
        match statement {
            Statement::ExportDeclaration(export) => match &export.declaration {
                // Overload signatures have no body: the implementation signature follows them.
                Declaration::FunctionDeclaration(f) if f.body.is_some() != declarations => {
                    out.extend(function_entry(f, &doc, None));
                }
                Declaration::VariableDeclaration(vars) if !declarations => {
                    for declarator in &vars.declarations {
                        out.extend(declarator_entry(declarator, &doc, None));
                    }
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(export)
                if !declarations && !export.export_kind.is_type() =>
            {
                for spec in &export.specifiers {
                    if !spec.export_kind.is_type() {
                        listed.push((
                            spec.local.name().to_string(),
                            spec.exported.name().to_string(),
                        ));
                    }
                }
            }
            Statement::ExpressionStatement(e) if !declarations => {
                listed.extend(commonjs_exports(&e.expression));
            }
            Statement::FunctionDeclaration(f) if f.body.is_some() => {
                if let Some(id) = &f.id {
                    local.insert(id.name.as_str().to_string(), (Local::Function(f), doc));
                }
            }
            Statement::VariableDeclaration(vars) => {
                for declarator in &vars.declarations {
                    if let Some(name) = binding_name(&declarator.id) {
                        local.insert(name, (Local::Declarator(declarator), doc.clone()));
                    }
                }
            }
            _ => {}
        }
    }
    for (name, exported) in listed {
        match local.get(&name) {
            Some((Local::Function(f), doc)) => out.extend(function_entry(f, doc, Some(exported))),
            Some((Local::Declarator(d), doc)) => {
                out.extend(declarator_entry(d, doc, Some(exported)));
            }
            None => {}
        }
    }
    out
}

fn commonjs_exports(expression: &Expression) -> Vec<(String, String)> {
    let Expression::AssignmentExpression(assign) = expression else {
        return Vec::new();
    };
    let is_module_exports = |e: &Expression| {
        matches!(e, Expression::StaticMemberExpression(m)
            if m.property.name == "exports"
                && matches!(&m.object, Expression::Identifier(o) if o.name == "module"))
    };
    let target = match &assign.left {
        oxc_ast::ast::AssignmentTarget::StaticMemberExpression(m) => {
            if m.property.name == "exports"
                && matches!(&m.object, Expression::Identifier(o) if o.name == "module")
            {
                None
            } else if is_module_exports(&m.object)
                || matches!(&m.object, Expression::Identifier(o) if o.name == "exports")
            {
                Some(m.property.name.to_string())
            } else {
                return Vec::new();
            }
        }
        _ => return Vec::new(),
    };
    match (target, &assign.right) {
        (Some(exported), Expression::Identifier(local)) => {
            vec![(local.name.to_string(), exported)]
        }
        (None, Expression::ObjectExpression(object)) => object
            .properties
            .iter()
            .filter_map(|p| match p {
                oxc_ast::ast::ObjectPropertyKind::ObjectProperty(prop) if !prop.computed => {
                    let exported = match &prop.key {
                        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                        oxc_ast::ast::PropertyKey::StringLiteral(s) => {
                            s.value.as_str()?.to_string()
                        }
                        _ => return None,
                    };
                    match &prop.value {
                        Expression::Identifier(local) => Some((local.name.to_string(), exported)),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

enum Local<'b, 'a> {
    Function(&'b oxc_ast::ast::Function<'a>),
    Declarator(&'b oxc_ast::ast::VariableDeclarator<'a>),
}

/// Type parameters as `T`, or `T extends <constraint>` when constrained.
fn type_params_of(
    tp: Option<&oxc_ast::ast::TSTypeParameterDeclaration>,
    text: &dyn Fn(oxc_span::Span) -> String,
) -> Vec<String> {
    tp.map(|tp| {
        tp.params
            .iter()
            .map(|p| match &p.constraint {
                Some(c) => format!("{} extends {}", p.name.name.as_str(), text(c.span())),
                None => p.name.name.as_str().to_string(),
            })
            .collect()
    })
    .unwrap_or_default()
}

fn uses_instanceof(code: &str) -> bool {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .any(|word| word == "instanceof")
}

/// A top-level interface or type alias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalType {
    pub name: String,
    /// The whole interface, or the right-hand side of a type alias.
    pub text: String,
    /// A type alias without type parameters: its uses can be replaced by `text`.
    pub alias: bool,
}

/// Interfaces and type aliases declared at the top level of a file.
pub fn local_types(path: &str, source: &str) -> Vec<LocalType> {
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let text = |span: oxc_span::Span| source[span.start as usize..span.end as usize].to_string();
    let mut out = Vec::new();
    for statement in &ret.program.body {
        let declaration = match statement {
            Statement::ExportDeclaration(export) => &export.declaration,
            Statement::TSInterfaceDeclaration(_) | Statement::TSTypeAliasDeclaration(_) => {
                match statement.as_declaration() {
                    Some(d) => d,
                    None => continue,
                }
            }
            _ => continue,
        };
        match declaration {
            Declaration::TSInterfaceDeclaration(d) => out.push(LocalType {
                name: d.id.name.as_str().to_string(),
                text: text(d.span),
                alias: false,
            }),
            Declaration::TSTypeAliasDeclaration(d) => out.push(LocalType {
                name: d.id.name.as_str().to_string(),
                text: text(d.type_annotation.span()),
                alias: d.type_parameters.is_none(),
            }),
            _ => {}
        }
    }
    out
}

fn binding_name(pattern: &BindingPattern) -> Option<String> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str().to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_function_and_arrow_exports() {
        let found = exported_functions(
            "a.ts",
            "export function score(query: string, text: string): number { return 1 }\n\
             export const words = (text: string, ...extra: string[]): string[] => []\n\
             export async function load(path?: string, strict = false) {}\n\
             function hidden() {}\n\
             export const value = 1\n",
        );
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["score", "words", "load"]);
        assert_eq!(found[0].params[1].ts_type, "string");
        assert_eq!(found[0].returns, "number");
        assert_eq!(found[1].line, 2);
        assert!(found[1].params[1].rest);
        assert!(found[2].is_async && found[2].params[0].optional);
        assert_eq!(
            (
                found[2].params[1].ts_type.as_str(),
                found[2].params[1].optional
            ),
            ("boolean", true)
        );
        assert_eq!(found[2].returns, "unknown");
    }

    #[test]
    fn reads_jsdoc_types_of_untyped_parameters() {
        let found = exported_functions(
            "m.js",
            "/**\n * @param {string} source\n * @param {?number} [limit]\n * @returns {string[]}\n */\n\
             export function parse(source, limit = 0) { return [] }\n\
             /** @param {...string} parts */\n\
             export const join = (...parts) => parts.join('')\n\
             /** @param {function(string): number} f */\n\
             export function apply(f, n = 1) {}\n",
        );
        let types = |i: usize| -> Vec<(&str, bool)> {
            found[i]
                .params
                .iter()
                .map(|p| (p.ts_type.as_str(), p.optional))
                .collect()
        };
        assert_eq!(types(0), [("string", false), ("(number) | null", true)]);
        assert_eq!(found[0].returns, "string[]");
        assert_eq!(types(1), [("(string)[]", true)]);
        assert_eq!(found[1].returns, "unknown");
        assert_eq!(types(2), [("unknown", false), ("number", true)]);
    }

    #[test]
    fn reads_classes() {
        let classes = exported_classes(
            "a.ts",
            "export class Counter {\n  private lines: string[] = []\n  #seen = 0\n  constructor(initial: string, private readonly sep = '\\n') {}\n\
             add(text: string): number { return 1 }\n  private hidden(): void {}\n  values(): string[] { return [] }\n}\n\
             export class Box extends Counter { get size(): number { return 1 } static of(): Box { return new Box('') } }\n\
             export class Point { x = 1\n  constructor(public y: number) {}\n  clone(): Point { return this } }\n",
        );
        assert_eq!(classes.len(), 3);
        let counter = &classes[0];
        assert_eq!(counter.blocker, None);
        assert_eq!(counter.constructor.len(), 2);
        assert_eq!(
            counter
                .methods
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            ["add", "values"]
        );
        let blocked = classes[1].blocker.as_deref().unwrap();
        assert!(
            blocked.contains("extends")
                && blocked.contains("accessor")
                && blocked.contains("static")
        );
        let point = classes[2].blocker.as_deref().unwrap();
        assert!(
            point.contains("public field `x`")
                && point.contains("parameter properties")
                && point.contains("returns an instance")
        );
    }

    #[test]
    fn notices_instanceof() {
        let found = exported_functions(
            "a.ts",
            "export function isErr(e: unknown): boolean { return e instanceof Error }\n\
             export const plain = (instanceofCount: number): number => instanceofCount\n",
        );
        assert!(found[0].uses_instanceof);
        assert!(!found[1].uses_instanceof);
    }

    #[test]
    fn reads_local_types() {
        let types = local_types(
            "a.ts",
            "export interface LineIndex { readonly line: (n: number) => string }\ntype Id = string\n",
        );
        let names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["LineIndex", "Id"]);
        assert!(types[0].text.contains("=>") && !types[0].alias);
        assert_eq!((types[1].text.as_str(), types[1].alias), ("string", true));
    }

    #[test]
    fn reads_export_lists() {
        let found = exported_functions(
            "a.js",
            "function printDoc(doc, options) { return doc }\n\
             const width = (text) => text.length\n\
             function hidden() {}\n\
             const value = 1\n\
             export { printDoc, width as getWidth, value };\n",
        );
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["printDoc", "getWidth"]);
        assert_eq!(found[0].params.len(), 2);
        assert_eq!(found[1].line, 2);
    }

    #[test]
    fn reads_commonjs_exports() {
        let found = exported_functions(
            "a.js",
            "\"use strict\";\n\
             async function lintText(args, text) { return text }\n\
             const width = (text) => text.length\n\
             function helper() {}\n\
             const value = 1\n\
             module.exports = { lintText, getWidth: width, value, other: () => 1 };\n\
             module.exports.helper = helper;\n",
        );
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["lintText", "getWidth", "helper"]);
        assert!(found[0].is_async);
        assert_eq!(found[0].params.len(), 2);
        let exported = exported_functions("b.js", "function f(a) {}\nexports.f = f;\n");
        assert_eq!(exported.len(), 1);
        assert!(exported_functions("c.js", "function f() {}\nfoo.exports = { f };\n").is_empty());
    }
}
