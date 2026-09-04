//! `Ty` — the structural Rust type the transpiler carries from resolution to
//! emission. TypeScript strings are produced from a `Ty` by `name_map`, at
//! emission, and are never parsed back.

mod def;
pub mod subst;

pub use def::{ArrayLen, Prim, TraitRef, Ty, TypeId};
pub use subst::bind_params;
