// MIRRORS: ankurah/proto/src/data.rs
import { Struct, Result, JsonError, jsonAll, jsonMap, dropOwned, OwnershipFatal, HashMap, HashSet, keyHash } from '@ankurah/base';
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
    const _t0 = this.id();
    try {
      return `Event(${_t0.toBase64Short()} ${this.collection}/${this.entityId.toBase64Short()} ${this.isEntityCreate() ? '(create) ' : ''}${this.parent.toBase64Short()} ${[...this.operations.deref()].map(([backend, ops]) => `${backend} => ${[...ops].map((op) => op.diff.length).reduce((a, b) => a + b, 0)}b`).join(' ')})`;
    } finally {
      _t0.drop();
    }
  }

  clone(): Event {
    return new Event(this.collection.clone(), this.entityId.clone(), this.operations.clone(), this.parent.clone());
  }

  debug(): string {
    return `Event { collection: ${this.collection.debug()}, entityId: ${this.entityId}, operations: ${this.operations.debug()}, parent: ${this.parent} }`;
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

  toJSON(): unknown {
    return { 'collection': this.collection.toJSON(), 'entity_id': this.entityId.toJSON(), 'operations': this.operations.toJSON(), 'parent': this.parent.toJSON() };
  }

  static fromJson(value: unknown): Result<Event, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `Event`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('collection' in _o)) {
        return Result.Err(JsonError.custom('missing field `collection`'));
      }
      const _rcollection = ((v: unknown) => CollectionId.fromJson(v))(_o['collection']);
      if (_rcollection.isErr()) return Result.Err(_rcollection.unwrapErr());
      const collection = _rcollection.unwrap();
      if (!('entity_id' in _o)) {
        return ((e: JsonError) => { dropOwned([collection]); return Result.Err(e); })(JsonError.custom('missing field `entity_id`'));
      }
      const _rentityId = ((v: unknown) => EntityId.fromJson(v))(_o['entity_id']);
      if (_rentityId.isErr()) return ((e: JsonError) => { dropOwned([collection]); return Result.Err(e); })(_rentityId.unwrapErr());
      const entityId = _rentityId.unwrap();
      if (!('operations' in _o)) {
        return ((e: JsonError) => { dropOwned([collection, entityId]); return Result.Err(e); })(JsonError.custom('missing field `operations`'));
      }
      const _roperations = ((v: unknown) => OperationSet.fromJson(v))(_o['operations']);
      if (_roperations.isErr()) return ((e: JsonError) => { dropOwned([collection, entityId]); return Result.Err(e); })(_roperations.unwrapErr());
      const operations = _roperations.unwrap();
      if (!('parent' in _o)) {
        return ((e: JsonError) => { dropOwned([collection, entityId, operations]); return Result.Err(e); })(JsonError.custom('missing field `parent`'));
      }
      const _rparent = ((v: unknown) => Clock.fromJson(v))(_o['parent']);
      if (_rparent.isErr()) return ((e: JsonError) => { dropOwned([collection, entityId, operations]); return Result.Err(e); })(_rparent.unwrapErr());
      const parent = _rparent.unwrap();
      return Result.Ok(new Event(collection, entityId, operations, parent));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
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
    try {
      return new EventFragment(attested.payload.takeField('operations'), attested.payload.takeField('parent'), attested.takeField('attestations'));
    } finally {
      attested.drop();
    }
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

  debug(): string {
    return `EventFragment { operations: ${this.operations.debug()}, parent: ${this.parent}, attestations: ${this.attestations.debug()} }`;
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

  toJSON(): unknown {
    return { 'operations': this.operations.toJSON(), 'parent': this.parent.toJSON(), 'attestations': this.attestations.toJSON() };
  }

  static fromJson(value: unknown): Result<EventFragment, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `EventFragment`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('operations' in _o)) {
        return Result.Err(JsonError.custom('missing field `operations`'));
      }
      const _roperations = ((v: unknown) => OperationSet.fromJson(v))(_o['operations']);
      if (_roperations.isErr()) return Result.Err(_roperations.unwrapErr());
      const operations = _roperations.unwrap();
      if (!('parent' in _o)) {
        return ((e: JsonError) => { dropOwned([operations]); return Result.Err(e); })(JsonError.custom('missing field `parent`'));
      }
      const _rparent = ((v: unknown) => Clock.fromJson(v))(_o['parent']);
      if (_rparent.isErr()) return ((e: JsonError) => { dropOwned([operations]); return Result.Err(e); })(_rparent.unwrapErr());
      const parent = _rparent.unwrap();
      if (!('attestations' in _o)) {
        return ((e: JsonError) => { dropOwned([operations, parent]); return Result.Err(e); })(JsonError.custom('missing field `attestations`'));
      }
      const _rattestations = ((v: unknown) => AttestationSet.fromJson(v))(_o['attestations']);
      if (_rattestations.isErr()) return ((e: JsonError) => { dropOwned([operations, parent]); return Result.Err(e); })(_rattestations.unwrapErr());
      const attestations = _rattestations.unwrap();
      return Result.Ok(new EventFragment(operations, parent, attestations));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
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
    try {
      return new StateFragment(attested.payload.takeField('state'), attested.takeField('attestations'));
    } finally {
      attested.drop();
    }
  }

  toString(): string {
    return `StateFragment(state ${this.state} attestations: ${this.attestations.deref().length})`;
  }

  equals(other: StateFragment): boolean {
    if (!this.state.equals(other.state)) return false;
    if (!this.attestations.equals(other.attestations)) return false;
    return true;
  }

  clone(): StateFragment {
    return new StateFragment(this.state.clone(), this.attestations.clone());
  }

  debug(): string {
    return `StateFragment { state: ${this.state.debug()}, attestations: ${this.attestations.debug()} }`;
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

  toJSON(): unknown {
    return { 'state': this.state.toJSON(), 'attestations': this.attestations.toJSON() };
  }

  static fromJson(value: unknown): Result<StateFragment, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `StateFragment`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('state' in _o)) {
        return Result.Err(JsonError.custom('missing field `state`'));
      }
      const _rstate = ((v: unknown) => State.fromJson(v))(_o['state']);
      if (_rstate.isErr()) return Result.Err(_rstate.unwrapErr());
      const state = _rstate.unwrap();
      if (!('attestations' in _o)) {
        return ((e: JsonError) => { dropOwned([state]); return Result.Err(e); })(JsonError.custom('missing field `attestations`'));
      }
      const _rattestations = ((v: unknown) => AttestationSet.fromJson(v))(_o['attestations']);
      if (_rattestations.isErr()) return ((e: JsonError) => { dropOwned([state]); return Result.Err(e); })(_rattestations.unwrapErr());
      const attestations = _rattestations.unwrap();
      return Result.Ok(new StateFragment(state, attestations));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class OperationSet extends Struct {
  readonly _0: HashMap<string, Operation[]>;

  constructor(_0: HashMap<string, Operation[]>) {
    super();
    this._0 = _0;
  }

  toString(): string {
    return `OperationSet(${[...this._0].map(([backend, ops]) => `${backend} => ${[...ops].map((op) => op.diff.length).reduce((a, b) => a + b, 0)}b`).join(' ')})`;
  }

  deref(): HashMap<string, Operation[]> {
    return this._0;
  }

  equals(other: OperationSet): boolean {
    { if (this._0.size !== other._0.size) return false; for (const [k, v] of this._0) { if (!other._0.has(k)) return false; const _w = other._0.get(k)!; if (!v.equals(_w)) return false; } }
    return true;
  }

  clone(): OperationSet {
    return new OperationSet(this._0.clone());
  }

  debug(): string {
    return `OperationSet(${this._0})`;
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
    { const _entries = [...this._0.entries()].sort((a, b) => { const x = [...a[0]], y = [...b[0]]; const n = Math.min(x.length, y.length); for (let i = 0; i < n; i++) { const d = (x[i].codePointAt(0) ?? 0) - (y[i].codePointAt(0) ?? 0); if (d !== 0) return d < 0 ? -1 : 1; } return x.length === y.length ? 0 : (x.length < y.length ? -1 : 1); }); writer.writeLength(_entries.length); for (const [k, v] of _entries) { writer.writeString(k); writer.writeVec(v, (w, item) => item.encode(w)); } };
  }

  static decode(reader: BincodeReader): OperationSet {
    const _0 = (() => { const _m = new HashMap<string, Operation[]>(); const _len = reader.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(reader.readString(), reader.readVec((r) => Operation.decode(r))); } return _m; })();
    return new OperationSet(_0);
  }

  toJSON(): unknown {
    return Object.fromEntries([...this._0.entries()].map(([k, x]) => [k, x.map((x) => x.toJSON())]));
  }

  static fromJson(value: unknown): Result<OperationSet, JsonError> {
    try {
      const _r_0 = ((v: unknown) => (v !== null && typeof v === 'object' && !Array.isArray(v) ? jsonMap(jsonAll(Object.entries(v as Record<string, unknown>).map(([k, v]) => jsonMap(((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => Operation.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(v), (x) => [k, x] as [string, Operation[]]))), (entries) => new HashMap<string, Operation[]>(entries)) : Result.Err(JsonError.custom('expected an object'))))(value);
      if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
      const _0 = _r_0.unwrap();
      return Result.Ok(new OperationSet(_0));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
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

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [keyHash(this.diff)].map((p) => p.length + ':' + p).join('');
  }

  clone(): Operation {
    return new Operation(new Uint8Array(this.diff));
  }

  debug(): string {
    return `Operation { diff: ${`[${Array.from(this.diff).map((e) => String(e)).join(', ')}]`} }`;
  }

  encode(writer: BincodeWriter): void {
    writer.writeByteVec(this.diff);
  }

  static decode(reader: BincodeReader): Operation {
    const diff = reader.readByteVec();
    return new Operation(diff);
  }

  toJSON(): unknown {
    return { 'diff': Array.from(this.diff) };
  }

  static fromJson(value: unknown): Result<Operation, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `Operation`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('diff' in _o)) {
        return Result.Err(JsonError.custom('missing field `diff`'));
      }
      const _rdiff = ((v: unknown) => (Array.isArray(v) && v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255) ? Result.Ok(new Uint8Array(v as number[])) : Result.Err(JsonError.custom('expected an array of bytes'))))(_o['diff']);
      if (_rdiff.isErr()) return Result.Err(_rdiff.unwrapErr());
      const diff = _rdiff.unwrap();
      return Result.Ok(new Operation(diff));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
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

  debug(): string {
    return `EntityState { entityId: ${this.entityId}, collection: ${this.collection.debug()}, state: ${this.state.debug()} }`;
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

  toJSON(): unknown {
    return { 'entity_id': this.entityId.toJSON(), 'collection': this.collection.toJSON(), 'state': this.state.toJSON() };
  }

  static fromJson(value: unknown): Result<EntityState, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `EntityState`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('entity_id' in _o)) {
        return Result.Err(JsonError.custom('missing field `entity_id`'));
      }
      const _rentityId = ((v: unknown) => EntityId.fromJson(v))(_o['entity_id']);
      if (_rentityId.isErr()) return Result.Err(_rentityId.unwrapErr());
      const entityId = _rentityId.unwrap();
      if (!('collection' in _o)) {
        return ((e: JsonError) => { dropOwned([entityId]); return Result.Err(e); })(JsonError.custom('missing field `collection`'));
      }
      const _rcollection = ((v: unknown) => CollectionId.fromJson(v))(_o['collection']);
      if (_rcollection.isErr()) return ((e: JsonError) => { dropOwned([entityId]); return Result.Err(e); })(_rcollection.unwrapErr());
      const collection = _rcollection.unwrap();
      if (!('state' in _o)) {
        return ((e: JsonError) => { dropOwned([entityId, collection]); return Result.Err(e); })(JsonError.custom('missing field `state`'));
      }
      const _rstate = ((v: unknown) => State.fromJson(v))(_o['state']);
      if (_rstate.isErr()) return ((e: JsonError) => { dropOwned([entityId, collection]); return Result.Err(e); })(_rstate.unwrapErr());
      const state = _rstate.unwrap();
      return Result.Ok(new EntityState(entityId, collection, state));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
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
    return `State(${this.head} buffers ${[...this.stateBuffers.deref()].map(([backend, buf]) => `${backend} => ${buf.length}b`).join(' ')})`;
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

  debug(): string {
    return `State { stateBuffers: ${this.stateBuffers.debug()}, head: ${this.head} }`;
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

  toJSON(): unknown {
    return { 'state_buffers': this.stateBuffers.toJSON(), 'head': this.head.toJSON() };
  }

  static fromJson(value: unknown): Result<State, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `State`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('state_buffers' in _o)) {
        return Result.Err(JsonError.custom('missing field `state_buffers`'));
      }
      const _rstateBuffers = ((v: unknown) => StateBuffers.fromJson(v))(_o['state_buffers']);
      if (_rstateBuffers.isErr()) return Result.Err(_rstateBuffers.unwrapErr());
      const stateBuffers = _rstateBuffers.unwrap();
      if (!('head' in _o)) {
        return ((e: JsonError) => { dropOwned([stateBuffers]); return Result.Err(e); })(JsonError.custom('missing field `head`'));
      }
      const _rhead = ((v: unknown) => Clock.fromJson(v))(_o['head']);
      if (_rhead.isErr()) return ((e: JsonError) => { dropOwned([stateBuffers]); return Result.Err(e); })(_rhead.unwrapErr());
      const head = _rhead.unwrap();
      return Result.Ok(new State(stateBuffers, head));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class StateBuffers extends Struct {
  readonly _0: HashMap<string, Uint8Array>;

  constructor(_0: HashMap<string, Uint8Array>) {
    super();
    this._0 = _0;
  }

  deref(): HashMap<string, Uint8Array> {
    return this._0;
  }

  equals(other: StateBuffers): boolean {
    { if (this._0.size !== other._0.size) return false; for (const [k, v] of this._0) { if (!other._0.has(k)) return false; const _w = other._0.get(k)!; if (!v.equals(_w)) return false; } }
    return true;
  }

  clone(): StateBuffers {
    return new StateBuffers(this._0.clone());
  }

  static default(): StateBuffers {
    return new StateBuffers(new HashMap());
  }

  debug(): string {
    return `StateBuffers(${this._0})`;
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
    { const _entries = [...this._0.entries()].sort((a, b) => { const x = [...a[0]], y = [...b[0]]; const n = Math.min(x.length, y.length); for (let i = 0; i < n; i++) { const d = (x[i].codePointAt(0) ?? 0) - (y[i].codePointAt(0) ?? 0); if (d !== 0) return d < 0 ? -1 : 1; } return x.length === y.length ? 0 : (x.length < y.length ? -1 : 1); }); writer.writeLength(_entries.length); for (const [k, v] of _entries) { writer.writeString(k); writer.writeByteVec(v); } };
  }

  static decode(reader: BincodeReader): StateBuffers {
    const _0 = (() => { const _m = new HashMap<string, Uint8Array>(); const _len = reader.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(reader.readString(), reader.readByteVec()); } return _m; })();
    return new StateBuffers(_0);
  }

  toJSON(): unknown {
    return Object.fromEntries([...this._0.entries()].map(([k, x]) => [k, Array.from(x)]));
  }

  static fromJson(value: unknown): Result<StateBuffers, JsonError> {
    try {
      const _r_0 = ((v: unknown) => (v !== null && typeof v === 'object' && !Array.isArray(v) ? jsonMap(jsonAll(Object.entries(v as Record<string, unknown>).map(([k, v]) => jsonMap(((v: unknown) => (Array.isArray(v) && v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255) ? Result.Ok(new Uint8Array(v as number[])) : Result.Err(JsonError.custom('expected an array of bytes'))))(v), (x) => [k, x] as [string, Uint8Array]))), (entries) => new HashMap<string, Uint8Array>(entries)) : Result.Err(JsonError.custom('expected an object'))))(value);
      if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
      const _0 = _r_0.unwrap();
      return Result.Ok(new StateBuffers(_0));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

