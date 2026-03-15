// MIRRORS: ankurah/storage/common/src/sorting.rs
//
// Divergence [E8]: Rust Stream/Pin/Poll → TypeScript AsyncIterable/AsyncIterator.
// Divergence: Rust BinaryHeap → manual array-based min/max heap in TS.
// Divergence: Rust Arc/Mutex → plain fields [E8].

import type { OrderByItem, OrderDirection } from '@ankurah/ankql';
import type { Filterable, Value } from '@ankurah/core';
import { valuePartialCmp } from '@ankurah/core';
import type { OrderByComponents } from './types.ts';

// ── Sort helper ─────────────────────────────────────────────────────

/**
 * Sort items in-place by ORDER BY clauses.
 * None values sort before Some values (NULLS FIRST semantics).
 *
 * Rust: `fn sort_items_by_order<T: Filterable>(items: &mut [T], order_by: &[OrderByItem])`
 */
export function sortItemsByOrder<T extends Filterable>(items: T[], orderBy: OrderByItem[]): void {
  items.sort((a, b) => {
    for (const orderItem of orderBy) {
      const propertyName = orderItem.path.property();
      const aVal = a.value(propertyName);
      const bVal = b.value(propertyName);

      const cmp = compareForSort(aVal, bVal, orderItem.direction);
      if (cmp !== 0) return cmp;
    }
    return 0;
  });
}

/**
 * Compare two optional values for sorting, respecting direction.
 * None sorts before Some (NULLS FIRST).
 */
function compareForSort(a: Value | null, b: Value | null, direction: OrderDirection): number {
  if (a === null && b === null) return 0;
  if (a === null) return -1; // None < Some
  if (b === null) return 1;  // Some > None

  const cmp = valuePartialCmp(a, b);
  if (cmp === null) return 0; // incomparable

  if (direction.is('Asc')) return cmp;
  return -cmp; // Desc reverses
}

// ── Partition key extraction ─────────────────────────────────────────

/**
 * Extract partition key (presort column values) from an item.
 *
 * Rust: `fn extract_partition_key<T: Filterable>(item: &T, presort: &[OrderByItem]) -> Vec<Option<Value>>`
 */
function extractPartitionKey<T extends Filterable>(item: T, presort: OrderByItem[]): (Value | null)[] {
  return presort.map((p) => item.value(p.path.property()));
}

/**
 * Compare two partition keys for equality.
 */
function partitionKeysEqual(a: (Value | null)[], b: (Value | null)[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const av = a[i];
    const bv = b[i];
    if (av === null && bv === null) continue;
    if (av === null || bv === null) return false;
    if (av.type !== bv.type) return false;
    const cmp = valuePartialCmp(av, bv);
    if (cmp !== 0) return false;
  }
  return true;
}

// ── SortedIterable ────────────────────────────────────────────────────

/**
 * Sorted async iterable with partition-aware support.
 * - When presort is empty: global sort by spill columns.
 * - When presort is non-empty: partition-aware sort (sort within partitions defined by presort values).
 *
 * Rust: `pub struct SortedStream<S>`
 *
 * Divergence [E8]: Rust Stream → TypeScript AsyncIterable.
 */
export async function* sortedIterable<T extends Filterable>(
  inner: AsyncIterable<T>,
  orderBy: OrderByComponents,
): AsyncGenerator<T> {
  if (orderBy.presort.length === 0) {
    // Global sort: collect all, sort, emit
    const items: T[] = [];
    for await (const item of inner) {
      items.push(item);
    }
    sortItemsByOrder(items, orderBy.spill);
    yield* items;
    return;
  }

  // Partition-aware sorting
  let currentPartition: T[] = [];
  let currentPartitionKey: (Value | null)[] | null = null;

  for await (const item of inner) {
    const itemKey = extractPartitionKey(item, orderBy.presort);

    if (currentPartitionKey === null) {
      // First item - start new partition
      currentPartitionKey = itemKey;
      currentPartition.push(item);
    } else if (partitionKeysEqual(currentPartitionKey, itemKey)) {
      // Same partition - accumulate
      currentPartition.push(item);
    } else {
      // Partition changed - sort and emit previous partition
      sortItemsByOrder(currentPartition, orderBy.spill);
      yield* currentPartition;

      // Start new partition with current item
      currentPartitionKey = itemKey;
      currentPartition = [item];
    }
  }

  // Emit final partition
  if (currentPartition.length > 0) {
    sortItemsByOrder(currentPartition, orderBy.spill);
    yield* currentPartition;
  }
}

// ── LimitedIterable ──────────────────────────────────────────────────

/**
 * Limited async iterable - terminates after N items.
 *
 * Rust: `pub struct LimitedStream<I>`
 *
 * Divergence [E8]: Rust Stream → TypeScript AsyncIterable.
 */
export async function* limitedIterable<T>(
  inner: AsyncIterable<T>,
  limit: number | null,
): AsyncGenerator<T> {
  if (limit !== null && limit <= 0) return;

  let count = 0;
  for await (const item of inner) {
    yield item;
    count++;
    if (limit !== null && count >= limit) return;
  }
}

// ── TopK ─────────────────────────────────────────────────────────────

