// MIRRORS: ankurah/storage/common/src/sorting.rs #[cfg(test)]

import { describe, test, expect } from 'bun:test';
import { PathExpr, OrderByItem, OrderDirection } from '@ankurah/ankql';
import type { Filterable, Value } from '@ankurah/core';
import { OrderByComponents } from './types.ts';
import { sortedIterable, limitedIterable, topKIterable } from './sorting.ts';

// ── Test helpers ────────────────────────────────────────────────────

/** Helper to collect async iterable items */
async function collect<T>(iter: AsyncIterable<T>): Promise<T[]> {
  const result: T[] = [];
  for await (const item of iter) {
    result.push(item);
  }
  return result;
}

/** Helper to wrap items in an async iterable */
async function* iterFrom<T>(items: T[]): AsyncGenerator<T> {
  for (const item of items) {
    yield item;
  }
}

/** Test item that implements Filterable for unit testing */
class TestItem implements Filterable {
  readonly values: Map<string, Value>;

  constructor(values: Map<string, Value>) {
    this.values = values;
  }

  static new(pairs: [string, Value][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, v);
    }
    return new TestItem(values);
  }

  static int(pairs: [string, number][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, { type: 'I32', value: v });
    }
    return new TestItem(values);
  }

  static str(pairs: [string, string][]): TestItem {
    const values = new Map<string, Value>();
    for (const [k, v] of pairs) {
      values.set(k, { type: 'String', value: v });
    }
    return new TestItem(values);
  }

  static mixed(cat: string, name: string): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('name', { type: 'String', value: name });
    return new TestItem(values);
  }

  static catVal(cat: string, val: number): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('val', { type: 'I32', value: val });
    return new TestItem(values);
  }

  static catSubcatVal(cat: string, subcat: string, val: number): TestItem {
    const values = new Map<string, Value>();
    values.set('cat', { type: 'String', value: cat });
    values.set('subcat', { type: 'String', value: subcat });
    values.set('val', { type: 'I32', value: val });
    return new TestItem(values);
  }

  collection(): string { return 'test'; }

  value(property: string): Value | null {
    return this.values.get(property) ?? null;
  }
}

/** Helper to extract i32 from Value */
function extractI32(v: Value): number {
  if (v.type !== 'I32') throw new Error('expected I32');
  return v.value;
}

/** Helper to extract String from Value */
function extractString(v: Value): string {
  if (v.type !== 'String') throw new Error('expected String');
  return v.value;
}

function oby(col: string, dir: OrderDirection): OrderByItem {
  return new OrderByItem(PathExpr.simple(col), dir);
}

function obyAsc(col: string): OrderByItem { return oby(col, OrderDirection.Asc()); }

function obyDesc(col: string): OrderByItem { return oby(col, OrderDirection.Desc()); }

// ============================================================================
// LimitedStream Tests
// ============================================================================

describe('LimitedStream', () => {
  test('basic', async () => {
    const items = [1, 2, 3, 4, 5];
    const limited = await collect(limitedIterable(iterFrom(items), 3));
    expect(limited).toEqual([1, 2, 3]);
  });

  test('no_limit', async () => {
    const items = [1, 2, 3, 4, 5];
    const limited = await collect(limitedIterable(iterFrom(items), null));
    expect(limited).toEqual([1, 2, 3, 4, 5]);
  });

  test('limit_exceeds_items', async () => {
    const items = [1, 2, 3];
    const limited = await collect(limitedIterable(iterFrom(items), 10));
    expect(limited).toEqual([1, 2, 3]);
  });

  test('zero_limit', async () => {
    const items = [1, 2, 3];
    const limited = await collect(limitedIterable(iterFrom(items), 0));
    expect(limited).toEqual([]);
  });

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
  test('asc', async () => {
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const values = sorted.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([1, 2, 3]);
  });

  test('desc', async () => {
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 3]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyDesc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const values = sorted.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([3, 2, 1]);
  });

  test('multi_column', async () => {
    const items = [TestItem.mixed('B', 'Z'), TestItem.mixed('A', 'Y'), TestItem.mixed('A', 'X'), TestItem.mixed('B', 'W')];

    const orderBy = new OrderByComponents([], [obyAsc('cat'), obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'W', 'Z']); // A-X, A-Y, B-W, B-Z
  });

  test('empty_input', async () => {
    const items: TestItem[] = [];
    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));
    expect(sorted).toEqual([]);
  });

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

  test('single_partition', async () => {
    // All items in same partition
    const items = [TestItem.mixed('A', 'Z'), TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Y')];

    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'Z']);
  });

  test('single_item_partitions', async () => {
    // Single item per partition
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('B', 'Y'), TestItem.mixed('C', 'Z')];

    const orderBy = new OrderByComponents([obyAsc('cat')], [obyAsc('name')]);
    const sorted = await collect(sortedIterable(iterFrom(items), orderBy));

    const names = sorted.map((i) => extractString(i.value('name')!));
    expect(names).toEqual(['X', 'Y', 'Z']);
  });

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

  test('k_exceeds_items', async () => {
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 10));

    const values = topk.map((i) => extractI32(i.value('x')!));
    expect(values).toEqual([1, 3]);
  });

  test('k_zero', async () => {
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];

    const orderBy = new OrderByComponents([], [obyAsc('x')]);
    const topk = await collect(topKIterable(iterFrom(items), orderBy, 0));

    expect(topk).toEqual([]);
  });

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
