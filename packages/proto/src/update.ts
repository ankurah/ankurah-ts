// MIRRORS: ankurah/proto/src/update.rs

import { BincodeReader, BincodeWriter } from './codec';
import { EntityId, UpdateId, QueryId } from './id';
import { CollectionId } from './collection';
import { EventFragment, StateFragment } from './data';

// ─── MembershipChange ───────────────────────────────────────────────────────

/**
 * MembershipChange: how an entity's membership changed for a specific predicate.
 * Derived serde as enum (u32 variant index, no fields).
 *
 * Variant indices:
 *   0 = Initial
 *   1 = Add
 *   2 = Remove
 */
export type MembershipChange =
  | { type: 'Initial' }
  | { type: 'Add' }
  | { type: 'Remove' };

export function encodeMembershipChange(writer: BincodeWriter, mc: MembershipChange): void {
  switch (mc.type) {
    case 'Initial': writer.writeVariant(0); break;
    case 'Add': writer.writeVariant(1); break;
    case 'Remove': writer.writeVariant(2); break;
  }
}

export function decodeMembershipChange(reader: BincodeReader): MembershipChange {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: return { type: 'Initial' };
    case 1: return { type: 'Add' };
    case 2: return { type: 'Remove' };
    default: throw new Error(`Unknown MembershipChange variant: ${variant}`);
  }
}

// ─── UpdateContent ──────────────────────────────────────────────────────────

/**
 * UpdateContent: content of an update.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = EventOnly(Vec<EventFragment>)
 *   1 = StateAndEvent(StateFragment, Vec<EventFragment>)
 */
export type UpdateContent =
  | { type: 'EventOnly'; events: EventFragment[] }
  | { type: 'StateAndEvent'; state: StateFragment; events: EventFragment[] };

export function encodeUpdateContent(writer: BincodeWriter, content: UpdateContent): void {
  switch (content.type) {
    case 'EventOnly':
      writer.writeVariant(0);
      writer.writeVec(content.events, (w, e) => e.encode(w));
      break;
    case 'StateAndEvent':
      writer.writeVariant(1);
      content.state.encode(writer);
      writer.writeVec(content.events, (w, e) => e.encode(w));
      break;
  }
}

export function decodeUpdateContent(reader: BincodeReader): UpdateContent {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const events = reader.readVec(r => EventFragment.decode(r));
      return { type: 'EventOnly', events };
    }
    case 1: {
      const state = StateFragment.decode(reader);
      const events = reader.readVec(r => EventFragment.decode(r));
      return { type: 'StateAndEvent', state, events };
    }
    default:
      throw new Error(`Unknown UpdateContent variant: ${variant}`);
  }
}

/** Decompose UpdateContent into optional state and event fragments. */
export function updateContentIntoParts(
  content: UpdateContent,
): { state: StateFragment | null; events: EventFragment[] | null } {
  switch (content.type) {
    case 'EventOnly':
      return { state: null, events: content.events };
    case 'StateAndEvent':
      return { state: content.state, events: content.events };
  }
}

// ─── SubscriptionUpdateItem ─────────────────────────────────────────────────

/**
 * SubscriptionUpdateItem: a single entity update with subscription relevance info.
 * Derived serde — struct { entity_id, collection, content, predicate_relevance }.
 */
export class SubscriptionUpdateItem {
  readonly entityId: EntityId;
  readonly collection: CollectionId;
  readonly content: UpdateContent;
  readonly predicateRelevance: Array<[QueryId, MembershipChange]>;

  constructor(
    entityId: EntityId,
    collection: CollectionId,
    content: UpdateContent,
    predicateRelevance: Array<[QueryId, MembershipChange]>,
  ) {
    this.entityId = entityId;
    this.collection = collection;
    this.content = content;
    this.predicateRelevance = predicateRelevance;
  }

  toString(): string {
    let contentStr: string;
    switch (this.content.type) {
      case 'EventOnly':
        contentStr = `Events(${this.content.events.length})`;
        break;
      case 'StateAndEvent':
        contentStr = `State+Events(${this.content.state}, ${this.content.events.length})`;
        break;
    }
    const predStr = this.predicateRelevance.length > 0
      ? ` predicates:${this.predicateRelevance.length}`
      : '';
    return `${this.collection}/${this.entityId}: ${contentStr}${predStr}`;
  }

  encode(writer: BincodeWriter): void {
    this.entityId.encode(writer);
    this.collection.encode(writer);
    encodeUpdateContent(writer, this.content);
    // Vec<(QueryId, MembershipChange)>
    writer.writeVec(this.predicateRelevance, (w, [qid, mc]) => {
      qid.encode(w);
      encodeMembershipChange(w, mc);
    });
  }

