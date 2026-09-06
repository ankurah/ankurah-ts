// MIRRORS: ankurah/core/src/value/cast_predicate.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { castPredicateTypes } from './cast_predicate';
import { Result, Struct, dropOwned } from '@ankurah/base';
import { Comparison } from '../lineage';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

class TestSchema extends Struct implements CollectionSchema {

  fieldType(path: PathExpr): Result<ValueType, PropertyError> {
    const propertyName = path.property();
    if (propertyName === 'id') {
      return Result.Ok(new ValueType('EntityId', {}));
    } else {
      return Result.Ok(new ValueType('String', {}));
    }
  }
}

describe('cast_predicate unit tests', () => {
  test('test_cast_id_field_string_to_entity_id', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('id') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    {
      const _v1 = castPredicate;
      if (_v1.is('Comparison')) {
        const { right } = _v1.value;
        try {
          {
            const _v = right;
            if (_v.is('Literal') && (_v.value._0.is('EntityId'))) {
              const { _0: ulid } = _v.value._0.value;
              expect(EntityId.fromUlid(ulid)).toEqual(entityId);
            } else {
            _v.drop();
            throw new Error(`Expected EntityId literal, got ${right.debug()}`);
          }
          }
        } finally {
          dropOwned(right);
        }
      } else {
      _v1.drop();
      throw new Error('Expected Comparison predicate');
    }
    }
  });

  test('test_cast_literal_equals_field', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('Comparison', { left: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Path', { _0: PathExpr.simple('id') }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    {
      const _v1 = castPredicate;
      if (_v1.is('Comparison')) {
        const { left } = _v1.value;
        try {
          {
            const _v = left;
            if (_v.is('Literal') && (_v.value._0.is('EntityId'))) {
              const { _0: ulid } = _v.value._0.value;
              expect(EntityId.fromUlid(ulid)).toEqual(entityId);
            } else {
            _v.drop();
            throw new Error(`Expected EntityId literal, got ${left.debug()}`);
          }
          }
        } finally {
          dropOwned(left);
        }
      } else {
      _v1.drop();
      throw new Error('Expected Comparison predicate');
    }
    }
  });

  test('test_cast_complex_predicate', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('And', { _0: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('id') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }) }), _1: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'test' }) }) }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    {
      const _v4 = castPredicate;
      if (_v4.is('And')) {
        const { _0: leftPred, _1: rightPred } = _v4.value;
        try {
          try {
            {
              const _v1 = leftPred;
              if (_v1.is('Comparison')) {
                const { right } = _v1.value;
                try {
                  {
                    const _v = right;
                    if (_v.is('Literal') && (_v.value._0.is('EntityId'))) {
                      const { _0: ulid } = _v.value._0.value;
                      expect(EntityId.fromUlid(ulid)).toEqual(entityId);
                    } else {
                    _v.drop();
                    throw new Error('Expected EntityId literal for id field');
                  }
                  }
                } finally {
                  dropOwned(right);
                }
              } else {
              _v1.drop();
            }
            }
            {
              const _v3 = rightPred;
              if (_v3.is('Comparison')) {
                const { right } = _v3.value;
                try {
                  {
                    const _v2 = right;
                    if (_v2.is('Literal') && (_v2.value._0.is('String'))) {
                      const { _0: s } = _v2.value._0.value;
                      expect(s).toEqual('test');
                    } else {
                    _v2.drop();
                    throw new Error('Expected String literal for name field');
                  }
                  }
                } finally {
                  dropOwned(right);
                }
              } else {
              _v3.drop();
            }
            }
          } finally {
            dropOwned(rightPred);
          }
        } finally {
          dropOwned(leftPred);
        }
      } else {
      _v4.drop();
      throw new Error('Expected And predicate');
    }
    }
  });

});
