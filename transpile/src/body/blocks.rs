//! A block, and the statements in it.
//!
//! For: Rust drops what a scope owns at its end, at every early exit out of it,
//! and while an unwind passes through, and TypeScript does none of that — so a
//! block is not a run of translated statements but a run of statements with the
//! releases the scope owes written into it. That is what this writes: which
//! locals the block claims, where each `try`/`finally` opens, and what one
//! statement is once the declarations it lifted out of itself stand before it.

use crate::control_flow;
use crate::macros;
use crate::ownership;
use crate::body::places::EntryFinish;

use super::{
    as_move_closure, extract_macro, is_match_with_write_arms,
    is_mut_binding, is_write_macro, is_write_macro_path, pattern_names, references_var,
    BodyTranslator,
};

impl BodyTranslator<'_> {
    // ── Block translation with ownership tracking ───────────────────

    /// A function's own body.
    ///
    /// Rust drops a by-value parameter when the function returns, so the body
    /// owns its parameters the way it owns its locals — released in the same
    /// `finally`, after everything the body itself declared, and not at all
    /// where the body hands one on.
    pub fn translate_fn_block(
        &self,
        block: &syn::Block,
        params: &[(String, crate::ty::Ty)],
    ) -> String {
        self.push_block();
        // The parameters are claimed before the body is written, because a
        // parameter handed away inside a branch needs its drop flag registered
        // by the time the branch that sets it is translated.
        self.note_once_closures(&block.stmts);
        // C1: a local this body hands out as `&mut` and whose type the port
        // writes as a JavaScript VALUE has to live in a cell, because the
        // callee's writes have nowhere else to go. Read before anything is
        // translated, so the `let` that introduces it declares the cell.
        *self.cell_candidates.borrow_mut() = super::cells::cells_wanted(block);
        let owned = self.claim_params(block, params);
        let body = self.translate_block_stmts(block);
        self.pop_scope();
        for owned in &owned {
            if let Some(source) = &owned.source {
                self.own.flags.borrow_mut().remove(source);
            }
        }
        let mut out = body;
        for param in owned.iter().rev() {
            out = ownership::wrap(&out, param);
        }
        let mut declarations = String::new();
        // A formatter composes one string, and every `write!` in it appends to
        // this. It used to be spliced in by `emit.rs` searching the finished
        // text for `_result +=`, which found the statement forms and not the
        // tail — so a `Display` that ended in `write!(f, "b")` answered `"b"`
        // rather than everything it had written.
        if self.formatter && self.wrote_result.get() {
            declarations.push_str("let _result = '';\n");
        }
        for param in &owned {
            if let Some(flag) = &param.flag {
                declarations.push_str(&format!("let {} = false;\n", flag));
            }
        }
        format!("{}{}", declarations, out)
    }

    pub fn translate_block(&self, block: &syn::Block) -> String {
        // A Rust block is a scope: a `let` inside it shadows what is outside and
        // stops shadowing at the closing brace, and TypeScript's `const` in a
        // nested block does the same.
        self.push_block();
        let out = self.translate_block_stmts(block);
        self.pop_scope();
        out
    }

    /// The body of a LOOP, whose value nothing wants.
    ///
    /// A block's tail is its value, and a block that is the function's body
    /// hands that value to the caller — so the tail is written in return
    /// position. A loop body is neither: Rust types it `()`, and one turn's
    /// tail is not the function's answer. Written as a return, ankql's
    /// `generate_expr_sql` came out `return item.match({ .. })` — the loop left
    /// on its first turn, and the `?` inside an arm returned a bare
    /// `Result.Err` as the function's value.
    pub fn translate_loop_block(&self, block: &syn::Block) -> String {
        self.push_block();
        let stmts = &block.stmts;
        self.note_once_closures(stmts);
        let dispositions = self.analyse_moves(stmts);
        let ordinals = std::cell::RefCell::new(std::collections::HashMap::new());
        let out = self.emit_from_at(stmts, 0, &dispositions, &ordinals, false);
        self.pop_scope();
        out
    }

    /// A block's statements, with the releases the block owes written into it.
    ///
    /// Every value the block still owns when it ends is released in a
    /// `finally`, so that a `return`, a `?`, a `break` and a thrown fatal all
    /// leave through it — which is what Rust's drop glue does and what a run of
    /// drops at the end of the block did not. The `try` opens immediately after
    /// each declaration rather than at the top of the block: a `const` declared
    /// inside a `try` is not in scope in its `finally`, and opening one `try`
    /// per declaration also gets reverse declaration order for free, since the
    /// innermost `finally` runs first.
    pub(crate) fn translate_block_stmts(&self, block: &syn::Block) -> String {
        let stmts = &block.stmts;
        self.note_once_closures(stmts);
        let dispositions = self.analyse_moves(stmts);
        let ordinals = std::cell::RefCell::new(std::collections::HashMap::new());
        self.emit_from(stmts, 0, &dispositions, &ordinals)
    }

    /// Statements `i..` of a block, with everything after an owning
    /// declaration nested inside that declaration's `try`.
    pub(crate) fn emit_from(
        &self,
        stmts: &[syn::Stmt],
        i: usize,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
    ) -> String {
        self.emit_from_at(stmts, i, dispositions, ordinals, true)
    }

    /// The same, with `tail_is_value` saying whether this block's last
    /// expression is the block's VALUE. It is for every block but a loop body.
    pub(crate) fn emit_from_at(
        &self,
        stmts: &[syn::Stmt],
        i: usize,
        dispositions: &ownership::Dispositions,
        ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
        tail_is_value: bool,
    ) -> String {
        let Some(stmt) = stmts.get(i) else {
            return String::new();
        };
        // A brace-delimited macro at the end of a block is the block's value in
        // Rust, and `syn` parses it as `Stmt::Macro` rather than `Stmt::Expr` —
        // so the tail path used to walk past `select! { .. }` written last and
        // throw away the value the arm produced. A macro with a semicolon after
        // it is a statement like any other.
        let tail_macro = match stmt {
            syn::Stmt::Macro(mac) => mac.semi_token.is_none(),
            _ => false,
        };
        let is_tail = tail_is_value
            && i + 1 == stmts.len()
            && (matches!(stmt, syn::Stmt::Expr(_, None)) || tail_macro);

        // What this statement's own `let` should do with each name it binds,
        // read before it is translated because `local()` acts on it.
        self.set_stmt_dispositions(stmt, dispositions, ordinals);

        // A move written directly by this statement sets its flag first: after
        // it would be dead code behind a `return`, and the flag only ever
        // decides what the `finally` releases. What the statement lifted ABOVE
        // the flag (J3) — an argument that can throw before the call starts —
        // is collected while it is translated and stands ahead of it.
        let flags = self.flag_sets(stmt);
        let previous_before = std::mem::take(&mut *self.own.before_flags.borrow_mut());
        let mut out = String::new();

        let previous_prelude = std::mem::take(&mut *self.own.prelude.borrow_mut());
        let previous_pending = std::mem::take(&mut *self.own.pending.borrow_mut());
        let holes_before = crate::body::holes_written();
        let text = if is_tail {
            let held;
            let expr = match stmt {
                syn::Stmt::Expr(expr, None) => expr,
                syn::Stmt::Macro(mac) => {
                    held = syn::Expr::Macro(syn::ExprMacro {
                        attrs: mac.attrs.clone(),
                        mac: mac.mac.clone(),
                    });
                    &held
                }
                _ => unreachable!("the tail was just matched"),
            };
            // A block's tail is the block's value, so it leaves through
            // whatever the block itself was expected to produce — and through
            // the function's return type where the block *is* the function's
            // body (spec 4.6). Forcing the function's return on every tail made
            // `let bytes: Vec<u8> = { vec![10, 11] };` ask the tail for the
            // function's type instead of the `let`'s.
            // A formatter's tail write appends and then answers the string it
            // has composed. This is where the old textual post-pass could not
            // reach: it rewrote the lines that read `return Result.Ok(..)`, and
            // a tail `write!(f, "b")` is not one of those, so everything the
            // body had written before it was thrown away.
            if let Some((mac, _)) =
                crate::body::formatter_write(expr).filter(|_| self.formatter)
            {
                let written = macros::translate_macro(mac, self);
                self.wrote_result.set(true);
                format!("_result += {};\nreturn _result;\n", written)
            } else {
            let want = self.expectation_for(expr).or_else(|| self.fn_return.clone());
            self.expecting(expr, want.as_ref(), || {
                format!(
                    "{}\n",
                    control_flow::translate_expr_in_return_position(expr, self)
                )
            })
            }
        } else {
            self.stmt(stmt)
        };
        // K3: a statement that REFUSED hands nothing away, so its move flags
        // are lies — a set flag turns the block's release off for a value the
        // hole left sitting there (`core/value/cast_predicate.ts`'s `ExprList`
        // arm leaked its payload on every call). Taking the flags off puts the
        // value back under the block's `finally`; a local marked moved outright
        // has no `finally`, so its release is written above the throw.
        let refused = crate::body::holes_written() > holes_before;
        let flags = if refused { String::new() } else { flags };
        let before_flags =
            std::mem::replace(&mut *self.own.before_flags.borrow_mut(), previous_before).join("");
        out.push_str(&before_flags);
        if refused {
            out.push_str(&self.released_by_a_refusal(stmt, dispositions, ordinals));
        }
        out.push_str(&flags);
        let prelude = std::mem::replace(&mut *self.own.prelude.borrow_mut(), previous_prelude);
        let owned = std::mem::replace(&mut *self.own.pending.borrow_mut(), previous_pending);

        let rest = self.emit_from_at(stmts, i + 1, dispositions, ordinals, tail_is_value);
        // A drop flag is only readable while the local it stands for is in
        // scope. Taking it off again keeps a later block that reuses the name
        // from setting a flag nothing tests.
        for local in &owned {
            if let Some(source) = &local.source {
                self.own.flags.borrow_mut().remove(source);
            }
        }

        // A `let` puts a name in scope for everything after it, so a temporary
        // its initialiser lifted has to stay live that long too: both open a
        // `try` over the rest of the block, and the releases then happen in
        // reverse declaration order. Any other statement is over when it is
        // over, and Rust drops its temporaries there — which is what keeps a
        // lock taken in an argument from being held for the rest of the block.
        let declares = matches!(stmt, syn::Stmt::Local(_));
        let mut inner = text;
        if declares {
            // Releasing a guard at the end of its statement is what keeps the
            // lock from outliving the line that took it. Where the statement
            // *is* the rest of the block, the `finally` below says the same
            // thing, so only one of them is written.
            if !rest.trim().is_empty() {
                for hoist in prelude.iter().rev() {
                    if let Some(temp) = &hoist.owned {
                        inner.push_str(&temp.statement_release());
                    }
                }
            }
            let mut tail = rest;
            for local in owned.iter().rev() {
                tail = ownership::wrap(&tail, local);
            }
            inner.push_str(&tail);
            out.push_str(&ownership::hoisted(&inner, &prelude));
            return out;
        }
        out.push_str(&ownership::hoisted(&inner, &prelude));
        out.push_str(&rest);
        out
    }

    // ── Statement translation ───────────────────────────────────────

    pub(crate) fn stmt(&self, stmt: &syn::Stmt) -> String {
        match stmt {
            syn::Stmt::Local(local) => self.local(local),
            syn::Stmt::Expr(expr, semi) => {
                // Detect standalone `expr?;` — emit Result check
                // Every `write!`/`writeln!` in a formatter APPENDS, in all four
                // forms it is written in: with and without `?`, with and
                // without a semicolon. Only `write!(..)?;` used to, so a
                // semicolon-form write was an unused string expression and a
                // tail write replaced everything before it.
                if self.formatter {
                    if let Some((mac, returns)) = crate::body::formatter_write(expr) {
                        let written = macros::translate_macro(mac, self);
                        self.wrote_result.set(true);
                        // A write that LEAVES the formatter — a tail, or an
                        // explicit `return write!(..)` — appends and then
                        // answers what has been composed. One that carries on
                        // only appends.
                        return if returns || semi.is_none() {
                            format!("_result += {};\nreturn _result;\n", written)
                        } else {
                            format!("_result += {};\n", written)
                        };
                    }
                }
                if semi.is_some() {
                    if let syn::Expr::Try(try_expr) = expr {
                        // Special case: write!(f, ...)?; in Display impls — emit string append
                        if is_write_macro(&try_expr.expr) {
                            let fmt_str = macros::translate_macro(extract_macro(&try_expr.expr).unwrap(), self);
                            return format!("_result += {};\n", fmt_str);
                        }
                        // A `?` whose value nobody binds. Rust drops the `Ok`
                        // payload at the end of the statement, and the wrapper
                        // with it; `wrapper.drop()` cascades into both, which
                        // is why the wrapper is not simply abandoned here.
                        let lowered = self.lower_try(try_expr);
                        return match &lowered.wrapper {
                            Some(wrapper) => {
                                format!("{}{}.drop();\n", lowered.declaration, wrapper)
                            }
                            // An `Option` has no wrapper here — the port writes
                            // it as a nullable — but Rust still drops the `Some`
                            // payload at the end of the statement.
                            None => {
                                let release = self
                                    .release_of(&syn::Expr::Try(try_expr.clone()), &lowered.value)
                                    .unwrap_or_else(|| format!("{};", lowered.value));
                                format!("{}{}\n", lowered.declaration, release)
                            }
                        };
                    }
                }
                // A statement that ends in a semicolon throws its value away.
                // The call written below reads that, because what it answers
                // decides which runtime method it is.
                let discarded = match (semi, expr) {
                    (Some(_), syn::Expr::MethodCall(call)) => {
                        let at = syn::spanned::Spanned::span(&call.method).start();
                        self.discarded_call.replace(Some((at.line, at.column)))
                    }
                    _ => self.discarded_call.replace(None),
                };
                let ts = self.expr(expr);
                self.discarded_call.set(discarded);
                // If a match expression contains write! arms (Display pattern),
                // append the result to _result
                let ts = if is_match_with_write_arms(expr) {
                    self.wrote_result.set(true);
                    format!("_result += {}", ts)
                } else {
                    ts
                };
                if semi.is_some() {
                    format!("{};\n", self.discard(expr, ts))
                } else if ts.trim_end().ends_with('}') || ts.trim_end().ends_with(';') {
                    format!("{}\n", ts)
                } else {
                    // A tail with no semicolon in Rust is still a STATEMENT
                    // here when nothing wants its value — a loop body's last
                    // expression — and `return x` with no semicolon after it
                    // leaves the emitted file leaning on automatic insertion.
                    format!("{};\n", ts)
                }
            }
            syn::Stmt::Item(syn::Item::Const(c)) => self.body_const(c),
            syn::Stmt::Item(_) => String::new(),
            // A macro written with a semicolon is `Stmt::Macro`, so a
            // `write!(f, ..);` never reached the append above and was emitted
            // as an unused string expression.
            syn::Stmt::Macro(macro_stmt)
                if self.formatter && is_write_macro_path(&macro_stmt.mac) =>
            {
                let written = macros::translate_macro(&macro_stmt.mac, self);
                self.wrote_result.set(true);
                format!("_result += {};\n", written)
            }
            syn::Stmt::Macro(macro_stmt) => {
                let ts = macros::translate_macro(&macro_stmt.mac, self);
                if macro_stmt.semi_token.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
        }
    }

    pub(crate) fn local(&self, local: &syn::Local) -> String {
        let pat = Self::pat_static(&local.pat);

        let Some(init) = &local.init else {
            return format!("let {};\n", pat);
        };

        // Read before the initialiser is translated. An initialiser that is a
        // block of its own — `let sub = { let c = c.clone(); f(c) }` — runs the
        // whole block machinery again and leaves its own statement's answers
        // behind, so asking afterwards asked about the wrong statement and
        // dropped a local the outer block had already handed away.
        let disposition = self
            .own
            .stmt_dispositions
            .borrow()
            .get(&pat)
            .copied()
            .unwrap_or(ownership::Disposition::Kept);

        // Rust allows `let x = x.method()` to shadow; JavaScript refuses a
        // second declaration of the same name in the same block. This has to
        // be asked before the binding is made — and of EVERY name the pattern
        // binds, because `let [queryId, ..] = ..` shadows each of them on its
        // own.
        let mut shadowing: Vec<String> = pattern_names(&local.pat)
            .into_iter()
            .filter(|name| self.redeclares_here(name))
            .collect();
        // `let _ = expr;` binds nothing in Rust and `const _ = expr;` binds a
        // variable called `_` here, so a second one in the same scope — beside
        // a closure parameter the source also wrote `_` — is a duplicate
        // declaration. It takes a fresh name for the same reason a shadow does.
        if pattern_names(&local.pat).is_empty() && self.redeclares_here(&pat) {
            shadowing.push(pat.clone());
        }
        let already_in_scope = !shadowing.is_empty();

        // The initialiser is translated before the binding exists, because
        // it is written in the scope the `let` is shadowing:
        // `let stack = stack.borrow_mut()` borrows the *outer* `stack`, and
        // binding first would resolve that receiver to the guard the line is
        // about to introduce and reach through it.
        // `let listener = move |..| ..` binds a closure that owns what it
        // captured, so the block releases it as it releases any owned local —
        // and asking the engine for a closure's type would only report a gap
        // this line has already closed.
        let bound_closure = as_move_closure(&init.expr)
            .filter(|closure| !self.owned_captures(closure).is_empty());
        let entry_slot = self.finishes_an_entry(&init.expr);
        // What the `let` wrote for itself is what its initialiser has to
        // produce (spec 4.6): `let f: Box<dyn Fn(u32)> = |x| ..` types `x`, and
        // `let n: u8 = 1` writes a byte rather than the `i32` a bare literal
        // defaults to.
        let annotation = self
            .types
            .as_ref()
            .and_then(|tc| tc.borrow().local_annotation(local));
        let ty = match bound_closure {
            Some(_) => None,
            None => self.expecting(&init.expr, annotation.as_ref(), || self.resolve_local(local)),
        };
        let expr = match bound_closure {
            Some(closure) => self.closure(
                closure,
                ownership::closures::Placement::Bound,
                annotation.as_ref(),
            ),
            None => self.expecting(&init.expr, annotation.as_ref(), || match entry_slot {
                // R1: the `let` answers the same question the `*` does.
                EntryFinish::Slot => {
                    self.through_place(&init.expr, || self.moved_value(&init.expr))
                }
                EntryFinish::Hole | EntryFinish::Neither => self.moved_value(&init.expr),
            }),
        };

        // Only now does the name mean the new value. A `let` may take one
        // apart — `let (a, b) = ...`, `let Foo { x } = ...` — so every name
        // the pattern writes is bound, each typed from its own position.
        if self.types.is_some() {
            self.bind_pattern_here(&local.pat, ty.as_ref());
        }

        // `let PAT = e else { .. };` — the pattern is REFUTABLE, and the else
        // block runs when it does not match. Both the test and the else block
        // were dropped: `let ScanState::Scanning { .. } = state else { return
        // None };` came out as the destructuring alone, so the variant was
        // never tested and the `return None` was gone. Twelve sites.
        if let Some((_tok, diverge)) = &init.diverge {
            let subject = self.fresh_temp();
            let (test, bind) = self.pattern_test(&subject, &local.pat);
            // The else block diverges — Rust requires it — so it is written as
            // statements, where a `return`, a `break` and a `throw` all mean
            // what they meant in the source.
            let otherwise = self.statements(diverge);
            let bind = self.freshen_bindings(bind, &shadowing);
            return format!(
                "const {} = {};\nif (!({})) {{\n{}}}\n{}",
                subject,
                expr,
                test,
                crate::body::indent(&format!("{}\n", otherwise.trim_end())),
                bind
            );
        }

        // C1: a local this body hands out as `&mut` and whose type the port
        // writes as a JavaScript VALUE lives in a cell, because a number, a
        // string and a boolean are copied at the call and the callee's writes
        // would go nowhere. Decided here, where the type is known.
        let wants_a_cell = self.cell_candidates.borrow().iter().any(|c| *c == pat)
            && ty
                .as_ref()
                .is_some_and(|ty| match &self.types {
                    Some(tc) => crate::is_value_spelling(&crate::name_map::map_ty(
                        tc.borrow().registry,
                        ty,
                    )),
                    None => false,
                });
        // A finisher the engine had to REFUSE wrote a hole, not a slot, and
        // reading `.value` off a hole says nothing the hole does not already.
        // The disposition comes from the LOWERING (I1): reading it off the
        // rendered text meant an initialiser whose value carried the characters
        // `unsupported(` for any other reason stopped binding the slot.
        let entry_slot = entry_slot == EntryFinish::Slot;
        if wants_a_cell || entry_slot {
            // `freshen` hands out a NEW name each time it is asked, so the
            // rename is computed once, here, and not again below.
            let name = self.freshened_pattern(&local.pat, &shadowing);
            self.hold_in_a_cell(&name);
            // A finisher already answers the runtime's write-through slot; a
            // value local needs one built around it.
            let held = match entry_slot {
                true => expr,
                false => format!("new BorrowMut({})", expr),
            };
            return format!("const {} = {};\n", name, held);
        }

        // A name the enclosing block-as-expression already threaded in as a
        // parameter is already this value; declaring it again would shadow
        // what was threaded.
        if self.threaded.borrow().iter().any(|n| *n == pat) {
            let rust_name = if let syn::Pat::Ident(ident) = &local.pat {
                ident.ident.to_string()
            } else {
                pat.clone()
            };
            if references_var(&init.expr, &rust_name) {
                return String::new();
            }
        }
        let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
        // A Rust shadow introduces a *new* variable. Assigning to the old
        // one instead changed a value other code — a closure that captured
        // it, a caller that owns it — can still see. JavaScript will not
        // declare the same name twice here, so the shadow is emitted under a
        // fresh identifier and every later use of the name follows it.
        let emitted = if already_in_scope {
            self.freshened_pattern(&local.pat, &shadowing)
        } else {
            pat.clone()
        };
        if bound_closure.is_some() {
            self.own.owned_closure_locals.borrow_mut().push(emitted.clone());
        }
        let drops = bound_closure.map(|_| ownership::Drops::Own);
        let flag = self.claim_local(&pat, &emitted, ty.as_ref(), drops, &local.pat, disposition);
        format!("{}{} {} = {};\n", flag, keyword, emitted, expr)
    }

}

