// MIRRORS: ankurah/core/src/node.rs
import { Struct, Drop, Result, Arc, Weak, AnyhowError, dropOwned, OwnershipFatal, UnsupportedShape, tracing, dropUnbound, debugString, HashMap, oneshot, spawn } from '@ankurah/base';
import { Attested, CollectionId, EntityState, Clock, DeltaContent, EntityDelta, EntityId, Event, NodeMessage, NodeRequest, NodeRequestBody, NodeResponse, NodeResponseBody, NodeUpdate, NodeUpdateAck, NodeUpdateAckBody, NodeUpdateBody, Presence, QueryId, RequestId, StateFragment, TransactionId, UpdateId } from '@ankurah/proto';
import { EntityChange } from './changes';
import { CollectionSet } from './collectionset';
import { PeerSender, SendError } from './connector';
import { Context } from './context';
import { Entity, WeakEntitySet } from './entity';
import { MutationError, RequestError, RetrievalError } from './error';
import { Comparison, EventAccumulator } from './lineage';
import { WeakEntityLiveQuery } from './livequery';
import { NodeApplier } from './node_applier';
import { SubscriptionRelay } from './peer_subscription/client_relay';
import { SubscriptionHandler } from './peer_subscription/server';
import { Reactor } from './reactor';
import { LocalRetriever } from './retrieval';
import { StorageCollectionWrapper } from './storage';
import { SystemManager } from './system';
import { spawn } from './task';
import { TypeResolver } from './type_resolver';
import { expandStates } from './util/expand_states';
import { Iterable_dispatch_iterable } from './util/iterable';
import { SafeMap } from './util/safemap';
import { SafeSet } from './util/safeset';
import { ParseError, Predicate, Selection, parseSelection } from '@ankurah/ankql';

export class PeerState extends Struct {
  sender: PeerSender;
  _durable: boolean;
  subscriptionHandler: SubscriptionHandler;
  pendingRequests: SafeMap<RequestId, oneshot.Sender<Result<NodeResponseBody, RequestError>>>;
  pendingUpdates: SafeMap<UpdateId, oneshot.Sender<Result<NodeResponseBody, RequestError>>>;

  constructor(sender: PeerSender, _durable: boolean, subscriptionHandler: SubscriptionHandler, pendingRequests: SafeMap<RequestId, oneshot.Sender<Result<NodeResponseBody, RequestError>>>, pendingUpdates: SafeMap<UpdateId, oneshot.Sender<Result<NodeResponseBody, RequestError>>>) {
    super();
    this.sender = sender;
    this._durable = _durable;
    this.subscriptionHandler = subscriptionHandler;
    this.pendingRequests = pendingRequests;
    this.pendingUpdates = pendingUpdates;
  }

  sendMessage(message: NodeMessage): Result<void, SendError> {
    return this.sender.sendMessage(message);
  }
}

export class MatchArgs extends Struct {
  readonly selection: Selection;
  readonly cached: boolean;

  constructor(selection: Selection, cached: boolean) {
    super();
    this.selection = selection;
    this.cached = cached;
  }

  static nocache<T>(s: T, _convT: (value: T) => Result<Selection, ParseError>): Result<MatchArgs, ParseError> {
    const _r0 = _convT(s);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    return Result.Ok(new MatchArgs(_r0.unwrap(), false));
  }

  static fromPredicate(val: Predicate): MatchArgs {
    return new MatchArgs(new Selection(val, null, null), true);
  }

  static fromSelection(val: Selection): MatchArgs {
    return new MatchArgs(val, true);
  }
}

export class Node<SE extends StorageEngine, PA extends PolicyAgent> extends Struct implements TNodeErased<Entity> {
  _0: Arc<NodeInner<SE, PA>>;

  constructor(_0: Arc<NodeInner<SE, PA>>) {
    super();
    this._0 = _0;
  }

  static new<SE, PA>(engine: Arc<SE>, policyAgent: PA): Node<SE, PA> {
    try {
      let _moved0 = false;
      const collections = CollectionSet.new(engine.clone());
      try {
        let _moved1 = false;
        const entityset = Default.default();
        try {
          const id = EntityId.new();
          let _moved2 = false;
          const reactor = Reactor.new();
          try {
            undefined /* notice_info!("Node {id:#} created as ephemeral") */;
            let _moved4 = false;
            const _b3 = collections.clone();
            try {
              let _moved6 = false;
              const _b5 = entityset.clone();
              try {
                const _b7 = reactor.clone();
                _moved4 = true;
                _moved6 = true;
                let _moved8 = false;
                const systemManager = SystemManager.new(_b3, _b5, _b7, false);
                try {
                  const subscriptionRelay = SubscriptionRelay.new();
                  let _moved10 = false;
                  const _b9 = SafeMap.new();
                  try {
                    let _moved12 = false;
                    const _b11 = SafeSet.new();
                    try {
                      let _moved14 = false;
                      const _b13 = SafeMap.new();
                      try {
                        const _b15 = TypeResolver.new();
                        _moved10 = true;
                        _moved12 = true;
                        _moved14 = true;
                        _moved0 = true;
                        _moved1 = true;
                        _moved2 = true;
                        _moved8 = true;
                        let _moved16 = false;
                        const node = new Node(Arc.new(new NodeInner(id, false, collections, entityset, _b9, _b11, _b13, reactor, policyAgent, systemManager, subscriptionRelay, _b15)));
                        try {
                          {
                            const _v = node.deref().value.subscriptionRelay;
                            if (_v != null) {
                              const relay = _v;
                              const weakNode = node.weak();
                              let _c18;
                              const _t17 = relay.setNode(Arc.new(weakNode));
                              try {
                                _c18 = _t17.isErr();
                              } finally {
                                _t17.drop();
                              }
                              if (_c18) {
                                tracing.warn('Failed to set message sender for subscription relay');
                              }
                            }
                          }
                          _moved16 = true;
                          return node;
                        } finally {
                          if (!_moved16) node.drop();
                        }
                      } finally {
                        if (!_moved14) dropOwned(_b13);
                      }
                    } finally {
                      if (!_moved12) dropOwned(_b11);
                    }
                  } finally {
                    if (!_moved10) dropOwned(_b9);
                  }
                } finally {
                  if (!_moved8) systemManager.drop();
                }
              } finally {
                if (!_moved6) dropOwned(_b5);
              }
            } finally {
              if (!_moved4) dropOwned(_b3);
            }
          } finally {
            if (!_moved2) reactor.drop();
          }
        } finally {
          if (!_moved1) entityset.drop();
        }
      } finally {
        if (!_moved0) collections.drop();
      }
    } finally {
      engine.drop();
    }
  }

