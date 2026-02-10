// MIRRORS: ankurah/proto/src/subscription.rs
//
// QueryId is defined in id.ts (following Rust structure where the struct
// is in subscription.rs but we co-locate all IDs in id.ts for TS convenience).
// This file re-exports it to maintain the Rust module mapping.

export { QueryId } from './id';
