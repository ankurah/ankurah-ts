// TS-ONLY: functional replacement for #[derive(Model)]
//
// Exception E1: Rust uses #[derive(Model)] proc macro to generate View, Mutable, and Model
// implementations at compile time. TypeScript has no proc macro system, so we use a
// runtime defineModel() function that produces equivalent View/Mutable classes with
// typed accessors.

import type { CollectionId, EntityId, State } from '@ankurah/proto';
import type { ViewInstance, MutableInstance, ViewConstructor, ModelDefinition } from './model.ts';
import type { Entity } from './entity.ts';

// ---------------------------------------------------------------------------
// Field definition types
// ---------------------------------------------------------------------------

/**
 * Discriminant for the property backend that stores a field's data.
 *
 * Matches the Rust backend names used in backend_registry:
 * - 'lww' corresponds to LWW (Last-Writer-Wins register)
 * - 'yjs' corresponds to YrsString / YrsText (Yjs CRDT text)
 * - 'ephemeral' is TS-only for non-persisted fields
 */
export type BackendKind = 'lww' | 'yjs' | 'ephemeral';

/**
 * Metadata for a single field within a model.
 *
 * This is the runtime representation of what Rust's derive macro computes at compile time
 * from the field type and #[active_type(...)] attribute.
 *
 * The generic parameters encode:
 * - Projected: The value type returned by View getters (e.g., string, number)
 * - Active: The active/mutable handle type returned by Mutable getters (e.g., LWW<string>)
 */
export interface FieldDefinition<Projected = unknown, Active = unknown> {
  /** Which backend stores this field's data */
  readonly backend: BackendKind;

  /**
   * Marker for the projected (read) type.
   * This is never called at runtime; it exists purely for TypeScript type inference.
   */
  readonly _projected: Projected;

  /**
   * Marker for the active (mutable handle) type.
   * This is never called at runtime; it exists purely for TypeScript type inference.
   */
  readonly _active: Active;
}

// ---------------------------------------------------------------------------
// Field definition helpers
// ---------------------------------------------------------------------------

/**
 * Placeholder type for the LWW (Last-Writer-Wins) active handle.
 * The real implementation will be in property/value/lww.ts.
 *
 * View getter returns T directly.
 * Mutable getter returns an LWW<T> handle with get()/set() methods.
 */
export interface LWW<T> {
  /** Get the current value */
  get(): T;
  /** Set a new value */
  set(value: T): void;
}

/**
 * Placeholder type for the YjsText active handle.
 * The real implementation will be in property/value/yjs.ts (Exception E5: yrs -> yjs).
 *
 * View getter returns string directly.
 * Mutable getter returns a YjsText handle with insert()/delete()/toString() methods.
 */
export interface YjsText {
  /** Insert text at the given position */
  insert(index: number, text: string): void;
  /** Delete count characters starting at index */
  delete(index: number, length: number): void;
  /** Get the full text content */
  toString(): string;
}

/**
 * Define an LWW-backed field.
 *
 * In Rust, LWW fields are inferred from the field type or specified with #[active_type(LWW)].
 * The derive macro maps projected types (String, i32, bool, etc.) to their LWW<T> active types.
 *
 * View getter returns T (the projected value).
 * Mutable getter returns LWW<T> (a handle with get/set).
 *
 * @example
 * ```typescript
 * const Video = defineModel('video', {
 *   visibility: lww<string>(),
 *   createdAt: lww<number>(),
 * });
 * ```
 */
export function lww<T>(): FieldDefinition<T, LWW<T>> {
  return {
    backend: 'lww',
    _projected: undefined as unknown as T,
    _active: undefined as unknown as LWW<T>,
  };
}

/**
 * Define a Yjs text-backed field (collaborative rich text).
 *
 * In Rust, this corresponds to YrsString / #[active_type(YrsString)].
 * Exception E5: Rust uses Yrs (the Rust port); TS uses Yjs (the original JS library).
 *
 * View getter returns string.
 * Mutable getter returns YjsText (a handle with insert/delete/toString).
 *
 * @example
 * ```typescript
 * const Video = defineModel('video', {
 *   title: yrsText(),
 *   description: yrsText(),
 * });
 * ```
 */
export function yrsText(): FieldDefinition<string, YjsText> {
  return {
    backend: 'yjs',
    _projected: undefined as unknown as string,
    _active: undefined as unknown as YjsText,
  };
}

/**
 * Define an ephemeral (non-persisted) field.
 *
 * In Rust, these are marked with #[model(ephemeral)] and are not stored in any backend.
 * They exist on the View/Mutable structs but are not serialized or synced.
 *
 * Both View and Mutable getters return T directly.
 *
 * @example
 * ```typescript
 * const Video = defineModel('video', {
 *   title: yrsText(),
 *   _localPlaybackState: ephemeral<string>(),
 * });
 * ```
 */
