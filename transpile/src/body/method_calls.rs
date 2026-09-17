//! One method call, written as the JavaScript the receiver's own type says.
//!
//! For: which lowering a call takes is decided by the receiver — a runtime
//! type's own method, a native shape's rewrite, a trait's dispatcher, or the
//! name unchanged — and holding that decision in one place is what keeps the
//! four from disagreeing about the same call.

use super::{parenthesise_receiver, BodyTranslator};
use crate::ty::Ty;
use crate::{name_map, native_types};

impl BodyTranslator<'_> {
    /// The call, at the type the position around it wants.
    pub(crate) fn method_call_value(
        &self,
        call: &syn::ExprMethodCall,
        expected: Option<&Ty>,
    ) -> String {
            // `unwrap` and its kind take a wrapper apart, so what the
            // position wants of the whole is what it wants of the payload:
            // `assert_eq!(id, from_str(&s).unwrap())` says the parse
            // produces an `EntityId`, and nothing else does.
            let through = self.receiver_expectation(call, expected);
            // A receiver is a VALUE position: `(if ok { a } else { b }).len()`
            // written as a statement puts an `if` in front of the `.`, which
            // does not parse.
            let receiver = self.expecting(&call.receiver, through.as_ref(), || {
                self.receiver_value(call)
            });
            let receiver = self.hoist_receiver(call, receiver);
            let receiver = parenthesise_receiver(&call.receiver, receiver);
            let rust_method = call.method.to_string();
            let (receiver, named_early) = self.name_nullable_receiver_early(call, &rust_method, receiver);
            let ts_method = name_map::map_fn_name(&rust_method);

            let args = self.method_arguments(call, &rust_method);

            if let Some(w) = self.answered_before_dispatch(call, &rust_method, &receiver, &args) {
                return w;
            }

            // ── unwrap/expect: single decision point ──
            // In TS, only Result has a real .unwrap(). All other types
            // (guards, Option/nullable, etc.) treat it as identity.
            if matches!(rust_method.as_str(), "unwrap" | "expect") {
                // A `Result` the port really builds is unwrapped; a lock
                // call's `LockResult` was never built, because the port's
                // `lock()` hands back the guard. Everything else — a guard,
                // a nullable — has no `unwrap` of its own and the call
                // writes nothing.
                let receiver_ty = self.resolve_expr_type(&call.receiver);
                let instead = format!("`{}` is treated as the identity", rust_method);
                let is_result = self
                    .or_fallback(receiver_ty, &instead)
                    .and_then(|ty| self.types.as_ref().map(|tc| tc.borrow().is_result(&ty)))
                    .unwrap_or(false)
                    && !self.writes_the_value_not_the_result(&call.receiver);
                if !is_result {
                    // The call still resolved — `unwrap` on a `LockResult`
                    // is `Result::unwrap` and hands back the guard — so it
                    // is recorded before the runtime's own answer is
                    // written.
                    self.record_resolution(call, &rust_method);
                    // On an `Option` these PANIC when there is nothing
                    // there, and the port writes `Option<T>` as `T | null`:
                    // written as the identity, `steps.last().expect("..")`
                    // handed the `null` on and the message was thrown away,
                    // so a `None` became a value read further down instead
                    // of a stop. `??` reads exactly null and undefined,
                    // which is what "nothing there" is here, and it reads
                    // the receiver once.
                    let nullable = self
                        .resolve_expr_type(&call.receiver)
                        .ok()
                        .is_some_and(|ty| self.is_nullable(&ty));
                    if nullable {
                        let message = match (rust_method.as_str(), args.first()) {
                            ("expect", Some(text)) => text.clone(),
                            _ => crate::body::quoted(
                                "called `Option::unwrap()` on a `None` value",
                            ),
                        };
                        return format!(
                            "({} ?? (() => {{ throw new Error({}); }})())",
                            receiver, message
                        );
                    }
                    return receiver.to_string();
                }
            }
            if let Some(written) =
                self.nullable_unwrap_or(call, &rust_method, &receiver, &args)
            {
                return written;
            }

            // ── a conversion ──
            // `into`, `try_into`, `to_string` and `to_owned` name a
            // conversion rather than a method the receiver's type carries,
            // and what the port writes for one is decided by the pair of
            // types, not by the name.
            if let Some(converted) =
                self.conversion_method(call, &receiver, expected)
            {
                self.record_resolution(call, &rust_method);
                return converted;
            }

            // ── the resolved call ──
            // The engine says which function this is and what has to be
            // written between the receiver and it: one accessor per wrapper
            // the chain went through. The translation is then chosen by the
            // type the callee is actually written for, not by the method's
            // name.
            if let Some(tc) = &self.types {
                let found = tc.borrow().resolve_method_call_with(
                    &call.receiver,
                    &rust_method,
                    call.turbofish.as_ref(),
                );
                let instead = format!("`{}` is dispatched by name", rust_method);
                if let Some(found) = self.or_fallback(found, &instead) {
                    let mut recv =
                        self.receiver_of(call, &rust_method, receiver.clone());
                    if let Some(refused) = self.lost_write_through_a_holder(call, &found, &rust_method) {
                        return self.render_translation(
                            refused,
                            &recv,
                            &ts_method,
                            &args,
                            syn::spanned::Spanned::span(&call.receiver),
                        );
                    }
                    for accessor in found.accessors() {
                        recv = format!("{}.{}", recv, accessor);
                    }
                    let tc_ref = tc.borrow();
                    crate::trace::record(
                        tc_ref.registry,
                        &tc_ref.sink.file(),
                        syn::spanned::Spanned::span(&call.receiver),
                        &rust_method,
                        &found,
                    );
                    // Three calls the receiver's own class does not carry,
                    // each written somewhere other than
                    // `receiver.method(..)`.
                    drop(tc_ref);
                    if let Some(written) = self.call_written_elsewhere(&found, &recv, &args, call) {
                        return written;
                    }
                    let tc_ref = tc.borrow();
                    let call_args: Vec<syn::Expr> = call.args.iter().cloned().collect();
                    let bind_receiver = |written: &str| match named_early {
                        // Already named above, in Rust's evaluation order.
                        true => written.to_string(),
                        false => self.name_once(Some(&call.receiver), written),
                    };
                    let bind_eager = |at: usize, written: &str| {
                        self.name_eager(call_args.get(at), written)
                    };
                    let bind_closure = |_: usize, written: &str| self.name_closure(written);
                    let once = native_types::nullable::Once {
                        bind_receiver: &bind_receiver,
                        bind_eager: &bind_eager,
                        bind_closure: &bind_closure,
                    };
                    drop(tc_ref);
                    let shape =
                        self.receiver_shape_of(call, &rust_method, found.receiver_type());
                    let tc_ref = tc.borrow();
                    let translated = native_types::translate_method_using(
                        tc_ref.registry,
                        &shape,
                        &recv,
                        &rust_method,
                        &args,
                        self.position_of(call),
                        &once,
                    );
                    // `Ord::cmp` owns `compareTo`, and a written-out
                    // `PartialOrd::partial_cmp` is a method of its own. The
                    // CALL has to write the name the class declares — and
                    // only a call the native tables PASS THROUGH writes a
                    // name at all: a `partial_cmp` on a number is a
                    // comparison written out, and asking there reported a
                    // question nothing was about to answer.
                    let named = match matches!(translated, native_types::MethodTranslation::Passthrough) {
                        true => self.ordering_method_name(&tc_ref, &found, &rust_method, &ts_method, call),
                        false => ts_method.clone(),
                    };
                    drop(tc_ref);
                    // Spec 4.4b: only a call the native tables passed
                    // through is an emitted function with a dictionary
                    // parameter to fill.
                    let passthrough =
                        matches!(translated, native_types::MethodTranslation::Passthrough);
                    let args = self.with_dictionaries(passthrough, args, &found, call);
                    return self.render_translation(
                        translated,
                        &recv,
                        &named,
                        &args,
                        syn::spanned::Spanned::span(call),
                    );
                }
            }

            let arg_exprs: Vec<syn::Expr> = call.args.iter().cloned().collect();
            self.translate_unresolved_call_using(
                &receiver,
                &rust_method,
                &ts_method,
                &args,
                &arg_exprs,
                Some(&call.receiver),
                self.position_of(call),
            )
    }
}
