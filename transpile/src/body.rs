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

/// The write a formatter statement performs, and whether the statement LEAVES
/// the formatter having performed it.
///
/// `write!(f, "..")` appends and carries on; `return write!(f, "..")` appends
/// and then answers what the formatter has composed. The second was read as an
/// ordinary `return`, so the string it wrote became the whole answer and
/// everything written before it was discarded: `Display for Size` answered
/// `'big)'` where Rust answers `Size(big)`.
pub(crate) fn formatter_write(expr: &syn::Expr) -> Option<(&syn::Macro, bool)> {
    match expr {
        syn::Expr::Return(ret) => {
            let value = ret.expr.as_deref()?;
            as_write_macro(value).map(|mac| (mac, true))
        }
        _ => as_write_macro(expr).map(|mac| (mac, false)),
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
/// The text of an R12 hole: what an emitted file carries where the port has no
/// lowering for a Rust shape.
///
/// One spelling, in one place, so a hole is greppable in emitted output and the
/// harness can hold a ledger of them. `unsupported` answers `never`, so this
/// stands wherever the expression it replaces stood.
pub fn hole_text(what: &str) -> String {
    format!("unsupported({})", quoted(what))
}

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
    /// Names this body reads and writes through a runtime cell (C1).
    ///
    /// A `&mut usize` parameter is a place the callee WRITES and the caller
    /// reads back. JavaScript passes a number, a string and a boolean by value,
    /// so a plain parameter carries the write nowhere: ankql's SQL generator
    /// takes `buffer: &mut String` and `found_placeholders: &mut usize`, and
    /// every axis of `selection/sql.ts` answered the empty string because the
    /// buffer the callee filled was a copy. Such a parameter is a
    /// `BorrowMut<T>` here, and every read of the name is `name.value`.
    pub boxed: std::cell::RefCell<Vec<String>>,
    /// Locals this body hands out as `&mut`. Whether each really needs a cell
    /// is settled at its `let`, which is the only place its type is known: a
    /// `&mut` to a class is already a reference in JavaScript.
    pub cell_candidates: std::cell::RefCell<Vec<String>>,
    /// Of the cells, the ones that are PARAMETERS.
    ///
    /// The difference matters in argument position only. A `&mut T` parameter
    /// IS the reference: `f(buffer)` where `buffer: &mut String` reborrows it,
    /// and Rust needs no `&mut` to say so — so the cell goes over whole. A
    /// boxed LOCAL is the value, and `f(buffer)` moves it, so `.value` is what
    /// the callee gets. Everywhere else both read through `.value`, because
    /// Rust dereferences a `&mut` for every other use.
    pub cell_params: std::cell::RefCell<Vec<String>>,
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
    /// The call a `*` is about to write THROUGH, as the position of its method
    /// name.
    ///
    /// `map.entry(k).or_insert(0)` is a `&mut V` in Rust and a write-through
    /// `Slot` in the runtime: `*it += 1` stores into the map through it, and
    /// every other use reads the value it holds. The deref arm marks the call
    /// it is about to wrap so the entry lowering hands back the Slot there and
    /// the value everywhere else.
    written_through: std::cell::Cell<Option<(usize, usize)>>,
    /// Is the value the pattern being written is matched against BORROWED?
    ///
    /// Rust's default binding mode (RFC 2005) says a pattern matched against a
    /// reference binds by reference, and the payload reads have to agree:
    /// `match &result { Ok(v) => … }` binds `v: &T` and leaves the `Result`
    /// whole, where `unwrap()` takes the wrapper apart and marks it moved.
    borrowed_subject: std::cell::Cell<bool>,
    /// While an arm of a CONSUMING match is being written, the jump it
    /// performs is handed back to the caller instead of being written where it
    /// stands: an arm of `intoMatch` is a function, and `break` cannot leave
    /// one — `return break` does not even parse. The caller reads the sentinel
    /// and performs the jump itself.
    pub(crate) jump_as_value: std::cell::Cell<bool>,
    /// The loops written INSIDE the body currently being lifted, innermost
    /// last, each as the label it carries.
    ///
    /// A `break` or a `continue` naming one of these is an ordinary JavaScript
    /// jump: the loop is in the same arrow the jump is written in. Only a jump
    /// that reaches PAST every one of them has to travel out as a sentinel.
    /// With this stack missing, ankql's `generate_expr_sql` handed the
    /// `continue` that skips a NUL byte back to the arm's caller and left the
    /// String arm on the first NUL, writing an unterminated SQL literal.
    pub(crate) loops_in_lift: std::cell::RefCell<Vec<Option<String>>>,
    /// The loops being written, innermost last, each with the name a `break`
    /// carrying a value assigns to and the label it breaks — where the loop
    /// stands in VALUE position and so has one.
    ///
    /// `let v = loop { .. break n; };` is an expression in Rust and a statement
    /// in TypeScript: written as it stood, `const v = while (true) { .. }` does
    /// not parse. The loop is hoisted above the statement that wanted its value
    /// and each `break n` assigns to the name before it leaves.
    pub(crate) loop_frames: std::cell::RefCell<Vec<LoopFrame>>,
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

/// One loop being written, and what a `break` carrying a value does in it.
#[derive(Clone)]
pub struct LoopFrame {
    /// The label the loop carries, where the source wrote one.
    pub label: Option<String>,
    /// The name a `break <value>` assigns to, and the label it then leaves —
    /// only for a loop whose own value the code around it wanted.
    pub value: Option<(String, String)>,
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
            written_through: std::cell::Cell::new(None),
            borrowed_subject: std::cell::Cell::new(false),
            jump_as_value: std::cell::Cell::new(false),
            loops_in_lift: std::cell::RefCell::new(Vec::new()),
            loop_frames: std::cell::RefCell::new(Vec::new()),
            boxed: std::cell::RefCell::new(Vec::new()),
            cell_candidates: std::cell::RefCell::new(Vec::new()),
            cell_params: std::cell::RefCell::new(Vec::new()),
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
            written_through: std::cell::Cell::new(None),
            borrowed_subject: std::cell::Cell::new(false),
            jump_as_value: std::cell::Cell::new(false),
            loops_in_lift: std::cell::RefCell::new(Vec::new()),
            loop_frames: std::cell::RefCell::new(Vec::new()),
            boxed: std::cell::RefCell::new(Vec::new()),
            cell_candidates: std::cell::RefCell::new(Vec::new()),
            cell_params: std::cell::RefCell::new(Vec::new()),
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
                let receiver = parenthesise_receiver(&call.receiver, receiver);
                let rust_method = call.method.to_string();
                let (receiver, named_early) = self.name_nullable_receiver_early(call, &rust_method, receiver);
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
                        let wants = want.get(index).and_then(|t| t.as_ref());
                        // C1: a cell handed to a parameter that is itself a
                        // `&mut` to a value goes over as the CELL. Rust
                        // reborrows a `&mut` implicitly — `f(buffer)` where
                        // `buffer: &mut String` passes the reference — and
                        // `buffer.value` would hand the callee a copy of the
                        // string, which is the defect this is all about.
                        if self.names_a_cell_param(a) {
                            return Self::path_static(match a {
                                syn::Expr::Path(path) => &path.path,
                                _ => unreachable!("names_a_cell answered for a path"),
                            });
                        }
                        self.expecting(a, wants, || self.moved_value(a))
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
                        // A call through an open bound: the receiver is a type
                        // parameter, so the engine reached the trait's own
                        // declaration and no impl. Where the trait's impls are
                        // emitted as module-level functions, the class the
                        // receiver will be has no such method and
                        // `subject.members()` is a call on `undefined`.
                        if let Some(name) = self.open_bound_call(&tc_ref, &found, call) {
                            drop(tc_ref);
                            let mut written = vec![recv];
                            written.extend(args.iter().cloned());
                            return format!("{}({})", name, written.join(", "));
                        }
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
                        let translated = native_types::translate_method_using(
                            tc_ref.registry,
                            found.receiver_type(),
                            &recv,
                            &rust_method,
                            &args,
                            native_types::Position {
                                used: !self.discards(call),
                                reads_as_value: !self.is_written_through(call),
                            },
                            &once,
                        );
                        // R9: `Ord::cmp` owns `compareTo`, and a written-out
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
                // A CALLEE is a postfix base like any other: `get_function()
                // .await(8)` came out `await getFunction()(8)`, which calls the
                // promise.
                let func = parenthesise_receiver(&call.func, func);
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
                        // C1: a `&mut T` parameter IS the reference, so handing
                        // it to another one reborrows — Rust needs no `&mut` to
                        // say so — and the CELL goes over. `.value` would hand
                        // the callee a copy of the string, which is the defect
                        // this is all about.
                        if self.names_a_cell_param(a) {
                            return Self::path_static(match a {
                                syn::Expr::Path(path) => &path.path,
                                _ => unreachable!("names_a_cell_param answered for a path"),
                            });
                        }
                        // D11: a `&mut T` whose `T` the port writes as a VALUE
                        // is a cell, and only a LOCAL is held in one.
                        if let Some(hole) =
                            self.cell_argument_gap(a, want.get(index).and_then(|t| t.as_ref()))
                        {
                            return hole;
                        }
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
                // R10: a parameter bound by `Fn`, `FnMut` or `FnOnce` may be
                // handed a plain function or the `OwnedClosure` the emitter
                // writes when the closure captured values with drop glue, and
                // the callee cannot see which. `invoke` is the one place that
                // tells them apart.
                if let Some(helper) = self.bound_closure_helper(&call.func) {
                    let mut through = vec![func.clone()];
                    through.extend(args.iter().cloned());
                    return format!("{}({})", helper, through.join(", "));
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
                            let place = self.deref_place_read_once(unary);
                            let op = translate_binop(&bin.op);
                            // R7 reaches through the cell too. `*n += 1` on a
                            // `&mut u32` is Rust arithmetic on a `u32`, and
                            // writing `n.value += 1` skipped the overflow check
                            // the same statement gets when the place is a local.
                            let width = self
                                .quietly(|| self.resolve_expr_type(&bin.left))
                                .ok()
                                .and_then(|ty| match ty.peel_refs() {
                                    crate::ty::Ty::Prim(prim) if prim.is_integer() => Some(*prim),
                                    _ => None,
                                });
                            let want = width.map(crate::ty::Ty::Prim);
                            let right = self
                                .expecting(&bin.right, want.as_ref(), || self.expr_value(&bin.right));
                            if let (Some(prim), Some(helper)) =
                                (width, crate::operators::primitives::checked_helper(op))
                            {
                                return format!(
                                    "{place} = {helper}({place}, {right}, '{width}')",
                                    place = place,
                                    helper = helper,
                                    right = right,
                                    width = crate::operators::primitives::width_name(prim)
                                );
                            }
                            return format!("{} {} {}", place, op, right);
                        }
                    }
                }
                // An operand is a VALUE position: an `if`, a `match`, a block
                // or a `loop` written there is a statement in TypeScript, and
                // `total + if (c) { .. }` does not parse.
                let left = self.expecting(&bin.left, shift_expectation(&bin.op, expected.as_ref()), || {
                    self.expr_value(&bin.left)
                });
                // `&&` and `||` evaluate their right operand only if the left
                // one allows it, so anything that operand took to evaluate
                // itself belongs inside the branch the short circuit guards.
                if matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
                    let (right, lifted) = self.with_own_hoists(|| self.expr_value(&bin.right));
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
                    self.expecting(&bin.right, want.as_ref(), || self.expr_value(&bin.right));
                // What the operator resolves to: the impl's method where the
                // operands are not primitives, and the JavaScript operator with
                // whatever correction its arithmetic needs where they are.
                if let Some(resolved) = self.binary_operator(bin, &left, &right) {
                    return resolved;
                }
                format!("{} {} {}", left, translate_binop(&bin.op), right)
            }

            syn::Expr::Unary(unary) => {
                // `-1i64` is one literal in Rust and two tokens here, and the
                // width belongs to the LITERAL: without the expectation reaching
                // it, `const MIN: i64 = -9007199254740991;` came out with no `n`
                // suffix, so the value was a `number` where a `bigint` was
                // declared. The same forwarding is what makes `x / -1` resolve
                // its operand under the operator's primitive.
                //
                // Only a primitive expectation travels: `-duration` on a type
                // with an `impl Neg` is a call, and its operand is not the
                // Output type.
                let want = match &expected {
                    Some(ty @ crate::ty::Ty::Prim(_))
                        if matches!(unary.op, syn::UnOp::Neg(_) | syn::UnOp::Not(_)) =>
                    {
                        Some(ty.clone())
                    }
                    _ => None,
                };
                let e = match &want {
                    Some(ty) => self.expecting(&unary.expr, Some(ty), || self.expr(&unary.expr)),
                    None => self.expr(&unary.expr),
                };
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
                let value = match &ret.expr {
                    // What is returned leaves through the function's return
                    // type, which is what says what it has to be.
                    Some(expr) => {
                        let want = self.fn_return.clone();
                        Some(self.expecting(expr, want.as_ref(), || self.moved_value(expr)))
                    }
                    None => None,
                };
                // Inside a body the emitter lifted into an arrow, a `return`
                // returns from the arrow. It is handed back as a value
                // instead, and the statement that reads the lifted value
                // performs the real return.
                if self.jump_as_value.get() {
                    return crate::control_flow::sentinel::return_marker(
                        value.as_deref().unwrap_or("undefined"),
                    );
                }
                match value {
                    Some(value) => format!("return {}", value),
                    None => "return".to_string(),
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
                    return self.while_let(
                        let_expr,
                        &while_loop.body,
                        &label,
                        &while_loop.label,
                    );
                }
                // Rust evaluates the condition afresh each turn and drops
                // what it produced before the body runs. Where the condition
                // lifted anything, the test moves inside the loop: leaving it
                // in the `while` header evaluated it once and held whatever it
                // took for the life of the loop.
                let (cond, lifted) = self.with_own_hoists(|| self.expr(&while_loop.cond));
                let body = crate::control_flow::sentinel::inside_a_loop(
                    self,
                    &while_loop.label,
                    || self.translate_loop_block(&while_loop.body),
                );
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
                let body = crate::control_flow::sentinel::inside_a_loop(
                    self,
                    &loop_expr.label,
                    || self.translate_loop_block(&loop_expr.body),
                );
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
                if crate::control_flow::sentinel::jump_leaves_the_lift(self, &brk.label) {
                    return crate::control_flow::sentinel::jump_marker("break", &brk.label);
                }
                // `break n` in a loop whose value the code around it wanted:
                // the value is what the loop produces, so it is assigned to the
                // name that stands for it and then the loop is left. Written as
                // a comment beside a bare `break`, the value was discarded.
                if let Some(value) = &brk.expr {
                    if let Some((held, label)) =
                        crate::control_flow::sentinel::value_loop_for(self, &brk.label)
                    {
                        let written = self.expr_value(value);
                        return format!("{} = {};\nbreak {}", held, written, label);
                    }
                }
                let target = ownership::iteration::target_of(&brk.label);
                if let Some(expr) = &brk.expr {
                    format!("break{} /* {} */", target, self.expr(expr))
                } else {
                    format!("break{}", target)
                }
            }

            syn::Expr::Continue(cont) => {
                if crate::control_flow::sentinel::jump_leaves_the_lift(self, &cont.label) {
                    return crate::control_flow::sentinel::jump_marker("continue", &cont.label);
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
                    let base = self.expr(&idx.expr);
                    let base = parenthesise_receiver(&idx.expr, base);
                    return format!("{}.slice({}{})", base, from, end);
                }
                // `[Owned::new()][0]` reads out of a sequence the expression
                // itself built, and that sequence is a temporary Rust drops at
                // the end of the statement.
                let base = self.expr(&idx.expr);
                let base = self.hoist_produced(&idx.expr, base);
                let base = parenthesise_receiver(&idx.expr, base);
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
            // C1: `&mut x` where `x` is already a cell hands the cell over.
            // `x` alone would read `x.value` — the value, not the place — and
            // the callee's writes would go nowhere.
            syn::Expr::Reference(reference)
                if reference.mutability.is_some() && self.names_a_cell(&reference.expr) =>
            {
                Self::path_static(match &*reference.expr {
                    syn::Expr::Path(path) => &path.path,
                    _ => unreachable!("names_a_cell answered for a path"),
                })
            }

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
                // `proto::Presence { .. }` where `proto` is this file's name
                // for `ankurah_proto`: the port flattens a crate into a
                // package, so the type is imported by its leaf and the
                // qualifier names nothing here.
                let mut name = self
                    .through_sibling_crate(&s.path)
                    .or_else(|| self.through_local_module(&s.path))
                    .unwrap_or_else(|| Self::path_static(&s.path));
                if name == "Self" { name = self.self_type.to_string(); }
                self.struct_literal(s, &name)
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
            // What is awaited is read for its value, so a `match` or an `if`
            // written there is a value and not a statement: `.await` on a
            // `match` whose arm leaves the function used to reach the
            // statement form and put a bare `if` where the operand belonged.
            syn::Expr::Await(await_expr) => {
                format!("await {}", self.expr_value(&await_expr.base))
            }

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
                // The block is a future of its own, so a `return` inside it
                // returns from it and belongs to it. Whatever the body around
                // it is doing with jumps is not this block's business.
                let body = self.inside_its_own_function(|| self.translate_block(&async_block.block));
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
    /// Translate something that is a function of its own — a closure body, an
    /// `async` block — with the enclosing body's jump handling put aside.
    ///
    /// A `return` inside a closure returns from the closure, and Rust means it
    /// that way too, so the sentinel the enclosing lift is collecting has no
    /// business here.
    pub(crate) fn inside_its_own_function<R>(&self, write: impl FnOnce() -> R) -> R {
        let previous = self.jump_as_value.replace(false);
        let answer = write();
        self.jump_as_value.set(previous);
        answer
    }

    /// A name for a value the emitted code has to hold on to.
    ///
    /// A pattern match tests its subject and then takes it apart, and the
    /// subject has to be the *same* value both times: `if let Some(x) =
    /// c.step().await?` that writes the call twice calls it twice.
    /// A name for a value the translator needs to hold, which no binding in
    /// scope already answers to.
    ///
    /// The counter alone would hand out `_v` to a body that declares its own
    /// `let _v`, and the emitted `const _v = ..` would shadow it for the rest
    /// of the block — the Rust name still read, and reading the wrong value.
    /// The type context knows every local in scope, so a candidate it already
    /// knows is passed over. Where the context is busy (the translator asks for
    /// a temporary in the middle of resolving something) the counter stands on
    /// its own, which is what it always did.
    pub fn fresh_temp(&self) -> String {
        loop {
            let n = self.temporaries.get();
            self.temporaries.set(n + 1);
            let candidate = if n == 0 { "_v".to_string() } else { format!("_v{}", n) };
            let taken = self
                .types
                .as_ref()
                .and_then(|tc| tc.try_borrow().ok().map(|tc| tc.lookup(&candidate).is_some()))
                .unwrap_or(false);
            if !taken {
                return candidate;
            }
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
        written_label: &Option<syn::Label>,
    ) -> String {
        // The scrutinee is read afresh every turn, in value position: it is the
        // turn's own value, and an `if` written there is a run of statements
        // that `const _v = …` cannot hold. Whatever it lifted belongs inside
        // the loop with it, because it is taken again on the next turn.
        let (scrutinee, lifted) = self.with_own_hoists(|| self.expr_value(&let_expr.expr));
        let ty = self.borrowed_scrutinee_type(&let_expr.expr);
        let _bindings = self.enter_pattern(&let_expr.pat, ty.as_ref());
        // The pattern binds afresh each turn, and Rust drops what it bound at
        // the end of that turn — so the release goes inside the loop, not after
        // it.
        let owned = self.claim_bindings(&bound_names(&let_expr.pat), &body.stmts);
        let translated = crate::control_flow::sentinel::inside_a_loop(self, written_label, || {
            self.translate_loop_block(body)
        });
        drop(_bindings);

        let subject = self.fresh_temp();
        // The binding scope closed above, so the borrowed-ness of the value
        // this turn takes apart is said again here.
        let (test, bind) =
            self.matching(ty.as_ref(), || self.pattern_test(&subject, &let_expr.pat));
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
    /// The same place, read ONCE.
    ///
    /// `*counts.entry(k).or_insert(0) += 1` is one place in Rust and two
    /// mentions here — `p = f(p, 1)` — so a place with a side effect performed
    /// it twice: the entry was created twice and the key cloned twice, and the
    /// second clone leaked. The receiver is named first where it is not already
    /// a place, and the accessor hangs off the name.
    /// A `&mut <place>` handed to a parameter the port holds in a CELL, where
    /// the place is not a local.
    ///
    /// C1 turns `&mut u32` into a `BorrowMut<number>` so the callee's write
    /// reaches the caller, and only a local can be held in one: `&mut c.n`
    /// hands the callee a copy of the number, and the write goes nowhere.
    /// `ownership.md` said this was reported; it was not, and the emitted call
    /// passed a bare `number` to a `BorrowMut<number>` parameter.
    ///
    /// R12: the site says what it could not translate and stops there, rather
    /// than running an update nobody sees.
    fn cell_argument_gap(&self, arg: &syn::Expr, want: Option<&crate::ty::Ty>) -> Option<String> {
        let crate::ty::Ty::Ref { mutable: true, inner } = want? else {
            return None;
        };
        let spelled = match &self.types {
            Some(tc) => crate::name_map::map_ty(tc.borrow().registry, inner),
            None => return None,
        };
        if !crate::is_value_spelling(&spelled) {
            return None;
        }
        let syn::Expr::Reference(reference) = arg else { return None };
        if reference.mutability.is_none() {
            return None;
        }
        // A single name is the case C1 covers: the local is held in a cell and
        // the cell is what goes over.
        if matches!(&*reference.expr, syn::Expr::Path(path) if path.path.segments.len() == 1) {
            return None;
        }
        Some(self.hole(
            syn::spanned::Spanned::span(arg),
            format!(
                "`&mut {}` borrows a place that is not a local, and a `&mut` to a value \
                 JavaScript copies is passed as a cell — which only a local can be held in, so \
                 the callee's write would reach nobody",
                spelled
            ),
        ))
    }

    pub(crate) fn deref_place_read_once(&self, unary: &syn::ExprUnary) -> String {
        if crate::body::is_place(&unary.expr) || self.names_a_cell(&unary.expr) {
            return self.deref_place(unary);
        }
        // `hoist_produced` first, because a temporary with drop glue — a mutex
        // guard — owes a release and a bare `const` would not give it one. Only
        // where it declines does the place get a plain name of its own.
        let written = self.through_place(&unary.expr, || self.expr(&unary.expr));
        let held = self.hoist_produced(&unary.expr, written.clone());
        let inner = if held == written { self.hoist_name(written) } else { held };
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

    pub(crate) fn deref_place(&self, unary: &syn::ExprUnary) -> String {
        // C1: a name the body holds in a cell is ALREADY read through it —
        // `path_expr` writes `found.value` — so `*found` is that place and not
        // a second `.value` on top of it.
        if self.names_a_cell(&unary.expr) {
            return self.expr(&unary.expr);
        }
        let inner = self.through_place(&unary.expr, || self.expr(&unary.expr));
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

/// An expression written where a VALUE belongs, and the jump one may carry out
/// of that position.
mod values;
pub(crate) use values::*;

/// What the translator knows while it writes: scopes, expectations, reports.
mod scopes;

/// A call the engine could not resolve, and the free functions an impl with no
/// class of its own became.
pub(crate) mod calls;

/// Reading a field, and the places a value moves out of.
mod places;

/// A path in expression position, and the values a path names.
mod paths;

/// The small pieces of TypeScript the translator writes by hand.
mod writing;
pub(crate) use writing::*;
/// What a pattern asks of a value, and what it takes out of it.
mod pat_shape;
mod patterns;
#[cfg(test)]
mod patterns_tests;
/// What a module-level `const` and a `static` are, and what naming one means.
#[cfg(test)]
mod const_tests;
/// What the pattern machinery writes, tested apart from the rest of the body.
#[cfg(test)]
mod pattern_tests;
/// Names the emitter chooses, and the places it cannot choose one at all.
#[cfg(test)]
mod shadow_tests;

