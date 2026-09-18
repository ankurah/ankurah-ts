//! Answering "what is the type of this expression?" for the body translator.

/// What a call resolves to, and what it wants of its arguments.
pub(crate) mod arguments;
mod awaiting;
mod branches;
#[cfg(test)]
mod branch_tests;
pub(crate) mod calls;
pub mod closures;
#[cfg(test)]
mod closure_tests;
mod context;
mod variants;
mod shapes;
mod structs;
#[cfg(test)]
mod context_tests;
pub mod expected;
mod literals;
mod locals;
mod macro_types;
mod patterns;
#[cfg(test)]
mod pattern_tests;
mod operands;
mod prepass;
#[cfg(test)]
mod prepass_tests;
mod as_written;
mod block_type;
mod scope;
mod standing;
mod mismatch;
mod vars;
#[cfg(test)]
mod await_tests;
#[cfg(test)]
mod mismatch_tests;
#[cfg(test)]
mod borrow_tests;
#[cfg(test)]
mod vars_tests;
#[cfg(test)]
mod obligation_tests;

pub use closures::ClosureSig;
pub use context::TypeContext;
pub use shapes::{expr_form, member_name};
/// The callable an expected type describes. Only the closure tests ask for it
/// directly; the engine reaches it through `closure_signature`.
#[cfg(test)]
pub use expected::fn_shape;
