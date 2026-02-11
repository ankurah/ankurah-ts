// MIRRORS: ankurah/core/src/property/backend/yrs.rs
// Exception E5: yrs.rs -> yjs.ts due to library rename (Yrs -> Yjs)

import * as Y from 'yjs';

import { Operation } from '@ankurah/proto';
import {
  Broadcast,
  type BroadcastId,
  type Listener,
  ListenerGuard,
} from '@ankurah/signals';

import type { PropertyBackend } from './index.ts';
import type { PropertyName } from '../index.ts';
import type { Value } from '../../value/index.ts';
import { MutationError, StateError } from '../../error.ts';

// ---------------------------------------------------------------------------
// YjsBackend
// ---------------------------------------------------------------------------

/**
 * Stores one or more properties of an entity using a Yjs CRDT document.
 *
 * Rust: `pub struct YrsBackend { doc, previous_state, field_broadcasts }`
 * Divergence: Uses Yjs (JS) instead of Yrs (Rust) [E5].
 * Divergence: No Mutex wrappers needed — single-threaded JS [E8].
 *
 * CRITICAL: All encoding/decoding uses V2 functions for wire compatibility with Yrs.
 */
export class YjsBackend implements PropertyBackend {
  readonly doc: Y.Doc;
  private previousState: Uint8Array;
  private fieldBroadcasts: Map<PropertyName, Broadcast> = new Map();

  constructor(doc?: Y.Doc) {
    this.doc = doc ?? new Y.Doc();
    this.previousState = Y.encodeStateVector(this.doc);
  }

  // ── Static factory methods ──────────────────────────────────────────

