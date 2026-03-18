// MIRRORS: ankurah/proto/src/message.rs

import { Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { AuthData } from './auth';
import { CollectionId } from './collection';
import { EntityId, QueryId } from './id';
import { Presence } from './peering';
import { NodeRequest, NodeResponse, EntityIdRange } from './request';
import { NodeUpdate, NodeUpdateAck } from './update';

// Divergence: EntityIdRange re-exported here for convenience — not present in Rust message.rs [E4]
export { EntityIdRange };

// ─── Message ────────────────────────────────────────────────────────────────

type MessageV = {
  Presence: { presence: Presence };
  PeerMessage: { nodeMessage: NodeMessage };
};

export class Message extends Enum<MessageV> {
  // impl Display for Message
  toString(): string {
    return this.match({
      Presence: (v) => `Presence: ${v.presence}`,
      PeerMessage: (v) => `PeerMessage: ${v.nodeMessage}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Presence: (v) => {
        writer.writeVariant(0);
        v.presence.encode(writer);
      },
      PeerMessage: (v) => {
        writer.writeVariant(1);
        v.nodeMessage.encode(writer);
      },
    });
  }

  static decode(reader: BincodeReader): Message {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const presence = Presence.decode(reader);
        return new Message('Presence', { presence });
      }
      case 1: {
        const nodeMessage = NodeMessage.decode(reader);
        return new Message('PeerMessage', { nodeMessage });
      }
      default:
        throw new Error(`Unknown Message variant: ${variant}`);
    }
  }
}

// ─── NodeMessage ────────────────────────────────────────────────────────────

type NodeMessageV = {
  Request: { auth: AuthData[]; request: NodeRequest };
  Response: { response: NodeResponse };
  Update: { update: NodeUpdate };
  UpdateAck: { updateAck: NodeUpdateAck };
  UnsubscribeQuery: { from: EntityId; queryId: QueryId };
  // Divergence: UnsubscribeEntities not in Rust — TS-ahead variant for entity-level unsubscribe [E4]
  UnsubscribeEntities: { from: EntityId; collection: CollectionId; ranges: EntityIdRange[] };
};

export class NodeMessage extends Enum<NodeMessageV> {
  // impl Display for NodeMessage
  toString(): string {
    return this.match({
      Request: (v) => `Request: ${v.request}`,
      Response: (v) => `Response: ${v.response}`,
      Update: (v) => `Update: ${v.update}`,
      UpdateAck: (v) => `UpdateAck: ${v.updateAck}`,
      UnsubscribeQuery: (v) => `Unsubscribe: ${v.from} ${v.queryId}`,
      UnsubscribeEntities: (v) => `UnsubscribeEntities: ${v.from} ${v.collection} ranges:${v.ranges.length}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Request: (v) => {
        writer.writeVariant(0);
        writer.writeVec(v.auth, (w, a) => a.encode(w));
        v.request.encode(writer);
      },
      Response: (v) => {
        writer.writeVariant(1);
        v.response.encode(writer);
      },
      Update: (v) => {
        writer.writeVariant(2);
        v.update.encode(writer);
      },
      UpdateAck: (v) => {
        writer.writeVariant(3);
        v.updateAck.encode(writer);
      },
      UnsubscribeQuery: (v) => {
        writer.writeVariant(4);
        v.from.encode(writer);
        v.queryId.encode(writer);
      },
      UnsubscribeEntities: (v) => {
        writer.writeVariant(5);
        v.from.encode(writer);
        v.collection.encode(writer);
        writer.writeVec(v.ranges, (w, r) => r.encode(w));
      },
    });
  }

  static decode(reader: BincodeReader): NodeMessage {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const auth = reader.readVec(r => AuthData.decode(r));
        const request = NodeRequest.decode(reader);
        return new NodeMessage('Request', { auth, request });
      }
      case 1: {
        const response = NodeResponse.decode(reader);
        return new NodeMessage('Response', { response });
      }
      case 2: {
        const update = NodeUpdate.decode(reader);
        return new NodeMessage('Update', { update });
      }
      case 3: {
        const updateAck = NodeUpdateAck.decode(reader);
        return new NodeMessage('UpdateAck', { updateAck });
      }
      case 4: {
        const from = EntityId.decode(reader);
        const queryId = QueryId.decode(reader);
        return new NodeMessage('UnsubscribeQuery', { from, queryId });
      }
      case 5: {
        const from = EntityId.decode(reader);
        const collection = CollectionId.decode(reader);
        const ranges = reader.readVec(r => EntityIdRange.decode(r));
        return new NodeMessage('UnsubscribeEntities', { from, collection, ranges });
      }
      default:
        throw new Error(`Unknown NodeMessage variant: ${variant}`);
    }
  }
}
