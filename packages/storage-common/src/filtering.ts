// MIRRORS: ankurah/storage/common/src/filtering.rs
//
// Divergence [E8]: Rust Stream/Pin/Poll → TypeScript AsyncIterable/AsyncIterator.

import type { Predicate } from '@ankurah/ankql';
import type { Filterable } from '@ankurah/core';
import { evaluatePredicate } from '@ankurah/core';
import { EntityId } from '@ankurah/proto';
import type { OrderByComponents } from './types.ts';
import { sortedIterable, limitedIterable, topKIterable } from './sorting.ts';

// Rust: fn FilteredStream::new — SKIP: absorbed into filterPredicate free function
// Rust: fn FilteredStream::poll_next — SKIP: absorbed into filterPredicate async generator
// Rust: fn ExtractIdsStream::new — SKIP: absorbed into extractIds free function
// Rust: fn ExtractIdsStream::poll_next — SKIP: absorbed into extractIds async generator

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
 */
// Rust: fn filter_predicate
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
 */
// Rust: fn sort_by
export function sortBy<T extends Filterable>(
  inner: AsyncIterable<T>,
  orderBy: OrderByComponents,
): AsyncGenerator<T> {
  return sortedIterable(inner, orderBy);
}

/**
 * Limit iterable to N items.
 */
// Rust: fn limit
export function limit<T>(
  inner: AsyncIterable<T>,
  limitN: number | null,
): AsyncGenerator<T> {
  return limitedIterable(inner, limitN);
}

/**
 * Top-K with sort and limit (partition-aware when presort is non-empty).
 */
// Rust: fn top_k
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
 */
export interface HasEntityId {
  // Rust: fn entity_id
  entityId(): EntityId;
}

/**
 * Extract entity IDs from materialized values.
 */
// Rust: fn extract_ids
export async function* extractIds<T extends HasEntityId>(
  inner: AsyncIterable<T>,
): AsyncGenerator<EntityId> {
  for await (const item of inner) {
    yield item.entityId();
  }
}
