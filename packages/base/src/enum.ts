// TS-ONLY: Base class for ported Rust enums
import { AkObject } from './object.ts';
import { disposeSymbol } from './drop_registry.ts';

/**
 * V = variant map: { VariantName: DataType, ... }
 * Unit variants use {} (empty object). Data variants use { field: Type, ... }.
 *
 * Example:
 *   type CausalRelationV = {
 *     Equal: {};
 *     DivergedSince: { meet: Clock; subject: Clock; other: Clock };
 *     BudgetExceeded: { subject: Clock; other: Clock };
 *   };
 *   class CausalRelation extends Enum<CausalRelationV> {
 *     static Equal = () => new CausalRelation('Equal', {});
 *     static DivergedSince = (v: CausalRelationV['DivergedSince']) => new CausalRelation('DivergedSince', v);
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

  override [disposeSymbol](): void {
    if (this.isDropped) return;
    for (const key of Object.getOwnPropertyNames(this.value)) {
      const field = (this.value as any)[key];
      if (typeof field?.[disposeSymbol] === 'function') {
        field[disposeSymbol]();
      }
    }
    super[disposeSymbol]();
  }

  toString(): string {
    return `${this.constructor.name}::${this.type}`;
  }
}
