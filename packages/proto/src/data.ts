// MIRRORS: ankurah/proto/src/data.rs

import { BincodeReader, BincodeWriter, compareUtf8Bytes } from './codec';
import { Attested, AttestationSet } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import { EntityId, EventId } from './id';

// ─── Operation ──────────────────────────────────────────────────────────────

/**
 * Operation: a single diff blob.
 * Derived serde — struct { diff: Vec<u8> }.
 */
export class Operation {
  readonly diff: Uint8Array;

  constructor(diff: Uint8Array) {
    this.diff = diff;
  }

  equals(other: Operation): boolean {
    if (this.diff.length !== other.diff.length) return false;
    for (let i = 0; i < this.diff.length; i++) {
      if (this.diff[i] !== other.diff[i]) return false;
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this.diff);
  }

  static decode(reader: BincodeReader): Operation {
    return new Operation(reader.readByteVec());
  }
}

// ─── OperationSet ───────────────────────────────────────────────────────────

/**
 * OperationSet: BTreeMap<String, Vec<Operation>>.
 * Derived serde — serialized as BTreeMap (sorted by key in UTF-8 byte order).
 */
export class OperationSet {
  readonly map: Map<string, Operation[]>;

  constructor(map: Map<string, Operation[]> = new Map()) {
    this.map = map;
  }

  get(key: string): Operation[] | undefined {
    return this.map.get(key);
  }

  [Symbol.iterator](): Iterator<[string, Operation[]]> {
    return this.map[Symbol.iterator]();
  }

  entries(): IterableIterator<[string, Operation[]]> {
    return this.map.entries();
  }

  toString(): string {
    const parts: string[] = [];
    for (const [backend, ops] of this.map) {
      const totalBytes = ops.reduce((sum, op) => sum + op.diff.length, 0);
      parts.push(`${backend} => ${totalBytes}b`);
    }
    return `OperationSet(${parts.join(' ')})`;
  }

  equals(other: OperationSet): boolean {
    if (this.map.size !== other.map.size) return false;
    for (const [key, ops] of this.map) {
      const otherOps = other.map.get(key);
      if (!otherOps || ops.length !== otherOps.length) return false;
      for (let i = 0; i < ops.length; i++) {
        if (!ops[i].equals(otherOps[i])) return false;
      }
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    // BTreeMap<String, Vec<Operation>> — sorted by key
    const entries = [...this.map.entries()].sort((a, b) => compareUtf8Bytes(a[0], b[0]));
    writer.writeLength(entries.length);
    for (const [key, ops] of entries) {
      writer.writeString(key);
      writer.writeVec(ops, (w, op) => op.encode(w));
    }
  }

  static decode(reader: BincodeReader): OperationSet {
    const len = reader.readLength();
    const map = new Map<string, Operation[]>();
    for (let i = 0; i < len; i++) {
      const key = reader.readString();
      const ops = reader.readVec(r => Operation.decode(r));
      map.set(key, ops);
    }
    return new OperationSet(map);
  }
}

// ─── StateBuffers ───────────────────────────────────────────────────────────

/**
 * StateBuffers: BTreeMap<String, Vec<u8>>.
 * Derived serde — serialized as BTreeMap (sorted by key).
 */
export class StateBuffers {
  readonly map: Map<string, Uint8Array>;

  constructor(map: Map<string, Uint8Array> = new Map()) {
    this.map = map;
  }

  static default(): StateBuffers {
    return new StateBuffers();
  }

  get(key: string): Uint8Array | undefined {
    return this.map.get(key);
  }

  [Symbol.iterator](): Iterator<[string, Uint8Array]> {
    return this.map[Symbol.iterator]();
  }

  entries(): IterableIterator<[string, Uint8Array]> {
    return this.map.entries();
  }

