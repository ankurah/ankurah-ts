// Tests for property/value/entity_ref.ts — mirrors ankurah/core/src/property/value/entity_ref.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { EntityId } from '@ankurah/proto';
import { Ref } from '../src/property/value/entity_ref.ts';
import { PropertyError } from '../src/property/traits.ts';
import type { Value } from '../src/value/index.ts';

describe('property/value/entity_ref', () => {
  test('ref_roundtrip', () => {
    const id = EntityId.new();
    const r = Ref.new(id);

    const value = r.intoValue();
    expect(value.type).toBe('EntityId');

    const recovered = Ref.fromValue(value);
    expect(recovered.entityId().equals(id)).toBe(true);
  });

  test('ref_from_entity_id', () => {
    const id = EntityId.new();
    const r = Ref.fromEntityId(id);
    expect(r.entityId().equals(id)).toBe(true);
  });

  test('ref_into_entity_id', () => {
    const id = EntityId.new();
    const r = Ref.new(id);
    const recovered = r.toEntityId();
    expect(recovered.equals(id)).toBe(true);
  });

  test('ref_missing', () => {
    expect(() => Ref.fromValue(null)).toThrow(PropertyError);
    try {
      Ref.fromValue(null);
    } catch (e) {
      expect(e).toBeInstanceOf(PropertyError);
      expect((e as PropertyError).kind).toBe('Missing');
    }
  });

  test('ref_invalid_string', () => {
    // Invalid base64 string should throw PropertyError (backwards compat path tries to parse)
    const value: Value = { type: 'String', value: 'not an id' };
    expect(() => Ref.fromValue(value)).toThrow(PropertyError);
    try {
      Ref.fromValue(value);
    } catch (e) {
      expect(e).toBeInstanceOf(PropertyError);
      expect((e as PropertyError).kind).toBe('InvalidValue');
    }
  });

  test('ref_invalid_variant', () => {
    // Completely wrong type should throw PropertyError with InvalidVariant
    const value: Value = { type: 'I64', value: 42 };
    expect(() => Ref.fromValue(value)).toThrow(PropertyError);
    try {
      Ref.fromValue(value);
    } catch (e) {
      expect(e).toBeInstanceOf(PropertyError);
      expect((e as PropertyError).kind).toBe('InvalidVariant');
    }
  });
});
