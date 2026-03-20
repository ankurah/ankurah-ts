// TS-ONLY: Base class for ported Rust enums
import { AkObject } from './object.ts';
import { disposeSymbol } from './drop_registry.ts';

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
 */
export class Enum<V extends Record<string, object> = Record<string, object>> extends AkObject {
  readonly type: string & keyof V;
  readonly value: V[keyof V];

  constructor(type: string & keyof V, value: V[keyof V]) {
    super();
    this.type = type;
    this.value = value;
  }

  match<R>(arms: { [K in keyof V]: (value: V[K]) => R }): R {
    const arm = (arms as any)[this.type];
    if (!arm) throw new Error(`Non-exhaustive match: missing arm for '${this.type}'`);
    return arm(this.value);
  }

  is<K extends keyof V>(variant: K): this is Enum<V> & { type: K; value: V[K] } {
    return this.type === variant;
  }

  override drop(): void {
    if (this.isDropped) return;
    super.drop();
    // Cascade disposal into the variant's value object
    for (const key of Object.getOwnPropertyNames(this.value)) {
      const field = (this.value as any)[key];
      if (field == null) continue;
      if (typeof field.drop === 'function') {
        field.drop();
      } else if (Array.isArray(field)) {
        for (const item of field) {
          if (item != null && typeof item.drop === 'function') {
            item.drop();
          } else if (Array.isArray(item)) {
            for (const inner of item) {
              if (inner != null && typeof inner.drop === 'function') {
                inner.drop();
              }
            }
          }
        }
      } else if (field instanceof Map) {
        for (const mapVal of field.values()) {
          if (mapVal == null) continue;
          if (typeof mapVal.drop === 'function') {
            mapVal.drop();
          } else if (Array.isArray(mapVal)) {
            for (const item of mapVal) {
              if (item != null && typeof item.drop === 'function') {
                item.drop();
              }
            }
          }
        }
      }
    }
  }

  toString(): string {
    return `${this.constructor.name}::${this.type}`;
  }
}
