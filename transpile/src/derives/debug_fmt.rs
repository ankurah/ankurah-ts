//! `#[derive(Debug)]`, and what `{:?}` renders.
//!
//! For: a Rust program prints its data with `{:?}` in panics, assertions, error
//! messages and log lines, and the port has to print the same thing. rustc's
//! derive writes one shape — `Name { field: .. }`, `Variant(payload)`, the bare
//! name for a unit — and this writes that shape in TypeScript.
//!
//! `Debug` is a trait, so which rendering a value gets is decided by its type
//! and not by what it looks like at runtime. That is why every choice here is
//! made from the `Ty` the engine resolved: a Rust `String` prints quoted, a Rust
//! enum prints its variant name, and both are a JavaScript string. A value the
//! engine could not type has no Debug the port can name, and the caller says so
//! rather than picking one.

use crate::registry::{TypeKind, TypeRegistry};
use crate::ty::{Prim, Ty, TypeId};

/// The TypeScript expression that renders `expr` the way `{:?}` renders the
/// value it holds, or the reason the port cannot say.
pub fn debug_expr(reg: &TypeRegistry, ty: Option<&Ty>, expr: &str) -> Result<String, String> {
    let Some(ty) = ty else {
        return Err("the engine could not type it".to_string());
    };
    match ty.peel_refs() {
        // Rust quotes and escapes a string under Debug, which is what JSON's
        // own string form does.
        Ty::Str => Ok(format!("JSON.stringify({})", expr)),
        // Rust prints a `char` between single quotes, and the port writes a
        // `char` as a one-character string — so the quotes are the rendering.
        Ty::Prim(Prim::Char) => Ok(format!("`'${{{}}}'`", expr)),
        // A float keeps its decimal point: Rust's Debug for `1.0f64` is `1.0`
        // and JavaScript's `String(1.0)` is `1`, so a `Value::F64(1.0)` printed
        // `F64(1)` where Rust prints `F64(1.0)`. And `-0.0` prints with its
        // sign, which `String(-0)` drops.
        Ty::Prim(Prim::F32 | Prim::F64) => Ok(debug_float(expr)),
        Ty::Prim(_) => Ok(format!("String({})", expr)),
        Ty::Slice(elem) => sequence(reg, elem, expr),
        Ty::Array { elem, .. } => sequence(reg, elem, expr),
        // Rust prints a tuple as `(a, b)`, each element through its own Debug.
        // The port writes a tuple as an ARRAY, so each element is read by its
        // index — and the subject is read once, into a name.
        Ty::Tuple(elems) => {
            let parts: Result<Vec<String>, String> = elems
                .iter()
                .enumerate()
                .map(|(at, e)| debug_expr(reg, Some(e), &format!("$t[{}]", at)))
                .collect();
            let parts = parts?;
            let rendered: Vec<String> = parts.iter().map(|p| format!("${{{}}}", p)).collect();
            Ok(format!("(($t) => `({})`)({})", rendered.join(", "), expr))
        }
        Ty::Named { id, args } => named(reg, *id, args, expr),
        other => Err(format!("`{}` has no Debug rendering in the port", describe(other))),
    }
}

/// A float the way Rust's Debug prints one.
///
/// Rust keeps the decimal point on a whole number — `1.0f64` is `1.0`, not `1`
/// — keeps the sign on `-0.0`, and spells the infinities `inf` and `-inf`.
/// JavaScript's `String` does none of those. Written inline rather than as a
/// runtime helper, because it is one expression and the subject is read once.
fn debug_float(expr: &str) -> String {
    let finite = "Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)";
    let other = "$f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'";
    format!("(($f) => Number.isFinite($f) ? ({}) : ({}))({})", finite, other, expr)
}

