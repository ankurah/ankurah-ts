// MIRRORS: ankurah/core/src/property/value/entity_ref.rs

import { EntityId } from '@ankurah/proto';
import { Expr, Literal } from '@ankurah/ankql';

import type { Value } from '../../value/index.ts';
import { PropertyError } from '../traits.ts';
import type { ViewInstance } from '../../model.ts';

// Divergence: Context import is forward-declared to avoid circular dependency.
// The get() method uses a loose type; callers pass a real Context. [E8]
interface ContextLike {
  get<V extends ViewInstance>(id: EntityId): Promise<V>;
}

// ---------------------------------------------------------------------------
// Ref<T> — typed entity reference
// ---------------------------------------------------------------------------

/**
 * A typed reference to another entity.
 *
 * Stores an EntityId internally but carries compile-time type information
 * about the target model, enabling type-safe `.get()` calls.
 *
 * Rust: `pub struct Ref<T>`
 * Divergence: Rust uses PhantomData<T>; TS uses generic parameter (erased at runtime) [E4].
 * Divergence: Rust derives Serialize/Deserialize; TS uses intoValue()/fromValue() [E4].
 */
export class Ref<_T = unknown> {
  readonly id: EntityId;

  constructor(id: EntityId) {
    this.id = id;
  }

  // ── Factories ──

  /** Create a new Ref from an EntityId. Mirrors Rust Ref::new(). */
  static new<T = unknown>(id: EntityId): Ref<T> {
    return new Ref<T>(id);
  }

  /** Create a Ref from a base64-encoded EntityId string. Mirrors Rust Ref::from_base64(). */
  static fromBase64<T = unknown>(s: string): Ref<T> {
    return new Ref<T>(EntityId.fromBase64(s));
  }

  /** Create a Ref from an EntityId. Mirrors Rust From<EntityId> for Ref<T>. */
  static fromEntityId<T = unknown>(id: EntityId): Ref<T> {
    return new Ref<T>(id);
  }

  /** Create a Ref from a View instance. Mirrors Rust From<&V> for Ref<V::Model>. */
  static fromView<T = unknown>(view: ViewInstance): Ref<T> {
    return new Ref<T>(view.id());
  }

  // ── Accessors ──

  /** Get the underlying EntityId. Mirrors Rust Ref::id(). */
  entityId(): EntityId {
    return this.id;
  }

  // ── Fetch ──

  /**
   * Fetch the referenced entity from the given context.
   *
   * Rust: `pub async fn get(&self, ctx: &Context) -> Result<T::View, RetrievalError>`
   * Divergence: Throws RetrievalError instead of returning Result [A8].
   */
  async get<V extends ViewInstance>(ctx: ContextLike): Promise<V> {
    return ctx.get<V>(this.id);
  }

  // ── Conversions ──

  /** Convert to an EntityId. Mirrors Rust From<Ref<T>> for EntityId. */
  toEntityId(): EntityId {
    return this.id;
  }

  /** Convert to an ankql Expr for use in predicates. Mirrors Rust From<Ref<T>> for Expr. */
  toExpr(): Expr {
    return Expr.Literal(Literal.EntityId(this.id.toBytes()));
  }

  // ── Property ──

  /**
   * Convert to a Value. Mirrors Rust Property::into_value() for Ref<T>.
   *
   * Rust: `fn into_value(&self) -> Result<Option<Value>, PropertyError>`
   */
  intoValue(): Value {
    return { type: 'EntityId', value: this.id };
  }

  /**
   * Create a Ref from a Value. Mirrors Rust Property::from_value() for Ref<T>.
   * Throws PropertyError on failure.
   *
   * Rust: `fn from_value(value: Option<Value>) -> Result<Self, PropertyError>`
   */
  static fromValue<T = unknown>(value: Value | null): Ref<T> {
    if (value === null) {
      throw PropertyError.missing();
    }
    if (value.type === 'EntityId') {
      return Ref.new<T>(value.value);
    }
    // Backwards compatibility: accept string EntityIds (e.g., from older schema)
    if (value.type === 'String') {
      try {
        return Ref.fromBase64<T>(value.value);
      } catch (e) {
        throw PropertyError.invalidValue(value.value, `Ref (${e instanceof Error ? e.message : String(e)})`);
      }
    }
    throw PropertyError.invalidVariant(value, 'Ref');
  }

  // ── Display ──

  /** Mirrors Rust Display for Ref<T>. */
  toString(): string {
    return this.id.toBase64();
  }

  /** Equality check. Mirrors Rust PartialEq for Ref<T>. */
  equals(other: Ref<unknown>): boolean {
    return this.id.equals(other.id);
  }
}
