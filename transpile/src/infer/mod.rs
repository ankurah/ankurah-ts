//! Answering "what is the type of this expression?" for the body translator.

/// What a call resolves to, and what it wants of its arguments.
pub(crate) mod arguments;
mod awaiting;
pub(crate) mod calls;
pub mod closures;
#[cfg(test)]
mod closure_tests;
mod context;
mod shapes;
mod structs;
#[cfg(test)]
mod context_tests;
pub mod expected;
mod literals;
mod locals;
mod macro_types;
mod patterns;
mod operands;
mod prepass;
mod scope;
mod vars;
#[cfg(test)]
mod await_tests;
#[cfg(test)]
mod mismatch_tests;
#[cfg(test)]
mod vars_tests;

pub use closures::ClosureSig;
pub use context::TypeContext;
pub use shapes::{expr_form, member_name};
/// The callable an expected type describes. Only the closure tests ask for it
/// directly; the engine reaches it through `closure_signature`.
#[cfg(test)]
pub use expected::fn_shape;
