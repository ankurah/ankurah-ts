// MIRRORS: ankurah/proto/src/message.rs

import { BincodeReader, BincodeWriter } from './codec';
import { AuthData } from './auth';
import { EntityId, QueryId } from './id';
import { Presence } from './peering';
import { NodeRequest, NodeResponse } from './request';
import { NodeUpdate, NodeUpdateAck } from './update';

// ─── NodeMessage ────────────────────────────────────────────────────────────

/**
 * NodeMessage enum.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = Request { auth: Vec<AuthData>, request: NodeRequest }
 *   1 = Response(NodeResponse)
 *   2 = Update(NodeUpdate)
 *   3 = UpdateAck(NodeUpdateAck)
 *   4 = UnsubscribeQuery { from: EntityId, query_id: QueryId }
 */
export type NodeMessage =
  | { type: 'Request'; auth: AuthData[]; request: NodeRequest }
  | { type: 'Response'; response: NodeResponse }
  | { type: 'Update'; update: NodeUpdate }
  | { type: 'UpdateAck'; updateAck: NodeUpdateAck }
  | { type: 'UnsubscribeQuery'; from: EntityId; queryId: QueryId };

export function encodeNodeMessage(writer: BincodeWriter, msg: NodeMessage): void {
  switch (msg.type) {
    case 'Request':
      writer.writeVariant(0);
      writer.writeVec(msg.auth, (w, a) => a.encode(w));
      msg.request.encode(writer);
      break;
    case 'Response':
      writer.writeVariant(1);
      msg.response.encode(writer);
      break;
    case 'Update':
      writer.writeVariant(2);
      msg.update.encode(writer);
      break;
    case 'UpdateAck':
      writer.writeVariant(3);
      msg.updateAck.encode(writer);
      break;
    case 'UnsubscribeQuery':
      writer.writeVariant(4);
      msg.from.encode(writer);
      msg.queryId.encode(writer);
      break;
  }
}

export function decodeNodeMessage(reader: BincodeReader): NodeMessage {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const auth = reader.readVec(r => AuthData.decode(r));
      const request = NodeRequest.decode(reader);
      return { type: 'Request', auth, request };
    }
    case 1: {
      const response = NodeResponse.decode(reader);
      return { type: 'Response', response };
    }
    case 2: {
      const update = NodeUpdate.decode(reader);
      return { type: 'Update', update };
    }
    case 3: {
      const updateAck = NodeUpdateAck.decode(reader);
      return { type: 'UpdateAck', updateAck };
    }
    case 4: {
      const from = EntityId.decode(reader);
      const queryId = QueryId.decode(reader);
      return { type: 'UnsubscribeQuery', from, queryId };
    }
    default:
      throw new Error(`Unknown NodeMessage variant: ${variant}`);
  }
}

function nodeMessageToString(msg: NodeMessage): string {
  switch (msg.type) {
    case 'Request': return `Request: ${msg.request}`;
    case 'Response': return `Response: ${msg.response}`;
    case 'Update': return `Update: ${msg.update}`;
    case 'UpdateAck': return `UpdateAck: ${msg.updateAck}`;
    case 'UnsubscribeQuery': return `Unsubscribe: ${msg.from} ${msg.queryId}`;
  }
}

// ─── Message ────────────────────────────────────────────────────────────────

/**
 * Message: top-level protocol message.
 * Derived serde as enum.
 *
 * Variant indices:
 *   0 = Presence(Presence)
 *   1 = PeerMessage(NodeMessage)
 */
export type Message =
  | { type: 'Presence'; presence: Presence }
  | { type: 'PeerMessage'; nodeMessage: NodeMessage };

export function encodeMessage(writer: BincodeWriter, msg: Message): void {
  switch (msg.type) {
    case 'Presence':
      writer.writeVariant(0);
      msg.presence.encode(writer);
      break;
    case 'PeerMessage':
      writer.writeVariant(1);
      encodeNodeMessage(writer, msg.nodeMessage);
      break;
  }
}

export function decodeMessage(reader: BincodeReader): Message {
  const variant = reader.readVariant();
  switch (variant) {
    case 0: {
      const presence = Presence.decode(reader);
      return { type: 'Presence', presence };
    }
    case 1: {
      const nodeMessage = decodeNodeMessage(reader);
      return { type: 'PeerMessage', nodeMessage };
    }
    default:
      throw new Error(`Unknown Message variant: ${variant}`);
  }
}

export function messageToString(msg: Message): string {
  switch (msg.type) {
    case 'Presence': return `Presence: ${msg.presence}`;
    case 'PeerMessage': return `PeerMessage: ${nodeMessageToString(msg.nodeMessage)}`;
  }
}
