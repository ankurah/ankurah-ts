// MIRRORS: ankurah/core/src/collation.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { F64_isMaximum, F64_isMinimum, F64_predecessorBytes, F64_successorBytes, F64_toBytes, I64_isMaximum, I64_isMinimum, I64_predecessorBytes, I64_successorBytes, RangeBound, Str_isMaximum, Str_isMinimum, Str_predecessorBytes, Str_successorBytes, Str_toBytes } from './collation';
import { dropOwned, unsupported } from '@ankurah/base';
import { Literal } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

describe('collation unit tests', () => {
  test('test_string_collation', () => {
    const s = 'hello';
    if (!((Str_successorBytes(s) ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })()) > Str_toBytes(s))) throw new Error('assertion failed');
    if (!((Str_predecessorBytes(s) ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })()) < Str_toBytes(s))) throw new Error('assertion failed');
    if (!(!Str_isMinimum(s))) throw new Error('assertion failed');
    if (!(!Str_isMaximum(s))) throw new Error('assertion failed');
    const empty = '';
    if (!(Str_isMinimum(empty))) throw new Error('assertion failed');
    if (!((Str_predecessorBytes(empty) == null))) throw new Error('assertion failed');
  });

  test('test_integer_collation', () => {
    const n = 42n;
    expect(unsupported('`i64::from_be_bytes` is a function Rust puts on a primitive type, and the port writes that type as a JavaScript primitive, which has no members and no spelling for this one')).toEqual(43n);
    expect(unsupported('`i64::from_be_bytes` is a function Rust puts on a primitive type, and the port writes that type as a JavaScript primitive, which has no members and no spelling for this one')).toEqual(41n);
    if (!(!I64_isMinimum(n))) throw new Error('assertion failed');
    if (!(!I64_isMaximum(n))) throw new Error('assertion failed');
    if (!((I64_successorBytes(9223372036854775807n) == null))) throw new Error('assertion failed');
    if (!((I64_predecessorBytes(-9223372036854775808n) == null))) throw new Error('assertion failed');
    if (!(I64_isMaximum(9223372036854775807n))) throw new Error('assertion failed');
    if (!(I64_isMinimum(-9223372036854775808n))) throw new Error('assertion failed');
  });

  test('test_float_collation', () => {
    const f = 1.0;
    if (!((F64_successorBytes(f) ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })()) > F64_toBytes(f))) throw new Error('assertion failed');
    if (!((F64_predecessorBytes(f) ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })()) < F64_toBytes(f))) throw new Error('assertion failed');
    if (!(!F64_isMinimum(f))) throw new Error('assertion failed');
    if (!(!F64_isMaximum(f))) throw new Error('assertion failed');
    if (!(F64_isMaximum(Infinity))) throw new Error('assertion failed');
    if (!(F64_isMinimum(-Infinity))) throw new Error('assertion failed');
    if (!((F64_successorBytes(Infinity) == null))) throw new Error('assertion failed');
    if (!((F64_predecessorBytes(-Infinity) == null))) throw new Error('assertion failed');
    const nan = NaN;
    if (!((F64_successorBytes(nan) == null))) throw new Error('assertion failed');
    if (!((F64_predecessorBytes(nan) == null))) throw new Error('assertion failed');
  });

  test('test_range_bounds', () => {
    const n = 42n;
    let _moved1 = false;
    const _b0 = new RangeBound('Included', { _0: 40 });
    try {
      const _b2 = new RangeBound('Included', { _0: 45 });
      _moved1 = true;
      if (!(I64_isInRange(n, _b0, _b2))) throw new Error('assertion failed');
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
    let _moved4 = false;
    const _b3 = new RangeBound('Included', { _0: 42 });
    try {
      const _b5 = new RangeBound('Included', { _0: 45 });
      _moved4 = true;
      if (!(I64_isInRange(n, _b3, _b5))) throw new Error('assertion failed');
    } finally {
      if (!_moved4) dropOwned(_b3);
    }
    let _moved7 = false;
    const _b6 = new RangeBound('Included', { _0: 40 });
    try {
      const _b8 = new RangeBound('Included', { _0: 42 });
      _moved7 = true;
      if (!(I64_isInRange(n, _b6, _b8))) throw new Error('assertion failed');
    } finally {
      if (!_moved7) dropOwned(_b6);
    }
    let _moved10 = false;
    const _b9 = new RangeBound('Excluded', { _0: 40 });
    try {
      const _b11 = new RangeBound('Excluded', { _0: 43 });
      _moved10 = true;
      if (!(I64_isInRange(n, _b9, _b11))) throw new Error('assertion failed');
    } finally {
      if (!_moved10) dropOwned(_b9);
    }
    let _moved13 = false;
    const _b12 = new RangeBound('Excluded', { _0: 42 });
    try {
      const _b14 = new RangeBound('Excluded', { _0: 43 });
      _moved13 = true;
      if (!(!I64_isInRange(n, _b12, _b14))) throw new Error('assertion failed');
    } finally {
      if (!_moved13) dropOwned(_b12);
    }
    let _moved16 = false;
    const _b15 = new RangeBound('Included', { _0: 42 });
    try {
      const _b17 = new RangeBound('Excluded', { _0: 43 });
      _moved16 = true;
      if (!(I64_isInRange(n, _b15, _b17))) throw new Error('assertion failed');
    } finally {
      if (!_moved16) dropOwned(_b15);
    }
    let _moved19 = false;
    const _b18 = new RangeBound('Excluded', { _0: 41 });
    try {
      const _b20 = new RangeBound('Excluded', { _0: 42 });
      _moved19 = true;
      if (!(!I64_isInRange(n, _b18, _b20))) throw new Error('assertion failed');
    } finally {
      if (!_moved19) dropOwned(_b18);
    }
    let _moved22 = false;
    const _b21 = new RangeBound('Unbounded', {});
    try {
      const _b23 = new RangeBound('Included', { _0: 45 });
      _moved22 = true;
      if (!(I64_isInRange(n, _b21, _b23))) throw new Error('assertion failed');
    } finally {
      if (!_moved22) dropOwned(_b21);
    }
    let _moved25 = false;
    const _b24 = new RangeBound('Included', { _0: 40 });
    try {
      const _b26 = new RangeBound('Unbounded', {});
      _moved25 = true;
      if (!(I64_isInRange(n, _b24, _b26))) throw new Error('assertion failed');
    } finally {
      if (!_moved25) dropOwned(_b24);
    }
    let _moved28 = false;
    const _b27 = new RangeBound('Unbounded', {});
    try {
      const _b29 = new RangeBound('Unbounded', {});
      _moved28 = true;
      if (!(I64_isInRange(n, _b27, _b29))) throw new Error('assertion failed');
    } finally {
      if (!_moved28) dropOwned(_b27);
    }
  });

  test('test_literal_i16_collation', () => {
    const lit = new ast.Literal('I16', { _0: 100 });
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const maxLit = new ast.Literal('I16', { _0: 32767 });
    const minLit = new ast.Literal('I16', { _0: -32768 });
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
  });

  test('test_literal_i32_collation', () => {
    const lit = new ast.Literal('I32', { _0: 1000 });
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const maxLit = new ast.Literal('I32', { _0: 2147483647 });
    const minLit = new ast.Literal('I32', { _0: -2147483648 });
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
  });

  test('test_literal_entity_id_collation', () => {
    const ulid = Ulid.new();
    const lit = new ast.Literal('EntityId', { _0: ulid });
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const minUlid = Ulid.fromBytes(Array(16).fill(0));
    const minLit = new ast.Literal('EntityId', { _0: minUlid });
    if (!(minLit.isMinimum())) throw new Error('assertion failed');
    if (!(minLit.predecessorBytes() == null)) throw new Error('assertion failed');
    const maxUlid = Ulid.fromBytes(Array(16).fill(255));
    const maxLit = new ast.Literal('EntityId', { _0: maxUlid });
    if (!(maxLit.isMaximum())) throw new Error('assertion failed');
    if (!(maxLit.successorBytes() == null)) throw new Error('assertion failed');
  });

  test('test_literal_binary_collation', () => {
    const lit = new ast.Literal('Binary', { _0: [1, 2, 3] });
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const emptyLit = new ast.Literal('Binary', { _0: [] });
    if (!(emptyLit.isMinimum())) throw new Error('assertion failed');
    if (!(emptyLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(!emptyLit.isMaximum())) throw new Error('assertion failed');
  });

  test('test_literal_object_collation', () => {
    const lit = new ast.Literal('Object', { _0: [10, 20, 30] });
    if (!(lit.successorBytes() > lit.toBytes())) throw new Error('assertion failed');
    if (!(lit.predecessorBytes() < lit.toBytes())) throw new Error('assertion failed');
    if (!(!lit.isMinimum())) throw new Error('assertion failed');
    if (!(!lit.isMaximum())) throw new Error('assertion failed');
    const emptyLit = new ast.Literal('Object', { _0: [] });
    if (!(emptyLit.isMinimum())) throw new Error('assertion failed');
    if (!(emptyLit.predecessorBytes() == null)) throw new Error('assertion failed');
    if (!(!emptyLit.isMaximum())) throw new Error('assertion failed');
  });

});