/// A named type: the std containers Rust prints structurally, then the crate's
/// own types, which print through the `debug()` the derive writes for them.
fn named(reg: &TypeRegistry, id: TypeId, args: &[Ty], expr: &str) -> Result<String, String> {
    let path = reg.name_of(id);
    let leaf = path.rsplit("::").next().unwrap_or(&path);
    match leaf {
        "String" => return Ok(format!("JSON.stringify({})", expr)),
        // `Box<T>` is invisible on the wire and invisible under Debug: Rust
        // prints what is inside it. An `Rc` and an `Arc` print their payload
        // too, and the port holds that payload in `.value` — reaching through
        // is what the runtime needs, and `this.inner.debug()` on an `Arc` was
        // a TypeError.
        "Box" => {
            if let Some(inner) = args.first() {
                return debug_expr(reg, Some(inner), expr);
            }
        }
        "Rc" | "Arc" => {
            if let Some(inner) = args.first() {
                return debug_expr(reg, Some(inner), &format!("{}.value", expr));
            }
        }
        "Option" => {
            let Some(inner) = args.first() else {
                return Err("an `Option` with no element type".to_string());
            };
            // The subject is read ONCE. Written twice — the test and the
            // payload — a `{:?}` on an expression with an effect performed it
            // twice.
            let some = debug_expr(reg, Some(inner), "$v")?;
            return Ok(format!(
                "(($v) => $v === null ? 'None' : `Some(${{{}}})`)({})",
                some, expr
            ));
        }
        "Vec" | "VecDeque" => {
            let Some(inner) = args.first() else {
                return Err("a `Vec` with no element type".to_string());
            };
            return sequence(reg, inner, expr);
        }
        // Rust prints a set as `{a, b}` and a map as `{k: v, w: x}`, each part
        // through its own Debug. The port holds both in a runtime container
        // that iterates its contents, and a `BTreeMap` iterates in key order —
        // which is what the ordering note in the container says.
        "HashSet" | "BTreeSet" => {
            let Some(inner) = args.first() else {
                return Err("a set with no element type".to_string());
            };
            let each = debug_expr(reg, Some(inner), "e")?;
            return Ok(format!(
                "`{{${{Array.from({}).map((e) => {}).join(', ')}}}}`",
                expr, each
            ));
        }
        "HashMap" | "BTreeMap" => {
            let (Some(key), Some(value)) = (args.first(), args.get(1)) else {
                return Err("a map with no key or value type".to_string());
            };
            // `$p` rather than `e`: a key or a value that is itself a
            // sequence renders through an arrow whose parameter is `e`, and the
            // pair would be reading the element it shadows.
            let k = debug_expr(reg, Some(key), "$p[0]")?;
            let v = debug_expr(reg, Some(value), "$p[1]")?;
            return Ok(format!(
                "`{{${{Array.from({}).map(($p) => `${{{}}}: ${{{}}}`).join(', ')}}}}`",
                expr, k, v
            ));
        }
        _ => {}
    }

    // The crate's own types print through the method the derive emits for them.
    let Some(def) = reg.def(id) else {
        return Err(format!("`{}` is not declared", path));
    };
    if !matches!(def.kind, TypeKind::Struct | TypeKind::Enum { .. }) {
        return Err(format!("`{}` is not a struct or an enum", path));
    }
    if reg.is_system(id) {
        return Err(format!("`{}` is a std type with no Debug rendering in the port", path));
    }
    if reg.members_are_hand_written(id) {
        // Only the `[provided_impls]` entry can say whether the file declares
        // one: the engine never reads the TypeScript it did not write. Without
        // the declaration the field printed through `toString`, which for a
        // class is `[object Object]`.
        if reg.declares_debug(id) {
            return Ok(format!("{}.debug()", expr));
        }
        return Err(format!(
            "`{}`'s TypeScript is written by hand and its `[provided_impls]` entry does not say \
             it declares `debug()`, so there is none to call",
            path
        ));
    }
    if !has_debug(reg, id) {
        return Err(format!("`{}` has no Debug the engine can find", path));
    }
    Ok(format!("{}.debug()", expr))
}

/// `[a, b, c]` — Rust's Debug for every sequence.
///
/// A `Vec<u8>` is a `Uint8Array` in the port, whose own `map` builds another
/// `Uint8Array` and would turn the rendered strings back into numbers, so the
/// elements are taken out into a plain array first.
fn sequence(reg: &TypeRegistry, elem: &Ty, expr: &str) -> Result<String, String> {
    let each = debug_expr(reg, Some(elem), "e")?;
    Ok(format!(
        "`[${{Array.from({}).map((e) => {}).join(', ')}}]`",
        expr, each
    ))
}

/// Does the impl table hold a `Debug` for this type?
pub fn has_debug(reg: &TypeRegistry, id: TypeId) -> bool {
    let Some(debug) = reg.system_type("std::fmt::Debug") else {
        return false;
    };
    let probe = crate::registry::method::Probe::new(reg, reg.crate_root());
    let def = match reg.def(id) {
        Some(def) => def,
        None => return false,
    };
    let ty = Ty::Named {
        id,
        args: def.type_params.iter().map(|p| Ty::Param(p.clone())).collect(),
    };
    probe.implements(&ty, debug)
}

