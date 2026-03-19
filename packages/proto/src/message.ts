// MIRRORS: ankurah/proto/src/message.rs
import { Enum } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { AuthData } from './auth';
import { EntityId } from './id';
import { Presence } from './peering';
import { NodeRequest, NodeResponse } from './request';
import { QueryId } from './subscription';
import { NodeUpdate, NodeUpdateAck } from './update';

export type MessageV = {
  Presence: { _0: Presence };
  PeerMessage: { _0: NodeMessage };
};

export class Message extends Enum<MessageV> {

  toString(): string {
    return this.match({
      Presence: (presence) => `Presence: ${presence}`,
      PeerMessage: (nodeMessage) => `PeerMessage: ${nodeMessage}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Presence: (v) => {
        writer.writeVariant(0);
        v._0.encode(writer);
      },
      PeerMessage: (v) => {
        writer.writeVariant(1);
        v._0.encode(writer);
      },
    });
  }

  static decode(reader: BincodeReader): Message {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const _0 = Presence.decode(reader);
        return new Message('Presence', { _0 });
      }
      case 1: {
        const _0 = NodeMessage.decode(reader);
        return new Message('PeerMessage', { _0 });
      }
      default: throw new Error(`Unknown Message variant: ${variant}`);
    }
  }
}

export type NodeMessageV = {
  Request: { auth: AuthData[]; request: NodeRequest };
  Response: { _0: NodeResponse };
  Update: { _0: NodeUpdate };
  UpdateAck: { _0: NodeUpdateAck };
  UnsubscribeQuery: { from: EntityId; queryId: QueryId };
};

export class NodeMessage extends Enum<NodeMessageV> {

  toString(): string {
    return this.match({
      Request: (request) => `Request: ${request}`,
      Response: (response) => `Response: ${response}`,
      Update: (update) => `Update: ${update}`,
      UpdateAck: (updateAck) => `UpdateAck: ${updateAck}`,
      UnsubscribeQuery: (from, queryId) => `Unsubscribe: ${from} ${queryId}`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      Request: (v) => {
        writer.writeVariant(0);
        writer.writeVec(v.auth, (w, item) => item.encode(w));
        v.request.encode(writer);
      },
      Response: (v) => {
        writer.writeVariant(1);
        v._0.encode(writer);
      },
      Update: (v) => {
        writer.writeVariant(2);
        v._0.encode(writer);
      },
      UpdateAck: (v) => {
        writer.writeVariant(3);
        v._0.encode(writer);
      },
      UnsubscribeQuery: (v) => {
        writer.writeVariant(4);
        v.from.encode(writer);
        v.queryId.encode(writer);
      },
    });
  }

  static decode(reader: BincodeReader): NodeMessage {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const auth = reader.readVec((r) => AuthData.decode(r));
        const request = NodeRequest.decode(reader);
        return new NodeMessage('Request', { auth, request });
      }
      case 1: {
        const _0 = NodeResponse.decode(reader);
        return new NodeMessage('Response', { _0 });
      }
      case 2: {
        const _0 = NodeUpdate.decode(reader);
        return new NodeMessage('Update', { _0 });
      }
      case 3: {
        const _0 = NodeUpdateAck.decode(reader);
        return new NodeMessage('UpdateAck', { _0 });
      }
      case 4: {
        const from = EntityId.decode(reader);
        const queryId = QueryId.decode(reader);
        return new NodeMessage('UnsubscribeQuery', { from, queryId });
      }
      default: throw new Error(`Unknown NodeMessage variant: ${variant}`);
    }
  }
}

