// TS-ONLY: Base class for all ported Rust types (see E11)

import {
  assertNotPoisoned,
  creationStack,
  disposeSymbol,
  fatalDoubleDrop,
  fatalUseAfterDrop,
  fatalUseAfterMove,
  leakRegistry,
} from './drop_registry.ts';

/**
 * The storage a container keeps its contents in, handed to a guard so the guard
 * reads and writes the container's own field instead of a copy of it. This is
 * what makes `*guard = v` in Rust — which replaces what the container holds —
 * translate to `guard.value = v` here.
 */
export interface Slot<T> {
  get(): T;
  set(value: T): void;
}

/**
 * Marks a provided type that owns nothing by definition — `&T` and `&mut T`,
 * whose drop in Rust releases nothing. The cascade steps over these in silence
 * instead of warning about an unwrapped foreign object.
 *
 * A fresh Symbol rather than Symbol.for, so it cannot be looked up from the
 * global registry: only code that can import it — this package — grants it.
 */
export const nonOwning: unique symbol = Symbol('ankurah.nonOwning');

// Objects the cascade will never own anything through, so reaching one is not a
// sign of a missing wrapper. Everything else without drop glue gets one warning.
function isInert(val: object): boolean {
  return (
    (val as any)[nonOwning] === true ||
    ArrayBuffer.isView(val) ||
    val instanceof ArrayBuffer ||
    val instanceof Promise ||
    val instanceof Date ||
    val instanceof RegExp ||
    val instanceof Error
  );
}

const warnedForeign = new Set<string>();

function warnForeign(val: object): void {
  const name = val.constructor?.name ?? '(anonymous)';
  if (warnedForeign.has(name)) return;
  warnedForeign.add(name);
  console.warn(
    `ankurah: the drop cascade reached a ${name}, which has no drop glue.\n` +
    `Whatever it owns will not be released. A ported type extends Struct, Enum or\n` +
    `Drop; a foreign object needs a provided type to wrap it.`,
  );
}

/**
 * Drop one owned slot — the unit of work the cascade applies to each thing it
 * walks: a struct field, an enum variant field, an Arc's contents, or the value
 * inside a Mutex/RwLock/RefCell.
 *
 * A value with drop glue is dropped, and that drop cascades onward on its own.
 * A collection owns its elements, so an array, Map or Set is walked and each
 * element dropped the same way, to any depth — Rust drops a Vec<Vec<T>> all the
 * way down. A Map is walked over its keys as well as its values, because a
 * HashMap<K, V> owns both. A plain object — one whose prototype is
 * Object.prototype or null, so a record the port emitted rather than an instance
 * of some type — is walked over its own enumerable values. A primitive owns
 * nothing and is let go.
 *
 * Arc ends the walk: it decrements, and only the last strong drop cascades into
 * the contents. Nothing here tracks which objects it has already seen, because
 * in Rust an owned value has exactly one owner. So reaching the same object
 * twice in one cascade — a map key that some other value also owns, say — means
 * the emitter aliased an owned value, and the second drop reports it as fatal.
 */
export function dropOwned(value: unknown): void {
  const val = value as any;
  if (val == null) return;
  if (typeof val.drop === 'function') {
    val.drop();
  } else if (Array.isArray(val)) {
    for (const item of val) dropOwned(item);
  } else if (val instanceof Map) {
    for (const [key, mapVal] of val) {
      dropOwned(key);
      dropOwned(mapVal);
    }
  } else if (val instanceof Set) {
    for (const item of val) dropOwned(item);
  } else if (typeof val === 'object') {
    const proto = Object.getPrototypeOf(val);
    if (proto === Object.prototype || proto === null) {
      for (const field of Object.values(val)) dropOwned(field);
    } else if (!isInert(val)) {
      warnForeign(val);
    }
  }
}

export class AkObject {
  readonly #label: string;
  #dropped = false;
  #moved = false;

  /** @param label — what to call this value in diagnostics. Defaults to the
   *  class name; a guard or a container field passes the site it stands for. */
  constructor(label?: string) {
    this.#label = label ?? this.constructor.name;
    leakRegistry.register(this, { label: this.#label, creationStack: creationStack() }, this);
  }

  /** What diagnostics call this value. */
  protected get label(): string { return this.#label; }

  /** Released by its owner. */
  get isDropped(): boolean { return this.#dropped; }

  /**
   * Handed away by a method that took `self`. Kept separate from isDropped
   * rather than folded into it, because the two are different bugs with
   * different fixes — a dropped value was released here, a moved one was given
   * to somebody else — and because a guard's deliberate idempotence pre-check
   * reads isDropped, which would then silently swallow a use-after-move.
   */
  get isMoved(): boolean { return this.#moved; }

  /**
   * Drop this value. This is the whole template and subclasses do not override
   * it: mark, unregister, run the type's own cleanup while its fields are still
   * alive, then drop the fields. Rust runs `Drop::drop` before dropping fields,
   * which is why onDrop() comes first — a cleanup body that reads a field would
   * otherwise find it already released.
   *
   * The transpiler emits an onDrop() body for `impl Drop`, and emits .drop()
   * calls for scope cleanup.
   */
  drop(): void {
    assertNotPoisoned();
    if (this.#moved) fatalUseAfterMove(this.#label);
    if (this.#dropped) fatalDoubleDrop(this.#label);
    this.#dropped = true;
    leakRegistry.unregister(this);
    try {
      this.onDrop();
    } finally {
      // Deduped by identity: a subclass that exposes its payload both as an own
      // property and through ownedFields() would otherwise drop it twice, and a
      // second drop is fatal.
      const seen = new Set<unknown>();
      for (const field of this.ownedFields()) {
        if (seen.has(field)) continue;
        seen.add(field);
        dropOwned(field);
      }
    }
  }

  /** Rust's `impl Drop` body. Runs with every field still alive; no-op by default. */
  protected onDrop(): void {}

  /**
   * The values this object owns, which drop() releases after onDrop() has run.
   * The default is every own property. A type that keeps its state in #private
   * fields — Enum, the containers — overrides this to hand them over, since the
   * cascade cannot see private state.
   */
  protected ownedFields(): unknown[] {
    return Object.getOwnPropertyNames(this).map((key) => (this as any)[key]);
  }

  /** Symbol.dispose — delegates to drop() */
  [disposeSymbol](): void {
    this.drop();
  }

  protected assertNotDropped(): void {
    assertNotPoisoned();
    if (this.#moved) fatalUseAfterMove(this.#label);
    if (this.#dropped) fatalUseAfterDrop(this.#label);
  }

  /**
   * Rust's move-out of `self`: a method took this value by value, its payload
   * now belongs to somebody else, and this object no longer exists. It leaves
   * the leak registry without cascading — there is nothing here left to release,
   * so a moved value is not a leak — and every later use of it is fatal.
   */
  protected markMoved(): void {
    this.#moved = true;
    leakRegistry.unregister(this);
  }
}
