// MIRRORS: ankurah/core/src/value/mod.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { extractAtPath, valueEquals } from './index.ts';
import type { Value } from './index.ts';

describe('extractAtPath', () => {
  // Rust: fn test_extract_at_path_empty()
  test('empty path returns self unchanged', () => {
    const value: Value = { type: 'String', value: 'hello' };
    const result = extractAtPath(value, []);
    expect(result).not.toBeNull();
    expect(valueEquals(result!, { type: 'String', value: 'hello' })).toBe(true);
  });

  // Rust: fn test_extract_at_path_json_string()
  test('json string field extraction', () => {
    const value: Value = { type: 'Json', value: { session_id: 'sess123' } };
    const result = extractAtPath(value, ['session_id']);
    expect(result).not.toBeNull();
    expect(valueEquals(result!, { type: 'String', value: 'sess123' })).toBe(true);
  });

  // Rust: fn test_extract_at_path_json_number()
  test('json number field extraction', () => {
    const value: Value = { type: 'Json', value: { count: 42 } };
    const result = extractAtPath(value, ['count']);
    expect(result).not.toBeNull();
    expect(valueEquals(result!, { type: 'I64', value: 42 })).toBe(true);
  });

  // Rust: fn test_extract_at_path_json_nested()
  test('json nested field extraction', () => {
    const value: Value = { type: 'Json', value: { context: { user: { name: 'Alice' } } } };
    const result = extractAtPath(value, ['context', 'user', 'name']);
    expect(result).not.toBeNull();
    expect(valueEquals(result!, { type: 'String', value: 'Alice' })).toBe(true);
  });

  // Rust: fn test_extract_at_path_missing()
  test('missing field returns null', () => {
    const value: Value = { type: 'Json', value: { session_id: 'sess123' } };
    const result = extractAtPath(value, ['nonexistent']);
    expect(result).toBeNull();
  });

  // Rust: fn test_extract_at_path_non_json()
  test('non-json with non-empty path returns null', () => {
    const value: Value = { type: 'String', value: 'not json' };
    // Non-empty path on non-JSON string returns null
    const result = extractAtPath(value, ['field']);
    expect(result).toBeNull();
  });
});
