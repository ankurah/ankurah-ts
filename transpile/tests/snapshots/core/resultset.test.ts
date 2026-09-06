// MIRRORS: ankurah/core/src/resultset.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EntityResultSet, IVec } from './resultset';
import { HashMap, Struct, range, unsupported, valueEquals } from '@ankurah/base';
import { IndexDirection, IndexKeyPart, KeySpec, NullsOrder } from './indexing/key_spec';
import { Value, ValueType } from './value/index';

class TestEntity extends Struct implements AbstractEntity {
  id: EntityId;
  collection: CollectionId;
  properties: HashMap<string, Value>;

  constructor(id: EntityId, collection: CollectionId, properties: HashMap<string, Value>) {
    super();
    this.id = id;
    this.collection = collection;
    this.properties = properties;
  }

  static new(id: number, properties: HashMap<string, Value>): TestEntity {
    let idBytes = Array(16).fill(0);
    idBytes[15] = id;
    return new TestEntity(EntityId.fromBytes(idBytes), CollectionId.fixedName('test'), properties);
  }

  collection(): CollectionId {
    return this.collection.clone();
  }

  id(): EntityId {
    return this.id;
  }

  value(field: string): Value | null {
    if (field === 'id') {
      return new Value('EntityId', { _0: this.id.clone() });
    } else {
      return this.properties.get(field);
    }
  }

  clone(): TestEntity {
    return new TestEntity(this.id.clone(), this.collection.clone(), this.properties.clone());
  }

  debug(): string {
    return `TestEntity { id: ${this.id}, collection: ${this.collection.debug()}, properties: ${`{${Array.from(this.properties).map(($p) => `${JSON.stringify($p[0])}: ${$p[1].debug()}`).join(', ')}}`} }`;
  }
}

