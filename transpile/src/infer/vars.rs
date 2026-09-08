//! How a body reads a type the source wrote, and the unknowns it mints where
//! the source left one off.
//!
//! A `let` whose type only a later use decides, and a path written without its
//! type arguments, both stand for a type nothing has said yet. Each becomes a
//! variable here; constraints bind it, and what nothing binds is reported.

use super::context::TypeContext;
use crate::diag::Diag;
use crate::registry::{resolve_type, TypeEnv};
use crate::ty::{Mismatch, Ty};

impl TypeContext<'_> {
    /// The type with every unknown the solver has settled replaced by what it
    /// stands for.
    pub fn solved(&self, ty: &Ty) -> Ty {
        self.vars.borrow().resolve(ty)
    }

    /// Constrain two types to be the same. A shape the walk cannot reconcile is
    /// handed back for the caller to report at the site that asked.
    pub fn constrain(&self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
        self.vars.borrow_mut().unify(a, b)
    }

    /// Resolve a written type in this module, with the generics in scope.
    pub fn resolve_written_type(&self, ty: &syn::Type) -> Result<Ty, Diag> {
        resolve_type(ty, &self.type_env())
    }

    /// The same, minting an unknown for each type argument the
    /// source left off rather than refusing the path. `Vec::new()` writes no
    /// element type because the uses below it decide one.
    pub fn resolve_written_type_open(&self, ty: &syn::Type) -> Result<Ty, Diag> {
        resolve_type(ty, &self.type_env().with_vars(&self.vars))
    }

    fn type_env(&self) -> TypeEnv<'_> {
        TypeEnv::new(self.registry, self.module, self.sink)
            .with_params(&self.params)
            .with_self(self.self_ty.as_ref())
    }
}

impl TypeContext<'_> {
    /// Constrain each declared parameter against what the argument actually is,
    /// which is what settles an unknown the callee's own type carries.
    ///
    /// A closure argument is skipped: its parameters come FROM the bound at
    /// that position, so reading its type here would ask the question
    /// backwards.
    pub(super) fn constrain_arguments(&self, declared: &[Ty], args: &[&syn::Expr]) {
        for (want, arg) in declared.iter().zip(args) {
            if !want.mentions_any_var() {
                continue;
            }
            if self.constrain_callable(arg, want) {
                continue;
            }
            // A bound that is not callable says what the argument can DO, not
            // what it is: `impl IntoIterator` and a `Vec` are never the same
            // type, and settling the bound's own unknown needs a projection
            // through it rather than an equation with the argument.
            if matches!(
                want.peel_refs(),
                Ty::ImplTrait { .. } | Ty::Dyn { .. } | Ty::Assoc { .. }
            ) {
                continue;
            }
            // Speculative: the argument is being read for what it says about
            // the callee, and what it cannot say is not this site's failure.
            let mark = self.sink.mark();
            let actual = self.resolve_expr(arg);
            self.sink.rewind(mark);
            let Ok(actual) = actual else { continue };
            // Emission erases `&`, so an argument stands for what it refers to.
            if let Err(mismatch) = self.constrain(want.peel_refs(), actual.peel_refs()) {
                self.report_mismatch(syn::spanned::Spanned::span(*arg), want, &actual, &mismatch);
            }
        }
    }

    /// Constrain a CALLABLE position by what the value there answers when it is
    /// called, and say whether this position was one.
    ///
    /// `Calculated::new(move || a.get() + b.get())` declares
    /// `fn new<F>(compute: F) -> Self where F: Fn() -> T`: the bound's inputs
    /// say what the closure's parameters are, and the closure's tail is the only
    /// thing that says what `T` is. The two TYPES are never equal — a bound is a
    /// capability and `impl Fn + Send + Sync` is not `impl Fn` — so the
    /// constraint is between what each of them answers.
    fn constrain_callable(&self, arg: &syn::Expr, want: &Ty) -> bool {
        let probe = self.probe();
        let Some(shape) =
            super::expected::fn_shape_through_impls(&probe, self.registry, want, &self.param_bounds)
        else {
            return false;
        };
        if !shape.output.mentions_any_var() {
            return true;
        }
        // Speculative: what the argument cannot say is not this site's failure.
        let mark = self.sink.mark();
        let answered = match super::calls::unparenthesise(arg) {
            syn::Expr::Closure(closure) => self.closure_signature(closure, Some(want)).ret,
            other => self.resolve_expr(other).ok().and_then(|ty| {
                super::expected::fn_shape_through_impls(
                    &probe,
                    self.registry,
                    &ty,
                    &self.param_bounds,
                )
                .map(|found| found.output)
            }),
        };
        self.sink.rewind(mark);
        let Some(answered) = answered else { return true };
        if let Err(mismatch) = self.constrain(&shape.output, answered.peel_refs()) {
            self.report_mismatch(
                syn::spanned::Spanned::span(arg),
                &shape.output,
                &answered,
                &mismatch,
            );
        }
        true
    }

    /// Say that a constraint has no solution, naming both types. Neither side
    /// is chosen: a type the engine cannot decide is one it reports.
    pub(super) fn report_mismatch(
        &self,
        span: proc_macro2::Span,
        want: &Ty,
        found: &Ty,
        mismatch: &Mismatch,
    ) {
        self.sink.report(
            span,
            format!(
                "`{}` and `{}` are constrained to be the same type here and cannot be: {}",
                self.registry.describe(&self.solved(want)),
                self.registry.describe(&self.solved(found)),
                mismatch
            ),
        );
    }
}
