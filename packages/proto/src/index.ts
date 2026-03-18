// MIRRORS: ankurah/proto/src/lib.rs
//
// @ankurah/proto — Wire protocol types and bincode codec.
//
// Re-exports mirror Rust lib.rs `pub use` statements.
// Exception E9: wasm.rs skipped (WASM-only)
// Exception E10: postgres.rs skipped (feature-gated, out of scope)

// ── Modules (matching Rust `pub mod` declarations) ──

export * from './auth';
export * from './clock';
export * from './collection';
export * from './data';
export * from './error';
export * from './id';
export * from './message';
export * from './peering';
export * from './request';
export { QueryId } from './subscription';
export * from './transaction';
export * from './update';

// ── TS-only modules ──

export * from './codec';

// Divergence: human_id and sys are `pub mod` in Rust (accessible as proto::human_id::humanize etc)
// but NOT `pub use` re-exported from the crate root. TS re-exports for convenience. [E4]
export * from './human_id';
export * from './sys';
