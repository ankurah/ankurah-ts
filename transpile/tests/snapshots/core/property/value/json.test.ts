// MIRRORS: ankurah/core/src/property/value/json.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Json } from './json';
import { Value } from '../../value/index';

describe('json unit tests', () => {
  test('test_json_roundtrip', () => {
    const original = Json.object([['name', 'test'], ['count', 42], ['nested', undefined /* json!({ "inner" : "value" }) */]]);
    try {
      const value = original.intoValue().unwrap();
      const recovered = Json.fromValue(value).unwrap();
      try {
        expect(original).toEqual(recovered);
      } finally {
        recovered.drop();
      }
    } finally {
      original.drop();
    }
  });

  test('test_json_get_path', () => {
    const json = Json.new(undefined /* json!({ "licensing" : { "territory" : "US" , "rights" : { "holder" : "Label" } } }) */);
    try {
      expect(json.getPath(['licensing', 'territory'])).toEqual('US');
      expect(json.getPath(['licensing', 'rights', 'holder'])).toEqual('Label');
      expect(json.getPath(['licensing', 'nonexistent'])).toEqual(null);
      expect(json.getPath(['nonexistent'])).toEqual(null);
    } finally {
      json.drop();
    }
  });

  test('test_json_null', () => {
    const json = Json.null();
    try {
      if (!(json.isNull())) throw new Error('assertion failed');
      const value = json.intoValue().unwrap();
      const recovered = Json.fromValue(value).unwrap();
      try {
        if (!(recovered.isNull())) throw new Error('assertion failed');
      } finally {
        recovered.drop();
      }
    } finally {
      json.drop();
    }
  });

  test('test_json_missing', () => {
    const result = Json.fromValue(null);
    try {
      if (!(((result) => {
        if (!(result.isErr())) return false;
        const _v = result.unwrapErr();
        return true;
      })(result))) throw new Error('assertion failed');
    } finally {
      result.drop();
    }
  });

  test('test_json_invalid_variant', () => {
    const result = Json.fromValue(new Value('String', { _0: 'not json bytes' }));
    try {
      if (!(((result) => {
        if (!(result.isErr())) return false;
        const _v = result.unwrapErr();
        return true;
      })(result))) throw new Error('assertion failed');
    } finally {
      result.drop();
    }
  });

  test('test_json_deref', () => {
    const json = Json.new(undefined /* json!({ "key" : "value" }) */);
    try {
      if (!(json.isObject())) throw new Error('assertion failed');
      expect(((json.deref() as Record<string, unknown>)?.['key'] ?? null)).toEqual('value');
    } finally {
      json.drop();
    }
  });

});
