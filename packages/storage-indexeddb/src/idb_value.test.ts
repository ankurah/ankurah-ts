// MIRRORS: ankurah/storage/indexeddb-wasm/src/idb_value.rs #[cfg(test)]

import { describe, expect, test } from 'bun:test';
import { MAX_SAFE_INTEGER, MIN_SAFE_INTEGER } from './idb_value.ts';

describe('IdbValue', () => {
  test('test_safe_integer_range', () => {
    // Verify our constants are correct
    expect(MAX_SAFE_INTEGER).toBe(9_007_199_254_740_991);
    expect(MIN_SAFE_INTEGER).toBe(-9_007_199_254_740_991);

    // Safe range is 2^53 - 1
    expect(MAX_SAFE_INTEGER).toBe(2 ** 53 - 1);
    expect(MIN_SAFE_INTEGER).toBe(-(2 ** 53 - 1));
  });

  test('test_timestamp_safety', () => {
    // Current Unix timestamp in milliseconds (2024)
    const now = 1700000000000;
    expect(now).toBeLessThan(MAX_SAFE_INTEGER);

    // Year 285,000 CE would still be safe
    const farFuture = 8_900_000_000_000_000;
    expect(farFuture).toBeLessThan(MAX_SAFE_INTEGER);
  });
});