export function ephemeral<T>(): FieldDefinition<T, T> {
  return {
    backend: 'ephemeral',
    _projected: undefined as unknown as T,
    _active: undefined as unknown as T,
  };
}

// ---------------------------------------------------------------------------
// Type-level utilities
// ---------------------------------------------------------------------------

/**
 * Extract the projected type map from a fields definition object.
 * Maps each field name to its View getter return type.
 *
 * Given `{ title: FieldDefinition<string, YjsText>, age: FieldDefinition<number, LWW<number>> }`,
 * produces `{ title: string, age: number }`.
 */
type ProjectedTypes<F extends Record<string, FieldDefinition>> = {
  [K in keyof F]: F[K] extends FieldDefinition<infer P, any> ? P : never;
};

/**
 * Extract the active type map from a fields definition object.
 * Maps each field name to its Mutable getter return type.
 *
 * Given `{ title: FieldDefinition<string, YjsText>, age: FieldDefinition<number, LWW<number>> }`,
 * produces `{ title: YjsText, age: LWW<number> }`.
 */
type ActiveTypes<F extends Record<string, FieldDefinition>> = {
  [K in keyof F]: F[K] extends FieldDefinition<any, infer A> ? A : never;
};

/**
 * The View interface generated for a model with fields F.
 * Extends ViewInstance with typed getter methods for each field.
 *
 * Each field becomes a method that returns the projected type:
 * `title(): string`, `age(): number`, etc.
 *
 * Mirrors how the Rust derive macro generates:
 * ```rust
 * pub fn title(&self) -> Result<String, PropertyError> { ... }
 * ```
 * But in TS we throw on error rather than returning Result [A8].
 */
type GeneratedView<F extends Record<string, FieldDefinition>> = ViewInstance & {
  [K in keyof F & string]: () => ProjectedTypes<F>[K];
};

/**
 * The Mutable interface generated for a model with fields F.
 * Extends MutableInstance with typed getter methods for each field.
 *
 * Each field becomes a method that returns the active type handle:
 * `title(): YjsText`, `visibility(): LWW<string>`, etc.
 *
 * Mirrors how the Rust derive macro generates:
 * ```rust
 * pub fn title(&self) -> YrsString<String> { ... }
 * ```
 */
type GeneratedMutable<F extends Record<string, FieldDefinition>> = MutableInstance & {
  [K in keyof F & string]: () => ActiveTypes<F>[K];
};

// ---------------------------------------------------------------------------
// FieldMetadata — runtime field descriptor
// ---------------------------------------------------------------------------

/**
 * Runtime metadata for a field within a model.
 * Used by the framework for introspection (e.g., building queries, initializing backends).
 */
export interface FieldMetadata {
  /** The field name (e.g., "title", "visibility") */
  readonly name: string;
  /** Which backend stores this field */
  readonly backend: BackendKind;
}

// ---------------------------------------------------------------------------
// defineModel return type
// ---------------------------------------------------------------------------

/**
 * The complete model definition object returned by defineModel().
 *
 * This is the TS equivalent of what #[derive(Model)] generates in Rust:
 * - A View struct with typed read accessors
 * - A Mutable struct with typed write accessors
 * - The Model trait impl with collection() and initialize_new_entity()
 */
export interface DefinedModel<F extends Record<string, FieldDefinition>> extends ModelDefinition<GeneratedView<F>, GeneratedMutable<F>> {
  /** The collection name string */
  readonly collectionName: string;

  /** Runtime metadata for each field */
  readonly fields: FieldMetadata[];

  /** The View class — construct with View.fromEntity(entity) */
  readonly View: ViewConstructor<GeneratedView<F>> & {
    fromEntity(entity: Entity): GeneratedView<F>;
  };

  /** The Mutable class — construct with new Mutable(entity) */
  readonly Mutable: new (entity: Entity) => GeneratedMutable<F>;
}

// ---------------------------------------------------------------------------
// defineModel()
// ---------------------------------------------------------------------------

/**
 * Define a model with typed fields.
 *
 * This is the TS equivalent of Rust's `#[derive(Model)]` proc macro (Exception E1).
 * It takes a collection name and a record of field definitions, and returns a
 * model definition object with typed View and Mutable classes.
 *
 * The returned View class has getter methods for each field that return the projected
 * (read) type. The returned Mutable class has getter methods that return the active
 * (mutable handle) type.
 *
 * @param collectionName - The collection identifier (lowercase model name in Rust convention)
 * @param fields - Record of field name to FieldDefinition (created via lww(), yrsText(), etc.)
 * @returns A DefinedModel with View class, Mutable class, and metadata
 *
 * @example
 * ```typescript
 * const Video = defineModel('video', {
 *   title: yrsText(),
 *   description: yrsText(),
 *   visibility: lww<string>(),
 *   createdAt: lww<number>(),
 * });
 *
 * // View usage:
 * const view = Video.View.fromEntity(entity);
 * const title: string = view.title();
 *
 * // Mutable usage:
 * const mut = new Video.Mutable(entity);
 * mut.title().insert(0, 'Hello');
 * mut.visibility().set('public');
 * ```
 */
