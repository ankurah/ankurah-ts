//! Reading a field out of a struct, where the read hands the field away.
//!
//! `let one = pair.one;` is a partial move in Rust: `one` is the caller's from
//! there, the rest of `pair` is still `pair`'s, and reading `pair.one` again is
//! a compile error. Emitting the plain property read left both of them owning
//! the same value, so the block released `one` and `pair`'s cascade released it
//! a second time.
//!
//! `AkObject.takeField(name)` is the runtime's answer: it hands the value over,
//! stops the cascade reaching it, and makes a later read fatal. This file says
//! which reads get it — the ones in a position that takes a value, of a field
//! that has drop glue.

use crate::ownership::Drops;

/// The property read `takeField` replaces, or nothing where the read borrows.
///
/// `base` is the already-emitted receiver text and `member` the emitted field
/// name; `drops` is what the field's own type owes.
pub fn take_field(base: &str, member: &str, drops: Drops) -> Option<String> {
    drops
        .is_droppable()
        .then(|| format!("{}.takeField('{}')", base, member))
}

/// Is this expression a place a value can be moved out of — a field of a local,
/// of `self`, or of another field?
///
/// A method call's result is a fresh value nobody else holds, so moving out of
/// it takes nothing from anybody; only a *place* has an owner left behind.
pub fn is_field_of_place(expr: &syn::Expr) -> bool {
    let syn::Expr::Field(field) = expr else {
        return false;
    };
    base_is_place(&field.base)
}

/// The place a projection starts from: `attested.payload.operations` is rooted
/// at `attested`, and who owns that decides whether the read hands anything
/// away.
pub fn root_of(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Field(field) => root_of(&field.base),
        syn::Expr::Index(index) => root_of(&index.expr),
        syn::Expr::Paren(p) => root_of(&p.expr),
        syn::Expr::Group(g) => root_of(&g.expr),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => root_of(&u.expr),
        syn::Expr::Reference(r) => root_of(&r.expr),
        other => other,
    }
}

fn base_is_place(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(_) => true,
        syn::Expr::Field(field) => base_is_place(&field.base),
        syn::Expr::Paren(p) => base_is_place(&p.expr),
        syn::Expr::Group(g) => base_is_place(&g.expr),
        syn::Expr::Unary(u) => {
            matches!(u.op, syn::UnOp::Deref(_)) && base_is_place(&u.expr)
        }
        _ => false,
    }
}

/// Does a field of this type hold only borrows?
///
/// A `&T` field owns nothing, and neither does a `Vec<&T>` or an `Option<&T>`:
/// the container is the port's own array or nullable, and everything inside it
/// belongs to somebody else. The cascade cannot tell them apart by walking
/// properties, so a type with one says so in `ownedFields()`. A crate type
/// instantiated with a reference is still its own object with its own `drop()`,
/// and stays owned.
pub fn borrows_only(reg: &crate::registry::TypeRegistry, ty: &crate::ty::Ty) -> bool {
    use crate::ty::Ty;
    match ty {
        Ty::Ref { .. } => true,
        Ty::Tuple(elems) => !elems.is_empty() && elems.iter().all(|e| borrows_only(reg, e)),
        Ty::Array { elem, .. } | Ty::Slice(elem) => borrows_only(reg, elem),
        // A declared std type the runtime writes as a plain value — an array
        // for `Vec`, a `Map` for `HashMap`, `T | null` for `Option` — owns
        // exactly what its arguments own. One with a `drop()` of its own does
        // not: dropping it is what releases it, references inside or not.
        Ty::Named { id, args } => {
            reg.is_system(*id)
                && reg.shapes().glue(*id).is_none()
                && reg.system_type("std::result::Result") != Some(*id)
                && !args.is_empty()
                && args.iter().all(|arg| borrows_only(reg, arg))
        }
        _ => false,
    }
}
