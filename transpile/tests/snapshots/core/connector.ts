// MIRRORS: ankurah/core/src/connector.rs
import { Enum, Result, timeout } from '@ankurah/base';
import { Attested, EntityState } from '@ankurah/proto';
import { Attested, EntityId, EntityState, NodeMessage, Presence } from '@ankurah/proto';

export type SendErrorV = {
  ConnectionClosed: {};
  Timeout: {};
  Other: { _0: Error };
  Unknown: {};
};

export class SendError extends Enum<SendErrorV> {

  debug(): string {
    return this.match({
      ConnectionClosed: () => 'ConnectionClosed',
      Timeout: () => 'Timeout',
      Other: (v) => `Other(${v._0})`,
      Unknown: () => 'Unknown',
    });
  }

  override toString(): string {
    return this.match({
      ConnectionClosed: () => 'Connection closed',
      Timeout: () => 'Send timeout',
      Other: (v) => `Other error: ${v._0}`,
      Unknown: () => 'Unknown error',
    });
  }

  /** The error this one wraps: Rust's `Error::source`. */
  source(): unknown {
    switch (this.type) {
      case 'Other': return (this.value as any)._0;
      default: return null;
    }
  }

  static fromError(inner: Error): SendError {
    return new SendError('Other', { _0: inner });
  }
}

export interface PeerSender {
  sendMessage(message: NodeMessage): Result<void, SendError>;
  recipientNodeId(): EntityId;
  cloned(): PeerSender;
}

export interface NodeComms {
  id(): EntityId;
  durable(): boolean;
  systemRoot(): Attested<EntityState> | null;
  registerPeer(presence: Presence, sender: PeerSender): void;
  deregisterPeer(nodeId: EntityId): void;
  handleMessage(message: NodeMessage): Promise<Result<void, Error>>;
  cloned(): NodeComms;
}

export function Node_id<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>): EntityId {
  return self.deref().value.id;
}

export function Node_durable<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>): boolean {
  return self.deref().value.durable;
}

export function Node_systemRoot<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>): Attested<EntityState> | null {
  return self.deref().value.system.root();
}

export function Node_registerPeer<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>, presence: Presence, sender: PeerSender): void {
  self.registerPeer(presence, sender);
}

export function Node_deregisterPeer<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>, nodeId: EntityId): void {
  self.deregisterPeer(nodeId);
}

export async function Node_handleMessage<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>, message: NodeMessage): Promise<Result<void, Error>> {
  return await self.handleMessage(message);
}

export function Node_cloned<SE extends StorageEngine, PA extends PolicyAgent>(self: Node<SE, PA>): NodeComms {
  return self.clone();
}

