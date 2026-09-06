// MIRRORS: ankurah/connectors/local-process/src/lib.rs
import { Struct, Drop, Result, dropOwned, JoinHandle, tokio, mpsc, Sender, Receiver } from '@ankurah/base';
import { Node, PeerSender, PolicyAgent, SendError, StorageEngine, WeakNode } from '@ankurah/core';
import { EntityId, NodeMessage, Presence } from '@ankurah/proto';

export class LocalProcessSender extends Struct implements PeerSender {
  sender: Sender<NodeMessage>;
  nodeId: EntityId;

  constructor(sender: Sender<NodeMessage>, nodeId: EntityId) {
    super();
    this.sender = sender;
    this.nodeId = nodeId;
  }

  sendMessage(message: NodeMessage): Result<void, SendError> {
    const _r0 = this.sender.trySend(message).mapErr((_) => {
      try {
        return new SendError('ConnectionClosed', {});
      } finally {
        dropOwned(_);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    return Result.Ok([]);
  }

  recipientNodeId(): EntityId {
    return this.nodeId;
  }

  cloned(): PeerSender {
    return this.clone();
  }

  clone(): LocalProcessSender {
    return new LocalProcessSender(this.sender.clone(), this.nodeId.clone());
  }
}

export class LocalProcessConnection<SE1 extends StorageEngine, PA1 extends PolicyAgent, SE2 extends StorageEngine, PA2 extends PolicyAgent> extends Drop {
  receiver1Task: JoinHandle<void>;
  receiver2Task: JoinHandle<void>;
  node1: WeakNode<SE1, PA1>;
  node2: WeakNode<SE2, PA2>;
  node1Id: EntityId;
  node2Id: EntityId;

  constructor(receiver1Task: JoinHandle<void>, receiver2Task: JoinHandle<void>, node1: WeakNode<SE1, PA1>, node2: WeakNode<SE2, PA2>, node1Id: EntityId, node2Id: EntityId) {
    super();
    this.receiver1Task = receiver1Task;
    this.receiver2Task = receiver2Task;
    this.node1 = node1;
    this.node2 = node2;
    this.node1Id = node1Id;
    this.node2Id = node2Id;
  }

  static async new<SE1, PA1, SE2, PA2>(node1: Node<SE1, PA1>, node2: Node<SE2, PA2>): Promise<Result<LocalProcessConnection<SE1, PA1, SE2, PA2>, Error>> {
    const [node1Tx, node1Rx] = mpsc.channel(1024);
    const [node2Tx, node2Rx] = mpsc.channel(1024);
    node1.registerPeer(new Presence(node2.deref().value.id, node2.deref().value.durable, node2.deref().value.system.root()), new LocalProcessSender(node2Tx, node2.deref().value.id));
    node2.registerPeer(new Presence(node1.deref().value.id, node1.deref().value.durable, node1.deref().value.system.root()), new LocalProcessSender(node1Tx, node1.deref().value.id));
    const receiver1Task = LocalProcessConnection.setupReceiver(node1.clone(), node1Rx);
    const receiver2Task = LocalProcessConnection.setupReceiver(node2.clone(), node2Rx);
    return Result.Ok(new LocalProcessConnection(receiver1Task, receiver2Task, node1.weak(), node2.weak(), node1.deref().value.id, node2.deref().value.id));
  }

  static setupReceiver<SE1, PA1, SE2, PA2, SE, PA>(node: Node<SE, PA>, rx: Receiver<NodeMessage>): JoinHandle<void> {
    try {
      try {
        return tokio.spawn((async () => {
          for (;;) {
            const _v = await rx.recv();
            if (!(_v != null)) {
              break;
            }
            const message = _v;
            let _moved0 = false;
            try {
              const node_1 = node.clone();
              try {
                tokio.spawn((async () => {
                  _moved0 = true;
                  const _ = await node_1.handleMessage(message);
                })());
              } finally {
                node_1.drop();
              }
            } finally {
              if (!_moved0) message.drop();
            }
          }
        })());
      } finally {
        rx.drop();
      }
    } finally {
      node.drop();
    }
  }

  protected override onDrop(): void {
    this.receiver1Task.abort();
    this.receiver2Task.abort();
    {
      const _v = this.node1.upgrade();
      if (_v != null) {
        const node1 = _v;
        try {
          node1.deregisterPeer(this.node2Id);
        } finally {
          node1.drop();
        }
      }
    }
    {
      const _v1 = this.node2.upgrade();
      if (_v1 != null) {
        const node2 = _v1;
        try {
          node2.deregisterPeer(this.node1Id);
        } finally {
          node2.drop();
        }
      }
    }
  }
}

