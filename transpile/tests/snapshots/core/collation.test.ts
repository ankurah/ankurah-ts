// MIRRORS: ankurah/core/src/collation.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { RangeBound } from './collation';
import { Literal } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

describe('collation unit tests', () => {
  test('test_string_collation', () => {
    const s = 'hello';
    if (!(Str_successorBytes(s) > Str_toBytes(s))) throw new Error('assertion failed');
    if (!(Str_predecessorBytes(s) < Str_toBytes(s))) throw new Error('assertion failed');
    if (!(!Str_isMinimum(s))) throw new Error('assertion failed');
    if (!(!Str_isMaximum(s))) throw new Error('assertion failed');
    const empty = '';
    if (!(Str_isMinimum(empty))) throw new Error('assertion failed');
    if (!(Str_predecessorBytes(empty) == null)) throw new Error('assertion failed');
  });

  test('test_integer_collation', () => {
    const n = 42n;
    expect(i64.fromBeBytes(I64_successorBytes(n).tryInto())).toEqual(43n);
    expect(i64.fromBeBytes(I64_predecessorBytes(n).tryInto())).toEqual(41n);
    if (!(!I64_isMinimum(n))) throw new Error('assertion failed');
    if (!(!I64_isMaximum(n))) throw new Error('assertion failed');
    if (!(i64.MAX.successorBytes() == null)) throw new Error('assertion failed');
    if (!(i64.MIN.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(i64.MAX.isMaximum())) throw new Error('assertion failed');
    if (!(i64.MIN.isMinimum())) throw new Error('assertion failed');
  });

  test('test_float_collation', () => {
    const f = 1.0;
    if (!(F64_successorBytes(f) > F64_toBytes(f))) throw new Error('assertion failed');
    if (!(F64_predecessorBytes(f) < F64_toBytes(f))) throw new Error('assertion failed');
    if (!(!F64_isMinimum(f))) throw new Error('assertion failed');
    if (!(!F64_isMaximum(f))) throw new Error('assertion failed');
    if (!(f64.INFINITY.isMaximum())) throw new Error('assertion failed');
    if (!(f64.NEG_INFINITY.isMinimum())) throw new Error('assertion failed');
    if (!(f64.INFINITY.successorBytes() == null)) throw new Error('assertion failed');
    if (!(f64.NEG_INFINITY.predecessorBytes() == null)) throw new Error('assertion failed');
    const nan = f64.NAN;
    if (!(nan.successorBytes() == null)) throw new Error('assertion failed');
    if (!(nan.predecessorBytes() == null)) throw new Error('assertion failed');
  });

  test('test_range_bounds', () => {
    const n = 42n;
    if (!(I64_isInRange(n, new RangeBound('Included', { _0: 40 }), new RangeBound('Included', { _0: 45 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Included', { _0: 42 }), new RangeBound('Included', { _0: 45 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Included', { _0: 40 }), new RangeBound('Included', { _0: 42 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Excluded', { _0: 40 }), new RangeBound('Excluded', { _0: 43 })))) throw new Error('assertion failed');
    if (!(!I64_isInRange(n, new RangeBound('Excluded', { _0: 42 }), new RangeBound('Excluded', { _0: 43 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Included', { _0: 42 }), new RangeBound('Excluded', { _0: 43 })))) throw new Error('assertion failed');
    if (!(!I64_isInRange(n, new RangeBound('Excluded', { _0: 41 }), new RangeBound('Excluded', { _0: 42 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Unbounded', {}), new RangeBound('Included', { _0: 45 })))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Included', { _0: 40 }), new RangeBound('Unbounded', {})))) throw new Error('assertion failed');
    if (!(I64_isInRange(n, new RangeBound('Unbounded', {}), new RangeBound('Unbounded', {})))) throw new Error('assertion failed');
  });

  test('test_literal_i16_collation', () => {
    const lit = ast.Literal.I16(100);
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const maxLit = ast.Literal.I16(i16.MAX);
    const minLit = ast.Literal.I16(i16.MIN);
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
  });

  test('test_literal_i32_collation', () => {
    const lit = ast.Literal.I32(1000);
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const maxLit = ast.Literal.I32(i32.MAX);
    const minLit = ast.Literal.I32(i32.MIN);
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
  });

  test('test_literal_entity_id_collation', () => {
    const ulid = Ulid.new();
    const lit = ast.Literal.EntityId(ulid);
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const minUlid = Ulid.fromBytes(Array(16).fill(0));
    const minLit = ast.Literal.EntityId(minUlid);
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    const maxUlid = Ulid.fromBytes(Array(16).fill(255));
    const maxLit = ast.Literal.EntityId(maxUlid);
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
  });

  test('test_literal_binary_collation', () => {
    const lit = ast.Literal.Binary([1, 2, 3]);
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const emptyLit = ast.Literal.Binary([]);
    if (!(emptyLit.isMinimum())) throw new Error('assertion failed');
    if (!(emptyLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(!emptyLit.isMaximum())) throw new Error('assertion failed');
  });

  test('test_literal_object_collation', () => {
    const lit = ast.Literal.Object([10, 20, 30]);
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const emptyLit = ast.Literal.Object([]);
    if (!(emptyLit.isMinimum())) throw new Error('assertion failed');
    if (!(emptyLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(!emptyLit.isMaximum())) throw new Error('assertion failed');
  });

});
