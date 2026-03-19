// MIRRORS: ankurah/proto/src/data.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter, compareUtf8Bytes } from './codec';
import { Attested, AttestationSet } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import { EntityId, EventId } from './id';

// Divergence: EventId is defined in data.rs in Rust but co-located in id.ts in TS
// to share base64/ULID utilities and avoid circular dependencies [E4]

export class Event extends Struct {
  readonly collection: CollectionId;
  readonly entityId: EntityId;
  readonly operations: OperationSet;
  /// The set of concurrent events (usually only one) which is the precursor of this event
  readonly parent: Clock;

  constructor(collection: CollectionId, entityId: EntityId, operations: OperationSet, parent: Clock) {
    super();
    this.collection = collection;
    this.entityId = entityId;
    this.operations = operations;
    this.parent = parent;
  }

  // impl Event
  isEntityCreate(): boolean {
    return this.parent.isEmpty();
  }

  id(): EventId {
    return EventId.fromParts(this.entityId, this.operations, this.parent);
  }

  // impl Display for Event
  toString(): string {
    const parts: string[] = [];
    for (const [backend, ops] of this.operations) {
      const totalBytes = ops.reduce((sum, op) => sum + op.diff.length, 0);
      parts.push(`${backend} => ${totalBytes}b`);
    }
    const create = this.isEntityCreate() ? '(create) ' : '';
    return `Event(${this.id().toBase64Short()} ${this.collection}/${this.entityId.toBase64Short()} ${create}${this.parent.toBase64Short()} ${parts.join(' ')})`;
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

export class EventFragment extends Struct {
  readonly operations: OperationSet;
  readonly parent: Clock;
  readonly attestations: AttestationSet;

  constructor(operations: OperationSet, parent: Clock, attestations: AttestationSet) {
    super();
    this.operations = operations;
    this.parent = parent;
    this.attestations = attestations;
  }

  // impl From<Attested<Event>> for EventFragment
  static fromAttestedEvent(attested: Attested<Event>): EventFragment {
    return new EventFragment(attested.payload.operations, attested.payload.parent, attested.attestations);
  }

  // impl From<(EntityId, CollectionId, EventFragment)> for Attested<Event>
  static toAttestedEvent(entityId: EntityId, collection: CollectionId, frag: EventFragment): Attested<Event> {
    const event = new Event(collection, entityId, frag.operations, frag.parent);
    return new Attested(event, frag.attestations);
  }

  equals(other: EventFragment): boolean {
    return this.operations.equals(other.operations) &&
      this.parent.equals(other.parent) &&
      this.attestations.equals(other.attestations);
  }

  // impl Display for EventFragment
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

export class StateFragment extends Struct {
  readonly state: State;
  readonly attestations: AttestationSet;

  constructor(state: State, attestations: AttestationSet) {
    super();
    this.state = state;
    this.attestations = attestations;
  }

  // impl From<Attested<EntityState>> for StateFragment
  static fromAttestedEntityState(attested: Attested<EntityState>): StateFragment {
    return new StateFragment(attested.payload.state, attested.attestations);
  }

  // impl From<(EntityId, CollectionId, StateFragment)> for Attested<EntityState>
  static toAttestedEntityState(entityId: EntityId, collection: CollectionId, frag: StateFragment): Attested<EntityState> {
    const entityState = new EntityState(entityId, collection, frag.state);
    return new Attested(entityState, frag.attestations);
  }

  equals(other: StateFragment): boolean {
    return this.state.equals(other.state) && this.attestations.equals(other.attestations);
  }

  // impl Display for StateFragment
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

export class OperationSet extends Struct {
  readonly _0: Map<string, Operation[]>;

  constructor(_0: Map<string, Operation[]> = new Map()) {
    super();
    this._0 = _0;
  }

  // impl Display for OperationSet
  toString(): string {
    const parts: string[] = [];
    for (const [backend, ops] of this._0) {
      const totalBytes = ops.reduce((sum, op) => sum + op.diff.length, 0);
      parts.push(`${backend} => ${totalBytes}b`);
    }
    return `OperationSet(${parts.join(' ')})`;
  }

  // impl Deref for OperationSet — target: BTreeMap<String, Vec<Operation>>
  get(key: string): Operation[] | undefined {
    return this._0.get(key);
  }

  [Symbol.iterator](): Iterator<[string, Operation[]]> {
    return this._0[Symbol.iterator]();
  }

  entries(): IterableIterator<[string, Operation[]]> {
    return this._0.entries();
  }

  equals(other: OperationSet): boolean {
    if (this._0.size !== other._0.size) return false;
    for (const [key, ops] of this._0) {
      const otherOps = other._0.get(key);
      if (!otherOps || ops.length !== otherOps.length) return false;
      for (let i = 0; i < ops.length; i++) {
        if (!ops[i].equals(otherOps[i])) return false;
      }
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    // BTreeMap<String, Vec<Operation>> — sorted by key in UTF-8 byte order
    const entries = [...this._0.entries()].sort((a, b) => compareUtf8Bytes(a[0], b[0]));
    writer.writeLength(entries.length);
    for (const [key, ops] of entries) {
      writer.writeString(key);
      writer.writeVec(ops, (w, op) => op.encode(w));
    }
  }

  static decode(reader: BincodeReader): OperationSet {
    const len = reader.readLength();
    const _0 = new Map<string, Operation[]>();
    for (let i = 0; i < len; i++) {
      const key = reader.readString();
      const ops = reader.readVec(r => Operation.decode(r));
      _0.set(key, ops);
    }
    return new OperationSet(_0);
  }
}

export class Operation extends Struct {
  readonly diff: Uint8Array;

  constructor(diff: Uint8Array) {
    super();
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

export class EntityState extends Struct {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly state: State;

  constructor(entityId: EntityId, collection: CollectionId, state: State) {
    super();
    this.entityId = entityId;
    this.collection = collection;
    this.state = state;
  }

  equals(other: EntityState): boolean {
    return this.entityId.equals(other.entityId) &&
      this.collection.equals(other.collection) &&
      this.state.equals(other.state);
  }

  // impl Display for EntityState
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

export class State extends Struct {
  /// The current accumulated state of the entity inclusive of all events up to this point
  readonly stateBuffers: StateBuffers;
  /// The set of concurrent events (usually only one) which have been applied to the entity state above
  readonly head: Clock;

  constructor(stateBuffers: StateBuffers = StateBuffers.default(), head: Clock = Clock.default()) {
    super();
    this.stateBuffers = stateBuffers;
    this.head = head;
  }

  static default(): State {
    return new State();
  }

  equals(other: State): boolean {
    return this.stateBuffers.equals(other.stateBuffers) && this.head.equals(other.head);
  }

  // impl Display for State
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

export class StateBuffers extends Struct {
  readonly _0: Map<string, Uint8Array>;

  constructor(_0: Map<string, Uint8Array> = new Map()) {
    super();
    this._0 = _0;
  }

  static default(): StateBuffers {
    return new StateBuffers();
  }

  // impl Deref for StateBuffers — target: BTreeMap<String, Vec<u8>>
  get(key: string): Uint8Array | undefined {
    return this._0.get(key);
  }

  [Symbol.iterator](): Iterator<[string, Uint8Array]> {
    return this._0[Symbol.iterator]();
  }

  entries(): IterableIterator<[string, Uint8Array]> {
    return this._0.entries();
  }

  equals(other: StateBuffers): boolean {
    if (this._0.size !== other._0.size) return false;
    for (const [key, buf] of this._0) {
      const otherBuf = other._0.get(key);
      if (!otherBuf || buf.length !== otherBuf.length) return false;
      for (let i = 0; i < buf.length; i++) {
        if (buf[i] !== otherBuf[i]) return false;
      }
    }
    return true;
  }

  encode(writer: BincodeWriter): void {
    const entries = [...this._0.entries()].sort((a, b) => compareUtf8Bytes(a[0], b[0]));
    writer.writeLength(entries.length);
    for (const [key, buf] of entries) {
      writer.writeString(key);
      writer.writeByteVec(buf);
    }
  }

  static decode(reader: BincodeReader): StateBuffers {
    const len = reader.readLength();
    const _0 = new Map<string, Uint8Array>();
    for (let i = 0; i < len; i++) {
      const key = reader.readString();
      const buf = reader.readByteVec();
      _0.set(key, buf);
    }
    return new StateBuffers(_0);
  }
}

// ─── Attested<Event> helpers ─────────────────────────────────────────────────
// These are impl blocks on Attested<Event> and Attested<EntityState> in Rust.
// Since Attested is generic and defined in auth.ts, we add free functions here.

export function attestedEventCollection(attested: Attested<Event>): CollectionId {
  return attested.payload.collection;
}

// impl From<Event> for Attested<Event>
export function attestedEventFromEvent(event: Event): Attested<Event> {
  return new Attested(event, AttestationSet.default());
}

// impl From<EntityState> for Attested<EntityState>
export function attestedEntityStateFromEntityState(entityState: EntityState): Attested<EntityState> {
  return new Attested(entityState, AttestationSet.default());
}

export function attestedEventFromParts(entityId: EntityId, collection: CollectionId, frag: EventFragment): Attested<Event> {
  return EventFragment.toAttestedEvent(entityId, collection, frag);
}

export function attestedEntityStateFromParts(entityId: EntityId, collection: CollectionId, fragment: StateFragment): Attested<EntityState> {
  return StateFragment.toAttestedEntityState(entityId, collection, fragment);
}

export function attestedEntityStateToParts(attested: Attested<EntityState>): [EntityId, CollectionId, StateFragment] {
  return [attested.payload.entityId, attested.payload.collection, new StateFragment(attested.payload.state, attested.attestations)];
}
