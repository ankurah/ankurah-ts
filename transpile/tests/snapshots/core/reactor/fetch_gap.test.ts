// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { buildContinuationPredicate, inferValueTypeForField } from './fetch_gap';
import { Arc, HashMap, Mutex, Struct, dropOwned } from '@ankurah/base';
import { Value, ValueType } from '../value/index';
import { OrderByItem, OrderDirection, PathExpr, Predicate } from '@ankurah/ankql';

class TestEntity extends Struct implements AbstractEntity {
  id: EntityId;
  collection: CollectionId;
  data: Arc<Mutex<HashMap<string, Value>>>;

  constructor(id: EntityId, collection: CollectionId, data: Arc<Mutex<HashMap<string, Value>>>) {
    super();
    this.id = id;
    this.collection = collection;
    this.data = data;
  }

  static new(id: number, data: HashMap<string, Value>): TestEntity {
    let idBytes = Array(16).fill(0);
    idBytes[15] = id;
    return new TestEntity(proto.EntityId.fromBytes(idBytes), proto.CollectionId.fixedName('test'), Arc.new(new Mutex(data)));
  }

  collection(): CollectionId {
    return this.collection.clone();
  }

  id(): EntityId {
    return this.id;
  }

  value(field: string): Value | null {
    const _t0 = this.data.value.lock();
    try {
      return _t0.value.get(field);
    } finally {
      _t0.drop();
    }
  }

  clone(): TestEntity {
    return new TestEntity(this.id.clone(), this.collection.clone(), this.data.clone());
  }

  debug(): string {
    return `TestEntity { id: ${this.id.debug()}, collection: ${this.collection.debug()}, data: ${this.data} }`;
  }
}

describe('fetch_gap unit tests', () => {
  test('test_build_gap_predicate_single_column_asc', () => {
    const entity = TestEntity.new(1, new Map([['name', new Value('String', { _0: 'John' })]]));
    try {
      const originalPredicate = new Predicate('True', {});
      try {
        const orderBy = [new OrderByItem(PathExpr.simple('name'), new OrderDirection('Asc', {}))];
        try {
          const gapPredicate = buildContinuationPredicate(originalPredicate, orderBy, entity).unwrap();
          try {
            const expected = undefined /* selection!("true AND name >= 'John' AND id != {}" , entity . id ()) */.predicate;
            expect(gapPredicate).toEqual(expected);
          } finally {
            gapPredicate.drop();
          }
        } finally {
          dropOwned(orderBy);
        }
      } finally {
        originalPredicate.drop();
      }
    } finally {
      entity.drop();
    }
  });

  test('test_build_gap_predicate_multi_column', () => {
    const entity = TestEntity.new(2, new Map([['name', new Value('String', { _0: 'John' })], ['age', new Value('I32', { _0: 30 })]]));
    try {
      const originalPredicate = new Predicate('True', {});
      try {
        const orderBy = [new OrderByItem(PathExpr.simple('name'), new OrderDirection('Asc', {})), new OrderByItem(PathExpr.simple('age'), new OrderDirection('Desc', {}))];
        try {
          const gapPredicate = buildContinuationPredicate(originalPredicate, orderBy, entity).unwrap();
          try {
            const expected = undefined /* selection!("true AND name >= 'John' AND age <= 30 AND id != {}" , entity . id ()) */.predicate;
            expect(gapPredicate).toEqual(expected);
          } finally {
            gapPredicate.drop();
          }
        } finally {
          dropOwned(orderBy);
        }
      } finally {
        originalPredicate.drop();
      }
    } finally {
      entity.drop();
    }
  });

  test('test_infer_value_type_for_field', () => {
    const entities = [TestEntity.new(1, new Map([['name', new Value('String', { _0: 'Alice' })]])), TestEntity.new(2, new Map([['age', new Value('I32', { _0: 25 })]]))];
    try {
      expect(inferValueTypeForField(entities, 'name')).toEqual(new ValueType('String', {}));
      expect(inferValueTypeForField(entities, 'age')).toEqual(new ValueType('I32', {}));
      expect(inferValueTypeForField(entities, 'nonexistent')).toEqual(new ValueType('String', {}));
    } finally {
      dropOwned(entities);
    }
  });

});
