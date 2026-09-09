//! Whether two types the solve produced disagree, and how a site says so.
//!
//! A constraint that fails is only a fact about the program where the engine
//! read both sides: a coercion Rust performs, a projection the impl table never
//! read through, and an unknown nothing settled each look like a disagreement
//! and are not one. This is the judgement, kept apart from the sources that
//! raise the constraints.

use crate::ty::{Mismatch, Ty};

use super::context::TypeContext;

impl TypeContext<'_> {
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
    pub(super) fn mismatch_message(&self, want: &Ty, found: &Ty, mismatch: &Mismatch) -> String {
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
    pub(super) fn disagrees(
        &self,
        probe: &crate::registry::Probe<'_>,
        want: &Ty,
        found: &Ty,
        at_a_coercion_site: bool,
    ) -> bool {
        // Rust's deref coercion is a REFERENCE coercion: `&Arc<T>` stands where
        // `&T` is declared, and an `Arc<T>` by value does not stand where a `T`
        // is. Read before the references are aligned away.
        let borrowed = matches!(want, Ty::Ref { .. }) || matches!(found, Ty::Ref { .. });
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
        differ
            && !self.reaches(probe, want, found, borrowed)
            && !self.reaches(probe, found, want, borrowed)
    }

    fn any_disagrees(&self, probe: &crate::registry::Probe<'_>, xs: &[Ty], ys: &[Ty]) -> bool {
        xs.iter().zip(ys).any(|(x, y)| self.disagrees(probe, x, y, false))
    }

    /// Does one type reach the other along the deref chain?
    ///
    /// Only such a hop makes the two ONE representation, which is the whole of
    /// why they do not disagree by value: `Box<Expr>` is erased and `&String`
    /// and `&str` are both strings. A hop the emitted code DOES write — an
    /// `Arc` hands out its value through a field — is a different value with a
    /// different owner, and `fn take(value: Arc<Inner>) -> Inner { value }`
    /// returned the handle. Through a reference every hop stands, because that
    /// is where Rust itself coerces.
    fn reaches(
        &self,
        probe: &crate::registry::Probe<'_>,
        from: &Ty,
        to: &Ty,
        borrowed: bool,
    ) -> bool {
        // A reference layer is not a `Deref` impl: the alignment rule discounts
        // exactly one, and taking another here would let `&&T` stand where a
        // `T` is written.
        if matches!(from, Ty::Ref { .. }) {
            return false;
        }
        let mut at = from.clone();
        for _ in 0..8 {
            let Some(step) = probe.deref_once(&at) else { return false };
            if step.accessor.is_some() && !borrowed {
                return false;
            }
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
