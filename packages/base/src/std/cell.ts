// TS-ONLY: Maps Rust's std::cell module to JS (see E11)
//
// Provides:
//   - RefCell<T>: runtime borrow checking (maps to std::cell::RefCell<T>)
//   - Ref<T>: shared borrow guard (maps to std::cell::Ref<T>)
//   - RefMut<T>: exclusive mutable borrow guard (maps to std::cell::RefMut<T>)
//
// See port/ownership.md and port/ownership/provided-types.md for API spec.

import { DropGuard } from './drop.ts';
import { ReadGuard, WriteGuard, dropContainer } from './guard.ts';
import type { Slot } from '../object.ts';

// ── BorrowState ──────────────────────────────────────────────────────────

type BorrowState =
  | { kind: 'not_borrowed' }
  | { kind: 'shared'; count: number }
  | { kind: 'mut_borrowed' };

// ── RefCell<T> ───────────────────────────────────────────────────────────
//
// 1:1 equivalent of Rust's std::cell::RefCell<T>. Runtime borrow checking —
// a conflicting borrow panics, just as it does in Rust.
//
// Usage:
//   const cell = new RefCell(value);
//   const guard = cell.borrowMut();
//   guard.value.field = 42;
//   guard.drop(); // borrow released

export class RefCell<T> {
  #value: T;
  #state: BorrowState = { kind: 'not_borrowed' };
  readonly #guard: DropGuard;
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
    this.#guard = new DropGuard(this, this.#label);
  }

  #slot(): Slot<T> {
    return {
      get: () => this.#value,
      set: (v) => { this.#value = v; },
    };
  }

  /**
   * Shared read-only borrow. Throws if a mutable borrow is active — a borrow
   * conflict panics in Rust too. Multiple shared borrows can be live at once.
   */
  borrow(): Ref<T> {
    this.#guard.assertNotDropped();
    if (this.#state.kind === 'mut_borrowed') {
      throw new Error(`${this.#label} already mutably borrowed — cannot take shared borrow`);
    }
    this.#state = {
      kind: 'shared',
      count: this.#state.kind === 'shared' ? this.#state.count + 1 : 1,
    };
    return new Ref<T>(this.#slot(), () => {
      if (this.#state.kind === 'shared') {
        this.#state = this.#state.count <= 1
          ? { kind: 'not_borrowed' }
          : { kind: 'shared', count: this.#state.count - 1 };
      }
    }, this.#label);
  }

  /** Exclusive mutable borrow. Throws if any borrow is active. */
  borrowMut(): RefMut<T> {
    this.#guard.assertNotDropped();
    if (this.#state.kind !== 'not_borrowed') {
      if (this.#state.kind === 'mut_borrowed') {
        throw new Error(`${this.#label} already mutably borrowed`);
      }
      throw new Error(`${this.#label} already shared-borrowed (count: ${this.#state.count})`);
    }
    this.#state = { kind: 'mut_borrowed' };
    return new RefMut<T>(this.#slot(), () => {
      this.#state = { kind: 'not_borrowed' };
      this.#onMutRelease?.();
    }, this.#label);
  }

  /**
   * Dropping a RefCell<T> in Rust drops the T inside it. The value sits in a
   * #private field the owning object's cascade cannot see, so the RefCell drops
   * it. A live borrow means the emitted drop scope is wrong, and releasing the
   * value under it would be the corruption Rust prevents.
   */
  drop(): void {
    dropContainer(
      this,
      this.#guard,
      this.#label,
      () => {
        if (this.#state.kind === 'mut_borrowed') return 'RefMut';
        if (this.#state.kind === 'shared') return 'Ref';
        return null;
      },
      () => this.#value,
    );
  }
}

/** Shared read-only borrow guard. Equivalent to Rust's std::cell::Ref<T>. */
export class Ref<T> extends ReadGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `Ref on ${label}`);
  }
}

/** Exclusive mutable borrow guard. Equivalent to Rust's std::cell::RefMut<T>. */
export class RefMut<T> extends WriteGuard<T> {
  /** @internal */
  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(slot, release, `RefMut on ${label}`);
  }
}