  /**
   * Construct a YjsBackend from a V2 state buffer.
   *
   * Rust: `fn from_state_buffer(state_buffer: &Vec<u8>) -> Result<Self, RetrievalError>`
   * TS: Throws on failure [A8].
   */
  static fromStateBuffer(buffer: Uint8Array): YjsBackend {
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, buffer);
    const backend = new YjsBackend(doc);
    // Reset previousState to current state after hydration
    backend.previousState = Y.encodeStateVector(doc);
    return backend;
  }

  /**
   * Unique property backend identifier.
   *
   * Rust: `fn property_backend_name() -> String { "yrs" }`
   * Note: Returns "yjs" in TS since we use Yjs, but wire format is compatible.
   */
  static propertyBackendName(): string {
    return 'yjs';
  }

  // ── Text field accessors ────────────────────────────────────────────

  /**
   * Get the string value of a named text field.
   * Returns null if the field does not exist in the document.
   *
   * Rust: `fn get_string(&self, property_name: impl AsRef<str>) -> Option<String>`
   */
  getString(propertyName: PropertyName): string | null {
    // Check if the field actually exists in the document's shared types
    const shared = this.doc.share.get(propertyName);
    if (!shared) {
      return null;
    }
    const text = this.doc.getText(propertyName);
    const str = text.toString();
    // Match Rust behavior: return None for empty strings (no content inserted)
    return str.length === 0 ? null : str;
  }

  /**
   * Insert text at a given index in a named text field.
   *
   * Rust: `fn insert(&self, property_name: impl AsRef<str>, index: u32, value: &str) -> Result<(), MutationError>`
   */
  insert(propertyName: PropertyName, index: number, value: string): void {
    const text = this.doc.getText(propertyName);
    text.insert(index, value);
  }

  /**
   * Delete a range of characters from a named text field.
   *
   * Rust: `fn delete(&self, property_name: impl AsRef<str>, index: u32, length: u32) -> Result<(), MutationError>`
   */
  delete(propertyName: PropertyName, index: number, length: number): void {
    const text = this.doc.getText(propertyName);
    text.delete(index, length);
  }

  // ── PropertyBackend interface ───────────────────────────────────────

  /**
   * Get the list of property names known to this backend.
   *
   * Rust: `fn properties(&self) -> Vec<String>`
   * Divergence: Rust uses root_refs from transaction; Yjs uses doc.share map.
   */
  properties(): PropertyName[] {
    return Array.from(this.doc.share.keys());
  }

  /**
   * Get the value of a specific property as a Value.
   *
   * Rust: `fn property_value(&self, property_name: &PropertyName) -> Option<Value>`
   */
  propertyValue(propertyName: PropertyName): Value | null {
    const str = this.getString(propertyName);
    if (str === null) return null;
    return { type: 'String', value: str };
  }

  /**
   * Get all property values as a map.
   *
   * Rust: `fn property_values(&self) -> BTreeMap<PropertyName, Option<Value>>`
   */
  propertyValues(): Map<PropertyName, Value | null> {
    const result = new Map<PropertyName, Value | null>();
    for (const name of this.properties()) {
      result.set(name, this.propertyValue(name));
    }
    return result;
  }

  /**
   * Serialize the full document state as a V2 update buffer.
   *
   * Rust: `fn to_state_buffer(&self) -> Result<Vec<u8>, StateError>`
   * CRITICAL: Uses encodeStateAsUpdateV2 with empty StateVector to get full state.
   */
  toStateBuffer(): Uint8Array {
    return Y.encodeStateAsUpdateV2(this.doc);
  }

  /**
   * Compute operations (diff) since the last call to this method.
   * Returns null if no changes have been made.
   *
   * Rust: `fn to_operations(&self) -> Result<Option<Vec<Operation>>, MutationError>`
   * CRITICAL: Uses encodeStateAsUpdateV2 with previousState for incremental diff.
   */
  toOperations(): Operation[] | null {
    const currentState = Y.encodeStateVector(this.doc);

    // Quick check: if state vector hasn't changed, there's definitely no diff
    if (this.stateVectorsEqual(this.previousState, currentState)) {
      return null;
    }

    const diff = Y.encodeStateAsUpdateV2(this.doc, this.previousState);
    this.previousState = currentState;

    return [new Operation(diff)];
  }

  /**
   * Apply operations from a remote peer.
   * Notifies field listeners for any fields that changed.
   *
   * Rust: `fn apply_operations(&self, operations: &Vec<Operation>) -> Result<(), MutationError>`
   */
  applyOperations(operations: Operation[]): void {
    const changedFields = new Set<string>();

    for (const operation of operations) {
      this.applyUpdate(operation.diff, changedFields);
    }

    // Notify field subscribers for fields that actually changed
    for (const fieldName of changedFields) {
      const broadcast = this.fieldBroadcasts.get(fieldName);
      if (broadcast) {
        broadcast.send();
      }
    }
  }

  /**
   * Create a deep copy (fork) of this backend.
   *
   * Rust: `fn fork(&self) -> Arc<dyn PropertyBackend>`
   * Divergence: Returns plain YjsBackend, not Arc [E8].
   */
  fork(): YjsBackend {
    const stateBuffer = this.toStateBuffer();
    return YjsBackend.fromStateBuffer(stateBuffer);
  }

  /**
   * Listen to changes for a specific field.
   * Auto-creates the broadcast if it doesn't exist yet.
   *
   * Rust: `fn listen_field(&self, field_name: &PropertyName, listener: Listener) -> ListenerGuard`
   */
  listenField(fieldName: PropertyName, listener: Listener): ListenerGuard {
    // Get or create the broadcast for this field
    let broadcast = this.fieldBroadcasts.get(fieldName);
    if (!broadcast) {
      broadcast = new Broadcast();
      this.fieldBroadcasts.set(fieldName, broadcast);
    }

    // Subscribe to the broadcast and return the guard
    const guard = broadcast.reference().listen({ type: 'NotifyOnly', callback: listener });
    return new ListenerGuard(guard);
  }

  // ── Additional public methods ───────────────────────────────────────

  /**
   * Get the broadcast ID for a specific field, creating the broadcast if necessary.
   *
   * Rust: `pub fn field_broadcast_id(&self, field_name: &PropertyName) -> BroadcastId`
   */
  fieldBroadcastId(fieldName: PropertyName): BroadcastId {
    let broadcast = this.fieldBroadcasts.get(fieldName);
    if (!broadcast) {
      broadcast = new Broadcast();
      this.fieldBroadcasts.set(fieldName, broadcast);
    }
    return broadcast.id();
  }

  // ── Private helpers ─────────────────────────────────────────────────

  /**
   * Apply a single V2 update to the document, tracking which fields changed.
   *
   * Rust: `fn apply_update(&self, update: &[u8], changed_fields: &Arc<Mutex<HashSet<String>>>) -> Result<(), MutationError>`
   * Divergence: No Mutex needed — single-threaded JS [E8].
   *
   * Uses Yjs observe on known text fields to detect which ones changed.
   */
  private applyUpdate(update: Uint8Array, changedFields: Set<string>): void {
    // Set up observers on all known field broadcasts to detect changes
    // Mirrors Rust: subscribe to each known text field before applying the update
    const unobservers: Array<() => void> = [];

    for (const fieldName of this.fieldBroadcasts.keys()) {
      const text = this.doc.getText(fieldName);
      const observer = (_event: Y.YTextEvent) => {
        changedFields.add(fieldName);
      };
      text.observe(observer);
      unobservers.push(() => text.unobserve(observer));
    }

    try {
      Y.applyUpdateV2(this.doc, update);
    } catch (e) {
      // Clean up observers before rethrowing
      for (const unobserve of unobservers) {
        unobserve();
      }
      throw MutationError.updateFailed(e instanceof Error ? e : new Error(String(e)));
    }

    // Clean up observers
    for (const unobserve of unobservers) {
      unobserve();
    }
  }

  /**
   * Compare two encoded state vectors for equality.
   *
   * Rust: `if diff == Update::EMPTY_V2`
   * Divergence: In TS we compare state vectors before and after to detect emptiness,
   * rather than comparing against a static empty pattern [E5].
   */
  private stateVectorsEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }
}
