// TS-ONLY: Maps Rust's borrowing semantics to JS (see E11)
//
// &T and &mut T in Rust are non-owning references. Dropping them is a no-op.
// Borrow<T> and BorrowMut<T> override [Symbol.dispose]() to no-op so that
// the parent's auto-cascade harmlessly calls them without propagating.

import { disposeSymbol } from '../drop_registry.ts';

// ── Borrow<T> ── maps to &T ─────────────────────────────────────────────

export class Borrow<T> {
  readonly #value: T;

  constructor(value: T) {
    this.#value = value;
  }

  get value(): T {
    return this.#value;
  }

  // Drop glue is a no-op — we don't own this value.
  [disposeSymbol](): void {}
}

// ── BorrowMut<T> ── maps to &mut T ──────────────────────────────────────

export class BorrowMut<T> {
  #value: T;

  constructor(value: T) {
    this.#value = value;
  }

  get value(): T {
    return this.#value;
  }

  set value(v: T) {
    this.#value = v;
  }

  // Drop glue is a no-op — we don't own this value.
  [disposeSymbol](): void {}
}
