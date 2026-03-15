// MIRRORS: ankurah/core/src/property/value/json.rs

import type { Value } from '../../value/index.ts';
import type { Property } from '../index.ts';
import { PropertyError } from '../traits.ts';

// ---------------------------------------------------------------------------
// Json
// ---------------------------------------------------------------------------

/**
 * A JSON property type for storing structured/nested data.
 *
 * Stores data as serialized JSON bytes using LWW (last-writer-wins) semantics.
 * The inner value can represent any JSON structure: objects, arrays,
 * strings, numbers, booleans, or null.
 *
 * Rust: `pub struct Json(pub serde_json::Value)`
 * Divergence: Rust wraps serde_json::Value; TS wraps `unknown` (any JSON-compatible value) [E8].
 */
export class Json implements Property {
  readonly inner: unknown;

  constructor(value: unknown) {
    this.inner = value;
  }

  // ── Static constructors ──

  /** Create a new Json from a value. Rust: `pub fn new(value: serde_json::Value) -> Self` */
  static new(value: unknown): Json {
    return new Json(value);
  }

  /** Create a Json null value. Rust: `pub fn null() -> Self` */
  static null(): Json {
    return new Json(null);
  }

  /** Create a Json object from key-value pairs. Rust: `pub fn object(pairs: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self` */
  static object(pairs: Iterable<[string, unknown]>): Json {
    const obj: Record<string, unknown> = {};
    for (const [k, v] of pairs) {
      obj[k] = v;
    }
    return new Json(obj);
  }

  /** Create a Json array. Rust: `pub fn array(items: impl IntoIterator<Item = Value>) -> Self` */
  static array(items: Iterable<unknown>): Json {
    return new Json(Array.from(items));
  }

  // ── Accessors ──

  /** Get the inner value. Rust: `pub fn inner(&self) -> &serde_json::Value` */
  // Note: `inner` is a public readonly field in TS (see above).

  /** Consume self and return the inner value. Rust: `pub fn into_inner(self) -> serde_json::Value` */
  intoInner(): unknown {
    return this.inner;
  }

  /**
   * Get a nested value by path (e.g., ["licensing", "territory"]).
   * Returns undefined if the path doesn't exist or any intermediate value is not an object.
   *
   * Rust: `pub fn get_path(&self, path: &[&str]) -> Option<&serde_json::Value>`
   */
  getPath(path: string[]): unknown | undefined {
    let current: unknown = this.inner;
    for (const step of path) {
      if (current === null || current === undefined || typeof current !== 'object') {
        return undefined;
      }
      current = (current as Record<string, unknown>)[step];
      if (current === undefined) {
        return undefined;
      }
    }
    return current;
  }

  /** Check if this Json is null. Rust: `pub fn is_null(&self) -> bool` */
  isNull(): boolean {
    return this.inner === null;
  }

  /** Check if this Json is an object. Rust: `pub fn is_object(&self) -> bool` */
  isObject(): boolean {
    return this.inner !== null && typeof this.inner === 'object' && !Array.isArray(this.inner);
  }

  /** Check if this Json is an array. Rust: `pub fn is_array(&self) -> bool` */
  isArray(): boolean {
    return Array.isArray(this.inner);
  }

  // ── Default ──

  /** Rust: `impl Default for Json { fn default() -> Self { Json::null() } }` */
  static default(): Json {
    return Json.null();
  }

  // ── From conversions ──
  // Rust: impl From<serde_json::Value> for Json — use constructor directly
  // Rust: impl From<Json> for serde_json::Value — use intoInner()
  // Rust: impl Deref/DerefMut — no TS equivalent

  // ── WASM / UniFFI bindings — omitted (TS-only runtime) ──

  // ── Property impl ──

  /**
   * Serialize this value into a Value for storage.
   *
   * Rust: `impl Property for Json { fn into_value(&self) -> Result<Option<Value>, PropertyError> }`
   */
  intoValue(): Value | null {
    return { type: 'Json', value: structuredClone(this.inner) };
  }

  /**
   * Deserialize from a Value (or null for missing).
   *
   * Rust: `fn from_value(value: Option<Value>) -> Result<Self, PropertyError>`
   */
  static fromValue(value: Value | null): Json {
    if (value === null) {
      throw PropertyError.missing();
    }
    if (value.type === 'Json') {
      return new Json(value.value);
    }
    if (value.type === 'Binary') {
      // Accept Binary for backwards compatibility
      try {
        const jsonStr = new TextDecoder().decode(value.value);
        const parsed = JSON.parse(jsonStr);
        return new Json(parsed);
      } catch (e) {
        throw PropertyError.deserializeError(e instanceof Error ? e : new Error(String(e)));
      }
    }
    throw PropertyError.invalidVariant(value, 'Json');
  }
}

// Rust: json! macro — omitted. TS has JSON literals natively.