export function defineModel<F extends Record<string, FieldDefinition>>(
  collectionName: string,
  fields: F,
): DefinedModel<F> {
  // Build field metadata array
  const fieldMetadata: FieldMetadata[] = Object.entries(fields).map(([name, def]) => ({
    name,
    backend: def.backend,
  }));

  const fieldNames = Object.keys(fields);

  // ---- View class ----
  // Mirrors the struct generated by derive/src/model/view.rs
  class GeneratedViewClass implements ViewInstance {
    private readonly _entity: Entity;

    constructor(entity: Entity) {
      this._entity = entity;
    }

    id(): EntityId {
      return this._entity.id();
    }

    collection(): CollectionId {
      return collectionName as unknown as CollectionId;
    }

    entity(): Entity {
      return this._entity;
    }

    static fromEntity(entity: Entity): GeneratedViewClass {
      return new GeneratedViewClass(entity);
    }
  }

  // Add typed getter methods to View prototype for each field.
  // Mirrors the Rust derive macro output:
  //   pub fn title(&self) -> Result<String, PropertyError> {
  //       use ankurah::property::{FromActiveType, FromEntity};
  //       CurrentObserver::track(self);
  //       let active_result = YrsString::<String>::from_entity("title".into(), &self.entity);
  //       String::from_active(active_result)
  //   }
  for (const fieldName of fieldNames) {
    Object.defineProperty(GeneratedViewClass.prototype, fieldName, {
      value: function (this: GeneratedViewClass) {
        // TODO: Wire up actual backend property retrieval once Entity and PropertyBackend are ported.
        // This will call FromEntity to get the active type, then FromActiveType to project.
        const entity = this.entity();
        if (entity && typeof entity.getPropertyValue === 'function') {
          return entity.getPropertyValue(fieldName);
        }
        return undefined;
      },
      writable: false,
      enumerable: true,
      configurable: false,
    });
  }

  // ---- Mutable class ----
  // Mirrors the struct generated by derive/src/model/mutable.rs
  class GeneratedMutableClass implements MutableInstance {
    readonly _entity: Entity;

    constructor(entity: Entity) {
      this._entity = entity;
    }

    id(): EntityId {
      return this._entity.id();
    }

    collection(): CollectionId {
      return collectionName as unknown as CollectionId;
    }

    entity(): Entity {
      return this._entity;
    }

    state(): State {
      return this._entity.toState();
    }

    read(): ViewInstance {
      // Rust: fn read(&self) -> Self::View { ... }
      // Simplified version - full implementation will handle EntityKind::Transacted upstream cloning
      return GeneratedViewClass.fromEntity(this._entity);
    }
  }

  // Add typed getter methods to Mutable prototype for each field.
  // Mirrors the Rust derive macro output:
  //   pub fn title(&self) -> YrsString<String> {
  //       use ankurah::property::FromEntity;
  //       YrsString::<String>::from_entity("title".into(), &self.entity)
  //   }
  for (const fieldName of fieldNames) {
    Object.defineProperty(GeneratedMutableClass.prototype, fieldName, {
      value: function (this: GeneratedMutableClass) {
        // TODO: Wire up actual backend active type retrieval once Entity and PropertyBackend are ported.
        // This will call FromEntity to get the active type handle.
        const entity = this.entity();
        if (entity && typeof entity.getActiveHandle === 'function') {
          return entity.getActiveHandle(fieldName, (fields as Record<string, FieldDefinition>)[fieldName].backend);
        }
        return undefined;
      },
      writable: false,
      enumerable: true,
      configurable: false,
    });
  }

  // ---- Model definition object ----
  const modelDef: DefinedModel<F> = {
    collectionName,
    fields: fieldMetadata,

    View: GeneratedViewClass as unknown as DefinedModel<F>['View'],
    Mutable: GeneratedMutableClass as unknown as DefinedModel<F>['Mutable'],

    collection(): CollectionId {
      return collectionName as unknown as CollectionId;
    },

    initializeNewEntity(entity: Entity, values: Record<string, unknown>): void {
      // Rust: fn initialize_new_entity(&self, entity: &Entity) {
      //     use ::ankurah::property::InitializeWith;
      //     YrsString::<String>::initialize_with(&entity, "title".into(), &self.title);
      //     LWW::<String>::initialize_with(&entity, "visibility".into(), &self.visibility);
      // }
      // TODO: Wire up actual InitializeWith calls once property backends are ported.
      for (const [name, def] of Object.entries(fields)) {
        if (name in values && entity && typeof entity.initializeProperty === 'function') {
          entity.initializeProperty(name, values[name], (def as FieldDefinition).backend);
        }
      }
    },
  };

  return modelDef;
}
