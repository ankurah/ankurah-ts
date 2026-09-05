// MIRRORS: ankurah/storage/common/src/traits.rs
import { Result } from '@ankurah/base';
import { EntityId, RetrievalError } from '@ankurah/core';
import { Attested, EntityState } from '@ankurah/proto';

export abstract class EntityIdStream {
  limit(limit: bigint | null): LimitedStream<Self> {
    return LimitedStream.new(this, limit);
  }
}

export abstract class EntityStateStream {
  collectStates(): Promise<Result<Attested<EntityState>[], RetrievalError>> {
    return (async () => {
      let results = [];
      let stream = undefined /* pin!(self) */;
      for (;;) {
        const _v = await stream.next();
        if (!(_v != null)) {
          break;
        }
        const item = _v;
        if (item.isOk()) {
          const state = item.unwrap();
          results.push(state)
        } else {
          const e = item.unwrapErr();
          return Result.Err(e)
        }
      }
      return Result.Ok(results);
    })();
  }
}

export interface ScanExt {
  extractEntityIds(): EntityIdStream;
}

export abstract class GetPropertyValueStream {
  filterPredicate(_predicate: Predicate): FilteredStream<Self> {
    throw new Error('TODO');
  }
  sortBy(_orderBy: OrderByItem[]): SortedStream<Self> {
    throw new Error('TODO');
  }
  topK(_orderBy: OrderByItem[], _k: number): TopKStream<Self> {
    throw new Error('TODO');
  }
}

