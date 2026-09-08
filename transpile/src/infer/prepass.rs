//! One walk over a whole function body, before any of it is written.
//!
//! For: a local's type is often decided BELOW its own statement — an empty
//! `Vec` by a later `push`, a generic constructor by the closure handed to it.
//! Typing and emission are one walk, so this walk runs first, binds names as it
//! meets them, and lets every call it passes constrain the body's unknowns. The
//! emission walk then asks the same questions at the same positions and reads
//! the answers, because a variable is keyed by where it is written.

use syn::visit::Visit;

use super::context::TypeContext;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// Collect what the whole body says about its unknowns, leaving the scopes
    /// as they were found.
    ///
    /// Quiet: what this walk cannot type, the emission walk reports where it
    /// stands, and reporting here would say all of it twice.
    pub fn collect_constraints(&mut self, block: &syn::Block) {
        let mark = self.sink.mark();
        // A constraint that binds an unknown late lets one that could not run
        // before it run now — method resolution cannot start from a receiver
        // nothing has bound — so the walk repeats until a round binds nothing
        // new. It ends because a binding is never revised: every round but the
        // last binds at least one more variable.
        let mut rounds = 0usize;
        let limit = loop {
            let before = self.vars.borrow().bound_count();
            crate::trace::without_recording(|| Prepass { tc: self }.visit_block(block));
            rounds += 1;
            if self.vars.borrow().bound_count() == before {
                break None;
            }
            if rounds > self.vars.borrow().len() {
                break Some(rounds);
            }
        };
        self.sink.rewind(mark);
        // Past one round per variable the table has stopped being monotone,
        // which is a defect in the solver rather than a gap in the body.
        if let Some(rounds) = limit {
            self.sink.report(
                syn::spanned::Spanned::span(block),
                format!("the types of this body did not settle in {} rounds", rounds),
            );
        }
    }
}

struct Prepass<'a, 'b> {
    tc: &'a mut TypeContext<'b>,
}

impl Prepass<'_, '_> {
    /// The type of an expression, asked for its constraints alone.
    fn type_of(&mut self, expr: &syn::Expr) -> Option<Ty> {
        self.tc.resolve_expr(expr).ok()
    }

    /// A scope that holds only what a pattern binds, for the arm or the branch
    /// that pattern guards.
    fn in_pattern_scope(&mut self, pat: &syn::Pat, ty: Option<&Ty>, walk: impl FnOnce(&mut Self)) {
        self.tc.scopes.push_block();
        self.tc.bind_pattern(pat, ty);
        walk(self);
        self.tc.scopes.pop();
    }
}

impl<'ast> Visit<'ast> for Prepass<'_, '_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.tc.scopes.push_block();
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.tc.scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some(diverge) = &init.diverge {
                self.visit_expr(&diverge.1);
            }
        }
        let ty = self.tc.resolve_local_type(local).ok();
        self.tc.bind_pattern(&local.pat, ty.as_ref());
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        match expr {
            // A `let` in a condition binds for the branch it guards and not
            // beyond it, so the branch opens the scope and the condition binds
            // into it.
            syn::Expr::If(if_expr) => {
                self.tc.scopes.push_block();
                self.visit_expr(&if_expr.cond);
                self.visit_block(&if_expr.then_branch);
                self.tc.scopes.pop();
                if let Some((_, other)) = &if_expr.else_branch {
                    self.visit_expr(other);
                }
            }
            syn::Expr::While(while_expr) => {
                self.tc.scopes.push_block();
                self.visit_expr(&while_expr.cond);
                self.visit_block(&while_expr.body);
                self.tc.scopes.pop();
            }
            syn::Expr::Let(let_expr) => {
                let ty = self.type_of(&let_expr.expr);
                self.tc.bind_pattern(&let_expr.pat, ty.as_ref());
            }
            // `for (entity, event) in entity_events` types both names from the
            // sequence's item, which is a projection through `IntoIterator`.
            syn::Expr::ForLoop(for_loop) => {
                let item = self
                    .type_of(&for_loop.expr)
                    .and_then(|seq| self.tc.iteration_item(&seq));
                self.visit_expr(&for_loop.expr);
                self.in_pattern_scope(&for_loop.pat, item.as_ref(), |pass| {
                    pass.visit_block(&for_loop.body)
                });
            }
            syn::Expr::Match(match_expr) => {
                let scrutinee = self.type_of(&match_expr.expr);
                self.visit_expr(&match_expr.expr);
                for arm in &match_expr.arms {
                    self.in_pattern_scope(&arm.pat, scrutinee.as_ref(), |pass| {
                        if let Some((_, guard)) = &arm.guard {
                            pass.visit_expr(guard);
                        }
                        pass.visit_expr(&arm.body);
                    });
                }
            }
            // A closure's parameters come from the position it stands in, which
            // the call that carries it has already read.
            syn::Expr::Closure(closure) => {
                let bindings = self.tc.closure_signature(closure, None).bindings;
                self.tc.scopes.push_closure(Vec::new());
                for (name, ty) in bindings {
                    match ty {
                        Some(ty) => self.tc.scopes.bind(name, ty),
                        // In scope without a type, so a use of the name reads
                        // as the parameter it is rather than an outer binding
                        // that happens to share it.
                        None => self.tc.scopes.bind_untyped(name),
                    }
                }
                self.visit_expr(&closure.body);
                self.tc.scopes.pop();
            }
            other => {
                self.type_of(other);
                syn::visit::visit_expr(self, other);
            }
        }
    }

    /// An item inside a body declares its own names in its own body, and none
    /// of it is this function's.
    fn visit_item(&mut self, _item: &'ast syn::Item) {}
}
