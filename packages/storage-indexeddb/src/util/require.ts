// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/require.rs

// Divergence: Rust WBGRequire trait handles JsValue/Option/Result<_,JsValue> [E16]
// error conversions for wasm-bindgen. In TS, we don't need this trait because
// native JS errors are already Error objects. This file provides a simple `require`
// helper for consistency.

/**
 * Assert a value is not null/undefined, throwing with a descriptive message.
 *
 * Mirrors Rust `WBGRequire::require(self, err)` for Option<T>.
 */
export function require<T>(value: T | null | undefined, msg: string): T {
  if (value === null || value === undefined) {
    throw new Error(`${msg} is null/undefined`);
  }
  return value;
}
