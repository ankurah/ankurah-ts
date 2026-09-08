// MIRRORS: ankurah/core/src/node_applier.rs
import { Struct, Result, OwnedClosure, dropOwned, tracing, dropUnbound } from '@ankurah/base';
import { Event, Attested, CollectionId, EntityDelta, EntityId, EntityState, EventFragment, SubscriptionUpdateItem } from '@ankurah/proto';
import { EntityChange } from './changes';
import { Entity } from './entity';
import { ApplyError, ApplyErrorItem, MutationError } from './error';
import { Node } from './node';
import { EphemeralNodeRetriever } from './retrieval';
import { StorageCollectionWrapper } from './storage';
import { ReadyChunks } from './util/ready_chunks';

export class NodeApplier extends Struct {

  static async applyUpdates<SE, PA>(node: Node<SE, PA>, fromPeerId: EntityId, items: SubscriptionUpdateItem[]): Promise<Result<void, MutationError>> {
    let _moved0 = false;
    try {
      tracing.debug(`received subscription update for ${items.length} items`);
      const _v = node.deref().value.subscriptionRelay;
      if (!(_v != null)) {
        return Result.Err(new MutationError('InvalidUpdate', { _0: 'Should not be receiving updates without a subscription relay' }));
      }
      const relay = _v;
      const cdata = relay.getContextsForPeer(fromPeerId);
      if (cdata.size === 0) {
        return Result.Err(new MutationError('InvalidUpdate', { _0: 'Should not be receiving updates without at least predicate context' }));
      }
      let changes = [];
      _moved0 = true;
      const _seq4 = items;
      let _at5 = 0;
      try {
        while (_at5 < _seq4.length) {
          const update = _seq4[_at5++];
          let _moved1 = false;
          try {
            const retriever = EphemeralNodeRetriever.new(update.collection.clone(), node, cdata);
            try {
              _moved1 = true;
              const _r2 = await NodeApplier.applyUpdate(node, fromPeerId, update, retriever, changes, []);
              if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
              _r2.drop();
              const _r3 = await retriever.storeUsedEvents();
              if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
              _r3.drop();
            } finally {
              retriever.drop();
            }
          } finally {
            if (!_moved1) update.drop();
          }
        }
      } finally {
        dropOwned(_seq4.slice(_at5));
      }
      await node.deref().value.reactor.notifyChange(changes);
      return Result.Ok([]);
    } finally {
      if (!_moved0) dropOwned(items);
    }
  }

