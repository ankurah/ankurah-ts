// MIRRORS: ankurah/core/src/collectionset.rs

import type { CollectionId } from '@ankurah/proto';
import type { StorageEngine, StorageCollection } from './storage.ts';
import type { RetrievalError, MutationError } from './error.ts';

// ---------------------------------------------------------------------------
// CollectionSet — lazy-init cache for StorageCollection handles
// ---------------------------------------------------------------------------

/**
 * Lazy-initialising cache for StorageCollection handles.
 * Provides deduplication so the same collection ID always yields the same handle.
 *
 * Rust: `pub struct CollectionSet<SE>(Arc<Inner<SE>>)`
 * Divergence: No Arc/Inner split — single-threaded JS, plain class [E8].
 * Divergence: No RwLock on collections map — single-threaded JS [E8].
 * Divergence: Not generic over SE — TS uses the StorageEngine interface directly [E7].
 */
export class CollectionSet {
  private readonly storageEngine: StorageEngine;
  private readonly collections: Map<string, StorageCollection> = new Map();

  constructor(storageEngine: StorageEngine) {
    this.storageEngine = storageEngine;
  }

  /**
   * Get or lazily create a StorageCollection for the given collection ID.
   *
   * Rust: `pub async fn get(&self, id: &CollectionId) -> Result<StorageCollectionWrapper, RetrievalError>`
   * Divergence: Returns StorageCollection directly instead of StorageCollectionWrapper [E7].
   *
   * Note: Concurrent calls for the same collection ID may race, but the Map
   * ensures only one handle is retained (last-write-wins is fine since all
   * handles point to the same underlying storage).
   */
  async get(id: CollectionId): Promise<StorageCollection> {
    const key = collectionIdKey(id);
    const existing = this.collections.get(key);
    if (existing) {
      return existing;
    }

    const collection = await this.storageEngine.collection(id);
    // Another caller may have raced us; keep the first one if present
    // Divergence: No Entry API — use has() check instead [E7].
    if (!this.collections.has(key)) {
      this.collections.set(key, collection);
    }

    return this.collections.get(key)!;
  }

  /**
   * List all collection IDs currently cached in memory.
   *
   * Rust: `pub async fn list_collections(&self) -> Result<Vec<CollectionId>, RetrievalError>`
   * Divergence: Returns string keys rather than CollectionId values,
   *   because we store by string key. Callers should reconstruct CollectionId if needed [E7].
   */
  listCollections(): string[] {
    return Array.from(this.collections.keys());
  }

  /**
   * Delete all collections from the cache.
   *
   * Rust: `pub async fn delete_all_collections(&self) -> Result<bool, MutationError>`
   * Divergence: StorageEngine interface doesn't have deleteAllCollections yet,
   *   so this only clears the in-memory cache [E7].
   */
  deleteAllCollections(): void {
    this.collections.clear();
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Convert CollectionId to a stable string key for Map lookups. */
function collectionIdKey(id: CollectionId): string {
  return id.toString();
}
