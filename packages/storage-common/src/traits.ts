// MIRRORS: ankurah/storage/common/src/traits.rs
//
// Divergence [E8]: Rust Stream/Pin/Poll → TypeScript AsyncIterable/AsyncIterator.
// Divergence [E7]: Rust traits with default methods → TypeScript interfaces + free functions.
// Divergence: Blanket impls are implicit — any AsyncIterable with the right Item type qualifies.

import type { EntityId } from '@ankurah/proto';
import type { Attested, EntityState } from '@ankurah/proto';
import type { Predicate, OrderByItem } from '@ankurah/ankql';
import { limitedIterable } from './sorting.ts';

// ── EntityIdStream ──────────────────────────────────────────────────

/**
 * Stream of entity IDs - generic over any storage engine.
 *
 * Rust: `pub trait EntityIdStream: Stream<Item = Result<EntityId, RetrievalError>> + Unpin + Sized`
 *
 * Divergence [E7]: Rust trait → TypeScript type alias. Any AsyncIterable<Result<EntityId, RetrievalError>>
 * qualifies (mirrors the blanket impl in Rust).
 */
export type EntityIdIterable = AsyncIterable<EntityId>;

/**
 * Limit the number of entity IDs returned.
 */
// Rust: fn limit
export function entityIdLimit(
  inner: EntityIdIterable,
  limit: number | null,
): AsyncGenerator<EntityId> {
  return limitedIterable(inner, limit);
}

// ── EntityStateStream ───────────────────────────────────────────────

/**
 * Stream of entity states - generic over any storage engine.
 *
 * Rust: `pub trait EntityStateStream: Stream<Item = Result<Attested<EntityState>, RetrievalError>> + Unpin`
 *
 * Divergence [E7]: Rust trait → TypeScript type alias. Any AsyncIterable qualifies (blanket impl).
 */
export type EntityStateIterable = AsyncIterable<Attested<EntityState>>;

/**
 * Collect states, failing fast on first error (async version).
 *
 * Divergence [E8]: Rust Result-based stream with fail-fast → TS async function consuming AsyncIterable.
 * In TS, errors propagate as thrown exceptions rather than Result items.
 */
// Rust: fn collect_states
export async function collectStates(
  inner: EntityStateIterable,
): Promise<Attested<EntityState>[]> {
  const results: Attested<EntityState>[] = [];
  for await (const state of inner) {
    results.push(state);
  }
  return results;
}

// ── ScanExt ─────────────────────────────────────────────────────────

/**
 * Generic scan operations that can be implemented by any KV store.
 *
 * Rust: `pub trait ScanExt: Sized`
 *
 * Divergence [E7]: Rust trait with associated type → TypeScript interface.
 */
export interface ScanExt {
  /**
   * Extract entity IDs from keys (e.g., index key suffix or collection key).
   */
  // Rust: fn extract_entity_ids
  extractEntityIds(): EntityIdIterable;
}

// ── GetPropertyValueStream ──────────────────────────────────────────

/**
 * Default combinators that construct wrapper streams.
 *
 * Rust: `pub trait GetPropertyValueStream: Stream + Unpin + Sized`
 *
 * Divergence [E7]: Rust trait with default methods → free functions on AsyncIterable.
 * The Rust impl uses `todo!()` for all methods — TS mirrors this with `throw` stubs.
 */

/**
 * Filter: returns a filtered iterable over this stream.
 */
// Rust: fn filter_predicate
export function propertyValueFilterPredicate<T>(
  _inner: AsyncIterable<T>,
  _predicate: Predicate,
): AsyncIterable<T> {
  // Divergence: Rust uses todo!() — TS mirrors with throw
  throw new Error('TODO: construct FilteredStream(self, predicate.clone())');
}

/**
 * Sort: returns a sorted iterable over this stream (mutually exclusive with limit/top_k).
 */
// Rust: fn sort_by
export function propertyValueSortBy<T>(
  _inner: AsyncIterable<T>,
  _orderBy: OrderByItem[],
): AsyncIterable<T> {
  // Divergence: Rust uses todo!() — TS mirrors with throw
  throw new Error('TODO: construct SortedStream(self, order_by.to_vec())');
}

/**
 * Top-K: returns a top-k iterable over this stream (mutually exclusive with sort/limit).
 */
// Rust: fn top_k
export function propertyValueTopK<T>(
  _inner: AsyncIterable<T>,
  _orderBy: OrderByItem[],
  _k: number,
): AsyncIterable<T> {
  // Divergence: Rust uses todo!() — TS mirrors with throw
  throw new Error('TODO: construct TopKStream(self, order_by.to_vec(), k)');
}
