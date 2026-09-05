// MIRRORS: ankurah/core/src/peer_subscription/client_relay.rs
import { Struct, Enum, Result, Arc, Mutex, AnyhowError, dropOwned, tracing, HashMap, HashSet, tokio, select, spawn, Sender, Receiver } from '@ankurah/base';
import { CollectionId } from '@ankurah/proto';
import { SendError } from '../connector';
import { ApplyError, MutationError, RequestError, RetrievalError, StateError } from '../error';
import { ContextData, Node } from '../node';
import { NodeApplier } from '../node_applier';
import { AccessDenied } from '../policy';
import { PropertyError } from '../property/traits';
import { EphemeralNodeRetriever, GetEvents } from '../retrieval';
import { spawn } from '../task';
import { SafeSet } from '../util/safeset';
import { ParseError, Predicate, Selection } from '@ankurah/ankql';
import { CollectionId, DecodeError, EntityId, KnownEntity, NodeRequestBody, NodeResponseBody, QueryId } from '@ankurah/proto';
import { Get } from '@ankurah/signals';

export class Content<CD extends ContextData> extends Struct {
  readonly queryId: QueryId;
  readonly collectionId: CollectionId;
  readonly selection: Selection;
  readonly contextData: CD;
  readonly version: number;

  constructor(queryId: QueryId, collectionId: CollectionId, selection: Selection, contextData: CD, version: number) {
    super();
    this.queryId = queryId;
    this.collectionId = collectionId;
    this.selection = selection;
    this.contextData = contextData;
    this.version = version;
  }

  debug(): string {
    return `Content { queryId: ${this.queryId.debug()}, collectionId: ${this.collectionId.debug()}, selection: ${this.selection.debug()}, contextData: ${this.contextData}, version: ${String(this.version)} }`;
  }
}

export class RemoteQueryState<CD extends ContextData, Q extends RemoteQuerySubscriber> extends Struct {
  content: Arc<Content<CD>>;
  status: Status;
  readonly livequery: Q;

  constructor(content: Arc<Content<CD>>, status: Status, livequery: Q) {
    super();
    this.content = content;
    this.status = status;
    this.livequery = livequery;
  }
}

class SubscriptionRelayInner<CD extends ContextData, Q extends RemoteQuerySubscriber> extends Struct {
  subscriptions: Mutex<HashMap<QueryId, RemoteQueryState<CD, Q>>>;
  connectedPeers: SafeSet<EntityId>;
  node: OnceLock<Arc<TNode>>;
  _shutdownTx: Sender<void>;

  constructor(subscriptions: Mutex<HashMap<QueryId, RemoteQueryState<CD, Q>>>, connectedPeers: SafeSet<EntityId>, node: OnceLock<Arc<TNode>>, _shutdownTx: Sender<void>) {
    super();
    this.subscriptions = subscriptions;
    this.connectedPeers = connectedPeers;
    this.node = node;
    this._shutdownTx = _shutdownTx;
  }
}

export class SubscriptionRelay<CD extends ContextData, Q extends RemoteQuerySubscriber> extends Struct {
  inner: Arc<SubscriptionRelayInner<CD, Q>>;

  constructor(inner: Arc<SubscriptionRelayInner<CD, Q>>) {
    super();
    this.inner = inner;
  }

  static new<CD, Q>(): SubscriptionRelay<CD, Q> {
    const [shutdownTx, shutdownRx] = tokio.sync.mpsc.channel(1);
    const relay = new SubscriptionRelay(Arc.new(new SubscriptionRelayInner(new Mutex(new HashMap()), SafeSet.new(), OnceLock.new(), shutdownTx)));
    relay.startRetryTask(shutdownRx);
    return relay;
  }

  setNode(node: Arc<TNode>): Result<void, void> {
    return this.inner.value.node.set(node).mapErr((_) => []);
  }

