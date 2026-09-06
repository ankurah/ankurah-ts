//! What a call, a match or an operator takes away from the block that held it.
//!
//! For: the move scan reads the SYNTAX — it walks a block looking for the
//! places a value is handed on — and the syntax of `r.unwrap()`, `match e`,
//! `a + b`, `-a` and `f()` says nothing about whether the callee consumes what
//! it was given. The type engine and the impl table say. This is where the two
//! meet: one answer per shape, each asked of the engine and none of them
//! guessed from a name.

use crate::ownership;

use super::BodyTranslator;

/// Does this method call take its receiver by value?
///
/// The move analysis asks; the impl table answers. `Result::unwrap`,
/// `Option::take` and the `into_*` family all take `self`, and a receiver they
/// took is not the block's to release any more.
impl ownership::moves::Consumes for BodyTranslator<'_> {
    /// J4: the whole-call refusals are the `map.entry(..)` finisher family and
    /// nothing else today — a receiver the engine could not type says nothing
    /// `or_insert` can be written from, and a value type with no default says
    /// nothing `or_default` can. The question is the LOWERING's, asked exactly
    /// as `finishes_an_entry` asks it (I1's rule), so the scan and the emitter
    /// cannot disagree about which calls are written.
    fn refuses_call(&self, call: &syn::ExprMethodCall) -> bool {
        let mark = self.types.as_ref().map(|tc| tc.borrow().sink.mark());
        let answer = self.finishes_an_entry(&syn::Expr::MethodCall(call.clone()));
        // Asking is not translating: the resolution files what it deferred, and
        // the translation of the call reports it once.
        if let (Some(tc), Some(mark)) = (&self.types, mark) {
            tc.borrow().sink.rewind(mark);
        }
        answer == crate::body::places::EntryFinish::Hole || self.refuses_named_iterator_terminal(call)
    }

    fn consumes_receiver(&self, call: &syn::ExprMethodCall) -> bool {
        // F1: a consuming iterator terminal takes the sequence's ELEMENTS, and
        // the block that produced them owes them nothing afterwards. Rust's own
        // signature says `find(&mut self)`, which reads as "the receiver is
        // borrowed" — true of the ITERATOR, and false of the items it walks,
        // which is what the port's array is. Asked in one place so the move
        // scan and the lowering cannot disagree about who releases what.
        if self.terminal_owns_the_sequence(call) {
            return true;
        }
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating. The resolution files the questions it
        // deferred, and this asks the same call several times — once per
        // statement scan, once per flag scan — so the record is wound back to
        // where it stood. The translation of the call reports them once.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(
            &call.receiver,
            &call.method.to_string(),
            call.turbofish.as_ref(),
        );
        tc.sink.rewind(mark);
        let Ok(found) = found else {
            // `.await` on a named future and `Result::unwrap` on a receiver the
            // engine could not type are the two that matter, and both are worth
            // reading off the name rather than losing: taking a receiver that
            // was not taken leaks, and leaving one that was taken double-drops.
            return matches!(
                call.method.to_string().as_str(),
                "unwrap" | "expect" | "unwrap_err" | "expect_err" | "unwrap_or" | "unwrap_or_else"
                    | "unwrap_or_default" | "into" | "into_inner" | "into_iter" | "take"
                    | "ok" | "err" | "map_err" | "and_then" | "or_else"
            ) || call.method.to_string().starts_with("into_");
        };
        matches!(
            tc.registry.method_self_kind(&found),
            Some(crate::types::SelfKind::Value)
        )
    }

    fn consumes_scrutinee(&self, m: &syn::ExprMatch) -> bool {
        // A `Result` match reads its payload with `unwrap()` or `unwrapErr()`,
        // each of which takes the wrapper apart and hands back what it held —
        // so the match MOVES its subject whatever the arms go on to bind, and
        // the block that owned it owes no release afterwards. Asked only
        // through `match_takes`, which reads the arms' patterns, this answered
        // "nothing taken" and `storage-sqlite/engine.ts:493` wrote
        // `finally { result.drop() }` under both reads: a use after move on
        // every path. Only a BORROWED `Result` is read through, and that one is
        // `okRef()`/`errRef()`.
        //
        // Asking is not translating: the scan asks this of every match several
        // times, and the subject's own gaps are reported where the match is
        // written out.
        if crate::match_expr::is_result_match(&m.arms)
            && !matches!(
                self.quietly(|| self.borrowed_scrutinee_type(&m.expr)),
                Some(crate::ty::Ty::Ref { .. })
            )
        {
            return true;
        }
        self.match_takes(m) == ownership::scrutinee::Takes::Payload
    }

    fn consumes_let_scrutinee(&self, let_expr: &syn::ExprLet) -> bool {
        self.let_takes(let_expr) == ownership::scrutinee::Takes::Payload
    }

    fn consumes_operands(&self, bin: &syn::ExprBinary) -> (bool, bool) {
        self.operator_takes(bin)
    }

    fn consumes_unary_operand(&self, unary: &syn::ExprUnary) -> bool {
        self.unary_takes(unary)
    }

    fn consumes_callee(&self, call: &syn::ExprCall) -> bool {
        // A callee whose bound is `FnOnce` and nothing else is consumed by the
        // call itself — `invoke` is the helper that says so. The move scan was
        // never told, so the parameter counted as untouched, and the body was
        // given no release at all in case the call had taken it: a path that
        // did not call left the closure and everything it captured to nobody.
        if self.bound_closure_helper(&call.func) == Some("invoke") {
            return true;
        }
        let syn::Expr::Path(path) = call.func.as_ref() else { return false };
        let Some(ident) = path.path.get_ident() else { return false };
        let name = crate::name_map::escape_reserved(&crate::name_map::to_camel_case(&ident.to_string()));
        self.own.once_closure_locals.borrow().iter().any(|n| *n == name)
    }
}

