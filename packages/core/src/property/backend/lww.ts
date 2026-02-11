// MIRRORS: ankurah/core/src/property/backend/lww.rs

import {
  BincodeWriter,
  BincodeReader,
  Operation,
  EntityId,
} from '@ankurah/proto';
import {
  Broadcast,
  BroadcastId,
  type Listener,
  ListenerGuard,
} from '@ankurah/signals';

import type { PropertyBackend } from './index.ts';
import type { PropertyName } from '../index.ts';
import type { Value } from '../../value/index.ts';
import { MutationError, RetrievalError, StateError } from '../../error.ts';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LWW_DIFF_VERSION: number = 1;

// ---------------------------------------------------------------------------
// ValueEntry — internal per-property state
// ---------------------------------------------------------------------------

interface ValueEntry {
  value: Value | null;
  committed: boolean;
}

/** Deep-clone a ValueEntry (clone the value within). */
function cloneValueEntry(entry: ValueEntry): ValueEntry {
  return {
    value: entry.value !== null ? cloneValue(entry.value) : null,
    committed: entry.committed,
  };
}

// ---------------------------------------------------------------------------
// Value bincode serialization helpers
// ---------------------------------------------------------------------------

// Variant indices match Rust serde derive order:
// 0=I16, 1=I32, 2=I64, 3=F64, 4=Bool, 5=String, 6=EntityId, 7=Object, 8=Binary, 9=Json

const enc = new TextEncoder();
const dec = new TextDecoder();

function writeValue(writer: BincodeWriter, value: Value): void {
  switch (value.type) {
    case 'I16':
      writer.writeVariant(0);
      writer.writeI16(value.value);
      break;
    case 'I32':
      writer.writeVariant(1);
      writer.writeI32(value.value);
      break;
    case 'I64':
      writer.writeVariant(2);
      // Rust i64 — serialized as i64 in bincode
      writer.writeI64(BigInt(value.value));
      break;
    case 'F64':
      writer.writeVariant(3);
      writer.writeF64(value.value);
      break;
    case 'Bool':
      writer.writeVariant(4);
      writer.writeBool(value.value);
      break;
    case 'String':
      writer.writeVariant(5);
      writer.writeString(value.value);
      break;
    case 'EntityId':
      writer.writeVariant(6);
      // EntityId custom serde: raw 16 bytes
      value.value.encode(writer);
      break;
    case 'Object':
      writer.writeVariant(7);
      writer.writeByteVec(value.value);
      break;
    case 'Binary':
      writer.writeVariant(8);
      writer.writeByteVec(value.value);
      break;
    case 'Json':
      // Json uses json_as_bytes: JSON string -> serde_json::to_vec() -> bincode Vec<u8>
      writer.writeVariant(9);
      const jsonBytes = enc.encode(JSON.stringify(value.value));
      writer.writeByteVec(jsonBytes);
      break;
  }
}

function readValue(reader: BincodeReader): Value {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: // I16
      return { type: 'I16', value: reader.readI16() };
    case 1: // I32
      return { type: 'I32', value: reader.readI32() };
    case 2: // I64
      return { type: 'I64', value: Number(reader.readI64()) };
    case 3: // F64
      return { type: 'F64', value: reader.readF64() };
    case 4: // Bool
      return { type: 'Bool', value: reader.readBool() };
    case 5: // String
      return { type: 'String', value: reader.readString() };
    case 6: // EntityId — custom serde: raw 16 bytes
      return { type: 'EntityId', value: EntityId.decode(reader) };
    case 7: // Object
      return { type: 'Object', value: reader.readByteVec() };
    case 8: // Binary
      return { type: 'Binary', value: reader.readByteVec() };
    case 9: { // Json — json_as_bytes: bincode Vec<u8> -> serde_json::from_slice
      const jsonBytes = reader.readByteVec();
      const jsonStr = dec.decode(jsonBytes);
      return { type: 'Json', value: JSON.parse(jsonStr) };
    }
    default:
      throw new Error(`Unknown Value variant index: ${variant}`);
  }
}

