// MIRRORS: ankurah/storage/indexeddb-wasm/src/statics.rs

// Divergence: Rust uses lazy_static! with Property(JsValue) wrappers. [E16]
// In TS, these are just string constants since we use the native IndexedDB API.

export const ID_KEY = 'id';              // Special case - no prefix
export const HEAD_KEY = '__head';
export const COLLECTION_KEY = '__collection';
export const STATE_BUFFER_KEY = '__state_buffer';
export const ENTITY_ID_KEY = '__entity_id';
export const OPERATIONS_KEY = '__operations';
export const ATTESTATIONS_KEY = '__attestations';
export const PARENT_KEY = '__parent';
