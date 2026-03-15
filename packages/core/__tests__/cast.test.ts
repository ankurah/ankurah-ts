// Tests for value/cast.ts — mirrors ankurah/core/src/value/cast.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { EntityId } from '@ankurah/proto';
import type { Value } from '../src/value/index.ts';
import { ValueType, valueEquals } from '../src/value/index.ts';
import { castTo, tryCastTo, CastErrorException } from '../src/value/cast.ts';

describe('value/cast', () => {
  test('string_to_entity_id', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const value: Value = { type: 'String', value: base64Str };

    const result = castTo(value, ValueType.EntityId);
    expect(result.type).toBe('EntityId');
    if (result.type === 'EntityId') {
      expect(result.value.equals(entityId)).toBe(true);
    }
  });

  test('entity_id_to_string', () => {
    const entityId = EntityId.new();
    const value: Value = { type: 'EntityId', value: entityId };

    const result = castTo(value, ValueType.String);
    expect(result.type).toBe('String');
    if (result.type === 'String') {
      expect(result.value).toBe(entityId.toBase64());
    }
  });

  test('invalid_entity_id_string', () => {
    const value: Value = { type: 'String', value: 'invalid-entity-id' };
    expect(() => castTo(value, ValueType.EntityId)).toThrow(CastErrorException);
    try {
      castTo(value, ValueType.EntityId);
    } catch (e) {
      expect(e).toBeInstanceOf(CastErrorException);
      expect((e as CastErrorException).castError.type).toBe('InvalidFormat');
    }
  });

  test('numeric_upcasting', () => {
    const value: Value = { type: 'I16', value: 42 };

    const i32Result = castTo(value, ValueType.I32);
    expect(valueEquals(i32Result, { type: 'I32', value: 42 })).toBe(true);

    const i64Result = castTo(value, ValueType.I64);
    expect(valueEquals(i64Result, { type: 'I64', value: 42 })).toBe(true);

    const f64Result = castTo(value, ValueType.F64);
    expect(valueEquals(f64Result, { type: 'F64', value: 42.0 })).toBe(true);
  });

  test('numeric_downcasting', () => {
    const value: Value = { type: 'I32', value: 42 };
    const result = castTo(value, ValueType.I16);
    expect(valueEquals(result, { type: 'I16', value: 42 })).toBe(true);

    const largeValue: Value = { type: 'I32', value: 100000 };
    expect(() => castTo(largeValue, ValueType.I16)).toThrow(CastErrorException);
    try {
      castTo(largeValue, ValueType.I16);
    } catch (e) {
      expect((e as CastErrorException).castError.type).toBe('NumericOverflow');
    }
  });

  test('string_to_numeric', () => {
    const value: Value = { type: 'String', value: '42' };

    expect(valueEquals(castTo(value, ValueType.I16), { type: 'I16', value: 42 })).toBe(true);
    expect(valueEquals(castTo(value, ValueType.I32), { type: 'I32', value: 42 })).toBe(true);
    expect(valueEquals(castTo(value, ValueType.I64), { type: 'I64', value: 42 })).toBe(true);
    expect(valueEquals(castTo(value, ValueType.F64), { type: 'F64', value: 42.0 })).toBe(true);
  });

  test('string_to_bool', () => {
    expect(valueEquals(castTo({ type: 'String', value: 'true' }, ValueType.Bool), { type: 'Bool', value: true })).toBe(true);
    expect(valueEquals(castTo({ type: 'String', value: 'false' }, ValueType.Bool), { type: 'Bool', value: false })).toBe(true);
    expect(valueEquals(castTo({ type: 'String', value: '1' }, ValueType.Bool), { type: 'Bool', value: true })).toBe(true);
    expect(valueEquals(castTo({ type: 'String', value: '0' }, ValueType.Bool), { type: 'Bool', value: false })).toBe(true);

    expect(() => castTo({ type: 'String', value: 'maybe' }, ValueType.Bool)).toThrow(CastErrorException);
    try {
      castTo({ type: 'String', value: 'maybe' }, ValueType.Bool);
    } catch (e) {
      expect((e as CastErrorException).castError.type).toBe('InvalidFormat');
    }
  });

  test('incompatible_types', () => {
    // Binary to I32 is truly incompatible
    const value: Value = { type: 'Binary', value: new Uint8Array([1, 2, 3]) };
    expect(() => castTo(value, ValueType.I32)).toThrow(CastErrorException);
    try {
      castTo(value, ValueType.I32);
    } catch (e) {
      expect((e as CastErrorException).castError.type).toBe('IncompatibleTypes');
    }
  });

  test('same_type_cast', () => {
    const value: Value = { type: 'I32', value: 42 };
    const result = castTo(value, ValueType.I32);
    expect(valueEquals(result, value)).toBe(true);
  });
});