#[cfg(test)]
mod formatter_tests {
    use crate::testing::Fixture;

    /// Every `write!` inside a `Display` APPENDS to what the formatter has
    /// composed, in each of the forms a source writes it. `return write!(..)`
    /// was read as an ordinary `return`, so the string it wrote became the whole
    /// answer and everything written before it was discarded: `Size(200)`
    /// printed as `big)` where Rust prints `Size(big)`.
    #[test]
    fn a_returned_write_appends_and_then_answers_the_accumulator() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "use std::fmt;\n\
             pub struct Size(pub u32);\n\
             impl fmt::Display for Size {\n\
               fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n\
                 write!(f, \"Size(\")?;\n\
                 if self.0 > 100 {\n\
                   return write!(f, \"big)\");\n\
                 }\n\
                 write!(f, \"{})\", self.0)\n\
               }\n\
             }",
        )]);
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("_result += 'big)';"), "the write appends:\n{}", ts);
        assert!(!ts.contains("return 'big)';"), "and does not replace:\n{}", ts);
        // The early exit still leaves, with what the formatter has composed.
        let early = ts.find("_result += 'big)';").expect("the append");
        let answer = ts[early..].find("return _result;").expect("and the answer after it");
        assert!(answer < 40, "the return follows the append:\n{}", &ts[early..early + 80]);
    }
}
