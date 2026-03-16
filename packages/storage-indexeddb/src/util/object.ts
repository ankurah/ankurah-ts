// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/object.rs

// Divergence: Rust wraps JsValue in SendWrapper and uses Reflect for property [E16]
// access with TryFrom<JsValue> conversions. In TS, we work with plain objects
// and use simple property access since there's no WASM boundary.

import { idbToValue } from '../idb_value.ts';
import type { Value } from '@ankurah/core';

/**
 * Wrapper around a plain JS object from IndexedDB, providing typed property access.
 *
 * Mirrors Rust `Object` which wraps `SendWrapper<JsValue>` with `get<T>()` / `set()`.
 */
export class IdbObject {
  private readonly obj: Record<string, unknown>;

  constructor(obj: unknown) {
    this.obj = obj as Record<string, unknown>;
  }

  /** Get a required property value. Throws if missing. */
  get(key: string): unknown {
    const v = this.obj[key];
    if (v === undefined) {
      throw new Error(`Failed to get property '${key}'`);
    }
    return v;
  }

  /** Get an optional property value. Returns undefined if missing. */
  getOpt(key: string): unknown | undefined {
    const v = this.obj[key];
    if (v === null || v === undefined) {
      return undefined;
    }
    return v;
  }

  /** Get a property as a Value (using IDB→Value conversion). Returns undefined if missing. */
  getValueOpt(key: string): Value | undefined {
    const v = this.obj[key];
    if (v === null || v === undefined) {
      return undefined;
    }
    return idbToValue(v);
  }

  /** Set a property on the underlying object. */
  set(key: string, value: unknown): void {
    this.obj[key] = value;
  }

  /** Get the raw underlying object. */
  raw(): unknown {
    return this.obj;
  }
}
