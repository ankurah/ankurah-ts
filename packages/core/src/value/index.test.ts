// MIRRORS: ankurah/core/src/value/mod.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Value } from './mod';
import { dropOwned } from '@ankurah/base';
import { Json } from '../property/value/json';

describe('mod unit tests', () => {
  test('test_extract_at_path_empty', () => {
    const value = new Value('String', { _0: 'hello' });
    try {
      const result = value.extractAtPath([]);
      try {
        expect(result).toEqual(new Value('String', { _0: 'hello' }));
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

  test('test_extract_at_path_json_string', () => {
    const json = undefined /* json!({ "session_id" : "sess123" }) */;
    const value = new Value('Json', { _0: json });
    try {
      const result = value.extractAtPath(['session_id']);
      try {
        expect(result).toEqual(new Value('String', { _0: 'sess123' }));
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

  test('test_extract_at_path_json_number', () => {
    const json = undefined /* json!({ "count" : 42 }) */;
    const value = new Value('Json', { _0: json });
    try {
      const result = value.extractAtPath(['count']);
      try {
        expect(result).toEqual(new Value('I64', { _0: 42n }));
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

  test('test_extract_at_path_json_nested', () => {
    const json = undefined /* json!({ "context" : { "user" : { "name" : "Alice" } } }) */;
    const value = new Value('Json', { _0: json });
    try {
      const result = value.extractAtPath(['context', 'user', 'name']);
      try {
        expect(result).toEqual(new Value('String', { _0: 'Alice' }));
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

  test('test_extract_at_path_missing', () => {
    const json = undefined /* json!({ "session_id" : "sess123" }) */;
    const value = new Value('Json', { _0: json });
    try {
      const result = value.extractAtPath(['nonexistent']);
      try {
        expect(result).toEqual(null);
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

  test('test_extract_at_path_non_json', () => {
    const value = new Value('String', { _0: 'not json' });
    try {
      const result = value.extractAtPath(['field']);
      try {
        expect(result).toEqual(null);
      } finally {
        dropOwned(result);
      }
    } finally {
      value.drop();
    }
  });

});
