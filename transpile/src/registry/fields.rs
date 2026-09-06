//! Where a field lives, and what has to be written to reach it.
//!
//! A field may sit on the receiver itself or behind a `Deref` step, and a TUPLE
//! has positional fields the port reads by INDEX rather than by name, because
//! the port writes a tuple as an array.

use super::method::{FieldResolution, Probe};
use crate::ty::{bind_params, Ty};

impl Probe<'_> {
    /// The type of `expr.field`, walking the same chain method calls walk.
    pub fn resolve_field(&self, receiver: &Ty, field: &str) -> Option<FieldResolution> {
        let steps = self.deref_chain(receiver).ok()?;
        let mut candidates: Vec<Ty> = vec![receiver.clone()];
        candidates.extend(steps.iter().map(|s| s.to.clone()));
        for (depth, candidate) in candidates.iter().enumerate() {
            if let Some(ty) = self.field_on(candidate, field) {
                return Some(FieldResolution {
                    ty: self.normalize(&ty),
                    steps: steps[..depth].to_vec(),
                });
            }
        }
        None
    }

    fn field_on(&self, ty: &Ty, field: &str) -> Option<Ty> {
        // A TUPLE's positional fields. Rust writes them `value.0`, extraction
        // spells them `_0` the way emission does, and nothing answered — so
        // `(A, B, C)`'s three fields were each "no field `_0` on ..", and the
        // emitted `value._0` read `undefined` off the array the port writes a
        // tuple as. Fourteen reports in `proto` alone, all from one impl.
        if let Ty::Tuple(elems) = ty {
            let at: usize = field.strip_prefix('_')?.parse().ok()?;
            return elems.get(at).cloned();
        }
        let Ty::Named { id, args } = ty else {
            return None;
        };
        let def = self.reg.def(*id)?;
        let subst = bind_params(&def.type_params, args);
        def.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, ty)| ty.substitute(&subst))
    }
}
