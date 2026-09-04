//! `async-trait` 0.1.89
//!
//! `#[async_trait]` rewrites `async fn` in a trait into a method returning
//! `Pin<Box<dyn Future<Output = ..> + Send + 'async_trait>>`. The engine does
//! not expand macros, so the transpiler's attribute handler applies that
//! rewrite to the signature it already extracted (spec 4.10) and this file has
//! nothing to declare but the attribute's name. The rewrite matters: the
//! rust-analyzer oracle could not type anything under `#[async_trait]`, and
//! `core/src/context.rs` is why `try_sites.json` is empty.
