// TS-ONLY: Maps Rust's borrowing semantics to JS (see E11)
//
// &T and &mut T are non-owning: dropping one releases nothing, and the value it
// points at belongs to somebody else. Both are marked nonOwning so the drop
// cascade steps over them in silence — without the marker the cascade would
// report them as foreign objects it does not know how to release.

import { nonOwning } from '../object.ts';

// ── Borrow<T> ── maps to &T ─────────────────────────────────────────────

export class Borrow<T> {
  readonly [nonOwning] = true;
  readonly #value: T;

  constructor(value: T) {
    this.#value = value;
  }

  get value(): T {
    return this.#value;
  }
}

// ── BorrowMut<T> ── maps to &mut T ──────────────────────────────────────

export class BorrowMut<T> {
  readonly [nonOwning] = true;
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
}
