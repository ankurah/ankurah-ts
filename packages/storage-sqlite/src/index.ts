// MIRRORS: ankurah/storage/sqlite/src/lib.rs
//
// SQLite storage engine for Ankurah
//
// Provides a lightweight embedded database storage option using SQLite.
// Sits between pure KV stores and full SQL servers, offering:
//
// - Single-file database (portable, easy backup)
// - Full SQL query capabilities without external server
// - Native support on all platforms including mobile (iOS, Android)
//
// Platform-specific split (Exception E16):
//   Rust: single `storage-sqlite` crate using `rusqlite` (C bindings)
//   TS:   @ankurah/storage-sqlite — abstract/shared SQLite logic
//         @ankurah/storage-better-sqlite3 — Node.js backend (better-sqlite3)
//         @ankurah/storage-expo-sqlite — React Native backend (expo-sqlite)

// connection.rs: Rust uses bb8 pool + rusqlite.
// Divergence: TS SQLite drivers handle connection management internally.
// No direct port of SqliteConnectionManager needed [E16].

export { SqliteStorageEngine, SqliteBucket, DEFAULT_POOL_SIZE } from './engine.ts';
export type { SqliteDriver } from './engine.ts';
export { SqliteError } from './error.ts';
export { SqlBuilder, SqlGenerationError, SplitPredicate, splitPredicateForSqlite } from './sql_builder.ts';
export type { SqlParam } from './sql_builder.ts';
export {
  sqliteValueType,
  sqliteValueIsJsonb,
  sqliteValueAsJsonString,
  sqliteValueToParam,
  sqliteValueFromValue,
  sqliteValueFromOptionalValue,
} from './value.ts';
export type { SqliteValue } from './value.ts';
