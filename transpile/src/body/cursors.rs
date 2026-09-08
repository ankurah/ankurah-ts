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
use crate::registry::method::{ParamBounds, Probe};
use crate::registry::TypeRegistry;
use crate::ty::{TraitRef, Ty};

/// What a cursor walks, where a bound says.
///
/// A cursor whose `Item` no bound writes is still a cursor: what it hands out
/// is simply not known where the question was asked.
pub(crate) struct Cursor {
    pub item: Option<Ty>,
}

impl Cursor {
    /// Does the cursor OWN the elements it hands out, or point at somebody
    /// else's?
    ///
    /// The two ownership axes of a cursor are its ELEMENTS and its REST, and
    /// this is the first: `I: Iterator<Item = &Token>` walks tokens the caller
    /// still owns, so the cursor must release none of them. Read off the
    /// bound's `Item` because that is the only place the answer is written —
    /// the port's `SeqCursor<Token>` spelling drops the `&` (TypeScript has no
    /// reference), so by the time the type is written the axis is gone. Asked
    /// nowhere, a `countRefs(new SeqCursor([...tokens]))` released the caller's
    /// tokens and the caller's own `token.drop()` was a double drop.
    ///
    /// An `Item` no bound writes is treated as OWNED, which is what the port
    /// did before the axis existed: a cursor built from an `into_iter()` owns
    /// its walk, and that is the shape the corpus writes.
    pub(crate) fn elements(&self) -> crate::native_types::iterator::Elements {
        use crate::native_types::iterator::Elements;
        match &self.item {
            Some(Ty::Ref { .. }) => Elements::Borrowed,
            _ => Elements::Owned,
        }
    }
}

/// Does the port hold a value of this type as a `SeqCursor`, and what does it
/// walk?
///
/// ONE question, asked in one place, because four decisions have to agree: how
/// a signature spells the value, what a call site builds when it hands one
/// over, what the emitter writes on a receiver of that type, and what the scope
/// releases. Asked four ways they DID disagree — `ownership::glue` answered yes
/// for any unnormalised projection carrying an `Iterator` bound and wrote
/// `it.drop()` on a value nothing had wrapped, and the receiver rewriting
/// answered yes for any associated type NAMED `IntoIter`, so a
/// `trait Factory { type IntoIter: Measure; }` came out
/// `f.make().takeRest().amount()`.
///
/// Two shapes and no others, because the port BUILDS a cursor in exactly one
/// place — `into_iter()` on a bounded type parameter — and this is that fact
/// read back. Every OTHER projection carrying an `Iterator` bound is left
/// alone, and that is not a detail: `Iterable_dispatch_iterable(cdata)` in
/// `core/node.ts` has such a type and is an ARRAY, so asking the wider question
/// wrote `.takeRest()` on a value with no such method.
pub(crate) fn cursor_of(probe: &Probe<'_>, ty: &Ty) -> Option<Cursor> {
    let iterator = probe.reg.system_type("std::iter::Iterator")?;
    let into = probe.reg.system_type("std::iter::IntoIterator");
    let ty = ty.peel_refs();
    match ty {
        // A bare type parameter bounded by `Iterator` is a PARAMETER whose
        // caller hands it a walk.
        Ty::Param(_) => {
            let bounds = probe.bounds_of(ty);
            // A parameter bounded ONLY by `IntoIterator` is the caller's own
            // SEQUENCE: the port writes that as `Iterable<V>` and spreads it,
            // and it becomes a cursor at the `into_iter()` inside the body.
            //
            // GG4: only. `I: Iterator<Item = V> + IntoIterator` has an explicit
            // `Iterator` bound, which is what makes `next()` legal on it, and
            // rejecting the parameter for the `IntoIterator` beside that bound
            // wrote it as an array — `walk.next is not a function`.
            if bounds.iter().all(|bound| Some(bound.id) == into) {
                return None;
            }
            item_of(&bounds, iterator).map(|item| Cursor { item })
        }
        // The `<I as IntoIterator>::IntoIter` an `into_iter()` on a bounded
        // parameter answers, unnormalised — a projection that normalises is a
        // concrete iterator and the port writes it as the array it is.
        //
        // Named by its ASSOCIATED NAME as well as by the trait it is written
        // through, because an unqualified `I::IntoIter` carries no trait at all
        // and is the same projection. Where a trait IS written it has to be
        // `IntoIterator`, and the projection has to carry an `Iterator` bound:
        // an associated type that merely shares the name walks nothing.
        Ty::Assoc { base, name, trait_ } => {
            if name != "IntoIter" || !matches!(base.peel_refs(), Ty::Param(_)) {
                return None;
            }
            if !matches!(probe.normalize(ty), Ty::Assoc { .. }) {
                return None;
            }
            if let Some(written) = trait_ {
                if Some(written.id) != into {
                    return None;
                }
            }
            item_of(&probe.bounds_of(ty), iterator).map(|item| Cursor { item })
        }
        _ => None,
    }
}