  static newDurable<SE, PA>(engine: Arc<SE>, policyAgent: PA): Node<SE, PA> {
    let _moved0 = false;
    const collections = CollectionSet.new(engine);
    try {
      let _moved1 = false;
      const entityset = Default.default();
      try {
        const id = EntityId.new();
        let _moved2 = false;
        const reactor = Reactor.new();
        try {
          undefined /* notice_info!("Node {id:#} created as durable") */;
          let _moved4 = false;
          const _b3 = collections.clone();
          try {
            let _moved6 = false;
            const _b5 = entityset.clone();
            try {
              const _b7 = reactor.clone();
              _moved4 = true;
              _moved6 = true;
              let _moved8 = false;
              const systemManager = SystemManager.new(_b3, _b5, _b7, true);
              try {
                let _moved10 = false;
                const _b9 = SafeMap.new();
                try {
                  let _moved12 = false;
                  const _b11 = SafeSet.new();
                  try {
                    let _moved14 = false;
                    const _b13 = SafeMap.new();
                    try {
                      const _b15 = TypeResolver.new();
                      _moved10 = true;
                      _moved12 = true;
                      _moved14 = true;
                      _moved0 = true;
                      _moved1 = true;
                      _moved2 = true;
                      _moved8 = true;
                      return new Node(Arc.new(new NodeInner(id, true, collections, entityset, _b9, _b11, _b13, reactor, policyAgent, systemManager, null, _b15)));
                    } finally {
                      if (!_moved14) dropOwned(_b13);
                    }
                  } finally {
                    if (!_moved12) dropOwned(_b11);
                  }
                } finally {
                  if (!_moved10) dropOwned(_b9);
                }
              } finally {
                if (!_moved8) systemManager.drop();
              }
            } finally {
              if (!_moved6) dropOwned(_b5);
            }
          } finally {
            if (!_moved4) dropOwned(_b3);
          }
        } finally {
          if (!_moved2) reactor.drop();
        }
      } finally {
        if (!_moved1) entityset.drop();
      }
    } finally {
      if (!_moved0) collections.drop();
    }
  }

  weak(): WeakNode<SE, PA> {
    return new WeakNode(this._0.downgrade());
  }

  registerPeer(presence: Presence, sender: PeerSender): void {
    let _moved0 = false;
    try {
      try {
        undefined /* action_info!(self , "register_peer" , "{}" , & presence) */;
        let _moved1 = false;
        const subscriptionHandler = SubscriptionHandler.new(presence.nodeId, this);
        try {
          let _moved3 = false;
          const _b2 = SafeMap.new();
          try {
            const _b4 = SafeMap.new();
            _moved3 = true;
            _moved0 = true;
            _moved1 = true;
            this.deref().value.peerConnections.insert(presence.nodeId, Arc.new(new PeerState(sender, presence.durable, subscriptionHandler, _b2, _b4)));
          } finally {
            if (!_moved3) dropOwned(_b2);
          }
          if (presence.durable) {
            this.deref().value.durablePeers.insert(presence.nodeId);
            {
              const _v = this.deref().value.subscriptionRelay;
              if (_v != null) {
                const relay = _v;
                relay.notifyPeerConnected(presence.nodeId);
              }
            }
            if (!this.deref().value.durable) {
              {
                const _v2 = presence.systemRoot;
                if (_v2 != null) {
                  const systemRoot = _v2;
                  let _moved5 = false;
                  try {
                    undefined /* action_info!(self , "received system root" , "{}" , & system_root . payload) */;
                    const me = this.clone();
                    try {
                      spawn((async () => {
                        _moved5 = true;
                        {
                          const _v1 = await me.deref().value.system.joinSystem(systemRoot);
                          if (_v1.isErr()) {
                            const e = _v1.unwrapErr();
                            try {
                              undefined /* action_error!(me , "failed to join system" , "{}" , & e) */;
                            } finally {
                              e.drop();
                            }
                          } else {
                          _v1.drop();
                          undefined /* action_info!(me , "successfully joined system") */;
                        }
                        }
                      })());
                    } finally {
                      me.drop();
                    }
                  } finally {
                    if (!_moved5) systemRoot.drop();
                  }
                } else {
                tracing.error(`Node(${this.deref().value.id}) durable peer ${presence.nodeId} has no system root`);
              }
              }
            }
          }
        } finally {
          if (!_moved1) subscriptionHandler.drop();
        }
      } finally {
        if (!_moved0) dropOwned(sender);
      }
    } finally {
      presence.drop();
    }
  }

  deregisterPeer(nodeId: EntityId): void {
    undefined /* notice_info!("Node({:#}) deregister_peer {:#}" , self . id , node_id) */;
    this.deref().value.durablePeers.remove(nodeId);
    {
      const _v = this.deref().value.peerConnections.remove(nodeId);
      if (_v != null) {
        const peerState = _v;
        try {
          undefined /* action_info!(self , "unsubscribing" , "subscription {} for peer {}" , peer_state . subscription_handler . subscription_id () , node_id) */;
        } finally {
          peerState.drop();
        }
      }
    }
    {
      const _v1 = this.deref().value.subscriptionRelay;
      if (_v1 != null) {
        const relay = _v1;
        relay.notifyPeerDisconnected(nodeId);
      }
    }
  }

  async request<C>(nodeId: EntityId, cdata: C, requestBody: NodeRequestBody): Promise<Result<NodeResponseBody, RequestError>> {
    let _moved0 = false;
    try {
      const [responseTx, responseRx] = oneshot.channel();
      let _moved1 = false;
      const requestId = RequestId.new();
      try {
        let _moved3 = false;
        const _b2 = requestId.clone();
        try {
          const _b4 = this.deref().value.id;
          _moved3 = true;
          _moved0 = true;
          let _moved5 = false;
          const request = new NodeRequest(_b2, nodeId, _b4, requestBody);
          try {
            const _r6 = this.deref().value.policyAgent.signRequest(this, cdata, request);
            if (_r6.isErr()) return Result.Err(RequestError.fromAccessDenied(_r6.unwrapErr()));
            let _moved7 = false;
            const auth = _r6.unwrap();
            try {
              const _m8 = this.deref().value.peerConnections.get(nodeId);
              const _m9 = new RequestError('PeerNotConnected', {});
              const _r10 = (_m8 != null ? (_m9.drop(), Result.Ok(_m8!)) : Result.Err(_m9));
              if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
              const connection = _r10.unwrap();
              try {
                _moved1 = true;
                connection.value.pendingRequests.insert(requestId, responseTx);
                _moved7 = true;
                _moved5 = true;
                const _r11 = connection.value.sendMessage(new NodeMessage('Request', { auth: auth, request: request }));
                if (_r11.isErr()) return Result.Err(RequestError.fromSendError(_r11.unwrapErr()));
                _r11.drop();
                const _r12 = (await responseRx).mapErr((_) => new RequestError('InternalChannelClosed', {}));
                if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                return _r12.unwrap();
              } finally {
                connection.drop();
              }
            } finally {
              if (!_moved7) dropOwned(auth);
            }
          } finally {
            if (!_moved5) request.drop();
          }
        } finally {
          if (!_moved3) dropOwned(_b2);
        }
      } finally {
        if (!_moved1) requestId.drop();
      }
    } finally {
      if (!_moved0) requestBody.drop();
    }
  }

