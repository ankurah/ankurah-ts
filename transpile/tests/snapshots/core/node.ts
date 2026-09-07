// MIRRORS: ankurah/core/src/node.rs
import { Struct, Drop, Result, Arc, Weak, AnyhowError, dropOwned, OwnershipFatal, UnsupportedShape, tracing, dropUnbound, unsupported, debugString, HashMap, oneshot, spawn } from '@ankurah/base';
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
      const collections = CollectionSet.new(engine.clone());
      let _moved0 = false;
      const entityset = Default.default();
      try {
        const id = EntityId.new();
        let _moved1 = false;
        const reactor = Reactor.new();
        try {
          undefined /* notice_info!("Node {id:#} created as ephemeral") */;
          const systemManager = SystemManager.new(collections.clone(), entityset.clone(), reactor.clone(), false);
          const subscriptionRelay = SubscriptionRelay.new();
          const _b2 = SafeMap.new();
          const _b3 = SafeSet.new();
          const _b4 = SafeMap.new();
          const _b5 = TypeResolver.new();
          _moved0 = true;
          _moved1 = true;
          const node = new Node(Arc.new(new NodeInner(id, false, collections, entityset, _b2, _b3, _b4, reactor, policyAgent, systemManager, subscriptionRelay, _b5)));
          {
            const _v = node.subscriptionRelay;
            if (_v != null) {
              const relay = _v;
              const weakNode = node.weak();
              if (relay.setNode(Arc.new(weakNode)).isErr()) {
                tracing.warn('Failed to set message sender for subscription relay');
              }
            }
          }
          return node;
        } finally {
          if (!_moved1) reactor.drop();
        }
      } finally {
        if (!_moved0) entityset.drop();
      }
    } finally {
      engine.drop();
    }
  }

  static newDurable<SE, PA>(engine: Arc<SE>, policyAgent: PA): Node<SE, PA> {
    const collections = CollectionSet.new(engine);
    let _moved0 = false;
    const entityset = Default.default();
    try {
      const id = EntityId.new();
      let _moved1 = false;
      const reactor = Reactor.new();
      try {
        undefined /* notice_info!("Node {id:#} created as durable") */;
        const systemManager = SystemManager.new(collections.clone(), entityset.clone(), reactor.clone(), true);
        const _b2 = SafeMap.new();
        const _b3 = SafeSet.new();
        const _b4 = SafeMap.new();
        const _b5 = TypeResolver.new();
        _moved0 = true;
        _moved1 = true;
        return new Node(Arc.new(new NodeInner(id, true, collections, entityset, _b2, _b3, _b4, reactor, policyAgent, systemManager, null, _b5)));
      } finally {
        if (!_moved1) reactor.drop();
      }
    } finally {
      if (!_moved0) entityset.drop();
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
          const _b2 = SafeMap.new();
          const _b3 = SafeMap.new();
          _moved0 = true;
          _moved1 = true;
          this.deref().value.peerConnections.insert(presence.nodeId, Arc.new(new PeerState(sender, presence.durable, subscriptionHandler, _b2, _b3)));
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
                  let _moved4 = false;
                  try {
                    undefined /* action_info!(self , "received system root" , "{}" , & system_root . payload) */;
                    const me = this.clone();
                    try {
                      spawn((async () => {
                        _moved4 = true;
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
                    if (!_moved4) systemRoot.drop();
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
    const [responseTx, responseRx] = oneshot.channel();
    let _moved0 = false;
    const requestId = RequestId.new();
    try {
      let _moved1 = false;
      const request = new NodeRequest(requestId.clone(), nodeId, this.deref().value.id, requestBody);
      try {
        const _r2 = this.deref().value.policyAgent.signRequest(this, cdata, request);
        if (_r2.isErr()) return Result.Err(RequestError.fromAccessDenied(_r2.unwrapErr()));
        let _moved3 = false;
        const auth = _r2.unwrap();
        try {
          const _m4 = this.deref().value.peerConnections.get(nodeId);
          const _m5 = new RequestError('PeerNotConnected', {});
          const _r6 = (_m4 != null ? (_m5.drop(), Result.Ok(_m4!)) : Result.Err(_m5));
          if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
          const connection = _r6.unwrap();
          try {
            _moved0 = true;
            connection.value.pendingRequests.insert(requestId, responseTx);
            _moved3 = true;
            _moved1 = true;
            const _r7 = connection.value.sendMessage(new NodeMessage('Request', { auth: auth, request: request }));
            if (_r7.isErr()) return Result.Err(RequestError.fromSendError(_r7.unwrapErr()));
            _r7.drop();
            const _r8 = (await responseRx).mapErr((_) => new RequestError('InternalChannelClosed', {}));
            if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
            return _r8.unwrap();
          } finally {
            connection.drop();
          }
        } finally {
          if (!_moved3) dropOwned(auth);
        }
      } finally {
        if (!_moved1) request.drop();
      }
    } finally {
      if (!_moved0) requestId.drop();
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
              const _r2 = Iterable_dispatch_iterable(cdata).exactlyOne().mapErr((_) => {
                try {
                  return AnyhowError.msg('Only one cdata is permitted for CommitTransaction');
                } finally {
                  dropOwned(_);
                }
              });
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
                  const _r10 = await expandStates(_r9.unwrap(), [...knownMatches].map((k) => k.entityId), storageCollection);
                  if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                  let _moved11 = false;
                  const expandedStates = _r10.unwrap();
                  try {
                    _moved4 = true;
                    const knownMap = HashMap.from([...knownMatches].map((k) => {
                      try {
                        return [k.entityId, k.takeField('head')];
                      } finally {
                        k.drop();
                      }
                    }));
                    let deltas = [];
                    _moved11 = true;
                    const _seq16 = expandedStates;
                    let _at17 = 0;
                    try {
                      while (_at17 < _seq16.length) {
                        const state = _seq16[_at17++];
                        let _moved12 = false;
                        try {
                          let _c14;
                          const _t13 = this.deref().value.policyAgent.checkRead(cdata, state.payload.entityId, collection, state.payload.state);
                          try {
                            _c14 = _t13.isErr();
                          } finally {
                            _t13.drop();
                          }
                          if (_c14) {
                            continue;
                          }
                          _moved12 = true;
                          const _r15 = await this.generateEntityDelta(knownMap, state, storageCollection);
                          if (_r15.isErr()) return Result.Err(_r15.unwrapErr());
                          {
                            const _v2 = _r15.unwrap();
                            if (_v2 != null) {
                              const delta = _v2;
                              deltas.push(delta);
                            }
                          }
                        } finally {
                          if (!_moved12) state.drop();
                        }
                      }
                    } finally {
                      dropOwned(_seq16.slice(_at17));
                    }
                    return Result.Ok(new NodeResponseBody('Fetch', { _0: deltas }));
                  } finally {
                    if (!_moved11) dropOwned(expandedStates);
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
            const _r18 = this.deref().value.policyAgent.canAccessCollection(cdata, collection);
            if (_r18.isErr()) return Result.Err(_r18.unwrapErr());
            _r18.drop();
            const _r19 = await this.deref().value.collections.get(collection);
            if (_r19.isErr()) return Result.Err(_r19.unwrapErr());
            const storageCollection = _r19.unwrap();
            try {
              let states = [];
              const _r20 = await storageCollection.deref().value.getStates(ids);
              if (_r20.isErr()) return Result.Err(_r20.unwrapErr());
              const _seq23 = _r20.unwrap();
              let _at24 = 0;
              try {
                while (_at24 < _seq23.length) {
                  const state = _seq23[_at24++];
                  let _moved21 = false;
                  try {
                    const _v3 = this.deref().value.policyAgent.checkRead(cdata, state.payload.entityId, collection, state.payload.state);
                    if (_v3.isOk()) {
                      const _v4 = _v3.unwrap();
                      _moved21 = true;
                      states.push(state)
                    } else {
                      const _v5 = _v3.unwrapErr();
                      _arm22: {
                        if (_v5.is('ByPolicy')) {
                          const _v6 = _v5;
                          _v6.drop();
                          break _arm22;
                        }
                        {
                          const e = _v5;
                          try {
                            return Result.Err(AnyhowError.msg(`Error from peer get: ${e}`))
                          } finally {
                            e.drop();
                          }
                        }
                      }
                    }
                  } finally {
                    if (!_moved21) state.drop();
                  }
                }
              } finally {
                dropOwned(_seq23.slice(_at24));
              }
              return Result.Ok(new NodeResponseBody('Get', { _0: states }));
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
          let _moved25 = false;
          try {
            try {
              const _r26 = this.deref().value.policyAgent.canAccessCollection(cdata, collection);
              if (_r26.isErr()) return Result.Err(_r26.unwrapErr());
              _r26.drop();
              const _r27 = await this.deref().value.collections.get(collection);
              if (_r27.isErr()) return Result.Err(_r27.unwrapErr());
              const storageCollection = _r27.unwrap();
              try {
                let events = [];
                _moved25 = true;
                const _r28 = await storageCollection.deref().value.getEvents(eventIds);
                if (_r28.isErr()) return Result.Err(_r28.unwrapErr());
                const _seq31 = _r28.unwrap();
                let _at32 = 0;
                try {
                  while (_at32 < _seq31.length) {
                    const event = _seq31[_at32++];
                    let _moved29 = false;
                    try {
                      const _v7 = this.deref().value.policyAgent.checkReadEvent(cdata, event);
                      if (_v7.isOk()) {
                        const _v8 = _v7.unwrap();
                        _moved29 = true;
                        events.push(event)
                      } else {
                        const _v9 = _v7.unwrapErr();
                        _arm30: {
                          if (_v9.is('ByPolicy')) {
                            const _v10 = _v9;
                            _v10.drop();
                            break _arm30;
                          }
                          {
                            const e = _v9;
                            try {
                              return Result.Err(AnyhowError.msg(`Error from peer subscription: ${e}`))
                            } finally {
                              e.drop();
                            }
                          }
                        }
                      }
                    } finally {
                      if (!_moved29) event.drop();
                    }
                  }
                } finally {
                  dropOwned(_seq31.slice(_at32));
                }
                return Result.Ok(new NodeResponseBody('GetEvents', { _0: events }));
              } finally {
                storageCollection.drop();
              }
            } finally {
              if (!_moved25) dropOwned(eventIds);
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
          let _moved33 = false;
          let _moved34 = false;
          let _moved35 = false;
          try {
            try {
              try {
                const _m36 = this.deref().value.peerConnections.get(request.from);
                const _r37 = (_m36 != null ? Result.Ok(_m36!) : Result.Err((() => AnyhowError.msg(`Peer ${request.from} not connected`))()));
                if (_r37.isErr()) return Result.Err(_r37.unwrapErr());
                const peerState = _r37.unwrap();
                try {
                  const _r38 = Iterable_dispatch_iterable(cdata).exactlyOne().mapErr((_) => {
                    try {
                      return AnyhowError.msg('Only one cdata is permitted for SubscribePredicate');
                    } finally {
                      dropOwned(_);
                    }
                  });
                  if (_r38.isErr()) return Result.Err(_r38.unwrapErr());
                  const cdata_1 = _r38.unwrap();
                  _moved33 = true;
                  _moved34 = true;
                  _moved35 = true;
                  return await peerState.value.subscriptionHandler.subscribeQuery(this, queryId, collection, selection, cdata_1, version, knownMatches);
                } finally {
                  peerState.drop();
                }
              } finally {
                if (!_moved35) dropOwned(knownMatches);
              }
            } finally {
              if (!_moved34) selection.drop();
            }
          } finally {
            if (!_moved33) collection.drop();
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
        let changes = [];
        for (const event of [...events]) {
          const _r0 = await this.deref().value.collections.get(event.payload.collection);
          if (_r0.isErr()) return Result.Err(MutationError.fromRetrievalError(_r0.unwrapErr()));
          const collection = _r0.unwrap();
          try {
            const retriever = LocalRetriever.new(collection.clone());
            try {
              const _r1 = await this.deref().value.entities.getRetrieveOrCreate(retriever, event.payload.collection, event.payload.entityId);
              if (_r1.isErr()) return Result.Err(MutationError.fromRetrievalError(_r1.unwrapErr()));
              const entity = _r1.unwrap();
              try {
                const _m6 = await (async () => {
                  if (event.payload.isEntityCreate() && (() => {
                    const _t2 = entity.head();
                    try {
                      return _t2.isEmpty();
                    } finally {
                      _t2.drop();
                    }
                  })()) {
                    const _r3 = await entity.applyEvent(retriever, event.payload);
                    if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
                    _r3.drop();
                    return [entity.clone(), entity.clone(), true];
                  } else {
                    const trxAlive = Arc.new(true);
                    let _moved4 = false;
                    const forked = entity.snapshot(trxAlive);
                    try {
                      const _r5 = await forked.applyEvent(retriever, event.payload);
                      if (_r5.isErr()) return { $jump: 'return', $value: Result.Err(_r5.unwrapErr()) };
                      _r5.drop();
                      _moved4 = true;
                      return [entity.clone(), forked, false];
                    } finally {
                      if (!_moved4) forked.drop();
                    }
                  }
                })();
                if ((_m6 as any)?.$jump === 'return') return (_m6 as any).$value;
                const [entityBefore, entityAfter, alreadyApplied] = (_m6 as any);
                const _r7 = this.deref().value.policyAgent.checkEvent(this, cdata, entityBefore, entityAfter, event.payload);
                if (_r7.isErr()) return Result.Err(MutationError.fromAccessDenied(_r7.unwrapErr()));
                {
                  const _v = _r7.unwrap();
                  if (_v != null) {
                    const attestation = _v;
                    event.attestations.push(attestation);
                  }
                }
                const _m9 = await (async () => {
                  if (alreadyApplied) {
                    return true;
                  } else {
                    const _r8 = await entity.applyEvent(retriever, event.payload);
                    if (_r8.isErr()) return { $jump: 'return', $value: Result.Err(_r8.unwrapErr()) };
                    return _r8.unwrap();
                  }
                })();
                if ((_m9 as any)?.$jump === 'return') return (_m9 as any).$value;
                const applied = (_m9 as any);
                if (applied) {
                  const _r10 = entity.toState();
                  if (_r10.isErr()) return Result.Err(MutationError.fromStateError(_r10.unwrapErr()));
                  let _moved11 = false;
                  const state = _r10.unwrap();
                  try {
                    const _b12 = entity.id();
                    const _b13 = entity.collection().clone();
                    _moved11 = true;
                    let _moved14 = false;
                    const entityState = new EntityState(_b12, _b13, state);
                    try {
                      let _moved15 = false;
                      const attestation = this.deref().value.policyAgent.attestState(this, entityState);
                      try {
                        _moved14 = true;
                        _moved15 = true;
                        const attested = Attested.opt(entityState, attestation);
                        const _r16 = await collection.deref().value.addEvent(event);
                        if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
                        _r16.drop();
                        const _r17 = await collection.deref().value.setState(attested);
                        if (_r17.isErr()) return Result.Err(_r17.unwrapErr());
                        _r17.drop();
                        const _r18 = EntityChange.new(entity.clone(), [event.clone()]);
                        if (_r18.isErr()) return Result.Err(_r18.unwrapErr());
                        changes.push(_r18.unwrap());
                      } finally {
                        if (!_moved15) dropOwned(attestation);
                      }
                    } finally {
                      if (!_moved14) entityState.drop();
                    }
                  } finally {
                    if (!_moved11) state.drop();
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
        await this.deref().value.reactor.notifyChange(changes);
        return Result.Ok([]);
      } finally {
        dropOwned(events);
      }
    } finally {
      id.drop();
    }
  }

  async generateEntityDelta(knownMap: HashMap<EntityId, Clock>, entityState: Attested<EntityState>, storageCollection: StorageCollectionWrapper): Promise<Result<EntityDelta | null, Error>> {
    const { payload: { entityId, collection, state }, attestations } = entityState;
    let _moved0 = false;
    const currentHead = state.head;
    try {
      {
        const _v4 = knownMap.get(entityId);
        if (_v4 != null) {
          const knownHead = _v4;
          if (knownHead.equals(currentHead)) {
            return Result.Ok(null);
          }
          _moved0 = true;
          const _v = await this.collectEventBridge(storageCollection, knownHead, currentHead);
          if (_v.isOk()) {
            const _v1 = _v.unwrap();
            {
              const attestedEvents = _v1;
              let _g2;
              try {
                _g2 = !(attestedEvents.length === 0);
              } catch (_e) {
                if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;
                dropOwned(attestedEvents);
                throw _e;
              }
              if (_g2) {
                let _moved1 = false;
                try {
                  {
                    _moved1 = true;
                    const eventFragments = [...attestedEvents].map((e) => e);
                    return Result.Ok(new EntityDelta(entityId, collection, new DeltaContent('EventBridge', { events: eventFragments })));
                  }
                } finally {
                  if (!_moved1) dropOwned(attestedEvents);
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
      let _moved3 = false;
      const stateFragment = new StateFragment(state, attestations);
      try {
        _moved3 = true;
        return Result.Ok(new EntityDelta(entityId, collection, new DeltaContent('StateSnapshot', { state: stateFragment })));
      } finally {
        if (!_moved3) stateFragment.drop();
      }
    } finally {
      if (!_moved0) currentHead.drop();
    }
  }

  async collectEventBridge(storageCollection: StorageCollectionWrapper, knownHead: Clock, currentHead: Clock): Promise<Result<Attested<Event>[], Error>> {
    const retriever = LocalRetriever.new(storageCollection.clone());
    try {
      const accumulator = EventAccumulator.new(null);
      let comparison = Comparison.newWithAccumulator(retriever, currentHead, knownHead, 100000, accumulator);
      while (true) {
        const _r0 = await comparison.step();
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const _v = _r0.unwrap();
        if (_v != null && (_v.is('Descends'))) {
          break;
        } else if (_v != null && (_v.is('Equal'))) {
          break;
        } else if (_v != null) {
          return Result.Ok([]);
        } else {

        }
      }
      return Result.Ok(unsupported('`unwrap_or_default` answers the value or the type\'s `Default`, and the engine could not type this receiver, so it cannot say which default that is'));
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
              const selection_1 = this.deref().value.typeResolver.resolveSelectionTypes(selection);
              this.deref().value.predicateContext.insert(queryId, cdata.clone());
              _moved0 = true;
              _moved1 = true;
              _moved2 = true;
              relay.subscribeQuery(queryId, collectionId, selection_1, cdata, version, livequery);
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
          let entities = [];
          _moved3 = true;
          const _seq5 = initialStates;
          let _at6 = 0;
          try {
            while (_at6 < _seq5.length) {
              const state = _seq5[_at6++];
              try {
                const _r4 = await this.deref().value.entities.withState(retriever, state.payload.entityId, collectionId.clone(), state.payload.takeField('state'));
                if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
                const [, entity] = _r4.unwrap();
                entities.push(entity);
              } finally {
                state.drop();
              }
            }
          } finally {
            dropOwned(_seq5.slice(_at6));
          }
          return Result.Ok(entities);
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

