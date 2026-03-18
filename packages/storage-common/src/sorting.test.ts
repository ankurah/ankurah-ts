// MIRRORS: ankurah/storage/common/src/sorting.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { PathExpr, OrderByItem, OrderDirection } from '@ankurah/ankql';
import type { Filterable, Value } from '@ankurah/core';
import { OrderByComponents } from './types.ts';
import { sortedIterable, limitedIterable, topKIterable } from './sorting.ts';

// ── Test helpers ────────────────────────────────────────────────────

// Rust: fn collect_stream
async function collect<T>(iter: AsyncIterable<T>): Promise<T[]> {
  const result: T[] = [];
  for await (const item of iter) {
    result.push(item);
  }
  return result;
}

// Rust: fn stream_from
async function* iterFrom<T>(items: T[]): AsyncGenerator<T> {
  for (const item of items) {
    yield item;
  }
}

// Rust: struct TestItem
class TestItem implements Filterable {
  readonly values: Map<string, Value>;

  constructor(values: Map<string, Value>) {
    this.values = values;
  }

  // Rust: fn new
  static new(pairs: [string, Value][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, v);
    }
    return new TestItem(values);
  }

  // Rust: fn int
  static int(pairs: [string, number][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, { type: 'I32', value: v });
    }
    return new TestItem(values);
  }

  // Rust: fn str
  static str(pairs: [string, string][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, { type: 'String', value: v });
    }
    return new TestItem(values);
  }

  // Rust: fn mixed
  static mixed(cat: string, name: string): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('name', { type: 'String', value: name });
    return new TestItem(values);
  }

  // Rust: fn cat_val
  static catVal(cat: string, val: number): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('val', { type: 'I32', value: val });
    return new TestItem(values);
  }

  // Rust: fn cat_subcat_val
  static catSubcatVal(cat: string, subcat: string, val: number): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('subcat', { type: 'String', value: subcat });
    values.set('val', { type: 'I32', value: val });
    return new TestItem(values);
  }

  // Rust: fn collection
  collection(): string { return 'test'; }

  // Rust: fn value
  value(property: string): Value | null {
    return this.values.get(property) ?? null;
  }
}

// Rust: fn extract_i32
function extractI32(v: Value): number {
  if (v.type !== 'I32') throw new Error('expected I32');
  return v.value;
}

// Rust: fn extract_string
function extractString(v: Value): string {
  if (v.type !== 'String') throw new Error('expected String');
  return v.value;
}

// Rust: fn oby
function oby(col: string, dir: OrderDirection): OrderByItem {
  return new OrderByItem(PathExpr.simple(col), dir);
}

// Rust: fn oby_asc
function obyAsc(col: string): OrderByItem { return oby(col, OrderDirection.Asc()); }

// Rust: fn oby_desc
function obyDesc(col: string): OrderByItem { return oby(col, OrderDirection.Desc()); }

// ============================================================================
// LimitedStream Tests
// ============================================================================

describe('LimitedStream', () => {
  // Rust: fn test_limited_stream_basic
  test('basic', async () => {
    const items = [1, 2, 3, 4, 5];
    const limited = await collect(limitedIterable(iterFrom(items), 3));
    expect(limited).toEqual([1, 2, 3]);
  });

  // Rust: fn test_limited_stream_no_limit
  test('no_limit', async () => {
    const items = [1, 2, 3, 4, 5];
    const limited = await collect(limitedIterable(iterFrom(items), null));
    expect(limited).toEqual([1, 2, 3, 4, 5]);
  });

  // Rust: fn test_limited_stream_limit_exceeds_items
  test('limit_exceeds_items', async () => {
    const items = [1, 2, 3];
    const limited = await collect(limitedIterable(iterFrom(items), 10));
    expect(limited).toEqual([1, 2, 3]);
  });

  // Rust: fn test_limited_stream_zero_limit
  test('zero_limit', async () => {
    const items = [1, 2, 3];
    const limited = await collect(limitedIterable(iterFrom(items), 0));
    expect(limited).toEqual([]);
  });

  // Rust: fn test_limited_stream_empty_input
  test('empty_input', async () => {
    const items: number[] = [];
    const limited = await collect(limitedIterable(iterFrom(items), 5));
    expect(limited).toEqual([]);
  });
});

// ============================================================================
// SortedStream Tests - Global Sort (empty presort)
// ============================================================================