  static decode(reader: BincodeReader): SubscriptionUpdateItem {
    const entityId = EntityId.decode(reader);
    const collection = CollectionId.decode(reader);
    const content = decodeUpdateContent(reader);
    const predicateRelevance = reader.readVec(r => {
      const qid = QueryId.decode(r);
      const mc = decodeMembershipChange(r);
      return [qid, mc] as [QueryId, MembershipChange];
    });
    return new SubscriptionUpdateItem(entityId, collection, content, predicateRelevance);
  }
}

// ─── NodeUpdateBody ─────────────────────────────────────────────────────────

/**
 * NodeUpdateBody enum.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = SubscriptionUpdate { items: Vec<SubscriptionUpdateItem> }
 */
export type NodeUpdateBody =
  | { type: 'SubscriptionUpdate'; items: SubscriptionUpdateItem[] };

export function encodeNodeUpdateBody(writer: BincodeWriter, body: NodeUpdateBody): void {
  switch (body.type) {
    case 'SubscriptionUpdate':
      writer.writeVariant(0);
      writer.writeVec(body.items, (w, item) => item.encode(w));
      break;
  }
}

export function decodeNodeUpdateBody(reader: BincodeReader): NodeUpdateBody {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const items = reader.readVec(r => SubscriptionUpdateItem.decode(r));
      return { type: 'SubscriptionUpdate', items };
    }
    default:
      throw new Error(`Unknown NodeUpdateBody variant: ${variant}`);
  }
}

// ─── NodeUpdate ─────────────────────────────────────────────────────────────

/**
 * NodeUpdate: an update from one node to another.
 * Derived serde — struct { id, from, to, body }.
 */
export class NodeUpdate {
  readonly id: UpdateId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeUpdateBody;

  constructor(id: UpdateId, from: EntityId, to: EntityId, body: NodeUpdateBody) {
    this.id = id;
    this.from = from;
    this.to = to;
    this.body = body;
  }

  toString(): string {
    return `Update ${this.id} from ${this.from}->${this.to}: ${nodeUpdateBodyToString(this.body)}`;
  }

  encode(writer: BincodeWriter): void {
    this.id.encode(writer);
    this.from.encode(writer);
    this.to.encode(writer);
    encodeNodeUpdateBody(writer, this.body);
  }

  static decode(reader: BincodeReader): NodeUpdate {
    const id = UpdateId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = decodeNodeUpdateBody(reader);
    return new NodeUpdate(id, from, to, body);
  }
}

function nodeUpdateBodyToString(body: NodeUpdateBody): string {
  switch (body.type) {
    case 'SubscriptionUpdate':
      return `SubscriptionUpdate [${body.items.map(i => `${i}`).join(', ')}]`;
  }
}

// ─── NodeUpdateAckBody ──────────────────────────────────────────────────────

/**
 * NodeUpdateAckBody enum.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = Success
 *   1 = Error(String)
 */
export type NodeUpdateAckBody =
  | { type: 'Success' }
  | { type: 'Error'; message: string };

export function encodeNodeUpdateAckBody(writer: BincodeWriter, body: NodeUpdateAckBody): void {
  switch (body.type) {
    case 'Success':
      writer.writeVariant(0);
      break;
    case 'Error':
      writer.writeVariant(1);
      writer.writeString(body.message);
      break;
  }
}

export function decodeNodeUpdateAckBody(reader: BincodeReader): NodeUpdateAckBody {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: return { type: 'Success' };
    case 1: {
      const message = reader.readString();
      return { type: 'Error', message };
    }
    default:
      throw new Error(`Unknown NodeUpdateAckBody variant: ${variant}`);
  }
}

// ─── NodeUpdateAck ──────────────────────────────────────────────────────────

/**
 * NodeUpdateAck: acknowledgement of an update.
 * Derived serde — struct { id, from, to, body }.
 */
export class NodeUpdateAck {
  readonly id: UpdateId;
  readonly from: EntityId;
  readonly to: EntityId;
  readonly body: NodeUpdateAckBody;

  constructor(id: UpdateId, from: EntityId, to: EntityId, body: NodeUpdateAckBody) {
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
    encodeNodeUpdateAckBody(writer, this.body);
  }

  static decode(reader: BincodeReader): NodeUpdateAck {
    const id = UpdateId.decode(reader);
    const from = EntityId.decode(reader);
    const to = EntityId.decode(reader);
    const body = decodeNodeUpdateAckBody(reader);
    return new NodeUpdateAck(id, from, to, body);
  }
}
