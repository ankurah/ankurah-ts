// MIRRORS: ankurah/core/src/property/backend/mod.rs

import type { Operation } from '@ankurah/proto';
import type { Listener, ListenerGuard } from '@ankurah/signals';

import type { PropertyName } from '../index.ts';
import type { Value } from '../../value/index.ts';

// Rust: pub use lww::LWWBackend;
import { LWWBackend } from './lww.ts';
export { LWWBackend };

// Rust: pub use yrs::YrsBackend;
// Exception E5: yrs -> yjs (library rename)
import { YjsBackend } from './yjs.ts';
export { YjsBackend };

// ---------------------------------------------------------------------------
// PropertyBackend interface
// ---------------------------------------------------------------------------

/**
 * Core abstraction for CRDT property backends.
 *
 * Rust: `trait PropertyBackend: Any + Send + Sync + Debug + 'static`
 * Divergence: Rust requires Send + Sync + Debug + 'static; TS has none of these constraints [E8].
 * Divergence: Rust uses Arc<Self> / Arc<dyn PropertyBackend>; TS uses plain references [E8].
 * Divergence: Rust `as_arc_dyn_any` and `as_debug` are Rust-specific trait-object helpers; omitted in TS [E8].
 */
export interface PropertyBackend {
  /**
   * Get the list of property names managed by this backend.
   *
   * Rust: `fn properties(&self) -> Vec<PropertyName>`
   */
  properties(): PropertyName[];

  /**
   * Get the value of a specific property.
   * Default implementation delegates to propertyValues().
   *
   * Rust: `fn property_value(&self, property_name: &PropertyName) -> Option<Value>`
   */
  propertyValue(propertyName: PropertyName): Value | null;

  /**
   * Get all property values as a map.
   *
   * Rust: `fn property_values(&self) -> BTreeMap<PropertyName, Option<Value>>`
   */
  propertyValues(): Map<PropertyName, Value | null>;

  /**
   * Get the latest state buffer for this property backend.
   * Throws StateError on failure.
   *
   * Rust: `fn to_state_buffer(&self) -> Result<Vec<u8>, StateError>`
   */
  toStateBuffer(): Uint8Array;

  /**
   * Retrieve operations applied to this backend since the last time we called this method.
   * Returns null if no operations have been applied.
   * Throws MutationError on failure.
   *
   * Rust: `fn to_operations(&self) -> Result<Option<Vec<Operation>>, MutationError>`
   */
  toOperations(): Operation[] | null;

  /**
   * Apply operations to this backend.
   * Throws MutationError on failure.
   *
   * Rust: `fn apply_operations(&self, operations: &Vec<Operation>) -> Result<(), MutationError>`
   */
  applyOperations(operations: Operation[]): void;

  /**
   * Create a fork (deep copy) of this backend.
   *
   * Rust: `fn fork(&self) -> Arc<dyn PropertyBackend>`
   * Divergence: Returns plain PropertyBackend, not Arc [E8].
   */
  fork(): PropertyBackend;

  /**
   * Listen to changes for a specific field managed by this backend.
   * Auto-creates the broadcast if it doesn't exist yet.
   * Returns a subscription guard that will unsubscribe when disposed.
   *
   * Rust: `fn listen_field(&self, field_name: &PropertyName, listener: Listener) -> ListenerGuard`
   * Divergence: Rust returns ankurah_signals::signal::ListenerGuard [E8].
   */
  listenField(fieldName: PropertyName, listener: Listener): ListenerGuard;
}

// ---------------------------------------------------------------------------
// PropertyBackendStatic — static methods that Rust puts on the trait with `where Self: Sized`
// ---------------------------------------------------------------------------

/**
 * Static methods for PropertyBackend that Rust defines with `where Self: Sized`.
 * In Rust these are associated functions on the trait. In TS, we model them as
 * a separate interface for the constructor/class side.
 *
 * Implementors (YjsBackend, LWWBackend) should satisfy this interface at the class level.
 */
export interface PropertyBackendStatic {
  /**
   * Unique property backend identifier (e.g., "yjs", "lww").
   *
   * Rust: `fn property_backend_name() -> String where Self: Sized`
   */
  propertyBackendName(): string;

  /**
   * Construct a property backend from a state buffer.
   * Throws RetrievalError on failure.
   *
   * Rust: `fn from_state_buffer(state_buffer: &Vec<u8>) -> Result<Self, RetrievalError> where Self: Sized`
   */
  fromStateBuffer(buffer: Uint8Array): PropertyBackend;

  /**
   * Construct a new empty backend.
   */
  new(): PropertyBackend;
}

// ---------------------------------------------------------------------------
// backendFromString — factory function (signature only, implementation deferred)
// ---------------------------------------------------------------------------

/**
 * Create a PropertyBackend by name, optionally hydrating from a state buffer.
 * Throws RetrievalError for unknown backend names.
 *
 * Rust: `pub fn backend_from_string(name: &str, buffer: Option<&Vec<u8>>) -> Result<Arc<dyn PropertyBackend>, RetrievalError>`
 *
 * NOTE: Implementation deferred until YjsBackend and LWWBackend are ported.
 * For now, this is a placeholder that will throw.
 */
export function backendFromString(
  name: string,
  buffer?: Uint8Array,
): PropertyBackend {
  if (name === 'lww') {
    return buffer ? LWWBackend.fromStateBuffer(buffer) : new LWWBackend();
  }
  if (name === 'yjs') {
    return buffer ? YjsBackend.fromStateBuffer(buffer) : new YjsBackend();
  }
  throw new Error(`Unknown backend: "${name}"`);
}
