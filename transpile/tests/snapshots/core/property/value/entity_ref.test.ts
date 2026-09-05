// MIRRORS: ankurah/core/src/property/value/entity_ref.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Ref } from './entity_ref';
import { Struct } from '@ankurah/base';
import { Value } from '../../value/index';
import { EntityId } from '@ankurah/proto';

class TestModel extends Struct {
}

describe('entity_ref unit tests', () => {
  test('test_ref_roundtrip', () => {
    const id = EntityId.new();
    const r = new Ref(id.clone());
    const value = r.intoValue().unwrap();
    if (!(value.is('EntityId'))) throw new Error('assertion failed');
    const recovered = Ref.fromValue(value);
    try {
      expect(recovered.id()).toEqual(id);
    } finally {
      recovered.drop();
    }
  });

  test('test_ref_from_entity_id', () => {
    const id = EntityId.new();
    const r = Ref.fromEntityId(id.clone());
    try {
      expect(r.id()).toEqual(id);
    } finally {
      r.drop();
    }
  });

  test('test_ref_into_entity_id', () => {
    const id = EntityId.new();
    const r = new Ref(id.clone());
    const recovered = EntityId.fromRefT(r);
    expect(recovered).toEqual(id);
  });

  test('test_ref_missing', () => {
    const result = Ref.fromValue(null);
    if (!(((result) => {
      if (!(result.isErr())) return false;
      const _v = result.unwrapErr();
      return true;
    })(result))) throw new Error('assertion failed');
  });

  test('test_ref_invalid_string', () => {
    const result = Ref.fromValue(new Value('String', { _0: 'not an id' }));
    if (!(((result) => {
      if (!(result.isErr())) return false;
      const _v = result.unwrapErr();
      return true;
    })(result))) throw new Error('assertion failed');
  });

  test('test_ref_invalid_variant', () => {
    const result = Ref.fromValue(new Value('I64', { _0: 42n }));
    if (!(((result) => {
      if (!(result.isErr())) return false;
      const _v = result.unwrapErr();
      return true;
    })(result))) throw new Error('assertion failed');
  });

});
