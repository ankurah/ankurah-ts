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
    pub fn collect_constraints(&mut self, block: &syn::Block, returns: Option<&Ty>) {
        let mark = self.sink.mark();
        self.vars.borrow_mut().set_solving(true);
        // What the BODY says about an argument the source left off is Rust's
        // answer, so it is collected before the default falls back into place —
        // and a method that could not resolve through an open unknown now can.
        let limit = self.to_a_fixed_point(block, returns);
        self.settle_defaults();
        let after = self.to_a_fixed_point(block, returns);
        self.settle_defaults();
        // Rust's own last step: an unsuffixed literal nothing else decided
        // takes `i32` or `f64`, and it is the only type this engine defaults.
        // One more round follows, because a receiver that was an open literal
        // is a receiver whose methods only now resolve.
        self.vars.borrow_mut().settle_kinds();
        let settled = self.to_a_fixed_point(block, returns);
        self.vars.borrow_mut().set_solving(false);
        let limit = limit.or(after).or(settled);
        self.sink.rewind(mark);
        // A constraint with no solution is what this walk KNOWS rather than
        // what it could not do, so it outlives the rewind: re-checked against
        // the table the solve left, and said once at the site that asked.
        self.settle_contradictions();
        // Past one round per variable the table has stopped being monotone,
        // which is a defect in the solver rather than a gap in the body.
        if let Some(rounds) = limit {
            self.sink.report(
                syn::spanned::Spanned::span(block),
                format!("the types of this body did not settle in {} rounds", rounds),
            );
        }
    }

    /// Walk the body until a round binds nothing new, saying how many rounds it
    /// took where that is more than one per variable.
    ///
    /// A constraint that binds an unknown late lets one that could not run
    /// before it run now — method resolution cannot start from a receiver
    /// nothing has bound. It ends because a binding is never revised: every
    /// round but the last binds at least one more variable.
    fn to_a_fixed_point(&mut self, block: &syn::Block, returns: Option<&Ty>) -> Option<usize> {
        let mut rounds = 0usize;
        loop {
            let before = self.vars.borrow().bound_count();
            crate::trace::without_recording(|| {
                Prepass {
                    tc: self,
                    returns: returns.cloned(),
                    at_body: true,
                }
                .visit_block(block)
            });
            rounds += 1;
            if self.vars.borrow().bound_count() == before {
                return None;
            }
            if rounds > self.vars.borrow().len() {
                return Some(rounds);
            }
        }
    }
}

struct Prepass<'a, 'b> {
    tc: &'a mut TypeContext<'b>,
    /// What the body being walked answers with: the function's declared return
    /// type, or the closure's own while its body is walked.
    returns: Option<Ty>,
    /// Is the next block the function's own? Its tail is what the function
    /// answers with; a nested block's is not.
    at_body: bool,
}

impl Prepass<'_, '_> {
    /// The type of an expression, asked for its constraints alone.
    fn type_of(&mut self, expr: &syn::Expr) -> Option<Ty> {
        self.tc.resolve_expr(expr).ok()
    }

    /// Constrain what this expression is against what the body answers with.
    fn answer_with(&mut self, expr: &syn::Expr) {
        let (Some(returns), Some(found)) = (self.returns.clone(), self.type_of(expr)) else {
            return;
        };
        self.tc
            .constrain_here(syn::spanned::Spanned::span(expr), &returns, &found);
    }

    /// Constrain what is being matched against what the pattern can only be
    /// matching.
    fn against_pattern(&mut self, scrutinee: &syn::Expr, ty: Option<&Ty>, pat: &syn::Pat) {
        let (Some(ty), Some(shape)) = (ty, self.tc.pattern_shape(pat)) else {
            return;
        };
        self.tc
            .constrain_here(syn::spanned::Spanned::span(scrutinee), ty, &shape);
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
        let body = std::mem::take(&mut self.at_body);
        self.tc.scopes.push_block();
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        // The function's tail is what it answers with, so it says as much about
        // an unknown inside it as a `return` does.
        if body {
            if let Some(syn::Stmt::Expr(tail, None)) = block.stmts.last() {
                self.answer_with(tail);
            }
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
            syn::Expr::Return(ret) => {
                syn::visit::visit_expr(self, expr);
                if let Some(value) = &ret.expr {
                    self.answer_with(value);
                }
            }
            // A place and the value written into it are one type: `self.head =
            // found` says what `found` is where the field is declared, and what
            // the field holds where the value is known.
            syn::Expr::Assign(assign) => {
                syn::visit::visit_expr(self, expr);
                let (Some(place), Some(value)) =
                    (self.type_of(&assign.left), self.type_of(&assign.right))
                else {
                    return;
                };
                self.tc.constrain_here(
                    syn::spanned::Spanned::span(&assign.right),
                    &place,
                    &value,
                );
            }
            syn::Expr::Let(let_expr) => {
                let ty = self.type_of(&let_expr.expr);
                self.against_pattern(&let_expr.expr, ty.as_ref(), &let_expr.pat);
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
                    self.against_pattern(&match_expr.expr, scrutinee.as_ref(), &arm.pat);
                    self.in_pattern_scope(&arm.pat, scrutinee.as_ref(), |pass| {
                        if let Some((_, guard)) = &arm.guard {
                            pass.visit_expr(guard);
                        }
                        pass.visit_expr(&arm.body);
                    });
                }
            }
            // A closure's parameters come from the position it stands in, which
            // the call that carries it has already read and recorded.
            syn::Expr::Closure(closure) => {
                let want = self.tc.closure_want(closure);
                let signature = self.tc.closure_signature(closure, want.as_ref());
                let bindings = signature.bindings;
                // A `return` inside a closure leaves through the closure's own
                // result, not the enclosing function's.
                let outer = std::mem::replace(&mut self.returns, signature.ret);
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
                self.returns = outer;
            }
            other => {
                self.type_of(other);
                syn::visit::visit_expr(self, other);
            }
        }
    }

    /// A `const` written inside a body binds a name the statements below it
    /// read, and its annotation is what types that name. A nested `fn` or
    /// `impl` declares its own names in its own body, and none of it is this
    /// function's.
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let syn::Item::Const(c) = item else { return };
        let Ok(ty) = self.tc.resolve_written_type(&c.ty) else { return };
        self.tc.bind(&c.ident.to_string(), ty);
    }
}
