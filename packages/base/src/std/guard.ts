// TS-ONLY: The shape every lock and borrow guard shares.
//
// Rust has five of these — MutexGuard, RwLockReadGuard, RwLockWriteGuard, Ref,
// RefMut — and they differ only in the state machine of the container that hands
// them out. The behaviour they share lives here so it is written once; the named
// subclasses stay, because emitted code should read like the Rust source.

import { Drop, type DropGuard } from './drop.ts';
import { dropOwned, type Slot } from '../object.ts';
import {
  assertNotPoisoned,
  fatalDoubleDrop,
  fatalOutstandingGuard,
  fatalSelfAssignment,
} from '../drop_registry.ts';

/**
 * A guard holds a borrow and releases it when dropped.
 *
 * Guards are the one type whose second drop is deliberately a no-op. The
 * emission model releases a guard temporary at the end of the statement that
 * produced it, then lists it again in the enclosing finally, so a guard being
 * dropped twice is by design. Everywhere else a second drop is fatal, and this
 * pre-check is the exception — do not "fix" it away.
 */
export abstract class Guard extends Drop {
  override drop(): void {
    if (this.isDropped) return;
    super.drop();
  }
}

// A Copy type in Rust cannot implement Drop, so re-storing one releases nothing
// and is legal. The emitter gives Copy types no drop glue, whatever class shape
// it picks for them, so that — not "is it a primitive" — is the test. Anything
// with drop glue assigned over itself is one value with two owners, which
// `*guard = *guard` will not compile to.
function isCopyLike(value: unknown): boolean {
  if (value === null) return true;
  if (typeof value !== 'object' && typeof value !== 'function') return true;
  return typeof (value as { drop?: unknown }).drop !== 'function';
}

/**
 * A guard over a container's contents. It reads through the container's own slot
 * rather than a snapshot, so it always sees what the container holds now, and it
 * owns nothing: the container is the owner, and dropping the guard releases only
 * the borrow.
 */
abstract class SlotGuard<T> extends Guard {
  readonly #slot: Slot<T>;
  readonly #release: () => void;

  constructor(slot: Slot<T>, release: () => void, label: string) {
    super(label);
    this.#slot = slot;
    this.#release = release;
  }

  get value(): T {
    this.assertNotDropped();
    return this.#slot.get();
  }

  /**
   * `*guard = v` in Rust: what the container held is dropped, and then it holds
   * v. The old value goes first, which is the order Rust runs it in.
   */
  protected store(v: T): void {
    const replaced = this.#slot.get();
    if (replaced === v && !isCopyLike(v)) fatalSelfAssignment(this.$label);
    dropOwned(replaced);
    this.#slot.set(v);
  }

  protected override onDrop(): void {
    this.#release();
  }
}

/** A shared borrow: readable, and assignment through it does not exist. */
export abstract class ReadGuard<T> extends SlotGuard<T> {}

/** An exclusive borrow: readable, and `guard.value = v` replaces the contents. */
export abstract class WriteGuard<T> extends SlotGuard<T> {
  override get value(): T {
    return super.value;
  }

  override set value(v: T) {
    this.assertNotDropped();
    this.store(v);
  }
}

/**
 * What all three containers do when they are dropped: refuse a second drop,
 * refuse to release their contents out from under a live guard, then leave the
 * registry and drop what was inside.
 *
 * @param outstanding — the name of a guard still holding this container, or null
 */
export function dropContainer(
  host: object,
  guard: DropGuard,
  label: string,
  outstanding: () => string | null,
  contents: () => unknown,
): void {
  assertNotPoisoned();
  if (guard.isDropped) fatalDoubleDrop(label);
  const held = outstanding();
  if (held !== null) fatalOutstandingGuard(label, held);
  guard.markDropped(host);
  dropOwned(contents());
}
