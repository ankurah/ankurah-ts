//! Answering "what is the type of this expression?" for the body translator.

/// What a call resolves to, and what it wants of its arguments.
mod calls;
pub mod closures;
#[cfg(test)]
mod closure_tests;
mod context;
#[cfg(test)]
mod context_tests;
pub mod expected;
mod literals;
mod patterns;
mod scope;

pub use closures::ClosureSig;
pub use context::{expr_form, member_name, TypeContext};
/// The callable an expected type describes. Only the closure tests ask for it
/// directly; the engine reaches it through `closure_signature`.
#[cfg(test)]
pub use expected::fn_shape;