  sendUpdate(nodeId: EntityId, notification: NodeUpdateBody): void {
    let _moved0 = false;
    try {
      tracing.debug(`${this}.send_update(${nodeId}, ${notification})`);
      const [responseTx, _responseRx] = oneshot.channel();
      let _moved1 = false;
      const id = UpdateId.new();
      try {
        const _v = this.deref().value.peerConnections.get(nodeId);
        if (!(_v != null)) {
          tracing.warn(`Failed to send update to peer ${nodeId}: ${new RequestError('PeerNotConnected', {})}`);
          return;
        }
        const connection = _v;
        connection.value.pendingUpdates.insert(id.clone(), responseTx);
        const _b2 = this.deref().value.id;
        _moved1 = true;
        _moved0 = true;
        let _moved3 = false;
        const notification_1 = new NodeMessage('Update', { _0: new NodeUpdate(id, _b2, nodeId, notification) });
        try {
          _moved3 = true;
          const _v1 = connection.value.sendMessage(notification_1);
          if (_v1.isOk()) {
            const _v2 = _v1.unwrap();

          } else {
            const e = _v1.unwrapErr();
            try {
              tracing.warn(`Failed to send update to peer ${nodeId}: ${e}`);
            } finally {
              e.drop();
            }
          };
        } finally {
          if (!_moved3) notification_1.drop();
        }
      } finally {
        if (!_moved1) id.drop();
      }
    } finally {
      if (!_moved0) notification.drop();
    }
  }

