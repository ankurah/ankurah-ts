// MIRRORS: ankurah/storage/indexeddb-wasm/src/idb_value.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { MAX_SAFE_INTEGER, MIN_SAFE_INTEGER } from './idb_value';
import { checkedSub } from '@ankurah/base';

describe('idb_value unit tests', () => {
  test('test_safe_integer_range', () => {
    expect(MAX_SAFE_INTEGER).toEqual(9007199254740991n);
    expect(MIN_SAFE_INTEGER).toEqual(-9007199254740991n);
    expect(MAX_SAFE_INTEGER).toEqual(checkedSub((BigInt.asIntN(64, (1n << 53n))), 1n, 'i64'));
    expect(MIN_SAFE_INTEGER).toEqual(-(checkedSub((BigInt.asIntN(64, (1n << 53n))), 1n, 'i64')));
  });

  test('test_timestamp_safety', () => {
    const now = 1700000000000n;
    if (!(now < MAX_SAFE_INTEGER)) throw new Error('assertion failed');
    const farFuture = 8900000000000000n;
    if (!(farFuture < MAX_SAFE_INTEGER)) throw new Error('assertion failed');
  });

});