/// The same question about a parameter of a signature that is not this body's,
/// where the bounds that answer it are the CALLEE's own and this body's say
/// nothing about the name — and, where the answer is yes, whether the callee
/// walks the caller's elements or its own.
pub(crate) fn declared_as_a_cursor(
    reg: &TypeRegistry,
    bounds: &ParamBounds,
    ty: &Ty,
) -> Option<crate::native_types::iterator::Elements> {
    let iterator = reg.system_type("std::iter::Iterator")?;
    let into = reg.system_type("std::iter::IntoIterator");
    let Ty::Param(param) = ty.peel_refs() else { return None };
    let mine: Vec<&TraitRef> =
        bounds.iter().filter(|(subject, _)| subject == param).map(|(_, t)| t).collect();
    // GG4: an EXPLICIT `Iterator` bound wins. `I: Iterator<Item = V> +
    // IntoIterator` is a parameter the body calls `next()` on, and rejecting it
    // for the `IntoIterator` beside the bound handed it over as a plain array —
    // `walk.next is not a function`, one frame down. Only `IntoIterator`
    // WITHOUT `Iterator` is the caller's transparent sequence.
    if !mine.iter().any(|bound| bound.id == iterator) {
        return None;
    }
    if mine.iter().all(|bound| Some(bound.id) == into) {
        return None;
    }
    let cloned: Vec<TraitRef> = mine.into_iter().cloned().collect();
    let item = item_of(&cloned, iterator)?;
    Some(Cursor { item }.elements())
}

/// Does a struct's FIELD hold a cursor, and what does that cursor own?
///
/// FF4: `Holder<I: Iterator<Item = Token>> { walk: I }` stores what a caller's
/// `into_iter()` built, and the answer is not in the field's type — `I` says
/// nothing — but in the DECLARATION's bound on `I`. Asked nowhere,
/// `Holder { walk: t.into_iter() }` emitted `new Holder([...tokens])` while
/// `pull()` read `this.walk.next()`, a method an array has not got.
pub(crate) fn field_walks_a_cursor(
    reg: &TypeRegistry,
    def: &crate::registry::TypeDef,
    field: &Ty,
) -> Option<crate::native_types::iterator::Elements> {
    declared_as_a_cursor(reg, &crate::registry::method::param_bounds_of(&def.bounds), field)
}

/// How the emitted CLASS spells a field the declaration walks as a cursor.
///
/// FF4's other half. The field is declared with the bare parameter — `readonly
/// walk: I` — which promises only what the bound promises, and a value the port
/// holds as a `SeqCursor` is not that. Both readers of the class say
/// `SeqCursor<V>`: the property and the constructor parameter, so that what the
/// construction site now hands over and what `this.walk.next()` asks for agree.
pub(crate) fn field_written_as_a_cursor(
    reg: &TypeRegistry,
    def: &crate::registry::TypeDef,
    field: &Ty,
) -> Option<String> {
    field_walks_a_cursor(reg, def, field)?;
    let iterator = reg.system_type("std::iter::Iterator")?;
    let Ty::Param(param) = field.peel_refs() else { return None };
    let mine: Vec<TraitRef> = def
        .bounds
        .iter()
        .filter(|b| matches!(&b.subject, Ty::Param(name) if name == param))
        .map(|b| b.trait_ref.clone())
        .collect();
    // The item has to be NAMED for the spelling to say anything; every
    // parameter a struct's own bounds mention is one the struct declares, so
    // there is no out-of-scope name to guard against here.
    let item = item_of(&mine, iterator)??;
    Some(format!("SeqCursor<{}>", crate::name_map::map_ty(reg, item.peel_refs())))
}