impl BodyTranslator<'_> {
    /// Does this call's lowering TAKE the elements of the sequence it walks?
    ///
    /// F1: Rust's consuming terminals own what they walk — `into_iter().find(p)`
    /// hands back the element it selected and drops every other one — and the
    /// port writes the iterator as an array, so the array IS the items. Written
    /// with the reading helper, such a chain released the element it had just
    /// handed back, or leaked every element it had not.
    ///
    /// Three things have to hold, and none of them is the method's NAME:
    ///
    ///   - the name and arity are one of the terminals whose owned spelling
    ///     exists (`iterator::is_owned_terminal`);
    ///   - the resolution came through `Iterator` — `slice::last(&self)` and
    ///     `Iterator::last(self)` are two methods of one name, and only the
    ///     second consumes;
    ///   - the sequence the expression produced owns its elements: a chain
    ///     built with `iter()` holds borrows and owes nothing, and so does one
    ///     over elements with no drop glue.
    ///
    /// A receiver that is a PLACE is left to the block that declared it: a
    /// named iterator (`let it = v.into_iter(); it.find(..)`) is partly
    /// consumed by the call and the port has no way to say which of the array's
    /// elements are left, so that shape is refused where it is translated
    /// rather than answered here.
    pub(crate) fn terminal_owns_the_sequence(&self, call: &syn::ExprMethodCall) -> bool {
        self.consuming_terminal(call) && !names_an_iterator_place(&call.receiver)
    }

    /// The shape that is neither: a consuming terminal called on a NAMED
    /// iterator over droppable elements.
    ///
    /// `let mut it = tokens.into_iter(); it.find(..)` consumes the elements the
    /// walk passed and leaves the rest in `it`, which Rust drops when `it` goes
    /// out of scope. The port writes `it` as the whole array, so after the call
    /// the array still holds every element — the consumed ones, and the one the
    /// call handed back. Neither release is writable: the block's `dropOwned(it)`
    /// released the element the caller had just been given, and taking the
    /// release away would leak the ones the walk never reached. So the call is
    /// refused (R12) rather than answered wrongly, and the block keeps the
    /// receiver, which is what a hole leaves it holding (J4).
    pub(crate) fn refuses_named_iterator_terminal(&self, call: &syn::ExprMethodCall) -> bool {
        self.consuming_terminal(call) && names_an_iterator_place(&call.receiver)
    }

    /// The same question without the place clause: is this a terminal that
    /// consumes droppable elements at all? The refusal reads it too.
    pub(crate) fn consuming_terminal(&self, call: &syn::ExprMethodCall) -> bool {
        self.walks_droppable_elements(call, crate::native_types::iterator::is_owned_terminal)
    }

    /// The same question for an eager ADAPTOR that discards elements.
    ///
    /// O3/O4: `filter`, `skip`, `take` and `step_by` throw elements away, and
    /// Rust drops what they throw away. Written as array operations they simply
    /// lost them, and the consuming terminal below could not release what the
    /// adaptor had already erased.
    ///
    /// T3: the place clause reaches here too. "An adaptor takes the iterator by
    /// value whatever it was called on" is false above a `by_ref`, which the
    /// terminal rule already treats as naming its receiver:
    /// `it.by_ref().filter(p).find(q)` emitted `iterFind(filterOwned(it, p), q)`
    /// and then released `it` on top of it — the rejected elements dropped
    /// twice, and the one the caller received dropped while the caller held it.
    /// Refused for the same reason `by_ref` above `find` is refused: the port
    /// writes an iterator as the whole array, so after the call it cannot say
    /// which of its elements are still the caller's.
    pub(crate) fn adaptor_owns_its_elements(&self, call: &syn::ExprMethodCall) -> bool {
        self.walks_droppable_elements(call, crate::native_types::iterator::is_owned_adaptor)
            && !names_an_iterator_place(&call.receiver)
    }

    /// The same, asked the other way round: is this an owning adaptor the port
    /// has to REFUSE because its receiver names an iterator the caller keeps?
    pub(crate) fn refuses_named_iterator_adaptor(&self, call: &syn::ExprMethodCall) -> bool {
        // `next` is in the same list, because its lowering owns the tail it
        // hands past — but it has a refusal of its own that says exactly what
        // is wrong with it (there is no cursor to advance), and that is the one
        // the hole ledger records. Saying the adaptor's instead told a reader
        // less.
        call.method != "next"
            && self.walks_droppable_elements(call, crate::native_types::iterator::is_owned_adaptor)
            && names_an_iterator_place(&call.receiver)
    }

    /// The three questions both of those ask, in the order they are cheapest.
    fn walks_droppable_elements(
        &self,
        call: &syn::ExprMethodCall,
        is_named: fn(&str, usize) -> bool,
    ) -> bool {
        let method = call.method.to_string();
        if !is_named(&method, call.args.len()) {
            return false;
        }
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating: this is asked once per statement scan and
        // again where the call is written, and the record is wound back so the
        // deferred questions are filed once, by the translation.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref());
        let receiver_ty = found.as_ref().ok().map(|f| f.receiver_type().clone());
        tc.sink.rewind(mark);
        let Ok(found) = found else { return false };
        let Some(trait_id) = tc.registry.method_trait(&found) else { return false };
        let is_iterator = ["std::iter::Iterator", "std::iter::DoubleEndedIterator"]
            .iter()
            .any(|path| tc.registry.system_type(path) == Some(trait_id));
        if !is_iterator {
            return false;
        }
        let Some(ty) = receiver_ty else { return false };
        crate::ownership::drops_of(&tc.probe(), &ty).is_droppable()
    }
}

