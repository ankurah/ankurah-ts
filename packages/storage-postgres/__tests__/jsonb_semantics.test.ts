// MIRRORS: ankurah/storage/postgres/tests/jsonb_semantics.rs
//
// PostgreSQL JSONB Comparison Semantics Tests
//
// These tests verify and document PostgreSQL's JSONB comparison behavior.
// They run raw SQL against a real PostgreSQL instance.
//
// Key behaviors verified:
// - Numeric JSONB comparisons are numeric (not lexicographic)
// - String JSONB comparisons are lexicographic
// - Cross-type comparisons return false (not error)
// - Float/int comparisons work correctly within numeric family

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import {
  createPostgresContainer,
  stopPostgresContainer,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

/// Helper to run a SQL query that returns a boolean
async function queryBool(sql: string): Promise<boolean> {
  const client = await pgCtx.pool.connect();
  try {
    const result = await client.query(sql);
    return result.rows[0][Object.keys(result.rows[0])[0]] as boolean;
  } finally {
    client.release();
  }
}

describe('jsonb_semantics', () => {
  // Rust: fn test_jsonb_numeric_comparison_is_numeric
  test('test_jsonb_numeric_comparison_is_numeric', async () => {
    // 9 > 10 should be FALSE (numeric comparison, not lexicographic)
    expect(await queryBool("SELECT '9'::jsonb > '10'::jsonb")).toBe(false);
    // 9 < 10 should be TRUE
    expect(await queryBool("SELECT '9'::jsonb < '10'::jsonb")).toBe(true);
    // 100 > 9 should be TRUE
    expect(await queryBool("SELECT '100'::jsonb > '9'::jsonb")).toBe(true);
  });

  // Rust: fn test_jsonb_string_comparison_is_lexicographic
  test('test_jsonb_string_comparison_is_lexicographic', async () => {
    // "9" > "10" lexicographically (because '9' > '1')
    expect(await queryBool(`SELECT '"9"'::jsonb > '"10"'::jsonb`)).toBe(true);
    // "abc" < "abd"
    expect(await queryBool(`SELECT '"abc"'::jsonb < '"abd"'::jsonb`)).toBe(true);
  });

  // Rust: fn test_jsonb_cross_type_comparison_returns_false
  test('test_jsonb_cross_type_comparison_returns_false', async () => {
    // Number 9 should NOT equal string "9"
    expect(await queryBool(`SELECT '9'::jsonb = '"9"'::jsonb`)).toBe(false);
    // Number 9 should NOT equal boolean true
    expect(await queryBool("SELECT '9'::jsonb = 'true'::jsonb")).toBe(false);
    // String "true" should NOT equal boolean true
    expect(await queryBool(`SELECT '"true"'::jsonb = 'true'::jsonb`)).toBe(false);
  });

  // Rust: fn test_jsonb_float_int_comparison
  test('test_jsonb_float_int_comparison', async () => {
    // 9 should equal 9.0
    expect(await queryBool("SELECT '9'::jsonb = '9.0'::jsonb")).toBe(true);
    // 9.5 > 9 should be true
    expect(await queryBool("SELECT '9.5'::jsonb > '9'::jsonb")).toBe(true);
    // 9 < 9.1 should be true
    expect(await queryBool("SELECT '9'::jsonb < '9.1'::jsonb")).toBe(true);
  });

  // Rust: fn test_jsonb_null_comparison
  test('test_jsonb_null_comparison', async () => {
    // JSONB null should equal JSONB null
    expect(await queryBool("SELECT 'null'::jsonb = 'null'::jsonb")).toBe(true);
    // JSONB null should not equal 0
    expect(await queryBool("SELECT 'null'::jsonb = '0'::jsonb")).toBe(false);
    // JSONB null should not equal empty string
    expect(await queryBool(`SELECT 'null'::jsonb = '""'::jsonb`)).toBe(false);
  });

  // Rust: fn test_jsonb_path_extraction_with_comparison
  test('test_jsonb_path_extraction_with_comparison', async () => {
    // data->'count' > '10'::jsonb pattern
    // JSON count 9 > 10 should be false
    expect(await queryBool(`SELECT ('{"count": 9}'::jsonb)->'count' > '10'::jsonb`)).toBe(false);
    // JSON count 100 > 10 should be true
    expect(await queryBool(`SELECT ('{"count": 100}'::jsonb)->'count' > '10'::jsonb`)).toBe(true);
    // JSON status = 'active' should match
    expect(await queryBool(`SELECT ('{"status": "active"}'::jsonb)->'status' = '"active"'::jsonb`)).toBe(true);
    // JSON number 9 should not equal JSON string '9'
    expect(await queryBool(`SELECT ('{"count": 9}'::jsonb)->'count' = '"9"'::jsonb`)).toBe(false);
  });
});
