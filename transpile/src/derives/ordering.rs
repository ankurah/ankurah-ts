//! `#[derive(PartialOrd)]` and `#[derive(Ord)]`, written out.
//!
//! For: a derived `Ord` is what a sort, a `BTreeMap` and a `min`/`max` run on,
//! and the port emitted `compareTo(other) { throw new Error('TODO'); }` — a
//! method that compiles and then throws the moment anything orders the type.
//!
//! Rust's derive compares field by field in declaration order and stops at the
//! first pair that differs; for an enum it compares the variants' declaration
//! order first and the payload only within one variant. That is what is written
//! here, over the resolved field types, and a field the port cannot order is
//! reported rather than compared by identity.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;
use crate::types::{EnumInfo, FieldInfo, StructInfo};

use super::Gap;

/// `compareTo` for a struct: its fields in declaration order.
pub fn struct_compare(reg: &TypeRegistry, s: &StructInfo, full_type: &str) -> (String, Vec<Gap>) {
    let mut gaps = Vec::new();
    let body = chain(
        reg,
        &s.fields,
        &|name| format!("this.{}", name),
        &|name| format!("other.{}", name),
        &s.name,
        &mut gaps,
    );
    (
        format!(
            "\n  compareTo(other: {}): number {{\n{}  }}\n",
            full_type, body
        ),
        gaps,
    )
}

/// `compareTo` for an enum: the variants' declaration order first, then the
/// payload of the variant both values share.
pub fn enum_compare(reg: &TypeRegistry, e: &EnumInfo, full_type: &str) -> (String, Vec<Gap>) {
    let mut gaps = Vec::new();
    let order: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
    let mut out = format!("\n  compareTo(other: {}): number {{\n", full_type);
    out.push_str(&format!("    const order = [{}];\n", order.join(", ")));
    out.push_str("    const a = order.indexOf(this.type);\n");
    out.push_str("    const b = order.indexOf(other.type);\n");
    out.push_str("    if (a !== b) return a < b ? -1 : 1;\n");
    let carrying: Vec<&crate::types::VariantInfo> =
        e.variants.iter().filter(|v| !v.fields.is_empty()).collect();
    if !carrying.is_empty() {
        out.push_str("    switch (this.type) {\n");
        for variant in carrying {
            out.push_str(&format!("      case '{}': {{\n", variant.name));
            let body = chain(
                reg,
                &variant.fields,
                &|name| format!("(this.value as any).{}", name),
                &|name| format!("(other.value as any).{}", name),
                &format!("{}::{}", e.name, variant.name),
                &mut gaps,
            );
            out.push_str(&indent_twice(&body));
            out.push_str("      }\n");
        }
        out.push_str("    }\n");
    }
    out.push_str("    return 0;\n  }\n");
    (out, gaps)
}

fn indent_twice(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::from("\n")
            } else {
                format!("    {}\n", line)
            }
        })
        .collect()
}

/// The fields compared in order, stopping at the first pair that differs.
fn chain(
    reg: &TypeRegistry,
    fields: &[FieldInfo],
    mine: &dyn Fn(&str) -> String,
    theirs: &dyn Fn(&str) -> String,
    owner: &str,
    gaps: &mut Vec<Gap>,
) -> String {
    let mut out = String::new();
    let mut first = true;
    for (index, field) in fields.iter().enumerate() {
        let name = field
            .name
            .clone()
            .unwrap_or_else(|| format!("_{}", index));
        let (left, right) = (mine(&name), theirs(&name));
        let comparison = match field.ty.as_ref() {
            Some(ty) => match compare_expr(reg, ty, &left, &right) {
                Ok(text) => text,
                Err(why) => {
                    gaps.push((
                        field.rust_ty_span(),
                        format!(
                            "`{}`'s derived ordering has no comparison for this field, because \
                             {}; the field is not compared, so two values differing only here \
                             order as equal",
                            owner, why
                        ),
                    ));
                    continue;
                }
            },
            None => {
                gaps.push((
                    field.rust_ty_span(),
                    format!(
                        "`{}`'s derived ordering has no comparison for this field, because the \
                         engine could not type it; the field is not compared",
                        owner
                    ),
                ));
                continue;
            }
        };
        if first {
            out.push_str(&format!("    let c = {};\n", comparison));
            first = false;
        } else {
            out.push_str(&format!("    c = {};\n", comparison));
        }
        out.push_str("    if (c !== 0) return c;\n");
    }
    out.push_str("    return 0;\n");
    out
}

