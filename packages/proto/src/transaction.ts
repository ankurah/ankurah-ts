// MIRRORS: ankurah/proto/src/transaction.rs
//
// TransactionId is defined in id.ts (following Rust structure where the struct
// is in transaction.rs but we co-locate all IDs in id.ts for TS convenience).
// This file re-exports it to maintain the Rust module mapping.

export { TransactionId } from './id';
