// MIRRORS: ankurah/core/src/util/expand_states.rs
import { Result, dropOwned, HashSet } from '@ankurah/base';
import { RetrievalError } from '../error';
import { StorageCollectionWrapper } from '../storage';
import { Attested, EntityId, EntityState } from '@ankurah/proto';

export async function expandStates(states: Attested<EntityState>[], additionalEntityIds: EntityId[], collection: StorageCollectionWrapper): Promise<Result<Attested<EntityState>[], RetrievalError>> {
  let _moved0 = false;
  try {
    let entityMap = HashSet.from([...states].map((s) => s.payload.entityId));
    for (const entityId of additionalEntityIds) {
      if (!entityMap.has(entityId)) {
        const _v = await collection.deref().value.getState(entityId);
        if (_v.isOk()) {
          const state = _v.unwrap();
          let _moved1 = false;
          try {
            {
              _moved1 = true;
              states.push(state);
              entityMap.add(entityId);
            }
          } finally {
            if (!_moved1) state.drop();
          }
        } else {
          const _v1 = _v.unwrapErr();
          _arm3: {
            if (_v1.is('EntityNotFound')) {
              const _v2 = _v1;
              try {
                {
                }
              } finally {
                _v2.drop();
              }
              break _arm3;
            }
            {
              const e = _v1;
              let _moved2 = false;
              try {
                {
                  _moved2 = true;
                  return Result.Err(e);
                }
              } finally {
                if (!_moved2) e.drop();
              }
            }
          }
        }
      }
    }
    _moved0 = true;
    return Result.Ok(states);
  } finally {
    if (!_moved0) dropOwned(states);
  }
}

