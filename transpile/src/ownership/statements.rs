//! What one statement releases before the next one runs.
//!
//! Three statements do something to a value that the block's own `finally`
//! cannot answer for. An assignment replaces what the place held, and Rust
//! drops the old value after the new one is built and before it is stored — a
//! `let mut` reassigned in a loop leaked one object per turn without it. A
//! statement whose value goes nowhere drops that value at the semicolon. And
//! `drop(x)` releases `x` exactly where the source says, the move analysis
//! having already taken it off the block's list.
//!
//! Each of them needs the same thing: the cost of releasing one value, written
//! against the text that names it. Where the emitter cannot pay that cost —
//! because the runtime writes the call as something whose result is not the
//! value Rust returns, or because a macro it does not expand was handed the
//! value — it says so at the site rather than emitting a release that would be
//! wrong.

use crate::body::BodyTranslator;
use crate::name_map;
use crate::native_types;
use crate::ownership;

impl<'a> BodyTranslator<'a> {
    /// `place = value`, with what the place held released where Rust releases
    /// it: after the new value is evaluated and before it is stored.
    ///
    /// The bare assignment abandoned the old value — a `let mut` reassigned in
    /// a loop leaked one object per turn.
    pub(crate) fn assign(&self, assign: &syn::ExprAssign) -> String {
        // `*place = value` stores through the wrapper, so the target is written
        // as a place rather than as a value.
        let left = match &*assign.left {
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.deref_place(unary)
            }
            other => self.expr(other),
        };
        // `*guard = value` is the runtime's own store: a `WriteGuard`'s setter
        // drops what the container held and then stores the new value, which is
        // the order Rust runs it in. A release written here as well dropped
        // that value a second time, which the runtime reports as a double drop.
        if self.stores_through_a_guard(&assign.left) {
            return format!("{} = {}", left, self.assigned_value(assign));
        }
        let Some(release) = self.release_of(&assign.left, &left) else {
            return format!("{} = {}", left, self.assigned_value(assign));
        };
        // Where a branch already handed the old value away, whether there is
        // anything left to release is what the drop flag answers, and a flag
        // reset is not a thing this emitter writes.
        if self.own.flags.borrow().contains_key(&left) {
            self.fallback(
                syn::spanned::Spanned::span(assign),
                format!(
                    "`{}` is handed away on some path and assigned on another; the value it \
                     held at the assignment is not released",
                    left
                ),
            );
            return format!("{} = {}", left, self.assigned_value(assign));
        }
        let held = self.fresh_hoist("_a");
        let right = self.assigned_value(assign);
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!("const {} = {};\n{}\n", held, right, release),
            owned: None,
            temp: None,
            refused: false,
            released_if_unreached: false,
            wrapper: false,
            flag: None,
        });
        format!("{} = {}", left, held)
    }

    /// The value being stored, asked for as the type the place holds.
    ///
    /// `state.head = event.id().into()` says what the `.into()` converts to
    /// nowhere else: the field is the only thing that names it.
    fn assigned_value(&self, assign: &syn::ExprAssign) -> String {
        let want = self.quietly(|| self.resolve_expr_type(&assign.left)).ok();
        self.expecting(&assign.right, want.as_ref(), || {
            self.moved_value(&assign.right)
        })
    }

    /// Does this place write through a guard, which releases what the container
    /// held as part of the store itself?
    fn stores_through_a_guard(&self, place: &syn::Expr) -> bool {
        let syn::Expr::Unary(unary) = place else {
            return false;
        };
        if !matches!(unary.op, syn::UnOp::Deref(_)) {
            return false;
        }
        let Some(tc) = &self.types else { return false };
        let ty = match self.quietly(|| self.resolve_expr_type(&unary.expr)) {
            Ok(ty) => ty,
            // A place the engine could not type is one nobody can say holds a
            // guard, and the answer decides whether the store releases what the
            // place held. Answering "no" in silence was one of the item-12
            // fallthroughs: the store is written as an ordinary assignment and
            // whatever the place held is not released.
            Err(_) => {
                self.fallback(
                    syn::spanned::Spanned::span(place),
                    "this store writes through a dereference the engine could not type, so \
                     whether the place holds a guard is not decided and the value it held is \
                     not released",
                );
                return false;
            }
        };
        ownership::drops_of(&tc.borrow().probe(), &ty) == ownership::Drops::Guard
    }

    /// `option.unwrap_or(default)` where the default owes a release.
    ///
    /// Rust evaluates the receiver, then the default, then keeps one of them and
    /// drops the other — so the emitted code names the receiver first, builds
    /// the default second, chooses, and releases the one it did not choose.
    /// Leaving the receiver inside the `??` built the default first, which runs
    /// two side effects in the opposite order from the program that was written.
    /// Where the default owns nothing there is nothing to release and `??`
    /// stands on its own.
    pub(crate) fn nullable_default(
        &self,
        receiver: &str,
        default: &syn::Expr,
        default_ts: &str,
    ) -> Option<String> {
        let subject = self.fresh_hoist("_o");
        let held = self.fresh_hoist("_d");
        let release = self.release_of(default, &held)?;
        let chosen = self.fresh_hoist("_u");
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!(
                "const {subject} = {receiver};\nconst {held} = {default_ts};\n\
                 const {chosen} = {subject} ?? {held};\n\
                 if ({chosen} !== {held}) {release}\n",
                subject = subject,
                receiver = receiver,
                held = held,
                default_ts = default_ts,
                chosen = chosen,
                release = release,
            ),
            owned: None,
            temp: None,
            refused: false,
            released_if_unreached: false,
            wrapper: false,
            flag: None,
        });
        Some(chosen)
    }

    /// What releasing the value in this place costs, written against the text
    /// that names it.
    pub(crate) fn release_of(&self, place: &syn::Expr, text: &str) -> Option<String> {
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(place)).ok()?;
        ownership::drops_of(&tc.borrow().probe(), &ty).release(text)
    }

    /// `drop(x)`, written as whatever releasing an `x` costs.
    ///
    /// The move analysis has already taken `x` off the block's list — it is an
    /// argument passed by value like any other — so this releases it once,
    /// where the source says.
    pub(crate) fn explicit_drop(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path) = &*call.func else {
            return None;
        };
        let written = crate::body::BodyTranslator::path_static(&path.path);
        if !matches!(written.as_str(), "drop" | "mem.drop") || call.args.len() != 1 {
            return None;
        }
        let arg = &call.args[0];
        let text = self.moved_value(arg);
        let tc = self.types.as_ref()?;
        let ty = self.quietly(|| self.resolve_expr_type(arg)).ok()?;
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        match drops {
            ownership::Drops::Unknown => {
                self.fallback(
                    syn::spanned::Spanned::span(call),
                    "`drop` is called on a value the engine cannot say owns anything, so \
                     nothing releases it",
                );
                None
            }
            // Rust runs no glue for a `Copy` value; the argument is still
            // evaluated, and `void` says the result goes nowhere.
            ownership::Drops::Nothing => Some(format!("void {}", text)),
            _ => drops.release_expr(&text),
        }
    }

    /// Does the runtime write this call as something whose result is not what
    /// Rust returns?
    ///
    /// `map.insert(k, v)` becomes `map.set(k, v)`, and JavaScript's `Map.set`
    /// hands back the map where Rust hands back the value it displaced.
    /// Releasing the statement's value there would release the whole map.
    pub(crate) fn rewritten_by_runtime(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::MethodCall(call) = expr else {
            return false;
        };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating; the call reports its own gaps where it is
        // written out.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(
            &call.receiver,
            &call.method.to_string(),
            call.turbofish.as_ref(),
        );
        tc.sink.rewind(mark);
        let found = match found {
            Ok(found) => found,
            // A call the engine could not resolve is one nobody can say the
            // runtime rewrites, and the answer decides whether the statement
            // releases what the call produced. Answering "no" in silence was
            // one of the item-12 fallthroughs: `map.insert(k, v);` on an
            // untyped map released the value the call answered — which for the
            // runtime's rewrite is the map itself.
            Err(_) => {
                drop(tc);
                self.fallback(
                    syn::spanned::Spanned::span(call),
                    format!(
                        "`{}` is a call the engine could not resolve, so whether the runtime \
                         writes it as something whose result is not the value Rust returns is \
                         not decided, and the statement releases what it answered",
                        call.method
                    ),
                );
                return false;
            }
        };
        let args: Vec<String> = call.args.iter().map(|_| "_".to_string()).collect();
        let translated = native_types::translate_method(
            tc.registry,
            found.receiver_type(),
            "_",
            &call.method.to_string(),
            &args,
        );
        matches!(translated, native_types::MethodTranslation::Expr(_))
    }

    /// A statement's value, thrown away. Rust drops it at the end of the
    /// statement, so the emitted statement releases it there.
    pub(crate) fn discard(&self, expr: &syn::Expr, text: String) -> String {
        // Every form that *builds* a value, not only the calls. `Owned::new();`
        // and `vec![Owned::new()];` each produce something Rust drops at the
        // semicolon, and both used to walk past this. The forms left out are
        // the ones that produce no value of their own: a place names storage
        // somebody else owns, and an `if`, a `match` or a block is written as
        // statements, which no release can be wrapped around.
        if !matches!(
            expr,
            syn::Expr::Call(_)
                | syn::Expr::MethodCall(_)
                | syn::Expr::Await(_)
                | syn::Expr::Struct(_)
                | syn::Expr::Array(_)
                | syn::Expr::Tuple(_)
                | syn::Expr::Cast(_)
                | syn::Expr::Paren(_)
        ) {
            return text;
        }
        let Some(tc) = &self.types else { return text };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(expr)) else {
            return text;
        };
        // `n.notified().await` yields the future's `Output`, and awaiting takes
        // the future itself for good. Where the engine could not project
        // through `Future` it hands back the future's own type, and releasing
        // that would drop a value the await already moved.
        if let syn::Expr::Await(await_expr) = expr {
            if self.quietly(|| self.resolve_expr_type(&await_expr.base)).ok().as_ref() == Some(&ty) {
                self.fallback(
                    syn::spanned::Spanned::span(expr),
                    "the engine could not say what awaiting this produces, so the value the \
                     statement threw away is not released",
                );
                return text;
            }
        }
        let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
        // A guard the statement produced is already lifted and released by the
        // hoist machinery, which also lists it in the enclosing `finally`.
        if drops == ownership::Drops::Guard {
            return text;
        }
        if self.rewritten_by_runtime(expr) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "the runtime writes this call as something whose result is not the value Rust \
                 returns, so the value the statement threw away is not released",
            );
            return text;
        }
        let held = if text.starts_with("await ") {
            format!("({})", text)
        } else {
            text.clone()
        };
        drops.release_expr(&held).unwrap_or(text)
    }

    /// Say so where a `select!` has a shape this lowering does not carry over.
    pub fn report_select_gap(&self, tokens: &proc_macro2::TokenStream, what: &str) {
        self.fallback(
            syn::spanned::Spanned::span(tokens),
            format!("`select!` is lowered to the runtime's arbiter, but {}", what),
        );
    }

    /// Say so where a macro the emitter does not expand is handed a value the
    /// block owns: the macro becomes a comment, and the value goes with it.
    pub fn report_unsupported_macro(&self, mac: &syn::Macro, name: &str) {
        let parse = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let Ok(args) = syn::parse::Parser::parse2(parse, mac.tokens.clone()) else {
            return;
        };
        let owned: Vec<String> = args
            .iter()
            .filter_map(|arg| match arg {
                syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                    Some(path.path.segments[0].ident.to_string())
                }
                _ => None,
            })
            .map(|name| name_map::to_camel_case(&name))
            .filter(|name| self.owes_a_release(name))
            .collect();
        if owned.is_empty() {
            return;
        }
        self.fallback(
            syn::spanned::Spanned::span(mac),
            format!(
                "`{}!` is emitted as a comment and is handed {}, which nothing then releases",
                name,
                owned
                    .iter()
                    .map(|n| format!("`{}`", n))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
}
