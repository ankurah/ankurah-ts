// MIRRORS: ankurah/core/src/peer_subscription/server.rs
import { Struct, Result, OwnedClosure, AnyhowError, dropOwned, tracing, unsupported, iterFilterMap, HashMap } from '@ankurah/base';
import { Attested, CollectionId, EntityId, Event, KnownEntity, NodeResponseBody, NodeUpdateBody, QueryId, SubscriptionUpdateItem, UpdateContent } from '@ankurah/proto';
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
    const subscription = node.deref().value.reactor.subscribe();
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
    return new SubscriptionHandler(peerId, subscription, guard);
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
            const _r5 = await node.deref().value.reactor.upsertQuery(this.subscription.id(), queryId, collectionId.clone(), selection.clone(), node, cdata, version);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            let _moved6 = false;
            const matchingEntities = _r5.unwrap();
            try {
              _moved6 = true;
              const initialStates = iterFilterMap([...matchingEntities], (e) => {
                const _r7 = e.toEntityState().ok();
                if (_r7 == null) return null;
                let _moved8 = false;
                const entityState = _r7;
                try {
                  let _moved9 = false;
                  const attestation = node.deref().value.policyAgent.attestState(node, entityState);
                  try {
                    _moved8 = true;
                    _moved9 = true;
                    return Attested.opt(entityState, attestation);
                  } finally {
                    if (!_moved9) dropOwned(attestation);
                  }
                } finally {
                  if (!_moved8) entityState.drop();
                }
              });
              const _r10 = await expandStates(initialStates, [...knownMatches].map((k) => k.entityId), storageCollection);
              if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
              let _moved11 = false;
              const expandedStates = _r10.unwrap();
              try {
                _moved0 = true;
                const knownMap = HashMap.from([...knownMatches].map((k) => [k.entityId, k.takeField('head')]));
                let deltas = [];
                _moved11 = true;
                const _seq13 = expandedStates;
                let _at14 = 0;
                try {
                  while (_at14 < _seq13.length) {
                    const state = _seq13[_at14++];
                    const _r12 = await node.generateEntityDelta(knownMap, state, storageCollection);
                    if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                    {
                      const _v = _r12.unwrap();
                      if (_v != null) {
                        const delta = _v;
                        deltas.push(delta);
                      }
                    }
                  }
                } finally {
                  dropOwned(_seq13.slice(_at14));
                }
                return Result.Ok(new NodeResponseBody('QuerySubscribed', { queryId: queryId, deltas: deltas }));
              } finally {
                if (!_moved11) dropOwned(expandedStates);
              }
            } finally {
              if (!_moved6) dropOwned(matchingEntities);
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
    const entityState = (_m0 as any);
    let _moved1 = false;
    const attestation = node.deref().value.policyAgent.attestState(node, entityState);
    try {
      _moved1 = true;
      const attestedState = Attested.opt(entityState, attestation);
      let _moved2 = false;
      const attestedEvents = item.events;
      try {
        _moved2 = true;
        let _moved3 = false;
        const content = new UpdateContent('StateAndEvent', { _0: attestedState, _1: [...attestedEvents].map((e) => e) });
        try {
          const predicateRelevance = unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into');
          _moved3 = true;
          return new SubscriptionUpdateItem(item.entity.id(), item.entity.collection().clone(), content, predicateRelevance);
        } finally {
          if (!_moved3) content.drop();
        }
      } finally {
        if (!_moved2) dropOwned(attestedEvents);
      }
    } finally {
      if (!_moved1) dropOwned(attestation);
    }
  } finally {
    item.drop();
  }
}

