// MIRRORS: ankurah/proto/src/update.rs
import { Struct, Enum, Result } from '@ankurah/base';
import { UpdateId } from './id.provided';
import { BincodeReader, BincodeWriter } from './codec';
import { CollectionId } from './collection';
import { EventFragment, StateFragment } from './data';
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
    const _r = `${this.collection}/${this.entityId}: `;
    if (_r.isErr()) return _r as any;
    this.content.match({
      EventOnly: (events) => `Events(${events.length})`.unwrap(),
      StateAndEvent: (state, events) => `State+Events(${state}, ${events.length})`.unwrap(),
    })
    if (!this.predicateRelevance.length === 0) {
      const _r = ` predicates:${this.predicateRelevance.length}`;
      if (_r.isErr()) return _r as any;
    }
    return Result.Ok([]);
  }

  clone(): SubscriptionUpdateItem {
    return new SubscriptionUpdateItem(this.entityId.clone(), this.collection.clone(), this.content.clone(), this.predicateRelevance.map(e => e.clone()));
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    this.content.encode(writer);
    writer.writeVec(this.predicateRelevance, (w, item) => item.encode(w));
  }

  static decode(reader: BincodeReader): SubscriptionUpdateItem {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = UpdateContent.decode(reader);
    const predicateRelevance = reader.readVec((r) => [QueryId, MembershipChange].decode(r));
    return new SubscriptionUpdateItem(entityId, collection, content, predicateRelevance);
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
}

export type NodeUpdateBodyV = {
  SubscriptionUpdate: { items: SubscriptionUpdateItem[] };
};

export class NodeUpdateBody extends Enum<NodeUpdateBodyV> {

  toString(): string {
    return this.match({
      SubscriptionUpdate: (items) => `SubscriptionUpdate [${Array.from(items).map((i) => `${i}`).join(', ')}]`,
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
}

export type UpdateContentV = {
  EventOnly: { _0: EventFragment[] };
  StateAndEvent: { _0: StateFragment; _1: EventFragment[] };
};

export class UpdateContent extends Enum<UpdateContentV> {

  intoParts(): [StateFragment | null, EventFragment[] | null] {
    return this.match({
      EventOnly: (events) => [null, events],
      StateAndEvent: (state, events) => [state, events],
    });
  }

  clone(): UpdateContent {
    return this.match({
      EventOnly: (v) => new UpdateContent('EventOnly', { _0: v._0.map(e => e.clone()) }),
      StateAndEvent: (v) => new UpdateContent('StateAndEvent', { _0: v._0.clone(), _1: v._1.map(e => e.clone()) }),
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
    return true;
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
}

export type NodeUpdateAckBodyV = {
  Success: {};
  Error: { _0: string };
};

export class NodeUpdateAckBody extends Enum<NodeUpdateAckBodyV> {

  toString(): string {
    return this.match({
      Success: () => 'Success',
      Error: (e) => `Error: ${e}`,
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
}