  static async applyUpdate<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, update: SubscriptionUpdateItem, retriever: R, changes: EntityChange[], entities: Pushable): Promise<Result<void, MutationError>> {
    const { entityId, collection: collectionId, content } = update;
    const _r0 = await node.deref().value.collections.get(collectionId);
    if (_r0.isErr()) return Result.Err(MutationError.fromRetrievalError(_r0.unwrapErr()));
    const collection = _r0.unwrap();
    try {
      const _m23 = await (content.intoMatch<any>({
        EventOnly: async (v) => {
          const eventFragments = v._0;
          let _moved1 = false;
          try {
            _moved1 = true;
            const _r2 = await NodeApplier.saveEvents(node, fromPeerId, entityId, collectionId, eventFragments, collection);
            if (_r2.isErr()) return { $jump: 'return', $value: Result.Err(_r2.unwrapErr()) };
            let _moved3 = false;
            const events = _r2.unwrap();
            try {
              const _r4 = await node.deref().value.entities.getRetrieveOrCreate(retriever, collectionId, entityId);
              if (_r4.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromRetrievalError(_r4.unwrapErr())) };
              let _moved5 = false;
              const entity = _r4.unwrap();
              try {
                entities.push(entity.clone());
                let _moved6 = false;
                let appliedEvents = [];
                try {
                  _moved3 = true;
                  const _seq10 = events;
                  let _at11 = 0;
                  try {
                    while (_at11 < _seq10.length) {
                      const event = _seq10[_at11++];
                      let _moved7 = false;
                      try {
                        let _c9;
                        const _r8 = await entity.applyEvent(retriever, event.payload);
                        if (_r8.isErr()) return { $jump: 'return', $value: Result.Err(_r8.unwrapErr()) };
                        _c9 = _r8.unwrap();
                        if (_c9) {
                          _moved7 = true;
                          appliedEvents.push(event);
                        }
                      } finally {
                        if (!_moved7) event.drop();
                      }
                    }
                  } finally {
                    dropOwned(_seq10.slice(_at11));
                  }
                  if (!(appliedEvents.length === 0)) {
                    _moved5 = true;
                    _moved6 = true;
                    const _r12 = EntityChange.new(entity, appliedEvents);
                    if (_r12.isErr()) return { $jump: 'return', $value: Result.Err(_r12.unwrapErr()) };
                    changes.push(_r12.unwrap());
                  }
                } finally {
                  if (!_moved6) dropOwned(appliedEvents);
                }
              } finally {
                if (!_moved5) entity.drop();
              }
            } finally {
              if (!_moved3) dropOwned(events);
            }
          } finally {
            if (!_moved1) dropOwned(eventFragments);
          }
        },
        StateAndEvent: async (v) => {
          const stateFragment = v._0;
          const eventFragments = v._1;
          let _moved13 = false;
          try {
            try {
              _moved13 = true;
              const _r14 = await NodeApplier.saveEvents(node, fromPeerId, entityId, collectionId, eventFragments, collection);
              if (_r14.isErr()) return { $jump: 'return', $value: Result.Err(_r14.unwrapErr()) };
              let _moved15 = false;
              const events = _r14.unwrap();
              try {
                let _moved17 = false;
                const _b16 = collectionId.clone();
                try {
                  const _b18 = stateFragment.clone();
                  _moved17 = true;
                  const state = [entityId, _b16, _b18];
                  const _r19 = node.deref().value.policyAgent.validateReceivedState(node, fromPeerId, state);
                  if (_r19.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromAccessDenied(_r19.unwrapErr())) };
                  _r19.drop();
                  const _r20 = await node.deref().value.entities.withState(retriever, entityId, collectionId, state.payload.state);
                  if (_r20.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromRetrievalError(_r20.unwrapErr())) };
                  const [changed, entity] = _r20.unwrap();
                  entities.push(entity.clone());
                  if ((changed != null && (changed === true)) || (changed == null)) {
                    const _r21 = await NodeApplier.saveState(node, entity, collection);
                    if (_r21.isErr()) return { $jump: 'return', $value: Result.Err(_r21.unwrapErr()) };
                    _r21.drop();
                    _moved15 = true;
                    const _r22 = EntityChange.new(entity, events);
                    if (_r22.isErr()) return { $jump: 'return', $value: Result.Err(_r22.unwrapErr()) };
                    changes.push(_r22.unwrap());
                  }
                } finally {
                  if (!_moved17) dropOwned(_b16);
                }
              } finally {
                if (!_moved15) dropOwned(events);
              }
            } finally {
              if (!_moved13) dropOwned(eventFragments);
            }
          } finally {
            stateFragment.drop();
          }
        },
      }));
      if ((_m23 as any)?.$jump === 'return') return (_m23 as any).$value;
      return Result.Ok([]);
    } finally {
      collection.drop();
    }
  }

  static async saveEvents<SE, PA>(node: Node<SE, PA>, fromPeerId: EntityId, entityId: EntityId, collectionId: CollectionId, fragments: EventFragment[], collection: StorageCollectionWrapper): Promise<Result<Attested<Event>[], MutationError>> {
    let _moved0 = false;
    try {
      let attestedEvents = [];
      _moved0 = true;
      const _seq5 = fragments;
      let _at6 = 0;
      try {
        while (_at6 < _seq5.length) {
          const fragment = _seq5[_at6++];
          let _moved1 = false;
          try {
            const _b2 = collectionId.clone();
            _moved1 = true;
            const attestedEvent = [entityId, _b2, fragment];
            const _r3 = node.deref().value.policyAgent.validateReceivedEvent(node, fromPeerId, attestedEvent);
            if (_r3.isErr()) return Result.Err(MutationError.fromAccessDenied(_r3.unwrapErr()));
            _r3.drop();
            const _r4 = await collection.deref().value.addEvent(attestedEvent);
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            _r4.drop();
            attestedEvents.push(attestedEvent);
          } finally {
            if (!_moved1) fragment.drop();
          }
        }
      } finally {
        dropOwned(_seq5.slice(_at6));
      }
      return Result.Ok(attestedEvents);
    } finally {
      if (!_moved0) dropOwned(fragments);
    }
  }

  static async saveState<SE, PA>(node: Node<SE, PA>, entity: Entity, collectionWrapper: StorageCollectionWrapper): Promise<Result<void, MutationError>> {
    const _r0 = entity.toState();
    if (_r0.isErr()) return Result.Err(MutationError.fromStateError(_r0.unwrapErr()));
    let _moved1 = false;
    const state = _r0.unwrap();
    try {
      const _b2 = entity.id();
      const _b3 = entity.collection().clone();
      _moved1 = true;
      let _moved4 = false;
      const entityState = new EntityState(_b2, _b3, state);
      try {
        let _moved5 = false;
        const attestation = node.deref().value.policyAgent.attestState(node, entityState);
        try {
          _moved4 = true;
          _moved5 = true;
          let _moved6 = false;
          const attested = Attested.opt(entityState, attestation);
          try {
            _moved6 = true;
            const _r7 = await collectionWrapper.deref().value.setState(attested);
            if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
            _r7.drop();
            return Result.Ok([]);
          } finally {
            if (!_moved6) attested.drop();
          }
        } finally {
          if (!_moved5) dropOwned(attestation);
        }
      } finally {
        if (!_moved4) entityState.drop();
      }
    } finally {
      if (!_moved1) state.drop();
    }
  }

  static async applyDeltas<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, deltas: EntityDelta[], retriever: R): Promise<Result<void, ApplyError>> {
    let readyChunks = ReadyChunks.new([...deltas].map((delta) => NodeApplier.applyDelta(node, fromPeerId, delta, retriever)));
    try {
      let allErrors = [];
      for (;;) {
        const _v2 = await readyChunks.next();
        if (!(_v2 != null)) {
          break;
        }
        const results = _v2;
        let batch = [];
        for (const result of results) {
          if (result.isOk()) {
            const _v = result.unwrap();
            _arm0: {
              if (_v != null) {
                const change = _v;
                batch.push(change)
                break _arm0;
              }
              {
                const _v1 = _v;
                dropOwned(_v1);
              }
            }
          } else {
            const errorItem = result.unwrapErr();
            let _moved1 = false;
            try {
              _moved1 = true;
              allErrors.push(errorItem);
            } finally {
              if (!_moved1) dropOwned(errorItem);
            }
          }
        }
        if (!(batch.length === 0)) {
          await node.deref().value.reactor.notifyChange(batch);
        }
      }
      if (!(allErrors.length === 0)) {
        return Result.Err(new ApplyError('Items', { _0: allErrors }));
      }
      return Result.Ok([]);
    } finally {
      readyChunks.drop();
    }
  }

  static async applyDelta<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, delta: EntityDelta, retriever: R): Promise<Result<EntityChange | null, ApplyErrorItem>> {
    let _moved0 = false;
    try {
      const entityId = delta.entityId;
      const collection = delta.collection.clone();
      _moved0 = true;
      const result = await NodeApplier.applyDeltaInner(node, fromPeerId, delta, retriever);
      return result.mapErr(new OwnedClosure([collection], (cause: MutationError) => new ApplyErrorItem(entityId, collection, cause), undefined, true));
    } finally {
      if (!_moved0) delta.drop();
    }
  }

  static async applyDeltaInner<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, delta: EntityDelta, retriever: R): Promise<Result<EntityChange | null, MutationError>> {
    try {
      const _r0 = await node.deref().value.collections.get(delta.collection);
      if (_r0.isErr()) return Result.Err(MutationError.fromRetrievalError(_r0.unwrapErr()));
      const collection = _r0.unwrap();
      try {
        return await (delta.takeField('content').intoMatch({
          StateSnapshot: async (v) => {
            const state = v.state;
            let _moved1 = false;
            try {
              const _b2 = delta.collection.clone();
              _moved1 = true;
              const attestedState = [delta.entityId, _b2, state];
              const _r3 = node.deref().value.policyAgent.validateReceivedState(node, fromPeerId, attestedState);
              if (_r3.isErr()) return Result.Err(MutationError.fromAccessDenied(_r3.unwrapErr()));
              _r3.drop();
              const _r4 = await node.deref().value.entities.withState(retriever, delta.entityId, delta.takeField('collection'), attestedState.payload.state);
              if (_r4.isErr()) return Result.Err(MutationError.fromRetrievalError(_r4.unwrapErr()));
              const [, entity] = _r4.unwrap();
              const _r5 = await NodeApplier.saveState(node, entity, collection);
              if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
              _r5.drop();
              const _r6 = EntityChange.new(entity, []);
              if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
              return Result.Ok(_r6.unwrap());
            } finally {
              if (!_moved1) state.drop();
            }
          },
          EventBridge: async (v) => {
            const events = v.events;
            let _moved7 = false;
            try {
              _moved7 = true;
              let _moved10 = false;
              const attestedEvents = [...events].map((f) => {
                try {
                  const _b9 = delta.collection.clone();
                  return [delta.entityId, _b9, f];
                } finally {
                  f.drop();
                }
              });
              try {
                retriever.stageEvents(attestedEvents.map((e) => e.clone()));
                const _r11 = await node.deref().value.entities.getRetrieveOrCreate(retriever, delta.collection, delta.entityId);
                if (_r11.isErr()) return Result.Err(MutationError.fromRetrievalError(_r11.unwrapErr()));
                let _moved12 = false;
                const entity = _r11.unwrap();
                try {
                  _moved10 = true;
                  for (const event of [...attestedEvents].reverse()) {
                    try {
                      const _r13 = await entity.applyEvent(retriever, event.payload);
                      if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
                      _r13.drop();
                      const _t14 = event.payload.id();
                      try {
                        retriever.markEventUsed(_t14);
                      } finally {
                        _t14.drop();
                      }
                    } finally {
                      event.drop();
                    }
                  }
                  const _r15 = await NodeApplier.saveState(node, entity, collection);
                  if (_r15.isErr()) return Result.Err(_r15.unwrapErr());
                  _r15.drop();
                  _moved12 = true;
                  const _r16 = EntityChange.new(entity, []);
                  if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
                  return Result.Ok(_r16.unwrap());
                } finally {
                  if (!_moved12) entity.drop();
                }
              } finally {
                if (!_moved10) dropOwned(attestedEvents);
              }
            } finally {
              if (!_moved7) dropOwned(events);
            }
          },
          StateAndRelation: async (v) => {
            try {
              throw new Error('unimplemented');
            } finally {
              dropUnbound(v, []);
            }
          },
        }));
      } finally {
        collection.drop();
      }
    } finally {
      delta.drop();
    }
  }
}

interface Pushable<T> {
  push(value: T): void;
}

export function Vec_push<T>(self: T[], value: T): void {
  self.push(value);
}

export function Unit_push<T>(self: void, arg: T): void {

}

