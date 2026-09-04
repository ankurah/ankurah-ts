// MIRRORS: ankurah/proto/src/data.rs
import { Struct, Result } from '@ankurah/base';
import { EventId } from './id.provided';
import { BincodeReader, BincodeWriter } from './codec';
import { AttestationSet, Attested } from './auth';
import { Clock } from './clock';
import { CollectionId } from './collection';
import { DecodeError } from './error';
import { EntityId } from './id';
export { EventId };

export class Event extends Struct {
  readonly collection: CollectionId;
  readonly entityId: EntityId;
  readonly operations: OperationSet;
  readonly parent: Clock;

  constructor(collection: CollectionId, entityId: EntityId, operations: OperationSet, parent: Clock) {
    super();
    this.collection = collection;
    this.entityId = entityId;
    this.operations = operations;
    this.parent = parent;
  }

  isEntityCreate(): boolean {
    return this.parent.isEmpty();
  }

  id(): EventId {
    return EventId.fromParts(this.entityId, this.operations, this.parent);
  }

  toString(): string {
    return `Event(${this.id().toBase64Short()} ${this.collection}/${this.entityId.toBase64Short()} ${this.isEntityCreate() ? '(create) ' : ''}${this.parent.toBase64Short()} ${[...this.operations].map(([backend, ops]) => `${backend} => ${[...].map((op) => op.diff.length).reduce((a, b) => a + b, 0)}b`).join(' ')})`;
  }

  clone(): Event {
    return new Event(this.collection.clone(), this.entityId.clone(), this.operations.clone(), this.parent.clone());
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

  static from(attested: Attested<Event>): EventFragment {
    return new EventFragment(attested.payload.operations, attested.payload.parent, attested.attestations);
  }

  toString(): string {
    return `EventFragment(parent ${this.parent} operations ${this.operations})`;
  }

  equals(other: EventFragment): boolean {
    if (!this.operations.equals(other.operations)) return false;
    if (!this.parent.equals(other.parent)) return false;
    if (!this.attestations.equals(other.attestations)) return false;
    return true;
  }

  clone(): EventFragment {
    return new EventFragment(this.operations.clone(), this.parent.clone(), this.attestations.clone());
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

  static from(attested: Attested<EntityState>): StateFragment {
    return new StateFragment(attested.payload.state, attested.attestations);
  }

  toString(): string {
    return `StateFragment(state ${this.state} attestations: ${this.attestations.length})`;
  }

  equals(other: StateFragment): boolean {
    if (!this.state.equals(other.state)) return false;
    if (!this.attestations.equals(other.attestations)) return false;
    return true;
  }

  clone(): StateFragment {
    return new StateFragment(this.state.clone(), this.attestations.clone());
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

  constructor(_0: Map<string, Operation[]>) {
    super();
    this._0 = _0;
  }

  toString(): string {
    return `OperationSet(${[...this._0].map(([backend, ops]) => `${backend} => ${[...].map((op) => op.diff.length).reduce((a, b) => a + b, 0)}b`).join(' ')})`;
  }

  deref(): Map<string, Operation[]> {
    return this._0;
  }

  equals(other: OperationSet): boolean {
    { if (this._0.size !== other._0.size) return false; for (const [k, v] of this._0) { if (!other._0.has(k)) return false; } }
    return true;
  }

  clone(): OperationSet {
    return new OperationSet(new Map(Array.from(this._0.entries()).map(([k, v]) => [k, v])));
  }

  get size(): number {
    return this._0.size;
  }

  [Symbol.iterator](): Iterator<any> {
    return this._0[Symbol.iterator]();
  }

  entries(): IterableIterator<any> {
    return this._0.entries();
  }

  get(key: any): any {
    return this._0.get(key);
  }

  encode(writer: BincodeWriter): void {
    { const _entries = [...this._0.entries()].sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0); writer.writeLength(_entries.length); for (const [k, v] of _entries) { writer.writeString(k); writer.writeVec(v, (w, item) => item.encode(w)); } };
  }

  static decode(reader: BincodeReader): OperationSet {
    const _0 = (() => { const _m = new Map(); const _len = reader.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(reader.readString(), reader.readVec((r) => Operation.decode(r))); } return _m; })();
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
    { if (this.diff.length !== other.diff.length) return false; for (let i = 0; i < this.diff.length; i++) { if (this.diff[i] !== other.diff[i]) return false; } }
    return true;
  }

  clone(): Operation {
    return new Operation(new Uint8Array(this.diff));
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this.diff);
  }

  static decode(reader: BincodeReader): Operation {
    const diff = reader.readByteVec();
    return new Operation(diff);
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

  toString(): string {
    return `EntityState(${this.entityId.toBase64Short()} ${this.state})`;
  }

  equals(other: EntityState): boolean {
    if (!this.entityId.equals(other.entityId)) return false;
    if (!this.collection.equals(other.collection)) return false;
    if (!this.state.equals(other.state)) return false;
    return true;
  }

  clone(): EntityState {
    return new EntityState(this.entityId.clone(), this.collection.clone(), this.state.clone());
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
  readonly stateBuffers: StateBuffers;
  readonly head: Clock;

  constructor(stateBuffers: StateBuffers, head: Clock) {
    super();
    this.stateBuffers = stateBuffers;
    this.head = head;
  }

  toString(): string {
    return `State(${this.head} buffers ${[...this.stateBuffers].map(([backend, buf]) => `${backend} => ${buf.length}b`).join(' ')})`;
  }

  equals(other: State): boolean {
    if (!this.stateBuffers.equals(other.stateBuffers)) return false;
    if (!this.head.equals(other.head)) return false;
    return true;
  }

  clone(): State {
    return new State(this.stateBuffers.clone(), this.head.clone());
  }

  static default(): State {
    return new State(StateBuffers.default(), Clock.default());
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

  constructor(_0: Map<string, Uint8Array>) {
    super();
    this._0 = _0;
  }

  deref(): Map<string, Uint8Array> {
    return this._0;
  }

  equals(other: StateBuffers): boolean {
    { if (this._0.size !== other._0.size) return false; for (const [k, v] of this._0) { if (!other._0.has(k)) return false; } }
    return true;
  }

  clone(): StateBuffers {
    return new StateBuffers(new Map(Array.from(this._0.entries()).map(([k, v]) => [k, v])));
  }

  static default(): StateBuffers {
    return new StateBuffers(new Map());
  }

  get size(): number {
    return this._0.size;
  }

  [Symbol.iterator](): Iterator<any> {
    return this._0[Symbol.iterator]();
  }

  entries(): IterableIterator<any> {
    return this._0.entries();
  }

  get(key: any): any {
    return this._0.get(key);
  }

  encode(writer: BincodeWriter): void {
    { const _entries = [...this._0.entries()].sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0); writer.writeLength(_entries.length); for (const [k, v] of _entries) { writer.writeString(k); writer.writeByteVec(v); } };
  }

  static decode(reader: BincodeReader): StateBuffers {
    const _0 = (() => { const _m = new Map(); const _len = reader.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(reader.readString(), reader.readByteVec()); } return _m; })();
    return new StateBuffers(_0);
  }
}