  subscribeQuery(queryId: QueryId, collectionId: CollectionId, selection: Selection, contextData: CD, version: number, livequery: Q): void {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        tracing.debug(`SubscriptionRelay.subscribe_predicate() - New predicate ${queryId} needs remote registration`);
        (() => {
          _moved0 = true;
          _moved1 = true;
          const _t2 = this.inner.value.subscriptions.lock();
          try {
            _t2.value.set(queryId, new RemoteQueryState(Arc.new(new Content(queryId, collectionId, selection, contextData, version)), new Status('PendingRemote', {}), livequery));
          } finally {
            _t2.drop();
          }
        })()
        if (!this.inner.value.connectedPeers.isEmpty()) {
          this.setupRemoteSubscriptions();
        }
      } finally {
        if (!_moved1) selection.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  updateQuery(queryId: QueryId, selection: Selection, version: number): Result<void, AnyhowError> {
    let _moved0 = false;
    try {
      tracing.debug(`SubscriptionRelay.update_query() - New query ${queryId} needs remote registration`);
      const _m5 = (() => {
        {
          let subscriptions = this.inner.value.subscriptions.lock();
          try {
            const _v = subscriptions.value.get(queryId);
            if (_v != null) {
              const state = _v;
              {
                const oldContent = state.content;
                try {
                  const _a1 = Arc.new(new Content(oldContent.value.queryId, oldContent.value.collectionId.clone(), selection.clone(), oldContent.value.contextData.clone(), version));
                  state.content.drop();
                  state.content = _a1;
                  const _m3 = () => {
                    const _a2 = new Status('PendingRemote', {});
                    state.status.drop();
                    state.status = _a2;
                    return null;
                  };
                  return state.status.match<any>({
                    Established: (v) => {
                      const peerId = v._0;
                      const _oldVersion = v._1;
                      const _a4 = new Status('Requested', { _0: peerId, _1: version });
                      state.status.drop();
                      state.status = _a4;
                      return [peerId, state.content.value.collectionId.clone(), state.content.value.contextData.clone()];
                    },
                    PendingRemote: () => {
                      return _m3();
                    },
                    Requested: () => {
                      return _m3();
                    },
                    PendingUpdate: () => {
                      return _m3();
                    },
                    Failed: () => {
                      return _m3();
                    },
                  });
                } finally {
                  oldContent.drop();
                }
              }
            } else {
              return { $jump: 'return', $value: Result.Err(AnyhowError.msg(`Predicate ${queryId} not found`)) };
            }
          } finally {
            subscriptions.drop();
          }
        }
      })();
      if ((_m5 as any)?.$jump === 'return') return (_m5 as any).$value;
      const update = (_m5 as any);
      if (update != null) {
        const [peerId, collectionId, contextData] = update;
        (() => {
          _moved0 = true;
          this.updateQueryOnPeer(peerId, queryId, collectionId, selection, version, contextData);
        })()
      } else {
        (() => {
          this.setupRemoteSubscriptions();
        })()
      };
      return Result.Ok([]);
    } finally {
      if (!_moved0) selection.drop();
    }
  }

  updateQueryOnPeer(peerId: EntityId, queryId: QueryId, collectionId: CollectionId, selection: Selection, version: number, contextData: CD): void {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        const me = this.clone();
        try {
          spawn((async () => {
            {
              const _v4 = me.inner.value.node.get();
              if (_v4 != null) {
                const node = _v4;
                const _t2 = me.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
                try {
                  const livequery = _t2.value.get(queryId) != null ? ((state) => state.livequery.clone())(_t2.value.get(queryId)!) : null;
                  _t2.drop();
                  _moved0 = true;
                  _moved1 = true;
                  const _v = await TNode_dispatch_remoteSubscribe(node.value, peerId, queryId, collectionId, selection, contextData, version);
                  if (_v.isOk()) {
                    const _v1 = _v.unwrap();
                    {
                      {
                        const _v2 = livequery;
                        if (_v2 != null) {
                          const lq = _v2;
                          await lq.subscriptionEstablished(version);
                        }
                      }
                      let subscriptions = me.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
                      try {
                        {
                          const _v3 = subscriptions.value.get(queryId);
                          if (_v3 != null) {
                            const info = _v3;
                            const _a3 = new Status('Established', { _0: peerId, _1: version });
                            info.status.drop();
                            info.status = _a3;
                          }
                        }
                        tracing.debug(`Successfully updated predicate ${queryId} on peer ${peerId} subscription`);
                      } finally {
                        subscriptions.drop();
                      }
                    }
                  } else {
                    const e = _v.unwrapErr();
                    let _moved4 = false;
                    try {
                      {
                        _moved4 = true;
                        await me.handleError(queryId, peerId, e, livequery);
                      }
                    } finally {
                      if (!_moved4) e.drop();
                    }
                  }
                } finally {
                  _t2.drop();
                }
              }
            }
          })());
        } finally {
          me.drop();
        }
      } finally {
        if (!_moved1) selection.drop();
      }
    } finally {
      if (!_moved0) collectionId.drop();
    }
  }

  unsubscribePredicate(queryId: QueryId): void {
    tracing.debug(`Unregistering predicate ${queryId}`);
    {
      let subscriptions = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
      try {
        {
          const _v3 = subscriptions.value.remove(queryId);
          if (_v3 != null) {
            const info = _v3;
            try {
              {
                const _v2 = info.status;
                if (_v2.is('Established')) {
                  const { _0: peerId, _1: _version } = _v2.value;
                  const node = this.inner.value.node.get();
                  {
                    const _v1 = node;
                    if (_v1 != null) {
                      const node = _v1;
                      const node_1 = node.clone();
                      try {
                        const peerId_1 = peerId;
                        spawn((async () => {
                          {
                            const _v = await TNode_dispatch_peerUnsubscribe(node_1.value, peerId_1, queryId);
                            if (_v.isErr()) {
                              const e = _v.unwrapErr();
                              tracing.warn(`Failed to send unsubscribe message for ${queryId}: ${e}`);
                            } else {
                            tracing.debug(`Successfully sent unsubscribe message for ${queryId}`);
                          }
                          }
                        })());
                      } finally {
                        node_1.drop();
                      }
                    }
                  }
                }
              }
            } finally {
              info.drop();
            }
          }
        }
      } finally {
        subscriptions.drop();
      }
    }
  }

  notifyPeerDisconnected(peerId: EntityId): void {
    tracing.debug(`Peer ${peerId} disconnected, orphaning predicate registrations`);
    this.inner.value.connectedPeers.remove(peerId);
    const _t0 = this.inner.value.subscriptions.lock();
    try {
      for (const info of _t0.value.values()) {
        {
          const _v = info.status;
          if ((_v.is('Established')) || (_v.is('Requested'))) {
            const { _0: establishedPeerId } = _v.value;
            if (establishedPeerId === peerId) {
              const _a1 = new Status('PendingRemote', {});
              info.status.drop();
              info.status = _a1;
              tracing.warn(`Predicate ${info.content.value.queryId} orphaned due to peer ${peerId} disconnect`);
            }
          }
        }
      }
    } finally {
      _t0.drop();
    }
    this.setupRemoteSubscriptions();
  }

  notifyPeerConnected(peerId: EntityId): void {
    tracing.debug(`SubscriptionRelay.notify_peer_connected() - Peer ${peerId} connected, registering predicates on peer subscription`);
    this.inner.value.connectedPeers.insert(peerId);
    this.setupRemoteSubscriptions();
  }

  getStatus(queryId: QueryId): Status | null {
    const subscriptions = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
    try {
      return subscriptions.value.get(queryId) != null ? ((info) => info.status.clone())(subscriptions.value.get(queryId)!) : null;
    } finally {
      subscriptions.drop();
    }
  }

  getContextsForPeer(peerId: EntityId): HashSet<CD> {
    const subscriptions = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
    try {
      let contexts = new HashSet();
      for (const [, state] of [...subscriptions.value]) {
        if ((state.status.is('Established')) || (state.status.is('Requested'))) {
          const { _0: establishedPeer } = state.status.value;
          if (establishedPeer.equals(peerId)) {
            contexts.insert(state.content.value.contextData.clone());
          }
        } else {
          {
          }
        }
      }
      return contexts;
    } finally {
      subscriptions.drop();
    }
  }

  setupRemoteSubscriptions(): void {
    const _m0 = (() => {
      const _v = this.inner.value.node.get();
      if (_v != null) {
        const node = _v;
        return node;
      } else {
        {
          tracing.warn('No node configured for remote subscription setup');
          return { $jump: 'return', $value: undefined };
        }
      }
    })();
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
    const node = (_m0 as any);
    const connectedPeers = this.inner.value.connectedPeers.toVec();
    if (connectedPeers.length === 0) {
      tracing.warn('No durable peers available for remote subscription setup');
      return;
    }
    const targetPeer = connectedPeers[0];
    const _t1 = this.inner.value.subscriptions.lock();
    try {
      const pending = _t1.value.values().filterMap((info) => {
        {
          const _v1 = info.status;
          if (_v1.is('PendingRemote')) {
            const _a2 = new Status('Requested', { _0: targetPeer, _1: info.content.value.version });
            info.status.drop();
            info.status = _a2;
            return info.content.clone();
          } else {
          return null;
        }
        }
      });
      _t1.drop();
      if (pending.length === 0) {
        return;
      }
      tracing.debug(`Registering ${pending.length} predicates on ${this.inner.value.connectedPeers.len()} peer subscriptions`);
      for (const content of pending) {
        spawn(this.clone().attemptSubscribe(node.clone(), targetPeer, content));
      }
    } finally {
      _t1.drop();
    }
  }

  async attemptSubscribe(node: Arc<TNode>, targetPeer: EntityId, content: Arc<Content<CD>>): Promise<void> {
    try {
      try {
        try {
          const queryId = content.value.queryId;
          const predicate = content.value.selection.clone();
          const contextData = content.value.contextData.clone();
          const version = content.value.version;
          const _t0 = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
          try {
            const livequery = _t0.value.get(queryId) != null ? ((state) => state.livequery.clone())(_t0.value.get(queryId)!) : null;
            _t0.drop();
            const _v = await TNode_dispatch_remoteSubscribe(node.value, targetPeer, queryId, content.value.collectionId.clone(), predicate, contextData, version);
            if (_v.isOk()) {
              const _v1 = _v.unwrap();
              {
                {
                  const _v2 = livequery;
                  if (_v2 != null) {
                    const lq = _v2;
                    await lq.subscriptionEstablished(version);
                  }
                }
                let subscriptions = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
                try {
                  {
                    const _v3 = subscriptions.value.get(queryId);
                    if (_v3 != null) {
                      const info = _v3;
                      const _a1 = new Status('Established', { _0: targetPeer, _1: version });
                      info.status.drop();
                      info.status = _a1;
                    }
                  }
                  tracing.debug(`Successfully registered predicate ${queryId} on peer ${targetPeer} subscription`);
                } finally {
                  subscriptions.drop();
                }
              }
            } else {
              const e = _v.unwrapErr();
              let _moved2 = false;
              try {
                {
                  _moved2 = true;
                  await this.handleError(queryId, targetPeer, e, livequery);
                }
              } finally {
                if (!_moved2) e.drop();
              }
            }
          } finally {
            _t0.drop();
          }
        } finally {
          content.drop();
        }
      } finally {
        node.drop();
      }
    } finally {
      this.drop();
    }
  }

  startRetryTask(shutdownRx: Receiver<void>): void {
    try {
      const me = this.clone();
      try {
        spawn((async () => {
          while (true) {
            const delay = futuresTimer.Delay.new(time.Duration.fromSecs(5n));
            const _v = [
              { tag: '_0', promise: delay },
              { tag: '_1', promise: shutdownRx.recv() },
            ];
            try {
              const _v1 = await select(_v);
              if (_v1.tag === '_0') {
                me.setupRemoteSubscriptions();
              } else if (_v1.tag === '_1') {
                tracing.debug('Retry task shutting down - SubscriptionRelay dropped');
                break;
              }
            } finally {
              for (const _v2 of _v) dropOwned(_v2.promise);
            }
          }
        })());
      } finally {
        me.drop();
      }
    } finally {
      shutdownRx.drop();
    }
  }

  async handleError(queryId: QueryId, targetPeer: EntityId, error: RetrievalError, livequery: Q | null): Promise<void> {
    let _moved0 = false;
    try {
      const errorMsg = error.toString();
      const isRetryable = (() => {
        return error.match({
          RequestError: (v) => {
            const reqErr = v._0;
            return reqErr.match({
              PeerNotConnected: () => true,
              ConnectionLost: () => true,
              SendError: (v) => true,
              InternalChannelClosed: () => true,
              ServerError: (v) => false,
              UnexpectedResponse: (v) => false,
              AccessDenied: (v) => false,
            });
          },
          AccessDenied: () => false,
          ParseError: () => false,
          EntityNotFound: () => false,
          EventNotFound: () => false,
          StorageError: () => false,
          CollectionNotFound: () => false,
          FailedUpdate: () => false,
          DeserializationError: () => false,
          NoDurablePeers: () => false,
          Other: () => false,
          InvalidBucketName: () => false,
          AnkqlFilter: () => false,
          FutureJoin: () => false,
          Anyhow: () => false,
          DecodeError: () => false,
          StateError: () => false,
          MutationError: () => false,
          PropertyError: () => false,
          ApplyError: () => false,
        });
      })();
      let subscriptions = this.inner.value.subscriptions.lock().unwrapOrElse((e) => e.intoInner());
      try {
        {
          const _v1 = subscriptions.value.get(queryId);
          if (_v1 != null) {
            const info = _v1;
            if (isRetryable) {
              const _a1 = new Status('PendingRemote', {});
              info.status.drop();
              info.status = _a1;
              tracing.warn(`Retryable failure for predicate ${queryId} with peer ${targetPeer}: ${errorMsg} - will retry`);
            } else {
              const _a2 = new Status('Failed', {});
              info.status.drop();
              info.status = _a2;
              tracing.error(`Permanent failure for predicate ${queryId} with peer ${targetPeer}: ${errorMsg} - no retry`);
              {
                const _v = livequery;
                if (_v != null) {
                  const lq = _v;
                  _moved0 = true;
                  lq.setLastError(error);
                }
              }
            }
          }
        }
      } finally {
        subscriptions.drop();
      }
    } finally {
      if (!_moved0) error.drop();
    }
  }

  static default<CD, Q>(): SubscriptionRelay<CD, Q> {
    return SubscriptionRelay.new();
  }

  clone(): SubscriptionRelay<CD, Q> {
    return new SubscriptionRelay(this.inner.clone());
  }
}

