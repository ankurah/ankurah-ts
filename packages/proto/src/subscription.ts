// MIRRORS: ankurah/proto/src/subscription.rs
//
// Divergence: QueryId struct is defined in id.ts (co-located with other ULID IDs)
// rather than here. This file re-exports it to maintain the Rust module mapping. [E4]

export { QueryId } from './id';
