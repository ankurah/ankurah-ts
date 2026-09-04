//! Answering "what is the type of this expression?" for the body translator.

mod context;
#[cfg(test)]
mod context_tests;
mod patterns;
mod scope;

pub use context::TypeContext;
