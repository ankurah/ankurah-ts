// MIRRORS: ankurah/storage/indexeddb-wasm/tests/idb_value.rs

import { describe, expect, test } from 'bun:test';
import { valueToIdb, idbToValue, MAX_SAFE_INTEGER, MIN_SAFE_INTEGER } from '../src/idb_value.ts';
import type { Value } from '@ankurah/core';

describe('IdbValue integration', () => {
  test('test_i64_positive_safe_range_as_number', () => {
    // Positive values within safe range should be stored as number
    const small: Value = { type: 'I64', value: 100 };
    const jsVal = valueToIdb(small);
    expect(typeof jsVal).toBe('number');
    expect(jsVal).toBe(100);

    // At the boundary
    const atMax: Value = { type: 'I64', value: MAX_SAFE_INTEGER };
    const jsValMax = valueToIdb(atMax);
    expect(typeof jsValMax).toBe('number');
    expect(jsValMax).toBe(MAX_SAFE_INTEGER);
  });

  test('test_i64_positive_beyond_safe_as_string', () => {
    // Positive values beyond safe range should be stored as zero-padded strings
    const beyond: Value = { type: 'I64', value: MAX_SAFE_INTEGER + 1 };
    const jsVal = valueToIdb(beyond);
    expect(typeof jsVal).toBe('string');
    expect(jsVal).toBe('00009007199254740992');

    // Large value — i64 MAX approximation in JS (note: JS number can't hold i64::MAX exactly)
    // Using a value that's representable: 9223372036854775000 (won't lose precision as it's beyond safe int)
    const large: Value = { type: 'I64', value: 9_223_372_036_854_775_000 };
    const jsValLarge = valueToIdb(large);
    expect(typeof jsValLarge).toBe('string');
    // Note: JS can't represent this exactly, so we just check it's a padded string
    expect((jsValLarge as string).length).toBe(20);
  });

  test('test_i64_negative_always_number', () => {
    // Negative values are always stored as number
    const smallNeg: Value = { type: 'I64', value: -100 };
    const jsVal = valueToIdb(smallNeg);
    expect(typeof jsVal).toBe('number');
    expect(jsVal).toBe(-100);

    // At safe boundary
    const atMin: Value = { type: 'I64', value: MIN_SAFE_INTEGER };
    const jsValMin = valueToIdb(atMin);
    expect(typeof jsValMin).toBe('number');
    expect(jsValMin).toBe(MIN_SAFE_INTEGER);

    // Beyond safe range (accepts truncation)
    const beyondNeg: Value = { type: 'I64', value: MIN_SAFE_INTEGER - 1 };
    const jsValBeyond = valueToIdb(beyondNeg);
    expect(typeof jsValBeyond).toBe('number');
  });

  test('test_i64_string_roundtrip', () => {
    // Large positive i64 should be stored as string
    const original: Value = { type: 'I64', value: 9_223_372_036_854_775_000 };
    const jsVal = valueToIdb(original);

    // Should be stored as string
    expect(typeof jsVal).toBe('string');

    // Comes back as String, use casting to recover i64
    const recovered = idbToValue(jsVal);

    // Should be a String initially (type info lost for large values)
    expect(recovered.type).toBe('String');
  });

  test('test_i64_ordering_across_threshold', () => {
    // Values should maintain ordering across the number/string threshold
    const before: Value = { type: 'I64', value: MAX_SAFE_INTEGER };
    const after: Value = { type: 'I64', value: MAX_SAFE_INTEGER + 1 };

    const jsBefore = valueToIdb(before);
    const jsAfter = valueToIdb(after);

    // Before is a number
    expect(typeof jsBefore).toBe('number');
    // After is a string
    expect(typeof jsAfter).toBe('string');

    // IndexedDB guarantees: all numbers < all strings
    // So this maintains correct ordering: before < after
  });
});