export type StatusV = {
  PendingRemote: {};
  Requested: { _0: EntityId; _1: number };
  Established: { _0: EntityId; _1: number };
  PendingUpdate: { _0: EntityId; _1: number };
  Failed: {};
};

export class Status extends Enum<StatusV> {

  clone(): Status {
    return this.match({
      PendingRemote: () => new Status('PendingRemote', {}),
      Requested: (v) => new Status('Requested', { _0: v._0.clone(), _1: v._1 }),
      Established: (v) => new Status('Established', { _0: v._0.clone(), _1: v._1 }),
      PendingUpdate: (v) => new Status('PendingUpdate', { _0: v._0.clone(), _1: v._1 }),
      Failed: () => new Status('Failed', {}),
    });
  }

  debug(): string {
    return this.match({
      PendingRemote: () => 'PendingRemote',
      Requested: (v) => `Requested(${v._0.debug()}, ${String(v._1)})`,
      Established: (v) => `Established(${v._0.debug()}, ${String(v._1)})`,
      PendingUpdate: (v) => `PendingUpdate(${v._0.debug()}, ${String(v._1)})`,
      Failed: () => 'Failed',
    });
  }
}

export interface RemoteQuerySubscriber {
  subscriptionEstablished(version: number): Promise<void>;
  setLastError(error: RetrievalError): void;
}