  async handleMessage(message: NodeMessage): Promise<Result<void, Error>> {
    const _m17 = await (message.intoMatch<any>({
      Update: async (v) => {
        const update = v._0;
        let _moved0 = false;
        try {
          tracing.debug(`Node(${this.deref().value.id}) received update ${update}`);
          const _m1 = this.deref().value.peerConnections.get(update.from);
          {
            const _v4 = (_m1 != null ? ((c) => {
            try {
              return c.value.sender.cloned();
            } finally {
              c.drop();
            }
          })(_m1!) : null);
            if (_v4 != null) {
              const sender = _v4;
              const _from = update.from;
              const _id = update.id.clone();
              try {
                if (!update.to.equals(this.deref().value.id)) {
                  tracing.warn(`${this.deref().value.id} received message from ${update.from} but is not the intended recipient`);
                  return { $jump: 'return', $value: Result.Ok([]) };
                }
                let _moved2 = false;
                const id = update.id.clone();
                try {
                  const to = update.from;
                  const from = this.deref().value.id;
                  _moved0 = true;
                  let _moved3 = false;
                  const body = await (async () => {
                    const _v2 = await this.handleUpdate(update);
                    if (_v2.isOk()) {
                      const _v3 = _v2.unwrap();
                      return new NodeUpdateAckBody('Success', {});
                    } else {
                      const e = _v2.unwrapErr();
                      return new NodeUpdateAckBody('Error', { _0: e.toString() });
                    }
                  })();
                  try {
                    _moved2 = true;
                    _moved3 = true;
                    const _r4 = sender.sendMessage(new NodeMessage('UpdateAck', { _0: new NodeUpdateAck(id, from, to, body) }));
                    if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(_r4.unwrapErr()) };
                    _r4.drop();
                  } finally {
                    if (!_moved3) body.drop();
                  }
                } finally {
                  if (!_moved2) id.drop();
                }
              } finally {
                _id.drop();
              }
            }
          }
        } finally {
          if (!_moved0) update.drop();
        }
      },
      UpdateAck: async (v) => {
        const ack = v._0;
        try {
          tracing.debug(`Node(${this.deref().value.id}) received ack notification ${ack.id} ${ack.body}`);
        } finally {
          ack.drop();
        }
      },
      Request: async (v) => {
        const auth = v.auth;
        const request = v.request;
        let _moved5 = false;
        try {
          try {
            tracing.debug(`Node(${this.deref().value.id}) received request ${request}`);
            const _m6 = this.deref().value.peerConnections.get(request.from);
            {
              const _v9 = (_m6 != null ? ((c) => {
              try {
                return c.value.sender.cloned();
              } finally {
                c.drop();
              }
            })(_m6!) : null);
              if (_v9 != null) {
                const sender = _v9;
                const from = request.from;
                let _moved7 = false;
                const requestId = request.id.clone();
                try {
                  if (!request.to.equals(this.deref().value.id)) {
                    tracing.warn(`${this.deref().value.id} received message from ${request.from} but is not the intended recipient`);
                    return { $jump: 'return', $value: Result.Ok([]) };
                  }
                  let _moved8 = false;
                  const body = await (async () => {
                    const _v7 = await this.deref().value.policyAgent.checkRequest(this, auth, request);
                    if (_v7.isOk()) {
                      const cdata = _v7.unwrap();
                      _moved5 = true;
                      const _v8 = await this.handleRequest(cdata, request);
                      if (_v8.isOk()) {
                        const result = _v8.unwrap();
                        return result;
                      } else {
                        const e = _v8.unwrapErr();
                        return new NodeResponseBody('Error', { _0: e.toString() });
                      }
                    } else {
                      const e = _v7.unwrapErr();
                      try {
                        return new NodeResponseBody('Error', { _0: e.toString() });
                      } finally {
                        e.drop();
                      }
                    }
                  })();
                  try {
                    const _b9 = this.deref().value.id;
                    _moved7 = true;
                    _moved8 = true;
                    const _result = sender.sendMessage(new NodeMessage('Response', { _0: new NodeResponse(requestId, _b9, from, body) }));
                  } finally {
                    if (!_moved8) body.drop();
                  }
                } finally {
                  if (!_moved7) requestId.drop();
                }
              }
            }
          } finally {
            if (!_moved5) request.drop();
          }
        } finally {
          dropOwned(auth);
        }
      },
      Response: async (v) => {
        const response = v._0;
        try {
          tracing.debug(`Node ${this.deref().value.id} received response ${response}`);
          const _m10 = this.deref().value.peerConnections.get(response.from);
          const _m11 = new RequestError('PeerNotConnected', {});
          const _r12 = (_m10 != null ? (_m11.drop(), Result.Ok(_m10!)) : Result.Err(_m11));
          if (_r12.isErr()) return { $jump: 'return', $value: Result.Err(_r12.unwrapErr()) };
          const connection = _r12.unwrap();
          try {
            {
              const _v10 = connection.value.pendingRequests.remove(response.requestId);
              if (_v10 != null) {
                const tx = _v10;
                let _moved13 = false;
                try {
                  _moved13 = true;
                  const _b14 = Result.Ok(response.takeField('body'));
                  const _r15 = tx.send(_b14).mapErr((e) => {
                    try {
                      return AnyhowError.msg(`Failed to send response: ${e}`);
                    } finally {
                      e.drop();
                    }
                  });
                  if (_r15.isErr()) return { $jump: 'return', $value: Result.Err(_r15.unwrapErr()) };
                  _r15.drop();
                } finally {
                  if (!_moved13) tx.drop();
                }
              }
            }
          } finally {
            connection.drop();
          }
        } finally {
          response.drop();
        }
      },
      UnsubscribeQuery: async (v) => {
        const from = v.from;
        const queryId = v.queryId;
        {
          const _v11 = this.deref().value.peerConnections.get(from);
          if (_v11 != null) {
            const peerState = _v11;
            try {
              const _r16 = peerState.value.subscriptionHandler.removePredicate(queryId);
              if (_r16.isErr()) return { $jump: 'return', $value: Result.Err(_r16.unwrapErr()) };
              _r16.drop();
            } finally {
              peerState.drop();
            }
          }
        }
      },
    }));
    if ((_m17 as any)?.$jump === 'return') return (_m17 as any).$value;
    return Result.Ok([]);
  }

  async handleRequest<C>(cdata: C, request: NodeRequest): Promise<Result<NodeResponseBody, Error>> {
    try {
      return await (request.takeField('body').intoMatch({
        CommitTransaction: async (v) => {
          const id = v.id;
          const events = v.events;
          let _moved0 = false;
          let _moved1 = false;
          try {
            try {
              const _r2 = Iterable_dispatch_iterable(cdata).exactlyOne().mapErr((_) => AnyhowError.msg('Only one cdata is permitted for CommitTransaction'));
              if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
              const cdata_1 = _r2.unwrap();
              const _b3 = id.clone();
              _moved1 = true;
              const _v = await this.commitRemoteTransaction(cdata_1, _b3, events);
              if (_v.isOk()) {
                const _v1 = _v.unwrap();
                _moved0 = true;
                return Result.Ok(new NodeResponseBody('CommitComplete', { id: id }));
              } else {
                const e = _v.unwrapErr();
                try {
                  return Result.Ok(new NodeResponseBody('Error', { _0: e.toString() }));
                } finally {
                  e.drop();
                }
              }
            } finally {
              if (!_moved1) dropOwned(events);
            }
          } finally {
            if (!_moved0) id.drop();
          }
        },
        Fetch: async (v) => {
          const collection = v.collection;
          const selection = v.selection;
          const knownMatches = v.knownMatches;
          let _moved4 = false;
          try {
            try {
              try {
                const _r5 = this.deref().value.policyAgent.canAccessCollection(cdata, collection);
                if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
                _r5.drop();
                const _r6 = await this.deref().value.collections.get(collection);
                if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
                const storageCollection = _r6.unwrap();
                try {
                  const _r8 = this.deref().value.policyAgent.filterPredicate(cdata, collection, selection.takeField('predicate'));
                  if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                  const _a7 = _r8.unwrap();
                  selection.predicate.drop();
                  selection.predicate = _a7;
                  const _r9 = await storageCollection.deref().value.fetchStates(selection);
                  if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                  let _moved11 = false;
                  const _b10 = _r9.unwrap();
                  try {
                    const _b12 = [...knownMatches].map((k) => k.entityId);
                    const _r13 = await expandStates(_b10, _b12, storageCollection);
                    if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
                    _moved11 = true;
                    let _moved14 = false;
                    const expandedStates = _r13.unwrap();
                    try {
                      _moved4 = true;
                      const knownMap = HashMap.from([...knownMatches].map((k) => {
                        try {
                          return [k.entityId, k.takeField('head')];
                        } finally {
                          k.drop();
                        }
                      }));
                      let _moved15 = false;
                      let deltas = [];
                      try {
                        _moved14 = true;
                        const _seq20 = expandedStates;
                        let _at21 = 0;
                        try {
                          while (_at21 < _seq20.length) {
                            const state = _seq20[_at21++];
                            let _moved16 = false;
                            try {
                              let _c18;
                              const _t17 = this.deref().value.policyAgent.checkRead(cdata, state.payload.entityId, collection, state.payload.state);
                              try {
                                _c18 = _t17.isErr();
                              } finally {
                                _t17.drop();
                              }
                              if (_c18) {
                                continue;
                              }
                              _moved16 = true;
                              const _r19 = await this.generateEntityDelta(knownMap, state, storageCollection);
                              if (_r19.isErr()) return Result.Err(_r19.unwrapErr());
                              {
                                const _v2 = _r19.unwrap();
                                if (_v2 != null) {
                                  const delta = _v2;
                                  deltas.push(delta);
                                }
                              }
                            } finally {
                              if (!_moved16) state.drop();
                            }
                          }
                        } finally {
                          dropOwned(_seq20.slice(_at21));
                        }
                        _moved15 = true;
                        return Result.Ok(new NodeResponseBody('Fetch', { _0: deltas }));
                      } finally {
                        if (!_moved15) dropOwned(deltas);
                      }
                    } finally {
                      if (!_moved14) dropOwned(expandedStates);
                    }
                  } finally {
                    if (!_moved11) dropOwned(_b10);
                  }
                } finally {
                  storageCollection.drop();
                }
              } finally {
                if (!_moved4) dropOwned(knownMatches);
              }
            } finally {
              selection.drop();
            }
          } finally {
            collection.drop();
          }
        },
        Get: async (v) => {
          const collection = v.collection;
          const ids = v.ids;
          try {
            const _r22 = this.deref().value.policyAgent.canAccessCollection(cdata, collection);
            if (_r22.isErr()) return Result.Err(_r22.unwrapErr());
            _r22.drop();
            const _r23 = await this.deref().value.collections.get(collection);
            if (_r23.isErr()) return Result.Err(_r23.unwrapErr());
            const storageCollection = _r23.unwrap();
            try {
              let _moved24 = false;
              let states = [];
              try {
                const _r25 = await storageCollection.deref().value.getStates(ids);
                if (_r25.isErr()) return Result.Err(_r25.unwrapErr());
                const _seq28 = _r25.unwrap();
                let _at29 = 0;
                try {
                  while (_at29 < _seq28.length) {
                    const state = _seq28[_at29++];
                    let _moved26 = false;
                    try {
                      const _v3 = this.deref().value.policyAgent.checkRead(cdata, state.payload.entityId, collection, state.payload.state);
                      if (_v3.isOk()) {
                        const _v4 = _v3.unwrap();
                        _moved26 = true;
                        states.push(state)
                      } else {
                        const _v5 = _v3.unwrapErr();
                        _arm27: {
                          if (_v5.is('ByPolicy')) {
                            const _v6 = _v5;
                            _v6.drop();
                            break _arm27;
                          }
                          {
                            const e = _v5;
                            try {
                              return Result.Err(AnyhowError.msg(`Error from peer get: ${e}`));
                            } finally {
                              e.drop();
                            }
                          }
                        }
                      }
                    } finally {
                      if (!_moved26) state.drop();
                    }
                  }
                } finally {
                  dropOwned(_seq28.slice(_at29));
                }
                _moved24 = true;
                return Result.Ok(new NodeResponseBody('Get', { _0: states }));
              } finally {
                if (!_moved24) dropOwned(states);
              }
            } finally {
              storageCollection.drop();
            }
          } finally {
            collection.drop();
          }
        },
        GetEvents: async (v) => {
          const collection = v.collection;
          const eventIds = v.eventIds;
          let _moved30 = false;
          try {
            try {
              const _r31 = this.deref().value.policyAgent.canAccessCollection(cdata, collection);
              if (_r31.isErr()) return Result.Err(_r31.unwrapErr());
              _r31.drop();
              const _r32 = await this.deref().value.collections.get(collection);
              if (_r32.isErr()) return Result.Err(_r32.unwrapErr());
              const storageCollection = _r32.unwrap();
              try {
                let _moved33 = false;
                let events = [];
                try {
                  _moved30 = true;
                  const _r34 = await storageCollection.deref().value.getEvents(eventIds);
                  if (_r34.isErr()) return Result.Err(_r34.unwrapErr());
                  const _seq37 = _r34.unwrap();
                  let _at38 = 0;
                  try {
                    while (_at38 < _seq37.length) {
                      const event = _seq37[_at38++];
                      let _moved35 = false;
                      try {
                        const _v7 = this.deref().value.policyAgent.checkReadEvent(cdata, event);
                        if (_v7.isOk()) {
                          const _v8 = _v7.unwrap();
                          _moved35 = true;
                          events.push(event)
                        } else {
                          const _v9 = _v7.unwrapErr();
                          _arm36: {
                            if (_v9.is('ByPolicy')) {
                              const _v10 = _v9;
                              _v10.drop();
                              break _arm36;
                            }
                            {
                              const e = _v9;
                              try {
                                return Result.Err(AnyhowError.msg(`Error from peer subscription: ${e}`));
                              } finally {
                                e.drop();
                              }
                            }
                          }
                        }
                      } finally {
                        if (!_moved35) event.drop();
                      }
                    }
                  } finally {
                    dropOwned(_seq37.slice(_at38));
                  }
                  _moved33 = true;
                  return Result.Ok(new NodeResponseBody('GetEvents', { _0: events }));
                } finally {
                  if (!_moved33) dropOwned(events);
                }
              } finally {
                storageCollection.drop();
              }
            } finally {
              if (!_moved30) dropOwned(eventIds);
            }
          } finally {
            collection.drop();
          }
        },
        SubscribeQuery: async (v) => {
          const queryId = v.queryId;
          const collection = v.collection;
          const selection = v.selection;
          const version = v.version;
          const knownMatches = v.knownMatches;
          let _moved39 = false;
          let _moved40 = false;
          let _moved41 = false;
          try {
            try {
              try {
                const _m42 = this.deref().value.peerConnections.get(request.from);
                const _r43 = (_m42 != null ? Result.Ok(_m42!) : Result.Err((() => AnyhowError.msg(`Peer ${request.from} not connected`))()));
                if (_r43.isErr()) return Result.Err(_r43.unwrapErr());
                const peerState = _r43.unwrap();
                try {
                  const _r44 = Iterable_dispatch_iterable(cdata).exactlyOne().mapErr((_) => AnyhowError.msg('Only one cdata is permitted for SubscribePredicate'));
                  if (_r44.isErr()) return Result.Err(_r44.unwrapErr());
                  const cdata_1 = _r44.unwrap();
                  _moved39 = true;
                  _moved40 = true;
                  _moved41 = true;
                  return await peerState.value.subscriptionHandler.subscribeQuery(this, queryId, collection, selection, cdata_1, version, knownMatches);
                } finally {
                  peerState.drop();
                }
              } finally {
                if (!_moved41) dropOwned(knownMatches);
              }
            } finally {
              if (!_moved40) selection.drop();
            }
          } finally {
            if (!_moved39) collection.drop();
          }
        },
      }));
    } finally {
      request.drop();
    }
  }

  async handleUpdate(notification: NodeUpdate): Promise<Result<void, Error>> {
    try {
      const _v = this.deref().value.peerConnections.get(notification.from);
      if (!(_v != null)) {
        return Result.Err(AnyhowError.msg(`Rejected notification from unknown node ${notification.from}`));
      }
      const _connection = _v;
      return await (notification.takeField('body').intoMatch({
        SubscriptionUpdate: async (v) => {
          const items = v.items;
          let _moved0 = false;
          try {
            tracing.debug(`Node(${this.deref().value.id}) received subscription update from peer ${notification.from}`);
            _moved0 = true;
            const _r1 = await NodeApplier.applyUpdates(this, notification.from, items);
            if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
            _r1.drop();
            return Result.Ok([]);
          } finally {
            if (!_moved0) dropOwned(items);
          }
        },
      }));
    } finally {
      notification.drop();
    }
  }

  async relayToRequiredPeers(cdata: ContextData, id: TransactionId, events: Attested<Event>[]): Promise<Result<void, MutationError>> {
    try {
      for (const peerId of this.getDurablePeers()) {
        const _v = await this.request(peerId, cdata, new NodeRequestBody('CommitTransaction', { id: id.clone(), events: events.map((e) => e.clone()) }));
        if (_v.isOk()) {
          const _v1 = _v.unwrap();
          _arm0: {
            if (_v1.is('CommitComplete')) {
              const _v2 = _v1;
              try {
                []
              } finally {
                _v2.drop();
              }
              break _arm0;
            }
            if (_v1.is('Error')) {
              const _v3 = _v1;
              const { _0: e } = _v1.value;
              try {
                return Result.Err(new MutationError('General', { _0: io.Error.other(`Peer ${peerId} rejected: ${e}`) }));
              } finally {
                _v3.drop();
              }
            }
            {
              const _v4 = _v1;
              try {
                return Result.Err(new MutationError('General', { _0: io.Error.other(`Peer ${peerId} returned unexpected response`) }));
              } finally {
                _v4.drop();
              }
            }
          }
        } else {
          const _v5 = _v.unwrapErr();
          try {
            return Result.Err(new MutationError('General', { _0: io.Error.other(`Peer ${peerId} returned unexpected response`) }));
          } finally {
            _v5.drop();
          }
        }
      }
      return Result.Ok([]);
    } finally {
      id.drop();
    }
  }

  async commitRemoteTransaction(cdata: ContextData, id: TransactionId, events: Attested<Event>[]): Promise<Result<void, MutationError>> {
    try {
      try {
        tracing.debug(`${this} commiting transaction ${id} with ${events.length} events`);
        let _moved0 = false;
        let changes = [];
        try {
          for (const event of [...events]) {
            const _r1 = await this.deref().value.collections.get(event.payload.collection);
            if (_r1.isErr()) return Result.Err(MutationError.fromRetrievalError(_r1.unwrapErr()));
            const collection = _r1.unwrap();
            try {
              const retriever = LocalRetriever.new(collection.clone());
              try {
                const _r2 = await this.deref().value.entities.getRetrieveOrCreate(retriever, event.payload.collection, event.payload.entityId);
                if (_r2.isErr()) return Result.Err(MutationError.fromRetrievalError(_r2.unwrapErr()));
                const entity = _r2.unwrap();
                try {
                  const _m11 = await (async () => {
                    if (event.payload.isEntityCreate() && (() => {
                      const _t3 = entity.head();
                      try {
                        return _t3.isEmpty();
                      } finally {
                        _t3.drop();
                      }
                    })()) {
                      const _r4 = await entity.applyEvent(retriever, event.payload);
                      if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(_r4.unwrapErr()) };
                      _r4.drop();
                      let _moved6 = false;
                      const _b5 = entity.clone();
                      try {
                        const _b7 = entity.clone();
                        _moved6 = true;
                        return [_b5, _b7, true];
                      } finally {
                        if (!_moved6) dropOwned(_b5);
                      }
                    } else {
                      const trxAlive = Arc.new(true);
                      let _moved8 = false;
                      const forked = entity.snapshot(trxAlive);
                      try {
                        const _r9 = await forked.applyEvent(retriever, event.payload);
                        if (_r9.isErr()) return { $jump: 'return', $value: Result.Err(_r9.unwrapErr()) };
                        _r9.drop();
                        const _b10 = entity.clone();
                        _moved8 = true;
                        return [_b10, forked, false];
                      } finally {
                        if (!_moved8) forked.drop();
                      }
                    }
                  })();
                  if ((_m11 as any)?.$jump === 'return') return (_m11 as any).$value;
                  const [entityBefore, entityAfter, alreadyApplied] = (_m11 as any);
                  const _r12 = this.deref().value.policyAgent.checkEvent(this, cdata, entityBefore, entityAfter, event.payload);
                  if (_r12.isErr()) return Result.Err(MutationError.fromAccessDenied(_r12.unwrapErr()));
                  {
                    const _v = _r12.unwrap();
                    if (_v != null) {
                      const attestation = _v;
                      event.attestations.push(attestation);
                    }
                  }
                  const _m14 = await (async () => {
                    if (alreadyApplied) {
                      return true;
                    } else {
                      const _r13 = await entity.applyEvent(retriever, event.payload);
                      if (_r13.isErr()) return { $jump: 'return', $value: Result.Err(_r13.unwrapErr()) };
                      return _r13.unwrap();
                    }
                  })();
                  if ((_m14 as any)?.$jump === 'return') return (_m14 as any).$value;
                  const applied = (_m14 as any);
                  if (applied) {
                    const _r15 = entity.toState();
                    if (_r15.isErr()) return Result.Err(MutationError.fromStateError(_r15.unwrapErr()));
                    let _moved16 = false;
                    const state = _r15.unwrap();
                    try {
                      const _b17 = entity.id();
                      const _b18 = entity.collection().clone();
                      _moved16 = true;
                      let _moved19 = false;
                      const entityState = new EntityState(_b17, _b18, state);
                      try {
                        let _moved20 = false;
                        const attestation = this.deref().value.policyAgent.attestState(this, entityState);
                        try {
                          _moved19 = true;
                          _moved20 = true;
                          let _moved21 = false;
                          const attested = Attested.opt(entityState, attestation);
                          try {
                            const _r22 = await collection.deref().value.addEvent(event);
                            if (_r22.isErr()) return Result.Err(_r22.unwrapErr());
                            _r22.drop();
                            _moved21 = true;
                            const _r23 = await collection.deref().value.setState(attested);
                            if (_r23.isErr()) return Result.Err(_r23.unwrapErr());
                            _r23.drop();
                            let _moved25 = false;
                            const _b24 = entity.clone();
                            try {
                              const _b26 = [event.clone()];
                              const _r27 = EntityChange.new(_b24, _b26);
                              if (_r27.isErr()) return Result.Err(_r27.unwrapErr());
                              _moved25 = true;
                              changes.push(_r27.unwrap());
                            } finally {
                              if (!_moved25) dropOwned(_b24);
                            }
                          } finally {
                            if (!_moved21) attested.drop();
                          }
                        } finally {
                          if (!_moved20) dropOwned(attestation);
                        }
                      } finally {
                        if (!_moved19) entityState.drop();
                      }
                    } finally {
                      if (!_moved16) state.drop();
                    }
                  }
                } finally {
                  entity.drop();
                }
              } finally {
                retriever.drop();
              }
            } finally {
              collection.drop();
            }
          }
          _moved0 = true;
          await this.deref().value.reactor.notifyChange(changes);
          return Result.Ok([]);
        } finally {
          if (!_moved0) dropOwned(changes);
        }
      } finally {
        dropOwned(events);
      }
    } finally {
      id.drop();
    }
  }

  async generateEntityDelta(knownMap: HashMap<EntityId, Clock>, entityState: Attested<EntityState>, storageCollection: StorageCollectionWrapper): Promise<Result<EntityDelta | null, Error>> {
    const { payload: { entityId, collection, state }, attestations } = entityState;
    const currentHead = state.head;
    {
      const _v4 = knownMap.get(entityId);
      if (_v4 != null) {
        const knownHead = _v4;
        if (knownHead.equals(currentHead)) {
          return Result.Ok(null);
        }
        const _v = await this.collectEventBridge(storageCollection, knownHead, currentHead);
        if (_v.isOk()) {
          const _v1 = _v.unwrap();
          {
            const attestedEvents = _v1;
            let _g1;
            try {
              _g1 = !(attestedEvents.length === 0);
            } catch (_e) {
              if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
              dropOwned(attestedEvents);
              throw _e;
            }
            if (_g1) {
              let _moved0 = false;
              try {
                {
                  _moved0 = true;
                  const eventFragments = [...attestedEvents].map((e) => e);
                  return Result.Ok(new EntityDelta(entityId, collection, new DeltaContent('EventBridge', { events: eventFragments })));
                }
              } finally {
                if (!_moved0) dropOwned(attestedEvents);
              }
            }
          }
          {
            const _v2 = _v1;
            try {
              {
              }
            } finally {
              dropOwned(_v2);
            }
          }
        } else {
          const _v3 = _v.unwrapErr();
          {
          }
        }
      }
    }
    let _moved2 = false;
    const stateFragment = new StateFragment(state, attestations);
    try {
      _moved2 = true;
      return Result.Ok(new EntityDelta(entityId, collection, new DeltaContent('StateSnapshot', { state: stateFragment })));
    } finally {
      if (!_moved2) stateFragment.drop();
    }
  }

  async collectEventBridge(storageCollection: StorageCollectionWrapper, knownHead: Clock, currentHead: Clock): Promise<Result<Attested<Event>[], Error>> {
    const retriever = LocalRetriever.new(storageCollection.clone());
    try {
      const accumulator = EventAccumulator.new(null);
      let _moved0 = false;
      let comparison = Comparison.newWithAccumulator(retriever, currentHead, knownHead, 100000, accumulator);
      try {
        while (true) {
          const _r1 = await comparison.step();
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          const _v = _r1.unwrap();
          if (_v != null && (_v.is('Descends'))) {
            break;
          } else if (_v != null && (_v.is('Equal'))) {
            break;
          } else if (_v != null) {
            return Result.Ok([]);
          } else {

          }
        }
        _moved0 = true;
        return Result.Ok((comparison.takeAccumulatedEvents() ?? []));
      } finally {
        if (!_moved0) comparison.drop();
      }
    } finally {
      retriever.drop();
    }
  }

  nextEntityId(): EntityId {
    return EntityId.new();
  }

  context(data: ContextData): Result<Context, AnyhowError> {
    if (!this.deref().value.system.isSystemReady()) {
      return Result.Err(AnyhowError.msg('System is not ready'));
    }
    return Result.Ok(Context.new(Node.clone(this), data));
  }

  async contextAsync(data: ContextData): Promise<Context> {
    await this.deref().value.system.waitSystemReady();
    return Context.new(Node.clone(this), data);
  }

  async getFromPeer(collectionId: CollectionId, ids: EntityId[], cdata: ContextData): Promise<Result<void, RetrievalError>> {
    const _m0 = this.getDurablePeerRandom();
    const _m1 = new RetrievalError('NoDurablePeers', {});
    const _r2 = (_m0 != null ? (_m1.drop(), Result.Ok(_m0!)) : Result.Err(_m1));
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    const peerId = _r2.unwrap();
    const _r3 = (await this.request(peerId, cdata, new NodeRequestBody('Get', { collection: collectionId.clone(), ids: ids }))).mapErr((e) => {
      try {
        return new RetrievalError('Other', { _0: `${e.debug()}` });
      } finally {
        e.drop();
      }
    });
    if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
    return await (_r3.unwrap().intoMatch({
      Get: async (v) => {
        const states = v._0;
        let _moved4 = false;
        try {
          const _r5 = await this.deref().value.collections.get(collectionId);
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          const collection = _r5.unwrap();
          try {
            _moved4 = true;
            const _seq9 = states;
            let _at10 = 0;
            try {
              while (_at10 < _seq9.length) {
                const state = _seq9[_at10++];
                let _moved6 = false;
                try {
                  const _r7 = this.deref().value.policyAgent.validateReceivedState(this, peerId, state);
                  if (_r7.isErr()) return Result.Err(RetrievalError.fromAccessDenied(_r7.unwrapErr()));
                  _r7.drop();
                  _moved6 = true;
                  const _r8 = (await collection.deref().value.setState(state)).mapErr((e) => {
                    try {
                      return new RetrievalError('Other', { _0: `${e.debug()}` });
                    } finally {
                      e.drop();
                    }
                  });
                  if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                  _r8.drop();
                } finally {
                  if (!_moved6) state.drop();
                }
              }
            } finally {
              dropOwned(_seq9.slice(_at10));
            }
            return Result.Ok([]);
          } finally {
            collection.drop();
          }
        } finally {
          if (!_moved4) dropOwned(states);
        }
      },
      Error: async (v) => {
        const e = v._0;
        tracing.debug(`Error from peer fetch: ${e}`);
        return Result.Err(new RetrievalError('Other', { _0: `${debugString(e)}` }));
      },
      CommitComplete: (v) => {
        try {
          tracing.debug('Unexpected response type from peer get');
          return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
        } finally {
          dropUnbound(v, []);
        }
      },
      Fetch: (v) => {
        try {
          tracing.debug('Unexpected response type from peer get');
          return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
        } finally {
          dropUnbound(v, []);
        }
      },
      GetEvents: (v) => {
        try {
          tracing.debug('Unexpected response type from peer get');
          return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
        } finally {
          dropUnbound(v, []);
        }
      },
      QuerySubscribed: (v) => {
        try {
          tracing.debug('Unexpected response type from peer get');
          return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
        } finally {
          dropUnbound(v, []);
        }
      },
      Success: () => {
        tracing.debug('Unexpected response type from peer get');
        return Result.Err(new RetrievalError('Other', { _0: 'Unexpected response type' }));
      },
    }));
  }

  getDurablePeerRandom(): EntityId | null {
    let rng = rand.threadRng();
    const peers = this.deref().value.durablePeers.toVec();
    return peers.choose(rng).copied();
  }

  getDurablePeers(): EntityId[] {
    return this.deref().value.durablePeers.toVec();
  }

  subscribeRemoteQuery(queryId: QueryId, collectionId: CollectionId, selection: Selection, cdata: ContextData, version: number, livequery: WeakEntityLiveQuery): void {
    let _moved0 = false;
    let _moved1 = false;
    let _moved2 = false;
    try {
      try {
        try {
          {
            const _v = this.deref().value.subscriptionRelay;
            if (_v != null) {
              const relay = _v;
              _moved1 = true;
              let _moved3 = false;
              const selection_1 = this.deref().value.typeResolver.resolveSelectionTypes(selection);
              try {
                this.deref().value.predicateContext.insert(queryId, cdata.clone());
                _moved0 = true;
                _moved3 = true;
                _moved2 = true;
                relay.subscribeQuery(queryId, collectionId, selection_1, cdata, version, livequery);
              } finally {
                if (!_moved3) selection_1.drop();
              }
            }
          }
        } finally {
          if (!_moved2) livequery.drop();
        }
      } finally {
        if (!_moved1) selection.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  async fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Result<Entity[], RetrievalError>> {
    const _r0 = await this.deref().value.collections.get(collectionId);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const storageCollection = _r0.unwrap();
    try {
      const _r2 = await storageCollection.deref().value.fetchStates(selection);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      let _moved3 = false;
      const initialStates = _r2.unwrap();
      try {
        _moved1 = true;
        const retriever = LocalRetriever.new(storageCollection);
        try {
          let _moved4 = false;
          let entities = [];
          try {
            _moved3 = true;
            const _seq9 = initialStates;
            let _at10 = 0;
            try {
              while (_at10 < _seq9.length) {
                const state = _seq9[_at10++];
                try {
                  let _moved6 = false;
                  const _b5 = collectionId.clone();
                  try {
                    const _b7 = state.payload.takeField('state');
                    const _r8 = await this.deref().value.entities.withState(retriever, state.payload.entityId, _b5, _b7);
                    if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                    _moved6 = true;
                    const [, entity] = _r8.unwrap();
                    entities.push(entity);
                  } finally {
                    if (!_moved6) dropOwned(_b5);
                  }
                } finally {
                  state.drop();
                }
              }
            } finally {
              dropOwned(_seq9.slice(_at10));
            }
            _moved4 = true;
            return Result.Ok(entities);
          } finally {
            if (!_moved4) dropOwned(entities);
          }
        } finally {
          retriever.drop();
        }
      } finally {
        if (!_moved3) dropOwned(initialStates);
      }
    } finally {
      if (!_moved1) storageCollection.drop();
    }
  }

  clone(): Node<SE, PA> {
    return new Node(this._0.clone());
  }

  deref(): Arc<NodeInner<SE, PA>> {
    return this._0;
  }

  unsubscribeRemotePredicate(queryId: QueryId): void {
    this.deref().value.predicateContext.remove(queryId);
    {
      const _v = this.deref().value.subscriptionRelay;
      if (_v != null) {
        const relay = _v;
        relay.unsubscribePredicate(queryId);
      }
    }
  }

  updateRemoteQuery(queryId: QueryId, selection: Selection, version: number): Result<void, AnyhowError> {
    let _moved0 = false;
    try {
      {
        const _v = this.deref().value.subscriptionRelay;
        if (_v != null) {
          const relay = _v;
          _moved0 = true;
          const selection_1 = this.deref().value.typeResolver.resolveSelectionTypes(selection);
          _moved0 = true;
          const _r1 = relay.updateQuery(queryId, selection_1, version);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          _r1.drop();
        }
      }
      return Result.Ok([]);
    } finally {
      if (!_moved0) selection.drop();
    }
  }

  reactor(): Reactor<Entity, Attested<Event>> {
    return this._0.value.reactor;
  }

  hasSubscriptionRelay(): boolean {
    return (this.deref().value.subscriptionRelay != null);
  }

  toString(): string {
    return `\u{1b}[1;34mnode\u{1b}[2m[\u{1b}[1;34m${this.deref().value.id.toBase64Short()}\u{1b}[2m]\u{1b}[0m`;
  }
}

export class WeakNode<SE, PA extends PolicyAgent> extends Struct {
  _0: Weak<NodeInner<SE, PA>>;

  constructor(_0: Weak<NodeInner<SE, PA>>) {
    super();
    this._0 = _0;
  }

  upgrade(): Node<SE, PA> | null {
    const _m0 = this._0.upgrade();
    return (_m0 != null ? (Node)(_m0!) : null);
  }

  clone(): WeakNode<SE, PA> {
    return new WeakNode(this._0.clone());
  }
}

export class NodeInner<SE extends StorageEngine, PA extends PolicyAgent> extends Drop {
  readonly id: EntityId;
  readonly durable: boolean;
  readonly collections: CollectionSet<SE>;
  entities: WeakEntitySet;
  peerConnections: SafeMap<EntityId, Arc<PeerState>>;
  durablePeers: SafeSet<EntityId>;
  predicateContext: SafeMap<QueryId, ContextData>;
  reactor: Reactor<Entity, Attested<Event>>;
  policyAgent: PA;
  readonly system: SystemManager<SE, PA>;
  subscriptionRelay: SubscriptionRelay<ContextData, WeakEntityLiveQuery> | null;
  typeResolver: TypeResolver;

  constructor(id: EntityId, durable: boolean, collections: CollectionSet<SE>, entities: WeakEntitySet, peerConnections: SafeMap<EntityId, Arc<PeerState>>, durablePeers: SafeSet<EntityId>, predicateContext: SafeMap<QueryId, ContextData>, reactor: Reactor<Entity, Attested<Event>>, policyAgent: PA, system: SystemManager<SE, PA>, subscriptionRelay: SubscriptionRelay<ContextData, WeakEntityLiveQuery> | null, typeResolver: TypeResolver) {
    super();
    this.id = id;
    this.durable = durable;
    this.collections = collections;
    this.entities = entities;
    this.peerConnections = peerConnections;
    this.durablePeers = durablePeers;
    this.predicateContext = predicateContext;
    this.reactor = reactor;
    this.policyAgent = policyAgent;
    this.system = system;
    this.subscriptionRelay = subscriptionRelay;
    this.typeResolver = typeResolver;
  }

  async requestRemoteUnsubscribe(queryId: QueryId, peers: EntityId[]): Promise<Result<void, Error>> {
    const _seq1 = this.peerConnections.getList(peers);
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const [peerId, item] = _seq1[_at2++];
        {
          const _v = item;
          if (_v != null) {
            const connection = _v;
            try {
              const _r0 = connection.value.sendMessage(new NodeMessage('UnsubscribeQuery', { from: peerId, queryId: queryId }));
              if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
              _r0.drop();
            } finally {
              connection.drop();
            }
          } else {
          tracing.warn(`Peer ${peerId} not connected`);
        }
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    return Result.Ok([]);
  }

  protected override onDrop(): void {
    undefined /* notice_info!("Node({}) dropped" , self . id) */;
  }
}

export interface ContextData {
}

export interface TNodeErased<E extends AbstractEntity & Filterable = Entity> {
  unsubscribeRemotePredicate(queryId: QueryId): void;
  updateRemoteQuery(queryId: QueryId, selection: Selection, version: number): Result<void, Error>;
  fetchEntitiesFromLocal(collectionId: CollectionId, selection: Selection): Promise<Result<E[], RetrievalError>>;
  reactor(): Reactor<E>;
  hasSubscriptionRelay(): boolean;
}

export function nocache<T extends TryInto>(s: T, _convT: (value: T) => Result<Selection, ParseError>): Result<MatchArgs, ParseError> {
  return MatchArgs.nocache(s, _convT);
}

export function Str_tryInto(self: string): Result<MatchArgs, ParseError> {
  const _r0 = parseSelection(self);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(new MatchArgs(_r0.unwrap(), true));
}

export function String_tryInto(self: string): Result<MatchArgs, ParseError> {
  const _r0 = parseSelection(self);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(new MatchArgs(_r0.unwrap(), true));
}

export function RetrievalError_fromParseError(e: ParseError): RetrievalError {
  return new RetrievalError('ParseError', { _0: e });
}

