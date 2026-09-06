// MIRRORS: ankurah/core/src/reactor/watcherset.rs
import { Struct, Enum, Arc, derivedClone, valueEquals, HashMap, HashSet } from '@ankurah/base';
import { Expr, Predicate, Literal } from '@ankurah/ankql';
import { Comparison } from '../lineage';
import { AbstractEntity } from '../reactor';
import { CandidateChanges } from './candidate_changes';
import { ComparisonIndex } from './comparison_index';
import { PropertyPath } from './property_path';
import { ReactorSubscriptionId } from './subscription';
import { Subscription } from './subscription_state';
import { CollectionId, EntityId, QueryId } from '@ankurah/proto';

export class WatcherSet extends Struct {
  indexWatchers: HashMap<[CollectionId, PropertyPath], ComparisonIndex<[ReactorSubscriptionId, QueryId]>>;
  wildcardWatchers: HashMap<CollectionId, HashSet<[ReactorSubscriptionId, QueryId]>>;
  entityWatchers: HashMap<EntityId, HashSet<EntityWatcherId>>;

  constructor(indexWatchers: HashMap<[CollectionId, PropertyPath], ComparisonIndex<[ReactorSubscriptionId, QueryId]>>, wildcardWatchers: HashMap<CollectionId, HashSet<[ReactorSubscriptionId, QueryId]>>, entityWatchers: HashMap<EntityId, HashSet<EntityWatcherId>>) {
    super();
    this.indexWatchers = indexWatchers;
    this.wildcardWatchers = wildcardWatchers;
    this.entityWatchers = entityWatchers;
  }

  static new(): WatcherSet {
    return new WatcherSet(new HashMap<[CollectionId, PropertyPath], ComparisonIndex<[ReactorSubscriptionId, QueryId]>>(), new HashMap<CollectionId, HashSet<[ReactorSubscriptionId, QueryId]>>(), new HashMap<EntityId, HashSet<EntityWatcherId>>());
  }

  accumulateInterestedWatchers<E extends AbstractEntity, C>(entity: E, offset: number, changesArc: Arc<C[]>, candidatesBySub: HashMap<ReactorSubscriptionId, CandidateChanges<C>>): void {
    const entityId = AbstractEntity.id(entity);
    for (const [[collectionId, propertyPath], indexRef] of this.indexWatchers) {
      if (valueEquals(collectionId, AbstractEntity.collection(entity))) {
        {
          const _v = propertyPath.extractValue(entity);
          if (_v != null) {
            const value = _v;
            for (const [subscriptionId, queryId] of indexRef.findMatching(value)) {
              candidatesBySub.entry(subscriptionId).orInsertWith(() => CandidateChanges.new(changesArc.value.map((e) => derivedClone(e)))).value.addQuery(queryId, offset);
            }
          }
        }
      }
    }
    {
      const _v1 = this.wildcardWatchers.get(AbstractEntity.collection(entity));
      if (_v1 != null) {
        const watchers = _v1;
        for (const [subscriptionId, queryId] of [...watchers]) {
          candidatesBySub.entry(subscriptionId).orInsertWith(() => CandidateChanges.new(changesArc.value.map((e) => derivedClone(e)))).value.addQuery(queryId, offset);
        }
      }
    }
    {
      const _v2 = this.entityWatchers.get(entityId);
      if (_v2 != null) {
        const subscriptionIds = _v2;
        for (const subId of [...subscriptionIds]) {
          subId.match({
            Predicate: (v) => {
              const subscriptionId = v._0;
              const queryId = v._1;
              candidatesBySub.entry(subscriptionId).orInsertWith(() => CandidateChanges.new(changesArc.value.map((e) => derivedClone(e)))).value.addQuery(queryId, offset);
            },
            Subscription: (v) => {
              const subscriptionId = v._0;
              candidatesBySub.entry(subscriptionId).orInsertWith(() => CandidateChanges.new(changesArc.value.map((e) => derivedClone(e)))).value.addEntity(offset);
            },
          });
        }
      }
    }
  }

  applyWatcherChange(change: WatcherChange): void {
    try {
      return change.match({
        Add: (v) => {
          const entityId = v.entityId;
          const subscriptionId = v.subscriptionId;
          const queryId = v.queryId;
          this.entityWatchers.entry(entityId).orDefault(() => new HashSet()).value.add(new EntityWatcherId('Predicate', { _0: subscriptionId, _1: queryId }));
        },
        Remove: (v) => {
          const entityId = v.entityId;
          const subscriptionId = v.subscriptionId;
          const queryId = v.queryId;
          {
            const _v = this.entityWatchers.get(entityId);
            if (_v != null) {
              const watchers = _v;
              const _t0 = new EntityWatcherId('Predicate', { _0: subscriptionId, _1: queryId });
              try {
                watchers.delete(_t0);
              } finally {
                _t0.drop();
              }
              if (watchers.size === 0) {
                this.entityWatchers.delete(entityId);
              }
            }
          }
        },
      });
    } finally {
      change.drop();
    }
  }

  addEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void {
    this.entityWatchers.entry(entityId).orDefault(() => new HashSet()).value.add(new EntityWatcherId('Subscription', { _0: subscriptionId }));
  }

  removeEntitySubscription(subscriptionId: ReactorSubscriptionId, entityId: EntityId): void {
    {
      const _v = this.entityWatchers.get(entityId);
      if (_v != null) {
        const watchers = _v;
        const _t0 = new EntityWatcherId('Subscription', { _0: subscriptionId });
        try {
          watchers.delete(_t0);
        } finally {
          _t0.drop();
        }
        if (watchers.size === 0) {
          this.entityWatchers.delete(entityId);
        }
      }
    }
  }

  removeEntitySubscriptions(subscriptionId: ReactorSubscriptionId, entityIds: EntityId[]): void {
    for (const entityId of entityIds) {
      this.removeEntitySubscription(subscriptionId, entityId);
    }
  }

  clearEntityWatchers(): void {
    this.entityWatchers.clear();
  }

  debugData(): [HashMap<[CollectionId, PropertyPath], ComparisonIndex<[ReactorSubscriptionId, QueryId]>>, HashMap<CollectionId, HashSet<[ReactorSubscriptionId, QueryId]>>, HashMap<EntityId, HashSet<EntityWatcherId>>] {
    return [this.indexWatchers, this.wildcardWatchers, this.entityWatchers];
  }

  addPredicateEntityWatchers(subscriptionId: ReactorSubscriptionId, queryId: QueryId, entityIds: EntityId[]): void {
    for (const entityId of entityIds) {
      this.entityWatchers.entry(entityId).orDefault(() => new HashSet()).value.add(new EntityWatcherId('Predicate', { _0: subscriptionId, _1: queryId }));
    }
  }

  cleanupRemovedPredicateWatchers(subscriptionId: ReactorSubscriptionId, queryId: QueryId, removedEntities: EntityId[]): void {
    for (const entityId of removedEntities) {
      {
        const _v = this.entityWatchers.get(entityId);
        if (_v != null) {
          const entityWatcher = _v;
          const _t0 = new EntityWatcherId('Predicate', { _0: subscriptionId, _1: queryId });
          try {
            entityWatcher.delete(_t0);
          } finally {
            _t0.drop();
          }
        }
      }
    }
  }

  recursePredicateWatchers(collectionId: CollectionId, predicate: Predicate, watcherId: [ReactorSubscriptionId, QueryId], op: WatcherOp): void {
    return predicate.match({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        {
          const _v = [left, right];
          if (((_v[0].is('Path')) && (_v[1].is('Literal'))) || ((_v[0].is('Literal')) && (_v[1].is('Path')))) {
            const path = (((_v[0].is('Path')) && (_v[1].is('Literal')))) ? _v[0].value._0 : (((_v[0].is('Literal')) && (_v[1].is('Path')))) ? _v[1].value._0 : undefined;
            const literal = (((_v[0].is('Path')) && (_v[1].is('Literal')))) ? _v[1].value._0 : (((_v[0].is('Literal')) && (_v[1].is('Path')))) ? _v[0].value._0 : undefined;
            const propertyPath = PropertyPath.fromPath(path);
            const index = this.indexWatchers.entry([collectionId.clone(), propertyPath]).orDefault(() => ComparisonIndex.default());
            return op.match({
              Add: () => {
                index.value.add((literal).clone(), operator.clone(), watcherId);
              },
              Remove: () => {
                index.value.remove((literal).clone(), operator.clone(), watcherId);
              },
            });
          } else {
        }
        }
      },
      And: (v) => {
        const left = v._0;
        const right = v._1;
        this.recursePredicateWatchers(collectionId, left, watcherId, op);
        this.recursePredicateWatchers(collectionId, right, watcherId, op);
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        this.recursePredicateWatchers(collectionId, left, watcherId, op);
        this.recursePredicateWatchers(collectionId, right, watcherId, op);
      },
      Not: (v) => {
        const pred = v._0;
        this.recursePredicateWatchers(collectionId, pred, watcherId, op);
      },
      IsNull: (v) => {
        throw new Error('unimplemented');
      },
      True: () => {
        const set = this.wildcardWatchers.entry(collectionId.clone()).orDefault(() => new HashSet());
        return op.match({
          Add: () => {
            set.value.add(watcherId);
          },
          Remove: () => {
            set.value.delete(watcherId);
          },
        });
      },
      False: () => {
        throw new Error('unimplemented');
      },
      Placeholder: () => {
        throw new Error('unimplemented');
      },
    });
  }
}

