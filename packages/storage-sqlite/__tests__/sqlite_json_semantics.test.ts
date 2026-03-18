// MIRRORS: ankurah/storage/sqlite/tests/sqlite_json_semantics.rs
//
// SQLite JSON Comparison Semantics Tests
//
// These tests verify and document SQLite's json_extract() comparison behavior.
// They run raw SQL against a real SQLite instance to:
// 1. Validate our understanding of json_extract() semantics
// 2. Catch any changes in future SQLite versions
// 3. Document the behavior we're relying on for JSON query pushdown
//
// Key behaviors verified:
// - Numeric comparisons via json_extract are numeric (not lexicographic)
// - String comparisons are lexicographic
// - Cross-type comparisons behavior
// - Float/int comparisons work correctly within numeric family

import { describe, test, expect } from 'bun:test';
import { bunSqliteDriver } from './common.ts';

/** Helper to run a SQL query that returns a boolean (0 or 1 in SQLite) */
function queryBool(sql: string): boolean {
  const driver = bunSqliteDriver();
  const result = driver.queryOne<{ value: number }>(`SELECT (${sql}) as value`);
  driver.close();
  return result !== null && result.value !== 0;
}

describe('sqlite_json_semantics', () => {
  test('test_json_extract_numeric_comparison_is_numeric', () => {
    // This test verifies that json_extract numeric comparison is numeric, NOT lexicographic.
    // If this were lexicographic, "9" > "10" would be true (because "9" > "1").
    // With proper numeric comparison, 9 > 10 is false.

    // 9 > 10 should be FALSE (numeric comparison)
    let result = queryBool("json_extract('{\"n\": 9}', '$.n') > json_extract('{\"n\": 10}', '$.n')");
    expect(result).toBe(false);

    // 9 < 10 should be TRUE
    result = queryBool("json_extract('{\"n\": 9}', '$.n') < json_extract('{\"n\": 10}', '$.n')");
    expect(result).toBe(true);

    // 100 > 9 should be TRUE
    result = queryBool("json_extract('{\"n\": 100}', '$.n') > json_extract('{\"n\": 9}', '$.n')");
    expect(result).toBe(true);
  });

  test('test_json_extract_string_comparison_is_lexicographic', () => {
    // json_extract string comparisons are lexicographic (as expected for strings)

    // "9" > "10" lexicographically (because '9' > '1')
    let result = queryBool(`json_extract('{"s": "9"}', '$.s') > json_extract('{"s": "10"}', '$.s')`);
    expect(result).toBe(true);

    // "abc" < "abd"
    result = queryBool(`json_extract('{"s": "abc"}', '$.s') < json_extract('{"s": "abd"}', '$.s')`);
    expect(result).toBe(true);
  });

  test('test_json_extract_cross_type_comparison', () => {
    // Cross-type comparisons in SQLite json_extract
    // SQLite's behavior differs from PostgreSQL JSONB here - it does type coercion

    // Number 9 compared to string "9" - SQLite may coerce
    const result1 = queryBool(`json_extract('{"n": 9}', '$.n') = json_extract('{"s": "9"}', '$.s')`);
    // Document actual behavior (SQLite coerces, so this may be true)
    console.log(`Number 9 = String '9': ${result1}`);

    // Number 9 compared to boolean true
    const result2 = queryBool(`json_extract('{"n": 9}', '$.n') = json_extract('{"b": true}', '$.b')`);
    expect(result2).toBe(false);
  });

  test('test_json_extract_float_int_comparison', () => {
    // Float and int comparisons should work correctly within the numeric family

    // 9 should equal 9.0
    let result = queryBool("json_extract('{\"n\": 9}', '$.n') = json_extract('{\"n\": 9.0}', '$.n')");
    expect(result).toBe(true);

    // 9.5 > 9 should be true
    result = queryBool("json_extract('{\"n\": 9.5}', '$.n') > json_extract('{\"n\": 9}', '$.n')");
    expect(result).toBe(true);

    // 9 < 9.1 should be true
    result = queryBool("json_extract('{\"n\": 9}', '$.n') < json_extract('{\"n\": 9.1}', '$.n')");
    expect(result).toBe(true);
  });

  test('test_json_extract_null_comparison', () => {
    // SQLite JSON null comparisons
    // Note: json_extract returns SQL NULL for JSON null, which has SQL NULL semantics

    // JSON null extracted becomes SQL NULL, and NULL = NULL is NULL (falsy)
    const result1 = queryBool("json_extract('{\"n\": null}', '$.n') IS NULL");
    expect(result1).toBe(true);

    // null should not equal 0
    const result2 = queryBool("COALESCE(json_extract('{\"n\": null}', '$.n') = 0, 0)");
    expect(result2).toBe(false);
  });

  test('test_json_extract_path_with_comparison', () => {
    // Test that our actual query pattern works correctly
    // This simulates: json_extract(data, '$.count') > 10

    // JSON count 9 > 10 should be false
    let result = queryBool(`json_extract('{"count": 9}', '$.count') > 10`);
    expect(result).toBe(false);

    // JSON count 100 > 10 should be true
    result = queryBool(`json_extract('{"count": 100}', '$.count') > 10`);
    expect(result).toBe(true);

    // String comparison
    result = queryBool(`json_extract('{"status": "active"}', '$.status') = 'active'`);
    expect(result).toBe(true);

    // Nested path extraction
    result = queryBool(`json_extract('{"user": {"name": "alice"}}', '$.user.name') = 'alice'`);
    expect(result).toBe(true);
  });
});
