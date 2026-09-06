import { describe, expect, test } from 'bun:test';
import {
  iterFilterMap,
  iterFind,
  iterFindMap,
  iterFirst,
  iterGet,
  iterLast,
  iterMaxBy,
  iterMaxByKey,
  iterMinBy,
  iterMinByKey,
  iterPosition,
  iterReduce,
  iterRposition,
  range,
  rangeIncl,
  stepBy,
} from '../src/std/iter.ts';

describe('the Option-returning iterator adaptors answer null, not a sentinel', () => {
  // J1's live case, in the shape the reactor's `remove` had it: the watcher had
  // already gone, `findIndex` answered -1, `-1 != null` read as present, and
  // `splice(-1, 1)` removed the last LIVE watcher.
  test('a position that is not there is null, so the caller does not splice(-1)', () => {
    const entries = ['a', 'b', 'c'];
    const pos = iterPosition(entries, (id) => id === 'gone');
    expect(pos).toBe(null);
    if (pos != null) entries.splice(pos, 1);
    expect(entries).toEqual(['a', 'b', 'c']);
  });

  test('a position that is there is its index', () => {
    expect(iterPosition(['a', 'b', 'c'], (id) => id === 'b')).toBe(1);
    expect(iterPosition([], () => true)).toBe(null);
  });

  test('rposition answers the LAST match', () => {
    expect(iterRposition(['a', 'b', 'a'], (x) => x === 'a')).toBe(2);
    expect(iterRposition(['a', 'b'], (x) => x === 'z')).toBe(null);
  });

  test('find answers null rather than undefined', () => {
    expect(iterFind([1, 2, 3], (n) => n > 1)).toBe(2);
    const missing = iterFind([1, 2, 3], (n) => n > 9);
    expect(missing).toBe(null);
    expect(missing === null).toBe(true); // what `undefined` would have failed
  });

  test('find_map answers the first Some the closure builds', () => {
    expect(iterFindMap([1, 2, 3], (n) => (n % 2 === 0 ? `even ${n}` : null))).toBe('even 2');
    expect(iterFindMap([1, 3], (n) => (n % 2 === 0 ? `even ${n}` : null))).toBe(null);
  });

  test('last, first and get answer null for a sequence that has none', () => {
    expect(iterLast([1, 2])).toBe(2);
    expect(iterLast([])).toBe(null);
    expect(iterFirst([1, 2])).toBe(1);
    expect(iterFirst([])).toBe(null);
    expect(iterGet([1, 2], 0)).toBe(1);
    expect(iterGet([1, 2], 5)).toBe(null);
    expect(iterGet(new Uint8Array([7, 8]), 1)).toBe(8);
    expect(iterGet(new Uint8Array([7, 8]), 9)).toBe(null);
  });

  // `Array.prototype.reduce` with no initial value throws on an empty array,
  // where Rust's `reduce` answers `None`.
  test('reduce answers null for an empty sequence rather than throwing', () => {
    expect(iterReduce([1, 2, 3], (a, b) => a + b)).toBe(6);
    expect(iterReduce([] as number[], (a, b) => a + b)).toBe(null);
  });
});

