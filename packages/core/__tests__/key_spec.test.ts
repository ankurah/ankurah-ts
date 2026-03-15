// MIRRORS: ankurah/core/src/indexing/key_spec.rs #[cfg(test)]
import { describe, test, expect } from 'bun:test';
import {
  IndexDirection,
  IndexSpecMatch,
  indexKeyPartAsc,
  indexKeyPartDesc,
  keySpecMatches,
} from '../src/indexing/key_spec.ts';
import { ValueType } from '../src/value/index.ts';
import type { KeySpec } from '../src/indexing/key_spec.ts';

describe('key_spec', () => {
  // Rust: fn test_exact_match()
  test('exact match', () => {
    const spec1: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const spec2: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };

    expect(keySpecMatches(spec1, spec2)).toBe(IndexSpecMatch.Match);
  });

  // Rust: fn test_prefix_match()
  test('prefix match', () => {
    // +a, -b matches +a, -b, +c
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const indexSpec: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };

    expect(keySpecMatches(querySpec, indexSpec)).toBe(IndexSpecMatch.Match);
  });

  // Rust: fn test_inverse_exact_match()
  test('inverse exact match', () => {
    // +a, -b matches -a, +b (inverse)
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const indexSpec: KeySpec = { keyparts: [indexKeyPartDesc('a', ValueType.String), indexKeyPartAsc('b', ValueType.String)] };

    expect(keySpecMatches(querySpec, indexSpec)).toBe(IndexSpecMatch.Inverse);
  });

  // Rust: fn test_inverse_prefix_match()
  test('inverse prefix match', () => {
    // +a, -b matches -a, +b, +c (inverse prefix)
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const indexSpec: KeySpec = {
      keyparts: [
        indexKeyPartDesc('a', ValueType.String),
        indexKeyPartAsc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };

    expect(keySpecMatches(querySpec, indexSpec)).toBe(IndexSpecMatch.Inverse);
  });

  // Rust: fn test_user_example()
  test('user example', () => {
    // "+a, -b matches +a, -b, any c AND -a, +b, any c"
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };

    // Test direct match: +a, -b, +c
    const indexSpec1: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };
    expect(keySpecMatches(querySpec, indexSpec1)).toBe(IndexSpecMatch.Match);

    // Test inverse match: -a, +b, -c
    const indexSpec2: KeySpec = {
      keyparts: [
        indexKeyPartDesc('a', ValueType.String),
        indexKeyPartAsc('b', ValueType.String),
        indexKeyPartDesc('c', ValueType.String),
      ],
    };
    expect(keySpecMatches(querySpec, indexSpec2)).toBe(IndexSpecMatch.Inverse);
  });

  // Rust: fn test_no_match_different_fields()
  test('no match different fields', () => {
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const indexSpec: KeySpec = { keyparts: [indexKeyPartAsc('x', ValueType.String), indexKeyPartDesc('y', ValueType.String)] };

    expect(keySpecMatches(querySpec, indexSpec)).toBeNull();
  });

  // Rust: fn test_no_match_partial_field_overlap()
  test('no match partial field overlap', () => {
    // +a, -b does not match +a, +b (different direction on second field)
    const querySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartDesc('b', ValueType.String)] };
    const indexSpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String), indexKeyPartAsc('b', ValueType.String)] };

    expect(keySpecMatches(querySpec, indexSpec)).toBeNull();
  });

  // Rust: fn test_no_match_query_longer_than_index()
  test('no match query longer than index', () => {
    // +a, -b, +c cannot match +a (query is longer than index)
    const querySpec: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };
    const indexSpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String)] };

    expect(keySpecMatches(querySpec, indexSpec)).toBeNull();
  });

  // Rust: fn test_empty_specs()
  test('empty specs', () => {
    const emptySpec: KeySpec = { keyparts: [] };
    const nonEmptySpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String)] };

    // Empty spec matches any spec (empty prefix)
    expect(keySpecMatches(emptySpec, nonEmptySpec)).toBe(IndexSpecMatch.Match);
    expect(keySpecMatches(emptySpec, emptySpec)).toBe(IndexSpecMatch.Match);

    // Non-empty spec does not match empty spec
    expect(keySpecMatches(nonEmptySpec, emptySpec)).toBeNull();
  });

  // Rust: fn test_single_field_cases()
  test('single field cases', () => {
    const ascSpec: KeySpec = { keyparts: [indexKeyPartAsc('a', ValueType.String)] };
    const descSpec: KeySpec = { keyparts: [indexKeyPartDesc('a', ValueType.String)] };

    // Direct match
    expect(keySpecMatches(ascSpec, ascSpec)).toBe(IndexSpecMatch.Match);

    // Inverse match
    expect(keySpecMatches(ascSpec, descSpec)).toBe(IndexSpecMatch.Inverse);
    expect(keySpecMatches(descSpec, ascSpec)).toBe(IndexSpecMatch.Inverse);
  });

  // Rust: fn test_complex_multi_field_scenarios()
  test('complex multi field scenarios', () => {
    // Test various combinations with 3+ fields
    const querySpec: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };

    // Exact match with additional fields
    const indexSpec1: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
        indexKeyPartDesc('d', ValueType.String),
      ],
    };
    expect(keySpecMatches(querySpec, indexSpec1)).toBe(IndexSpecMatch.Match);

    // Inverse match with additional fields
    const indexSpec2: KeySpec = {
      keyparts: [
        indexKeyPartDesc('a', ValueType.String),
        indexKeyPartAsc('b', ValueType.String),
        indexKeyPartDesc('c', ValueType.String),
        indexKeyPartAsc('d', ValueType.String),
      ],
    };
    expect(keySpecMatches(querySpec, indexSpec2)).toBe(IndexSpecMatch.Inverse);

    // No match - mixed directions that don't form inverse
    const indexSpec3: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartAsc('b', ValueType.String),
        indexKeyPartDesc('c', ValueType.String),
      ],
    };
    expect(keySpecMatches(querySpec, indexSpec3)).toBeNull();
  });

  // Rust: fn test_helper_methods()
  test('helper methods', () => {
    // Test IndexKeyPart helper methods
    const ascKeypart = indexKeyPartAsc('test', ValueType.String);
    expect(ascKeypart.column).toBe('test');
    expect(ascKeypart.direction).toBe(IndexDirection.Asc);
    expect(ascKeypart.nulls).toBeNull();
    expect(ascKeypart.collation).toBeNull();

    const descKeypart = indexKeyPartDesc('test', ValueType.String);
    expect(descKeypart.column).toBe('test');
    expect(descKeypart.direction).toBe(IndexDirection.Desc);
    expect(descKeypart.nulls).toBeNull();
    expect(descKeypart.collation).toBeNull();
  });

  // Rust: fn test_edge_case_behaviors()
  test('edge case behaviors', () => {
    // Test that matches works correctly with various edge cases
    const spec: KeySpec = {
      keyparts: [
        indexKeyPartAsc('a', ValueType.String),
        indexKeyPartDesc('b', ValueType.String),
        indexKeyPartAsc('c', ValueType.String),
      ],
    };

    // Self-match should always be Match
    expect(keySpecMatches(spec, spec)).toBe(IndexSpecMatch.Match);

    // Empty spec matches any spec
    const empty: KeySpec = { keyparts: [] };
    expect(keySpecMatches(empty, spec)).toBe(IndexSpecMatch.Match);
  });
});