describe('SortedStream - Global Sort', () => {
  // Rust: fn test_sorted_stream_global_sort_asc
  test('asc', async () => {
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const values = sorted.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([1, 2, 3]);
  });

  // Rust: fn test_sorted_stream_global_sort_desc
  test('desc', async () => {
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 3]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyDesc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const values = sorted.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([3, 2, 1]);
  });

  // Rust: fn test_sorted_stream_global_sort_multi_column
  test('multi_column', async () => {
    const items = [TestItem.mixed('B', 'Z'), TestItem.mixed('A', 'Y'), TestItem.mixed('A', 'X'), TestItem.mixed('B', 'W')];

    const orderBy = new OrderByComponents([], [obyAsc('cat'), obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'W', 'Z']); // A-X, A-Y, B-W, B-Z
  });

  // Rust: fn test_sorted_stream_empty_input
  test('empty_input', async () => {
    const items: TestItem[] = [];
    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));
    expect(sorted).toEqual([]);
  });

  // Rust: fn test_sorted_stream_single_item
  test('single_item', async () => {
    const items = [TestItem.int([['x', 42]])];
    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));
    expect(sorted.length).toBe(1);
  });
});

// ============================================================================
// SortedStream Tests - Partition-Aware Sort (non-empty presort)
// ============================================================================

describe('SortedStream - Partition-Aware Sort', () => {
  // Rust: fn test_sorted_stream_partition_aware_basic
  test('basic', async () => {
    // Input is PRE-SORTED by presort column (category)
    const items = [TestItem.mixed('A', 'Z'), TestItem.mixed('A', 'X'), TestItem.mixed('B', 'Y'), TestItem.mixed('B', 'W')];

    // presort: cat ASC (already sorted), spill: name ASC
    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    // Within A: X, Z (sorted)
    // Within B: W, Y (sorted)
    expect(names).toEqual(['X', 'Z', 'W', 'Y']);
  });

  // Rust: fn test_sorted_stream_partition_aware_mixed_directions
  test('mixed_directions', async () => {
    // Input PRE-SORTED by category ASC
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Z'), TestItem.mixed('B', 'W'), TestItem.mixed('B', 'Y')];

    // presort: cat ASC, spill: name DESC
    const orderBy = new OrderByComponents([obyAsc('cat')], [obyDesc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    // Within A: Z, X (desc)
    // Within B: Y, W (desc)
    expect(names).toEqual(['Z', 'X', 'Y', 'W']);
  });

  // Rust: fn test_sorted_stream_partition_aware_single_partition
  test('single_partition', async () => {
    // All items in same partition
    const items = [TestItem.mixed('A', 'Z'), TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Y')];

    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'Z']);
  });

  // Rust: fn test_sorted_stream_partition_aware_single_item_partitions
  test('single_item_partitions', async () => {
    // Single item per partition
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('B', 'Y'), TestItem.mixed('C', 'Z')];

    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'Z']);
  });

  // Rust: fn test_sorted_stream_partition_aware_empty_spill
  test('empty_spill', async () => {
    // When spill is empty but presort is non-empty, just pass through
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Z'), TestItem.mixed('B', 'Y')];

    // presort non-empty, spill empty
    const orderBy = new OrderByComponents([obyAsc('cat')], []);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    // Items should maintain their order within partitions (no sorting needed)
    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Z', 'Y']);
  });
});

// ============================================================================
// TopKStream Tests - Global TopK (empty presort)
// ============================================================================

describe('TopKStream - Global TopK', () => {
  // Rust: fn test_topk_stream_global_basic
  test('basic', async () => {
    const items = [
      TestItem.int([['x', 5]]),
      TestItem.int([['x', 1]]),
      TestItem.int([['x', 3]]),
      TestItem.int([['x', 4]]),
      TestItem.int([['x', 2]]),
    ];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 3));

    const values = topk.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([1, 2, 3]); // Top 3 smallest
  });

  // Rust: fn test_topk_stream_global_desc
  test('desc', async () => {
    const items = [
      TestItem.int([['x', 5]]),
      TestItem.int([['x', 1]]),
      TestItem.int([['x', 3]]),
      TestItem.int([['x', 4]]),
      TestItem.int([['x', 2]]),
    ];

    const orderBy = new OrderByComponents([], [obyDesc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 3));

    const values = topk.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([5, 4, 3]); // Top 3 largest
  });

  // Rust: fn test_topk_stream_global_k_exceeds_items
  test('k_exceeds_items', async () => {
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 10));

    const values = topk.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([1, 3]);
  });

  // Rust: fn test_topk_stream_global_k_zero
  test('k_zero', async () => {
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 0));

    expect(topk).toEqual([]);
  });

  // Rust: fn test_topk_stream_global_empty_input
  test('empty_input', async () => {
    const items: TestItem[] = [];
    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 5));
    expect(topk).toEqual([]);
  });
});

// ============================================================================
// TopKStream Tests - Partition-Aware TopK (non-empty presort)
// ============================================================================

