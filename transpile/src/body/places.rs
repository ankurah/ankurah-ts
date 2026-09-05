//! Reading a field, and the places a value moves out of.
//!
//! For: a Rust field read reaches through whatever wrappers stand between the
//! name and the value — a `Box`, an `Arc`, a guard — and the port holds each of
//! those differently, so `self.inner.count` is not `this.inner.count` until the
//! engine has said what `inner` is. And a read that MOVES is not a read at all:
//! `let x = s.field` takes the field out of the struct, which the runtime has
//! its own call for, because the struct keeps the rest and stops releasing that
//! one.

use crate::name_map;
use crate::native_types;

use super::{turbofish_type, turbofish_written, BodyTranslator};

impl BodyTranslator<'_> {
    // ── Field reads and the places a value moves out of ─────────────

    /// A field read split into what stands to the left of the field name and
    /// the field name itself, with every wrapper the field sits behind written
    /// out — one accessor per hop the engine took to find it.
    pub(crate) fn field_parts(&self, field: &syn::ExprField) -> (String, String) {
        let base = self.expr(&field.base);
        let member = match &field.member {
            syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
        };
        // A body with no type context never went through `path_expr`, so a bare
        // `self` written there still has to become the receiver this body emits
        // it as. One that did is already correct and must not be rewritten:
        // taking a module-level function's `self` parameter back to `this` read
        // a binding that does not exist there.
        let base = if base == "self" { self.self_name.to_string() } else { base };
        // A field read off an awaited value needs the same parentheses a method
        // call on one needs: JavaScript's `await` binds looser than the `.`.
        let base = crate::body::parenthesise_receiver(&field.base, base);
        // Reading a field off something the expression itself produced —
        // `m.lock().unwrap().n` — leaves that value with nobody to release it.
        // Rust drops it at the end of the statement, and it is the guard case
        // that shows: the mutex stayed locked for the life of the program.
        let base = self.hoist_produced(&field.base, base);
        let Some(tc) = &self.types else {
            return (base, member);
        };
        let found = tc.borrow().resolve_field_access(&field.base, &member);
        let instead = format!("`.{}` is emitted without a wrapper accessor", member);
        let Some(found) = self.or_fallback(found, &instead) else {
            return (base, member);
        };
        let mut receiver = base;
        for accessor in found.accessors() {
            receiver.push('.');
            receiver.push_str(&accessor);
        }
        (receiver, member)
    }

    /// A call written as the module-level function its impl was emitted as.
    ///
    /// The receiver goes first, as it does in Rust's own reading of a method.
    /// Where the impl is a blanket one the engine picked it without knowing
    /// what the receiver will be at run time — the bound is what the receiver
    /// has to satisfy, and several impls of the trait may satisfy it — so the
    /// site says which impl was written and which ones the emitted call cannot
    /// reach.
    pub(crate) fn render_free_call(
        &self,
        free: &crate::emit_impls::FreeCall,
        receiver: &str,
        args: &[String],
        call: &syn::ExprMethodCall,
    ) -> String {
        let name = if free.is_blanket {
            self.open_dispatcher(free, call).unwrap_or_else(|| free.name.clone())
        } else {
            free.name.clone()
        };
        let mut written = vec![receiver.to_string()];
        written.extend(args.iter().cloned());
        format!("{}({})", name, written.join(", "))
    }

    /// The function that picks among a trait's impls at run time, for a call
    /// the engine resolved only to the blanket one.
    ///
    /// The blanket impl is what the engine picks when the bound is open, and it
    /// is right only for the receivers the blanket is written for. The
    /// dispatcher tests the receiver's shape instead, so every impl of the
    /// trait is reachable — and where no dispatcher can be written, the site
    /// says which impls the emitted call cannot reach.
    fn open_dispatcher(
        &self,
        free: &crate::emit_impls::FreeCall,
        call: &syn::ExprMethodCall,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let reg = tc.registry;
        let trait_ref = reg.impl_def(free.impl_id).trait_ref.as_ref()?;
        let trait_id = trait_ref.id;
        let trait_name = reg.name_of(trait_id);
        // A trait another crate declares carries its dispatcher there, and this
        // run does not read that crate.
        let declared_here = reg
            .def(trait_id)
            .is_some_and(|def| !reg.modules().get(def.module).is_system);
        let refused = if declared_here {
            crate::emit_impls::dispatcher_refusal(reg, trait_id, &trait_name, &call.method.to_string())
        } else {
            Some("the trait is declared outside this crate, where its dispatcher lives".to_string())
        };
        if let Some(why) = refused {
            drop(tc);
            self.fallback(
                syn::spanned::Spanned::span(call),
                format!(
                    "`{}` here dispatches through a bound the engine cannot close, and no \
                     run-time selection among the trait's impls can be written because {}; the \
                     call is written as `{}`, the blanket impl's function, and a receiver one \
                     of the trait's other impls is written for reaches the wrong one",
                    call.method, why, free.name
                ),
            );
            return None;
        }
        crate::emit_impls::record_wanted(trait_id, &call.method.to_string());
        Some(crate::emit_impls::dispatcher_name(
            &trait_name,
            &crate::name_map::map_fn_name(&call.method.to_string()),
        ))
    }

    /// The name a call through an OPEN BOUND has to write, where the trait's
    /// impls become module-level functions.
    ///
    /// The engine resolved `subject.members()` only to `TClock`'s declaration,
    /// because `subject` is a type parameter. `Clock`'s impl of that trait is
    /// written in core while `Clock` itself is declared in proto, so the method
    /// is `Clock_members(self)` and the class carries nothing called `members`.
    /// One such impl is called by name; several go through the dispatcher.
    /// `v[a..b]`, which is a SLICE and not an index.
    ///
    /// Emitting the range as an index expression produced `v[/* range a..b */]`,
    /// which does not parse. Both ends are value positions, like any operand.
    pub(crate) fn slice_of(&self, idx: &syn::ExprIndex) -> Option<String> {
        let syn::Expr::Range(range) = &*idx.index else { return None };
        let from = range
            .start
            .as_ref()
            .map(|e| self.expr_value(e))
            .unwrap_or_else(|| "0".to_string());
        let end = match (&range.limits, range.end.as_ref().map(|e| self.expr_value(e))) {
            // `..=b` includes the last element.
            (syn::RangeLimits::Closed(_), Some(to)) => format!(", {} + 1", to),
            (_, Some(to)) => format!(", {}", to),
            (_, None) => String::new(),
        };
        let base = self.expr(&idx.expr);
        let base = crate::body::parenthesise_receiver(&idx.expr, base);
        Some(format!("{}.slice({}{})", base, from, end))
    }

    /// A call of a two-segment path on a primitive the port has no spelling
    /// for — `i64::from_be_bytes(bytes)` — as ONE hole.
    ///
    /// The hole has to stand where the CALL stands, not where the callee does:
    /// `unsupported(..)` answers `never`, and TypeScript refuses to call a
    /// `never`, so a hole written into the callee position left the emitted
    /// file failing to typecheck at a line that was already refused.
    pub(crate) fn primitive_call_hole(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path) = call.func.as_ref() else { return None };
        let segments: Vec<String> =
            path.path.segments.iter().map(|s| s.ident.to_string()).collect();
        match crate::ty::prim_consts::written_or_reason(&segments)? {
            Ok(_) => None,
            Err(why) => Some(self.hole(syn::spanned::Spanned::span(call), why)),
        }
    }

    /// What the position a call stands in says about its answer: is the answer
    /// used at all, and is it read as a VALUE rather than written through by a
    /// `*`?
    ///
    /// Both questions are the CALLER's, and the unresolved path used to answer
    /// the second one "yes" whatever the caller said: a
    /// `*entry(k).or_insert(0) += 1` whose receiver did not resolve would have
    /// been written `.value.value` (R8).
    pub(crate) fn position_of(&self, call: &syn::ExprMethodCall) -> native_types::Position {
        native_types::Position {
            used: !self.discards(call),
            reads_as_value: !self.is_written_through(call),
        }
    }

    /// Is this initialiser one of the three ways Rust finishes a
    /// `map.entry(k)`?
    ///
    /// For: a finisher answers `&mut V`, and a `let` that binds one binds the
    /// write-through slot itself — `let slot = map.entry(k).or_insert(0);
    /// *slot += 1` stores into the map, and a name bound to the number it held
    /// cannot. Every read of such a name goes through the slot, which is what
    /// holding it in a cell already writes.
    pub(crate) fn finishes_an_entry(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::MethodCall(call) = expr else {
            return false;
        };
        if !matches!(
            call.method.to_string().as_str(),
            "or_insert" | "or_insert_with" | "or_default"
        ) {
            return false;
        }
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        let Ok(receiver) = tc.resolve_expr(&call.receiver) else {
            return false;
        };
        crate::native_types::map::is_entry_type(tc.registry, &receiver)
    }

    /// A call the receiver's own class does not carry.
    ///
    /// For: `receiver.method(..)` is only the right shape when the emitted
    /// class declares the method. An impl written for a type with no emitted
    /// class became module-level functions and takes its receiver first; a
    /// `Deref` step answered by a bound has no one spelling at all; and a call
    /// through an open bound whose impls are module-level functions goes
    /// through their dispatcher. Each answers the text to write instead.
    pub(crate) fn call_written_elsewhere(
        &self,
        found: &crate::registry::MethodResolution,
        recv: &str,
        args: &[String],
        call: &syn::ExprMethodCall,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc_ref = tc.borrow();
        if let Some(free) = crate::emit_impls::free_call(tc_ref.registry, found) {
            drop(tc_ref);
            return Some(self.render_free_call(&free, recv, args, call));
        }
        if let Some(what) = self.bound_deref(&tc_ref, found) {
            drop(tc_ref);
            return Some(self.hole(syn::spanned::Spanned::span(call), what));
        }
        let name = self.open_bound_call(&tc_ref, found, call)?;
        drop(tc_ref);
        let mut written = vec![recv.to_string()];
        written.extend(args.iter().cloned());
        Some(format!("{}({})", name, written.join(", ")))
    }

    /// A `Deref` or `DerefMut` call the engine answered from a BOUND rather
    /// than from an impl.
    ///
    /// For: how a value is dereferenced is a fact about the port's runtime, and
    /// the port spells it differently for each implementor — `Arc` keeps its
    /// value in a field, a lock guard is read through one, and a crate's own
    /// `impl Deref` is a method the emitted class carries. A bound names no
    /// implementor, so there is no one call to write: `values.derefMut()` on a
    /// write guard is a `TypeError`, and it stood on core's property write
    /// path.
    pub(crate) fn bound_deref(
        &self,
        tc: &crate::infer::TypeContext<'_>,
        found: &crate::registry::MethodResolution,
    ) -> Option<String> {
        let crate::registry::Callee::TraitObject(trait_id, method) = &found.callee else {
            return None;
        };
        if !matches!(method.as_str(), "deref" | "deref_mut") {
            return None;
        }
        let trait_name = tc.registry.name_of(*trait_id);
        if !matches!(trait_name.as_str(), "Deref" | "DerefMut") {
            return None;
        }
        Some(format!(
            "`{}` here is a `{}` step taken through a bound, and the port writes each \
             implementor's dereference differently — a field on `Arc`, nothing at all on a lock \
             guard, a method on a crate's own class — so no one call stands for all of them",
            method, trait_name
        ))
    }

    pub(crate) fn open_bound_call(
        &self,
        tc: &crate::infer::TypeContext<'_>,
        found: &crate::registry::MethodResolution,
        call: &syn::ExprMethodCall,
    ) -> Option<String> {
        let crate::registry::Callee::TraitObject(trait_id, method) = &found.callee else {
            return None;
        };
        let reg = tc.registry;
        let trait_name = reg.name_of(*trait_id);
        match crate::emit_impls::open_bound_call(reg, *trait_id, &trait_name, method)? {
            crate::emit_impls::OpenCall::One(name) => Some(name),
            crate::emit_impls::OpenCall::Dispatch => {
                if let Some(why) =
                    crate::emit_impls::dispatcher_refusal(reg, *trait_id, &trait_name, method)
                {
                    self.fallback(
                        syn::spanned::Spanned::span(call),
                        format!(
                            "`{}` here is called through a bound on `{}`, whose impls are \
                             emitted as module-level functions, and no run-time selection \
                             among them can be written because {}; the call is written as a \
                             method on the receiver, which carries none",
                            method, trait_name, why
                        ),
                    );
                    return None;
                }
                crate::emit_impls::record_wanted(*trait_id, method);
                Some(crate::emit_impls::dispatcher_name(
                    &trait_name,
                    &crate::name_map::map_fn_name(method),
                ))
            }
        }
    }

    /// What the receiver of a wrapper-opening call is expected to produce.
    ///
    /// `unwrap`, `expect` and their kind hand back what a `Result` or an
    /// `Option` carries, so a position that wants a `T` of the call wants a
    /// `Result<T, E>` or an `Option<T>` of the receiver — which is how
    /// `serde_json::from_str(&s).unwrap()` learns, from the value it is
    /// compared against, which type reads itself out of the parsed text. Every
    /// other method leaves the receiver's type to the receiver.
    pub(crate) fn receiver_expectation(
        &self,
        call: &syn::ExprMethodCall,
        expected: Option<&crate::ty::Ty>,
    ) -> Option<crate::ty::Ty> {
        const OPENS_A_WRAPPER: [&str; 4] = ["unwrap", "expect", "unwrap_or_default", "ok"];
        let want = expected?;
        if !OPENS_A_WRAPPER.contains(&call.method.to_string().as_str()) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let receiver = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok()?;
        // The wrapper is the receiver's own, so the expectation is written
        // inside whichever one it turns out to be rather than guessed at.
        let crate::ty::Ty::Named { id, args } = receiver.peel_refs() else {
            return None;
        };
        let tc = tc.borrow();
        if !tc.is_result(&receiver) && !tc.is_option(&receiver) {
            return None;
        }
        Some(crate::ty::Ty::Named {
            id: *id,
            args: std::iter::once(want.clone())
                .chain(args.iter().skip(1).cloned())
                .collect(),
        })
    }

    /// A list literal, written as the runtime type the position wants.
    ///
    /// `Vec<u8>` and `[u8; N]` are a `Uint8Array` in the port, and a plain
    /// JavaScript array compares unequal to one — which is what
    /// `assert_eq!(bytes, [1, 2, 3])` was failing on.
    /// Does this position want a `Uint8Array` rather than an array?
    pub(crate) fn expects_bytes(&self, expected: &crate::ty::Ty) -> bool {
        match &self.types {
            Some(tc) => crate::infer::expected::expects_bytes(tc.borrow().registry, expected),
            None => false,
        }
    }

    pub(crate) fn sequence_literal(
        &self,
        items: Vec<String>,
        expected: Option<&crate::ty::Ty>,
    ) -> String {
        let list = format!("[{}]", items.join(", "));
        let bytes = match (expected, &self.types) {
            (Some(want), Some(tc)) => {
                crate::infer::expected::expects_bytes(tc.borrow().registry, want)
            }
            _ => false,
        };
        if bytes {
            format!("new Uint8Array({})", list)
        } else {
            list
        }
    }

    /// What each field of a struct literal is declared to hold.
    pub(crate) fn struct_field_types(
        &self,
        lit: &syn::ExprStruct,
    ) -> Vec<(String, crate::ty::Ty)> {
        match &self.types {
            Some(tc) => self.quietly(|| tc.borrow().struct_literal_field_types(lit)),
            None => Vec::new(),
        }
    }

    /// The order the declaration puts its fields in, which is the order the
    /// emitted constructor takes them. Answered even where the literal's type
    /// ARGUMENTS cannot be resolved — `Attested { payload, attestations }`
    /// writes no `T`, and the order does not depend on one.
    pub(crate) fn struct_field_order(&self, lit: &syn::ExprStruct) -> Vec<String> {
        match &self.types {
            Some(tc) => self.quietly(|| tc.borrow().struct_literal_field_order(lit)),
            None => Vec::new(),
        }
    }

    /// Which type reads itself out of a `serde_json::from_str` or a
    /// `bincode::deserialize`.
    ///
    /// Rust takes it from the turbofish where one is written and from the
    /// position otherwise — `let id: EntityId = bincode::deserialize(&bytes)?`
    /// names `EntityId` as surely as `deserialize::<EntityId>` does — so both
    /// are read here, the turbofish first because it is what the source said.
    pub(crate) fn read_into_type(
        &self,
        callee: Option<&syn::Path>,
        span: proc_macro2::Span,
    ) -> Option<String> {
        if let Some(written) = turbofish_type(callee) {
            return Some(written);
        }
        let want = self.expectation_at(span)?;
        let tc = self.types.as_ref()?;
        // The call yields the value itself; a `Result` or an `Option` around it
        // belongs to the `?` or the `unwrap` that follows, not to the type that
        // reads itself out of the bytes.
        let payload = tc.borrow().unwrapped_payload(&want);
        Some(name_map::map_ty(tc.borrow().registry, &payload))
    }

    /// Does the type this call reads into declare a `static fromJson`?
    ///
    /// The written turbofish or the position's expected type names it, and the
    /// registry says whether its serde derive wrote one.
    pub(crate) fn reads_json(&self, callee: Option<&syn::Path>, span: proc_macro2::Span) -> bool {
        let Some(tc) = self.types.as_ref() else { return false };
        let tc = tc.borrow();
        let ty = match turbofish_written(callee) {
            Some(written) => {
                let mark = tc.sink.mark();
                let resolved = tc.resolve_written_type(&written);
                tc.sink.rewind(mark);
                match resolved {
                    Ok(ty) => ty,
                    Err(_) => return false,
                }
            }
            None => match self.expectation_at(span) {
                Some(want) => tc.unwrapped_payload(&want),
                None => return false,
            },
        };
        // A hand-written class carries whatever its own file wrote, and the
        // engine has not read it: `Attested<T>`'s JSON lives in
        // `auth.provided.ts`, so the call is left to the port rather than
        // refused here.
        match ty.peel_refs().id() {
            Some(id) => tc.registry.reads_json(id) || tc.registry.members_are_hand_written(id),
            None => false,
        }
    }

    /// What the callee of a method call declares each of its arguments to be.
    ///
    /// Asking is not translating: the resolution files whatever it could not
    /// settle, and the call is resolved again when it is written, so the record
    /// is wound back and the same gap is not counted twice.
    pub(crate) fn argument_types(&self, call: &syn::ExprMethodCall) -> Vec<Option<crate::ty::Ty>> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let method = call.method.to_string();
        let found = self.quietly(|| {
            tc.borrow()
                .resolve_method_call_with(&call.receiver, &method, call.turbofish.as_ref())
        });
        let Ok(found) = found else {
            return Vec::new();
        };
        // The bound is written in the callee's own terms — `FnMut(Self::Item)`
        // — so the projection is settled against the receiver before the
        // closure is asked to take its parameter from it. A projection left
        // standing is not a type anything inside the closure can be resolved
        // against.
        let tc = tc.borrow();
        let probe = tc.probe();
        tc.registry
            .method_param_types(&found)
            .into_iter()
            .map(|ty| Some(probe.normalize(&ty)))
            .collect()
    }

}

