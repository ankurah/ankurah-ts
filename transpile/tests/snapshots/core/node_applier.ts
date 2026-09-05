// MIRRORS: ankurah/core/src/node_applier.rs
import { Struct, Result, OwnedClosure, dropOwned, tracing, dropUnbound } from '@ankurah/base';
import { Event, EventId, Attested, CollectionId, EntityDelta, EntityId, EntityState, EventFragment, SubscriptionUpdateItem } from '@ankurah/proto';
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
      const _seq3 = items;
      let _at4 = 0;
      try {
        while (_at4 < _seq3.length) {
          const update = _seq3[_at4++];
          const retriever = EphemeralNodeRetriever.new(update.collection.clone(), node, cdata);
          const _r1 = await NodeApplier.applyUpdate(node, fromPeerId, update, retriever, changes, []);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          _r1.drop();
          const _r2 = await retriever.storeUsedEvents();
          if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
          _r2.drop();
        }
      } finally {
        dropOwned(_seq3.slice(_at4));
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
      const _m19 = await (content.intoMatch<any>({
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
                let appliedEvents = [];
                _moved3 = true;
                const _seq9 = events;
                let _at10 = 0;
                try {
                  while (_at10 < _seq9.length) {
                    const event = _seq9[_at10++];
                    let _moved6 = false;
                    try {
                      let _c8;
                      const _r7 = await entity.applyEvent(retriever, event.payload);
                      if (_r7.isErr()) return { $jump: 'return', $value: Result.Err(_r7.unwrapErr()) };
                      _c8 = _r7.unwrap();
                      if (_c8) {
                        _moved6 = true;
                        appliedEvents.push(event);
                      }
                    } finally {
                      if (!_moved6) event.drop();
                    }
                  }
                } finally {
                  dropOwned(_seq9.slice(_at10));
                }
                if (!(appliedEvents.length === 0)) {
                  _moved5 = true;
                  const _r11 = EntityChange.new(entity, appliedEvents);
                  if (_r11.isErr()) return { $jump: 'return', $value: Result.Err(_r11.unwrapErr()) };
                  changes.push(_r11.unwrap());
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
          let _moved12 = false;
          try {
            try {
              _moved12 = true;
              const _r13 = await NodeApplier.saveEvents(node, fromPeerId, entityId, collectionId, eventFragments, collection);
              if (_r13.isErr()) return { $jump: 'return', $value: Result.Err(_r13.unwrapErr()) };
              let _moved14 = false;
              const events = _r13.unwrap();
              try {
                const state = [entityId, collectionId.clone(), stateFragment.clone()];
                const _r15 = node.deref().value.policyAgent.validateReceivedState(node, fromPeerId, state);
                if (_r15.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromAccessDenied(_r15.unwrapErr())) };
                _r15.drop();
                const _r16 = await node.deref().value.entities.withState(retriever, entityId, collectionId, state.payload.state);
                if (_r16.isErr()) return { $jump: 'return', $value: Result.Err(MutationError.fromRetrievalError(_r16.unwrapErr())) };
                const [changed, entity] = _r16.unwrap();
                entities.push(entity.clone());
                if ((changed != null && (changed === true)) || (changed == null)) {
                  const _r17 = await NodeApplier.saveState(node, entity, collection);
                  if (_r17.isErr()) return { $jump: 'return', $value: Result.Err(_r17.unwrapErr()) };
                  _r17.drop();
                  _moved14 = true;
                  const _r18 = EntityChange.new(entity, events);
                  if (_r18.isErr()) return { $jump: 'return', $value: Result.Err(_r18.unwrapErr()) };
                  changes.push(_r18.unwrap());
                }
              } finally {
                if (!_moved14) dropOwned(events);
              }
            } finally {
              if (!_moved12) dropOwned(eventFragments);
            }
          } finally {
            stateFragment.drop();
          }
        },
      }));
      if ((_m19 as any)?.$jump === 'return') return (_m19 as any).$value;
      return Result.Ok([]);
    } finally {
      collection.drop();
    }
  }

  static async saveEvents<SE, PA>(node: Node<SE, PA>, fromPeerId: EntityId, entityId: EntityId, collectionId: CollectionId, fragments: EventFragment[], collection: StorageCollectionWrapper): Promise<Result<Attested<Event>[], MutationError>> {
    let attestedEvents = [];
    const _seq2 = fragments;
    let _at3 = 0;
    try {
      while (_at3 < _seq2.length) {
        const fragment = _seq2[_at3++];
        const attestedEvent = [entityId, collectionId.clone(), fragment];
        const _r0 = node.deref().value.policyAgent.validateReceivedEvent(node, fromPeerId, attestedEvent);
        if (_r0.isErr()) return Result.Err(MutationError.fromAccessDenied(_r0.unwrapErr()));
        _r0.drop();
        const _r1 = await collection.deref().value.addEvent(attestedEvent);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        _r1.drop();
        attestedEvents.push(attestedEvent);
      }
    } finally {
      dropOwned(_seq2.slice(_at3));
    }
    return Result.Ok(attestedEvents);
  }

  static async saveState<SE, PA>(node: Node<SE, PA>, entity: Entity, collectionWrapper: StorageCollectionWrapper): Promise<Result<void, MutationError>> {
    const _r0 = entity.toState();
    if (_r0.isErr()) return Result.Err(MutationError.fromStateError(_r0.unwrapErr()));
    let _moved1 = false;
    const state = _r0.unwrap();
    try {
      _moved1 = true;
      let _moved2 = false;
      const entityState = new EntityState(entity.id(), entity.collection().clone(), state);
      try {
        let _moved3 = false;
        const attestation = node.deref().value.policyAgent.attestState(node, entityState);
        try {
          _moved2 = true;
          _moved3 = true;
          const attested = Attested.opt(entityState, attestation);
          const _r4 = await collectionWrapper.deref().value.setState(attested);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          _r4.drop();
          return Result.Ok([]);
        } finally {
          if (!_moved3) dropOwned(attestation);
        }
      } finally {
        if (!_moved2) entityState.drop();
      }
    } finally {
      if (!_moved1) state.drop();
    }
  }

  static async applyDeltas<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, deltas: EntityDelta[], retriever: R): Promise<Result<void, ApplyError>> {
    let readyChunks = ReadyChunks.new([...deltas].map((delta) => NodeApplier.applyDelta(node, fromPeerId, delta, retriever)));
    let allErrors = [];
    for (;;) {
      const _v1 = await readyChunks.next();
      if (!(_v1 != null)) {
        break;
      }
      const results = _v1;
      let batch = [];
      for (const result of results) {
        if (result.isOk()) {
          const none = result.unwrap();

        } else {
          const errorItem = result.unwrapErr();
          allErrors.push(errorItem);
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
  }

  static async applyDelta<SE, PA, R>(node: Node<SE, PA>, fromPeerId: EntityId, delta: EntityDelta, retriever: R): Promise<Result<EntityChange | null, ApplyErrorItem>> {
    const entityId = delta.entityId;
    const collection = delta.collection.clone();
    const result = await NodeApplier.applyDeltaInner(node, fromPeerId, delta, retriever);
    return result.mapErr(new OwnedClosure([collection], (cause) => new ApplyErrorItem(entityId, collection, cause), undefined, true));
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
              _moved1 = true;
              const attestedState = [delta.entityId, delta.collection.clone(), state];
              const _r2 = node.deref().value.policyAgent.validateReceivedState(node, fromPeerId, attestedState);
              if (_r2.isErr()) return Result.Err(MutationError.fromAccessDenied(_r2.unwrapErr()));
              _r2.drop();
              const _r3 = await node.deref().value.entities.withState(retriever, delta.entityId, delta.takeField('collection'), attestedState.payload.state);
              if (_r3.isErr()) return Result.Err(MutationError.fromRetrievalError(_r3.unwrapErr()));
              const [, entity] = _r3.unwrap();
              const _r4 = await NodeApplier.saveState(node, entity, collection);
              if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
              _r4.drop();
              const _r5 = EntityChange.new(entity, []);
              if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
              return Result.Ok(_r5.unwrap());
            } finally {
              if (!_moved1) state.drop();
            }
          },
          EventBridge: async (v) => {
            const events = v.events;
            let _moved6 = false;
            try {
              _moved6 = true;
              let _moved7 = false;
              const attestedEvents = [...events].map((f) => [delta.entityId, delta.collection.clone(), f]);
              try {
                retriever.stageEvents(attestedEvents.clone());
                const _r8 = await node.deref().value.entities.getRetrieveOrCreate(retriever, delta.collection, delta.entityId);
                if (_r8.isErr()) return Result.Err(MutationError.fromRetrievalError(_r8.unwrapErr()));
                let _moved9 = false;
                const entity = _r8.unwrap();
                try {
                  _moved7 = true;
                  for (const event of [...attestedEvents].rev()) {
                    try {
                      const _r10 = await entity.applyEvent(retriever, event.payload);
                      if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                      _r10.drop();
                      const _t11 = event.payload.id();
                      try {
                        retriever.markEventUsed(_t11);
                      } finally {
                        _t11.drop();
                      }
                    } finally {
                      event.drop();
                    }
                  }
                  const _r12 = await NodeApplier.saveState(node, entity, collection);
                  if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                  _r12.drop();
                  _moved9 = true;
                  const _r13 = EntityChange.new(entity, []);
                  if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
                  return Result.Ok(_r13.unwrap());
                } finally {
                  if (!_moved9) entity.drop();
                }
              } finally {
                if (!_moved7) dropOwned(attestedEvents);
              }
            } finally {
              if (!_moved6) dropOwned(events);
            }
          },
          StateAndRelation: (v) => {
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

