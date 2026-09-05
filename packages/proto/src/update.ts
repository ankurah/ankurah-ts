// MIRRORS: ankurah/proto/src/update.rs
import { Struct, Enum, Result, JsonError, jsonAll, dropOwned, OwnershipFatal } from '@ankurah/base';
import { UpdateId } from './id.provided';
import { BincodeReader, BincodeWriter } from './codec';
import { Attested } from './auth';
import { CollectionId } from './collection';
import { EntityState, EventFragment, State, StateFragment } from './data';
import { EntityId } from './id';
import { QueryId } from './subscription';
export { UpdateId };

export class SubscriptionUpdateItem extends Struct {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly content: UpdateContent;
  readonly predicateRelevance: [QueryId, MembershipChange][];

  constructor(entityId: EntityId, collection: CollectionId, content: UpdateContent, predicateRelevance: [QueryId, MembershipChange][]) {
    super();
    this.entityId = entityId;
    this.collection = collection;
    this.content = content;
    this.predicateRelevance = predicateRelevance;
  }

  toString(): string {
    let _result = '';
    _result += `${this.collection}/${this.entityId}: `;
    _result += this.content.match({
      EventOnly: (v) => {
        const events = v._0;
        return `Events(${events.length})`;
      },
      StateAndEvent: (v) => {
        const state = v._0;
        const events = v._1;
        return `State+Events(${state}, ${events.length})`;
      },
    });
    if (!(this.predicateRelevance.length === 0)) {
      _result += ` predicates:${this.predicateRelevance.length}`;
    }
    return _result;
  }

  clone(): SubscriptionUpdateItem {
    return new SubscriptionUpdateItem(this.entityId.clone(), this.collection.clone(), this.content.clone(), this.predicateRelevance.map(e => [e[0].clone(), e[1].clone()] as [QueryId, MembershipChange]));
  }

  debug(): string {
    return `SubscriptionUpdateItem { entityId: ${this.entityId}, collection: ${this.collection.debug()}, content: ${this.content.debug()}, predicateRelevance: ${this.predicateRelevance} }`;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    this.content.encode(writer);
    writer.writeVec(this.predicateRelevance, (w, item) => { item[0].encode(w); item[1].encode(w) });
  }

  static decode(reader: BincodeReader): SubscriptionUpdateItem {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = UpdateContent.decode(reader);
    const predicateRelevance = reader.readVec((r) => [QueryId.decode(r), MembershipChange.decode(r)] as [QueryId, MembershipChange]);
    return new SubscriptionUpdateItem(entityId, collection, content, predicateRelevance);
  }

  toJSON(): unknown {
    return { 'entity_id': this.entityId.toJSON(), 'collection': this.collection.toJSON(), 'content': this.content.toJSON(), 'predicate_relevance': this.predicateRelevance.map((x) => [x[0].toJSON(), x[1].toJSON()]) };
  }

