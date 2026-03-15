// MIRRORS: ankurah/core/src/reactor/comparison_index.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { QueryId } from '@ankurah/proto';
import { Literal, ComparisonOperator } from '@ankurah/ankql';
import { ComparisonIndex } from './comparison_index.ts';
import type { Value } from '../value/index.ts';

// ── Helpers ──

/** Sort QueryIds by bytes (matches Rust BTreeSet<QueryId> ordering). */
function sortByBytes(ids: QueryId[]): QueryId[] {
  return ids.slice().sort((a, b) => {
    for (let i = 0; i < 16; i++) {
      if (a.bytes[i] < b.bytes[i]) return -1;
      if (a.bytes[i] > b.bytes[i]) return 1;
    }
    return 0;
  });
}

function findSorted(index: ComparisonIndex<QueryId>, value: Value): QueryId[] {
  return sortByBytes(index.findMatching(value));
}

function idsEqual(actual: QueryId[], expected: QueryId[]): void {
  expect(actual.length).toBe(expected.length);
  for (let i = 0; i < actual.length; i++) {
    expect(actual[i].toUlidString()).toBe(expected[i].toUlidString());
  }
}

// ── Tests ──

describe('comparison_index', () => {
  // Rust: fn test_field_index()
  test('field index', () => {
    const index = new ComparisonIndex<QueryId>();

    // Less than 8
    const sub0 = QueryId.test(0n);
    index.add(Literal.I64(8n), ComparisonOperator.LessThan(), sub0);

    // 8 should match nothing
    idsEqual(findSorted(index, { type: 'I64', value: 8 }), []);

    // 7 should match sub0
    idsEqual(findSorted(index, { type: 'I64', value: 7 }), [sub0]);

    const sub1 = QueryId.test(1n);

    // Greater than 20
    index.add(Literal.I64(20n), ComparisonOperator.GreaterThan(), sub1);

    // 20 should match nothing
    idsEqual(findSorted(index, { type: 'I64', value: 20 }), []);

    // 21 should match sub1
    idsEqual(findSorted(index, { type: 'I64', value: 21 }), [sub1]);

    // Add subscriptions for exact match
    index.add(Literal.I64(5n), ComparisonOperator.Equal(), sub0);

    // Test exact match (5)
    idsEqual(findSorted(index, { type: 'I64', value: 5 }), [sub0]);

    // Less than 25
    index.add(Literal.I64(25n), ComparisonOperator.LessThan(), sub0);

    // 22 should match sub0 (< 25) and sub1 (> 20)
    idsEqual(findSorted(index, { type: 'I64', value: 22 }), [sub0, sub1]);

    // 25 should match sub1 because > 20
    idsEqual(findSorted(index, { type: 'I64', value: 25 }), [sub1]);

    // 26 should match sub1 because > 20
    idsEqual(findSorted(index, { type: 'I64', value: 26 }), [sub1]);
  });

  // Rust: fn test_field_index_not_equal()
  test('field index not equal', () => {
    const index = new ComparisonIndex<QueryId>();

    const sub0 = QueryId.test(0n);
    index.add(Literal.I64(8n), ComparisonOperator.NotEqual(), sub0);

    idsEqual(findSorted(index, { type: 'I64', value: 8 }), []);
    idsEqual(findSorted(index, { type: 'I64', value: 9 }), [sub0]);
  });
});
