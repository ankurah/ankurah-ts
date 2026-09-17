//! Whether two types the solve produced disagree, and how a site says so.
//!
//! A constraint that fails is only a fact about the program where the engine
//! read both sides: a coercion Rust performs, a projection the impl table never
//! read through, and an unknown nothing settled each look like a disagreement
//! and are not one. This is the judgement, kept apart from the sources that
//! raise the constraints.

use crate::ty::{Mismatch, Site, Ty};

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
    /// `site` says what the position does with the two. Rust coerces where a
    /// value MEETS a declared type — an argument, an annotation, a return, a
    /// field — and nowhere inside one: `Vec<Box<Expr>>` handed where
    /// `Vec<Expr>` is declared is a program Rust rejects.
    pub(super) fn disagrees(
        &self,
        probe: &crate::registry::Probe<'_>,
        want: &Ty,
        found: &Ty,
        site: Site,
    ) -> bool {
        // Rust's deref coercion is a REFERENCE coercion: `&Arc<T>` stands where
        // `&T` is declared, and an `Arc<T>` by value does not stand where a `T`
        // is. Read before the references are aligned away.
        let borrowed = matches!(want, Ty::Ref { .. }) || matches!(found, Ty::Ref { .. });
        let (want, found, borrows_meet) = match site {
            Site::Coercion => {
                let meet = self.borrows_meet(probe, want, found);
                let (aligned, actual, _) = aligned_references(want, found);
                (aligned, actual, meet)
            }
            // A comparison hands nothing over, so what either side OWNS is not
            // in question; the references still come off between them.
            Site::Comparison => {
                let (want, found, _borrows) = aligned_references(want, found);
                (want, found, true)
            }
            // Inside a type argument nothing is coerced, and two references of
            // the same depth say only what is under them.
            Site::Nested => match (want, found) {
                (Ty::Ref { inner: a, .. }, Ty::Ref { inner: b, .. }) => (&**a, &**b, true),
                (a, b) => (a, b, true),
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
                Some(b) => self.disagrees(probe, a, &b, Site::Nested),
                None => true,
            },
            (other, Ty::Slice(b) | Ty::Array { elem: b, .. }) => match self.sequence_element(other) {
                Some(a) => self.disagrees(probe, &a, b, Site::Nested),
                None => true,
            },
            _ => true,
        };
        // `&String` stands where `&str` is declared and `Box<Expr>` where
        // `Expr` is: Rust derefs one into the other and the port writes both as
        // one value, so the two do not disagree about the program.
        if site == Site::Nested {
            return differ;
        }
        // The borrows are read AFTER the two cores, so a side the engine has
        // not read — an unknown, a hole, a parameter, a projection — reports
        // nothing here either.
        !borrows_meet
            || (differ
                && !self.reaches(probe, want, found, borrowed)
                && !self.reaches(probe, found, want, borrowed))
    }

    /// Does the value the site hands over leave the target's borrow as
    /// declared?
    ///
    /// A borrow handed where a value is OWNED is what makes the callee release
    /// what the caller still holds, so it is read where there is something to
    /// release: a `&u8` where a `u8` is declared is a program Rust rejects and
    /// the port writes the same code either way. A borrow that is not unique
    /// enough loses a WRITE, which has nothing to do with drop glue.
    pub(super) fn borrows_meet(
        &self,
        probe: &crate::registry::Probe<'_>,
        want: &Ty,
        found: &Ty,
    ) -> bool {
        let (_, _, borrows) = aligned_references(want, found);
        let releases = crate::ownership::drops_of(probe, want).is_droppable();
        borrows.unique && (borrows.owned || !releases)
    }

    fn any_disagrees(&self, probe: &crate::registry::Probe<'_>, xs: &[Ty], ys: &[Ty]) -> bool {
        xs.iter().zip(ys).any(|(x, y)| self.disagrees(probe, x, y, Site::Nested))
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

/// The two types with the references Rust would take off between them, and
/// whether it would take them off at all.
///
/// Deref coercion strips references from the VALUE; it never changes what the
/// target OWNS or how uniquely it borrows. The engine discounts ONE reference
/// the declaration has over the value, because the scope holds some receivers
/// without the borrow Rust gives them.
fn aligned_references<'t>(want: &'t Ty, actual: &'t Ty) -> (&'t Ty, &'t Ty, Borrows) {
    let mut borrows = Borrows {
        // Rust has no coercion from a borrow to the value it points at.
        owned: !(!matches!(want, Ty::Ref { .. }) && matches!(actual, Ty::Ref { .. })),
        unique: true,
    };
    let (mut want, mut actual) = (want, actual);
    while let (
        Ty::Ref { inner: w, mutable: writes },
        Ty::Ref { inner: a, mutable: written_through },
    ) = (want, actual)
    {
        // A shared borrow never satisfies a unique one; the other way round is
        // Rust's own reborrow.
        if *writes && !*written_through {
            borrows.unique = false;
        }
        want = w;
        actual = a;
    }
    while let Ty::Ref { inner, .. } = actual {
        actual = inner;
    }
    if let Ty::Ref { inner, .. } = want {
        want = inner;
    }
    (want, actual, borrows)
}

/// What the value the site hands over does about the target's borrow: whether
/// the target still OWNS what it declares, and whether the borrow is unique
/// enough to be written through.
struct Borrows {
    owned: bool,
    unique: bool,
}
