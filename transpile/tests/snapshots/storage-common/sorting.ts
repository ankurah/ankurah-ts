// MIRRORS: ankurah/storage/common/src/sorting.rs
import { Struct, dropOwned, checkedAdd } from '@ankurah/base';
import { Filterable, Value } from '@ankurah/core';
import { OrderByComponents } from './types';
import { OrderByItem } from '@ankurah/ankql';
import { Context, Filterable, Value } from '@ankurah/core';
import { Item } from '@ankurah/proto';

export class SortedStream<S extends Unpin & Stream> extends Struct {
  inner: S | null;
  orderBy: OrderByComponents;
  currentPartition: Item[];
  currentPartitionKey: Value | null[] | null;
  sortedPartition: Item[] | null;
  exhausted: boolean;

  constructor(inner: S | null, orderBy: OrderByComponents, currentPartition: Item[], currentPartitionKey: Value | null[] | null, sortedPartition: Item[] | null, exhausted: boolean) {
    super();
    this.inner = inner;
    this.orderBy = orderBy;
    this.currentPartition = currentPartition;
    this.currentPartitionKey = currentPartitionKey;
    this.sortedPartition = sortedPartition;
    this.exhausted = exhausted;
  }

  static new<S>(inner: S, orderBy: OrderByComponents): SortedStream<S> {
    return new SortedStream(inner, orderBy, [], null, null, false);
  }

  pollNext(cx: Context): Poll<Item | null> {
    const this_ = this.getMut();
    if (this_.orderBy.presort.length === 0) {
      {
        const _v1 = this_.sortedPartition;
        if (_v1 != null) {
          const sorted = _v1;
          {
            const _v = sorted.next();
            if (_v != null) {
              const item = _v;
              return new Poll('Ready', { _0: item });
            }
          }
          this_.sortedPartition = null;
          return new Poll('Ready', { _0: null });
        }
      }
      while (true) {
        const _m0 = (() => {
          {
            const _v2 = this_.inner.asMut();
            if (!(_v2 != null)) {
              return { $jump: 'break' };
            }
            const inner = _v2;
            return Pin.new(inner).pollNext(cx);
          }
        })();
        if ((_m0 as any)?.$jump === 'break') break;
        const pollResult = (_m0 as any);
        return pollResult.match({
          Ready: (v) => {
            const item = v._0;
            this_.currentPartition.push(item);
          },
          Pending: () => {
            return Poll.Pending
          },
        });
      }
      return new Poll('Ready', { _0: null });
    }
    while (true) {
      {
        const _v4 = this_.sortedPartition;
        if (_v4 != null) {
          const sortedIter = _v4;
          {
            const _v3 = sortedIter.next();
            if (_v3 != null) {
              const item = _v3;
              return new Poll('Ready', { _0: item });
            }
          }
          this_.sortedPartition = null;
        }
      }
      if (this_.exhausted) {
        return new Poll('Ready', { _0: null });
      }
      const _m1 = (() => {
        {
          const _v5 = this_.inner.asMut();
          if (!(_v5 != null)) {
            return { $jump: 'return', $value: new Poll('Ready', { _0: null }) };
          }
          const inner = _v5;
          return Pin.new(inner).pollNext(cx);
        }
      })();
      if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
      const pollResult = (_m1 as any);
      return pollResult.match({
        Ready: (v) => {
          const item = v._0;
          let _moved2 = false;
          const itemKey = extractPartitionKey(item, this_.orderBy.presort);
          try {
            _match5: {
              if (this_.currentPartitionKey == null) {
                {
                  _moved2 = true;
                  const _a3 = itemKey;
                  dropOwned(this_.currentPartitionKey);
                  this_.currentPartitionKey = _a3;
                  this_.currentPartition.push(item);
                }
                break _match5;
              }
              if (this_.currentPartitionKey != null) {
                const currentKey = this_.currentPartitionKey;
                if (currentKey === itemKey) {
                  {
                    this_.currentPartition.push(item);
                  }
                  break _match5;
                }
              }
              if (this_.currentPartitionKey != null) {
                {
                  let partition = mem.take(this_.currentPartition);
                  sortItemsByOrder(partition, this_.orderBy.spill);
                  this_.sortedPartition = partition.intoIter();
                  _moved2 = true;
                  const _a4 = itemKey;
                  dropOwned(this_.currentPartitionKey);
                  this_.currentPartitionKey = _a4;
                  this_.currentPartition.push(item);
                }
                break _match5;
              }
            }
          } finally {
            if (!_moved2) dropOwned(itemKey);
          }
        },
        Pending: () => {
          return Poll.Pending
        },
      });
    }
  }
}

