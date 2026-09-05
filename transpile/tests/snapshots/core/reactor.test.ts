// MIRRORS: ankurah/core/src/reactor.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Reactor } from './reactor';
import { AnyhowError, Arc, HashMap, Mutex, Result, Struct, dropOwned } from '@ankurah/base';
import { MembershipChange, ReactorUpdate, ReactorUpdateItem } from './reactor/update';
import { EntityResultSet } from './resultset';
import { CollectionId, QueryId } from '@ankurah/proto';

class TestEntity extends Struct implements Filterable, AbstractEntity {
  id: EntityId;
  collection: CollectionId;
  state: Arc<Mutex<HashMap<string, string>>>;

  constructor(id: EntityId, collection: CollectionId, state: Arc<Mutex<HashMap<string, string>>>) {
    super();
    this.id = id;
    this.collection = collection;
    this.state = state;
  }

  static new(name: string, status: string): TestEntity {
    return new TestEntity(proto.EntityId.new(), proto.CollectionId.fixedName('album'), Arc.new(new Mutex(HashMap.from([['name', name], ['status', status]]))));
  }

  equals(other: TestEntity): boolean {
    return this.id === other.id;
  }

  compareTo(other: TestEntity): number {
    return this.id.compareTo(other.id);
  }

  collection(): string {
    return this.collection.asStr();
  }

  value(field: string): Value | null {
    const _t0 = this.state.value.lock();
    try {
      return _t0.value.get(field) != null ? (Value.String)(_t0.value.get(field)!) : null;
    } finally {
      _t0.drop();
    }
  }

  id(): EntityId {
    return this.id;
  }

  clone(): TestEntity {
    return new TestEntity(this.id.clone(), this.collection.clone(), this.state.clone());
  }

  debug(): string {
    return `TestEntity { id: ${this.id}, collection: ${this.collection}, state: ${this.state} }`;
  }
}

class TestEvent extends Struct {
  id: EventId;
  collection: CollectionId;
  changes: HashMap<string, string>;

  constructor(id: EventId, collection: CollectionId, changes: HashMap<string, string>) {
    super();
    this.id = id;
    this.collection = collection;
    this.changes = changes;
  }

  equals(other: TestEvent): boolean {
    if (!this.id.equals(other.id)) return false;
    if (!this.collection.equals(other.collection)) return false;
    { if (this.changes.size !== other.changes.size) return false; for (const [k, v] of this.changes) { if (!other.changes.has(k)) return false; const _w = other.changes.get(k)!; if (_w !== v) return false; } }
    return true;
  }

  clone(): TestEvent {
    return new TestEvent(this.id.clone(), this.collection.clone(), this.changes.clone());
  }

  debug(): string {
    return `TestEvent { id: ${this.id}, collection: ${this.collection}, changes: ${this.changes} }`;
  }
}

class MockGapFetcher extends Struct implements GapFetcher<TestEntity> {
  entities: TestEntity[];

  constructor(entities: TestEntity[]) {
    super();
    this.entities = entities;
  }

  static new(): MockGapFetcher {
    return new MockGapFetcher([]);
  }

  static withEntities(entities: TestEntity[]): MockGapFetcher {
    return new MockGapFetcher(entities);
  }

  async fetchGap(_collectionId: CollectionId, _selection: Selection, _lastEntity: TestEntity | null, _gapSize: number): Promise<Result<TestEntity[], RetrievalError>> {
    return Result.Ok(this.entities.clone());
  }
}

class MockNode extends Struct implements TNodeErased<TestEntity> {
  entities: TestEntity[];

  constructor(entities: TestEntity[]) {
    super();
    this.entities = entities;
  }

  unsubscribeRemotePredicate(_queryId: QueryId): void {

  }

  updateRemoteQuery(_queryId: QueryId, _selection: Selection, _version: number): Result<void, AnyhowError> {
    try {
      return Result.Ok([]);
    } finally {
      _selection.drop();
    }
  }

  async fetchEntitiesFromLocal(_collectionId: CollectionId, _selection: Selection): Promise<Result<TestEntity[], RetrievalError>> {
    return Result.Ok(this.entities.clone());
  }

  reactor(): Reactor<TestEntity, Attested<Event>> {
    throw new Error('MockNode::reactor() should not be called in this test');
  }

  hasSubscriptionRelay(): boolean {
    return false;
  }
}

describe('reactor unit tests', () => {
  function watcher(): [(arg0: T) => void, () => T[]] {
    const values = Arc.new(new Mutex([]));
    const accumulate = ((values) => {
      return (value) => {
        values.lock().push(value);
      };
    })(values.clone());
    const check = () => values.lock().drain(undefined /* range .. */);
    return [accumulate, check];
  }

  test('test_entity_remains_watched_after_predicate_stops_matching', async () => {
    const reactor = Reactor.new();
    try {
      const rsub = reactor.subscribe();
      try {
        const [w, check] = watcher();
        const _guard = rsub.subscribe(w);
        try {
          const queryId = QueryId.new();
          const collectionId = CollectionId.fixedName('album');
          const selection = 'status = \'pending\''.tryInto();
          const entity1 = TestEntity.new('Test Album', 'pending');
          try {
            const resultset = EntityResultSet.empty();
            const mockGapFetcher = Arc.new(MockGapFetcher.new());
            const mockNode = new MockNode([entity1.clone()]);
            try {
              await reactor.addQueryAndNotify(rsub.id(), queryId, collectionId, selection, mockNode, resultset, mockGapFetcher, []).unwrap();
              const _t0 = [new ReactorUpdate([new ReactorUpdateItem(entity1.clone(), [], [[queryId, new MembershipChange('Initial', {})]])])];
              try {
                expect(check()).toEqual(_t0);
              } finally {
                dropOwned(_t0);
              }
            } finally {
              mockNode.drop();
            }
          } finally {
            entity1.drop();
          }
        } finally {
          _guard.drop();
        }
      } finally {
        rsub.drop();
      }
    } finally {
      reactor.drop();
    }
  });

});