/** Write Option<Value>: 0x00=None, 0x01+Value */
function writeOptionValue(writer: BincodeWriter, value: Value | null): void {
  if (value === null) {
    writer.writeU8(0);
  } else {
    writer.writeU8(1);
    writeValue(writer, value);
  }
}

/** Read Option<Value> */
function readOptionValue(reader: BincodeReader): Value | null {
  const tag = reader.readU8();
  if (tag === 0) return null;
  if (tag === 1) return readValue(reader);
  throw new Error(`Invalid Option tag: 0x${tag.toString(16)}`);
}

/** Serialize BTreeMap<PropertyName, Option<Value>> to bincode bytes */
function serializePropertyMap(map: Map<PropertyName, Value | null>): Uint8Array {
  const writer = new BincodeWriter();
  writer.writeStringMap(map, (w, v) => writeOptionValue(w, v));
  return writer.finish();
}

/** Deserialize BTreeMap<PropertyName, Option<Value>> from bincode bytes */
function deserializePropertyMap(data: Uint8Array): Map<PropertyName, Value | null> {
  const reader = new BincodeReader(data);
  return reader.readStringMap((r) => readOptionValue(r));
}

/** Deep-clone a Value */
function cloneValue(v: Value): Value {
  switch (v.type) {
    case 'I16':
    case 'I32':
    case 'I64':
    case 'F64':
    case 'Bool':
    case 'String':
      return { ...v };
    case 'EntityId':
      return { type: 'EntityId', value: EntityId.fromBytes(v.value.toBytes()) };
    case 'Object':
      return { type: 'Object', value: new Uint8Array(v.value) };
    case 'Binary':
      return { type: 'Binary', value: new Uint8Array(v.value) };
    case 'Json':
      // Structured clone via JSON round-trip
      return { type: 'Json', value: JSON.parse(JSON.stringify(v.value)) };
  }
}

// ---------------------------------------------------------------------------
// LWWBackend
// ---------------------------------------------------------------------------

/**
 * Last-Writer-Wins property backend.
 *
 * Each property stores a single value. Conflicts are resolved by last-write-wins
 * semantics (the most recent write replaces the previous value).
 *
 * Rust: `pub struct LWWBackend`
 * Divergence: Rust uses RwLock<BTreeMap> and Mutex<BTreeMap>; TS uses plain Map [E8].
 */
export class LWWBackend implements PropertyBackend {
  // Divergence: Rust uses RwLock<BTreeMap<PropertyName, ValueEntry>>; TS uses plain Map [E8].
  private values: Map<PropertyName, ValueEntry>;
  // Divergence: Rust uses Mutex<BTreeMap<PropertyName, Broadcast>>; TS uses plain Map [E8].
  private fieldBroadcasts: Map<PropertyName, Broadcast<void>>;

  constructor() {
    this.values = new Map();
    this.fieldBroadcasts = new Map();
  }

  // ── LWWBackend-specific methods ──

  /** Set a property value (marks as uncommitted). */
  set(propertyName: PropertyName, value: Value | null): void {
    this.values.set(propertyName, { value, committed: false });
  }

  /** Get a property value, returning null if missing or explicitly null. */
  get(propertyName: PropertyName): Value | null {
    const entry = this.values.get(propertyName);
    return entry?.value ?? null;
  }

  /** Get the broadcast ID for a specific field, creating the broadcast if necessary. */
  fieldBroadcastId(fieldName: PropertyName): BroadcastId {
    let broadcast = this.fieldBroadcasts.get(fieldName);
    if (!broadcast) {
      broadcast = new Broadcast<void>();
      this.fieldBroadcasts.set(fieldName, broadcast);
    }
    return broadcast.id();
  }

  // ── PropertyBackend interface ──

  // Static methods (PropertyBackendStatic interface)
  static propertyBackendName(): string {
    return 'lww';
  }

