//! Which impl turns one type into another.
//!
//! For: `?` on a `Result<T, E1>` inside a function returning `Result<_, E2>`
//! calls `From::from` on the error, and `.into()` calls it on the value. Rust
//! picks the impl; without that answer the emitted code hands the error on
//! unconverted, which is the wrong value at every site where the two types
//! differ.
//!
//! A conversion is looked up by matching both halves at once. `impl<T> From<T>
//! for T` is written for every type in the language, so matching the target and
//! the source separately would let it stand for a conversion between two
//! different types; matching them in one substitution is what makes `T` refuse
//! to be `RetrievalError` and `MutationError` at the same time.

use super::impls::ImplId;
use super::Probe;
use crate::ty::subst::Subst;
use crate::ty::{unify, Ty, TypeId};

/// The path of the trait `?` and `.into()` go through.
pub const FROM_PATH: &str = "std::convert::From";

/// The path of the trait `try_into()` and `T::try_from(x)` go through.
pub const TRY_FROM_PATH: &str = "std::convert::TryFrom";

/// The impl that performs one conversion.
///
/// The bindings the match made and the bounds it could not decide are checked
/// and then dropped: nothing that writes a conversion call reads either, and a
/// field nobody reads is a claim nobody checks.
#[derive(Debug, Clone)]
pub struct Conversion {
    pub impl_id: ImplId,
}

/// Why a conversion could not be written.
#[derive(Debug, Clone)]
pub enum NoConversion {
    /// The trait itself is not declared, so there is nothing to search.
    NoTrait,
    /// Nothing in the impl table converts these two.
    None,
    /// More than one impl does, and the engine has no rule to choose between
    /// them. The corpus compiles under rustc, so this means the engine matched
    /// something rustc would not have.
    Ambiguous(Vec<ImplId>),
}

impl std::fmt::Display for NoConversion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoConversion::NoTrait => write!(f, "the trait is not declared"),
            NoConversion::None => write!(f, "no impl converts them"),
            NoConversion::Ambiguous(ids) => {
                write!(f, "{} impls convert them", ids.len())
            }
        }
    }
}

impl Probe<'_> {
    /// The `impl From<from> for to` that Rust would call, or why there is none.
    ///
    /// The identity — `from` and `to` being one type — is *not* answered here.
    /// A caller that writes a conversion call has to know the difference
    /// between "no call is needed" and "this call is needed", so the two
    /// questions stay apart.
    pub fn from_impl(&self, from: &Ty, to: &Ty) -> Result<Conversion, NoConversion> {
        self.conversion_impl(FROM_PATH, from, to)
    }

    /// The same for `TryFrom`, whose impl also supplies the `Error` it fails
    /// with.
    pub fn try_from_impl(&self, from: &Ty, to: &Ty) -> Result<Conversion, NoConversion> {
        self.conversion_impl(TRY_FROM_PATH, from, to)
    }

    /// The impl of a one-argument conversion trait that takes `from` to `to`.
    pub fn conversion_impl(
        &self,
        trait_path: &str,
        from: &Ty,
        to: &Ty,
    ) -> Result<Conversion, NoConversion> {
        self.two_sided_impl(trait_path, to, from, false)
    }

    /// The impl of an operator trait — `PartialEq<Rhs> for Lhs`, `Add<Rhs> for
    /// Lhs` — that applies to these two operands.
    ///
    /// The right-hand type is the trait's argument, and every operator trait
    /// declares that argument as `Rhs = Self`: an `impl PartialEq for Clock`
    /// writes no argument at all and means `PartialEq<Clock>`.
    pub fn operator_impl(
        &self,
        trait_path: &str,
        lhs: &Ty,
        rhs: &Ty,
    ) -> Result<Conversion, NoConversion> {
        self.two_sided_impl(trait_path, lhs, rhs, true)
    }

    /// The single impl of a trait whose self type is `subject` and whose one
    /// argument is `argument`.
    ///
    /// Both sides are matched into one substitution, so a parameter standing in
    /// both places has to be the same type in both — which is what tells the
    /// reflexive `impl<T> From<T> for T` apart from a real conversion.
    fn two_sided_impl(
        &self,
        trait_path: &str,
        subject: &Ty,
        argument: &Ty,
        argument_defaults_to_self: bool,
    ) -> Result<Conversion, NoConversion> {
        let Some(trait_id) = self.reg.system_type(trait_path) else {
            return Err(NoConversion::NoTrait);
        };
        let mut found: Vec<Conversion> = Vec::new();
        for &id in self.reg.impls().of_trait(trait_id) {
            if let Some(conversion) =
                self.matching_impl(id, trait_id, subject, argument, argument_defaults_to_self)
            {
                found.push(conversion);
            }
        }
        match found.len() {
            0 => Err(NoConversion::None),
            1 => Ok(found.pop().expect("one was found")),
            _ => Err(NoConversion::Ambiguous(
                found.iter().map(|c| c.impl_id).collect(),
            )),
        }
    }

    /// Does this one impl have `subject` as its self type and `argument` as the
    /// trait's one argument?
    fn matching_impl(
        &self,
        id: ImplId,
        trait_id: TypeId,
        subject: &Ty,
        argument: &Ty,
        argument_defaults_to_self: bool,
    ) -> Option<Conversion> {
        let def = self.reg.impl_def(id);
        let implemented = def.trait_ref.as_ref()?;
        if implemented.id != trait_id {
            return None;
        }
        // `impl PartialEq for Clock` writes no argument and means
        // `PartialEq<Clock>`; every operator trait declares `Rhs = Self`.
        let written_argument = match implemented.args.first() {
            Some(ty) => ty.clone(),
            None if argument_defaults_to_self => def.self_ty.clone(),
            None => return None,
        };
        if implemented.args.len() > 1 {
            return None;
        }
        // The impl's parameters are renamed apart for the length of the match,
        // for the reason `ImplDef::match_pattern` gives: the impl's `T` and a
        // `T` inside the types being matched are different parameters that
        // happen to share a name.
        let fresh: Vec<String> = def
            .generics
            .iter()
            .map(|g| format!("{}#c{}", g, id.0))
            .collect();
        let rename: Subst = def
            .generics
            .iter()
            .cloned()
            .zip(fresh.iter().map(|f| Ty::Param(f.clone())))
            .collect();
        let mut subst = Subst::new();
        unify(&fresh, &def.self_ty.substitute(&rename), subject, &mut subst).ok()?;
        unify(
            &fresh,
            &written_argument.substitute(&rename),
            argument,
            &mut subst,
        )
        .ok()?;
        let subst: Subst = def
            .generics
            .iter()
            .zip(&fresh)
            .filter_map(|(name, f)| subst.get(f).map(|ty| (name.clone(), ty.clone())))
            .collect();
        self.bounds_hold(&def.bounds, &subst)?;
        Some(Conversion { impl_id: id })
    }
}