  equals(other: StateBuffers): boolean {
    if (this.map.size !== other.map.size) return false;
    for (const [key, buf] of this.map) {
      const otherBuf = other.map.get(key);
      if (!otherBuf || buf.length !== otherBuf.length) return false;
      for (let i = 0; i < buf.length; i++) {
        if (buf[i] !== otherBuf[i]) return false;
      }
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    const entries = [...this.map.entries()].sort((a, b) => compareUtf8Bytes(a[0], b[0]));
    writer.writeLength(entries.length);
    for (const [key, buf] of entries) {
      writer.writeString(key);
      writer.writeByteVec(buf);
    }
  }

  static decode(reader: BincodeReader): StateBuffers {
    const len = reader.readLength();
    const map = new Map<string, Uint8Array>();
    for (let i = 0; i < len; i++) {
      const key = reader.readString();
      const buf = reader.readByteVec();
      map.set(key, buf);
    }
    return new StateBuffers(map);
  }
}

// ─── State ──────────────────────────────────────────────────────────────────

/**
 * State: accumulated entity state.
 * Derived serde — struct { state_buffers: StateBuffers, head: Clock }.
 */
export class State {
  readonly stateBuffers: StateBuffers;
  readonly head: Clock;

  constructor(stateBuffers: StateBuffers = StateBuffers.default(), head: Clock = Clock.default()) {
    this.stateBuffers = stateBuffers;
    this.head = head;
  }

  static default(): State {
    return new State();
  }

  equals(other: State): boolean {
    return this.stateBuffers.equals(other.stateBuffers) && this.head.equals(other.head);
  }

  toString(): string {
    const bufParts: string[] = [];
    for (const [backend, buf] of this.stateBuffers) {
      bufParts.push(`${backend} => ${buf.length}b`);
    }
    return `State(${this.head.toBase64Short()} buffers ${bufParts.join(' ')})`;
  }

  encode(writer: BincodeWriter): void {
    this.stateBuffers.encode(writer);
    this.head.encode(writer);
  }

  static decode(reader: BincodeReader): State {
    const stateBuffers = StateBuffers.decode(reader);
    const head = Clock.decode(reader);
    return new State(stateBuffers, head);
  }
}

// ─── EntityState ────────────────────────────────────────────────────────────

/**
 * EntityState: full entity state including identity.
 * Derived serde — struct { entity_id: EntityId, collection: CollectionId, state: State }.
 */
export class EntityState {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly state: State;

  constructor(entityId: EntityId, collection: CollectionId, state: State) {
    this.entityId = entityId;
    this.collection = collection;
    this.state = state;
  }

  equals(other: EntityState): boolean {
    return this.entityId.equals(other.entityId) &&
      this.collection.equals(other.collection) &&
      this.state.equals(other.state);
  }

  toString(): string {
    return `EntityState(${this.entityId.toBase64Short()} ${this.state})`;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    this.state.encode(writer);
  }

  static decode(reader: BincodeReader): EntityState {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const state = State.decode(reader);
    return new EntityState(entityId, collection, state);
  }
}

// ─── Event ──────────────────────────────────────────────────────────────────

/**
 * Event: a single change event.
 * Derived serde — struct { collection, entity_id, operations, parent }.
 */
export class Event {
  readonly collection: CollectionId;
  readonly entityId: EntityId;
  readonly operations: OperationSet;
  readonly parent: Clock;

  constructor(collection: CollectionId, entityId: EntityId, operations: OperationSet, parent: Clock) {
    this.collection = collection;
    this.entityId = entityId;
    this.operations = operations;
    this.parent = parent;
  }

  /**
   * Compute the EventId for this event (SHA-256 of bincode-serialized entity_id || operations || parent).
   *
   * Rust: `pub fn id(&self) -> EventId { EventId::from_parts(&self.entity_id, &self.operations, &self.parent) }`
   */
  id(): EventId {
    return EventId.fromParts(this.entityId, this.operations, this.parent);
  }

  /** Whether this event represents entity creation (empty parent clock). */
  isEntityCreate(): boolean {
    return this.parent.isEmpty();
  }