export class LimitedStream<I> extends Struct {
  inner: I;
  readonly limit: bigint | null;
  count: bigint;

  constructor(inner: I, limit: bigint | null, count: bigint) {
    super();
    this.inner = inner;
    this.limit = limit;
    this.count = count;
  }

  static new<I>(inner: I, limit: bigint | null): LimitedStream<I> {
    return new LimitedStream(inner, limit, 0n);
  }

  pollNext(cx: Context): Poll<Item | null> {
    {
      const _v = this.limit;
      if (_v != null) {
        const limit = _v;
        if (this.count >= limit) {
          return new Poll('Ready', { _0: null });
        }
      }
    }
    return Pin.new(this.inner).pollNext(cx).match({
      Ready: (v) => {
        const item = v._0;
        this.count = checkedAdd(this.count, 1n, 'u64');
        return new Poll('Ready', { _0: item });
      },
      Pending: () => Poll.Pending,
    });
  }
}

class HeapItem<T extends Filterable> extends Struct {
  item: T;
  orderBy: OrderByItem[];

  constructor(item: T, orderBy: OrderByItem[]) {
    super();
    this.item = item;
    this.orderBy = orderBy;
  }

  equals(other: HeapItem<T>): boolean {
    return this.compareTo(other) === 0;
  }

  compareTo(other: HeapItem<T>): number {
    return this.compareTo(other);
  }
}

export class TopKStream<S extends Unpin & Stream> extends Struct {
  inner: S | null;
  orderBy: OrderByComponents;
  k: number;
  emittedCount: number;
  currentPartition: Item[];
  currentPartitionKey: Value | null[] | null;
  sortedPartition: Item[] | null;
  exhausted: boolean;

  constructor(inner: S | null, orderBy: OrderByComponents, k: number, emittedCount: number, currentPartition: Item[], currentPartitionKey: Value | null[] | null, sortedPartition: Item[] | null, exhausted: boolean) {
    super();
    this.inner = inner;
    this.orderBy = orderBy;
    this.k = k;
    this.emittedCount = emittedCount;
    this.currentPartition = currentPartition;
    this.currentPartitionKey = currentPartitionKey;
    this.sortedPartition = sortedPartition;
    this.exhausted = exhausted;
  }

  static new<S>(inner: S, orderBy: OrderByComponents, k: number): TopKStream<S> {
    return new TopKStream(inner, orderBy, k, 0, [], null, null, false);
  }

