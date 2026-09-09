// MIRRORS: ankurah/core/src/peer_subscription/server.rs
import { Struct, Result, OwnedClosure, AnyhowError, dropOwned, tracing, unsupported, iterFilterMap, HashMap } from '@ankurah/base';
import { Attested, CollectionId, EntityId, Event, KnownEntity, NodeResponseBody, NodeUpdateBody, QueryId, StateFragment, SubscriptionUpdateItem, UpdateContent } from '@ankurah/proto';
import { SubscriptionGuard } from '@ankurah/signals';
import { Entity } from '../entity';
import { SubscriptionError } from '../error';
import { ContextData, Node } from '../node';
import { ReactorSubscription, ReactorSubscriptionId } from '../reactor/subscription';
import { ReactorUpdate, ReactorUpdateItem } from '../reactor/update';
import { expandStates } from '../util/expand_states';
import { Selection } from '@ankurah/ankql';

export class SubscriptionHandler extends Struct {
  _peerId: EntityId;
  subscription: ReactorSubscription<Entity, Attested<Event>>;
  _guard: SubscriptionGuard;

  constructor(_peerId: EntityId, subscription: ReactorSubscription<Entity, Attested<Event>>, _guard: SubscriptionGuard) {
    super();
    this._peerId = _peerId;
    this.subscription = subscription;
    this._guard = _guard;
  }

  static new<SE, PA>(peerId: EntityId, node: Node<SE, PA>): SubscriptionHandler {
    let _moved0 = false;
    const subscription = node.deref().value.reactor.subscribe();
    try {
      const weakNode = node.weak();
      const guard = subscription.subscribe(new OwnedClosure([weakNode], (update: ReactorUpdate<Entity, Attested<Event>>) => {
        tracing.info(`SubscriptionHandler[${peerId}] received reactor update with ${update.items.length} items`);
        {
          const _v = weakNode.upgrade();
          if (_v != null) {
            const node = _v;
            try {
              tracing.debug(`SubscriptionHandler[${peerId}] sending update to peer ${peerId}`);
              node.sendUpdate(peerId, new NodeUpdateBody('SubscriptionUpdate', { items: iterFilterMap([...update.items], (item) => convertItem(node, peerId, item)) }));
            } finally {
              node.drop();
            }
          }
        }
      }, undefined, true));
      _moved0 = true;
      return new SubscriptionHandler(peerId, subscription, guard);
    } finally {
      if (!_moved0) subscription.drop();
    }
  }

  subscriptionId(): ReactorSubscriptionId {
    return this.subscription.id();
  }

  subscription(): ReactorSubscription<Entity, Attested<Event>> {
    return this.subscription;
  }

  removePredicate(queryId: QueryId): Result<void, SubscriptionError> {
    const _r0 = this.subscription.removePredicate(queryId);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    return Result.Ok([]);
  }

