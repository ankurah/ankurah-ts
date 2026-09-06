//! Answering "what is the type of this expression?" for the body translator.

/// What a call resolves to, and what it wants of its arguments.
pub(crate) mod calls;
pub mod closures;
#[cfg(test)]
mod closure_tests;
mod context;
mod shapes;
#[cfg(test)]
mod context_tests;
pub mod expected;
mod literals;
mod macro_types;
mod patterns;
mod scope;

pub use closures::ClosureSig;
pub use context::TypeContext;
pub use shapes::{expr_form, member_name};
/// The callable an expected type describes. Only the closure tests ask for it
/// directly; the engine reaches it through `closure_signature`.
#[cfg(test)]
pub use expected::fn_shape;