/// How the emitted class writes ONE field: as a cursor where the declaration
/// walks it as one, and otherwise as the field's own type.
///
/// FF4: the property and the constructor parameter both read this, so that what
/// a construction site hands over and what `this.walk.next()` asks for agree.
pub(crate) fn field_spelling(
    reg: &TypeRegistry,
    owner: Option<crate::ty::TypeId>,
    field: &crate::types::FieldInfo,
) -> String {
    let spelled = owner
        .and_then(|id| reg.def(id))
        .zip(field.ty.as_ref())
        .and_then(|(def, ty)| field_written_as_a_cursor(reg, def, ty));
    spelled.unwrap_or_else(|| field.ts_ty(reg))
}

/// Which of a signature's parameters the port writes as a `SeqCursor`, and what
/// each of those owns.
pub(crate) fn cursor_parameters(
    reg: &TypeRegistry,
    sig: &crate::registry::MethodSig,
) -> Vec<Option<crate::native_types::iterator::Elements>> {
    let bounds = crate::registry::method::param_bounds_of(&sig.bounds);
    sig.params.iter().map(|(_, ty)| declared_as_a_cursor(reg, &bounds, ty)).collect()
}

/// The `Item` an `Iterator` bound binds, where one of these bounds is
/// `Iterator` at all.
fn item_of(bounds: &[TraitRef], iterator: crate::ty::TypeId) -> Option<Option<Ty>> {
    let bound = bounds.iter().find(|bound| bound.id == iterator)?;
    Some(bound.bindings.iter().find(|(name, _)| name == "Item").map(|(_, ty)| ty.clone()))
}

