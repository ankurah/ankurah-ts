//! Answering "what is the type of this expression?" for the body translator.

pub mod closures;
#[cfg(test)]
mod closure_tests;
mod context;
#[cfg(test)]
mod context_tests;
pub mod expected;
mod patterns;
mod scope;

pub use closures::ClosureSig;
pub use context::{member_name, TypeContext};
pub use expected::{fn_shape, FnShape};