  toString(): string {
    const parts: string[] = [];
    for (const [backend, ops] of this.operations) {
      const totalBytes = ops.reduce((sum, op) => sum + op.diff.length, 0);
      parts.push(`${backend} => ${totalBytes}b`);
    }
    const create = this.isEntityCreate() ? '(create) ' : '';
    return `Event(${this.collection}/${this.entityId.toBase64Short()} ${create}${this.parent.toBase64Short()} ${parts.join(' ')})`;
  }

  encode(writer: BincodeWriter): void {
    this.collection.encode(writer);
    this.entityId.encode(writer);
    this.operations.encode(writer);
    this.parent.encode(writer);
  }

  static decode(reader: BincodeReader): Event {
    const collection = CollectionId.decode(reader);
    const entityId = EntityId.decode(reader);
    const operations = OperationSet.decode(reader);
    const parent = Clock.decode(reader);
    return new Event(collection, entityId, operations, parent);
  }
}

// ─── EventFragment ──────────────────────────────────────────────────────────

/**
 * EventFragment: event data without entity identity (for wire efficiency).
 * Derived serde — struct { operations, parent, attestations }.
 */
export class EventFragment {
  readonly operations: OperationSet;
  readonly parent: Clock;
  readonly attestations: AttestationSet;

  constructor(operations: OperationSet, parent: Clock, attestations: AttestationSet) {
    this.operations = operations;
    this.parent = parent;
    this.attestations = attestations;
  }

  static fromAttestedEvent(attested: Attested<Event>): EventFragment {
    return new EventFragment(attested.payload.operations, attested.payload.parent, attested.attestations);
  }

  static toAttestedEvent(entityId: EntityId, collection: CollectionId, frag: EventFragment): Attested<Event> {
    const event = new Event(collection, entityId, frag.operations, frag.parent);
    return new Attested(event, frag.attestations);
  }

  equals(other: EventFragment): boolean {
    return this.operations.equals(other.operations) &&
      this.parent.equals(other.parent) &&
      this.attestations.equals(other.attestations);
  }

  toString(): string {
    return `EventFragment(parent ${this.parent} operations ${this.operations})`;
  }

  encode(writer: BincodeWriter): void {
    this.operations.encode(writer);
    this.parent.encode(writer);
    this.attestations.encode(writer);
  }

  static decode(reader: BincodeReader): EventFragment {
    const operations = OperationSet.decode(reader);
    const parent = Clock.decode(reader);
    const attestations = AttestationSet.decode(reader);
    return new EventFragment(operations, parent, attestations);
  }
}

// ─── StateFragment ──────────────────────────────────────────────────────────

/**
 * StateFragment: state data without entity identity (for wire efficiency).
 * Derived serde — struct { state, attestations }.
 */
export class StateFragment {
  readonly state: State;
  readonly attestations: AttestationSet;

  constructor(state: State, attestations: AttestationSet) {
    this.state = state;
    this.attestations = attestations;
  }

  static fromAttestedEntityState(attested: Attested<EntityState>): StateFragment {
    return new StateFragment(attested.payload.state, attested.attestations);
  }

  static toAttestedEntityState(entityId: EntityId, collection: CollectionId, frag: StateFragment): Attested<EntityState> {
    const entityState = new EntityState(entityId, collection, frag.state);
    return new Attested(entityState, frag.attestations);
  }

  equals(other: StateFragment): boolean {
    return this.state.equals(other.state) && this.attestations.equals(other.attestations);
  }

  toString(): string {
    return `StateFragment(state ${this.state} attestations: ${this.attestations.length})`;
  }

  encode(writer: BincodeWriter): void {
    this.state.encode(writer);
    this.attestations.encode(writer);
  }

  static decode(reader: BincodeReader): StateFragment {
    const state = State.decode(reader);
    const attestations = AttestationSet.decode(reader);
    return new StateFragment(state, attestations);
  }
}
