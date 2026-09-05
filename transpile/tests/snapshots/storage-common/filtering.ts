// MIRRORS: ankurah/storage/common/src/filtering.rs
import { Struct, Result } from '@ankurah/base';
import { Predicate } from '@ankurah/ankql';
import { Filterable, Context } from '@ankurah/core';
import { Item } from '@ankurah/proto';

export class FilteredStream<I> extends Struct {
  readonly inner: I;
  readonly predicate: Predicate;

  constructor(inner: I, predicate: Predicate) {
    super();
    this.inner = inner;
    this.predicate = predicate;
  }

  static new<I>(inner: I, predicate: Predicate): FilteredStream<I> {
    return new FilteredStream(inner, predicate);
  }

  pollNext(cx: Context): Poll<Item | null> {
    while (true) {
      const _v = Pin.new(this.inner).pollNext(cx);
      if (_v.is('Ready') && (_v.value._0 != null)) {
        const item = _v.value._0;
        const _v1 = evaluatePredicate(item, this.predicate);
        if (_v1.isOk()) {
          const _v3 = _v1.unwrap();
          continue;
        } else {
          const _v4 = _v1.unwrapErr();
          continue;
        }
      } else if (_v.is('Ready') && (_v.value._0 == null)) {
        return new Poll('Ready', { _0: null });
      } else {
        return Poll.Pending;
      }
    }
  }
}

export class ExtractIdsStream<I> extends Struct {
  readonly inner: I;

  constructor(inner: I) {
    super();
    this.inner = inner;
  }

  static new<I>(inner: I): ExtractIdsStream<I> {
    return new ExtractIdsStream(inner);
  }

  pollNext(cx: Context): Poll<Item | null> {
    return Pin.new(this.inner).pollNext(cx).match({
      Ready: (v) => {
        const item = v._0;
        return new Poll('Ready', { _0: Result.Ok(item.entityId()) });
      },
      Pending: () => Poll.Pending,
    });
  }
}

export abstract class ValueSetStream {
  filterPredicate(predicate: Predicate): FilteredStream<Self> {
    return FilteredStream.new(this, predicate.clone());
  }
  sortBy(orderBy: OrderByComponents): SortedStream<Self> {
    return SortedStream.new(this, orderBy);
  }
  limit(limit: bigint | null): LimitedStream<Self> {
    return LimitedStream.new(this, limit);
  }
  topK(orderBy: OrderByComponents, k: number): TopKStream<Self> {
    return TopKStream.new(this, orderBy, k);
  }
  extractIds(): ExtractIdsStream<Self> {
    return ExtractIdsStream.new(this);
  }
}

export interface HasEntityId {
  entityId(): EntityId;
}

