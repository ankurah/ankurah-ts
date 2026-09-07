//! The expression walk: every position that TAKES a value, and what it takes.
//!
//! For: a move is not a syntactic form. `f(x)` moves `x`, `f(&x)` does not, and
//! `x.into_iter()` moves the receiver only because the impl table says the
//! method takes `self`. So the walk goes position by position, asking
//! `Consumes` the questions the syntax cannot answer, and records a `Site` for
//! each name a position takes.
//!
//! It is the position-by-position half of `moves.rs`, which holds what a `Site`
//! is, how sites add up, and the scan's entry points.

use syn::parse::Parser as _;

use super::{collect_pattern_names, evaluating, nested, Scan, Site, Where};

impl Scan<'_> {
    /// Walk an expression for moves, in the positions that take a value.
    pub(super) fn walk(&self, expr: &syn::Expr, at: Where, out: &mut Vec<Site>) {
        match expr {
            syn::Expr::Call(call) => {
                let args: Vec<&syn::Expr> = call.args.iter().collect();
                if self.consumes.consumes_callee(call) {
                    self.moved(&call.func, evaluating(at, &args, 0), out);
                }
                for (index, arg) in args.iter().enumerate() {
                    self.moved(arg, evaluating(at, &args, index + 1), out);
                    self.walk(arg, at, out);
                }
                self.walk(&call.func, at, out);
            }

            syn::Expr::MethodCall(call) => {
                // J4: a call the engine refuses takes NOTHING: the hole that
                // replaces it throws, and the receiver and arguments are still
                // the block's to release. The sub-expressions are still walked,
                // because a move written INSIDE an argument — `f(g(x))` — is
                // one the hole does not undo... but nothing inside a refused
                // call is emitted at all, so the walk stops here too.
                if self.consumes.refuses_call(call) {
                    return;
                }
                // A method taking `self` consumes the receiver: `r.unwrap()`,
                // `v.into_iter()`, `opt.take()`.
                let args: Vec<&syn::Expr> = call.args.iter().collect();
                if self.consumes.consumes_receiver(call) {
                    self.moved(&call.receiver, evaluating(at, &args, 0), out);
                }
                self.walk(&call.receiver, at, out);
                for (index, arg) in args.iter().enumerate() {
                    self.moved(arg, evaluating(at, &args, index + 1), out);
                    self.walk(arg, at, out);
                }
            }

            // Awaiting a named future takes it by value, exactly as a
            // self-taking method does, and the emitter releases nothing after.
            syn::Expr::Await(await_expr) => {
                self.moved(&await_expr.base, at, out);
                self.walk(&await_expr.base, at, out);
            }

            syn::Expr::Assign(assign) => {
                self.moved(&assign.right, at, out);
                self.walk(&assign.right, at, out);
                self.walk(&assign.left, at, out);
            }

            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.moved(value, at, out);
                    self.walk(value, at, out);
                }
            }

            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.moved(value, at, out);
                    self.walk(value, at, out);
                }
            }

            syn::Expr::Struct(s) => {
                let fields: Vec<&syn::Expr> = s.fields.iter().map(|f| &f.expr).collect();
                for (index, field) in fields.iter().enumerate() {
                    self.moved(field, evaluating(at, &fields, index + 1), out);
                    self.walk(field, at, out);
                }
                if let Some(rest) = &s.rest {
                    self.walk(rest, at, out);
                }
            }

            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.moved(elem, at, out);
                    self.walk(elem, at, out);
                }
            }

            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.moved(elem, at, out);
                    self.walk(elem, at, out);
                }
            }

            // A closure that moves a value out of its environment captured it
            // by value, `move` written or not — Rust has no other way to
            // compile the body. A `move` closure claims everything it mentions.
            syn::Expr::Closure(closure) => {
                let mut params = Vec::new();
                for input in &closure.inputs {
                    collect_pattern_names(input, &mut params);
                }
                self.shadowed.borrow_mut().push(params);
                if closure.capture.is_some() {
                    self.mentions(&closure.body, out);
                }
                self.walk(&closure.body, Where::Closure, out);
                self.tail(&closure.body, Where::Closure, out);
                self.shadowed.borrow_mut().pop();
            }

            // The branches of a `?:` and the operands of `&&`/`||` have nowhere
            // to put a statement. A move there is reported rather than flagged.
            syn::Expr::Binary(bin) => {
                let short_circuit =
                    matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_));
                let operand = if short_circuit { Where::Unwritable } else { at };
                // An overloaded operator is a method call whose impl takes its
                // operands by value, and the call releases them.
                let (left, right) = self.consumes.consumes_operands(bin);
                if left {
                    self.moved(&bin.left, at, out);
                }
                if right {
                    self.moved(&bin.right, operand, out);
                }
                self.walk(&bin.left, at, out);
                self.walk(&bin.right, operand, out);
            }

            syn::Expr::Unary(unary) => {
                if self.consumes.consumes_unary_operand(unary) {
                    self.moved(&unary.expr, at, out);
                }
                self.walk(&unary.expr, at, out);
            }

            syn::Expr::If(if_expr) => {
                self.walk(&if_expr.cond, at, out);
                self.nested_block(&if_expr.then_branch.stmts, at, out);
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.walk(else_expr, nested(at), out);
                    self.tail(else_expr, nested(at), out);
                }
            }

            syn::Expr::Match(m) => {
                if self.consumes.consumes_scrutinee(m) {
                    self.moved(&m.expr, at, out);
                }
                self.walk(&m.expr, at, out);
                for arm in &m.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.walk(guard, Where::Unwritable, out);
                    }
                    // `other => ..` takes the whole subject into its own name,
                    // and only where that arm runs: the block keeps a flag and
                    // releases the subject on the paths the other arms took.
                    if crate::ownership::scrutinee::binds_whole_subject(&arm.pat) {
                        self.moved(&m.expr, nested(at), out);
                    }
                    self.walk(&arm.body, nested(at), out);
                    self.tail(&arm.body, nested(at), out);
                }
            }

            syn::Expr::Block(block) => self.nested_block(&block.block.stmts, at, out),

            syn::Expr::ForLoop(for_loop) => {
                self.moved(&for_loop.expr, at, out);
                self.walk(&for_loop.expr, at, out);
                self.nested_block(&for_loop.body.stmts, at, out);
            }

            syn::Expr::While(w) => {
                self.walk(&w.cond, at, out);
                self.nested_block(&w.body.stmts, at, out);
            }

            syn::Expr::Loop(l) => self.nested_block(&l.body.stmts, at, out),

            // `if let Some(payload) = value` moves out of `value` exactly as
            // a `match` arm does, and the block that owned it stops owning it.
            syn::Expr::Let(let_expr) => {
                if self.consumes.consumes_let_scrutinee(let_expr) {
                    self.moved(&let_expr.expr, at, out);
                }
                self.walk(&let_expr.expr, at, out)
            }
            syn::Expr::Async(block) => self.nested_block(&block.block.stmts, at, out),
            syn::Expr::Unsafe(block) => self.nested_block(&block.block.stmts, at, out),

            syn::Expr::Try(t) => self.walk(&t.expr, at, out),
            syn::Expr::Paren(p) => self.walk(&p.expr, at, out),
            syn::Expr::Group(g) => self.walk(&g.expr, at, out),
            syn::Expr::Reference(r) => self.walk(&r.expr, at, out),
            syn::Expr::Cast(c) => self.walk(&c.expr, at, out),
            syn::Expr::Field(f) => self.walk(&f.base, at, out),
            syn::Expr::Index(i) => {
                self.walk(&i.expr, at, out);
                self.walk(&i.index, at, out);
            }
            syn::Expr::Range(r) => {
                if let Some(from) = &r.start {
                    self.walk(from, at, out);
                }
                if let Some(to) = &r.end {
                    self.walk(to, at, out);
                }
            }
            syn::Expr::Repeat(r) => self.walk(&r.expr, at, out),

            // A macro's arguments are Rust written in this body, and the
            // supported ones each take theirs the way an ordinary call would:
            // `vec![a, b]` by value, `format!`/`assert!`/`write!` by reference.
            // Leaving them out of the scan let `Bag { items: vec![value] }`
            // release `value` here and in the `Bag` that now held it.
            syn::Expr::Macro(mac) => self.macro_args(&mac.mac, at, out),

            _ => {}
        }
    }

    /// The moves a supported macro's arguments write.
    ///
    /// An unsupported macro's tokens are not Rust the emitter reads, so nothing
    /// is claimed about them; the translation of the macro is what reports it.
    pub(super) fn macro_args(&self, mac: &syn::Macro, at: Where, out: &mut Vec<Site>) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        // `select!` takes each arm's future by value and drops every one of
        // them when it returns, so the block that named one must not release it
        // as well.
        if name == "select" {
            for future in crate::macros::select_futures(&mac.tokens) {
                self.moved(&future, at, out);
                self.walk(&future, at, out);
            }
            return;
        }
        let by_value = match name.as_str() {
            "vec" => true,
            "format" | "println" | "eprintln" | "write" | "writeln" | "panic" | "unreachable"
            | "assert" | "debug_assert" | "assert_eq" | "assert_ne" => false,
            _ => return,
        };
        let parse = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let Ok(args) = parse.parse2(mac.tokens.clone()) else {
            return;
        };
        for arg in &args {
            if by_value {
                self.moved(arg, at, out);
            }
            self.walk(arg, at, out);
        }
    }
}