describe('the max/min families keep Rust tie-breaking', () => {
  const byLen = (a: string, b: string) => a.length - b.length;

  // std picks the LAST maximum and the FIRST minimum among equals, and the
  // difference is visible whenever the elements carry more than the comparison
  // reads.
  test('max_by keeps the last of a tie and min_by the first', () => {
    expect(iterMaxBy(['aa', 'b', 'cc'], byLen)).toBe('cc');
    expect(iterMinBy(['a', 'bb', 'c'], byLen)).toBe('a');
    expect(iterMaxBy([] as string[], byLen)).toBe(null);
    expect(iterMinBy([] as string[], byLen)).toBe(null);
  });

  test('max_by_key and min_by_key order primitives and compareTo alike', () => {
    expect(iterMaxByKey(['aa', 'b', 'cc'], (s) => s.length)).toBe('cc');
    expect(iterMinByKey(['aa', 'b', 'cc'], (s) => s.length)).toBe('b');

    class Rank {
      constructor(readonly n: number) {}
      compareTo(o: Rank): number {
        return this.n - o.n;
      }
    }
    expect(iterMaxByKey([3, 1, 2], (n) => new Rank(n))).toBe(3);
  });

  // Answering 0 for a key with no order would silently pick the first element.
  test('a key that declares no order raises rather than picking one', () => {
    expect(() => iterMaxByKey([1, 2], () => ({ nothing: true }))).toThrow(/declares no order/);
  });

  // F2. `Iterator::max_by` is `fold1(|best, candidate| cmp::max_by(best,
  // candidate, compare))`, and `cmp::max_by` asks `compare(&best, &candidate)`.
  // The port asked `cmp(candidate, best)`, which an ANTISYMMETRIC comparator
  // hides in the winner and nothing else does: a comparator with a side effect
  // sees the pairs reversed, and one whose two arguments mean different things
  // answers about the wrong one. The oracle is `cargo run` over the same
  // sequence: Rust's max_by logs (1,2) then (2,3), and its min_by (1,2) then
  // (1,3).
  test('the comparator is asked cmp(best, candidate), in that order', () => {
    const seen: Array<[number, number]> = [];
    const trace = (a: number, b: number) => {
      seen.push([a, b]);
      return a - b;
    };
    expect(iterMaxBy([1, 2, 3], trace)).toBe(3);
    expect(seen).toEqual([
      [1, 2],
      [2, 3],
    ]);

    seen.length = 0;
    expect(iterMinBy([1, 2, 3], trace)).toBe(1);
    expect(seen).toEqual([
      [1, 2],
      [1, 3],
    ]);
  });

  // Rust writes `max_by_key` as `self.map(|x| (f(&x), x)).max_by(..)`, which is
  // lazy: the key closure is called ONCE per element, in element order. Calling
  // it inside the comparator called it twice per comparison and in the reverse
  // order — six calls over three elements instead of three.
  test('the key closure is called once per element, in element order', () => {
    const seen: number[] = [];
    const key = (n: number) => {
      seen.push(n);
      return n;
    };
    expect(iterMaxByKey([1, 2, 3], key)).toBe(3);
    expect(seen).toEqual([1, 2, 3]);

    seen.length = 0;
    expect(iterMinByKey([1, 2, 3], key)).toBe(1);
    expect(seen).toEqual([1, 2, 3]);
  });

  // The tie rules read off the same fold: `Less | Equal` takes the candidate
  // for max (so the LAST equal element wins) and only `Greater` takes it for
  // min (so the FIRST does). With the arguments the other way round both rules
  // had to be written inverted to compensate, and only an antisymmetric
  // comparator made the compensation exact.
  test('a comparator that is not antisymmetric still tie-breaks Rust’s way', () => {
    const items = [
      { id: 'a', rank: 1 },
      { id: 'b', rank: 1 },
      { id: 'c', rank: 1 },
    ];
    // `best.rank - candidate.rank` is 0 throughout: every element ties.
    const byRank = (x: { rank: number }, y: { rank: number }) => x.rank - y.rank;
    expect(iterMaxBy(items, byRank)?.id).toBe('c');
    expect(iterMinBy(items, byRank)?.id).toBe('a');
    expect(iterMaxByKey(items, (i) => i.rank)?.id).toBe('c');
    expect(iterMinByKey(items, (i) => i.rank)?.id).toBe('a');
  });
});

describe('a range is the sequence of its values', () => {
  // The port has no `Range` type, and a range used as a VALUE was written
  // `undefined`: `for attempt in 0..MAX_RETRIES` emitted
  // `for (const attempt of undefined)`, which raises the first time it is
  // reached. `Entity::commit`'s retry loop is one of those.
  test('a half-open range stops before its end and a closed one includes it', () => {
    expect(range(0, 3)).toEqual([0, 1, 2]);
    expect(rangeIncl(0, 3)).toEqual([0, 1, 2, 3]);
    expect(range(2, 5)).toEqual([2, 3, 4]);
  });

  // E7: `step_by` had no lowering at all and came out as `xs.stepBy(..)`, a
  // method no array declares.
  test('step_by keeps every nth value, and refuses a step of zero as Rust does', () => {
    expect(stepBy([0, 1, 2, 3, 4, 5], 2)).toEqual([0, 2, 4]);
    expect(stepBy(range(0, 10), 3)).toEqual([0, 3, 6, 9]);
    expect(stepBy([1, 2], 5)).toEqual([1]);
    expect(stepBy([], 2)).toEqual([]);
    expect(() => stepBy([1], 0)).toThrow(/positive integer/);
    expect(() => stepBy([1], -1)).toThrow(/positive integer/);
  });

  test('an empty or reversed range is empty, as Rust\'s is', () => {
    expect(range(3, 3)).toEqual([]);
    expect(range(5, 2)).toEqual([]);
    expect(rangeIncl(3, 3)).toEqual([3]);
  });

  // `filter_map` was camel-cased to `xs.filterMap(..)`, a method no array
  // declares — twelve emitted sites.
  test('filter_map keeps what the closure answers Some for', () => {
    expect(iterFilterMap([1, 2, 3, 4], (n) => (n % 2 === 0 ? n * 10 : null))).toEqual([20, 40]);
    expect(iterFilterMap([1, 3], (n) => (n % 2 === 0 ? n : null))).toEqual([]);
  });
});
