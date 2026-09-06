//! The method a BOUND declares, as a candidate in its own right.
//!
//! For: a `dyn Trait` receiver and a type parameter carrying the bound both
//! dispatch through the trait's own declaration — there is no impl to find,
//! because the concrete type is not known here. `impl Trait for dyn Trait`
//! says the same thing more precisely where somebody wrote one, so an impl
//! whose own bounds HOLD is the answer and the declaration is dropped. An impl
//! that only applies IF something nobody can decide holds is not: the caller
//! that wrote `fn f<I: IntoIterator>(values: I)` has SAID that `I` implements
//! the trait, and `impl<I: Iterator> IntoIterator for I` — which matches every
//! receiver and leaves `I: Iterator` deferred — is the weaker claim. Written
//! the other way round, `values.into_iter()` resolved through the blanket and
//! came out as `values.intoIter()`, a method nothing declares (G1).

use super::{Callee, Pick, Probe};
use crate::ty::subst::Subst;
use crate::ty::{bind_params, TraitRef, Ty};

impl Probe<'_> {
    /// Methods declared by a trait the candidate is known to implement because
    /// it *is* that trait: `dyn Trait`, or a parameter carrying the bound.
    pub(super) fn declared_picks(
        &self,
        candidate: &Ty,
        adjusted: &Ty,
        name: &str,
        explicit: &[Ty],
    ) -> Vec<Pick> {
        // One question, asked in one place: what is this receiver DECLARED to
        // implement. A projection is answered too (spec 4.4a) — `<I as
        // IntoIterator>::IntoIter` is an `Iterator` by its own declaration, and
        // `values_iter.next()` dispatches through that.
        let bounds: Vec<TraitRef> = self.bounds_of(candidate);
        if bounds.is_empty() {
            return Vec::new();
        }

        let mut picks = Vec::new();
        for bound in &bounds {
            // The trait the declaration sits on, with the arguments the bound
            // gave it: `T: Sub<u8>` reaches `Super<u8>::get`, not `Super<A>`'s.
            let Some((owner, method)) = self.reg.trait_method_of(bound, name) else {
                continue;
            };
            // The trait's own declaration writes its receiver in terms of
            // `Self`, which here is the object or the bounded parameter.
            let Some(receiver) = &method.sig.receiver else {
                continue;
            };
            let mut self_subst = Subst::new();
            self_subst.insert("Self".to_string(), candidate.clone());
            if &receiver.substitute(&self_subst) != adjusted {
                continue;
            }
            let Some(trait_def) = self.reg.trait_def(owner.id) else {
                continue;
            };
            let mut subst = bind_params(&trait_def.generics, &owner.args);
            subst.insert("Self".to_string(), candidate.clone());
            for (assoc, ty) in &owner.bindings {
                subst.insert(assoc.clone(), ty.clone());
            }
            // A turbofish says what the method's own parameters are, and a call
            // dispatched through a bound has them too: `i.collect::<Vec<_>>()`
            // on an `I: Iterator` said nothing about `Vec` without this.
            self.bind_explicit(&method.sig, explicit, &mut subst);
            picks.push(Pick {
                callee: Callee::TraitObject(owner.id, name.to_string(), DeclaredBound(())),
                ret: method.sig.ret.substitute(&subst),
                subst,
                obligations: Vec::new(),
            });
        }
        picks
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::Callee;
    use crate::testing::Fixture;
    use crate::ty::{TraitRef, Ty};

    /// A bound the caller DECLARED beats a blanket impl that only applies if
    /// something nobody can decide holds.
    ///
    /// `impl<I: Iterator> IntoIterator for I` matches every receiver, and its
    /// own `I: Iterator` cannot be decided for a type parameter. A caller that
    /// wrote `fn f<I: IntoIterator>(values: I)` has SAID that `I` implements
    /// the trait. Written the other way round, `values.into_iter()` resolved
    /// through the blanket, deferred `I: Iterator`, and came out as
    /// `values.intoIter()` — a method nothing declares (G1).
    #[test]
    fn a_declared_bound_beats_a_blanket_resting_on_an_undecided_one() {
        let c = Fixture::build(&[(
            "lib.rs",
            "pub trait Ping { fn ping(&self) -> u32; }\n\
             pub trait Pong { fn ping(&self) -> u32; }\n\
             impl<T: Pong> Ping for T { fn ping(&self) -> u32 { 0 } }\n",
        )]);
        let ping = c.reg.module_type(c.module("lib.rs"), "Ping").expect("declared");
        let bounds = vec![(
            "T".to_string(),
            TraitRef { id: ping, args: Vec::new(), bindings: Vec::new() },
        )];
        let found = c
            .probe("lib.rs")
            .with_bounds(&bounds)
            .resolve_method(&Ty::Param("T".into()), "ping")
            .expect("the declared bound answers");
        assert!(
            matches!(found.callee, Callee::TraitObject(id, ..) if id == ping),
            "the blanket answered instead of the bound: {:?}",
            found.callee
        );
        assert!(found.obligations.is_empty(), "{:?}", found.obligations);
    }

    /// An impl whose own bounds HOLD is still the more precise answer, and the
    /// declaration is dropped rather than clashing with it.
    #[test]
    fn an_impl_that_really_applies_still_wins() {
        let c = Fixture::build(&[(
            "lib.rs",
            "pub trait Ping { fn ping(&self) -> u32; }\n\
             pub struct Node;\n\
             impl Ping for Node { fn ping(&self) -> u32 { 1 } }\n",
        )]);
        let node = c.ty("lib.rs", "Node");
        let found = c.probe("lib.rs").resolve_method(&node, "ping").expect("the impl answers");
        assert!(
            matches!(found.callee, Callee::TraitImpl(..)),
            "the impl is what answers for a definite type: {:?}",
            found.callee
        );
    }
}

/// A witness that a `Callee::TraitObject` came from `declared_picks`.
///
/// H13: `trait_in_scope` exempts every `TraitObject` from the module's `use`
/// map — a trait the receiver's own type NAMES is in scope for the methods it
/// declares, whatever the module imported — and the comment asserting that only
/// this function produces one was enforced by nothing. Its field is private to
/// this module, so no other can build the variant at all: the claim is now the
/// type system's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredBound(pub(self) ());

impl DeclaredBound {
    /// For a test that builds the callee it expects to compare against. A test
    /// is not a second producer: it never reaches `trait_in_scope`.
    #[cfg(test)]
    pub(crate) fn for_tests() -> DeclaredBound {
        DeclaredBound(())
    }
}

#[cfg(test)]
mod witness_tests {
    use super::DeclaredBound;

    /// H13: `trait_in_scope` exempts every `Callee::TraitObject` from the
    /// module's `use` map, on the ground that `declared_picks` is the only
    /// thing that produces one. Nothing enforced that, so a second producer
    /// added anywhere would have silently taken the exemption with it. The
    /// witness's field is private to this module, so the claim is the type
    /// system's now: this test says what the compiler is checking, because a
    /// test cannot check a thing that will not compile.
    #[test]
    fn the_witness_is_this_modules_to_build() {
        // Buildable here, by the one function that answers a bound.
        let _here = DeclaredBound(());
        // And `DeclaredBound(())` written in any other module of the crate is a
        // compile error (E0603 on the private field), which is the enforcement.
        // What a test CAN say is that the test constructor is not the ordinary
        // one: it is behind `#[cfg(test)]` and never reaches `trait_in_scope`.
        assert_eq!(DeclaredBound::for_tests(), _here);
    }
}