fn describe(ty: &Ty) -> String {
    match ty {
        Ty::Tuple(_) => "a tuple".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::Param(name) => format!("the type parameter `{}`", name),
        Ty::Dyn { .. } => "a trait object".to_string(),
        Ty::Assoc { name, .. } => format!("the projection `{}`", name),
        _ => "this type".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", src)])
    }

    #[test]
    fn a_string_prints_quoted() {
        let f = built("pub struct S { pub name: String }");
        let ty = f.field("lib.rs", "S", "name");
        assert_eq!(
            debug_expr(&f.reg, Some(&ty), "this.name").unwrap(),
            "JSON.stringify(this.name)"
        );
    }

    #[test]
    fn an_integer_prints_as_itself() {
        let f = built("pub struct S { pub n: u32 }");
        let ty = f.field("lib.rs", "S", "n");
        assert_eq!(debug_expr(&f.reg, Some(&ty), "this.n").unwrap(), "String(this.n)");
    }

    #[test]
    fn a_crate_type_prints_through_its_own_debug() {
        let f = built(
            "#[derive(Debug)] pub struct Inner { pub n: u32 }\n\
             pub struct S { pub inner: Inner }",
        );
        let ty = f.field("lib.rs", "S", "inner");
        assert_eq!(debug_expr(&f.reg, Some(&ty), "this.inner").unwrap(), "this.inner.debug()");
    }

    #[test]
    fn a_crate_type_without_the_derive_is_refused() {
        let f = built("pub struct Inner { pub n: u32 }\npub struct S { pub inner: Inner }");
        let ty = f.field("lib.rs", "S", "inner");
        let err = debug_expr(&f.reg, Some(&ty), "this.inner").unwrap_err();
        assert!(err.contains("no Debug"), "{}", err);
    }

    /// PREMISE CHANGED 2026-09-04: the subject used to be written twice — once
    /// for the null test and once inside `Some(..)` — so a `{:?}` on an
    /// expression with an effect performed it twice. It is read once now.
    #[test]
    fn an_option_prints_none_or_some_reading_its_subject_once() {
        let f = built("pub struct S { pub n: Option<u32> }");
        let ty = f.field("lib.rs", "S", "n");
        assert_eq!(
            debug_expr(&f.reg, Some(&ty), "this.n").unwrap(),
            "(($v) => $v === null ? 'None' : `Some(${String($v)})`)(this.n)"
        );
    }

    /// Rust keeps a float's decimal point, its `-0.0` sign and its `inf`
    /// spelling; JavaScript's `String` does none of those, so a
    /// `Value::F64(1.0)` printed `F64(1)`.
    #[test]
    fn a_float_prints_the_way_rust_prints_one() {
        let f = built("pub struct S { pub x: f64 }");
        let ty = f.field("lib.rs", "S", "x");
        let written = debug_expr(&f.reg, Some(&ty), "this.x").unwrap();
        assert!(written.contains("toFixed(1)"), "{}", written);
        assert!(written.contains("'-0.0'"), "{}", written);
        assert!(written.contains("'inf'"), "{}", written);
    }

    /// A `char` prints between single quotes, and the port writes one as a
    /// one-character string — so the quotes are the whole rendering.
    #[test]
    fn a_char_prints_between_quotes() {
        let f = built("pub struct S { pub c: char }");
        let ty = f.field("lib.rs", "S", "c");
        assert_eq!(debug_expr(&f.reg, Some(&ty), "this.c").unwrap(), "`'${this.c}'`");
    }

    /// An `Rc` and an `Arc` print their payload, and the port holds that
    /// payload in `.value`: `this.inner.debug()` on an `Arc` was a TypeError.
    #[test]
    fn an_arc_prints_through_the_value_it_holds() {
        let f = built(
            "use std::sync::Arc;\n\
             #[derive(Debug)] pub struct Inner { pub n: u32 }\n\
             pub struct S { pub held: Arc<Inner> }",
        );
        let ty = f.field("lib.rs", "S", "held");
        assert_eq!(
            debug_expr(&f.reg, Some(&ty), "this.held").unwrap(),
            "this.held.value.debug()"
        );
    }

    #[test]
    fn a_vec_prints_its_elements() {
        let f = built("pub struct S { pub xs: Vec<u32> }");
        let ty = f.field("lib.rs", "S", "xs");
        assert_eq!(
            debug_expr(&f.reg, Some(&ty), "this.xs").unwrap(),
            "`[${Array.from(this.xs).map((e) => String(e)).join(', ')}]`"
        );
    }

    #[test]
    fn a_box_prints_what_is_inside_it() {
        let f = built("pub struct S { pub n: Box<u32> }");
        let ty = f.field("lib.rs", "S", "n");
        assert_eq!(debug_expr(&f.reg, Some(&ty), "this.n").unwrap(), "String(this.n)");
    }

    #[test]
    fn an_untyped_value_is_refused_rather_than_guessed() {
        let f = built("pub struct S { pub n: u32 }");
        assert!(debug_expr(&f.reg, None, "x").is_err());
    }
}
