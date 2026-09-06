// MIRRORS: ankurah/storage/common/src/sorting.rs
import { Struct, dropOwned, valueEquals, unsupported, checkedAdd } from '@ankurah/base';
import { Filterable, Value, Context } from '@ankurah/core';
import { OrderByComponents } from './types';
import { OrderByItem } from '@ankurah/ankql';
import { Item } from '@ankurah/proto';

export class SortedStream<S extends Unpin & Stream> extends Struct {
  inner: S | null;
  orderBy: OrderByComponents;
  currentPartition: Item[];
  currentPartitionKey: (Value | null)[] | null;
  sortedPartition: Item[] | null;
  exhausted: boolean;

  constructor(inner: S | null, orderBy: OrderByComponents, currentPartition: Item[], currentPartitionKey: (Value | null)[] | null, sortedPartition: Item[] | null, exhausted: boolean) {
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
            const _v = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
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
            const _v2 = this_.inner;
            if (!(_v2 != null)) {
              return { $jump: 'break' };
            }
            const inner = _v2;
            return Pin.new(inner).pollNext(cx);
          }
        })();
        if ((_m0 as any)?.$jump === 'break') break;
        const pollResult = (_m0 as any);
        const _m2 = pollResult.match<any>({
          Ready: (v) => {
            if (v._0 != null) {
              const item = v._0;
              this_.currentPartition.push(item);
            } else {
              let items = mem.take(this_.currentPartition);
              sortItemsByOrder(items, this_.orderBy.spill);
              this_.sortedPartition = items.intoIter();
              this_.inner = null;
              const _m1 = new Poll('Ready', { _0: null });
              return { $jump: 'return', $value: (this_.sortedPartition != null ? ((iter) => new Poll('Ready', { _0: unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance') }))(this_.sortedPartition!) : _m1) };
            }
          },
          Pending: () => {
            return { $jump: 'return', $value: Poll.Pending };
          },
        });
        if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
      }
      return new Poll('Ready', { _0: null });
    }
    while (true) {
      {
        const _v4 = this_.sortedPartition;
        if (_v4 != null) {
          const sortedIter = _v4;
          {
            const _v3 = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
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
      const _m3 = (() => {
        {
          const _v5 = this_.inner;
          if (!(_v5 != null)) {
            return { $jump: 'return', $value: new Poll('Ready', { _0: null }) };
          }
          const inner = _v5;
          return Pin.new(inner).pollNext(cx);
        }
      })();
      if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
      const pollResult = (_m3 as any);
      const _m8 = pollResult.match<any>({
        Ready: (v) => {
          if (v._0 != null) {
            const item = v._0;
            let _moved4 = false;
            const itemKey = extractPartitionKey(item, this_.orderBy.presort);
            try {
              _match7: {
                if (this_.currentPartitionKey == null) {
                  {
                    _moved4 = true;
                    const _a5 = itemKey;
                    dropOwned(this_.currentPartitionKey);
                    this_.currentPartitionKey = _a5;
                    this_.currentPartition.push(item);
                  }
                  break _match7;
                }
                if (this_.currentPartitionKey != null) {
                  const currentKey = this_.currentPartitionKey;
                  if (valueEquals(currentKey, itemKey)) {
                    {
                      this_.currentPartition.push(item);
                    }
                    break _match7;
                  }
                }
                {
                  {
                    let partition = mem.take(this_.currentPartition);
                    sortItemsByOrder(partition, this_.orderBy.spill);
                    this_.sortedPartition = partition.intoIter();
                    _moved4 = true;
                    const _a6 = itemKey;
                    dropOwned(this_.currentPartitionKey);
                    this_.currentPartitionKey = _a6;
                    this_.currentPartition.push(item);
                  }
                }
              }
            } finally {
              if (!_moved4) dropOwned(itemKey);
            }
          } else {
            this_.exhausted = true;
            this_.inner = null;
            if (!(this_.currentPartition.length === 0)) {
              let partition = mem.take(this_.currentPartition);
              sortItemsByOrder(partition, this_.orderBy.spill);
              this_.sortedPartition = partition.intoIter();
            }
          }
        },
        Pending: () => {
          return { $jump: 'return', $value: Poll.Pending }
        },
      });
      if ((_m8 as any)?.$jump === 'return') return (_m8 as any).$value;
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
        if (v._0 != null) {
          const item = v._0;
          this.count = checkedAdd(this.count, 1n, 'u64');
          return new Poll('Ready', { _0: item });
        } else {
          return new Poll('Ready', { _0: null });
        }
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
    for (const orderItem of this.orderBy) {
      const propertyName = orderItem.path.property();
      const selfVal = this.item.value(propertyName);
      const otherVal = other.item.value(propertyName);
      const cmp = (() => {
        const _v1 = [selfVal, otherVal, orderItem.direction];
        if ((_v1[0] == null) && (_v1[1] == null)) {
          return 0;
        } else if ((_v1[0] == null) && (_v1[1] != null) && (_v1[2].is('Asc'))) {
          try {
            return -1;
          } finally {
            dropOwned(_v1[1]);
          }
        } else if ((_v1[0] != null) && (_v1[1] == null) && (_v1[2].is('Asc'))) {
          try {
            return 1;
          } finally {
            dropOwned(_v1[0]);
          }
        } else if ((_v1[0] == null) && (_v1[1] != null) && (_v1[2].is('Desc'))) {
          try {
            return 1;
          } finally {
            dropOwned(_v1[1]);
          }
        } else if ((_v1[0] != null) && (_v1[1] == null) && (_v1[2].is('Desc'))) {
          try {
            return -1;
          } finally {
            dropOwned(_v1[0]);
          }
        } else if ((_v1[0] != null) && (_v1[1] != null) && (_v1[2].is('Asc'))) {
          const s = _v1[0];
          const o = _v1[1];
          return s.partialCompareTo(o) ?? 0;
        } else {
          const s = _v1[0];
          const o = _v1[1];
          return o.partialCompareTo(s) ?? 0;
        }
      })();
      if (cmp !== 0) {
        return cmp;
      }
    }
    return 0;
  }
}

export class TopKStream<S extends Unpin & Stream> extends Struct {
  inner: S | null;
  orderBy: OrderByComponents;
  k: number;
  emittedCount: number;
  currentPartition: Item[];
  currentPartitionKey: (Value | null)[] | null;
  sortedPartition: Item[] | null;
  exhausted: boolean;

  constructor(inner: S | null, orderBy: OrderByComponents, k: number, emittedCount: number, currentPartition: Item[], currentPartitionKey: (Value | null)[] | null, sortedPartition: Item[] | null, exhausted: boolean) {
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
            const _v = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
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
      let heap = BinaryHeap.new();
      try {
        while (true) {
          const _m1 = (() => {
            {
              const _v2 = this_.inner;
              if (!(_v2 != null)) {
                return { $jump: 'break' };
              }
              const inner = _v2;
              return Pin.new(inner).pollNext(cx);
            }
          })();
          if ((_m1 as any)?.$jump === 'break') break;
          const pollResult = (_m1 as any);
          const _m3 = pollResult.match<any>({
            Ready: (v) => {
              if (v._0 != null) {
                const item = v._0;
                const heapItem = new HeapItem(item, this_.orderBy.spill.map((e) => e.clone()));
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
              } else {
                this_.inner = null;
                _moved0 = true;
                let topK = heap.intoIter().map((h) => h.item);
                sortItemsByOrder(topK, this_.orderBy.spill);
                this_.sortedPartition = [...topK];
                const _m2 = new Poll('Ready', { _0: null });
                return { $jump: 'return', $value: (this_.sortedPartition != null ? ((iter) => {
                  {
                    const _v4 = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
                    if (_v4 != null) {
                      const item = _v4;
                      this_.emittedCount = checkedAdd(this_.emittedCount, 1, 'usize');
                      return new Poll('Ready', { _0: item });
                    } else {
                    return new Poll('Ready', { _0: null });
                  }
                  }
                })(this_.sortedPartition!) : _m2) };
              }
            },
            Pending: () => {
              return { $jump: 'return', $value: Poll.Pending }
            },
          });
          if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
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
        const _v6 = this_.sortedPartition;
        if (_v6 != null) {
          const sortedIter = _v6;
          {
            const _v5 = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
            if (_v5 != null) {
              const item = _v5;
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
      const _m4 = (() => {
        {
          const _v7 = this_.inner;
          if (!(_v7 != null)) {
            return { $jump: 'return', $value: new Poll('Ready', { _0: null }) };
          }
          const inner = _v7;
          return Pin.new(inner).pollNext(cx);
        }
      })();
      if ((_m4 as any)?.$jump === 'return') return (_m4 as any).$value;
      const pollResult = (_m4 as any);
      const _m9 = pollResult.match<any>({
        Ready: (v) => {
          if (v._0 != null) {
            const item = v._0;
            let _moved5 = false;
            const itemKey = extractPartitionKey(item, this_.orderBy.presort);
            try {
              _match8: {
                if (this_.currentPartitionKey == null) {
                  {
                    _moved5 = true;
                    const _a6 = itemKey;
                    dropOwned(this_.currentPartitionKey);
                    this_.currentPartitionKey = _a6;
                    this_.currentPartition.push(item);
                  }
                  break _match8;
                }
                if (this_.currentPartitionKey != null) {
                  const currentKey = this_.currentPartitionKey;
                  if (valueEquals(currentKey, itemKey)) {
                    {
                      this_.currentPartition.push(item);
                    }
                    break _match8;
                  }
                }
                {
                  {
                    let partition = mem.take(this_.currentPartition);
                    sortItemsByOrder(partition, this_.orderBy.spill);
                    this_.sortedPartition = partition.intoIter();
                    _moved5 = true;
                    const _a7 = itemKey;
                    dropOwned(this_.currentPartitionKey);
                    this_.currentPartitionKey = _a7;
                    this_.currentPartition.push(item);
                  }
                }
              }
            } finally {
              if (!_moved5) dropOwned(itemKey);
            }
          } else {
            this_.exhausted = true;
            this_.inner = null;
            if (!(this_.currentPartition.length === 0)) {
              let partition = mem.take(this_.currentPartition);
              sortItemsByOrder(partition, this_.orderBy.spill);
              this_.sortedPartition = partition.intoIter();
            }
          }
        },
        Pending: () => {
          return { $jump: 'return', $value: Poll.Pending }
        },
      });
      if ((_m9 as any)?.$jump === 'return') return (_m9 as any).$value;
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
          try {
            return -1;
          } finally {
            dropOwned(_v1[1]);
          }
        } else if ((_v1[0] != null) && (_v1[1] == null)) {
          try {
            return 1;
          } finally {
            dropOwned(_v1[0]);
          }
        } else if ((_v1[0] != null) && (_v1[1] != null) && (_v1[2].is('Asc'))) {
          const a = _v1[0];
          const b = _v1[1];
          return a.partialCompareTo(b) ?? 0;
        } else {
          const a = _v1[0];
          const b = _v1[1];
          return b.partialCompareTo(a) ?? 0;
        }
      })();
      if (cmp !== 0) {
        return cmp;
      }
    }
    return 0;
  });
}

function extractPartitionKey<T extends Filterable>(item: T, presort: OrderByItem[]): (Value | null)[] {
  return [...presort].map((p) => item.value(p.path.property()));
}

