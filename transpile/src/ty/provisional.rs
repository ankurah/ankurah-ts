//! An answer the impl table gave on a bound nothing had settled, and what
//! becomes of it.
//!
//! For: `impl<A: Step> Iterator for RangeInclusive<A>` cannot be proved while
//! `A` is an unknown, and refusing the impl would lose the only answer there
//! is. The answer is handed out so the body types as it would if the bound
//! held, and the table keeps hold of it: if the solve settles the bound to
//! false, the answer and every unknown that took it stand for nothing.

use super::def::{InferId, TraitRef, Ty};
use super::vars::InferTable;

/// A projection the impl table could read only through an impl whose own
/// bounds the solve had not settled.
///
/// For: an answer given on an undecided bound is not yet a fact about the
/// program. It is handed out so the body types as it would if the bound held,
/// and it is taken back — with everything bound from it — if the solve settles
/// the bound to false.
#[derive(Clone, Debug)]
pub struct Provisional {
    /// The projection that was read, so a report can name what was asked.
    pub read: Ty,
    /// Subject and trait of each bound the answer stood on, with the unknowns
    /// that decide them still in place.
    pub stood_on: Vec<(Ty, TraitRef)>,
    pub span: proc_macro2::Span,
    /// The unknowns a constraint reached while it read this answer. They are
    /// what took the answer, and what a false bound takes back.
    pub(super) derived: Vec<InferId>,
}

/// Which unknown at a site carries an answer read through a bound nothing has
/// decided. Clear of the others.
const PROVISIONAL: usize = usize::MAX - 2;

impl InferTable {
    /// Hold an answer that stands on bounds nothing has decided, at the unknown
    /// that now carries it. Everything downstream reads the answer through the
    /// table and types as it would have; what changes is that the engine can
    /// still take it back.
    pub fn provisionally(
        &mut self,
        read: &Ty,
        answer: &Ty,
        stood_on: Vec<(Ty, TraitRef)>,
        span: proc_macro2::Span,
    ) -> Ty {
        let at = span.start();
        let var = self.at_site(at.line, at.column, PROVISIONAL);
        // One unknown per written site, however many rounds read it: a fresh
        // one each round is a variable the fixed point never stops minting.
        if let Some(held) = self.provisional.get_mut(&var) {
            if held.read != *read {
                return answer.clone();
            }
            for bound in stood_on {
                if !held.stood_on.contains(&bound) {
                    held.stood_on.push(bound);
                }
            }
            return Ty::Var(var);
        }
        if self.assign(var, answer).is_err() {
            return answer.clone();
        }
        self.provisional.insert(
            var,
            Provisional {
                read: read.clone(),
                stood_on,
                span,
                derived: Vec::new(),
            },
        );
        Ty::Var(var)
    }

    /// Every answer still standing on an undecided bound.
    pub fn provisional_answers(&self) -> Vec<(InferId, Provisional)> {
        self.provisional.iter().map(|(id, p)| (*id, p.clone())).collect()
    }

    /// Take one such answer back: it and everything bound from it stand for
    /// nothing, so every type read through it is unknown again.
    pub fn withdraw(&mut self, var: InferId) {
        let derived = self.provisional.remove(&var).map(|p| p.derived).unwrap_or_default();
        self.poisoned.insert(var);
        self.poisoned.extend(derived);
    }

    /// A constraint that read a provisional answer hands that answer on to
    /// every other unknown it reached, so withdrawing the answer withdraws
    /// them too.
    pub(super) fn note_derivations(&mut self) {
        let touched = self.touched.clone();
        let carriers: Vec<InferId> =
            touched.iter().copied().filter(|id| self.provisional.contains_key(id)).collect();
        for carrier in carriers {
            let taken: Vec<InferId> = touched.iter().copied().filter(|id| *id != carrier).collect();
            if let Some(entry) = self.provisional.get_mut(&carrier) {
                entry.derived.extend(taken);
            }
        }
    }
}
