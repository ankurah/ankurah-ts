// Runs the emitted derived_equality against the real runtime. Every comparison
// below the top level used to be `.equals()`, and a `Uint8Array`, an array and
// a `HashSet` have none — so each of these threw a TypeError the moment
// something asked whether two values were the same, which is what proto's
// `StateBuffers` and `OperationSet` did.

import { expect, test } from 'bun:test';
import { HashMap, HashSet } from '@ankurah/base';
import { Buffers, Groups, Marked, Maybe, Nested, Sparse, Tag } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const bytes = (...ns: number[]) => new Uint8Array(ns);
const map = <V,>(entries: [string, V][]) => {
  const m = new HashMap<string, V>();
  for (const [k, v] of entries) m.set(k, v);
  return m;
};

test('a map of bytes is compared byte by byte', () => {
  const a = new Buffers(map([['x', bytes(1, 2)]]));
  const b = new Buffers(map([['x', bytes(1, 2)]]));
  const c = new Buffers(map([['x', bytes(1, 3)]]));
  const d = new Buffers(map([['y', bytes(1, 2)]]));
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  expect(a.equals(d)).toBe(false);
  a.drop(); b.drop(); c.drop(); d.drop();
});

test('a map of arrays is compared element by element, by the element rule', () => {
  const a = new Groups(map([['g', [new Tag('one'), new Tag('two')]]]));
  const b = new Groups(map([['g', [new Tag('one'), new Tag('two')]]]));
  const c = new Groups(map([['g', [new Tag('one'), new Tag('three')]]]));
  const d = new Groups(map([['g', [new Tag('one')]]]));
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  expect(a.equals(d)).toBe(false);
  a.drop(); b.drop(); c.drop(); d.drop();
});

test('a set is compared as a set', () => {
  const set = (...names: string[]) => {
    const s = new HashSet<Tag>();
    for (const n of names) s.insert(new Tag(n));
    return s;
  };
  const a = new Marked(set('one', 'two'));
  const b = new Marked(set('two', 'one'));
  const c = new Marked(set('one', 'three'));
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  a.drop(); b.drop(); c.drop();
});

test('an array of arrays is compared all the way down', () => {
  const a = new Nested([bytes(1), bytes(2, 3)]);
  const b = new Nested([bytes(1), bytes(2, 3)]);
  const c = new Nested([bytes(1), bytes(2, 4)]);
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  a.drop(); b.drop(); c.drop();
});

test('a nullable inside a container asks whether both are absent first', () => {
  const a = new Sparse(map<Tag | null>([['x', new Tag('one')], ['y', null]]));
  const b = new Sparse(map<Tag | null>([['x', new Tag('one')], ['y', null]]));
  const c = new Sparse(map<Tag | null>([['x', new Tag('one')], ['y', new Tag('two')]]));
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  a.drop(); b.drop(); c.drop();
});

test('a nullable field, and a nullable primitive field', () => {
  const a = new Maybe(new Tag('one'), 3);
  const b = new Maybe(new Tag('one'), 3);
  const c = new Maybe(null, 3);
  const d = new Maybe(new Tag('one'), null);
  expect(a.equals(b)).toBe(true);
  expect(a.equals(c)).toBe(false);
  expect(a.equals(d)).toBe(false);
  a.drop(); b.drop(); c.drop(); d.drop();
});

test('nothing leaked and nothing was dropped twice', () => {
  expectNoOwnershipReports();
});
