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
use crate::ty::{Contradiction, Mismatch, Ty, VarKind};

impl TypeContext<'_> {
    /// The type with every unknown the solver has settled replaced by what it
    /// stands for.
    pub fn solved(&self, ty: &Ty) -> Ty {
        self.vars.borrow().resolve(ty)
    }

    /// Constrain two types to be the same. A shape the walk cannot reconcile is
    /// handed back for the caller to report at the site that asked.
    ///
    /// Only the constraint walk binds. The walk that WRITES the body asks the
    /// same question of a scratch copy, so it still says where two types cannot
    /// meet while the table it reads stays as the solve left it.
    pub fn constrain(&self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
        if !self.vars.borrow().solving() {
            let mut scratch = self.vars.borrow().clone();
            let before = scratch.bindings_made();
            let answer = scratch.unify(a, b);
            let made = scratch.bindings_made() - before;
            self.vars.borrow_mut().note_scratch_bindings(made);
            return answer;
        }
        self.vars.borrow_mut().unify(a, b)
    }

    /// How many bindings the walk that writes this body would have made if it
    /// bound. It reads a solved table, so the answer is zero.
    pub fn scratch_bindings(&self) -> usize {
        self.vars.borrow().scratch_bindings()
    }

    /// Fall back to the written default of every argument the source left off,
    /// LAST: what the body says about such an argument is Rust's answer, and
    /// the declaration's default is what stands where the body said nothing.
    pub(super) fn settle_defaults(&self) {
        let deferred = self.vars.borrow().deferred_defaults();
        for (var, default, span) in deferred {
            let unknown = Ty::Var(var);
            let solved = self.solved(&unknown);
            if solved == unknown {
                let _ = self.constrain(&unknown, &default);
                continue;
            }
            // The body decided it, and the declaration says otherwise: neither
            // is chosen and the site says both.
            if let Err(mismatch) = self.vars.borrow().clone().unify(&solved, &default) {
                self.report_mismatch(span, &default, &solved, &mismatch);
            }
        }
    }

    /// How many of this body's unknowns stand for a type. An unknown minted to
    /// stand for nothing is not counted: a refusal answers with one, and the
    /// walk that writes a body may meet a refusal the solve never reached.
    pub fn bound_unknowns(&self) -> usize {
        self.vars.borrow().bindings_made()
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
    pub(super) fn constrain_arguments(&self, declared: &[Ty], args: &[&syn::Expr]) {
        // An adaptor writes its bound in its own terms — `FnMut(Self::Item)` —
        // and a projection left standing types nothing inside the closure, so
        // the receiver settles it before the position is read.
        let probe = self.probe();
        for (want, arg) in declared.iter().zip(args) {
            let want = &probe.normalize(want);
            // A callable position and a bound carrying an associated type are
            // read even when they carry no unknown of their own: what they
            // require is what types the closure standing there, and what they
            // project is what settles an unknown inside the ARGUMENT.
            let closure = super::calls::as_closure(arg).is_some();
            let bound = matches!(want.peel_refs(), Ty::ImplTrait { .. } | Ty::Dyn { .. });
            if closure || bound || want.mentions_any_var() {
                if self.constrain_callable(arg, want) {
                    continue;
                }
                // A bound that is not callable says what the argument can DO,
                // not what it is: `impl IntoIterator` and a `Vec` are never the
                // same type. What the two agree on is what the bound PROJECTS.
                if bound {
                    self.constrain_through_bound(arg, want.peel_refs());
                    continue;
                }
            }
            // An unknown on EITHER side is one this call settles: `let mut v =
            // Vec::new(); Filler::fill(&mut v)` decides `v`'s element at a
            // parameter whose own type is written out in full.
            let Some(actual) = self.argument_type(arg) else { continue };
            self.constrain_here(Spanned::span(*arg), want, &actual);
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
                // Through the type AS WRITTEN first: `&Vec<Tag>` iterates
                // `&Tag`, and reading the by-value impl instead binds the
                // caller's own values to a collection that releases them.
                let Some(found) = self
                    .project_with(&actual, trait_ref.clone(), name)
                    .or_else(|| self.project_with(actual.peel_refs(), trait_ref.clone(), name))
                else {
                    continue;
                };
                self.constrain_here(Spanned::span(arg), declared, &found);
            }
        }
    }

    /// Constrain two types at one written site, saying so where they cannot be
    /// reconciled.
    ///
    /// The value KEEPS every `&` it carries — a borrow bound to a by-value
    /// parameter is what that parameter holds. A `&` the declared side has over
    /// a value carrying none is the borrow the engine reads that value through.
    pub(super) fn constrain_here(&self, span: proc_macro2::Span, want: &Ty, actual: &Ty) {
        self.constrain_at(span, want, actual, true)
    }

    /// The same, saying whether Rust would coerce here. It coerces where a
    /// value MEETS a declared type and nowhere inside one, so the element
    /// recursion below asks with `false`.
    fn constrain_at(
        &self,
        span: proc_macro2::Span,
        want: &Ty,
        actual: &Ty,
        at_a_coercion_site: bool,
    ) {
        // ONE declared `&` over a value carrying none is the borrow the engine
        // reads that value through. The VALUE keeps every `&` it carries: a
        // borrow bound to a by-value parameter is what that parameter holds.
        let through_the_borrow = match want {
            Ty::Ref { inner, .. } if !matches!(actual, Ty::Ref { .. }) => &**inner,
            _ => want,
        };
        // A type parameter this body does not have belongs to the callee, and
        // names one type per call it makes rather than one the body can bind.
        if super::expected::holds_unbound(want, &self.params)
            || super::expected::holds_unbound(actual, &self.params)
        {
            return;
        }
        // An expression that never returns stands at every type, so it says
        // nothing about the one the position wanted.
        if *want == Ty::Never || *actual == Ty::Never {
            return;
        }
        if let Err(mismatch) = self.constrain(through_the_borrow, actual) {
            // `&Vec<T>` stands where `&[T]` is declared: Rust derefs one into
            // the other and the port writes both as one array, so what the two
            // say about each other is their ELEMENT.
            if let (Some(a), Some(b)) =
                (self.sequence_element(through_the_borrow), self.sequence_element(actual))
            {
                self.constrain_at(span, &a, &b, false);
                return;
            }
            // Only the solve reports. The walk that WRITES a body types
            // expressions the solve never reached, with every fallback in
            // place, so a disagreement it finds is the engine's own reading.
            if !self.vars.borrow().solving() {
                return;
            }
            if !self.disagrees(
                &self.probe(),
                &self.solved(want),
                &self.solved(actual),
                at_a_coercion_site,
            ) {
                return;
            }
            // A width the fallback chose stands for what nothing decided, so a
            // constraint it fails is the engine disagreeing with its own guess.
            let vars = self.vars.borrow();
            if vars.mentions_defaulted(want) || vars.mentions_defaulted(actual) {
                return;
            }
            drop(vars);
            // The collecting walk is quiet, so its report would be thrown away:
            // the table keeps the contradiction and the end of the solve says
            // it.
            self.vars.borrow_mut().record_contradiction(Contradiction {
                span,
                want: want.clone(),
                found: actual.clone(),
                mismatch,
                through: self.constraining_through.borrow().clone(),
                coerces: at_a_coercion_site,
            });
        }
    }

    /// Re-check every constraint the solve could not meet, poison what each one
    /// touched and say it once.
    ///
    /// A constraint that failed in an early round may be met by a later one, so
    /// only what still has no solution at the fixed point is a fact about the
    /// body; what it touched then stands for nothing, and the local that held
    /// it takes the untyped path.
    pub(super) fn settle_contradictions(&self) {
        let mut said = std::collections::HashSet::new();
        let collected = self.vars.borrow_mut().take_contradictions();
        for found in collected {
            let want = self.solved(&found.want);
            let actual = self.solved(&found.found);
            if self.vars.borrow().clone().unify(&want, &actual).is_ok() {
                continue;
            }
            if !self.disagrees(&self.probe(), &want, &actual, found.coerces) {
                continue;
            }
            // A type still carrying an unknown is one the engine did not
            // finish reading, and what it could not read is no evidence: the
            // unknown is reported where it was bound.
            if want.mentions_any_var() || actual.mentions_any_var() {
                continue;
            }
            let message = self.mismatch_message(&want, &actual, &found.mismatch);
            if !said.insert((crate::body::span_position(found.span), message.clone())) {
                continue;
            }
            self.vars.borrow_mut().poison(&found.want);
            self.vars.borrow_mut().poison(&found.found);
            if let Some(through) = &found.through {
                self.vars.borrow_mut().poison(through);
            }
            self.sink.report(found.span, message);
        }
    }

    /// What a sequence holds: a slice's, an array's, a `Vec`'s or a boxed
    /// slice's element, all of which the port writes as one array.
    fn sequence_element(&self, ty: &Ty) -> Option<Ty> {
        match self.solved(ty).peel_refs() {
            Ty::Slice(elem) | Ty::Array { elem, .. } => Some((**elem).clone()),
            Ty::Named { id, args } => {
                let is = |path| self.registry.system_type(path) == Some(*id);
                match (is("std::vec::Vec"), is("std::boxed::Box"), args.first()?) {
                    (true, _, Ty::Slice(elem)) | (_, true, Ty::Slice(elem)) => {
                        Some((**elem).clone())
                    }
                    (true, _, elem) => Some(elem.clone()),
                    // A `Box` is a sequence only of what it BOXES a slice of.
                    // Read as one of anything, `&Box::new(2u8)` stood where a
                    // slice of `u8` was declared and nothing said otherwise.
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The unknown standing at one written site, minted on the first ask and
    /// the same one after — which is what lets two walks over one body agree
    /// about what an unnamed type stands for.
    pub(super) fn var_at(&self, span: proc_macro2::Span, index: usize) -> Ty {
        let at = span.start();
        Ty::Var(self.vars.borrow_mut().at_site(at.line, at.column, index))
    }

    /// What a refusal contributes: nothing. An unknown standing for nothing,
    /// found again at the same site, so no constraint binds through the gap
    /// and every spelling path reads it as unknown.
    pub(super) fn unresolvable(&self, span: proc_macro2::Span) -> Ty {
        let at = span.start();
        Ty::Var(self.vars.borrow_mut().unresolvable_at_site(at.line, at.column))
    }

    /// The unknown standing for an unsuffixed literal, restricted to the kind
    /// Rust gives it. Minted while the solve runs, and afterwards found again
    /// at the site, so both walks over a body read one answer.
    pub(super) fn literal_var(&self, span: proc_macro2::Span, kind: VarKind) -> Option<Ty> {
        let at = span.start();
        let mut vars = self.vars.borrow_mut();
        if vars.solving() {
            return Some(Ty::Var(vars.kinded_at_site(at.line, at.column, 0, kind)));
        }
        vars.site(at.line, at.column, 0).map(Ty::Var)
    }

    /// What an argument is, with the reference the expression itself writes: a
    /// borrow handed to a by-value parameter is what that parameter holds, so
    /// `picked.push(t)` over a `&Thing` makes `picked` a `Vec<&Thing>`.
    fn argument_type(&self, arg: &syn::Expr) -> Option<Ty> {
        match super::calls::unparenthesise(arg) {
            syn::Expr::Reference(taken) => Some(Ty::Ref {
                mutable: taken.mutability.is_some(),
                inner: Box::new(self.argument_type(&taken.expr)?),
            }),
            other => self.actual_of(other),
        }
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
        if let Some(closure) = super::calls::as_closure(arg) {
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
        self.constrain_here(Spanned::span(arg), &shape.output, &answered);
        true
    }

    /// Constrain what a callable position projects against what the closure
    /// standing there answers, asked where the closure's captures are bound.
    ///
    /// `Calculated::new({ let base = base.read(); move || base.get() * 2 })`
    /// says `T` is an `i32` only from inside the block: at the call, `base`
    /// names nothing and the body has no type.
    pub(super) fn constrain_callable_result(
        &self,
        closure: &syn::ExprClosure,
        want: &Ty,
        answered: Option<&Ty>,
    ) {
        let Some(answered) = answered else { return };
        let probe = self.probe();
        let Some(shape) =
            super::expected::fn_shape_through_impls(&probe, self.registry, want, &self.param_bounds)
        else {
            return;
        };
        if !shape.output.mentions_any_var() {
            return;
        }
        self.constrain_here(Spanned::span(closure), &shape.output, answered);
    }

    /// Constrain each argument of a FREE call against what its parameter is
    /// declared to be. A free call answers with its declared return type, so
    /// this is the only place its parameters are read.
    pub(super) fn constrain_free_arguments(&self, call: &syn::ExprCall) {
        // Speculative: a callee this body cannot read is not this site's
        // failure, and the call itself reports what it could not resolve.
        let mark = self.sink.mark();
        let declared = self.call_argument_types_of(call, None).unwrap_or_default();
        self.sink.rewind(mark);
        if declared.iter().all(Option::is_none) {
            return;
        }
        let args: Vec<&syn::Expr> = call.args.iter().collect();
        let want: Vec<Ty> = declared
            .iter()
            .map(|ty| ty.clone().unwrap_or(Ty::Infer))
            .collect();
        self.constrain_arguments(&want, &args);
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
        let message = self.mismatch_message(&self.solved(want), &self.solved(found), mismatch);
        self.sink.report(span, message);
    }

    /// The wording of that refusal, over types the caller has already solved.
    fn mismatch_message(&self, want: &Ty, found: &Ty, mismatch: &Mismatch) -> String {
        format!(
            "`{}` and `{}` are constrained to be the same type here and cannot be: {}",
            self.registry.describe(want),
            self.registry.describe(found),
            mismatch
        )
    }
}

impl TypeContext<'_> {
    /// Do two SOLVED types disagree about something the engine has READ?
    ///
    /// A projection still standing in a solved type is one the impl table has
    /// not read, and it may yet BE the type beside it, so that component is
    /// evidence neither way. Every other component is compared, so a concrete
    /// neighbour of an unread projection still contradicts and still reports.
    /// `at_a_coercion_site` says whether Rust would coerce here. It does so
    /// where a value MEETS a declared type — an argument, an annotation, a
    /// return, a field — and nowhere inside one: `Vec<Box<Expr>>` handed where
    /// `Vec<Expr>` is declared is a program Rust rejects.
    fn disagrees(
        &self,
        probe: &crate::registry::Probe<'_>,
        want: &Ty,
        found: &Ty,
        at_a_coercion_site: bool,
    ) -> bool {
        let (want, found) = match at_a_coercion_site {
            true => aligned_references(want, found),
            // Inside a type argument nothing is coerced, and two references of
            // the same depth say only what is under them.
            false => match (want, found) {
                (Ty::Ref { inner: a, .. }, Ty::Ref { inner: b, .. }) => (&**a, &**b),
                (a, b) => (a, b),
            },
        };
        let differ = match (want, found) {
            // Nothing the engine has read: a projection, an unknown, the
            // source's own hole, a parameter awaiting its instantiation, and a
            // bound beside a type that merely meets it.
            (Ty::Assoc { .. } | Ty::Var(_) | Ty::Infer | Ty::Param(_) | Ty::Never, _)
            | (_, Ty::Assoc { .. } | Ty::Var(_) | Ty::Infer | Ty::Param(_) | Ty::Never)
            | (Ty::Dyn { .. } | Ty::ImplTrait { .. }, _)
            | (_, Ty::Dyn { .. } | Ty::ImplTrait { .. }) => return false,
            (Ty::Named { id: a, args: xs }, Ty::Named { id: b, args: ys }) => {
                a != b || xs.len() != ys.len() || self.any_disagrees(probe, xs, ys)
            }
            (Ty::Tuple(xs), Ty::Tuple(ys)) => {
                xs.len() != ys.len() || self.any_disagrees(probe, xs, ys)
            }
            (Ty::Prim(a), Ty::Prim(b)) => a != b,
            (Ty::Str, Ty::Str) | (Ty::Unit, Ty::Unit) => false,
            // Two sequences: which container each side names is the deref the
            // port writes as one array, so only the element is evidence.
            // A slice and a sequence that derefs to it are one array in the
            // port, so only their ELEMENTS are evidence. A slice beside
            // something that is not a sequence at all is not that array.
            (Ty::Slice(a) | Ty::Array { elem: a, .. }, other) => match self.sequence_element(other) {
                Some(b) => self.disagrees(probe, a, &b, false),
                None => true,
            },
            (other, Ty::Slice(b) | Ty::Array { elem: b, .. }) => match self.sequence_element(other) {
                Some(a) => self.disagrees(probe, &a, b, false),
                None => true,
            },
            _ => true,
        };
        // `&String` stands where `&str` is declared and `Box<Expr>` where
        // `Expr` is: Rust derefs one into the other and the port writes both as
        // one value, so the two do not disagree about the program.
        if !at_a_coercion_site {
            return differ;
        }
        differ && !self.reaches(probe, want, found) && !self.reaches(probe, found, want)
    }

    fn any_disagrees(&self, probe: &crate::registry::Probe<'_>, xs: &[Ty], ys: &[Ty]) -> bool {
        xs.iter().zip(ys).any(|(x, y)| self.disagrees(probe, x, y, false))
    }

    /// Does one type reach the other along the deref chain?
    fn reaches(&self, probe: &crate::registry::Probe<'_>, from: &Ty, to: &Ty) -> bool {
        // A reference layer is not a `Deref` impl: the alignment rule discounts
        // exactly one, and taking another here would let `&&T` stand where a
        // `T` is written.
        if matches!(from, Ty::Ref { .. }) {
            return false;
        }
        let mut at = from.clone();
        for _ in 0..8 {
            let Some(step) = probe.deref_once(&at) else { return false };
            if *step.to.peel_refs() == *to {
                return true;
            }
            at = step.to;
        }
        false
    }
}

/// The two types with the references Rust would take off between them.
///
/// Deref coercion strips references from the VALUE, so one carrying more than
/// the declaration wants still meets it. The engine discounts ONE the
/// declaration has over the value, because the scope holds some receivers
/// without the borrow Rust gives them; a second is a type the caller never
/// wrote.
fn aligned_references<'t>(want: &'t Ty, actual: &'t Ty) -> (&'t Ty, &'t Ty) {
    let (mut want, mut actual) = (want, actual);
    while let (Ty::Ref { inner: w, .. }, Ty::Ref { inner: a, .. }) = (want, actual) {
        want = w;
        actual = a;
    }
    while let Ty::Ref { inner, .. } = actual {
        actual = inner;
    }
    if let Ty::Ref { inner, .. } = want {
        want = inner;
    }
    (want, actual)
}