describe('resultset unit tests', () => {
  test('test_entity_id_ordering', () => {
    const resultset = EntityResultSet.empty();
    try {
      let write = resultset.write();
      const entity1 = TestEntity.new(1, new HashMap<string, Value>());
      try {
        const entity2 = TestEntity.new(2, new HashMap<string, Value>());
        try {
          const entity3 = TestEntity.new(3, new HashMap<string, Value>());
          try {
            write.add(entity3.clone());
            write.add(entity1.clone());
            write.add(entity2.clone());
            write.drop();
            const readGuard = resultset.read();
            try {
              const entities = readGuard.iterEntities();
              expect(entities.length).toEqual(3);
              expect(entities[0]._0).toEqual(entity1.id);
              expect(entities[1]._0).toEqual(entity2.id);
              expect(entities[2]._0).toEqual(entity3.id);
            } finally {
              readGuard.drop();
            }
          } finally {
            entity3.drop();
          }
        } finally {
          entity2.drop();
        }
      } finally {
        entity1.drop();
      }
    } finally {
      resultset.drop();
    }
  });

  test('test_order_by_with_tie_breaking', () => {
    const resultset = EntityResultSet.empty();
    try {
      let props1 = new HashMap();
      props1.insert('name', new Value('String', { _0: 'Alice' }));
      const entity1 = TestEntity.new(1, props1);
      try {
        let props2 = new HashMap();
        props2.insert('name', new Value('String', { _0: 'Alice' }));
        const entity2 = TestEntity.new(2, props2);
        try {
          let props3 = new HashMap();
          props3.insert('name', new Value('String', { _0: 'Bob' }));
          const entity3 = TestEntity.new(3, props3);
          try {
            const keySpec = new KeySpec([new IndexKeyPart('name', null, new IndexDirection('Asc', {}), new ValueType('String', {}), new NullsOrder('Last', {}), null)]);
            resultset.orderBy(keySpec);
            let write = resultset.write();
            write.add(entity2.clone());
            write.add(entity3.clone());
            write.add(entity1.clone());
            write.drop();
            const readGuard = resultset.read();
            try {
              const entities = readGuard.iterEntities();
              expect(entities.length).toEqual(3);
              expect(entities[0]._0).toEqual(entity1.id);
              expect(entities[1]._0).toEqual(entity2.id);
              expect(entities[2]._0).toEqual(entity3.id);
            } finally {
              readGuard.drop();
            }
          } finally {
            entity3.drop();
          }
        } finally {
          entity2.drop();
        }
      } finally {
        entity1.drop();
      }
    } finally {
      resultset.drop();
    }
  });

  test('test_limit_functionality', () => {
    const resultset = EntityResultSet.empty();
    try {
      let write = resultset.write();
      for (const i of range(0, 5)) {
        let props = new HashMap();
        props.insert('value', new Value('I32', { _0: (i | 0) }));
        const entity = TestEntity.new(i, props);
        write.add(entity);
      }
      write.drop();
      expect(resultset.len()).toEqual(5);
      resultset.limit(3);
      expect(resultset.len()).toEqual(3);
      resultset.limit(null);
      expect(resultset.len()).toEqual(3);
    } finally {
      resultset.drop();
    }
  });

  test('test_dirty_tracking', () => {
    const resultset = EntityResultSet.empty();
    try {
      let props = new HashMap();
      props.insert('active', new Value('Bool', { _0: true }));
      const entity1 = TestEntity.new(1, props);
      try {
        let props_1 = new HashMap();
        props_1.insert('active', new Value('Bool', { _0: false }));
        const entity2 = TestEntity.new(2, props_1);
        try {
          let write = resultset.write();
          write.add(entity1.clone());
          write.add(entity2.clone());
          write.markAllDirty();
          const removed = write.retainDirty((entity) => valueEquals(entity.value('active'), new Value('Bool', { _0: true })));
          write.drop();
          expect(removed.length).toEqual(1);
          expect(removed[0]).toEqual(entity2.id);
          expect(resultset.len()).toEqual(1);
          const _t0 = resultset.read();
          try {
            expect((unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance') ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })())[0]).toEqual(entity1.id);
          } finally {
            _t0.drop();
          }
        } finally {
          entity2.drop();
        }
      } finally {
        entity1.drop();
      }
    } finally {
      resultset.drop();
    }
  });

  test('test_write_guard_atomic_operations', () => {
    const resultset = EntityResultSet.empty();
    try {
      (() => {
        let write = resultset.write();
        try {
          const entity1 = TestEntity.new(1, new HashMap<string, Value>());
          const entity2 = TestEntity.new(2, new HashMap<string, Value>());
          write.add(entity1);
          write.add(entity2);
          expect(write.iterEntities().length).toEqual(2);
        } finally {
          write.drop();
        }
      })();
      expect(resultset.len()).toEqual(2);
    } finally {
      resultset.drop();
    }
  });

  test('test_ivec_small_keys', () => {
    const smallKey = IVec.fromSlice(new Uint8Array([104, 101, 108, 108, 111]));
    try {
      const anotherSmall = IVec.fromSlice(new Uint8Array([119, 111, 114, 108, 100]));
      try {
        const emptyKey = IVec.fromSlice(new Uint8Array([]));
        try {
          if (!(smallKey.compareTo(anotherSmall) < 0)) throw new Error('assertion failed');
          if (!(emptyKey.compareTo(smallKey) < 0)) throw new Error('assertion failed');
          const keyAb = IVec.fromSlice(new Uint8Array([97, 98]));
          try {
            const keyAbc = IVec.fromSlice(new Uint8Array([97, 98, 99]));
            try {
              if (!(keyAb.compareTo(keyAbc) < 0)) throw new Error('assertion failed');
            } finally {
              keyAbc.drop();
            }
          } finally {
            keyAb.drop();
          }
        } finally {
          emptyKey.drop();
        }
      } finally {
        anotherSmall.drop();
      }
    } finally {
      smallKey.drop();
    }
  });

  test('test_ivec_large_keys', () => {
    const largeKey = IVec.fromSlice(Array(20).fill(1));
    try {
      const smallKey = IVec.fromSlice(Array(10).fill(1));
      try {
        if (!(smallKey.compareTo(largeKey) < 0)) throw new Error('assertion failed');
      } finally {
        smallKey.drop();
      }
    } finally {
      largeKey.drop();
    }
  });

  test('test_ivec_boundary', () => {
    const exactly16 = IVec.fromSlice(Array(16).fill(1));
    try {
      const exactly17 = IVec.fromSlice(Array(17).fill(1));
      try {
        if (!(exactly16.compareTo(exactly17) < 0)) throw new Error('assertion failed');
        exactly16.match({
          Small: (v) => {
            [];
          },
          Large: (v) => {
            throw new Error('16-byte key should use Small variant');
          },
        });
        return exactly17.match({
          Large: (v) => {
            [];
          },
          Small: (v) => {
            throw new Error('17-byte key should use Large variant');
          },
        });
      } finally {
        exactly17.drop();
      }
    } finally {
      exactly16.drop();
    }
  });

});