impl BodyTranslator<'_> {
    /// Whether a call's lowering owns the elements of the sequence it walks —
    /// the same question `terminal_owns_the_sequence` answers, in the shape the
    /// lowering takes it.
    pub(crate) fn element_ownership(
        &self,
        call: &syn::ExprMethodCall,
    ) -> crate::native_types::iterator::Elements {
        use crate::native_types::iterator::Elements;
        let owned = self.terminal_owns_the_sequence(call) || self.adaptor_owns_its_elements(call);
        // T5: "not droppable" and "the engine cannot say" are two different
        // answers, and only one of them is a reason to write the reading
        // helper. `views.into_iter().next()` on an `R: View + Clone` takes
        // `iterFirst` because `R`'s drop glue is unknown, and the tail is then
        // released by nobody — safe today only because no `View` in the corpus
        // has any. Said out loud rather than defaulted.
        if !owned && self.elements_the_engine_cannot_type(call) {
            self.fallback(
                syn::spanned::Spanned::span(call),
                format!(
                    "`{}` walks elements whose drop glue the engine could not name, so it is \
                     written as the reading helper and the elements it does not hand back are \
                     released by nobody",
                    call.method
                ),
            );
        }
        match owned {
            true => Elements::Owned,
            false => Elements::Borrowed,
        }
    }

    /// Was the ownership answer above decided by `Drops::Unknown` — a bare type
    /// parameter or an unnormalised projection — rather than by a type the
    /// engine can see owns nothing?
    fn elements_the_engine_cannot_type(&self, call: &syn::ExprMethodCall) -> bool {
        let method = call.method.to_string();
        let named = crate::native_types::iterator::is_owned_terminal(&method, call.args.len())
            || crate::native_types::iterator::is_owned_adaptor(&method, call.args.len());
        if !named {
            return false;
        }
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref());
        let receiver_ty = found.as_ref().ok().map(|f| f.receiver_type().clone());
        tc.sink.rewind(mark);
        let Some(ty) = receiver_ty else { return false };
        matches!(crate::ownership::drops_of(&tc.probe(), &ty), crate::ownership::Drops::Unknown)
    }

    /// Did this terminal take its CALLBACK by value, so that it is what
    /// releases it?
    ///
    /// O2: Rust's terminals take their `F` by value and drop it where the call
    /// ends. `find(&mut p)` type-checks through the `impl FnMut for &mut F`
    /// blanket, so what the terminal takes by value is the REFERENCE: dropping
    /// it does nothing and `p` is still the caller's to call again. The port
    /// has no reference to hand over — the closure object itself goes — so the
    /// helper is told which of the two this is. Released regardless, the next
    /// call read captures that were gone and the caller's own release dropped
    /// it a second time.
    ///
    /// The argument's WRITTEN form answers first, because `&mut p` is the shape
    /// Rust writes for this and nothing about the parameter's declared bound
    /// says it; a NAME whose own type resolves to a reference is the other
    /// spelling and is asked of the type.
    pub(crate) fn callback_ownership(
        &self,
        call: &syn::ExprMethodCall,
    ) -> crate::native_types::iterator::Callback {
        use crate::native_types::iterator::Callback;
        let Some(arg) = call.args.first() else { return Callback::Owned };
        if matches!(arg, syn::Expr::Reference(_)) {
            return Callback::Borrowed;
        }
        let Some(tc) = &self.types else { return Callback::Owned };
        let tc = tc.borrow();
        // Asking is not translating: what this resolution defers is reported
        // once, by the translation of the argument itself.
        let mark = tc.sink.mark();
        let ty = tc.resolve_expr(arg);
        tc.sink.rewind(mark);
        match ty {
            Ok(crate::ty::Ty::Ref { .. }) => Callback::Borrowed,
            _ => Callback::Owned,
        }
    }

    /// The refusal a consuming terminal on a NAMED iterator gets, as the reason
    /// the hole carries.
    pub(crate) fn named_iterator_refusal(&self, call: &syn::ExprMethodCall) -> Option<String> {
        let refused = self.refuses_named_iterator_terminal(call)
            || self.refuses_named_iterator_adaptor(call);
        refused.then(|| {
            format!(
                "`{}` consumes the elements it walks and leaves the rest in the iterator this \
                 receiver names; the port writes an iterator as the whole array, so after the \
                 call it cannot say which of its elements are still the caller's",
                call.method
            )
        })
    }
}

/// Does this receiver name a place the caller still holds — an iterator the
/// chain below will only partly consume?
///
/// `Iterator::by_ref(&mut self) -> &mut Self` is a borrowed view of whatever it
/// was called on, so `it.by_ref().find(..)` names `it` exactly as
/// `(&mut it).find(..)` does. Asked of the written receiver alone, the `by_ref`
/// spelling was a method CALL and not a place: the terminal above it took the
/// owned lowering, consumed the tail Rust leaves in `it`, and the block's own
/// `dropOwned(it)` released it a second time (O5).
fn names_an_iterator_place(receiver: &syn::Expr) -> bool {
    match receiver {
        syn::Expr::MethodCall(call) if call.method == "by_ref" && call.args.is_empty() => {
            names_an_iterator_place(&call.receiver)
        }
        other => crate::body::is_place(other),
    }
}
