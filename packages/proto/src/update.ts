// MIRRORS: ankurah/proto/src/update.rs

import { Struct, Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { EntityId, UpdateId, QueryId } from './id';
import { CollectionId } from './collection';
import { Attested } from './auth';
import { EntityState, EventFragment, StateFragment, attestedEntityStateFromParts } from './data';

export { UpdateId };

// ─── NodeUpdateBody ──────────────────────────────────────────────────────────

type NodeUpdateBodyV = {
  /// New events for a subscription
  SubscriptionUpdate: { items: SubscriptionUpdateItem[] };
};

export class NodeUpdateBody extends Enum<NodeUpdateBodyV> {
  // impl Display for NodeUpdateBody
  toString(): string {
    return this.match({
      SubscriptionUpdate: (v) =>
        `SubscriptionUpdate [${v.items.map((i) => `${i}`).join(', ')}]`,
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
      default:
        throw new Error(`Unknown NodeUpdateBody variant: ${variant}`);
    }
  }
}

// ─── UpdateContent ───────────────────────────────────────────────────────────

type UpdateContentV = {
  /// Only events, no state (peer already has the state)
  EventOnly: { events: EventFragment[] };
  /// Both state and events (peer needs both)
  StateAndEvent: { state: StateFragment; events: EventFragment[] };
};

export class UpdateContent extends Enum<UpdateContentV> {
  /// Decompose into optional state and event fragments
  intoParts(): { state: StateFragment | null; events: EventFragment[] | null } {
    return this.match({
      EventOnly: (v) => ({ state: null, events: v.events }),
      StateAndEvent: (v) => ({ state: v.state as StateFragment | null, events: v.events }),
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      EventOnly: (v) => {
        writer.writeVariant(0);
        writer.writeVec(v.events, (w, e) => e.encode(w));
      },
      StateAndEvent: (v) => {
        writer.writeVariant(1);
        v.state.encode(writer);
        writer.writeVec(v.events, (w, e) => e.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): UpdateContent {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const events = reader.readVec((r) => EventFragment.decode(r));
        return new UpdateContent('EventOnly', { events });
      }
      case 1: {
        const state = StateFragment.decode(reader);
        const events = reader.readVec((r) => EventFragment.decode(r));
        return new UpdateContent('StateAndEvent', { state, events });
      }
      default:
        throw new Error(`Unknown UpdateContent variant: ${variant}`);
    }
  }
}

// ─── MembershipChange ────────────────────────────────────────────────────────

type MembershipChangeV = {
  /// First time seeing this entity for this predicate
  Initial: {};
  /// Entity now matches predicate (wasn't matching before)
  Add: {};
  /// Entity no longer matches predicate (was matching before)
  Remove: {};
};

export class MembershipChange extends Enum<MembershipChangeV> {
  // derive(PartialEq)
  equals(other: MembershipChange): boolean {
    return this.type === other.type;
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Initial: () => { writer.writeVariant(0); },
      Add: () => { writer.writeVariant(1); },
      Remove: () => { writer.writeVariant(2); },
    });
  }

  static decode(reader: BincodeReader): MembershipChange {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: return new MembershipChange('Initial', {});
      case 1: return new MembershipChange('Add', {});
      case 2: return new MembershipChange('Remove', {});
      default:
        throw new Error(`Unknown MembershipChange variant: ${variant}`);
    }
  }
}

// ─── SubscriptionUpdateItem ──────────────────────────────────────────────────

export class SubscriptionUpdateItem extends Struct {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly content: UpdateContent;
  /// Which predicates this update is relevant to and how
  /// Uses PredicateId for remote subscriptions
  readonly predicateRelevance: Array<[QueryId, MembershipChange]>;

  constructor(
    entityId: EntityId,
    collection: CollectionId,
    content: UpdateContent,
    predicateRelevance: Array<[QueryId, MembershipChange]>,
  ) {
    super();
    this.entityId = entityId;
    this.collection = collection;
    this.content = content;
    this.predicateRelevance = predicateRelevance;
  }

  // impl TryFrom<SubscriptionUpdateItem> for Attested<EntityState>
  tryIntoAttestedEntityState(): Attested<EntityState> {
    return this.content.match({
      StateAndEvent: (v) =>
        attestedEntityStateFromParts(this.entityId, this.collection, v.state),
      EventOnly: () => {
        throw new Error('Cannot convert event-only update to entity state');
      },
    });
  }

  // impl Display for SubscriptionUpdateItem
  toString(): string {
    const contentStr = this.content.match({
      EventOnly: (v) => `Events(${v.events.length})`,
      StateAndEvent: (v) => `State+Events(${v.state}, ${v.events.length})`,
    });

    const predStr = this.predicateRelevance.length > 0
      ? ` predicates:${this.predicateRelevance.length}`
      : '';

    return `${this.collection}/${this.entityId}: ${contentStr}${predStr}`;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    this.content.encode(writer);
    // Vec<(QueryId, MembershipChange)>
    writer.writeVec(this.predicateRelevance, (w, [qid, mc]) => {
      qid.encode(w);
      mc.encode(w);
    });
  }

  static decode(reader: BincodeReader): SubscriptionUpdateItem {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = UpdateContent.decode(reader);
    const predicateRelevance = reader.readVec((r) => {
      const qid = QueryId.decode(r);
      const mc = MembershipChange.decode(r);
      return [qid, mc] as [QueryId, MembershipChange];
    });
    return new SubscriptionUpdateItem(entityId, collection, content, predicateRelevance);
  }
}

// ─── NodeUpdate ──────────────────────────────────────────────────────────────

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

  // impl Display for NodeUpdate
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

// ─── NodeUpdateAck ───────────────────────────────────────────────────────────

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

  // impl Display for NodeUpdateAck
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

// ─── NodeUpdateAckBody ───────────────────────────────────────────────────────

type NodeUpdateAckBodyV = {
  Success: {};
  Error: { message: string };
};

export class NodeUpdateAckBody extends Enum<NodeUpdateAckBodyV> {
  // impl Display for NodeUpdateAckBody
  toString(): string {
    return this.match({
      Success: () => 'Success',
      Error: (v) => `Error: ${v.message}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Success: () => {
        writer.writeVariant(0);
      },
      Error: (v) => {
        writer.writeVariant(1);
        writer.writeString(v.message);
      },
    });
  }

  static decode(reader: BincodeReader): NodeUpdateAckBody {
    const variant = reader.readVariant();
    switch (variant) {
      case 0:
        return new NodeUpdateAckBody('Success', {});
      case 1: {
        const message = reader.readString();
        return new NodeUpdateAckBody('Error', { message });
      }
      default:
        throw new Error(`Unknown NodeUpdateAckBody variant: ${variant}`);
    }
  }
}
