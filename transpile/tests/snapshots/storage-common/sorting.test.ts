// MIRRORS: ankurah/storage/common/src/sorting.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { LimitedStream, SortedStream, TopKStream } from './sorting';
import { HashMap, Struct, debugString, dropOwned, unsupported } from '@ankurah/base';
import { OrderByComponents } from './types';
import { OrderByItem, OrderDirection, PathExpr } from '@ankurah/ankql';
import { Json } from '@ankurah/core';
import { EntityId } from '@ankurah/proto';

class TestItem extends Struct implements Filterable {
  values: HashMap<string, Value>;

  constructor(values: HashMap<string, Value>) {
    super();
    this.values = values;
  }

  static new(pairs: [string, Value][]): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      for (const [k, v] of pairs) {
        values.set(k, v.clone());
      }
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  static int(pairs: [string, number][]): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      for (const [k, v] of pairs) {
        values.set(k, new Value('I32', { _0: v }));
      }
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  static str(pairs: [string, string][]): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      for (const [k, v] of pairs) {
        values.set(k, new Value('String', { _0: v }));
      }
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  static mixed(cat: string, name: string): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      values.set('cat', new Value('String', { _0: cat }));
      values.set('name', new Value('String', { _0: name }));
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  static catVal(cat: string, val: number): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      values.set('cat', new Value('String', { _0: cat }));
      values.set('val', new Value('I32', { _0: val }));
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  static catSubcatVal(cat: string, subcat: string, val: number): TestItem {
    let _moved0 = false;
    let values = new HashMap();
    try {
      values.set('cat', new Value('String', { _0: cat }));
      values.set('subcat', new Value('String', { _0: subcat }));
      values.set('val', new Value('I32', { _0: val }));
      _moved0 = true;
      return new TestItem(values);
    } finally {
      if (!_moved0) dropOwned(values);
    }
  }

  collection(): string {
    return 'test';
  }

  value(property: string): Value | null {
    return this.values.get(property);
  }

  equals(other: TestItem): boolean {
    { if (this.values.size !== other.values.size) return false; for (const [k, v] of this.values) { if (!other.values.has(k)) return false; const _w = other.values.get(k)!; if (!v.equals(_w)) return false; } }
    return true;
  }

  clone(): TestItem {
    return new TestItem(this.values.clone());
  }

  debug(): string {
    return `TestItem { values: ${`{${Array.from(this.values).map(($p) => `${debugString($p[0])}: ${$p[1].debug()}`).join(', ')}}`} }`;
  }
}

