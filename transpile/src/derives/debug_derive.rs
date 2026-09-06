//! The `debug()` a `#[derive(Debug)]` writes.
//!
//! For: `{:?}` has to print something, and what rustc's derive prints is a
//! fixed shape built from the type's own name and its field names. That shape
//! is what this emits, so a `{:?}` in a panic message or a log line reads the
//! same in the port as it did in Rust.
//!
//! rustc prints a struct as `Name { field: value }`, a tuple struct as
//! `Name(value)`, a unit as `Name`, and an enum variant by the *variant's* name
//! alone, with no type name in front of it. Each field prints through its own
//! `Debug`, which `debug_fmt` resolves from the field's type.

use proc_macro2::Span;

use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, FieldInfo, StructInfo};

use super::debug_fmt::debug_expr;
use super::Gap;

/// Fields written `_0`, `_1` are a tuple's, and print in parentheses.
fn is_tuple(fields: &[FieldInfo]) -> bool {
    !fields.is_empty()
        && fields
            .iter()
            .all(|f| f.name.as_deref().is_some_and(|n| n.starts_with('_') && n[1..].chars().all(|c| c.is_ascii_digit())))
}

/// The body of `debug()` for a struct, and whatever the port could not render.
pub fn struct_debug(reg: &TypeRegistry, s: &StructInfo) -> (String, Vec<Gap>) {
    let mut gaps = Vec::new();
    let body = shape(reg, &s.name, &s.fields, "this.", s.span, &mut gaps);
    (
        format!("\n  debug(): string {{\n    return {};\n  }}\n", body),
        gaps,
    )
}

/// The body of `debug()` for an enum. Each arm is the variant's own shape.
pub fn enum_debug(reg: &TypeRegistry, e: &EnumInfo) -> (String, Vec<Gap>) {
    let mut gaps = Vec::new();
    let mut out = String::from("\n  debug(): string {\n    return this.match({\n");
    for variant in &e.variants {
        let body = shape(reg, &variant.name, &variant.fields, "v.", variant.span, &mut gaps);
        if variant.fields.is_empty() {
            out.push_str(&format!("      {}: () => {},\n", variant.name, body));
        } else {
            out.push_str(&format!("      {}: (v) => {},\n", variant.name, body));
        }
    }
    out.push_str("    });\n  }\n");
    (out, gaps)
}

/// One name and its fields, in the shape rustc's derive prints.
fn shape(
    reg: &TypeRegistry,
    name: &str,
    fields: &[FieldInfo],
    receiver: &str,
    at: Span,
    gaps: &mut Vec<Gap>,
) -> String {
    if fields.is_empty() {
        return crate::macros::format_emit::quoted(name);
    }
    let rendered: Vec<(Option<String>, String)> = fields
        .iter()
        .filter_map(|f| {
            let field = f.name.as_deref()?;
            let expr = format!("{}{}", receiver, field);
            let value = match debug_expr(reg, f.ty.as_ref(), &expr) {
                Ok(value) => value,
                Err(why) => {
                    gaps.push((
                        at,
                        format!(
                            "`{}`'s `{}` prints under Debug as whatever its `toString` says, \
                             because {}",
                            name, field, why
                        ),
                    ));
                    expr
                }
            };
            Some((Some(field.to_string()), value))
        })
        .collect();

    if is_tuple(fields) {
        let parts: Vec<String> = rendered.iter().map(|(_, v)| format!("${{{}}}", v)).collect();
        format!("`{}({})`", name, parts.join(", "))
    } else {
        let parts: Vec<String> = rendered
            .iter()
            .map(|(n, v)| format!("{}: ${{{}}}", n.as_deref().unwrap_or("_"), v))
            .collect();
        format!("`{} {{ {} }}`", name, parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn struct_of<'a>(f: &'a Fixture, name: &str) -> &'a StructInfo {
        f.files[0]
            .file
            .structs
            .iter()
            .find(|s| s.name == name)
            .expect("struct")
    }

    fn enum_of<'a>(f: &'a Fixture, name: &str) -> &'a EnumInfo {
        f.files[0]
            .file
            .enums
            .iter()
            .find(|e| e.name == name)
            .expect("enum")
    }

    #[test]
    fn a_named_struct_prints_its_fields() {
        let f = Fixture::build(&[(
            "lib.rs",
            "#[derive(Debug)] pub struct S { pub name: String, pub n: u32 }",
        )]);
        let (ts, gaps) = struct_debug(&f.reg, struct_of(&f, "S"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(
            ts.contains("return `S { name: ${JSON.stringify(this.name)}, n: ${String(this.n)} }`;"),
            "{}",
            ts
        );
    }

    #[test]
    fn a_tuple_struct_prints_in_parentheses() {
        let f = Fixture::build(&[("lib.rs", "#[derive(Debug)] pub struct S(pub u32);")]);
        let (ts, _) = struct_debug(&f.reg, struct_of(&f, "S"));
        assert!(ts.contains("return `S(${String(this._0)})`;"), "{}", ts);
    }

    #[test]
    fn a_unit_struct_is_its_own_name() {
        let f = Fixture::build(&[("lib.rs", "#[derive(Debug)] pub struct S;")]);
        let (ts, _) = struct_debug(&f.reg, struct_of(&f, "S"));
        assert!(ts.contains("return 'S';"), "{}", ts);
    }

    #[test]
    fn an_enum_prints_the_variant_name_without_the_type() {
        let f = Fixture::build(&[(
            "lib.rs",
            "#[derive(Debug)] pub enum E { A, B(String), C { n: u32 } }",
        )]);
        let (ts, gaps) = enum_debug(&f.reg, enum_of(&f, "E"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(ts.contains("A: () => 'A',"), "{}", ts);
        assert!(ts.contains("B: (v) => `B(${JSON.stringify(v._0)})`,"), "{}", ts);
        assert!(ts.contains("C: (v) => `C { n: ${String(v.n)} }`,"), "{}", ts);
    }

    #[test]
    fn a_field_with_no_debug_is_reported_and_printed_by_its_to_string() {
        let f = Fixture::build(&[(
            "lib.rs",
            "pub struct Inner;\n#[derive(Debug)] pub struct S { pub inner: Inner }",
        )]);
        let (ts, gaps) = struct_debug(&f.reg, struct_of(&f, "S"));
        assert_eq!(gaps.len(), 1, "{:?}", gaps);
        assert!(gaps[0].1.contains("no Debug"), "{}", gaps[0].1);
        assert!(ts.contains("${this.inner}"), "{}", ts);
    }
}
