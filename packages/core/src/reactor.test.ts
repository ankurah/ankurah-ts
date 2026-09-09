// MIRRORS: ankurah/core/src/reactor.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Reactor } from './reactor';
import { AnyhowError, Arc, HashMap, Invocable, Mutex, OwnedClosure, Result, Struct, debugString, dropOwned, invokeRef, unsupported, valueEquals } from '@ankurah/base';
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
    return valueEquals(this.id, other.id);
  }

  partialCompareTo(other: TestEntity): number | null {
    return this.id.compareTo(other.id);
  }

  collection(): string {
    return this.collection.asStr();
  }

  value(field: string): Value | null {
    const _t0 = this.state.value.lock();
    try {
      const _m1 = _t0.value.get(field);
      return (_m1 != null ? (Value.String)(_m1!) : null);
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
    { if (this.changes.size !== other.changes.size) return false; for (const [k, v] of this.changes) { if (!other.changes.has(k)) return false; const _w = other.changes.get(k)!; if (v !== _w) return false; } }
    return true;
  }

  clone(): TestEvent {
    return new TestEvent(this.id.clone(), this.collection.clone(), this.changes.clone());
  }

  debug(): string {
    return `TestEvent { id: ${this.id}, collection: ${this.collection}, changes: ${`{${Array.from(this.changes).map(($p) => `${debugString($p[0])}: ${debugString($p[1])}`).join(', ')}}`} }`;
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
    return Result.Ok(this.entities.map((e) => e.clone()));
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
    return Result.Ok(this.entities.map((e) => e.clone()));
  }

  reactor(): Reactor<TestEntity, Attested<Event>> {
    throw new Error('MockNode::reactor() should not be called in this test');
  }

  hasSubscriptionRelay(): boolean {
    return false;
  }
}

describe('reactor unit tests', () => {
  function watcher<T extends Clone>(): [Invocable<[T], void>, Invocable<[], T[]>] {
    const values = Arc.new(new Mutex([]));
    const accumulate = ((values) => {
      return new OwnedClosure([values], (value: T) => {
        const _t0 = values.value.lock();
        try {
          _t0.value.push(value);
        } finally {
          _t0.drop();
        }
      });
    })(values.clone());
    const check = new OwnedClosure([values], () => {
      const _t1 = values.value.lock();
      try {
        return unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into');
      } finally {
        _t1.drop();
      }
    });
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
          let _moved0 = false;
          const collectionId = CollectionId.fixedName('album');
          try {
            let _moved1 = false;
            const selection = 'status = \'pending\''.tryInto();
            try {
              const entity1 = TestEntity.new('Test Album', 'pending');
              try {
                let _moved2 = false;
                const resultset = EntityResultSet.empty();
                try {
                  let _moved3 = false;
                  const mockGapFetcher = Arc.new(MockGapFetcher.new());
                  try {
                    const mockNode = new MockNode([entity1.clone()]);
                    try {
                      const _b4 = rsub.id();
                      _moved0 = true;
                      _moved1 = true;
                      _moved2 = true;
                      _moved3 = true;
                      (await reactor.addQueryAndNotify(_b4, queryId, collectionId, selection, mockNode, resultset, mockGapFetcher, [])).unwrap();
                      let _moved6 = false;
                      const _b5 = entity1.clone();
                      try {
                        const _b7 = [[queryId, new MembershipChange('Initial', {})]];
                        const _t8 = [new ReactorUpdate([new ReactorUpdateItem(_b5, [], _b7)])];
                        try {
                          _moved6 = true;
                          expect(invokeRef(check)).toEqual(_t8);
                        } finally {
                          dropOwned(_t8);
                        }
                      } finally {
                        if (!_moved6) dropOwned(_b5);
                      }
                    } finally {
                      mockNode.drop();
                    }
                  } finally {
                    if (!_moved3) mockGapFetcher.drop();
                  }
                } finally {
                  if (!_moved2) resultset.drop();
                }
              } finally {
                entity1.drop();
              }
            } finally {
              if (!_moved1) selection.drop();
            }
          } finally {
            if (!_moved0) collectionId.drop();
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