type EntityWatcherIdV = {
  Predicate: { _0: ReactorSubscriptionId; _1: QueryId };
  Subscription: { _0: ReactorSubscriptionId };
};

class EntityWatcherId extends Enum<EntityWatcherIdV> {

  subscriptionId(): ReactorSubscriptionId {
    return this.match({
      Predicate: (v) => {
        const subId = v._0;
        return subId;
      },
      Subscription: (v) => {
        const subId = v._0;
        return subId;
      },
    });
  }

  clone(): EntityWatcherId {
    return this.match({
      Predicate: (v) => new EntityWatcherId('Predicate', { _0: v._0.clone(), _1: v._1.clone() }),
      Subscription: (v) => new EntityWatcherId('Subscription', { _0: v._0.clone() }),
    });
  }

  equals(other: EntityWatcherId): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Predicate': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        if (!(this.value as any)._1.equals((other.value as any)._1)) return false;
        break;
      }
      case 'Subscription': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    switch (this.type) {
      case 'Predicate': return ['Predicate', (this.value as any)._0.hash(), (this.value as any)._1.hash()].map((p) => p.length + ':' + p).join('');
      case 'Subscription': return ['Subscription', (this.value as any)._0.hash()].map((p) => p.length + ':' + p).join('');
    }
    return String(this.type);
  }

  compareTo(other: EntityWatcherId): number {
    const order = ['Predicate', 'Subscription'];
    const a = order.indexOf(this.type);
    const b = order.indexOf(other.type);
    if (a !== b) return a < b ? -1 : 1;
    switch (this.type) {
      case 'Predicate': {
        let c = (this.value as any)._0.compareTo((other.value as any)._0);
        if (c !== 0) return c;
        c = (this.value as any)._1.compareTo((other.value as any)._1);
        if (c !== 0) return c;
        return 0;
      }
      case 'Subscription': {
        let c = (this.value as any)._0.compareTo((other.value as any)._0);
        if (c !== 0) return c;
        return 0;
      }
    }
    return 0;
  }

  debug(): string {
    return this.match({
      Predicate: (v) => `Predicate(${v._0.debug()}, ${v._1})`,
      Subscription: (v) => `Subscription(${v._0.debug()})`,
    });
  }
}

export type WatcherOpV = {
  Add: {};
  Remove: {};
};

export class WatcherOp extends Enum<WatcherOpV> {

  clone(): WatcherOp {
    return new WatcherOp(this.type, { ...this.value });
  }

  debug(): string {
    return this.match({
      Add: () => 'Add',
      Remove: () => 'Remove',
    });
  }
}

export type WatcherChangeV = {
  Add: { entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId };
  Remove: { entityId: EntityId; subscriptionId: ReactorSubscriptionId; queryId: QueryId };
};

export class WatcherChange extends Enum<WatcherChangeV> {

  static add(entityId: EntityId, subscriptionId: ReactorSubscriptionId, queryId: QueryId): WatcherChange {
    return new WatcherChange('Add', { entityId: entityId, subscriptionId: subscriptionId, queryId: queryId });
  }

  static remove(entityId: EntityId, subscriptionId: ReactorSubscriptionId, queryId: QueryId): WatcherChange {
    return new WatcherChange('Remove', { entityId: entityId, subscriptionId: subscriptionId, queryId: queryId });
  }

  clone(): WatcherChange {
    return this.match({
      Add: (v) => new WatcherChange('Add', { entityId: v.entityId.clone(), subscriptionId: v.subscriptionId.clone(), queryId: v.queryId.clone() }),
      Remove: (v) => new WatcherChange('Remove', { entityId: v.entityId.clone(), subscriptionId: v.subscriptionId.clone(), queryId: v.queryId.clone() }),
    });
  }

  debug(): string {
    return this.match({
      Add: (v) => `Add { entityId: ${v.entityId}, subscriptionId: ${v.subscriptionId.debug()}, queryId: ${v.queryId} }`,
      Remove: (v) => `Remove { entityId: ${v.entityId}, subscriptionId: ${v.subscriptionId.debug()}, queryId: ${v.queryId} }`,
    });
  }
}