impl BodyTranslator<'_> {
    /// Is this type an OPAQUE iterator — one the body cannot see through, so
    /// that the port holds it as a cursor?
    pub(crate) fn is_an_opaque_iterator(&self, ty: &Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        cursor_of(&tc.probe(), ty).is_some()
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
        if !self.is_an_opaque_iterator(&ty) {
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
    // The SPELLING is layered on the representation: this says how to write a
    // value the port already holds as a cursor, never whether it holds one.
    let item = cursor_of(&tc.probe(), ty)?.item?;
    // The `&` of an `Item = &Token` is dropped here and NOT forgotten: the
    // ownership it carries is the constructed cursor's `'borrow'` mode.
    let item = item.peel_refs().clone();
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
            // `next` is asked of the cursor itself, and `by_ref` IS the cursor
            // (`cursor_by_ref`). Neither gives up a rest.
            "next" | "by_ref" => written,
            _ if crate::native_types::iterator::cursor_answers_in_place(
                rust_method,
                call.args.len(),
            )
            .is_some() =>
            {
                written
            }
            _ => {
                self.cursor_gives_up_its_rest(&call.receiver, rust_method, call.args.len(), written)
            }
        }
    }

    /// `walk.by_ref()` on a cursor: the borrowed VIEW of a cursor is the cursor.
    ///
    /// `Iterator::by_ref(&mut self) -> &mut Self` hands back a reference, and
    /// the port has none — so what the chain above it is written against is the
    /// cursor place itself. Written as a consumption, `it.by_ref()` was
    /// `it.takeRest()`, which takes the whole rest out and marks the cursor
    /// moved, and the caller's own `it.drop()` was then a use after move; a
    /// `for` loop over it asked the same question a second time and wrote
    /// `it.takeRest().takeRest()`, a method an array has not got.
    /// A cursor asked for one of the `&mut self` methods that STOP as soon as
    /// they can: the cursor answers it, and keeps what it did not visit.
    ///
    /// FF1: `walk.any(p)` in Rust pulls elements out of `walk` one at a time,
    /// hands each to the predicate BY VALUE, and stops at the first `true` —
    /// leaving everything after it in `walk`, for `walk`'s owner to drop.
    /// Written through the rest, `walk.drainRest().some(p)` pulled the whole
    /// tail out of the cursor and gave it to an array method that stops early,
    /// so every element after the match was released by nobody, and
    /// `walk.takeRest()` before it marked the cursor moved under the owner's
    /// own `walk.drop()`.
    ///
    /// The port's `SeqCursor` carries one method per name here, walking exactly
    /// as far as Rust does and releasing exactly what Rust drops (the elements
    /// `find` rejected; nothing, where the predicate was handed the element by
    /// value and is its owner).
    pub(crate) fn cursor_walks_in_place(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
        args: &[String],
    ) -> Option<String> {
        let ts = crate::native_types::iterator::cursor_answers_in_place(rust_method, args.len())?;
        let ty = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok()?;
        if !self.is_an_opaque_iterator(&ty) {
            return None;
        }
        self.record_resolution(call, rust_method);
        Some(format!("{}.{}({})", receiver, ts, args.join(", ")))
    }

    pub(crate) fn cursor_by_ref(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
    ) -> Option<String> {
        if rust_method != "by_ref" || !call.args.is_empty() {
            return None;
        }
        let ty = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok()?;
        if !self.is_an_opaque_iterator(&ty) {
            return None;
        }
        self.record_resolution(call, rust_method);
        Some(receiver.to_string())
    }

    /// A cursor asked for anything but `next` gives up its rest — and which of
    /// the two ways depends on whether the caller keeps it.
    ///
    /// By VALUE the cursor is consumed, which is what every consuming
    /// `Iterator` method does, so `takeRest()` marks it moved. Through a
    /// REFERENCE it is not: `&mut I` is an `Iterator` by the blanket impl, so
    /// `drain(&mut it)` whose body writes `values.collect()` drains what the
    /// reference points at and leaves the iterator alive for its owner.
    /// Consumed there, the owner's own `it.drop()` was a use after move —
    /// fatal, on a program Rust runs — so a reborrow uses `drainRest()`, which
    /// empties the cursor without taking it.
    pub(crate) fn cursor_gives_up_its_rest(
        &self,
        expr: &syn::Expr,
        method: &str,
        arity: usize,
        written: String,
    ) -> String {
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(expr)) else { return written };
        if !self.is_an_opaque_iterator(&ty) {
            return written;
        }
        // FF1: the METHOD says whether the cursor is consumed, and the receiver
        // expression only says whether the caller holds a reborrow. Read off
        // the expression alone, `walk.any(p)` on an owned cursor was
        // `walk.takeRest().some(p)` — the cursor marked moved with the caller's
        // own `walk.drop()` still below it, which is a use after move on a
        // program Rust runs.
        let by_reference = matches!(ty, Ty::Ref { .. })
            || crate::native_types::iterator::takes_self_by_reference(method, arity);
        match by_reference {
            true => format!("{}.drainRest()", written),
            false => format!("{}.takeRest()", written),
        }
    }

    /// The type the native tables are asked about for a method's receiver.
    ///
    /// A cursor that GAVE UP ITS REST is an array from that point on, and the
    /// array's own table is what knows how to write a call on it — including
    /// which of its helpers releases the elements an adaptor drops. Dispatched
    /// on the unmodified type parameter instead, `walk.takeRest().count()`,
    /// `.take(1)` and `.skip(1)` named methods no array has.
    pub(crate) fn receiver_shape_of(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        resolved: &Ty,
    ) -> Ty {
        if rust_method != "next" {
            if let Some(item) = self.rest_of_a_cursor(&call.receiver) {
                return Ty::Slice(Box::new(item));
            }
        }
        match &self.types {
            Some(tc) => crate::body::calls::receiver_shape(&tc.borrow(), resolved),
            None => resolved.clone(),
        }
    }

    /// What the rest of a cursor is a sequence OF, where this expression names
    /// a cursor and its bound says what it walks.
    fn rest_of_a_cursor(&self, expr: &syn::Expr) -> Option<Ty> {
        let ty = self.quietly(|| self.resolve_expr_type(expr)).ok()?;
        let tc = self.types.as_ref()?.borrow();
        cursor_of(&tc.probe(), &ty)?.item
    }

    /// Which of a free call's parameters the callee walks as a cursor, so that
    /// a concrete sequence handed to one is wrapped where it crosses the
    /// boundary.
    ///
    /// Read off the callee's OWN bounds. The substituted parameter types the
    /// expectation machinery hands out drop anything still open, and a cursor
    /// parameter is exactly an open one, so the boundary was invisible there.
    pub(crate) fn cursor_parameters_of_call(
        &self,
        call: &syn::ExprCall,
        expected: Option<&Ty>,
    ) -> Vec<Option<crate::native_types::iterator::Elements>> {
        let Some(tc) = &self.types else { return Vec::new() };
        // Asking is not translating: the resolution files whatever it could not
        // settle, and the call is resolved again when it is written.
        let Some((sig, _)) = self.quietly(|| tc.borrow().call_sig(call, expected)) else {
            return Vec::new();
        };
        cursor_parameters(tc.borrow().registry, &sig)
    }

    /// Which of a method call's parameters the callee walks as a cursor.
    ///
    /// Asking is not translating: the resolution files whatever it could not
    /// settle, and the call is resolved again when it is written, so the record
    /// is wound back and the same gap is not counted twice.
    pub(crate) fn cursor_arguments(
        &self,
        call: &syn::ExprMethodCall,
    ) -> Vec<Option<crate::native_types::iterator::Elements>> {
        let Some(tc) = &self.types else { return Vec::new() };
        let method = call.method.to_string();
        let found = self.quietly(|| {
            tc.borrow().resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref())
        });
        let Ok(found) = found else { return Vec::new() };
        let tc = tc.borrow();
        match tc.registry.method_sig(&found) {
            Some(sig) => cursor_parameters(tc.registry, &sig),
            None => Vec::new(),
        }
    }

    /// Which of a struct literal's fields the DECLARATION walks as a cursor.
    pub(crate) fn cursor_fields(
        &self,
        lit: &syn::ExprStruct,
    ) -> Vec<(String, crate::native_types::iterator::Elements)> {
        let Some(tc) = &self.types else { return Vec::new() };
        let tc = tc.borrow();
        // Found by the literal's PATH, because most literals write no type
        // arguments — `Holder { walk: t.into_iter() }` writes none — and what
        // the declaration requires of its parameters does not depend on one.
        let Some(id) = tc.struct_literal_declaration(lit) else { return Vec::new() };
        let Some(def) = tc.registry.def(id) else { return Vec::new() };
        def.fields
            .iter()
            .filter_map(|(name, ty)| {
                field_walks_a_cursor(tc.registry, def, ty).map(|e| (name.clone(), e))
            })
            .collect()
    }

    /// A value handed to a parameter the callee walks as a CURSOR.
    ///
    /// `first(tokens.into_iter())` from a body that knows what `tokens` is
    /// writes `[...tokens]` — an array — while `first`'s own parameter is
    /// written `SeqCursor<Token>`, so `walk.next()` inside it called a method
    /// an array has not got. The sequence is wrapped where it crosses the
    /// boundary; a value that is ALREADY a cursor goes over as it stands, and a
    /// value the engine cannot type is reported rather than guessed at, because
    /// wrapping a cursor a second time and handing an array over unwrapped are
    /// both `TypeError`s one frame down.
    pub(crate) fn adapted_to_a_cursor(
        &self,
        arg: &syn::Expr,
        wanted: Option<crate::native_types::iterator::Elements>,
        written: String,
    ) -> String {
        let Some(elements) = wanted else { return written };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(arg)) else {
            self.fallback(
                syn::spanned::Spanned::span(arg),
                "this argument stands at a parameter the callee walks one element at a time, \
                 and the engine could not type it, so whether it is already a walk or a \
                 sequence to be wrapped in one is not known here; it is handed on as written"
                    .to_string(),
            );
            return written;
        };
        let already = {
            let Some(tc) = &self.types else { return written };
            let tc = tc.borrow();
            cursor_of(&tc.probe(), &ty).is_some()
        };
        // A cursor OWNS the array it walks, so an argument the port already
        // wrote as a fresh array — `tokens.into_iter()` is `[...tokens]` — is
        // handed straight to it. Spread a second time it was
        // `new SeqCursor([...[...tokens]])`, which copies the same elements
        // twice on one line.
        // GG1: a cursor over BORROWED items points at the caller's elements and
        // releases none of them, which is the mode the constructor is told.
        let mode = match elements {
            crate::native_types::iterator::Elements::Owned => "",
            crate::native_types::iterator::Elements::Borrowed => ", 'borrow'",
        };
        match (already, self.builds_its_own_sequence(arg)) {
            (true, _) => written,
            (false, true) => format!("new SeqCursor({}{})", written, mode),
            (false, false) => format!("new SeqCursor([...{}]{})", written, mode),
        }
    }
}
