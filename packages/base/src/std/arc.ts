// TS-ONLY: Maps Rust's std::sync::Arc<T> and std::sync::Weak<T> to JS (see E11)
//
// CRITICAL: In JS, `const x = arc` does NOT increment the refcount.
// You MUST use `arc.clone()` to create a new owning reference.

import {
  assertNotPoisoned,
  creationStack,
  disposeSymbol,
  fatalDoubleDrop,
  fatalUseAfterMove,
  leakRegistry,
} from '../drop_registry.ts';
import { DropGuard } from './drop.ts';
import { dropOwned } from '../object.ts';

interface ArcInner<T> {
  value: T;
  strongCount: number;
  weakCount: number;
  dropped: boolean;
}

// Module-private identity generator (shared by Arc and Weak)
let _ptrCounter = 0;
const _ptrMap = new WeakMap<object, number>();
function _ptrId(inner: object): number {
  let id = _ptrMap.get(inner);
  if (id === undefined) {
    id = ++_ptrCounter;
    _ptrMap.set(inner, id);
  }
  return id;
}

function labelFor(kind: string, value: unknown): string {
  return `${kind}<${(value as any)?.constructor?.name ?? '?'}>`;
}

export class Arc<T> {
  readonly #inner: ArcInner<T>;
  readonly #label: string;
  #released = false;

  private constructor(inner: ArcInner<T>) {
    this.#inner = inner;
    this.#label = labelFor('Arc', inner.value);
    leakRegistry.register(this, { label: this.#label, creationStack: creationStack() }, this);
  }

  static new<T>(value: T): Arc<T> {
    return new Arc<T>({ value, strongCount: 1, weakCount: 0, dropped: false });
  }

  /**
   * This handle is one owner among several. Releasing it ends THIS handle, even
   * while clones live on, so every accessor below is closed to it afterwards —
   * in Rust the moved-out handle is simply no longer nameable.
   */
  #assertLive(): void {
    assertNotPoisoned();
    if (this.#released) fatalUseAfterMove(this.#label);
  }

  clone(): Arc<T> {
    this.#assertLive();
    this.#inner.strongCount++;
    return new Arc<T>(this.#inner);
  }

  get value(): T {
    this.#assertLive();
    return this.#inner.value;
  }

  // Dropping one handle twice is a bug even though other handles may still hold
  // the value: each handle is its own owner, and the refcount would go one lower
  // than the number of owners that actually let go.
  drop(): void {
    assertNotPoisoned();
    if (this.#released) fatalDoubleDrop(this.#label);
    this.#released = true;
    leakRegistry.unregister(this);
    this.#inner.strongCount--;
    if (this.#inner.strongCount === 0) {
      this.#inner.dropped = true;
      // Last strong reference gone, so Rust drops the contents here. Going
      // through dropOwned reaches every kind of owned slot, not just AkObject:
      // Arc<Mutex<T>> and Arc<Vec<T>> are dropped as thoroughly as Arc<Struct>.
      const contents = this.#inner.value;
      // Let go of the payload before dropping it, so a Weak that outlives this
      // Arc holds only the bookkeeping and not the whole dropped object graph.
      // Rust empties the allocation's value slot when the last strong handle
      // goes and leaves the weak count behind; T cannot spell an empty slot, so
      // the emptiness is guarded by `dropped` instead of by the type.
      // Divergence: no T can represent the value slot Rust has already freed.
      this.#inner.value = undefined as unknown as T;
      dropOwned(contents);
    }
  }

  downgrade(): Weak<T> {
    this.#assertLive();
    this.#inner.weakCount++;
    return new Weak<T>(this.#inner);
  }

  get strongCount(): number {
    this.#assertLive();
    return this.#inner.strongCount;
  }

  /** Identity-based pointer address (uses inner object identity) */
  asPtr(): number {
    this.#assertLive();
    return _ptrId(this.#inner);
  }

  [disposeSymbol](): void {
    this.drop();
  }
}

/**
 * A non-owning handle. It does not keep the value alive, but Rust still runs
 * drop glue for it — it decrements the weak count — so a Weak that is never
 * dropped is a leak like anything else.
 */
export class Weak<T> {
  readonly #inner: ArcInner<T>;
  readonly #label: string;
  readonly #guard: DropGuard;

  /** @internal — produced by Arc.downgrade() and Weak.clone() */
  constructor(inner: ArcInner<T>) {
    this.#inner = inner;
    this.#label = labelFor('Weak', inner.value);
    this.#guard = new DropGuard(this, this.#label);
  }

  clone(): Weak<T> {
    this.#guard.assertNotDropped();
    this.#inner.weakCount++;
    return new Weak<T>(this.#inner);
  }

  upgrade(): Arc<T> | null {
    this.#guard.assertNotDropped();
    if (this.#inner.dropped || this.#inner.strongCount === 0) return null;
    this.#inner.strongCount++;
    return new (Arc as any)(this.#inner);
  }

  /** Identity-based pointer address (same inner → same address) */
  asPtr(): number {
    this.#guard.assertNotDropped();
    return _ptrId(this.#inner);
  }

  get weakCount(): number {
    this.#guard.assertNotDropped();
    return this.#inner.weakCount;
  }

  drop(): void {
    assertNotPoisoned();
    if (this.#guard.isDropped) fatalDoubleDrop(this.#label);
    this.#guard.markDropped(this);
    if (this.#inner.weakCount > 0) this.#inner.weakCount--;
  }
}