  async subscribeQuery<SE, PA>(node: Node<SE, PA>, queryId: QueryId, collectionId: CollectionId, selection: Selection, cdata: ContextData, version: number, knownMatches: KnownEntity[]): Promise<Result<NodeResponseBody, Error>> {
    let _moved0 = false;
    try {
      try {
        try {
          if (version === 0) {
            return Result.Err(AnyhowError.msg('Invalid version 0 for subscription'));
          }
          const _r1 = node.deref().value.policyAgent.canAccessCollection(cdata, collectionId);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          _r1.drop();
          const _r3 = node.deref().value.policyAgent.filterPredicate(cdata, collectionId, selection.takeField('predicate'));
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          const _a2 = _r3.unwrap();
          selection.predicate.drop();
          selection.predicate = _a2;
          const _r4 = await node.deref().value.collections.get(collectionId);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          const storageCollection = _r4.unwrap();
          try {
            const _b5 = this.subscription.id();
            let _moved7 = false;
            const _b6 = collectionId.clone();
            try {
              const _b8 = selection.clone();
              const _r9 = await node.deref().value.reactor.upsertQuery(_b5, queryId, _b6, _b8, node, cdata, version);
              if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
              _moved7 = true;
              let _moved10 = false;
              const matchingEntities = _r9.unwrap();
              try {
                _moved10 = true;
                const initialStates = iterFilterMap([...matchingEntities], (e) => {
                  try {
                    const _r11 = e.toEntityState().ok();
                    if (_r11 == null) return null;
                    let _moved12 = false;
                    const entityState = _r11;
                    try {
                      let _moved13 = false;
                      const attestation = node.deref().value.policyAgent.attestState(node, entityState);
                      try {
                        _moved12 = true;
                        _moved13 = true;
                        return Attested.opt(entityState, attestation);
                      } finally {
                        if (!_moved13) dropOwned(attestation);
                      }
                    } finally {
                      if (!_moved12) entityState.drop();
                    }
                  } finally {
                    e.drop();
                  }
                });
                const _r14 = await expandStates(initialStates, [...knownMatches].map((k) => k.entityId), storageCollection);
                if (_r14.isErr()) return Result.Err(_r14.unwrapErr());
                let _moved15 = false;
                const expandedStates = _r14.unwrap();
                try {
                  _moved0 = true;
                  const knownMap = HashMap.from([...knownMatches].map((k) => {
                    try {
                      return [k.entityId, k.takeField('head')];
                    } finally {
                      k.drop();
                    }
                  }));
                  let _moved16 = false;
                  let deltas = [];
                  try {
                    _moved15 = true;
                    const _seq18 = expandedStates;
                    let _at19 = 0;
                    try {
                      while (_at19 < _seq18.length) {
                        const state = _seq18[_at19++];
                        const _r17 = await node.generateEntityDelta(knownMap, state, storageCollection);
                        if (_r17.isErr()) return Result.Err(_r17.unwrapErr());
                        {
                          const _v = _r17.unwrap();
                          if (_v != null) {
                            const delta = _v;
                            deltas.push(delta);
                          }
                        }
                      }
                    } finally {
                      dropOwned(_seq18.slice(_at19));
                    }
                    _moved16 = true;
                    return Result.Ok(new NodeResponseBody('QuerySubscribed', { queryId: queryId, deltas: deltas }));
                  } finally {
                    if (!_moved16) dropOwned(deltas);
                  }
                } finally {
                  if (!_moved15) dropOwned(expandedStates);
                }
              } finally {
                if (!_moved10) dropOwned(matchingEntities);
              }
            } finally {
              if (!_moved7) dropOwned(_b6);
            }
          } finally {
            storageCollection.drop();
          }
        } finally {
          if (!_moved0) dropOwned(knownMatches);
        }
      } finally {
        selection.drop();
      }
    } finally {
      collectionId.drop();
    }
  }
}

function convertItem<SE, PA>(node: Node<SE, PA>, peerId: EntityId, item: ReactorUpdateItem<Entity, Attested<Event>>): SubscriptionUpdateItem | null {
  try {
    const _m0 = (() => {
      const _v = item.entity.toEntityState();
      if (_v.isOk()) {
        const entityState = _v.unwrap();
        return entityState;
      } else {
        const e = _v.unwrapErr();
        try {
          {
            tracing.warn(`Failed to convert entity ${item.entity.id()} to EntityState for peer ${peerId}: ${e}`);
            return { $jump: 'return', $value: null };
          }
        } finally {
          e.drop();
        }
      }
    })();
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
    let _moved1 = false;
    const entityState = (_m0 as any);
    try {
      let _moved2 = false;
      const attestation = node.deref().value.policyAgent.attestState(node, entityState);
      try {
        _moved1 = true;
        _moved2 = true;
        let _moved3 = false;
        const attestedState = Attested.opt(entityState, attestation);
        try {
          let _moved4 = false;
          const attestedEvents = item.events;
          try {
            _moved3 = true;
            _moved4 = true;
            let _moved5 = false;
            const content = new UpdateContent('StateAndEvent', { _0: StateFragment.from(attestedState), _1: [...attestedEvents].map((e) => e) });
            try {
              const predicateRelevance = unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into');
              const _b6 = item.entity.id();
              const _b7 = item.entity.collection().clone();
              _moved5 = true;
              return new SubscriptionUpdateItem(_b6, _b7, content, predicateRelevance);
            } finally {
              if (!_moved5) content.drop();
            }
          } finally {
            if (!_moved4) dropOwned(attestedEvents);
          }
        } finally {
          if (!_moved3) attestedState.drop();
        }
      } finally {
        if (!_moved2) dropOwned(attestation);
      }
    } finally {
      if (!_moved1) entityState.drop();
    }
  } finally {
    item.drop();
  }
}

