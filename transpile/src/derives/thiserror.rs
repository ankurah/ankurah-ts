//! `#[derive(thiserror::Error)]`, emitted rather than expanded.
//!
//! For: an error's text is part of what a program does — it goes into logs, into
//! test assertions, and back to a user — so the port has to print the same
//! sentence the Rust program printed. thiserror builds that sentence from the
//! `#[error("..")]` on each variant, and this writes the same sentence in
//! TypeScript.
//!
//! Two things come out of the derive. The `Display` becomes `toString()`, which
//! overrides the one `Enum` provides, because the derived one says what went
//! wrong and the base one says only which variant this is. Each `#[from]` field
//! becomes a `static from(inner)`, which is the conversion a `?` calls when it
//! carries an inner error out through this type.
//!
//! A placeholder names a variant's field: `{0}` is the first field of a tuple
//! variant, which emission writes as `_0`, and `{name}` is a named field. The
//! variant name never appears in the text — thiserror prints the message alone.

use proc_macro2::Span;

use crate::registry::TypeRegistry;
use crate::ty::Ty;
use crate::types::{EnumInfo, FieldInfo};

use crate::macros::format_emit::{render, Operands};
use crate::macros::format_spec::{parse, ArgRef};

use super::debug_fmt::debug_expr;
use super::Gap;

/// Is this the thiserror derive? It is written `Error` behind a
/// `use thiserror::Error`, and `thiserror::Error` where the import is absent.
/// `std::error::Error` cannot be derived, so an `Error` in a derive list is
/// thiserror's in every case rustc accepts.
pub fn is_thiserror(derives: &[String]) -> bool {
    derives
        .iter()
        .any(|d| d == "Error" || d.replace(' ', "") == "thiserror::Error")
}

/// The `toString()` and the `static from`s this enum's derive writes, with
/// whatever the port could not carry over.
pub fn enum_error(
    reg: &TypeRegistry,
    self_id: Option<crate::ty::TypeId>,
    e: &EnumInfo,
) -> (String, Vec<Gap>) {
    let mut gaps = Vec::new();
    let mut out = String::new();
    out.push_str(&display(reg, e, &mut gaps));
    out.push_str(&source_accessor(e));
    out.push_str(&from_impls(reg, self_id, e, &mut gaps));
    (out, gaps)
}