  pollNext(cx: Context): Poll<Item | null> {
    const this_ = this.getMut();
    if (this_.emittedCount >= this_.k) {
      return new Poll('Ready', { _0: null });
    }
    if (this_.orderBy.presort.length === 0) {
      {
        const _v1 = this_.sortedPartition;
        if (_v1 != null) {
          const sorted = _v1;
          {
            const _v = sorted.next();
            if (_v != null) {
              const item = _v;
              this_.emittedCount = checkedAdd(this_.emittedCount, 1, 'usize');
              return new Poll('Ready', { _0: item });
            }
          }
          this_.sortedPartition = null;
          return new Poll('Ready', { _0: null });
        }
      }
      let _moved0 = false;
      const heap = BinaryHeap.new();
      try {
        while (true) {
          const _m1 = (() => {
            {
              const _v2 = this_.inner.asMut();
              if (!(_v2 != null)) {
                return { $jump: 'break' };
              }
              const inner = _v2;
              return Pin.new(inner).pollNext(cx);
            }
          })();
          if ((_m1 as any)?.$jump === 'break') break;
          const pollResult = (_m1 as any);
          return pollResult.match({
            Ready: (v) => {
              const item = v._0;
              const heapItem = new HeapItem(item, this_.orderBy.spill.clone());
              if (heap.len() < this_.k) {
                heap.push(heapItem);
              } else {
                const _v3 = heap.peek();
                if (_v3 != null) {
                  const worst = _v3;
                  if (heapItem < worst) {
                    dropOwned(heap.pop());
                    heap.push(heapItem);
                  }
                }
              }
            },
            Pending: () => {
              return Poll.Pending
            },
          });
        }
        return new Poll('Ready', { _0: null });
      } finally {
        if (!_moved0) dropOwned(heap);
      }
    }
    while (true) {
      if (this_.emittedCount >= this_.k) {
        return new Poll('Ready', { _0: null });
      }
      {
        const _v5 = this_.sortedPartition;
        if (_v5 != null) {
          const sortedIter = _v5;
          {
            const _v4 = sortedIter.next();
            if (_v4 != null) {
              const item = _v4;
              this_.emittedCount = checkedAdd(this_.emittedCount, 1, 'usize');
              return new Poll('Ready', { _0: item });
            }
          }
          this_.sortedPartition = null;
        }
      }
      if (this_.exhausted) {
        return new Poll('Ready', { _0: null });
      }
      const _m2 = (() => {
        {
          const _v6 = this_.inner.asMut();
          if (!(_v6 != null)) {
            return { $jump: 'return', $value: new Poll('Ready', { _0: null }) };
          }
          const inner = _v6;
          return Pin.new(inner).pollNext(cx);
        }
      })();
      if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
      const pollResult = (_m2 as any);
      return pollResult.match({
        Ready: (v) => {
          const item = v._0;
          let _moved3 = false;
          const itemKey = extractPartitionKey(item, this_.orderBy.presort);
          try {
            _match6: {
              if (this_.currentPartitionKey == null) {
                {
                  _moved3 = true;
                  const _a4 = itemKey;
                  dropOwned(this_.currentPartitionKey);
                  this_.currentPartitionKey = _a4;
                  this_.currentPartition.push(item);
                }
                break _match6;
              }
              if (this_.currentPartitionKey != null) {
                const currentKey = this_.currentPartitionKey;
                if (currentKey === itemKey) {
                  {
                    this_.currentPartition.push(item);
                  }
                  break _match6;
                }
              }
              if (this_.currentPartitionKey != null) {
                {
                  let partition = mem.take(this_.currentPartition);
                  sortItemsByOrder(partition, this_.orderBy.spill);
                  this_.sortedPartition = partition.intoIter();
                  _moved3 = true;
                  const _a5 = itemKey;
                  dropOwned(this_.currentPartitionKey);
                  this_.currentPartitionKey = _a5;
                  this_.currentPartition.push(item);
                }
                break _match6;
              }
            }
          } finally {
            if (!_moved3) dropOwned(itemKey);
          }
        },
        Pending: () => {
          return Poll.Pending
        },
      });
    }
  }
}

function sortItemsByOrder<T extends Filterable>(items: T[], orderBy: OrderByItem[]): void {
  items.sort((a, b) => {
    for (const orderItem of orderBy) {
      const propertyName = orderItem.path.property();
      const aVal = a.value(propertyName);
      const bVal = b.value(propertyName);
      const cmp = (() => {
        const _v1 = [aVal, bVal, orderItem.direction];
        if ((_v1[0] == null) && (_v1[1] == null)) {
          return 0;
        } else if ((_v1[0] == null) && (_v1[1] != null)) {
          return -1;
        } else if ((_v1[0] != null) && (_v1[1] == null)) {
          return 1;
        } else if ((_v1[0] != null) && (_v1[1] != null) && (_v1[2].is('Asc'))) {
          const a = _v1[0];
          const b = _v1[1];
          return a.compareTo(b) ?? 0;
        } else {
          const a = _v1[0];
          const b = _v1[1];
          return b.compareTo(a) ?? 0;
        }
      })();
      if (cmp !== 0) {
        return cmp;
      }
    }
    return 0;
  });
}

function extractPartitionKey<T extends Filterable>(item: T, presort: OrderByItem[]): Value | null[] {
  return [...presort].map((p) => item.value(p.path.property()));
}

