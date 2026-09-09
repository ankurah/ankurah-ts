//! How a body reads a type the source wrote, and the unknowns it mints where
//! the source left one off.
//!
//! A `let` whose type only a later use decides, and a path written without its
//! type arguments, both stand for a type nothing has said yet. Each becomes a
//! variable here; constraints bind it, and what nothing binds is reported.

use syn::spanned::Spanned;

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
            // A callable position and a bound carrying an associated type are
            // read even when they carry no unknown of their own: what they
            // require is what types the closure standing there, and what they
            // project is what settles an unknown inside the ARGUMENT.
            let closure = matches!(super::calls::unparenthesise(arg), syn::Expr::Closure(_));
            let bound = matches!(want.peel_refs(), Ty::ImplTrait { .. } | Ty::Dyn { .. });
            if !closure && !bound && !want.mentions_any_var() {
                continue;
            }
            if self.constrain_callable(arg, want) {
                continue;
            }
            // A bound that is not callable says what the argument can DO, not
            // what it is: `impl IntoIterator` and a `Vec` are never the same
            // type. What the two agree on is what the bound PROJECTS.
            if bound {
                self.constrain_through_bound(arg, want.peel_refs());
                continue;
            }
            if !want.mentions_any_var() {
                continue;
            }
            let Some(actual) = self.actual_of(arg) else { continue };
            // Emission erases `&`, so an argument stands for what it refers to.
            self.constrain_here(Spanned::span(*arg), want.peel_refs(), actual.peel_refs());
        }
    }

    /// Constrain what a bound PROJECTS through the type standing at that
    /// position, rather than the bound itself.
    ///
    /// `ReadyChunks::new` declares `I: IntoIterator<Item = F>`; handed a
    /// `Vec<Receiver<i32>>` it says `F` is the item the impl table reads out of
    /// a `Vec<Receiver<i32>>`. The bound and the argument are never the same
    /// type, and this is the whole of what they say about each other.
    fn constrain_through_bound(&self, arg: &syn::Expr, want: &Ty) {
        let (Ty::ImplTrait { bounds } | Ty::Dyn { traits: bounds }) = want else { return };
        if bounds.iter().all(|bound| bound.bindings.is_empty()) {
            return;
        }
        let Some(actual) = self.actual_of(arg) else { return };
        let actual = self.solved(&actual);
        for bound in bounds {
            let trait_ref = crate::ty::TraitRef {
                id: bound.id,
                args: bound.args.clone(),
                bindings: Vec::new(),
            };
            for (name, declared) in &bound.bindings {
                let Some(found) = self.project_with(actual.peel_refs(), trait_ref.clone(), name)
                else {
                    continue;
                };
                self.constrain_here(Spanned::span(arg), declared, &found);
            }
        }
    }

    /// Constrain two types at one written site, saying so where they cannot be
    /// reconciled.
    pub(super) fn constrain_here(&self, span: proc_macro2::Span, want: &Ty, actual: &Ty) {
        if !want.mentions_any_var() && !actual.mentions_any_var() {
            return;
        }
        // An expression that never returns stands at every type, so it says
        // nothing about the one the position wanted.
        if *want == Ty::Never || *actual == Ty::Never {
            return;
        }
        if let Err(mismatch) = self.constrain(want, actual) {
            // Two shapes that will not meet are a contradiction only where the
            // engine has read both: `I::IntoIter::Item` IS `F` wherever a
            // `where` clause says so, and the impl table reads that clause only
            // for a type something has instantiated.
            if self.solved(want).mentions_projection() || self.solved(actual).mentions_projection()
            {
                return;
            }
            self.report_mismatch(span, want, actual, &mismatch);
        }
    }

    /// The unknown standing at one written site, minted on the first ask and
    /// the same one after — which is what lets two walks over one body agree
    /// about what an unnamed type stands for.
    pub(super) fn var_at(&self, span: proc_macro2::Span, index: usize) -> Ty {
        let at = span.start();
        Ty::Var(self.vars.borrow_mut().at_site(at.line, at.column, index))
    }

    /// What an argument is, asked for what it says about the callee. What it
    /// cannot say is not this site's failure, so the question is asked quietly.
    pub(super) fn actual_of(&self, arg: &syn::Expr) -> Option<Ty> {
        let mark = self.sink.mark();
        let found = self.resolve_expr(arg).ok();
        self.sink.rewind(mark);
        found
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
        // The closure's parameters are this bound's inputs, and the walk that
        // types the closure's own body meets it after this call is read.
        if let syn::Expr::Closure(closure) = super::calls::unparenthesise(arg) {
            self.closure_wants
                .borrow_mut()
                .insert(crate::body::span_position(Spanned::span(closure)), want.clone());
        }
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
        self.constrain_here(Spanned::span(arg), &shape.output, answered.peel_refs());
        true
    }

    /// Record what each argument position of a free call requires, so that a
    /// closure standing at one is typed by the bound rather than by nothing.
    ///
    /// A free call answers with its declared return type and never reads its
    /// parameters, so this is the only place that asks them.
    pub(super) fn note_closure_positions(&self, call: &syn::ExprCall) {
        let closures = call
            .args
            .iter()
            .any(|arg| matches!(super::calls::unparenthesise(arg), syn::Expr::Closure(_)));
        if !closures {
            return;
        }
        // Speculative: a callee this body cannot read is not this site's
        // failure, and the call itself reports what it could not resolve.
        let mark = self.sink.mark();
        let declared = self.call_argument_types_of(call, None).unwrap_or_default();
        self.sink.rewind(mark);
        for (want, arg) in declared.iter().zip(call.args.iter()) {
            let Some(want) = want else { continue };
            if matches!(super::calls::unparenthesise(arg), syn::Expr::Closure(_)) {
                self.constrain_callable(arg, want);
            }
        }
    }

    /// What the position this closure stands in requires of it, if a call has
    /// already read that position.
    pub(super) fn closure_want(&self, closure: &syn::ExprClosure) -> Option<Ty> {
        let at = crate::body::span_position(Spanned::span(closure));
        self.closure_wants.borrow().get(&at).cloned()
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