/// Every local this block hands out as `&mut`, by the name the emitter writes.
///
/// A `&mut` to a class is already a reference in JavaScript and needs no cell;
/// the decision about the TYPE is made where the local is declared, which is
/// the only place the type is known. This is the syntactic half: which names
/// are borrowed mutably at all.
pub(crate) fn cells_wanted(block: &syn::Block) -> Vec<String> {
    struct Borrows {
        names: Vec<String>,
    }
    impl syn::visit::Visit<'_> for Borrows {
        fn visit_expr_reference(&mut self, node: &syn::ExprReference) {
            if node.mutability.is_some() {
                if let syn::Expr::Path(path) = &*node.expr {
                    if path.path.segments.len() == 1 {
                        let name = crate::name_map::escape_reserved(&crate::name_map::to_camel_case(
                            &path.path.segments[0].ident.to_string(),
                        ));
                        if !self.names.contains(&name) {
                            self.names.push(name);
                        }
                    }
                }
            }
            syn::visit::visit_expr_reference(self, node);
        }
        // A closure's own body borrows in its own scope.
        fn visit_expr_closure(&mut self, _: &syn::ExprClosure) {}
    }
    let mut borrows = Borrows { names: Vec::new() };
    syn::visit::Visit::visit_block(&mut borrows, block);
    borrows.names
}
