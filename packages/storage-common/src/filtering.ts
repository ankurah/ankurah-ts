// MIRRORS: ankurah/storage/common/src/filtering.rs
//
// Divergence [E8]: Rust Stream/Pin/Poll → TypeScript AsyncIterable/AsyncIterator.

import type { Predicate } from '@ankurah/ankql';
import type { Filterable } from '@ankurah/core';
import { evaluatePredicate } from '@ankurah/core';
import { EntityId } from '@ankurah/proto';
import type { OrderByComponents } from './types.ts';
import { sortedIterable, limitedIterable, topKIterable } from './sorting.ts';

// ── ValueSetIterable ─────────────────────────────────────────────────

/**
 * Extension methods for async iterables of Filterable items.
 * Provides filter, sort, limit, and topK operations.
 *
 * Rust: `pub trait ValueSetStream: Stream + Unpin + Sized where Self::Item: Filterable`
 *
 * Divergence: Rust trait with methods on Stream → free functions on AsyncIterable in TS.
 * TypeScript does not support extension traits, so these are standalone functions.
 */

/**
 * Filter iterable items using a predicate.
 *
 * Rust: `fn filter_predicate(self, predicate: &Predicate) -> FilteredStream<Self>`
 */
export async function* filterPredicate<T extends Filterable>(
  inner: AsyncIterable<T>,
  predicate: Predicate,
): AsyncGenerator<T> {
  for await (const item of inner) {
    if (evaluatePredicate(item, predicate)) {
      yield item;
    }
  }
}

/**
 * Sort all items by OrderByComponents (partition-aware when presort is non-empty).
 *
 * Rust: `fn sort_by(self, order_by: OrderByComponents) -> SortedStream<Self>`
 */
export function sortBy<T extends Filterable>(
  inner: AsyncIterable<T>,
  orderBy: OrderByComponents,
): AsyncGenerator<T> {
  return sortedIterable(inner, orderBy);
}

/**
 * Limit iterable to N items.
 *
 * Rust: `fn limit(self, limit: Option<u64>) -> LimitedStream<Self>`
 */
export function limit<T>(
  inner: AsyncIterable<T>,
  limitN: number | null,
): AsyncGenerator<T> {
  return limitedIterable(inner, limitN);
}

/**
 * Top-K with sort and limit (partition-aware when presort is non-empty).
 *
 * Rust: `fn top_k(self, order_by: OrderByComponents, k: usize) -> TopKStream<Self>`
 */
export function topK<T extends Filterable>(
  inner: AsyncIterable<T>,
  orderBy: OrderByComponents,
  k: number,
): AsyncGenerator<T> {
  return topKIterable(inner, orderBy, k);
}

// ── HasEntityId ──────────────────────────────────────────────────────

/**
 * Trait for types that can provide an EntityId.
 *
 * Rust: `pub trait HasEntityId { fn entity_id(&self) -> EntityId; }`
 */
export interface HasEntityId {
  entityId(): EntityId;
}

/**
 * Extract entity IDs from materialized values.
 *
 * Rust: `fn extract_ids(self) -> ExtractIdsStream<Self>`
 */
export async function* extractIds<T extends HasEntityId>(
  inner: AsyncIterable<T>,
): AsyncGenerator<EntityId> {
  for await (const item of inner) {
    yield item.entityId();
  }
}
