// TS-ONLY: Base class for ported Rust enums
import { AkObject } from './object.ts';
import { fatalNonExhaustiveMatch } from './drop_registry.ts';

/**
 * V = variant map: { VariantName: DataType, ... }
 * Unit variants use {} (empty object). Data variants use { field: Type, ... }.
 *
 * Usage:
 *   type DeltaContentV = {
 *     StateSnapshot: { state: StateFragment };
 *     EventBridge: { events: EventFragment[] };
 *   };
 *   class DeltaContent extends Enum<DeltaContentV> {
 *     static StateSnapshot = (v: DeltaContentV['StateSnapshot']) => new DeltaContent('StateSnapshot', v);
 *     static EventBridge = (v: DeltaContentV['EventBridge']) => new DeltaContent('EventBridge', v);
 *   }
 *
 * The variant and its payload are held privately and reached through getters, so
 * that reading either one goes through the same liveness check as everything
 * else: a dropped or moved enum cannot be read behind the runtime's back. The
 * getters keep property syntax, so emitted code still says `e.type` and `e.value`.
 */
export class Enum<V extends Record<string, object> = Record<string, object>> extends AkObject {
  readonly #type: string & keyof V;
  readonly #value: V[keyof V];
  #payloadMoved = false;

  constructor(type: string & keyof V, value: V[keyof V]) {
    super();
    this.#type = type;
    this.#value = value;
  }

  get type(): string & keyof V {
    this.assertNotDropped();
    return this.#type;
  }

  get value(): V[keyof V] {
    this.assertNotDropped();
    return this.#value;
  }

  /**
   * Borrows: `match` reads the payload and leaves this enum whole, which is what
   * the emitter needs where the Rust source matches on a reference. A consuming
   * form, for `match self`, comes separately.
   */
  match<R>(arms: { [K in keyof V]: (value: V[K]) => R }): R {
    this.assertNotDropped();
    const arm = (arms as any)[this.#type];
    if (!arm) fatalNonExhaustiveMatch(this.$label, String(this.#type));
    return arm(this.#value);
  }

  /**
   * `match` on a scrutinee taken by value: the arm's bindings own the payload
   * from here, and this enum is gone.
   *
   * Named for Rust's `into_` convention, which is what a by-value conversion is
   * spelled with — `match self { … }` has no name of its own to borrow, and
   * calling this one `matchOwned` or `consume` would say less about what the
   * arm receives. It leaves this enum moved rather than dropped, exactly as
   * `Result`'s self-taking methods do, so the emitter emits no drop after one
   * and every later use is fatal.
   *
   * `Result` inherits this, and its `unwrap()` / `unwrapErr()` / `ok()` family
   * are the same move written out for the two variants it has.
   */
  intoMatch<R>(arms: { [K in keyof V]: (value: V[K]) => R }): R {
    this.assertNotDropped();
    const arm = (arms as any)[this.#type];
    if (!arm) fatalNonExhaustiveMatch(this.$label, String(this.#type));
    const payload = this.#value;
    // The payload belongs to the arm now, so the cascade must not reach it —
    // and this enum, being moved, is never dropped and never a leak.
    this.#payloadMoved = true;
    this.markMoved();
    // ONE UNWIND OWNER, and it is the arm. The arm receives the payload, takes
    // names out of it and releases the rest in its own `finally`, so it holds
    // every part of the payload on every path out — including a throw. This
    // used to drop the payload again on the way past, and an arm that had
    // already released a binding then saw `BUG: … was dropped twice` in place
    // of its own exception. An arm that leaves the payload unowned leaks it
    // rather than being rescued here: a leak is reported at the site that
    // caused it, where a double drop is reported at the innocent one.
    return arm(payload);
  }

  is<K extends keyof V>(variant: K): this is Enum<V> & { type: K; value: V[K] } {
    this.assertNotDropped();
    return this.#type === variant;
  }

  // The payload is #private, so the cascade cannot see it by walking properties
  // — unless intoMatch has already handed it to an arm, in which case it is no
  // longer this enum's to release.
  protected override ownedFields(): unknown[] {
    if (this.#payloadMoved) return super.ownedFields();
    return [...super.ownedFields(), this.#value];
  }

  /**
   * Deliberately does not assert liveness. Rendering a value is what a panic
   * message and a debugger do, and both run precisely when something has gone
   * wrong — so this reports the state rather than raising a second fault on top
   * of the first.
   */
  toString(): string {
    const state = this.isMoved ? ' (moved)' : this.isDropped ? ' (dropped)' : '';
    return `${this.constructor.name}::${this.#type}${state}`;
  }
}
