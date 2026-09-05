// Runs the emitted derived_equality against the real runtime. Every comparison
// below the top level used to be `.equals()`, and a `Uint8Array`, an array and
// a `HashSet` have none — so each of these threw a TypeError the moment
// something asked whether two values were the same, which is what proto's
// `StateBuffers` and `OperationSet` did.

import { expect, test } from 'bun:test';
import { HashMap, HashSet } from '@ankurah/base';
import { Buffers, Groups, Holder, Marked, Maybe, Nested, Paired, Slot, Sparse, Tag } from './input.ts';
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

test('a tuple field is compared position by position, and cloned the same way', () => {
  // At the parent this called `.equals()` on a JavaScript array.
  const make = () =>
    new Paired([1, new Tag('a')], [['k', new Tag('b')]], [[new Tag('c')], true], [new Tag('d')]);
  const a = make();
  const b = make();
  expect(a.equals(b)).toBe(true);
  const c = new Paired([2, new Tag('a')], [['k', new Tag('b')]], [[new Tag('c')], true], [new Tag('d')]);
  expect(a.equals(c)).toBe(false);
  const d = new Paired([1, new Tag('a')], [['k', new Tag('b')]], null, [new Tag('d')]);
  expect(a.equals(d)).toBe(false);
  // A one-element tuple is a tuple: the clone writer used to hand it to
  // `.clone()` on an array, which is a TypeError.
  const copy = a.clone();
  expect(copy.equals(a)).toBe(true);
  expect(copy.single[0]).not.toBe(a.single[0]);
  a.drop(); b.drop(); c.drop(); d.drop(); copy.drop();
});

test('a field written as a type PARAMETER is compared and copied at run time', () => {
  // A struct, which the fourth pass fixed, and an ENUM, which it did not.
  const numbers = new Holder(1, [2, 3]);
  const alike = new Holder(1, [2, 3]);
  const unlike = new Holder(1, [2, 4]);
  expect(numbers.equals(alike)).toBe(true);
  expect(numbers.equals(unlike)).toBe(false);
  const numbersCopy = numbers.clone();
  expect(numbersCopy.many).toEqual([2, 3]);
  numbers.drop(); alike.drop(); unlike.drop(); numbersCopy.drop();

  const one = new Slot<number>('One', { _0: 7 });
  const same = new Slot<number>('One', { _0: 7 });
  const other = new Slot<number>('One', { _0: 8 });
  // At the parent each of these called `.equals()` on a number.
  expect(one.equals(same)).toBe(true);
  expect(one.equals(other)).toBe(false);
  const oneCopy = one.clone();
  expect(oneCopy.value).toEqual({ _0: 7 });
  same.drop(); other.drop(); oneCopy.drop();

  const many = new Slot<number>('Many', { _0: [1, 2] });
  const manyAlike = new Slot<number>('Many', { _0: [1, 2] });
  expect(many.equals(manyAlike)).toBe(true);
  const manyCopy = many.clone();
  expect(manyCopy.value).toEqual({ _0: [1, 2] });
  const empty = new Slot<number>('Empty', {});
  expect(empty.equals(one)).toBe(false);
  many.drop(); manyAlike.drop(); manyCopy.drop(); empty.drop(); one.drop();
});

test('and the same parameter instantiated with a class still copies deeply', () => {
  const tags = new Slot<Tag>('Many', { _0: [new Tag('a')] });
  const copy = tags.clone();
  expect(copy.equals(tags)).toBe(true);
  expect((copy.value as { _0: Tag[] })._0[0]).not.toBe((tags.value as { _0: Tag[] })._0[0]);
  copy.drop();
  tags.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
