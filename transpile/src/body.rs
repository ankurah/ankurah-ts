//! Phase 2: Body translation — syn::Expr/Stmt → TS expression/statement text
//!
//! Translates Rust function bodies to TypeScript. Each syn expression variant
//! maps to a TS expression string. The output is deterministic and structural,
//! prioritizing 1:1 correspondence with the Rust source over elegance.

use syn;
use crate::name_map;
use crate::macros;
use crate::match_expr;
use crate::control_flow;
use crate::ownership;
use crate::native_types;

/// Check if an expression is a write!/writeln! macro call
fn is_write_macro(expr: &syn::Expr) -> bool {
    if let syn::Expr::Macro(mac) = expr {
        let name = mac.mac.path.segments.last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        matches!(name.as_str(), "write" | "writeln")
    } else {
        false
    }
}

/// Does this formatter body COMPOSE, or does every path through it write once?
///
/// `fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "{}", self.0) }`
/// composes nothing: the one write IS the string, and so is each arm of a
/// `match self { .. }` whose arms each write once. Those need no accumulator,
/// and the method stays the expression it always was. A body that writes twice
/// in sequence needs one, and it is the only thing that does.
pub fn writes_once_at_the_tail(block: &syn::Block) -> bool {
    matches!(block.stmts.as_slice(), [syn::Stmt::Expr(expr, None)] if writes_once(expr))
}

fn writes_once(expr: &syn::Expr) -> bool {
    match expr {
        _ if as_write_macro(expr).is_some() => true,
        syn::Expr::Match(m) => m.arms.iter().all(|arm| writes_once(&arm.body)),
        syn::Expr::If(if_expr) => {
            single_block_expr(&if_expr.then_branch).is_some_and(writes_once)
                && if_expr.else_branch.as_ref().is_some_and(|(_, e)| writes_once(e))
        }
        syn::Expr::Block(block) => writes_once_at_the_tail(&block.block),
        syn::Expr::Paren(p) => writes_once(&p.expr),
        _ => false,
    }
}

/// Is this macro a `write!` or a `writeln!`?
fn is_write_macro_path(mac: &syn::Macro) -> bool {
    let name = mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
    matches!(name.as_str(), "write" | "writeln")
}

/// The `write!`/`writeln!` an expression is, through the `?` it may carry.
pub(crate) fn as_write_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    match expr {
        syn::Expr::Try(try_expr) => as_write_macro(&try_expr.expr),
        syn::Expr::Paren(p) => as_write_macro(&p.expr),
        _ if is_write_macro(expr) => extract_macro(expr),
        _ => None,
    }
}

/// Extract the Macro from an expression (for write! detection)
fn extract_macro(expr: &syn::Expr) -> Option<&syn::Macro> {
    if let syn::Expr::Macro(mac) = expr {
        Some(&mac.mac)
    } else {
        None
    }
}

/// Check if a match expression has arms that are write! macro calls (Display pattern)
fn is_match_with_write_arms(expr: &syn::Expr) -> bool {
    if let syn::Expr::Match(m) = expr {
        m.arms.iter().any(|arm| {
            matches!(&*arm.body, syn::Expr::Try(t) if is_write_macro(&t.expr))
        })
    } else {
        false
    }
}

/// Extract a single expression from a block (for ternary conversion)
fn single_block_expr(block: &syn::Block) -> Option<&syn::Expr> {
    if block.stmts.len() == 1 {
        if let syn::Stmt::Expr(expr, _) = &block.stmts[0] {
            return Some(expr);
        }
    }
    None
}

// ── Public entry points ─────────────────────────────────────────────────

/// Translate a single expression (used by match_expr, control_flow, macros modules)
pub fn translate_expr(expr: &syn::Expr) -> String {
    BodyTranslator::new("Self").expr(expr)
}

/// Translate a pattern (used by match_expr, control_flow modules)
pub fn translate_pat(pat: &syn::Pat) -> String {
    BodyTranslator::pat_static(pat)
}

/// Indent each line by 2 spaces
pub fn indent(s: &str) -> String {
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + if s.ends_with('\n') { "\n" } else { "" }
}

/// Where an expression sits in the file, as the two ends of its span. Two
/// expressions are the same one when they start and end in the same place,
/// which is what an expectation is matched against.
type Position = ((usize, usize), (usize, usize));

pub(crate) fn position_of(expr: &syn::Expr) -> Position {
    span_position(syn::spanned::Spanned::span(expr))
}

pub(crate) fn span_position(span: proc_macro2::Span) -> Position {
    let (start, end) = (span.start(), span.end());
    ((start.line, start.column), (end.line, end.column))
}

// ── Translator struct ───────────────────────────────────────────────────