export interface TNode<CD extends ContextData> {
  remoteSubscribe(peerId: EntityId, queryId: QueryId, collectionId: CollectionId, selection: Selection, contextData: CD, version: number): Promise<Result<void, RetrievalError>>;
  peerUnsubscribe(peerId: EntityId, queryId: QueryId): Promise<Result<void, Error>>;
}

export async function WeakNode_remoteSubscribe<SE extends StorageEngine, PA extends PolicyAgent>(self: WeakNode<SE, PA>, peerId: EntityId, queryId: QueryId, collectionId: CollectionId, selection: Selection, contextData: ContextData, version: number): Promise<Result<void, RetrievalError>> {
  let _moved0 = false;
  try {
    try {
      const _r1 = self.upgrade().okOrElse(() => new RetrievalError('Other', { _0: 'Node has been dropped' }));
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      const node = _r1.unwrap();
      try {
        const _r2 = await node.fetchEntitiesFromLocal(collectionId, selection);
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        let _moved3 = false;
        const knownMatches = [..._r2.unwrap()].map((entity) => new ankurahProto.KnownEntity(entity.id(), entity.head()));
        try {
          _moved3 = true;
          const _r4 = await node.request(peerId, contextData, new NodeRequestBody('SubscribeQuery', { queryId: queryId, collection: collectionId.clone(), selection: selection.clone(), version: version, knownMatches: knownMatches })).mapErr((e) => new RetrievalError('RequestError', { _0: e }));
          if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(_r4.unwrapErr()) };
          const _m5 = await (async () => {
            return _r4.unwrap().intoMatch<any>({
              QuerySubscribed: (v) => {
                const _responseQueryId = v.queryId;
                const deltas = v.deltas;
                return deltas;
              },
              Error: (v) => {
                const e = v._0;
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('ServerError', { _0: e }) })) }
              },
              CommitComplete: (v) => {
                const other = new NodeResponseBody('CommitComplete', v);
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('UnexpectedResponse', { _0: other }) })) }
              },
              Fetch: (v) => {
                const other = new NodeResponseBody('Fetch', v);
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('UnexpectedResponse', { _0: other }) })) }
              },
              Get: (v) => {
                const other = new NodeResponseBody('Get', v);
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('UnexpectedResponse', { _0: other }) })) }
              },
              GetEvents: (v) => {
                const other = new NodeResponseBody('GetEvents', v);
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('UnexpectedResponse', { _0: other }) })) }
              },
              Success: (v) => {
                const other = new NodeResponseBody('Success', v);
                return { $jump: 'return', $value: Result.Err(new RetrievalError('RequestError', { _0: new RequestError('UnexpectedResponse', { _0: other }) })) }
              },
            });
          })();
          if ((_m5 as any)?.$jump === 'return') return (_m5 as any).$value;
          const deltas = (_m5 as any);
          tracing.debug(`Node.remote_subscribe: query_id: ${queryId}, collection_id: ${collectionId}, received deltas: ${deltas.length}`);
          _moved0 = true;
          const retriever = EphemeralNodeRetriever.new(collectionId, node, contextData);
          const applyResult = await NodeApplier.applyDeltas(node, peerId, deltas, retriever);
          try {
            const eventStoreResult = await retriever.storeUsedEvents();
            const _r6 = applyResult;
            if (_r6.isErr()) return Result.Err(RetrievalError.fromApplyError(_r6.unwrapErr()));
            _r6.drop();
            const _r7 = eventStoreResult;
            if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
            _r7.drop();
            return Result.Ok([]);
          } finally {
            applyResult.drop();
          }
        } finally {
          if (!_moved3) dropOwned(knownMatches);
        }
      } finally {
        node.drop();
      }
    } finally {
      selection.drop();
    }
  } finally {
    if (!_moved0) collectionId.drop();
  }
}

