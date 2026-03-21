// TS-ONLY: Maps Rust's std::sync::Arc<T> and std::sync::Weak<T> to JS (see E11)
//
// CRITICAL: In JS, `const x = arc` does NOT increment the refcount.
// You MUST use `arc.clone()` to create a new owning reference.

import { disposeSymbol, leakRegistry } from '../drop_registry.ts';

interface ArcInner<T> {
  value: T;
  strongCount: number;
  weakCount: number;
  dropped: boolean;
}

export class Arc<T> {
  readonly #inner: ArcInner<T>;
  #released = false;

  private constructor(inner: ArcInner<T>) {
    this.#inner = inner;
    const label = `Arc<${(inner.value as any)?.constructor?.name ?? '?'}>`;
    const creationStack = new Error().stack ?? '';
    leakRegistry.register(this, { label, creationStack, severity: 'fatal' }, this);
  }

  static new<T>(value: T): Arc<T> {
    return new Arc<T>({ value, strongCount: 1, weakCount: 0, dropped: false });
  }

  clone(): Arc<T> {
    if (this.#inner.dropped) throw new Error('Arc: cannot clone — inner already dropped');
    this.#inner.strongCount++;
    return new Arc<T>(this.#inner);
  }

  get value(): T {
    if (this.#inner.dropped) throw new Error('Arc: inner value has been dropped');
    return this.#inner.value;
  }

  drop(): void {
    if (this.#released) return;
    this.#released = true;
    leakRegistry.unregister(this);
    this.#inner.strongCount--;
    if (this.#inner.strongCount === 0) {
      this.#inner.dropped = true;
      const val = this.#inner.value;
      // Call drop glue (not just drop()) — triggers full cascade
      if (typeof (val as any)?.[disposeSymbol] === 'function') {
        (val as any)[disposeSymbol]();
      }
    }
  }

  downgrade(): Weak<T> {
    this.#inner.weakCount++;
    return new Weak<T>(this.#inner);
  }

  get strongCount(): number {
    return this.#inner.strongCount;
  }

  /** Identity-based pointer address (uses inner object identity) */
  asPtr(): number {
    // Use a WeakMap-based ID generator for stable identity
    return Arc.#ptrId(this.#inner);
  }

  static #ptrCounter = 0;
  static #ptrMap = new WeakMap<object, number>();
  static #ptrId(inner: object): number {
    let id = Arc.#ptrMap.get(inner);
    if (id === undefined) {
      id = ++Arc.#ptrCounter;
      Arc.#ptrMap.set(inner, id);
    }
    return id;
  }

  [disposeSymbol](): void {
    this.drop();
  }
}

export class Weak<T> {
  readonly #inner: ArcInner<T>;
  #released = false;

  /** @internal */
  constructor(inner: ArcInner<T>) {
    this.#inner = inner;
  }

  upgrade(): Arc<T> | null {
    if (this.#inner.dropped || this.#inner.strongCount === 0) return null;
    this.#inner.strongCount++;
    return new (Arc as any)(this.#inner);
  }

  /** Identity-based pointer address (same inner → same address) */
  asPtr(): number {
    return Arc.#ptrId(this.#inner);
  }

  drop(): void {
    if (this.#released) return;
    this.#released = true;
    if (this.#inner.weakCount > 0) this.#inner.weakCount--;
  }
}
