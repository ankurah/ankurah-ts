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

use super::{
    extract_macro, is_match_with_write_arms, is_write_macro, is_write_macro_path, BodyTranslator,
};

mod locals_stmt;

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
        format!("{}{}", self.block_declarations(&owned, &out), out)
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

        // O6: a flag stands immediately above the TRANSFER, and `?` is the
        // one shape whose transfer is in a hoist. `flag_sets_split` says which.
        let (flags_above, flags) = self.flag_sets_split(stmt);
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
        // Did THIS statement refuse? A hole inside a CALLABLE the statement
        // passes did not stop the statement: the call that received the
        // callback ran, took what it takes, and invoked the callback — which
        // is a throw out of a call that already happened, not a statement that
        // never ran. `xs.iter().find_map(|x| … <hole> …)` and a match arm are
        // both that shape. Reading the global counter alone put every one of
        // them on the refusal path, where the cleanup releases what the callee
        // now owns. The rendered text is what says which: an arrow between the
        // start of the statement and its first hole is the callback.
        let rendered: String = self
            .own
            .prelude
            .borrow()
            .iter()
            .map(|h| h.declaration.clone())
            .chain(std::iter::once(text.clone()))
            .collect();
        let refused = crate::body::holes_written() > holes_before
            && !hole_stands_inside_a_callable(&rendered);
        let (flags_above, flags) = match refused {
            true => (String::new(), String::new()),
            false => (flags_above, flags),
        };
        let prelude = std::mem::replace(&mut *self.own.prelude.borrow_mut(), previous_prelude);
        // U3: a flag whose transfer is inside a `?` operand travels with that
        // operand's own hoist and stands immediately above it. What is left
        // here is a transfer no hoist claimed — a `?` the lowering did not
        // hoist at all — and that flag still has to stand above the prelude,
        // because the statement's own text is not reached on the error path.
        for line in flags_above.lines() {
            if prelude.iter().any(|h| h.sets.lines().any(|s| s == line)) {
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        // I4: part of a refused statement RAN before it refused. A `?` operand
        // standing to the left of the hole is evaluated and its temporary holds
        // what it took, and so is every hoist above a hole that is in the
        // statement's own TEXT. Releasing the statement's source values above
        // it would then drop a value the prefix is about to read, so those
        // releases go in a `finally` around the statement, each under a flag
        // set where the transfer is written.
        //
        // R9/D11: there used to be two walks and two wrappers here, one for a
        // refusal in a hoist and one for a refusal in the text, and only the
        // first knew about by-value parameters — so `let _v = take2(held,
        // <hole>);` released neither of its two parameters. There is one walk
        // now, and one shape, used wherever a statement refuses.
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
        // The statement threw: what it lifted and did not consume is released
        // in a `finally` around it, and what follows it is never reached.
        if refused {
            out.push_str(&super::refusal::statement_that_refused(
                self, stmt, text, rest, &prelude, dispositions, ordinals,
            ));
            return out;
        }
        // Below the prelude: written above it, `eat(c, maybe().n)` marked `c`
        // moved and then called `maybe()`, and nothing released it there.
        let mut inner = format!("{}{}", flags, text);
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
            // N4: the flag's two halves are in two streams, and both are here.
            let mut owned = owned;
            self.drop_dead_flags(&mut inner, &rest, &mut owned);
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

/// Does the first hole in this rendered statement stand inside a callable the
/// statement passes, rather than in the statement's own evaluation?
fn hole_stands_inside_a_callable(text: &str) -> bool {
    match text.find("unsupported(") {
        Some(at) => text[..at].contains("=>"),
        None => false,
    }
}
