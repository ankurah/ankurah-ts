//! An OPAQUE iterator is a cursor, not the whole sequence.
//!
//! For: the port writes an iterator as the array it walks, which is right for
//! every chain it can see through — `xs.iter().filter(p).count()` has no cursor
//! in it at all once it is rewritten as array operations. It is wrong for the
//! one shape the array cannot express: a generic body that takes
//! `I: Iterator<Item = V>` and calls `next()` by hand. `Iterator::next` moves
//! ONE element out and leaves the rest in the iterator, and an array has
//! nowhere to record how far the walk has got — so `next` there was a hole
//! (R12), and `ankql`'s `Predicate::populate`, which pulls one value per
//! placeholder and then asks the iterator whether anything is left over, could
//! not run at all.
//!
//! What tells the two apart is a TYPE, not a method name: a bare type parameter
//! bounded by `IntoIterator` or `Iterator`, and the unnormalised projection
//! `<I as IntoIterator>::IntoIter` that `into_iter()` on one answers, are
//! opaque — nothing in the body says which iterator they are. Everything else
//! is a sequence the port can see through and keeps writing as an array.

use super::BodyTranslator;
use crate::ty::Ty;

impl BodyTranslator<'_> {
    /// Is this type an OPAQUE iterator — one the body cannot see through, so
    /// that the port holds it as a cursor?
    ///
    /// `want` is the trait the type has to be bounded by: `IntoIterator` for
    /// the value a walk is started from, `Iterator` for the walk itself.
    pub(crate) fn is_an_opaque_iterator(&self, ty: &Ty, want: &str) -> bool {
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        let ty = ty.peel_refs();
        let Some(wanted) = tc.registry.system_type(want) else { return false };
        // Two shapes and no others, because the port BUILDS a cursor in exactly
        // one place and the answer here has to be the same fact read back.
        //
        // The `<I as IntoIterator>::IntoIter` an `into_iter()` on a bounded
        // parameter answers, unnormalised — a projection that normalises is a
        // concrete iterator and the port writes it as the array it is. And a
        // bare type parameter bounded by `Iterator`, which is a PARAMETER whose
        // caller hands it one of those.
        //
        // Every OTHER projection carrying an `Iterator` bound is left alone,
        // and that is not a detail: `Iterable_dispatch_iterable(cdata)` in
        // `core/node.ts` has such a type and is an ARRAY, so asking the wider
        // question wrote `.takeRest()` on a value with no such method.
        match ty {
            Ty::Param(_) => {
                tc.probe().bounds_of(ty).into_iter().any(|bound| bound.id == wanted)
            }
            // Named by its ASSOCIATED NAME rather than by the trait it is
            // written through, because an unqualified `I::IntoIter` carries no
            // trait at all and is the same projection.
            Ty::Assoc { base, name, .. } => {
                name == "IntoIter"
                    && matches!(base.peel_refs(), Ty::Param(_))
                    && matches!(tc.probe().normalize(ty), Ty::Assoc { .. })
            }
            _ => false,
        }
    }

    /// `values.next()` on an opaque iterator: the cursor hands out the element
    /// it is pointing at and steps past it.
    pub(crate) fn cursor_next(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
    ) -> Option<String> {
        if rust_method != "next" || !call.args.is_empty() {
            return None;
        }
        let ty = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok()?;
        if !self.is_an_opaque_iterator(&ty, "std::iter::Iterator") {
            return None;
        }
        self.record_resolution(call, rust_method);
        Some(format!("{}.next()", receiver))
    }
}

/// The port's spelling of a parameter whose type is an OPAQUE iterator.
///
/// A body that takes `values: &mut I` with `I: Iterator<Item = V>` is handed a
/// cursor, because that is what the caller's `into_iter()` built — so the
/// parameter is declared as one. Written from the type rather than from the
/// bound's spelling: `I extends Iterable<V>` is what a sequence the port can
/// spread looks like, and a cursor is not spreadable and does not want to be.
///
/// Only `Iterator`. A parameter bounded by `IntoIterator` is the caller's own
/// sequence, which the port writes as `Iterable<V>` and spreads; it becomes a
/// cursor at the `into_iter()` inside the body, not at the boundary.
pub(crate) fn written_as_a_cursor(
    tc: &crate::infer::TypeContext<'_>,
    ty: &crate::ty::Ty,
    declared: &[String],
) -> Option<String> {
    let ty = ty.peel_refs();
    if !matches!(ty, Ty::Param(_)) {
        return None;
    }
    let iterator = tc.registry.system_type("std::iter::Iterator")?;
    let into = tc.registry.system_type("std::iter::IntoIterator");
    let probe = tc.probe();
    let bounds = probe.bounds_of(ty);
    if bounds.iter().any(|bound| Some(bound.id) == into) {
        return None;
    }
    let item = bounds.iter().find(|bound| bound.id == iterator).and_then(|bound| {
        bound.bindings.iter().find(|(name, _)| name == "Item").map(|(_, ty)| ty.clone())
    })?;
    // The item has to be nameable WHERE THIS SIGNATURE IS WRITTEN.
    // `FilterIterator::new<I>(iter: I)` on an `impl<I: Iterator<Item = R>, R>`
    // has its item named by the impl's `R`, which the emitted static's own
    // generic list does not declare: writing `SeqCursor<R>` there named
    // something nothing declares. Where the item cannot be named the parameter
    // keeps the spelling it had.
    if tc.params.iter().any(|p| item.mentions_param(p) && !declared.contains(p)) {
        return None;
    }
    Some(format!("SeqCursor<{}>", crate::name_map::map_ty(tc.registry, &item)))
}

impl BodyTranslator<'_> {
    /// A cursor asked for anything but `next` GIVES UP ITS REST.
    ///
    /// The cursor is what an opaque iterator IS in the port, and Rust reaches
    /// one with the whole of `Iterator` — `for v in it`, `it.collect()`,
    /// `it.count()`. Every one of those consumes the iterator and sees exactly
    /// the elements it has not yet handed out, which is what `takeRest()`
    /// answers: the tail, taken OUT of the cursor, so that dropping the cursor
    /// afterwards releases nothing twice. Without this, `for v in
    /// values.into_iter()` iterated a `SeqCursor`, which has no
    /// `Symbol.iterator`, and `values.into_iter().collect()` answered a cursor
    /// where an array was declared.
    /// The receiver a resolved method call is written against: itself, or a
    /// cursor's remaining elements where the method is not `next`.
    pub(crate) fn receiver_of(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        written: String,
    ) -> String {
        match rust_method {
            "next" => written,
            _ => self.cursor_gives_up_its_rest(&call.receiver, written),
        }
    }

    pub(crate) fn cursor_gives_up_its_rest(&self, expr: &syn::Expr, written: String) -> String {
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(expr)) else { return written };
        if !self.is_an_opaque_iterator(&ty, "std::iter::Iterator") {
            return written;
        }
        format!("{}.takeRest()", written)
    }
}