describe('sorting unit tests', () => {
  function collectStream(stream: S): Item[] {
    return futures.executor.blockOn(unsupported('`collect` into `Collect<S, C>` is a `FromIterator` the port has no construction for'));
  }

  function streamFrom(items: T[]): Iter<T[]> {
    return futures.stream(items);
  }

  function extractI32(v: Value): number {
    return v.match({
      I32: (v) => {
        const n = v._0;
        return n;
      },
      I16: () => {
        throw new Error('expected I32');
      },
      I64: () => {
        throw new Error('expected I32');
      },
      F64: () => {
        throw new Error('expected I32');
      },
      Bool: () => {
        throw new Error('expected I32');
      },
      String: () => {
        throw new Error('expected I32');
      },
      EntityId: () => {
        throw new Error('expected I32');
      },
      Object: () => {
        throw new Error('expected I32');
      },
      Binary: () => {
        throw new Error('expected I32');
      },
      Json: () => {
        throw new Error('expected I32');
      },
    });
  }

  function extractString(v: Value): string {
    return v.match({
      String: (v) => {
        const s = v._0;
        return s;
      },
      I16: () => {
        throw new Error('expected String');
      },
      I32: () => {
        throw new Error('expected String');
      },
      I64: () => {
        throw new Error('expected String');
      },
      F64: () => {
        throw new Error('expected String');
      },
      Bool: () => {
        throw new Error('expected String');
      },
      EntityId: () => {
        throw new Error('expected String');
      },
      Object: () => {
        throw new Error('expected String');
      },
      Binary: () => {
        throw new Error('expected String');
      },
      Json: () => {
        throw new Error('expected String');
      },
    });
  }

  function oby(col: string, dir: OrderDirection): OrderByItem {
    let _moved0 = false;
    try {
      const _b1 = PathExpr.simple(col);
      _moved0 = true;
      return new OrderByItem(_b1, dir);
    } finally {
      if (!_moved0) dir.drop();
    }
  }

  function obyAsc(col: string): OrderByItem {
    return oby(col, new OrderDirection('Asc', {}));
  }

  function obyDesc(col: string): OrderByItem {
    return oby(col, new OrderDirection('Desc', {}));
  }

  test('test_limited_stream_basic', () => {
    const items = [1, 2, 3, 4, 5];
    const limited = collectStream(LimitedStream.new(streamFrom(items), 3n));
    expect(limited).toEqual([1, 2, 3]);
  });

  test('test_limited_stream_no_limit', () => {
    const items = [1, 2, 3, 4, 5];
    const limited = collectStream(LimitedStream.new(streamFrom(items), null));
    expect(limited).toEqual([1, 2, 3, 4, 5]);
  });

  test('test_limited_stream_limit_exceeds_items', () => {
    const items = [1, 2, 3];
    const limited = collectStream(LimitedStream.new(streamFrom(items), 10n));
    expect(limited).toEqual([1, 2, 3]);
  });

  test('test_limited_stream_zero_limit', () => {
    const items = [1, 2, 3];
    const limited = collectStream(LimitedStream.new(streamFrom(items), 0n));
    if (!(limited.length === 0)) throw new Error('assertion failed');
  });

  test('test_limited_stream_empty_input', () => {
    const items = [];
    const limited = collectStream(LimitedStream.new(streamFrom(items), 5n));
    if (!(limited.length === 0)) throw new Error('assertion failed');
  });

  test('test_sorted_stream_global_sort_asc', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const values = [...sorted].map((i) => extractI32(i.value('x')));
        expect(values).toEqual([1, 2, 3]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_global_sort_desc', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 3]]), TestItem.int([['x', 2]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyDesc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const values = [...sorted].map((i) => extractI32(i.value('x')));
        expect(values).toEqual([3, 2, 1]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_global_sort_multi_column', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('B', 'Z'), TestItem.mixed('A', 'Y'), TestItem.mixed('A', 'X'), TestItem.mixed('B', 'W')];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('cat'), obyAsc('name')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const names = [...sorted].map((i) => extractString(i.value('name')));
        expect(names).toEqual(['X', 'Y', 'W', 'Z']);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_empty_input', () => {
    let _moved0 = false;
    const items = [];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        if (!(sorted.length === 0)) throw new Error('assertion failed');
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_single_item', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 42]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        expect(sorted.length).toEqual(1);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_partition_aware_basic', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('A', 'Z'), TestItem.mixed('A', 'X'), TestItem.mixed('B', 'Y'), TestItem.mixed('B', 'W')];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyAsc('name')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
          const names = [...sorted].map((i) => extractString(i.value('name')));
          expect(names).toEqual(['X', 'Z', 'W', 'Y']);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_partition_aware_mixed_directions', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Z'), TestItem.mixed('B', 'W'), TestItem.mixed('B', 'Y')];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyDesc('name')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
          const names = [...sorted].map((i) => extractString(i.value('name')));
          expect(names).toEqual(['Z', 'X', 'Y', 'W']);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_partition_aware_single_partition', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('A', 'Z'), TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Y')];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyAsc('name')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
          const names = [...sorted].map((i) => extractString(i.value('name')));
          expect(names).toEqual(['X', 'Y', 'Z']);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_partition_aware_single_item_partitions', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('B', 'Y'), TestItem.mixed('C', 'Z')];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyAsc('name')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
          const names = [...sorted].map((i) => extractString(i.value('name')));
          expect(names).toEqual(['X', 'Y', 'Z']);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_partition_aware_empty_spill', () => {
    let _moved0 = false;
    const items = [TestItem.mixed('A', 'X'), TestItem.mixed('A', 'Z'), TestItem.mixed('B', 'Y')];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([obyAsc('cat')], []);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const names = [...sorted].map((i) => extractString(i.value('name')));
        expect(names).toEqual(['X', 'Z', 'Y']);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_global_basic', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 5]]), TestItem.int([['x', 1]]), TestItem.int([['x', 3]]), TestItem.int([['x', 4]]), TestItem.int([['x', 2]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 3));
        const values = [...topk].map((i) => extractI32(i.value('x')));
        expect(values).toEqual([1, 2, 3]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_global_desc', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 5]]), TestItem.int([['x', 1]]), TestItem.int([['x', 3]]), TestItem.int([['x', 4]]), TestItem.int([['x', 2]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyDesc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 3));
        const values = [...topk].map((i) => extractI32(i.value('x')));
        expect(values).toEqual([5, 4, 3]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_global_k_exceeds_items', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 3]]), TestItem.int([['x', 1]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 10));
        const values = [...topk].map((i) => extractI32(i.value('x')));
        expect(values).toEqual([1, 3]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_global_k_zero', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 1]]), TestItem.int([['x', 2]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 0));
        if (!(topk.length === 0)) throw new Error('assertion failed');
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_global_empty_input', () => {
    let _moved0 = false;
    const items = [];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 5));
        if (!(topk.length === 0)) throw new Error('assertion failed');
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_partition_aware_basic', () => {
    let _moved0 = false;
    const items = [TestItem.catVal('A', 3), TestItem.catVal('A', 1), TestItem.catVal('A', 2), TestItem.catVal('B', 6), TestItem.catVal('B', 4), TestItem.catVal('B', 5)];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyAsc('val')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 4));
          const values = [...topk].map((i) => extractI32(i.value('val')));
          expect(values).toEqual([1, 2, 3, 4]);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_partition_aware_limit_within_partition', () => {
    let _moved0 = false;
    const items = [TestItem.catVal('A', 5), TestItem.catVal('A', 1), TestItem.catVal('A', 3), TestItem.catVal('A', 2), TestItem.catVal('A', 4), TestItem.catVal('B', 10)];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyAsc('val')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 3));
          const values = [...topk].map((i) => extractI32(i.value('val')));
          expect(values).toEqual([1, 2, 3]);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_topk_stream_partition_aware_mixed_directions', () => {
    let _moved0 = false;
    const items = [TestItem.catVal('A', 1), TestItem.catVal('A', 3), TestItem.catVal('A', 2), TestItem.catVal('B', 4), TestItem.catVal('B', 6)];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat')];
      try {
        const _b3 = [obyDesc('val')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const topk = collectStream(TopKStream.new(streamFrom(items), orderBy, 4));
          const values = [...topk].map((i) => extractI32(i.value('val')));
          expect(values).toEqual([3, 2, 1, 6]);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_null_sorts_first_asc', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 2]]), TestItem.new([]), TestItem.int([['x', 1]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const values = [...sorted].map((i) => i.value('x').map((v) => extractI32(v)));
        expect(values).toEqual([null, 1, 2]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_null_sorts_first_desc', () => {
    let _moved0 = false;
    const items = [TestItem.int([['x', 2]]), TestItem.new([]), TestItem.int([['x', 1]])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyDesc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        const values = [...sorted].map((i) => i.value('x').map((v) => extractI32(v)));
        expect(values).toEqual([null, 2, 1]);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_all_nulls', () => {
    let _moved0 = false;
    const items = [TestItem.new([]), TestItem.new([]), TestItem.new([])];
    try {
      let _moved1 = false;
      const orderBy = OrderByComponents.new([], [obyAsc('x')]);
      try {
        _moved0 = true;
        _moved1 = true;
        const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
        expect(sorted.length).toEqual(3);
      } finally {
        if (!_moved1) orderBy.drop();
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

  test('test_sorted_stream_multi_column_presort', () => {
    let _moved0 = false;
    const items = [TestItem.catSubcatVal('A', 'X', 3), TestItem.catSubcatVal('A', 'X', 1), TestItem.catSubcatVal('A', 'Y', 5), TestItem.catSubcatVal('A', 'Y', 4), TestItem.catSubcatVal('B', 'X', 7), TestItem.catSubcatVal('B', 'X', 6)];
    try {
      let _moved2 = false;
      const _b1 = [obyAsc('cat'), obyAsc('subcat')];
      try {
        const _b3 = [obyAsc('val')];
        _moved2 = true;
        let _moved4 = false;
        const orderBy = OrderByComponents.new(_b1, _b3);
        try {
          _moved0 = true;
          _moved4 = true;
          const sorted = collectStream(SortedStream.new(streamFrom(items), orderBy));
          const values = [...sorted].map((i) => extractI32(i.value('val')));
          expect(values).toEqual([1, 3, 4, 5, 6, 7]);
        } finally {
          if (!_moved4) orderBy.drop();
        }
      } finally {
        if (!_moved2) dropOwned(_b1);
      }
    } finally {
      if (!_moved0) dropOwned(items);
    }
  });

});
