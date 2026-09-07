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
use crate::body::BodyTranslator;
use crate::ownership;

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

/// The identifier a place is rooted at: `attested.payload.operations` answers
/// `attested`, and an expression rooted at anything but a name answers nothing.
pub fn root_name(expr: &syn::Expr) -> Option<String> {
    match root_of(expr) {
        syn::Expr::Path(path) if path.path.is_ident("self") => Some("this".to_string()),
        syn::Expr::Path(path) => path
            .path
            .get_ident()
            .map(|ident| crate::name_map::to_camel_case(&ident.to_string())),
        _ => None,
    }
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

impl<'a> BodyTranslator<'a> {
    /// Can this scope hand away what lives at this place?
    ///
    /// Only the owner can. A `&self` method lends its receiver, a local bound
    /// to a `&T` lends what it points at, and Rust refuses a move out of
    /// either — so a read there is a borrow, whatever the position looks like.
    /// Does the port hold this expression's value in a temporary with a
    /// release of its own?
    ///
    /// That is what `hoist_temporary` writes for any value a statement builds
    /// and does not name, and it is exactly the condition under which the
    /// emitted base of a field read is a NAME whose cascade can reach the
    /// field. A base that is already a place is somebody's; this asks about the
    /// ones that are not.
    fn holds_a_temporary(&self, base: &syn::Expr) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(base)) else { return false };
        ownership::drops_of(&tc.borrow().probe(), &ty) == ownership::Drops::Own
    }

    pub(crate) fn owns_place(&self, expr: &syn::Expr) -> bool {
        match ownership::places::root_of(expr) {
            syn::Expr::Path(path) if path.path.is_ident("self") => self.owns_self,
            root @ syn::Expr::Path(_) => !matches!(
                self.quietly(|| self.resolve_expr_type(root)),
                Ok(crate::ty::Ty::Ref { .. })
            ),
            // A value the expression built is nobody else's, so taking it apart
            // takes nothing from anybody.
            _ => true,
        }
    }

    /// An expression in a position that takes its value, rather than reading
    /// through it: an argument, a struct field, a tuple element, a `break`.
    ///
    /// The one thing that changes here is a field read. `take(pair.one)` hands
    /// `one` to the callee and leaves the rest of `pair` where it was, so the
    /// field has to come *out* of the struct — otherwise the callee releases it
    /// and `pair`'s own cascade releases it a second time.
    pub fn moved_value(&self, expr: &syn::Expr) -> String {
        self.partial_move(expr)
            .unwrap_or_else(|| self.expr_value(expr))
    }

    /// A method call's RECEIVER, as the value the callee is handed.
    ///
    /// A method declared `self` takes its receiver with it, and where the
    /// receiver is a FIELD of a place that is a partial move: Rust takes the
    /// field out of the struct, leaves the rest where it was, and drops what is
    /// left field by field. Written as a plain property read the struct and the
    /// callee both owned it — `selection.predicate.populate(..)` handed the
    /// predicate away and the emitted `selection.drop()` then raised
    /// `BUG: Predicate was used after being moved`, which is six of ankql's
    /// seven `ast.test.ts` failures. `takeField` is the same call a
    /// `let x = s.field` already writes; only this position was not asking for
    /// it.
    pub(crate) fn receiver_value(&self, call: &syn::ExprMethodCall) -> String {
        // A range that is the receiver of `contains` is never materialised: the
        // comparison is written from its BOUNDS, and a range of a width the
        // port cannot count still answers it. Translating the receiver here
        // would file the sequence's refusal for a sequence nothing builds.
        if self.contains_on_a_range(call) {
            return String::new();
        }
        match ownership::moves::Consumes::consumes_receiver(self, call) {
            true => self.moved_value(&call.receiver),
            false => self.expr_value(&call.receiver),
        }
    }

    /// `s.field` in a value position, as `s.takeField('field')` — or nothing
    /// where the read is not a move.
    pub(crate) fn partial_move(&self, expr: &syn::Expr) -> Option<String> {
        let syn::Expr::Field(field) = expr else {
            return None;
        };
        // Rust leaves the rest of a PLACE behind for whoever owns it, and
        // leaves nothing behind for a temporary: `h.clone().inner` moves the
        // field out of a value that dies on the same line, and Rust's drop glue
        // for that temporary knows the field is gone.
        //
        // Y3: the port's does not. It holds the temporary in a `_tN` with a
        // `finally` of its own, and that release cascades into the field the
        // new owner has already taken — `selection.clone().predicate.populate(..)`
        // was ankql's last failing test, reported as `Predicate was used after
        // being moved`. So a field read out of a temporary the port HOLDS is a
        // partial move too, and the temporary is the place it comes out of.
        if !ownership::places::is_field_of_place(expr) && !self.holds_a_temporary(&field.base) {
            return None;
        }
        if !self.owns_place(expr) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(expr)).ok()?;
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        // The member is read from the SOURCE, and the receiver is lowered only
        // once this answers yes. Lowering it to ask the question wrote the base
        // twice wherever the answer turned out to be no and the caller lowered
        // it again for the plain read: `(await getHolder()).items` awaited the
        // holder twice, which the `await_postfix` golden caught.
        let member = match &field.member {
            syn::Member::Named(ident) => crate::name_map::to_camel_case(&ident.to_string()),
            syn::Member::Unnamed(index) => format!("_{}", index.index),
        };
        // A field the port cannot take OUT is one Rust moved and the emitted
        // text still reads in place, so the struct's cascade reaches it as well
        // as the new owner. `Drops::Nothing` is not that — a `Copy` field, a
        // number, a string: there is nothing for a cascade to release twice.
        // `Cascade` and `Unknown` are, the first because the runtime writes the
        // field as a plain value with no `takeField` of its own and the second
        // because the engine cannot say, while the runtime's cascade looks at
        // what is actually there. Whoever was about to claim the struct has to
        // know.
        if matches!(drops, ownership::Drops::Cascade | ownership::Drops::Unknown) {
            if let Some(root) = ownership::places::root_name(expr) {
                self.own.partial_moves_written_as_reads.borrow_mut().push(root);
            }
        }
        if !drops.is_droppable() {
            return None;
        }
        if drops == ownership::Drops::Cascade {
            // `takeField` is `AkObject`'s, and a field the runtime writes as a
            // plain array or `Map` is not one: the read hands the same object to
            // two owners and the emitter has no way to say so.
            self.fallback(
                syn::spanned::Spanned::span(expr),
                format!(
                    "`{}` moves a field the runtime writes as a plain value; both the struct \
                     and the new owner release it",
                    member
                ),
            );
            return None;
        }
        let (receiver, member) = self.field_parts(field);
        ownership::places::take_field(&receiver, &member, drops)
    }
}
