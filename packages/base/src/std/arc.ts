// TS-ONLY: Maps Rust's std::sync::Arc<T> and std::sync::Weak<T> to JS (see E11)
//
// Arc<T> provides reference-counted shared ownership. When the last Arc is
// dropped, the inner value's drop() is called (if it extends Drop).
//
// Weak<T> is a non-owning reference that does not prevent the inner from
// being dropped. upgrade() returns Arc<T> | null.
//
// CRITICAL: In JS, `const x = arc` does NOT increment the refcount.
// You MUST use `arc.clone()` to create a new owning reference.
// Bare assignment shares the Arc object itself (same refcount).
//
// See port/ownership.md for the full design rationale.

import { Drop, disposeSymbol } from './drop.ts';

// ── Shared inner state ───────────────────────────────────────────────────

interface ArcInner<T> {
  value: T;
  strongCount: number;
  weakCount: number;
  dropped: boolean;
}

// ── Arc<T> ───────────────────────────────────────────────────────────────
//
// Reference-counted shared ownership. Maps to Rust's std::sync::Arc<T>.
//
// Usage:
//   const a = Arc.new(myValue);
//   const b = a.clone();  // refcount incremented
//   a.drop();             // refcount decremented, inner NOT dropped
//   b.drop();             // refcount hits 0, inner dropped

export class Arc<T> {
  readonly #inner: ArcInner<T>;

  private constructor(inner: ArcInner<T>) {
    this.#inner = inner;
  }

  /**
   * Create a new Arc wrapping a value. Initial refcount = 1.
   */
  static new<T>(value: T): Arc<T> {
    return new Arc<T>({ value, strongCount: 1, weakCount: 0, dropped: false });
  }

  /**
   * Clone this Arc, incrementing the refcount.
   * This is the ONLY correct way to share an Arc in JS.
   * Do NOT use `const x = arc` — that shares the Arc object without incrementing.
   */
  clone(): Arc<T> {
    if (this.#inner.dropped) {
      throw new Error('Arc: cannot clone — inner value has been dropped');
    }
    this.#inner.strongCount++;
    return new Arc<T>(this.#inner);
  }

  /**
   * Access the inner value. Throws if already dropped.
   */
  get value(): T {
    if (this.#inner.dropped) {
      throw new Error('Arc: inner value has been dropped');
    }
    return this.#inner.value;
  }

  /**
   * Decrement the refcount. When it hits zero, drop the inner value
   * (calls value.drop() if value extends Drop).
   */
  drop(): void {
    if (this.#inner.strongCount <= 0) return; // already fully released
    this.#inner.strongCount--;
    if (this.#inner.strongCount === 0) {
      this.#inner.dropped = true;
      const val = this.#inner.value;
      if (val && typeof (val as any).drop === 'function') {
        (val as any).drop();
      }
    }
  }

  /**
   * Create a Weak reference to this Arc's inner value.
   */
  downgrade(): Weak<T> {
    this.#inner.weakCount++;
    return new Weak<T>(this.#inner);
  }

  /**
   * Current strong reference count.
   */
  get strongCount(): number {
    return this.#inner.strongCount;
  }

  /**
   * ES2023 `using` support. Delegates to drop().
   */
  [disposeSymbol](): void {
    this.drop();
  }
}

// ── Weak<T> ──────────────────────────────────────────────────────────────
//
// Non-owning reference. Maps to Rust's std::sync::Weak<T>.
// Does not prevent the inner value from being dropped.
//
// Usage:
//   const weak = arc.downgrade();
//   const upgraded = weak.upgrade(); // Arc<T> | null

export class Weak<T> {
  readonly #inner: ArcInner<T>;

  /** @internal */
  constructor(inner: ArcInner<T>) {
    this.#inner = inner;
  }

  /**
   * Attempt to upgrade to a strong reference.
   * Returns a new Arc<T> (incrementing refcount) if the value is still alive,
   * or null if all strong references have been dropped.
   */
  upgrade(): Arc<T> | null {
    if (this.#inner.dropped || this.#inner.strongCount === 0) {
      return null;
    }
    this.#inner.strongCount++;
    return new (Arc as any)(this.#inner);
  }

  /**
   * Release this weak reference.
   */
  drop(): void {
    if (this.#inner.weakCount > 0) {
      this.#inner.weakCount--;
    }
  }
}