  static fromJson(value: unknown): Result<SubscriptionUpdateItem, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `SubscriptionUpdateItem`'));
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
      if (!('content' in _o)) {
        return ((e: JsonError) => { dropOwned([entityId, collection]); return Result.Err(e); })(JsonError.custom('missing field `content`'));
      }
      const _rcontent = ((v: unknown) => UpdateContent.fromJson(v))(_o['content']);
      if (_rcontent.isErr()) return ((e: JsonError) => { dropOwned([entityId, collection]); return Result.Err(e); })(_rcontent.unwrapErr());
      const content = _rcontent.unwrap();
      if (!('predicate_relevance' in _o)) {
        return ((e: JsonError) => { dropOwned([entityId, collection, content]); return Result.Err(e); })(JsonError.custom('missing field `predicate_relevance`'));
      }
      const _rpredicateRelevance = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => (Array.isArray(v) && v.length === 2 ? ((a: unknown[]) => jsonAll([((v: unknown) => QueryId.fromJson(v))(a[0]), ((v: unknown) => MembershipChange.fromJson(v))(a[1])]))(v) : Result.Err(JsonError.custom('expected an array of 2'))))) : Result.Err(JsonError.custom('expected an array'))))(_o['predicate_relevance']);
      if (_rpredicateRelevance.isErr()) return ((e: JsonError) => { dropOwned([entityId, collection, content]); return Result.Err(e); })(_rpredicateRelevance.unwrapErr());
      const predicateRelevance = _rpredicateRelevance.unwrap();
      return Result.Ok(new SubscriptionUpdateItem(entityId, collection, content, predicateRelevance));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class NodeUpdate extends Struct {
  readonly id: UpdateId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeUpdateBody;

  constructor(id: UpdateId, from: EntityId, to: EntityId, body: NodeUpdateBody) {
    super();
    this.id = id;
    this.from = from;
    this.to = to;
    this.body = body;
  }

  toString(): string {
    return `Update ${this.id} from ${this.from}->${this.to}: ${this.body}`;
  }

  debug(): string {
    return `NodeUpdate { id: ${this.id}, from: ${this.from}, to: ${this.to}, body: ${this.body.debug()} }`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this.from.encode(writer);
    this.to.encode(writer);
    this.body.encode(writer);
  }

  static decode(reader: BincodeReader): NodeUpdate {
    const id = UpdateId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = NodeUpdateBody.decode(reader);
    return new NodeUpdate(id, from, to, body);
  }

  toJSON(): unknown {
    return { 'id': this.id.toJSON(), 'from': this.from.toJSON(), 'to': this.to.toJSON(), 'body': this.body.toJSON() };
  }

  static fromJson(value: unknown): Result<NodeUpdate, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `NodeUpdate`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('id' in _o)) {
        return Result.Err(JsonError.custom('missing field `id`'));
      }
      const _rid = ((v: unknown) => UpdateId.fromJson(v))(_o['id']);
      if (_rid.isErr()) return Result.Err(_rid.unwrapErr());
      const id = _rid.unwrap();
      if (!('from' in _o)) {
        return ((e: JsonError) => { dropOwned([id]); return Result.Err(e); })(JsonError.custom('missing field `from`'));
      }
      const _rfrom = ((v: unknown) => EntityId.fromJson(v))(_o['from']);
      if (_rfrom.isErr()) return ((e: JsonError) => { dropOwned([id]); return Result.Err(e); })(_rfrom.unwrapErr());
      const from = _rfrom.unwrap();
      if (!('to' in _o)) {
        return ((e: JsonError) => { dropOwned([id, from]); return Result.Err(e); })(JsonError.custom('missing field `to`'));
      }
      const _rto = ((v: unknown) => EntityId.fromJson(v))(_o['to']);
      if (_rto.isErr()) return ((e: JsonError) => { dropOwned([id, from]); return Result.Err(e); })(_rto.unwrapErr());
      const to = _rto.unwrap();
      if (!('body' in _o)) {
        return ((e: JsonError) => { dropOwned([id, from, to]); return Result.Err(e); })(JsonError.custom('missing field `body`'));
      }
      const _rbody = ((v: unknown) => NodeUpdateBody.fromJson(v))(_o['body']);
      if (_rbody.isErr()) return ((e: JsonError) => { dropOwned([id, from, to]); return Result.Err(e); })(_rbody.unwrapErr());
      const body = _rbody.unwrap();
      return Result.Ok(new NodeUpdate(id, from, to, body));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export class NodeUpdateAck extends Struct {
  readonly id: UpdateId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeUpdateAckBody;

  constructor(id: UpdateId, from: EntityId, to: EntityId, body: NodeUpdateAckBody) {
    super();
    this.id = id;
    this.from = from;
    this.to = to;
    this.body = body;
  }

  toString(): string {
    return `UpdateAck(${this.id})`;
  }

  debug(): string {
    return `NodeUpdateAck { id: ${this.id}, from: ${this.from}, to: ${this.to}, body: ${this.body.debug()} }`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this.from.encode(writer);
    this.to.encode(writer);
    this.body.encode(writer);
  }

  static decode(reader: BincodeReader): NodeUpdateAck {
    const id = UpdateId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = NodeUpdateAckBody.decode(reader);
    return new NodeUpdateAck(id, from, to, body);
  }

  toJSON(): unknown {
    return { 'id': this.id.toJSON(), 'from': this.from.toJSON(), 'to': this.to.toJSON(), 'body': this.body.toJSON() };
  }

  static fromJson(value: unknown): Result<NodeUpdateAck, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `NodeUpdateAck`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('id' in _o)) {
        return Result.Err(JsonError.custom('missing field `id`'));
      }
      const _rid = ((v: unknown) => UpdateId.fromJson(v))(_o['id']);
      if (_rid.isErr()) return Result.Err(_rid.unwrapErr());
      const id = _rid.unwrap();
      if (!('from' in _o)) {
        return ((e: JsonError) => { dropOwned([id]); return Result.Err(e); })(JsonError.custom('missing field `from`'));
      }
      const _rfrom = ((v: unknown) => EntityId.fromJson(v))(_o['from']);
      if (_rfrom.isErr()) return ((e: JsonError) => { dropOwned([id]); return Result.Err(e); })(_rfrom.unwrapErr());
      const from = _rfrom.unwrap();
      if (!('to' in _o)) {
        return ((e: JsonError) => { dropOwned([id, from]); return Result.Err(e); })(JsonError.custom('missing field `to`'));
      }
      const _rto = ((v: unknown) => EntityId.fromJson(v))(_o['to']);
      if (_rto.isErr()) return ((e: JsonError) => { dropOwned([id, from]); return Result.Err(e); })(_rto.unwrapErr());
      const to = _rto.unwrap();
      if (!('body' in _o)) {
        return ((e: JsonError) => { dropOwned([id, from, to]); return Result.Err(e); })(JsonError.custom('missing field `body`'));
      }
      const _rbody = ((v: unknown) => NodeUpdateAckBody.fromJson(v))(_o['body']);
      if (_rbody.isErr()) return ((e: JsonError) => { dropOwned([id, from, to]); return Result.Err(e); })(_rbody.unwrapErr());
      const body = _rbody.unwrap();
      return Result.Ok(new NodeUpdateAck(id, from, to, body));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type NodeUpdateBodyV = {
  SubscriptionUpdate: { items: SubscriptionUpdateItem[] };
};

