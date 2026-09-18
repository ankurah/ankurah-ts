//! The question a method, a bound or a projection is asked in, and what an
//! answer to it stood on.
//!
//! One `Probe` is one question: the module that wrote the call, the bounds on
//! the type parameters in scope, and — filled in as the answer is worked out —
//! the bounds the impl table had to assume rather than prove. The caller reads
//! those and decides what an answer only as good as an undecided bound is worth.

use super::Holds;
use crate::registry::{ModuleId, TypeRegistry};
use crate::ty::{TraitRef, Ty};

/// What a call is being resolved in: the module that wrote it, and the bounds
/// on the type parameters in scope, so that `self.notify()` inside a trait's
/// own default body reaches the trait's declaration.
pub struct Probe<'a> {
    pub reg: &'a TypeRegistry,
    pub module: ModuleId,
    pub param_bounds: &'a [(String, TraitRef)],
    /// Bounds an answer given through this probe stood on without being
    /// proved. The caller reads them and decides what to do with an answer
    /// that is only as good as a bound the solve has not settled.
    deferred: std::cell::RefCell<Vec<(Ty, TraitRef)>>,
}

impl<'a> Probe<'a> {
    pub fn new(reg: &'a TypeRegistry, module: ModuleId) -> Probe<'a> {
        Probe {
            reg,
            module,
            param_bounds: &[],
            deferred: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Note that an answer stood on bounds nothing has decided.
    pub(crate) fn note_deferred(&self, bounds: impl IntoIterator<Item = (Ty, TraitRef)>) {
        self.deferred.borrow_mut().extend(bounds);
    }

    /// The bounds every answer given here so far stood on, leaving none behind.
    pub fn take_deferred(&self) -> Vec<(Ty, TraitRef)> {
        std::mem::take(&mut *self.deferred.borrow_mut())
    }

    /// Does the impl table rule this bound out for that subject?
    ///
    /// A bound the engine cannot decide is not a false one: only a subject the
    /// table has read and found no impl for answers true here.
    pub fn rules_out(&self, subject: &Ty, bound: &TraitRef) -> bool {
        matches!(self.holds(subject, bound, 0), Holds::No)
    }

    pub fn with_bounds(mut self, bounds: &'a [(String, TraitRef)]) -> Probe<'a> {
        self.param_bounds = bounds;
        self
    }
}