/// `Error::source`: the error this one wraps, where a variant names one with
/// `#[source]` or with the `#[from]` that implies it.
///
/// Rust's `Error::source` is what a chain-printing helper walks, and the port
/// wrote nothing at all for it — a `#[source]` field was read as an ordinary
/// field and the chain stopped at the outermost error. A variant with no source
/// answers `null`, which is what Rust's default `source` answers.
fn source_accessor(e: &EnumInfo) -> String {
    let carrying: Vec<(&str, &str)> = e
        .variants
        .iter()
        .filter_map(|v| {
            let field = v.fields.iter().find(|f| f.is_source)?;
            Some((v.name.as_str(), field.name.as_deref()?))
        })
        .collect();
    if carrying.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n  /** The error this one wraps: Rust's `Error::source`. */\n  source(): unknown {\n    switch (this.type) {\n");
    for (variant, field) in carrying {
        out.push_str(&format!("      case '{}': return (this.value as any).{};\n", variant, field));
    }
    out.push_str("      default: return null;\n    }\n  }\n");
    out
}

/// The `Display` arm of an `#[error(transparent)]` variant: the wrapped error's
/// own text.
fn transparent_arm(variant: &crate::types::VariantInfo) -> Option<String> {
    // The wrapped error is the one field a transparent variant carries; where
    // it says which with `#[source]` or `#[from]`, that one.
    let field = variant
        .fields
        .iter()
        .find(|f| f.is_source)
        .or_else(|| variant.fields.first())?;
    let name = field.name.clone()?;
    Some(format!("      {}: (v) => v.{}.toString(),\n", variant.name, name))
}

/// `impl Display`: one arm per variant, rendering that variant's format string.
fn display(reg: &TypeRegistry, e: &EnumInfo, gaps: &mut Vec<Gap>) -> String {
    let mut arms = String::new();
    for variant in &e.variants {
        // `#[error(transparent)]`: the variant's text IS the wrapped error's.
        // The port used to write the variant's own name here, because the
        // attribute reader saw only the string form.
        if variant.error_text == Some(crate::types::ErrorText::Transparent) {
            match transparent_arm(variant) {
                Some(arm) => {
                    arms.push_str(&arm);
                    continue;
                }
                None => gaps.push((
                    variant.span,
                    format!(
                        "`{}::{}` is `#[error(transparent)]`, which forwards its text to the \
                         error it wraps, and this variant wraps none the engine can name",
                        e.name, variant.name
                    ),
                )),
            }
        }
        let Some(crate::types::ErrorText::Format(format)) = &variant.error_text else {
            gaps.push((
                variant.span,
                format!(
                    "`{}::{}` carries no `#[error(\"..\")]` the engine could read, so its text \
                     is whatever the base `toString` prints",
                    e.name, variant.name
                ),
            ));
            arms.push_str(&format!(
                "      {}: () => '{}::{}',\n",
                variant.name, e.name, variant.name
            ));
            continue;
        };
        let pieces = match parse(format) {
            Ok(pieces) => pieces,
            Err(why) => {
                gaps.push((
                    variant.span,
                    format!(
                        "`{}::{}`'s `#[error]` has {}, so its text is the format string as written",
                        e.name, variant.name, why
                    ),
                ));
                arms.push_str(&format!(
                    "      {}: () => {},\n",
                    variant.name,
                    crate::macros::format_emit::quoted(format)
                ));
                continue;
            }
        };
        let mut operands = Fields {
            reg,
            fields: &variant.fields,
            next: 0,
            owner: format!("{}::{}", e.name, variant.name),
            at: variant.span,
            gaps,
        };
        let text = render(&pieces, &mut operands);
        // A variant whose message never names a field takes no payload
        // parameter, which is what keeps the arm honest about what it reads.
        let reads_a_field = text.contains("v.");
        if reads_a_field {
            arms.push_str(&format!("      {}: (v) => {},\n", variant.name, text));
        } else {
            arms.push_str(&format!("      {}: () => {},\n", variant.name, text));
        }
    }
    format!(
        "\n  override toString(): string {{\n    return this.match({{\n{}    }});\n  }}\n",
        arms
    )
}

/// `impl From<Inner> for Self` for each `#[from]` field: what a `?` calls when
/// it carries an inner error out through this type.
fn from_impls(
    reg: &TypeRegistry,
    self_id: Option<crate::ty::TypeId>,
    e: &EnumInfo,
    gaps: &mut Vec<Gap>,
) -> String {
    let mut out = String::new();
    let mut seen: Vec<String> = Vec::new();
    for variant in &e.variants {
        for field in &variant.fields {
            if !field.is_from {
                continue;
            }
            let inner = field.ts_ty(reg);
            // The static's name comes from the source type as *written*, and it
            // is computed by the function every other `From` impl's method is
            // named with — a `?` reaching for this conversion asks that same
            // function, and two rules would have drifted apart.
            let method = crate::emit::disambiguate_trait_method(
                "from",
                "From",
                std::slice::from_ref(&crate::name_map::map_type(&field.rust_ty)),
                "",
                self_id,
            );
            // Two sources whose written spellings agree are one TypeScript
            // name, and a class cannot hold both.
            if seen.contains(&method) {
                gaps.push((
                    variant.span,
                    format!(
                        "`{}` converts from two types whose names both spell `{}`, and a class \
                         cannot hold both, so only the first is written",
                        e.name, method
                    ),
                ));
                continue;
            }
            seen.push(method.clone());
            let field_name = field.name.as_deref().unwrap_or("_0");
            out.push_str(&format!(
                "\n  static {}(inner: {}): {} {{\n    return new {}('{}', {{ {}: inner }});\n  }}\n",
                method, inner, e.name, e.name, variant.name, field_name
            ));
        }
    }
    out
}

/// A variant's fields, read the way a format string names them.
struct Fields<'a> {
    reg: &'a TypeRegistry,
    fields: &'a [FieldInfo],
    next: usize,
    owner: String,
    at: Span,
    gaps: &'a mut Vec<Gap>,
}

impl Operands for Fields<'_> {
    fn operand(&mut self, which: &ArgRef, _needs_type: bool) -> Option<(String, Option<Ty>)> {
        let field = match which {
            ArgRef::Next => {
                let at = self.next;
                self.next += 1;
                self.fields.get(at)?
            }
            ArgRef::Positional(index) => self.fields.get(*index)?,
            ArgRef::Named(name) => self
                .fields
                .iter()
                .find(|f| f.name.as_deref() == Some(name.as_str()))?,
        };
        Some((format!("v.{}", field.name.as_deref()?), field.ty.clone()))
    }

    fn debug(&mut self, expr: &str, ty: Option<&Ty>, _alternate: bool) -> String {
        match debug_expr(self.reg, ty, expr) {
            Ok(text) => text,
            Err(why) => {
                self.gaps.push((
                    self.at,
                    format!(
                        "`{}`'s message prints `{}` with `{{:?}}`, and it prints as whatever its \
                         `toString` says, because {}",
                        self.owner, expr, why
                    ),
                ));
                expr.to_string()
            }
        }
    }

    fn report(&mut self, what: String) {
        self.gaps
            .push((self.at, format!("in `{}`'s message, {}", self.owner, what)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", src)])
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
    fn a_message_with_no_fields_is_a_string_literal() {
        let f = built(
            "use thiserror::Error;\n\
             #[derive(Debug, Error)] pub enum E { #[error(\"Empty expression\")] Empty }",
        );
        let (ts, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(ts.contains("Empty: () => 'Empty expression',"), "{}", ts);
        assert!(ts.contains("override toString(): string"), "{}", ts);
    }

    #[test]
    fn a_positional_placeholder_reads_the_tuple_field() {
        let f = built(
            "use thiserror::Error;\n\
             #[derive(Debug, Error)] pub enum E { #[error(\"Syntax error: {0}\")] Syntax(String) }",
        );
        let (ts, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(ts.contains("Syntax: (v) => `Syntax error: ${v._0}`,"), "{}", ts);
    }

    #[test]
    fn named_placeholders_read_named_fields() {
        let f = built(
            "use thiserror::Error;\n\
             #[derive(Debug, Error)] pub enum E {\n\
               #[error(\"Placeholder count mismatch: expected {expected}, found {found}\")]\n\
               Mismatch { expected: usize, found: usize },\n\
             }",
        );
        let (ts, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(
            ts.contains(
                "Mismatch: (v) => `Placeholder count mismatch: expected ${v.expected}, found ${v.found}`,"
            ),
            "{}",
            ts
        );
    }

    #[test]
    fn a_debug_placeholder_uses_the_field_type_s_debug() {
        let f = built(
            "use thiserror::Error;\n\
             #[derive(Debug)] pub enum Rule { A }\n\
             #[derive(Debug, Error)] pub enum E {\n\
               #[error(\"Expected {expected}, got {got:?}\")]\n\
               Unexpected { expected: String, got: Rule },\n\
             }",
        );
        let (ts, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(gaps.is_empty(), "{:?}", gaps);
        assert!(
            ts.contains("Unexpected: (v) => `Expected ${v.expected}, got ${v.got.debug()}`,"),
            "{}",
            ts
        );
    }

    #[test]
    fn a_payload_the_message_never_names_takes_no_parameter() {
        let f = built(
            "use thiserror::Error;\n\
             pub struct Denied;\n\
             #[derive(Debug, Error)] pub enum E { #[error(\"access denied\")] Denied(Denied) }",
        );
        let (ts, _) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(ts.contains("Denied: () => 'access denied',"), "{}", ts);
    }

    #[test]
    fn a_from_field_becomes_a_static_from() {
        let f = built(
            "use thiserror::Error;\n\
             pub struct Inner;\n\
             #[derive(Debug, Error)] pub enum E { #[error(\"other: {0}\")] Other(#[from] Inner) }",
        );
        let (ts, _) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert!(
            ts.contains("static fromInner(inner: Inner): E {\n    return new E('Other', { _0: inner });"),
            "{}",
            ts
        );
    }

    #[test]
    fn each_from_field_gets_a_static_named_for_its_own_source() {
        let f = built(
            "use thiserror::Error;\n\
             pub struct A;\npub struct B;\n\
             #[derive(Debug, Error)] pub enum E {\n\
               #[error(\"a: {0}\")] A(#[from] A),\n\
               #[error(\"b: {0}\")] B(#[from] B),\n\
             }",
        );
        let (ts, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        // Two different source types are two different names, so both are
        // written: only a collision between their names is a gap.
        assert_eq!(ts.matches("static from").count(), 2, "{}", ts);
        assert!(ts.contains("static fromA(inner: A)"), "{}", ts);
        assert!(ts.contains("static fromB(inner: B)"), "{}", ts);
        assert!(gaps.is_empty(), "{:?}", gaps);
    }

    #[test]
    fn a_variant_with_no_error_attribute_is_reported() {
        let f = built(
            "use thiserror::Error;\n#[derive(Debug, Error)] pub enum E { Bare }",
        );
        let (_, gaps) = enum_error(&f.reg, None, enum_of(&f, "E"));
        assert_eq!(gaps.len(), 1, "{:?}", gaps);
        assert!(gaps[0].1.contains("carries no `#[error"), "{}", gaps[0].1);
    }

    /// `#[error(transparent)]` forwards the variant's text to the error it
    /// wraps. The attribute reader saw only the string form, so the variant's
    /// own name was written instead — reported, and wrong.
    #[test]
    fn a_transparent_variant_forwards_its_text() {
        let mut f = crate::testing::Fixture::build(&[(
            "lib.rs",
            "use thiserror::Error;\n\
             #[derive(Error, Debug)]\n\
             pub enum Inner { #[error(\"boom\")] Boom }\n\
             #[derive(Error, Debug)]\n\
             pub enum Outer { #[error(transparent)] Passed(#[from] Inner) }",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("Passed: (v) => v._0.toString(),"), "{}", ts);
        assert!(!ts.contains("'Outer::Passed'"), "{}", ts);
    }

    /// `#[source]`, and the `#[from]` that implies it, name the error this one
    /// wraps: `Error::source` answers it, and the port wrote nothing at all.
    #[test]
    fn a_source_field_is_reachable_through_source() {
        let mut f = crate::testing::Fixture::build(&[(
            "lib.rs",
            "use thiserror::Error;\n\
             #[derive(Error, Debug)]\n\
             pub enum Inner { #[error(\"boom\")] Boom }\n\
             #[derive(Error, Debug)]\n\
             pub enum Outer {\n\
               #[error(\"wrapped\")] Sourced { #[source] cause: Inner },\n\
               #[error(\"plain\")] Plain,\n\
             }",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("source(): unknown"), "{}", ts);
        assert!(ts.contains("case 'Sourced': return (this.value as any).cause;"), "{}", ts);
        assert!(ts.contains("default: return null;"), "{}", ts);
    }
}
