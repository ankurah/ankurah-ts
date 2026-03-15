// MIRRORS: ankurah/core/src/util/expand_states.rs

import type { Attested, EntityState, EntityId } from '@ankurah/proto';
import { RetrievalError } from '../error.ts';
import type { StorageCollection } from '../storage.ts';

/**
 * Expand initial_states to include additional entities that weren't in the predicate results.
 * This ensures we can generate proper deltas for entities that may no longer match the predicate.
 *
 * When a client has knowledge of entities that don't appear in the current predicate results,
 * we need to fetch their current state individually to generate proper deltas (including removals).
 *
 * Rust: `pub async fn expand_states(...) -> Result<Vec<Attested<EntityState>>, RetrievalError>`
 * Divergence: Takes StorageCollection instead of StorageCollectionWrapper [E7].
 */
export async function expandStates(
  states: Attested<EntityState>[],
  additionalEntityIds: Iterable<EntityId>,
  collection: StorageCollection,
): Promise<Attested<EntityState>[]> {
  const entityMap = new Set<string>();
  for (const s of states) {
    entityMap.add(s.payload.entityId.toBase64());
  }

  for (const entityId of additionalEntityIds) {
    const key = entityId.toBase64();
    if (!entityMap.has(key)) {
      try {
        const state = await collection.getState(entityId);
        states.push(state);
        entityMap.add(key);
      } catch (e) {
        if (e instanceof RetrievalError && e.kind === 'EntityNotFound') {
          // Entity was deleted - silently ignore
        } else {
          throw e;
        }
      }
    }
  }

  return states;
}