pub struct BodyTranslator<'a> {
    pub self_type: &'a str,
    /// Type context for comprehensive expression type resolution.
    /// Wraps ScopeStack + TypeRegistry. None for legacy codepaths
    /// (free function shims, match_expr, control_flow).
    pub types: Option<std::cell::RefCell<crate::infer::TypeContext<'a>>>,
    /// Inline module names for the current file — path qualifiers
    /// stripped during path resolution (symbols imported from separate .ts file).
    pub inline_module_names: Vec<String>,
    /// Names the enclosing block-as-expression already supplies as arrow
    /// function parameters. The `let` that would have introduced each of them
    /// has nothing left to do, and re-declaring one would shadow the value that
    /// was threaded in.
    threaded: std::cell::RefCell<Vec<String>>,
    /// How many temporaries this body has taken, so that one `if let` nested in
    /// another does not read both scrutinees into the same name.
    temporaries: std::cell::Cell<usize>,
    /// What the enclosing function returns, resolved. `?` reads it to say
    /// whether the error it propagates needs a conversion, and the `Result`
    /// lowering reads it to know it is inside a function that returns one.
    pub fn_return: Option<crate::ty::Ty>,
    /// Whether this method took `self` by value. A `&self` method lends the
    /// receiver, so nothing inside it can move a field out of `self` or match
    /// it by value — Rust would refuse both, and emitting them left the caller
    /// holding a value the body had marked moved.
    pub owns_self: bool,
    /// What this body still owes: the values in scope, the declarations lifted
    /// out of the statement being written, and the drop flags. Every decision
    /// that reads or writes it lives in `ownership::lowering`.
    pub own: crate::ownership::Lowering,
    /// What the position an expression stands in says its type has to be, and
    /// which expression that position holds.
    ///
    /// Rust settles a great deal from context that the expression itself does
    /// not carry — the width of a literal, the target of an `.into()`, a
    /// closure's parameter types — so the position hands the expression a type
    /// to be. The span is part of the slot because a sub-expression is often
    /// translated before its parent, and it must not take an answer meant for
    /// the parent.
    expecting: std::cell::RefCell<Option<(Position, crate::ty::Ty)>>,
    /// Is this body a formatter's — `Display::fmt` or `Debug::fmt`, whose
    /// `write!(f, ..)` calls compose one string?
    ///
    /// Rust's `fmt` writes into the formatter it was handed and answers
    /// `Ok(())`; TypeScript's `toString()` answers the string. So the body
    /// opens an accumulator, every `write!` appends to it, and `Ok(())` — the
    /// value Rust's `fmt` ends with — IS the accumulator.
    pub formatter: bool,
    /// The method call whose value the statement being written throws away,
    /// as the position of its method name.
    ///
    /// Rust's `HashMap::insert` answers the value it displaced and hands
    /// ownership of it to the caller; a statement that discards the answer
    /// leaves the container to release it, which is a different runtime method.
    /// The decision is the statement's, and the call is written well below it.
    discarded_call: std::cell::Cell<Option<(usize, usize)>>,
    /// While an arm of a CONSUMING match is being written, the jump it
    /// performs is handed back to the caller instead of being written where it
    /// stands: an arm of `intoMatch` is a function, and `break` cannot leave
    /// one — `return break` does not even parse. The caller reads the sentinel
    /// and performs the jump itself.
    pub(crate) jump_as_value: std::cell::Cell<bool>,
    /// Whether anything in this body actually named the accumulator. A `Debug`
    /// written with `f.debug_struct(..)` and a `Display` that hands the writing
    /// to a closure both write nothing of their own, and declaring an
    /// accumulator nothing appends to is a line that says something untrue.
    wrote_result: std::cell::Cell<bool>,
    /// The identifier Rust's `self` is emitted as in this body.
    ///
    /// A method on an emitted class writes `this`. A method whose impl is
    /// written for a type that has no class of its own — a blanket impl, or an
    /// impl on `Arc<Inner<T>>` — is emitted as a module-level function that
    /// takes its receiver as an ordinary first parameter, and there `self` is
    /// that parameter's name.
    pub self_name: &'a str,
}

impl<'a> BodyTranslator<'a> {
    pub fn new(self_type: &'a str) -> Self {
        Self {
            self_type,
            types: None,
            inline_module_names: vec![],
            threaded: std::cell::RefCell::new(Vec::new()),
            temporaries: std::cell::Cell::new(0),
            fn_return: None,
            owns_self: false,
            own: Default::default(),
            expecting: std::cell::RefCell::new(None),
            formatter: false,
            discarded_call: std::cell::Cell::new(None),
            jump_as_value: std::cell::Cell::new(false),
            wrote_result: std::cell::Cell::new(false),
            self_name: "this",
        }
    }

    pub fn with_context(self_type: &'a str, tc: crate::infer::TypeContext<'a>) -> Self {
        Self {
            self_type,
            types: Some(std::cell::RefCell::new(tc)),
            inline_module_names: vec![],
            threaded: std::cell::RefCell::new(Vec::new()),
            temporaries: std::cell::Cell::new(0),
            fn_return: None,
            owns_self: false,
            own: Default::default(),
            expecting: std::cell::RefCell::new(None),
            formatter: false,
            discarded_call: std::cell::Cell::new(None),
            jump_as_value: std::cell::Cell::new(false),
            wrote_result: std::cell::Cell::new(false),
            self_name: "this",
        }
    }

    // ── Matches: what the arms do to the subject ────────────────────





    // ── Expression translation ──────────────────────────────────────

