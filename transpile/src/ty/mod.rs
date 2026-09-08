//! `Ty` — the structural Rust type the transpiler carries from resolution to
//! emission. TypeScript strings are produced from a `Ty` by `name_map`, at
//! emission, and are never parsed back.

mod def;
pub mod prim_consts;
mod query;
pub mod subst;
mod unify;
mod vars;

pub use def::{ArrayLen, IdSpaceExhausted, Prim, TraitRef, Ty, TypeId};
pub use subst::bind_params;
pub use unify::{unify, Mismatch};
pub use vars::InferTable;
