// TS-ONLY: Maps Rust's std::cell module to JS (see E11)
//
// Provides:
//   - RefCell<T>: runtime borrow checking (maps to std::cell::RefCell<T>)
//   - Ref<T>: shared borrow guard (maps to std::cell::Ref<T>)
//   - RefMut<T>: exclusive mutable borrow guard (maps to std::cell::RefMut<T>)
//
// See port/ownership.md and port/ownership/provided-types.md for API spec.

import { Drop } from './drop.ts';

// ── BorrowState ──────────────────────────────────────────────────────────

type BorrowState =
  | { kind: 'not_borrowed' }
  | { kind: 'shared'; count: number }
  | { kind: 'mut_borrowed' };

// ── RefCell<T> ───────────────────────────────────────────────────────────
//
// 1:1 equivalent of Rust's std::cell::RefCell<T>. Runtime borrow checking —
// panics on double mutable borrow, just like Rust.
//
// Returns Ref<T> / RefMut<T> Drop guards (used with `using`).
//
// Usage:
//   const cell = new RefCell(value);
//   { using guard = cell.borrowMut(); guard.value.field = 42; }
//   // borrow released on dispose

export class RefCell<T> {
  readonly #value: T;
  #state: BorrowState = { kind: 'not_borrowed' };
  readonly #onMutRelease: (() => void) | undefined;
  readonly #label: string;

  /**
   * @param value — the wrapped value
   * @param options.onMutRelease — called after each mutable borrow is released
   * @param options.label — human-readable name for error messages (default: 'RefCell')
   */
  constructor(value: T, options?: { onMutRelease?: () => void; label?: string }) {
    this.#value = value;
    this.#onMutRelease = options?.onMutRelease;
    this.#label = options?.label ?? 'RefCell';
  }

  /**
   * Shared read-only borrow. Returns a Ref<T> Drop guard.
   * Throws if a mutable borrow is active.
   * Multiple shared borrows can be active simultaneously.
   */
  borrow(): Ref<T> {
    if (this.#state.kind === 'mut_borrowed') {
      throw new Error(`${this.#label} already mutably borrowed — cannot take shared borrow`);
    }
    if (this.#state.kind === 'shared') {
      this.#state = { kind: 'shared', count: this.#state.count + 1 };
    } else {
      this.#state = { kind: 'shared', count: 1 };
    }
    return new Ref<T>(this.#value, () => {
      if (this.#state.kind === 'shared') {
        if (this.#state.count <= 1) {
          this.#state = { kind: 'not_borrowed' };
        } else {
          this.#state = { kind: 'shared', count: this.#state.count - 1 };
        }
      }
    }, this.#label);
  }

  /**
   * Exclusive mutable borrow. Returns a RefMut<T> Drop guard.
   * Throws if any borrow (shared or mutable) is active.
   */
  borrowMut(): RefMut<T> {
    if (this.#state.kind !== 'not_borrowed') {
      if (this.#state.kind === 'mut_borrowed') {
        throw new Error(`${this.#label} already mutably borrowed`);
      }
      throw new Error(`${this.#label} already shared-borrowed (count: ${this.#state.count})`);
    }
    this.#state = { kind: 'mut_borrowed' };
    return new RefMut<T>(this.#value, () => {
      this.#state = { kind: 'not_borrowed' };
      this.#onMutRelease?.();
    }, this.#label);
  }
}

// ── Ref<T> ───────────────────────────────────────────────────────────────

/**
 * Shared read-only borrow guard for RefCell<T>.
 * Equivalent to Rust's std::cell::Ref<T>.
 */
export class Ref<T> extends Drop {
  readonly #value: T;
  readonly #release: () => void;

  /** @internal */
  constructor(value: T, release: () => void, label: string) {
    super();
    this.#value = value;
    this.#release = release;
  }

  get value(): T {
    this.assertNotDropped();
    return this.#value;
  }

  drop(): void {
    this.#release();
  }
}

// ── RefMut<T> ────────────────────────────────────────────────────────────

/**
 * Exclusive mutable borrow guard for RefCell<T>.
 * Equivalent to Rust's std::cell::RefMut<T>.
 */
export class RefMut<T> extends Drop {
  #value: T;
  readonly #release: () => void;

  /** @internal */
  constructor(value: T, release: () => void, label: string) {
    super();
    this.#value = value;
    this.#release = release;
  }

  get value(): T {
    this.assertNotDropped();
    return this.#value;
  }

  set value(v: T) {
    this.assertNotDropped();
    this.#value = v;
  }

  drop(): void {
    this.#release();
  }
}
