//! The shape of a TS type, for generating values that fit it.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    PropertyKey, Statement, TSLiteral, TSSignature, TSTupleElement, TSType, TSTypeName,
};
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::exports::LocalType;

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Str,
    Num,
    /// `int` in the type dialect.
    Int,
    Bool,
    Null,
    Undefined,
    LitStr(String),
    LitNum(f64),
    LitBool(bool),
    Union(Vec<Shape>),
    Array(Box<Shape>),
    Tuple(Vec<Shape>),
    /// Named properties `(name, optional, shape)`, and the value shape of an index signature.
    Object(Vec<(String, bool, Shape)>, Option<Box<Shape>>),
    /// A named type, resolved against the types in scope when values are generated.
    Ref(String, Vec<Shape>),
    /// Anything: `any`, `unknown`, or what this module does not model.
    Any,
}

/// Parses a type as written in TS source.
pub fn parse(text: &str) -> Shape {
    let source = format!("type __T = {text};");
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, &source, SourceType::ts()).parse();
    for statement in &ret.program.body {
        if let Statement::TSTypeAliasDeclaration(alias) = statement {
            return convert(&alias.type_annotation);
        }
    }
    Shape::Any
}

/// The shape a named type stands for, from the types in scope.
pub fn resolve(name: &str, args: &[Shape], types: &[LocalType]) -> Shape {
    match (name, args) {
        ("Array" | "ReadonlyArray" | "Set" | "ReadonlySet", [item]) => {
            Shape::Array(Box::new(item.clone()))
        }
        ("Record", [_, value]) => Shape::Object(Vec::new(), Some(Box::new(value.clone()))),
        ("Readonly" | "Promise" | "NonNullable", [inner]) => inner.clone(),
        ("Partial", [inner]) => match inner {
            Shape::Object(props, index) => Shape::Object(
                props
                    .iter()
                    .map(|(n, _, s)| (n.clone(), true, s.clone()))
                    .collect(),
                index.clone(),
            ),
            other => other.clone(),
        },
        ("int", []) => Shape::Int,
        _ => match types.iter().find(|t| t.name == name) {
            Some(t) if t.alias => parse(&t.text),
            // An interface: its body is a type literal (`extends` is not followed).
            Some(t) => t.text.find('{').map_or(Shape::Any, |i| parse(&t.text[i..])),
            None => Shape::Any,
        },
    }
}

fn convert(ty: &TSType) -> Shape {
    match ty {
        TSType::TSStringKeyword(_) => Shape::Str,
        TSType::TSNumberKeyword(_) => Shape::Num,
        TSType::TSBooleanKeyword(_) => Shape::Bool,
        TSType::TSNullKeyword(_) => Shape::Null,
        TSType::TSUndefinedKeyword(_) | TSType::TSVoidKeyword(_) => Shape::Undefined,
        TSType::TSLiteralType(lit) => match &lit.literal {
            TSLiteral::StringLiteral(s) => Shape::LitStr(s.value.display().to_string()),
            TSLiteral::NumericLiteral(n) => Shape::LitNum(n.value),
            TSLiteral::BooleanLiteral(b) => Shape::LitBool(b.value),
            _ => Shape::Any,
        },
        TSType::TSUnionType(u) => Shape::Union(u.types.iter().map(convert).collect()),
        TSType::TSArrayType(a) => Shape::Array(Box::new(convert(&a.element_type))),
        TSType::TSTypeOperatorType(op) => convert(&op.type_annotation),
        TSType::TSParenthesizedType(p) => convert(&p.type_annotation),
        TSType::TSTupleType(t) => Shape::Tuple(
            t.element_types
                .iter()
                .map(|e| match e {
                    TSTupleElement::TSOptionalType(o) => convert(&o.type_annotation),
                    TSTupleElement::TSRestType(r) => convert(&r.type_annotation),
                    other => other.as_ts_type().map_or(Shape::Any, convert),
                })
                .collect(),
        ),
        TSType::TSTypeLiteral(lit) => {
            let mut props = Vec::new();
            let mut index = None;
            for member in &lit.members {
                match member {
                    TSSignature::TSPropertySignature(p) => {
                        let Some(name) = property_name(&p.key) else {
                            continue;
                        };
                        let shape = p
                            .type_annotation
                            .as_ref()
                            .map_or(Shape::Any, |a| convert(&a.type_annotation));
                        props.push((name, p.optional, shape));
                    }
                    TSSignature::TSIndexSignature(s) => {
                        index = Some(Box::new(convert(&s.type_annotation.type_annotation)));
                    }
                    _ => {}
                }
            }
            Shape::Object(props, index)
        }
        TSType::TSIntersectionType(i) => {
            // `A & { b: string }`: merge the object parts.
            let mut props = Vec::new();
            let mut index = None;
            for part in i.types.iter().map(convert) {
                match part {
                    Shape::Object(p, idx) => {
                        props.extend(p);
                        index = index.or(idx);
                    }
                    other => return other,
                }
            }
            Shape::Object(props, index)
        }
        TSType::TSTypeReference(r) => {
            let name = match &r.type_name {
                TSTypeName::IdentifierReference(id) => id.name.as_str().to_string(),
                _ => return Shape::Any,
            };
            let args = r
                .type_arguments
                .as_ref()
                .map(|a| a.params.iter().map(convert).collect())
                .unwrap_or_default();
            Shape::Ref(name, args)
        }
        _ => Shape::Any,
    }
}

fn property_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_string()),
        PropertyKey::StringLiteral(s) => Some(s.value.display().to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shapes() {
        assert_eq!(
            parse("{ readonly key: string, n?: number, kind: 'a' | 'b' }"),
            Shape::Object(
                vec![
                    ("key".into(), false, Shape::Str),
                    ("n".into(), true, Shape::Num),
                    (
                        "kind".into(),
                        false,
                        Shape::Union(vec![Shape::LitStr("a".into()), Shape::LitStr("b".into())])
                    ),
                ],
                None
            )
        );
        assert_eq!(
            parse("readonly string[]"),
            Shape::Array(Box::new(Shape::Str))
        );
        assert_eq!(
            parse("Record<string, Leaf>"),
            Shape::Ref(
                "Record".into(),
                vec![Shape::Str, Shape::Ref("Leaf".into(), vec![])]
            )
        );
        assert_eq!(
            parse("{ [k: string]: number }"),
            Shape::Object(vec![], Some(Box::new(Shape::Num)))
        );
    }

    #[test]
    fn resolves_named_types() {
        let types = super::super::exports::local_types(
            "m.ts",
            "export interface Range { readonly start: number\n readonly end: number }\nexport type Leaf = string | null\n",
        );
        assert_eq!(
            resolve("Range", &[], &types),
            Shape::Object(
                vec![
                    ("start".into(), false, Shape::Num),
                    ("end".into(), false, Shape::Num)
                ],
                None
            )
        );
        assert_eq!(
            resolve("Leaf", &[], &types),
            Shape::Union(vec![Shape::Str, Shape::Null])
        );
        assert_eq!(
            resolve("Partial", &[parse("{ a: string }")], &types),
            Shape::Object(vec![("a".into(), true, Shape::Str)], None)
        );
    }
}