export async function WeakNode_peerUnsubscribe<SE extends StorageEngine, PA extends PolicyAgent>(self: WeakNode<SE, PA>, peerId: EntityId, queryId: QueryId): Promise<Result<void, AnyhowError>> {
  const _r0 = self.upgrade().okOrElse(() => AnyhowError.msg('Node has been dropped'));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const node = _r0.unwrap();
  try {
    const _r1 = await node.deref().value.requestRemoteUnsubscribe(queryId, [peerId]);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _r1.drop();
    return Result.Ok([]);
  } finally {
    node.drop();
  }
}

export function TNode_dispatch_peerUnsubscribe<CD>(self: unknown, peerId: EntityId, queryId: QueryId): Result<void, AnyhowError> {
  if (self instanceof WeakNode) return WeakNode_peerUnsubscribe(self as any, peerId, queryId);
  if (self instanceof MockMessageSender) return (self as any).peerUnsubscribe(peerId, queryId);
  throw new Error(`BUG: no TNode impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

export function TNode_dispatch_remoteSubscribe<CD>(self: unknown, peerId: EntityId, queryId: QueryId, collectionId: CollectionId, selection: Selection, contextData: CD, version: number): Result<void, RetrievalError> {
  if (self instanceof WeakNode) return WeakNode_remoteSubscribe(self as any, peerId, queryId, collectionId, selection, contextData, version);
  if (self instanceof MockMessageSender) return (self as any).remoteSubscribe(peerId, queryId, collectionId, selection, contextData, version);
  throw new Error(`BUG: no TNode impl for ${(self as object)?.constructor?.name ?? typeof self}`);
}

