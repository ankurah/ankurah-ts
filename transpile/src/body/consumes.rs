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
        self.consuming_terminal(call) && !crate::body::is_place(&call.receiver)
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
        self.consuming_terminal(call) && crate::body::is_place(&call.receiver)
    }

    /// The same question without the place clause: is this a terminal that
    /// consumes droppable elements at all? The refusal reads it too.
    pub(crate) fn consuming_terminal(&self, call: &syn::ExprMethodCall) -> bool {
        let method = call.method.to_string();
        if !crate::native_types::iterator::is_owned_terminal(&method, call.args.len()) {
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
        match self.terminal_owns_the_sequence(call) {
            true => Elements::Owned,
            false => Elements::Borrowed,
        }
    }

    /// The refusal a consuming terminal on a NAMED iterator gets, as the reason
    /// the hole carries.
    pub(crate) fn named_iterator_refusal(&self, call: &syn::ExprMethodCall) -> Option<String> {
        self.refuses_named_iterator_terminal(call).then(|| {
            format!(
                "`{}` consumes the elements it walks and leaves the rest in the iterator this \
                 receiver names; the port writes an iterator as the whole array, so after the \
                 call it cannot say which of its elements are still the caller's",
                call.method
            )
        })
    }
}