describe('TopKStream - Partition-Aware TopK', () => {
  // Rust: fn test_topk_stream_partition_aware_basic
  test('basic', async () => {
    // Input PRE-SORTED by category
    const items = [
      TestItem.catVal('A', 3),
      TestItem.catVal('A', 1),
      TestItem.catVal('A', 2),
      TestItem.catVal('B', 6),
      TestItem.catVal('B', 4),
      TestItem.catVal('B', 5),
    ];

    // presort: cat, spill: val ASC, LIMIT 4
    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('val')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 4));

    // Should get A's sorted: 1, 2, 3, then B's sorted: 4
    const values = topk.map((i) => extractI32(i.value('val')!));
    expect(values).toEqual([1, 2, 3, 4]);
  });

  // Rust: fn test_topk_stream_partition_aware_limit_within_partition
  test('limit_within_partition', async () => {
    // Input PRE-SORTED by category - A has 5 items, we only want 3
    const items = [
      TestItem.catVal('A', 5),
      TestItem.catVal('A', 1),
      TestItem.catVal('A', 3),
      TestItem.catVal('A', 2),
      TestItem.catVal('A', 4),
      TestItem.catVal('B', 10),
    ];

    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('val')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 3));

    // Should get A's smallest 3: 1, 2, 3
    const values = topk.map((i) => extractI32(i.value('val')!));
    expect(values).toEqual([1, 2, 3]);
  });

  // Rust: fn test_topk_stream_partition_aware_mixed_directions
  test('mixed_directions', async () => {
    // Input PRE-SORTED by category ASC
    const items = [
      TestItem.catVal('A', 1),
      TestItem.catVal('A', 3),
      TestItem.catVal('A', 2),
      TestItem.catVal('B', 4),
      TestItem.catVal('B', 6),
    ];

    // presort: cat ASC, spill: val DESC
    const orderBy = new OrderByComponents([obyAsc('cat')], [obyDesc('val')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 4));

    // A sorted desc: 3, 2, 1 - B sorted desc: 6
    const values = topk.map((i) => extractI32(i.value('val')!));
    expect(values).toEqual([3, 2, 1, 6]);
  });
});

// ============================================================================
// NULL Handling Tests
// ============================================================================

describe('NULL Handling', () => {
  // Rust: fn test_sorted_stream_null_sorts_first_asc
  test('null_sorts_first_asc', async () => {
    const items = [
      TestItem.int([['x', 2]]),
      TestItem.new([]), // x is NULL
      TestItem.int([['x', 1]]),
    ];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    // NULL sorts first, then 1, 2
    const values = sorted.map((i) => {
      const v = i.value('x');
      return v !== null ? extractI32(v) : null;
    });
    expect(values).toEqual([null, 1, 2]);
  });

  // Rust: fn test_sorted_stream_null_sorts_first_desc
  test('null_sorts_first_desc', async () => {
    // Note: Current implementation has NULLs sort first regardless of direction
    const items = [
      TestItem.int([['x', 2]]),
      TestItem.new([]), // x is NULL
      TestItem.int([['x', 1]]),
    ];

    const orderBy = new OrderByComponents([], [obyDesc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    // NULL sorts first even with DESC, then 2, 1
    const values = sorted.map((i) => {
      const v = i.value('x');
      return v !== null ? extractI32(v) : null;
    });
    expect(values).toEqual([null, 2, 1]);
  });

  // Rust: fn test_sorted_stream_all_nulls
  test('all_nulls', async () => {
    const items = [TestItem.new([]), TestItem.new([]), TestItem.new([])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    expect(sorted.length).toBe(3);
  });
});

// ============================================================================
// Multi-Column Presort Tests
// ============================================================================

describe('Multi-Column Presort', () => {
  // Rust: fn test_sorted_stream_multi_column_presort
  test('multi_column_presort', async () => {
    // Input PRE-SORTED by (cat, subcat)
    const items = [
      TestItem.catSubcatVal('A', 'X', 3),
      TestItem.catSubcatVal('A', 'X', 1),
      TestItem.catSubcatVal('A', 'Y', 5),
      TestItem.catSubcatVal('A', 'Y', 4),
      TestItem.catSubcatVal('B', 'X', 7),
      TestItem.catSubcatVal('B', 'X', 6),
    ];

    // presort: cat, subcat; spill: val ASC
    const orderBy = new OrderByComponents([obyAsc('cat'), obyAsc('subcat')], [obyAsc('val')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    // A-X: 1, 3
    // A-Y: 4, 5
    // B-X: 6, 7
    const values = sorted.map((i) => extractI32(i.value('val')!));
    expect(values).toEqual([1, 3, 4, 5, 6, 7]);
  });
});