    pub fn expr(&self, expr: &syn::Expr) -> String {
        // A formatter's `Ok(())` is the string it has written.
        if self.is_formatter_done(expr) {
            self.wrote_result.set(true);
            return "_result".to_string();
        }
        let expected = self.expectation_for(expr);
        match expr {
            // A 64-bit integer is a `bigint`, and JavaScript will not mix one
            // with a `number`: `1n + 1` throws rather than adding. So a literal
            // the engine typed as one carries the suffix that makes it a
            // `bigint` too.
            syn::Expr::Lit(lit) => {
                let Some(written) = translate_lit(&lit.lit) else {
                    self.fallback(
                        syn::spanned::Spanned::span(lit),
                        "this literal form has no spelling in the port, so the expression is \
                         written as `undefined`",
                    );
                    return "undefined".to_string();
                };
                match self.is_bigint_literal(&lit.lit, expected.as_ref()) {
                    true => format!("{}n", written),
                    false => written,
                }
            }
            syn::Expr::Path(path) => self.path_expr(&path.path),

            syn::Expr::Field(field) => {
                let (receiver, member) = self.field_parts(field);
                format!("{}.{}", receiver, member)
            }

            syn::Expr::MethodCall(call) => {
                // `unwrap` and its kind take a wrapper apart, so what the
                // position wants of the whole is what it wants of the payload:
                // `assert_eq!(id, from_str(&s).unwrap())` says the parse
                // produces an `EntityId`, and nothing else does.
                let through = self.receiver_expectation(call, expected.as_ref());
                let receiver = self.expecting(&call.receiver, through.as_ref(), || {
                    self.expr(&call.receiver)
                });
                let receiver = self.hoist_receiver(call, receiver);
                let receiver = parenthesise_literal(&call.receiver, receiver);
                let rust_method = call.method.to_string();
                let ts_method = name_map::map_fn_name(&rust_method);

                // The callee's signature is what says what each argument has to
                // be: a closure takes its parameter types from there, an
                // `.into()` its target, and a literal its width.
                let want = self.argument_types(call);
                let args: Vec<String> = call
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, a)| {
                        self.expecting(a, want.get(index).and_then(|t| t.as_ref()), || {
                            self.moved_value(a)
                        })
                    })
                    .collect();

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
                        // written, which is nothing at all.
                        self.record_resolution(call, &rust_method);
                        return receiver.to_string();
                    }
                }
                // `??` reads a *value* for null, and a `Result` is an object:
                // `r ?? d` always takes `r`, whatever it holds. Only the
                // nullable the port maps `Option` to can be written that way;
                // a `Result` calls the runtime's own method, which consumes the
                // receiver as Rust's does.
                if matches!(rust_method.as_str(), "unwrap_or" | "unwrap_or_else") && args.len() == 1
                {
                    let nullable = self
                        .resolve_expr_type(&call.receiver)
                        .ok()
                        .is_some_and(|ty| self.is_nullable(&ty));
                    if nullable {
                        if rust_method == "unwrap_or" {
                            // Rust evaluates the default before it knows whether
                            // it is wanted, and drops it where it is not. `??`
                            // alone left it to the leak registry.
                            if let Some(chosen) =
                                self.nullable_default(&receiver, &call.args[0], &args[0])
                            {
                                return chosen;
                            }
                        }
                        return match rust_method.as_str() {
                            "unwrap_or" => format!("{} ?? {}", receiver, args[0]),
                            _ => format!("{} ?? ({})()", receiver, args[0]),
                        };
                    }
                }

                // ── a conversion ──
                // `into`, `try_into`, `to_string` and `to_owned` name a
                // conversion rather than a method the receiver's type carries,
                // and what the port writes for one is decided by the pair of
                // types, not by the name.
                if let Some(converted) =
                    self.conversion_method(call, &receiver, expected.as_ref())
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
                        let mut recv = receiver.clone();
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
                        // An impl written for a type with no emitted class is a
                        // module-level function, and the receiver is its first
                        // argument.
                        if let Some(free) = crate::emit_impls::free_call(tc_ref.registry, &found) {
                            drop(tc_ref);
                            return self.render_free_call(&free, &recv, &args, call);
                        }
                        let translated = native_types::translate_method_using(
                            tc_ref.registry,
                            found.receiver_type(),
                            &recv,
                            &rust_method,
                            &args,
                            !self.discards(call),
                        );
                        drop(tc_ref);
                        return self.render_translation(
                            translated,
                            &recv,
                            &ts_method,
                            &args,
                            syn::spanned::Spanned::span(call),
                        );
                    }
                }

                self.translate_unresolved_call_using(
                    &receiver,
                    &rust_method,
                    &ts_method,
                    &args,
                    Some(&call.receiver),
                    !self.discards(call),
                )
            }

            syn::Expr::Call(call) => {
                // `drop(x)` runs x's glue where the source says, and what that
                // costs is the glue engine's answer, not always `.drop()`: a
                // `Vec<Owned>` is a JavaScript array and has no method of its
                // own.
                if let Some(released) = self.explicit_drop(call) {
                    return released;
                }
                // `(move || …)()` is created, called and dropped in the one
                // expression, so what it captured is released inside it.
                if let Some(closure) = as_closure(&call.func) {
                    let arrow =
                        self.closure(closure, ownership::closures::Placement::Immediate, None);
                    let args: Vec<String> =
                        call.args.iter().map(|a| self.moved_value(a)).collect();
                    return format!("({})({})", arrow, args.join(", "));
                }
                let func = self.expr(&call.func);
                // What the callee declares each argument to be, with the
                // position the call stands in used to close whatever the
                // signature left open. This is what types the closure in
                // `Box::new(move |level| ..)`, whose parameter the signature
                // alone says nothing about.
                let want = match &self.types {
                    Some(tc) => {
                        self.quietly(|| tc.borrow().call_argument_types(call, expected.as_ref()))
                    }
                    None => Vec::new(),
                };
                let args: Vec<String> = call
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, a)| {
                        self.expecting(a, want.get(index).and_then(|t| t.as_ref()), || {
                            self.moved_value(a)
                        })
                    })
                    .collect();
                // An `OwnedClosure` is deliberately not a bare callable, so a
                // call that reached the body without a liveness check would be
                // exactly the bug it exists to catch.
                if self.own.owned_closure_locals.borrow().iter().any(|n| *n == func) {
                    // A closure whose body hands a capture away is an `FnOnce`,
                    // and `callOnce` is what transfers the captures and marks
                    // it moved. `call` left them the closure's, so they were
                    // released twice — once by the body and once by the
                    // closure's own drop.
                    let once = self.own.once_closure_locals.borrow().iter().any(|n| *n == func);
                    let method = if once { "callOnce" } else { "call" };
                    return format!("{}.{}({})", func, method, args.join(", "));
                }
                let path = match &*call.func {
                    syn::Expr::Path(path) => Some(&path.path),
                    _ => None,
                };
                // `Target::from(x)` and `Target::try_from(x)` name the
                // conversion the impl table answers, and the function they land
                // on is the one emission gave that impl: `Wrapped.fromWire`,
                // not a `from` the class does not declare.
                if let Some(written) = self.conversion_call(path, call, &args) {
                    return written;
                }
                self.translate_call(&func, &args, syn::spanned::Spanned::span(call), path)
            }

            // `*place += value` reads and writes through the wrapper, and the
            // `*place` half is written by the deref arm below like any other
            // deref — which is what names the temporary it produced and
            // releases it. Writing it here instead released nothing.
            syn::Expr::Binary(bin) => {
                // `*place += value` stores through the wrapper, so the target is
                // written as a place; everything else about it is an ordinary
                // binary expression, and the deref arm below hoists whatever
                // the target produced either way.
                if is_assign_op(&bin.op) {
                    if let syn::Expr::Unary(unary) = &*bin.left {
                        if matches!(unary.op, syn::UnOp::Deref(_)) {
                            let place = self.deref_place(unary);
                            let op = translate_binop(&bin.op);
                            return format!("{} {} {}", place, op, self.expr(&bin.right));
                        }
                    }
                }
                let left = self.expecting(&bin.left, shift_expectation(&bin.op, expected.as_ref()), || {
                    self.expr(&bin.left)
                });
                // `&&` and `||` evaluate their right operand only if the left
                // one allows it, so anything that operand took to evaluate
                // itself belongs inside the branch the short circuit guards.
                if matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
                    let (right, lifted) = self.with_own_hoists(|| self.expr(&bin.right));
                    let right = self.short_circuit_operand(&bin.right, right, lifted);
                    return format!("{} {} {}", left, translate_binop(&bin.op), right);
                }
                // The other operand of a comparison is what says how wide an
                // unsuffixed literal is, and which type a bare `into` on the
                // right of an `==` converts to.
                //
                // A shift is the exception. What the position wants of the
                // shift is what it wants of the value being shifted, and the
                // shift *amount* — a `u32` in Rust — has to be a `bigint` in
                // JavaScript whenever the value is, because `1n << 63` throws.
                // So the amount is asked for under the same type: a written
                // literal comes out as a `bigint`, and an amount that is a
                // value of its own keeps its type and is converted at the
                // operator instead.
                let shift = matches!(bin.op, syn::BinOp::Shl(_) | syn::BinOp::Shr(_));
                let want = if shift {
                    expected.clone()
                } else {
                    self.quietly(|| self.resolve_expr_type(&bin.left)).ok()
                };
                let right =
                    self.expecting(&bin.right, want.as_ref(), || self.expr(&bin.right));
                // What the operator resolves to: the impl's method where the
                // operands are not primitives, and the JavaScript operator with
                // whatever correction its arithmetic needs where they are.
                if let Some(resolved) = self.binary_operator(bin, &left, &right) {
                    return resolved;
                }
                format!("{} {} {}", left, translate_binop(&bin.op), right)
            }

            syn::Expr::Unary(unary) => {
                let e = self.expr(&unary.expr);
                match &unary.op {
                    syn::UnOp::Not(_) => {
                        // Rust's `!` is the bitwise complement on an integer
                        // and the logical negation on a `bool`; JavaScript
                        // spells the first one `~`.
                        // The impl table first: a type with an `impl Not` has
                        // a method to call, and `unary_not`'s own report is
                        // for the case where nothing performs it.
                        if let Some(call) = self.unary_not_impl(unary, &e) {
                            return call;
                        }
                        if let Some(complement) = self.unary_not(unary, &e) {
                            return complement;
                        }
                        if e.contains("===") || e.contains("!==") || e.contains(">=") || e.contains("<=") {
                            format!("!({})", e)
                        } else {
                            format!("!{}", e)
                        }
                    }
                    // `-a` on anything but a number is `Neg::neg`, and `-object`
                    // is `NaN`.
                    syn::UnOp::Neg(_) => match self.unary_neg(unary, &e) {
                        Some(call) => call,
                        None => format!("-{}", e),
                    },
                    // `*guard` reads what the container holds, which the port
                    // writes as a field on the guard. Emitting the guard itself
                    // handed the guard where the value was wanted, and a guard
                    // the statement produced was never released — the lock
                    // stayed held for the life of the program.
                    syn::UnOp::Deref(_) => {
                        let written = self.hoist_produced(&unary.expr, e);
                        match &self.types {
                            Some(tc) => {
                                let accessor = tc.borrow().deref_accessor_of(&unary.expr);
                                match accessor {
                                    Ok(accessor) => format!("{}.{}", written, accessor),
                                    // Not every `*` reaches through a wrapper:
                                    // `*x` on a `&T` is the `T`, and emission
                                    // erases the reference.
                                    Err(_) => written,
                                }
                            }
                            None => written,
                        }
                    }
                    _ => format!("/* unknown unary op */ {}", e),
                }
            }

            syn::Expr::If(if_expr) => control_flow::translate_if(if_expr, self),

            // A block hands its value on from its tail, so what the position
            // wants of the block it wants of the tail — re-keyed onto the tail,
            // because an expectation is matched by the span of the expression
            // it was written for.
            syn::Expr::Block(block) => {
                let tail = block.block.stmts.last().and_then(|stmt| match stmt {
                    syn::Stmt::Expr(tail, None) => Some(tail),
                    _ => None,
                });
                if let Some(tail) = tail {
                    if block.block.stmts.len() == 1 {
                        return self.expecting(tail, expected.as_ref(), || self.expr(tail));
                    }
                    if expected.is_some() {
                        return self.expecting(tail, expected.as_ref(), || {
                            self.block_as_value(block)
                        });
                    }
                }
                self.block_as_value(block)
            }

            syn::Expr::Return(ret) => {
                if let Some(expr) = &ret.expr {
                    // What is returned leaves through the function's return
                    // type, which is what says what it has to be.
                    let want = self.fn_return.clone();
                    let value =
                        self.expecting(expr, want.as_ref(), || self.moved_value(expr));
                    format!("return {}", value)
                } else {
                    "return".to_string()
                }
            }

            syn::Expr::Match(me) => match_expr::translate_match(me, self),

            syn::Expr::Closure(closure) => self.closure(
                closure,
                ownership::closures::Placement::Loose,
                expected.as_ref(),
            ),

            syn::Expr::ForLoop(for_loop) => self.for_loop(for_loop),

            syn::Expr::While(while_loop) => {
                // `while let PAT = e` is a loop that re-evaluates `e` each turn,
                // tests it against the pattern and stops when it does not match.
                // Emitting the condition as an expression produced a comment
                // where the test should be and left the binding undeclared.
                let label = ownership::iteration::label_of(&while_loop.label);
                if let syn::Expr::Let(let_expr) = &*while_loop.cond {
                    return self.while_let(let_expr, &while_loop.body, &label);
                }
                // Rust evaluates the condition afresh each turn and drops
                // what it produced before the body runs. Where the condition
                // lifted anything, the test moves inside the loop: leaving it
                // in the `while` header evaluated it once and held whatever it
                // took for the life of the loop.
                let (cond, lifted) = self.with_own_hoists(|| self.expr(&while_loop.cond));
                let body = self.translate_block(&while_loop.body);
                if lifted.is_empty() {
                    return format!("{}while ({}) {{\n{}}}", label, cond, indent(&body));
                }
                let held = self.fresh_hoist("_c");
                let test = ownership::hoisted(&format!("{} = {};\n", held, cond), &lifted);
                format!(
                    "{}for (;;) {{\n  let {};\n{}  if (!{}) break;\n{}}}",
                    label,
                    held,
                    indent(&test),
                    held,
                    indent(&body)
                )
            }

            syn::Expr::Loop(loop_expr) => {
                let body = self.translate_block(&loop_expr.body);
                format!(
                    "{}while (true) {{\n{}}}",
                    ownership::iteration::label_of(&loop_expr.label),
                    indent(&body)
                )
            }

            // `break 'outer` names the loop it leaves, and so does
            // `continue 'outer`. A bare `break` leaves the innermost loop, which
            // is a different program wherever the source named an outer one.
            syn::Expr::Break(brk) => {
                if self.jump_as_value.get() {
                    return jump_sentinel("break", &brk.label);
                }
                let target = ownership::iteration::target_of(&brk.label);
                if let Some(expr) = &brk.expr {
                    format!("break{} /* {} */", target, self.expr(expr))
                } else {
                    format!("break{}", target)
                }
            }

            syn::Expr::Continue(cont) => {
                if self.jump_as_value.get() {
                    return jump_sentinel("continue", &cont.label);
                }
                format!("continue{}", ownership::iteration::target_of(&cont.label))
            }

            // A `*place = value` goes through the same door as every other
            // assignment. Writing it here instead named no temporary and
            // released nothing: `*cell.lock().unwrap() = v` left the mutex
            // locked for the life of the program, and the value the place
            // already held was abandoned where Rust drops it.
            syn::Expr::Assign(assign) => self.assign(assign),

            syn::Expr::Index(idx) => {
                // `v[a..b]` is a slice, not an index. Emitting the range as an
                // index expression produced `v[/* range a..b */]`, which does
                // not parse.
                if let syn::Expr::Range(range) = &*idx.index {
                    let from = range
                        .start
                        .as_ref()
                        .map(|e| self.expr(e))
                        .unwrap_or_else(|| "0".to_string());
                    let to = range.end.as_ref().map(|e| self.expr(e));
                    let end = match (&range.limits, to) {
                        // `..=b` includes the last element.
                        (syn::RangeLimits::Closed(_), Some(to)) => format!(", {} + 1", to),
                        (_, Some(to)) => format!(", {}", to),
                        (_, None) => String::new(),
                    };
                    return format!("{}.slice({}{})", self.expr(&idx.expr), from, end);
                }
                // `[Owned::new()][0]` reads out of a sequence the expression
                // itself built, and that sequence is a temporary Rust drops at
                // the end of the statement.
                let base = self.expr(&idx.expr);
                let base = self.hoist_produced(&idx.expr, base);
                let index = self.expr(&idx.index);
                if let Some(call) = self.index_through_impl(&idx.expr, &base, &index) {
                    return call;
                }
                format!("{}[{}]", base, index)
            }

            // `look(&Owned { .. })` builds a value the callee only reads, and
            // Rust drops it at the end of the statement. Emitting the
            // expression in place left nothing holding it and nothing releasing
            // it.
            syn::Expr::Reference(reference) => {
                let written = self.expr(&reference.expr);
                self.hoist_produced(&reference.expr, written)
            }

            // Parentheses group and say nothing else, so whatever the position
            // wants of the whole it wants of what is inside.
            syn::Expr::Paren(paren) => format!(
                "({})",
                self.expecting(&paren.expr, expected.as_ref(), || self.expr(&paren.expr))
            ),

            syn::Expr::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(|e| self.moved_value(e)).collect();
                format!("[{}]", parts.join(", "))
            }

            syn::Expr::Array(arr) => {
                let items: Vec<String> = arr.elems.iter().map(|e| self.moved_value(e)).collect();
                self.sequence_literal(items, expected.as_ref())
            }

            syn::Expr::Struct(s) => {
                // `Predicate::Comparison { left, operator, right }` builds an
                // enum VARIANT, not a struct, and the port builds one the way
                // every other construction of that enum does. Writing it as a
                // constructor produced `new Predicate.Comparison(a, b, c)`,
                // where `Predicate.Comparison` is not a constructor and the
                // field names were thrown away with the braces.
                if let Some(built) = self.struct_variant_literal(s) {
                    return built;
                }
                let mut name = Self::path_static(&s.path);
                if name == "Self" { name = self.self_type.to_string(); }
                // A field's declared type is what its initialiser has to be, so
                // `Header { len: 1 }` writes the width the field declares.
                let want = self.struct_field_types(s);
                let values: Vec<String> = s
                    .fields
                    .iter()
                    .map(|f| {
                        let member = crate::infer::member_name(&f.member);
                        let field = want.iter().find(|(name, _)| *name == member).map(|(_, t)| t);
                        self.expecting(&f.expr, field, || self.moved_value(&f.expr))
                    })
                    .collect();
                format!("new {}({})", name, values.join(", "))
            }

            syn::Expr::Try(try_expr) => {
                // Special case: write!(f, ...)? in expression position — just the format string
                if is_write_macro(&try_expr.expr) {
                    let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap(), self);
                    return fmt_str;
                }
                // What the position wants of the `?` is what it wants of the
                // payload, so the operand is asked for a `Result` of it:
                // `let id: EntityId = s.parse()?` is the only thing that says
                // which type the parse produces.
                let want = self.try_operand_expectation(expected.as_ref());
                let lowered =
                    self.expecting(&try_expr.expr, want.as_ref(), || self.lower_try(try_expr));
                self.own.prelude.borrow_mut().push(ownership::Hoist {
                    declaration: lowered.declaration,
                    owned: None,
                });
                lowered.value
            }
            syn::Expr::Await(await_expr) => format!("await {}", self.expr(&await_expr.base)),

            // A range is a value in Rust — `(0..n).rev()` calls a method on one
            // — and the port has no type for it. It used to be written as a
            // comment, which is not an expression: `(/* range 0..n */).rev()`
            // does not parse, and one of those stopped the compiler from
            // checking the rest of the file it was in. `undefined` parses and
            // is wrong, which is what the diagnostic says.
            syn::Expr::Range(range) => {
                let from = range.start.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                let to = range.end.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                self.fallback(
                    syn::spanned::Spanned::span(range),
                    format!(
                        "the range types in `std::ops` are not declared, so `{}..{}` is \
                         written as `undefined`",
                        from, to
                    ),
                );
                format!("undefined /* range {}..{} */", from, to)
            }

            syn::Expr::Cast(cast) => self.cast(cast),

            // A macro's own span is what its translation asks the expectation
            // by, so the type the position wants is re-keyed onto it: `vec![1,
            // 2]` behind a `Vec<u8>` writes bytes, not a JavaScript array.
            syn::Expr::Macro(mac) => {
                let held = self.expecting.replace(
                    expected
                        .as_ref()
                        .map(|ty| (span_position(syn::spanned::Spanned::span(&mac.mac)), ty.clone())),
                );
                let written = macros::translate_macro(&mac.mac, self);
                *self.expecting.borrow_mut() = held;
                written
            }

            // `unsafe { expr }` is its expression; the keyword says only who is
            // answerable for it. Translating the block as *statements* wrote a
            // `return` into an expression position — `const item = /* unsafe */
            // return data[i].assumeInitRef();;` — which does not parse.
            syn::Expr::Unsafe(unsafe_block) => {
                let written = match single_block_expr(&unsafe_block.block) {
                    Some(value) => self.expr_value(value),
                    None => {
                        self.fallback(
                            syn::spanned::Spanned::span(unsafe_block),
                            "this `unsafe` block is a run of statements rather than one                              expression, and the port has nowhere to put them where its value                              is wanted; a `.provided.ts` is what such a block needs",
                        );
                        format!(
                            "(() => {{
{}}})()",
                            indent(&self.translate_block(&unsafe_block.block))
                        )
                    }
                };
                format!("/* unsafe — consider provided impl */ {}", written)
            }

            syn::Expr::Async(async_block) => {
                let body = self.translate_block(&async_block.block);
                format!("(async () => {{\n{}}})()", indent(&body))
            }

            syn::Expr::Let(let_expr) => {
                let pat = Self::pat_static(&let_expr.pat);
                let expr = self.expr(&let_expr.expr);
                format!("/* let {} = {} */", pat, expr)
            }

            syn::Expr::Repeat(repeat) => {
                format!("Array({}).fill({})", self.expr(&repeat.len), self.expr(&repeat.expr))
            }

            syn::Expr::Group(group) => {
                self.expecting(&group.expr, expected.as_ref(), || self.expr(&group.expr))
            }

            // Every remaining form — a `Verbatim` `syn` did not parse among
            // them — is one the translator has no writing for, so the site says
            // so rather than leaving a comment where an expression stood.
            other => {
                self.fallback(
                    syn::spanned::Spanned::span(other),
                    format!(
                        "a {} is a form the translator has no writing for, so the expression \
                         is written as `undefined`",
                        crate::infer::expr_form(other)
                    ),
                );
                "undefined".to_string()
            }
        }
    }

    /// A multi-statement block standing where a value is wanted: an
    /// immediately-called arrow function, with the shadowing a Rust block
    /// allows and JavaScript does not threaded through as parameters.
    /// An expression whose value is wanted here and one of whose arms leaves
    /// the loop around it.
    ///
    /// The value is computed in a wrapper function, which a `break` cannot
    /// leave. So the arm that jumps hands the jump back instead, the statement
    /// before this one reads it and performs the jump, and what stands here is
    /// the value the other arms produced. `core/src/reactor/fetch_gap.ts` was
    /// one of the four emitted files a JavaScript engine refused to load.
    fn value_through_a_jump(&self, expr: &syn::Expr) -> String {
        let previous = self.jump_as_value.replace(true);
        let body = control_flow::translate_expr_in_return_position(expr, self);
        self.jump_as_value.set(previous);
        let awaits = control_flow::awaiting::awaits(expr);
        let held = self.hoist_name(iife("()", &format!("{}\n", body), "", awaits));
        let mut tests = String::new();
        for kind in crate::match_expr::jumps_out_of(expr) {
            let (word, label) = match kind.split_once('#') {
                Some((word, label)) => (
                    word.to_string(),
                    format!(" && ({} as any)?.$label === '{}'", held, label),
                ),
                None => (kind.clone(), String::new()),
            };
            let target = match kind.split_once('#') {
                Some((_, label)) => format!(" {}", label),
                None => String::new(),
            };
            tests.push_str(&format!(
                "if (({held} as any)?.$jump === '{word}'{label}) {word}{target};\n",
                held = held,
                word = word,
                label = label,
                target = target
            ));
        }
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: tests,
            owned: None,
        });
        // The value's type is the union of what the arms produced and the
        // sentinel, and the jump above is what rules the sentinel out.
        format!("({} as any)", held)
    }

    fn block_as_value(&self, block: &syn::ExprBlock) -> String {
        // Multi-statement block as expression → IIFE
        // Detect shadowed variables: if a local in the block has the same name
        // as a variable used in its init, thread it as an IIFE parameter
        let mut shadow_params: Vec<(String, String, Option<crate::ty::Ty>)> = Vec::new();
        for stmt in &block.block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                let pat_name = Self::pat_static(&local.pat);
                if let Some(init) = &local.init {
                    // Check if the init expression references pat_name as a
                    // standalone variable (not as a field name in a.b.c)
                    if references_var(&init.expr, &pat_name) {
                        // This is a shadow pattern — pass as IIFE param.
                        // The parameter holds the value of the initialiser,
                        // resolved out here, before the block's scope exists.
                        let resolved = self.resolve_expr_type(&init.expr);
                        let instead =
                            format!("IIFE parameter `{}` is left untyped", pat_name);
                        let ty = self.or_fallback(resolved, &instead);
                        let init_ts = self.expr(&init.expr);
                        shadow_params.push((pat_name, init_ts, ty));
                    }
                }
            }
        }
        if !shadow_params.is_empty() {
            // Thread shadowed variables as IIFE parameters.
            // Push shadow names into scope so local() skips their declarations
            // (they're already bound as IIFE params).
            self.push_block();
            for (name, _, ty) in &shadow_params {
                match ty {
                    Some(ty) => self.bind_var(name, ty.clone()),
                    None => self.bind_untyped(name),
                }
                self.threaded.borrow_mut().push(name.clone());
            }
            let body = self.translate_block_stmts(&block.block);
            for _ in &shadow_params {
                self.threaded.borrow_mut().pop();
            }
            self.pop_scope();
            let params: Vec<&str> =
                shadow_params.iter().map(|(n, _, _)| n.as_str()).collect();
            let args: Vec<&str> = shadow_params.iter().map(|(_, v, _)| v.as_str()).collect();
            iife(
                &format!("({})", params.join(", ")),
                &body,
                &args.join(", "),
                crate::control_flow::awaiting::block_awaits(&block.block),
            )
        } else {
            let body = self.translate_block(&block.block);
            iife(
                "()",
                &body,
                "",
                crate::control_flow::awaiting::block_awaits(&block.block),
            )
        }
    }

    /// An expression whose *value* the surrounding code needs.
    ///
    /// An `if` is a value in Rust and a statement in TypeScript, and the two
    /// need different code: as a statement it is an `if`, as a value it is a
    /// ternary where both branches are expressions and an immediately-called
    /// arrow function where they are not. Emitting the statement form in value
    /// position wrote a block where an expression had to stand, which does not
    /// parse.
    pub fn expr_value(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::If(if_expr) => {
                if let Some(ternary) = self.try_ternary(if_expr) {
                    return ternary;
                }
            }
            // A `match` is a value in Rust too. Where the port writes one as
            // the runtime's `match`, that is already an expression; where it
            // writes an `if`/`else` chain — an `Option` or a `Result` match —
            // the statements have to stand inside an arrow function, or
            // `const x = if (..) {` is what comes out, which does not parse.
            syn::Expr::Match(_) => {
                let written = self.expr(expr);
                if !match_expr::is_statements(&written) {
                    return written;
                }
            }
            _ => return self.expr(expr),
        }
        // The wrapper is a function, and `break` and `continue` cannot leave
        // one: `Cannot use "continue" here` is what a JavaScript engine says,
        // and the whole module then fails to load. The jump is handed back as
        // a value instead, and the statement that wanted the value performs it
        // before reading one.
        if crate::match_expr::jumps_out_of_a_loop(expr) {
            return self.value_through_a_jump(expr);
        }
        let body = control_flow::translate_expr_in_return_position(expr, self);
        iife(
            "()",
            &format!("{}\n", body),
            "",
            crate::control_flow::awaiting::awaits(expr),
        )
    }

    /// A name for a value the emitted code has to hold on to.
    ///
    /// A pattern match tests its subject and then takes it apart, and the
    /// subject has to be the *same* value both times: `if let Some(x) =
    /// c.step().await?` that writes the call twice calls it twice.
    pub fn fresh_temp(&self) -> String {
        let n = self.temporaries.get();
        self.temporaries.set(n + 1);
        if n == 0 {
            "_v".to_string()
        } else {
            format!("_v{}", n)
        }
    }


    /// An expression in a position that runs it rather than reading its value.
    ///
    /// A block runs as statements here rather than as an immediately-called
    /// arrow function, so a `break` written inside it still leaves the loop it
    /// was written in.
    pub fn statements(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Block(block) => self.translate_block(&block.block),
            _ => self.expr(expr),
        }
    }













    /// `while let PAT = e { body }` as a loop that tests each turn.
    ///
    /// The scrutinee is read once per turn into a temporary, tested against the
    /// pattern, and its payload bound inside the body — which is what Rust does
    /// and what the previous emission, a comment in the condition, did not.
    pub(crate) fn while_let(
        &self,
        let_expr: &syn::ExprLet,
        body: &syn::Block,
        label: &str,
    ) -> String {
        // The scrutinee is read afresh every turn, in value position: it is the
        // turn's own value, and an `if` written there is a run of statements
        // that `const _v = …` cannot hold. Whatever it lifted belongs inside
        // the loop with it, because it is taken again on the next turn.
        let (scrutinee, lifted) = self.with_own_hoists(|| self.expr_value(&let_expr.expr));
        let ty = self.scrutinee_type(&let_expr.expr);
        let _bindings = self.enter_pattern(&let_expr.pat, ty.as_ref());
        // The pattern binds afresh each turn, and Rust drops what it bound at
        // the end of that turn — so the release goes inside the loop, not after
        // it.
        let owned = self.claim_bindings(&bound_names(&let_expr.pat), &body.stmts);
        let translated = self.translate_block(body);
        drop(_bindings);

        let subject = self.fresh_temp();
        let (test, bind) = self.pattern_test(&subject, &let_expr.pat);
        let turn = self.wrap_bindings(&owned, translated);
        let leaving = self.abandoned_scrutinee(&let_expr.expr, &let_expr.pat, &subject);
        let read = ownership::hoisted(&format!("const {} = {};\n", subject, scrutinee), &lifted);
        format!(
            "{}for (;;) {{\n{}  if (!({})) {{\n{}    break;\n  }}\n{}{}}}",
            label,
            indent(&read),
            test,
            indent(&indent(&leaving)),
            indent(&bind),
            indent(&turn)
        )
    }

    /// What the turn owes for a scrutinee whose pattern did not match.
    ///
    /// Rust drops the value the turn read when no pattern took it apart, so the
    /// path that leaves the loop releases it. The path that *did* match is a
    /// different question: where the pattern took an owned payload out, that
    /// payload belongs to the binding from there and the enum it came out of
    /// has to be marked moved — which only `intoMatch` does, and an arrow
    /// function is not something a `break` can leave. That one is reported.
    fn abandoned_scrutinee(&self, expr: &syn::Expr, pat: &syn::Pat, subject: &str) -> String {
        // A nullable scrutinee is its own payload: `Option<T>` is `T | null`
        // here, so `Some(v)` binds the very value the turn read and the turn
        // that did not match read a `null`, which owns nothing. There is no
        // wrapper left over on either path.
        if self
            .quietly(|| self.resolve_expr_type(expr))
            .is_ok_and(|ty| self.is_nullable(&ty))
        {
            return String::new();
        }
        let Some(release) = self.release_of(expr, subject) else {
            return String::new();
        };
        if self.pattern_takes_a_payload(expr, pat) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "this `while let` takes an owned payload out of the value it read, and the \
                 value it came out of is not marked moved, so nothing releases the rest of it",
            );
        }
        format!("{}\n", release)
    }

    /// Does this pattern take a payload out of the scrutinee that the arm then
    /// owns, rather than binding the whole value?
    fn pattern_takes_a_payload(&self, expr: &syn::Expr, pat: &syn::Pat) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(subject) = self.quietly(|| self.resolve_expr_type(expr)) else {
            return false;
        };
        let tc = tc.borrow();
        let takes = ownership::scrutinee::takes(&tc.probe(), &subject, &[pat], |path| {
            let mark = tc.sink.mark();
            let payload = tc.payload_of(path, Some(&subject));
            tc.sink.rewind(mark);
            payload.unwrap_or_default().into_iter().map(|(_, ty)| ty).collect()
        });
        takes == ownership::scrutinee::Takes::Payload
    }

    /// `*place`, written as the place an assignment stores into.
    ///
    /// A `*` in a value position may reach through nothing at all — `*x` on a
    /// `&T` is the `T`, and emission erases the reference — so a deref the
    /// engine could not resolve is written as the value itself. An assignment
    /// target cannot be: `*guard = v` and `*guard += 1` store *through* the
    /// wrapper whatever the engine could say about it, and dropping the
    /// accessor there emitted `counter.lock() += 1`, which names no place at
    /// all. So the target keeps `.value` as its default, and says that it
    /// assumed it.
    pub(crate) fn deref_place(&self, unary: &syn::ExprUnary) -> String {
        let inner = self.expr(&unary.expr);
        let inner = self.hoist_produced(&unary.expr, inner);
        let Some(tc) = &self.types else {
            self.fallback(syn::spanned::Spanned::span(&*unary.expr), ASSUMED_ACCESSOR);
            return format!("{}.value", inner);
        };
        let accessor = tc.borrow().deref_accessor_of(&unary.expr);
        match self.or_fallback(accessor, ASSUMED_ACCESSOR) {
            Some(accessor) => format!("{}.{}", inner, accessor),
            None => format!("{}.value", inner),
        }
    }


    /// Try to translate an if/else as a ternary expression.
    /// Returns Some(ternary) if both branches are single expressions.
    pub(crate) fn try_ternary(&self, if_expr: &syn::ExprIf) -> Option<String> {
        // Must not be if-let
        if matches!(&*if_expr.cond, syn::Expr::Let(_)) { return None; }
        // Must have an else branch
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        // Then branch must be a single expression
        let then_val = single_block_expr(&if_expr.then_branch)?;
        // Else branch must be a single expression (not another if)
        let else_val = match else_expr.as_ref() {
            syn::Expr::Block(block) => single_block_expr(&block.block)?,
            _ => return None,
        };
        // A branch that hands a flagged local away needs a statement to set the
        // flag in, and a ternary branch is not one. The `if` form is.
        if !self.flag_sets_for(then_val).is_empty() || !self.flag_sets_for(else_val).is_empty() {
            return None;
        }
        // The branches are written out to find out whether they fit; a branch
        // that lifts a declaration out of itself does not, because the
        // declaration would run whichever branch was taken. The attempt takes
        // its diagnostics back with it, so the abandoned form is not counted.
        let mark = self.mark();
        let cond = self.expr(&if_expr.cond);
        let (then_ts, then_lifted) = self.with_own_hoists(|| self.expr_value(then_val));
        let (else_ts, else_lifted) = self.with_own_hoists(|| self.expr_value(else_val));
        if !then_lifted.is_empty() || !else_lifted.is_empty() {
            self.rewind(mark);
            return None;
        }
        Some(format!("{} ? {} : {}", cond, then_ts, else_ts))
    }

    /// Where the diagnostics record stands, for a form the translator may
    /// abandon.
    pub(crate) fn mark(&self) -> usize {
        self.types.as_ref().map(|tc| tc.borrow().sink.mark()).unwrap_or(0)
    }

    pub(crate) fn rewind(&self, mark: usize) {
        if let Some(tc) = &self.types {
            tc.borrow().sink.rewind(mark);
        }
    }

}


/// What a call, a match or an operator takes away from the block that held it.
mod consumes;

/// A block, and the statements in it.
mod blocks;

/// What the translator knows while it writes: scopes, expectations, reports.
mod scopes;

/// A call the engine could not resolve, and the free functions an impl with no
/// class of its own became.
mod calls;

/// Reading a field, and the places a value moves out of.
mod places;

/// A path in expression position, and the values a path names.
mod paths;

/// The small pieces of TypeScript the translator writes by hand.
mod writing;
pub(crate) use writing::*;
/// What a pattern asks of a value, and what it takes out of it.
mod patterns;
/// What the pattern machinery writes, tested apart from the rest of the body.
#[cfg(test)]
mod pattern_tests;