  static fromStateBuffer(stateBuffer: Uint8Array): LWWBackend {
    try {
      const rawMap = deserializePropertyMap(stateBuffer);
      const backend = new LWWBackend();
      for (const [key, value] of rawMap) {
        backend.values.set(key, { value, committed: true });
      }
      return backend;
    } catch (e) {
      throw RetrievalError.deserializationError(
        e instanceof Error ? e : new Error(String(e)),
      );
    }
  }

  fork(): PropertyBackend {
    const forked = new LWWBackend();
    for (const [key, entry] of this.values) {
      forked.values.set(key, cloneValueEntry(entry));
    }
    // Create fresh broadcasts (don't clone the existing ones for transaction isolation)
    return forked;
  }

  properties(): PropertyName[] {
    const keys = Array.from(this.values.keys());
    keys.sort();
    return keys;
  }

  propertyValue(propertyName: PropertyName): Value | null {
    return this.get(propertyName);
  }

  propertyValues(): Map<PropertyName, Value | null> {
    const result = new Map<PropertyName, Value | null>();
    for (const [key, entry] of this.values) {
      result.set(key, entry.value);
    }
    return result;
  }

  toStateBuffer(): Uint8Array {
    try {
      const propertyValues = this.propertyValues();
      return serializePropertyMap(propertyValues);
    } catch (e) {
      throw StateError.serializationError(
        e instanceof Error ? e : new Error(String(e)),
      );
    }
  }

  toOperations(): Operation[] | null {
    const changedValues = new Map<PropertyName, Value | null>();

    for (const [name, entry] of this.values) {
      if (!entry.committed) {
        changedValues.set(name, entry.value);
        entry.committed = true;
      }
    }

    if (changedValues.size === 0) {
      return null;
    }

    try {
      // Double-wrapped bincode: LWWDiff { version: u8, data: Vec<u8> }
      // where data = bincode_serialize(BTreeMap<PropertyName, Option<Value>>)
      const data = serializePropertyMap(changedValues);
      const outerWriter = new BincodeWriter();
      outerWriter.writeU8(LWW_DIFF_VERSION);
      outerWriter.writeByteVec(data);
      const diff = outerWriter.finish();

      return [new Operation(diff)];
    } catch (e) {
      throw MutationError.general(
        e instanceof Error ? e : new Error(String(e)),
      );
    }
  }

  applyOperations(operations: Operation[]): void {
    const changedFields: PropertyName[] = [];

    try {
      for (const operation of operations) {
        const outerReader = new BincodeReader(operation.diff);
        const version = outerReader.readU8();

        switch (version) {
          case 1: {
            const data = outerReader.readByteVec();
            const changes = deserializePropertyMap(data);

            for (const [propertyName, newValue] of changes) {
              // Insert as committed entry since this came from an operation
              this.values.set(propertyName, { value: newValue, committed: true });
              changedFields.push(propertyName);
            }
            break;
          }
          default:
            throw MutationError.updateFailed(
              new Error(`Unknown LWW operation version: ${version}`),
            );
        }
      }
    } catch (e) {
      if (e instanceof MutationError) throw e;
      throw MutationError.general(
        e instanceof Error ? e : new Error(String(e)),
      );
    }

    // Notify field subscribers for changed fields only
    for (const fieldName of changedFields) {
      const broadcast = this.fieldBroadcasts.get(fieldName);
      if (broadcast) {
        broadcast.send();
      }
    }
  }

  listenField(fieldName: PropertyName, listener: Listener): ListenerGuard {
    // Get or create the broadcast for this field
    let broadcast = this.fieldBroadcasts.get(fieldName);
    if (!broadcast) {
      broadcast = new Broadcast<void>();
      this.fieldBroadcasts.set(fieldName, broadcast);
    }

    // Subscribe to the broadcast and return the guard
    const broadcastGuard = broadcast.reference().listen({
      type: 'NotifyOnly',
      callback: listener,
    });
    return new ListenerGuard(broadcastGuard);
  }
}
