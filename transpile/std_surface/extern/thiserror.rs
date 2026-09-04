//! `thiserror` 1.0.69 (`proto`) and 2.0.17 (`ankql`, `core`, `storage-sqlite`)
//!
//! Nothing is declared here, deliberately. `thiserror::Error` is a *derive
//! macro*, not a runtime trait: `#[derive(thiserror::Error)]` generates
//! `impl Display`, `impl std::error::Error`, and one `impl From<Inner> for
//! Outer` per `#[from]` field on the annotated type. An earlier version of this
//! file declared `pub trait Error: std::error::Error {}`, which let a bound
//! `T: thiserror::Error` be "proved" against a trait that does not exist.
//!
//! The derive hook registers those generated impls from the attributes on
//! ankurah's own types (engine step 7, spec 4.10). `use thiserror::Error` in
//! the corpus is a macro import and resolves through the macro namespace, not
//! through anything declarable in a signature stub.
