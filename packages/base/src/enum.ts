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
    if (!arm) fatalNonExhaustiveMatch(this.label, String(this.#type));
    return arm(this.#value);
  }

  is<K extends keyof V>(variant: K): this is Enum<V> & { type: K; value: V[K] } {
    this.assertNotDropped();
    return this.#type === variant;
  }

  // The payload is #private, so the cascade cannot see it by walking properties.
  protected override ownedFields(): unknown[] {
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