export class NodeUpdateBody extends Enum<NodeUpdateBodyV> {

  toString(): string {
    return this.match({
      SubscriptionUpdate: (v) => {
        const items = v.items;
        return `SubscriptionUpdate [${[...items].map((i) => `${i}`).join(', ')}]`;
      },
    });
  }

  debug(): string {
    return this.match({
      SubscriptionUpdate: (v) => `SubscriptionUpdate { items: ${`[${Array.from(v.items).map((e) => e.debug()).join(', ')}]`} }`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      SubscriptionUpdate: (v) => {
        writer.writeVariant(0);
        writer.writeVec(v.items, (w, item) => item.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): NodeUpdateBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const items = reader.readVec((r) => SubscriptionUpdateItem.decode(r));
        return new NodeUpdateBody('SubscriptionUpdate', { items });
      }
      default: throw new Error(`Unknown NodeUpdateBody variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      SubscriptionUpdate: (v) => ({ 'SubscriptionUpdate': { 'items': v.items.map((x) => x.toJSON()) } }),
    });
  }

  static fromJson(value: unknown): Result<NodeUpdateBody, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `NodeUpdateBody`'));
      }
      const o = value as Record<string, unknown>;
      if ('SubscriptionUpdate' in o) {
        if (o['SubscriptionUpdate'] === null || typeof o['SubscriptionUpdate'] !== 'object' || Array.isArray(o['SubscriptionUpdate'])) {
          return Result.Err(JsonError.custom('expected an object for `NodeUpdateBody`'));
        }
        const _o = o['SubscriptionUpdate'] as Record<string, unknown>;
        if (!('items' in _o)) {
          return Result.Err(JsonError.custom('missing field `items`'));
        }
        const _ritems = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => SubscriptionUpdateItem.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(_o['items']);
        if (_ritems.isErr()) return Result.Err(_ritems.unwrapErr());
        const items = _ritems.unwrap();
        
        return Result.Ok(new NodeUpdateBody('SubscriptionUpdate', { items: items }));
      }
      return Result.Err(JsonError.custom('no variant of `NodeUpdateBody` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type UpdateContentV = {
  EventOnly: { _0: EventFragment[] };
  StateAndEvent: { _0: StateFragment; _1: EventFragment[] };
};

export class UpdateContent extends Enum<UpdateContentV> {

  intoParts(): [StateFragment | null, EventFragment[] | null] {
    return this.intoMatch({
      EventOnly: (v) => {
        const events = v._0;
        return [null, events] as any;
      },
      StateAndEvent: (v) => {
        const state = v._0;
        const events = v._1;
        return [state, events] as any;
      },
    });
  }

  clone(): UpdateContent {
    return this.match({
      EventOnly: (v) => new UpdateContent('EventOnly', { _0: v._0.map(e => e.clone()) }),
      StateAndEvent: (v) => new UpdateContent('StateAndEvent', { _0: v._0.clone(), _1: v._1.map(e => e.clone()) }),
    });
  }

  debug(): string {
    return this.match({
      EventOnly: (v) => `EventOnly(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      StateAndEvent: (v) => `StateAndEvent(${v._0.debug()}, ${`[${Array.from(v._1).map((e) => e.debug()).join(', ')}]`})`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      EventOnly: (v) => {
        writer.writeVariant(0);
        writer.writeVec(v._0, (w, item) => item.encode(w));
      },
      StateAndEvent: (v) => {
        writer.writeVariant(1);
        v._0.encode(writer);
        writer.writeVec(v._1, (w, item) => item.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): UpdateContent {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const _0 = reader.readVec((r) => EventFragment.decode(r));
        return new UpdateContent('EventOnly', { _0 });
      }
      case 1: {
        const _0 = StateFragment.decode(reader);
        const _1 = reader.readVec((r) => EventFragment.decode(r));
        return new UpdateContent('StateAndEvent', { _0, _1 });
      }
      default: throw new Error(`Unknown UpdateContent variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      EventOnly: (v) => ({ 'EventOnly': v._0.map((x) => x.toJSON()) }),
      StateAndEvent: (v) => ({ 'StateAndEvent': [v._0.toJSON(), v._1.map((x) => x.toJSON())] }),
    });
  }

  static fromJson(value: unknown): Result<UpdateContent, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `UpdateContent`'));
      }
      const o = value as Record<string, unknown>;
      if ('EventOnly' in o) {
        const _r_0 = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => EventFragment.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(o['EventOnly']);
        if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
        const _0 = _r_0.unwrap();
        
        return Result.Ok(new UpdateContent('EventOnly', { _0: _0 }));
      }
      if ('StateAndEvent' in o) {
        if (!Array.isArray(o['StateAndEvent']) || o['StateAndEvent'].length !== 2) {
          return Result.Err(JsonError.custom('expected an array of 2 for `UpdateContent`'));
        }
        const _a = o['StateAndEvent'] as unknown[];
        const _r_0 = ((v: unknown) => StateFragment.fromJson(v))(_a[0]);
        if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
        const _0 = _r_0.unwrap();
        const _r_1 = ((v: unknown) => (Array.isArray(v) ? jsonAll(v.map((v) => EventFragment.fromJson(v))) : Result.Err(JsonError.custom('expected an array'))))(_a[1]);
        if (_r_1.isErr()) return ((e: JsonError) => { dropOwned([_0]); return Result.Err(e); })(_r_1.unwrapErr());
        const _1 = _r_1.unwrap();
        
        return Result.Ok(new UpdateContent('StateAndEvent', { _0: _0, _1: _1 }));
      }
      return Result.Err(JsonError.custom('no variant of `UpdateContent` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type MembershipChangeV = {
  Initial: {};
  Add: {};
  Remove: {};
};

export class MembershipChange extends Enum<MembershipChangeV> {

  clone(): MembershipChange {
    return new MembershipChange(this.type, { ...this.value });
  }

  equals(other: MembershipChange): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Initial: () => 'Initial',
      Add: () => 'Add',
      Remove: () => 'Remove',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Initial: (v) => {
        writer.writeVariant(0);
      },
      Add: (v) => {
        writer.writeVariant(1);
      },
      Remove: (v) => {
        writer.writeVariant(2);
      },
    });
  }

  static decode(reader: BincodeReader): MembershipChange {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new MembershipChange('Initial', {});
      }
      case 1: {
        return new MembershipChange('Add', {});
      }
      case 2: {
        return new MembershipChange('Remove', {});
      }
      default: throw new Error(`Unknown MembershipChange variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Initial: () => 'Initial',
      Add: () => 'Add',
      Remove: () => 'Remove',
    });
  }

  static fromJson(value: unknown): Result<MembershipChange, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Initial': return Result.Ok(new MembershipChange('Initial', {}));
          case 'Add': return Result.Ok(new MembershipChange('Add', {}));
          case 'Remove': return Result.Ok(new MembershipChange('Remove', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `MembershipChange`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `MembershipChange` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

export type NodeUpdateAckBodyV = {
  Success: {};
  Error: { _0: string };
};

export class NodeUpdateAckBody extends Enum<NodeUpdateAckBodyV> {

  toString(): string {
    return this.match({
      Success: () => 'Success',
      Error: (v) => {
        const e = v._0;
        return `Error: ${e}`;
      },
    });
  }

  debug(): string {
    return this.match({
      Success: () => 'Success',
      Error: (v) => `Error(${JSON.stringify(v._0)})`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Success: (v) => {
        writer.writeVariant(0);
      },
      Error: (v) => {
        writer.writeVariant(1);
        writer.writeString(v._0);
      },
    });
  }

  static decode(reader: BincodeReader): NodeUpdateAckBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new NodeUpdateAckBody('Success', {});
      }
      case 1: {
        const _0 = reader.readString();
        return new NodeUpdateAckBody('Error', { _0 });
      }
      default: throw new Error(`Unknown NodeUpdateAckBody variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      Success: () => 'Success',
      Error: (v) => ({ 'Error': v._0 }),
    });
  }

  static fromJson(value: unknown): Result<NodeUpdateAckBody, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'Success': return Result.Ok(new NodeUpdateAckBody('Success', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `NodeUpdateAckBody`'));
      }
      const o = value as Record<string, unknown>;
      if ('Error' in o) {
        const _r_0 = ((v: unknown) => (typeof v === 'string' ? Result.Ok(v as string) : Result.Err(JsonError.custom('expected a string'))))(o['Error']);
        if (_r_0.isErr()) return Result.Err(_r_0.unwrapErr());
        const _0 = _r_0.unwrap();
        
        return Result.Ok(new NodeUpdateAckBody('Error', { _0: _0 }));
      }
      return Result.Err(JsonError.custom('no variant of `NodeUpdateAckBody` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

