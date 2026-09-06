//! An expression written where a VALUE belongs.
//!
//! For: Rust reads a `match`, an `if`, a block and a `loop` as expressions, and
//! TypeScript writes all four as statements. A statement cannot stand where a
//! value does — `const x = if (..) {` does not parse at all — so each of them
//! becomes a ternary where its shape allows one and an immediately-called arrow
//! otherwise. What makes it more than a rewrite is that a Rust value position
//! may still contain a jump: a `?`, a `return`, a `break`, a `continue`. None of
//! those can leave the function the arrow is, so the value the position asked
//! for comes back carrying the jump instead, and the statement below performs
//! it — which is the sentinel protocol in `control_flow::sentinel`.


use super::{iife, indent, references_var, BodyTranslator};
use crate::control_flow;
use crate::ownership;

impl BodyTranslator<'_> {
    /// An expression whose value is wanted here and one of whose arms leaves
    /// the loop around it.
    ///
    /// The value is computed in a wrapper function, which a `break` cannot
    /// leave. So the arm that jumps hands the jump back instead, the statement
    /// before this one reads it and performs the jump, and what stands here is
    /// the value the other arms produced. `core/src/reactor/fetch_gap.ts` was
    /// one of the four emitted files a JavaScript engine refused to load.
    pub(crate) fn value_through_a_jump(&self, expr: &syn::Expr) -> String {
        let (body, _) = control_flow::sentinel::lifting(self, || {
            control_flow::translate_expr_in_return_position(expr, self)
        });
        let awaits = control_flow::awaiting::awaits(expr);
        let held = self.hoist_name(iife("()", &format!("{}\n", body), "", awaits));
        let jumps = crate::control_flow::sentinel::jumps_out_of(expr);
        let tests = control_flow::sentinel::reader(
            self,
            &held,
            control_flow::sentinel::Handed {
                returns: crate::control_flow::sentinel::leaves_the_function(expr),
                jumps: &jumps,
            },
        );
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: tests,
            owned: None,
            temp: None,
            refused: false,
            released_if_unreached: false,
        });
        // The value's type is the union of what the arms produced and the
        // sentinel, and the jump above is what rules the sentinel out.
        format!("({} as any)", held)
    }

    /// `loop { .. break n; }` standing where a value is wanted.
    ///
    /// A hoisted name, then the loop under a label of its own, with every
    /// `break` carrying a value assigning to the name before it leaves. The
    /// expression itself is the name. Written as it stood, `const v = while
    /// (true) { .. }` is a statement where an expression has to be, which a
    /// JavaScript engine refuses to parse.
    pub(crate) fn loop_as_value(&self, loop_expr: &syn::ExprLoop) -> String {
        let held = self.fresh_hoist("_lv");
        let label = match &loop_expr.label {
            // A labelled loop keeps its own name, so a `break 'outer n` written
            // inside it still names the loop the source named.
            Some(label) => label.name.ident.to_string(),
            None => self.fresh_hoist("_at"),
        };
        let body = crate::control_flow::sentinel::inside_a_loop_for(
            self,
            &loop_expr.label,
            Some((held.clone(), label.clone())),
            || self.translate_loop_block(&loop_expr.body),
        );
        self.own.prelude.borrow_mut().push(ownership::Hoist {
            declaration: format!(
                "let {held};\n{label}: while (true) {{\n{body}}}\n",
                held = held,
                label = label,
                body = indent(&body)
            ),
            owned: None,
            temp: None,
            refused: false,
            released_if_unreached: false,
        });
        held
    }

    pub(crate) fn block_as_value(&self, block: &syn::ExprBlock) -> String {
        // A `?`, a `return`, a `break` or a `continue` written in the block
        // leaves something the arrow this block becomes is not: the function,
        // or the loop around it. The exit travels out as a value the statement
        // below performs; see `value_through_a_jump`.
        let whole = syn::Expr::Block(block.clone());
        if crate::control_flow::sentinel::leaves_the_function(&whole)
            || crate::control_flow::sentinel::jumps_out_of_a_loop(&whole)
        {
            return self.value_through_a_jump(&whole);
        }
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
            // #14: `let v = loop { .. break n; };`. The loop is a statement in
            // TypeScript, so it is hoisted above the statement that wanted its
            // value and each `break n` assigns to the name standing for it.
            syn::Expr::Loop(loop_expr) => return self.loop_as_value(loop_expr),
            // P1: `let n = { if ok { 1 } else { 2 } };`. A block of ONE
            // statement is written as that statement, and the statement is
            // asked for as an ordinary expression — so an `if` came out as an
            // `if`, where TypeScript needs a ternary or an arrow. The block is
            // transparent, so what the position wants of it, it wants of the
            // tail.
            syn::Expr::Block(block) if block.label.is_none() => {
                return match single_block_expr(&block.block) {
                    Some(tail) => self.expecting(tail, self.expectation_for(expr).as_ref(), || {
                        self.expr_value(tail)
                    }),
                    // A block of several statements is its own scope, and
                    // `block_as_value` is what threads a name it shadows — but
                    // a jump inside one leaves the enclosing loop or function,
                    // and neither `break` nor a `return` meant for the caller
                    // can be written inside the scope this becomes. `sink({
                    // break 'outer; })` came out `sink(break outer)`, which a
                    // JavaScript engine refuses to parse.
                    None => {
                        if crate::control_flow::sentinel::jumps_out_of_a_loop(expr)
                            || crate::control_flow::sentinel::leaves_the_function(expr)
                        {
                            self.value_through_a_jump(expr)
                        } else {
                            self.expr(expr)
                        }
                    }
                };
            }
            // A parenthesised expression is the expression: what the position
            // wants of `(if c { 1 } else { 2 })` it wants of the `if` inside.
            // Read as an ordinary expression, `(if yes { 1 } else { 2 }) + 3`
            // came out with a statement where an operand belongs.
            syn::Expr::Paren(paren) => {
                // The expectation is re-keyed onto the inner expression: it is
                // matched by SPAN, and the paren's span is not the inner one's.
                // Without it `(1 << 63)` under a `u64` expectation lost its
                // width and came out `1 << 63` — a `number`.
                let inner = self.expecting(&paren.expr, self.expectation_for(expr).as_ref(), || {
                    self.expr_value(&paren.expr)
                });
                // The author's own parentheses are kept unless the written form
                // already carries them: `((yes ? 1 : 2))` says nothing more
                // than `(yes ? 1 : 2)`, and dropping them outright would turn
                // `(a + b) * c` into `a + b * c`.
                return if inner.starts_with('(') && inner.ends_with(')') && balanced(&inner) {
                    inner
                } else {
                    format!("({})", inner)
                };
            }
            syn::Expr::Group(group) => return self.expr_value(&group.expr),
            // A `match` is a value in Rust too. Where the port writes one as
            // the runtime's `match`, that is already an expression; where it
            // writes an `if`/`else` chain — an `Option` or a `Result` match —
            // the statements have to stand inside an arrow function, or
            // `const x = if (..) {` is what comes out, which does not parse.
            syn::Expr::Match(_) => {
                // Ask before writing: an arm that leaves the function has to
                // leave THIS function, and every arm of the runtime's match is
                // an arrow. The written form is what the answer changes.
                if crate::control_flow::sentinel::leaves_the_function(expr) {
                    return self.value_through_a_jump(expr);
                }
                let written = self.expr(expr);
                // Asked of the lowering that just wrote it, not of the text:
                // the runtime's keyed `.match({..})` IS an expression and
                // every other strategy is an if-chain (K2).
                if !crate::control_flow::form::writes_statements(expr, self) {
                    return written;
                }
            }
            // A jump written where a VALUE belongs — `sink({ break 'outer; })`,
            // `f(return x)` — is not an expression in TypeScript at all:
            // `sink(break outer)` is a SyntaxError, and the module carrying it
            // does not load. It goes through the sentinel like any other jump
            // the position cannot hold.
            syn::Expr::Break(_) | syn::Expr::Continue(_) | syn::Expr::Return(_) => {}
            _ => return self.expr(expr),
        }
        // The wrapper is a function, and no jump can leave one. `break` and
        // `continue` do not parse there at all — `Cannot use "continue" here`
        // is what a JavaScript engine says, and the whole module then fails to
        // load — and a `return`, or the early exit a `?` performs, quietly
        // returns from the wrapper instead of from the function. Either way
        // the jump is handed back as a value, and the statement that wanted
        // the value performs it before reading one.
        if crate::control_flow::sentinel::jumps_out_of_a_loop(expr)
            || crate::control_flow::sentinel::leaves_the_function(expr)
        {
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
        // P2: a `break`, a `continue`, a `return` or a `?` is a statement, and
        // a ternary branch is not one. `total += if x == 0 { break } else { x }`
        // came out with a `break` inside the ternary, which a JavaScript engine
        // refuses to parse at all. The lifted form is what carries a jump.
        let whole = syn::Expr::If(if_expr.clone());
        if crate::control_flow::sentinel::jumps_out_of_a_loop(&whole)
            || crate::control_flow::sentinel::leaves_the_function(&whole)
        {
            return None;
        }
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
        // Parenthesised, because a ternary has the loosest precedence there is
        // and this stands wherever a value stands: `a == if yes { 1 } else { 2 }`
        // came out `a === yes ? 1 : 2`, which JavaScript reads as
        // `(a === yes) ? 1 : 2` — the comparison swallowed by the condition.
        // The same reason every `Option` combinator is parenthesised.
        Some(format!("({} ? {} : {})", cond, then_ts, else_ts))
    }
}

/// Is this text one parenthesised group — `(a ? b : c)` rather than
/// `(a) + (b)`, which also begins with `(` and ends with `)`?
pub(crate) fn balanced(text: &str) -> bool {
    let mut depth = 0usize;
    for (at, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1 == text.len();
                }
            }
            _ => {}
        }
    }
    false
}

pub(crate) fn single_block_expr(block: &syn::Block) -> Option<&syn::Expr> {
    if block.stmts.len() == 1 {
        if let syn::Stmt::Expr(expr, _) = &block.stmts[0] {
            return Some(expr);
        }
    }
    None
}