/// What orders two values of this type, as a number, or the reason the port
/// cannot say.
///
/// A string is compared the way JavaScript compares one — by UTF-16 code unit —
/// where Rust compares a `String` by byte. The two agree for everything below
/// U+10000 and disagree on the order of an astral character against one in the
/// surrogate range; the corpus orders ids and names, and this is recorded in
/// spec 7a rather than written around.
fn compare_expr(reg: &TypeRegistry, ty: &Ty, left: &str, right: &str) -> Result<String, String> {
    match js_shape(reg, ty) {
        JsShape::SameAs(inner) => compare_expr(reg, &inner, left, right),
        JsShape::Number | JsShape::BigInt | JsShape::Str => {
            Ok(format!("{l} < {r} ? -1 : {l} > {r} ? 1 : 0", l = left, r = right))
        }
        // Rust orders `false` before `true`.
        JsShape::Boolean => Ok(format!("Number({}) - Number({})", left, right)),
        // Rust orders `None` before `Some`, and two `Some`s by their payloads.
        JsShape::Nullable(inner) => {
            let inner = compare_expr(reg, &inner, &format!("{}!", left), &format!("{}!", right))?;
            Ok(format!(
                "{l} == null || {r} == null ? ({l} == null ? 0 : 1) - ({r} == null ? 0 : 1) : ({inner})",
                l = left,
                r = right,
                inner = inner
            ))
        }
        // A sequence is ordered element by element, and a prefix comes first.
        JsShape::Bytes => Ok(sequence(left, right, "a < b ? -1 : a > b ? 1 : 0")),
        JsShape::Array(elem) => {
            let inner = compare_expr(reg, &elem, "a", "b")?;
            Ok(sequence(left, right, &inner))
        }
        // A type of the port's own compares through the method its own derive
        // wrote — which is this one.
        JsShape::Plain => Ok(format!("{}.compareTo({})", left, right)),
        other => Err(format!(
            "the port writes it as {:?}, which has no ordering",
            other
        )),
    }
}

/// Two sequences compared element by element, with the shorter one first where
/// one is a prefix of the other — which is Rust's order for a slice.
fn sequence(left: &str, right: &str, element: &str) -> String {
    format!(
        "((xs, ys) => {{ const n = Math.min(xs.length, ys.length); \
         for (let i = 0; i < n; i++) {{ const a = xs[i], b = ys[i]; \
         const d = {element}; if (d !== 0) return d; }} \
         return xs.length - ys.length; }})({left}, {right})",
        element = element,
        left = left,
        right = right
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", src)])
    }

    #[test]
    fn a_struct_compares_its_fields_in_declaration_order() {
        let f = built("#[derive(PartialOrd, Ord)] pub struct Key { pub a: u32, pub b: String }");
        let s = f.files[0].file.structs.iter().find(|s| s.name == "Key").expect("struct");
        let (ts, gaps) = struct_compare(&f.reg, s, "Key");
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(ts.contains("let c = this.a < other.a ? -1 : this.a > other.a ? 1 : 0;"), "{}", ts);
        assert!(ts.contains("c = this.b < other.b ? -1 : this.b > other.b ? 1 : 0;"), "{}", ts);
        assert_eq!(ts.matches("if (c !== 0) return c;").count(), 2, "{}", ts);
    }

    #[test]
    fn an_enum_compares_its_variants_declaration_order_first() {
        let f = built("#[derive(PartialOrd, Ord)] pub enum Step { Low, Mid(u32), High }");
        let e = f.files[0].file.enums.iter().find(|e| e.name == "Step").expect("enum");
        let (ts, gaps) = enum_compare(&f.reg, e, "Step");
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(ts.contains("const order = ['Low', 'Mid', 'High'];"), "{}", ts);
        assert!(ts.contains("if (a !== b) return a < b ? -1 : 1;"), "{}", ts);
        assert!(ts.contains("case 'Mid': {"), "{}", ts);
    }

    #[test]
    fn a_field_the_port_cannot_order_is_reported_and_not_compared() {
        let f = built(
            "use std::collections::BTreeMap;\n\
             #[derive(PartialOrd, Ord)] pub struct Key { pub m: BTreeMap<u32, u32> }",
        );
        let s = f.files[0].file.structs.iter().find(|s| s.name == "Key").expect("struct");
        let (ts, gaps) = struct_compare(&f.reg, s, "Key");
        assert_eq!(gaps.len(), 1, "{:?}", gaps);
        assert!(!ts.contains("this.m"), "{}", ts);
    }
}