/**
 * TopK async iterable with partition-aware support.
 * - When presort is empty: global TopK by spill columns.
 * - When presort is non-empty: partition-aware TopK (can stop early once K items emitted).
 *
 * Uses a bounded heap: for ASC sort, keeps a max-heap of K smallest items;
 * for DESC, keeps a min-heap of K largest items.
 *
 * Rust: `pub struct TopKStream<S>`
 *
 * Divergence [E8]: Rust Stream/BinaryHeap → TypeScript AsyncIterable with array-based heap.
 */
export async function* topKIterable<T extends Filterable>(
  inner: AsyncIterable<T>,
  orderBy: OrderByComponents,
  k: number,
): AsyncGenerator<T> {
  if (k <= 0) return;

  if (orderBy.presort.length === 0) {
    // Global TopK: collect into heap, sort, emit
    const heap: T[] = [];
    for await (const item of inner) {
      heap.push(item);
      heapPushUp(heap, heap.length - 1, orderBy.spill);
      if (heap.length > k) {
        // Remove the "worst" element (heap root)
        swapItems(heap, 0, heap.length - 1);
        heap.pop();
        heapPushDown(heap, 0, orderBy.spill);
      }
    }
    // Sort the collected top-k and emit
    sortItemsByOrder(heap, orderBy.spill);
    yield* heap;
    return;
  }

  // Partition-aware TopK
  let emittedCount = 0;
  let currentPartition: T[] = [];
  let currentPartitionKey: (Value | null)[] | null = null;

  for await (const item of inner) {
    if (emittedCount >= k) return;

    const itemKey = extractPartitionKey(item, orderBy.presort);

    if (currentPartitionKey === null) {
      currentPartitionKey = itemKey;
      currentPartition.push(item);
    } else if (partitionKeysEqual(currentPartitionKey, itemKey)) {
      currentPartition.push(item);
    } else {
      // Partition changed - sort and emit previous partition
      sortItemsByOrder(currentPartition, orderBy.spill);
      for (const partItem of currentPartition) {
        if (emittedCount >= k) return;
        yield partItem;
        emittedCount++;
      }

      // Start new partition
      currentPartitionKey = itemKey;
      currentPartition = [item];
    }
  }

  // Emit final partition
  if (currentPartition.length > 0 && emittedCount < k) {
    sortItemsByOrder(currentPartition, orderBy.spill);
    for (const partItem of currentPartition) {
      if (emittedCount >= k) return;
      yield partItem;
      emittedCount++;
    }
  }
}

// ── Heap helpers (max-heap for TopK) ─────────────────────────────────

/**
 * Compare two items for the TopK heap.
 * Returns positive if `a` is "worse" (should be at the top of the heap to be evicted).
 *
 * For ASC order: larger values are "worse" → max-heap keeps smallest K.
 * For DESC order: smaller values are "worse" → effectively min-heap keeps largest K.
 */
function heapCompare<T extends Filterable>(a: T, b: T, orderBy: OrderByItem[]): number {
  for (const orderItem of orderBy) {
    const propertyName = orderItem.path.property();
    const aVal = a.value(propertyName);
    const bVal = b.value(propertyName);

    // For the heap, we want "worst" items at the top.
    // ASC: largest at top (normal compare), DESC: smallest at top (reversed).
    if (aVal === null && bVal === null) continue;
    if (aVal === null) {
      // ASC: None is smallest → less "worst" → negative
      // DESC: None is smallest → more "worst" → positive
      return orderItem.direction.is('Asc') ? -1 : 1;
    }
    if (bVal === null) {
      return orderItem.direction.is('Asc') ? 1 : -1;
    }

    const cmp = valuePartialCmp(aVal, bVal);
    if (cmp === null || cmp === 0) continue;

    // ASC: normal order (larger is worse), DESC: reversed (smaller is worse)
    return orderItem.direction.is('Asc') ? cmp : -cmp;
  }
  return 0;
}

function swapItems<T>(arr: T[], i: number, j: number): void {
  const tmp = arr[i];
  arr[i] = arr[j];
  arr[j] = tmp;
}

/** Push item at index up to maintain max-heap property. */
function heapPushUp<T extends Filterable>(heap: T[], index: number, orderBy: OrderByItem[]): void {
  while (index > 0) {
    const parent = Math.floor((index - 1) / 2);
    if (heapCompare(heap[index], heap[parent], orderBy) > 0) {
      swapItems(heap, index, parent);
      index = parent;
    } else {
      break;
    }
  }
}

/** Push item at index down to maintain max-heap property. */
function heapPushDown<T extends Filterable>(heap: T[], index: number, orderBy: OrderByItem[]): void {
  const size = heap.length;
  while (true) {
    let largest = index;
    const left = 2 * index + 1;
    const right = 2 * index + 2;

    if (left < size && heapCompare(heap[left], heap[largest], orderBy) > 0) {
      largest = left;
    }
    if (right < size && heapCompare(heap[right], heap[largest], orderBy) > 0) {
      largest = right;
    }

    if (largest !== index) {
      swapItems(heap, index, largest);
      index = largest;
    } else {
      break;
    }
  }
}
