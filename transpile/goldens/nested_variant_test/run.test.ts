// Runs the emitted nested_variant_test against the real runtime. Two things are
// under test.
//
// A pattern that takes NO name out of a value can still ask a question of it.
// Read as one, `Some(Status::Requested(_, _))` became a bare `!= null`, so
// core's `client_relay` answered "requested" for any status that was there at
// all, and `Wrap::Inner(Status::Requested(_, _))` lost its inner test the same
// way.
//
// And a derived `hash()` joined its parts with a separator a `String` field can
// contain: `Pair('x|s:y', 'z')` and `Pair('x', 'y|s:z')` hashed alike, so two
// different keys landed in one bucket.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import { Id, Pair, Status, Wrap, isAnything, isRequested, wrapsRequested } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

function requested(): Status {
  return new Status('Requested', { _0: new Id(1), _1: 2 });
}

function established(): Status {
  return new Status('Established', { _0: new Id(1), _1: 2 });
}

test('a nameless inner pattern still asks its question', () => {
  const yes = requested();
  expect(isRequested(yes)).toBe(true);
  yes.drop();

  const no = established();
  expect(isRequested(no)).toBe(false);
  no.drop();

  const idle = new Status('Idle', {});
  expect(isRequested(idle)).toBe(false);
  idle.drop();

  expect(isRequested(null)).toBe(false);
});

test('and one member deep, inside a variant payload', () => {
  const yes = new Wrap('Inner', { _0: requested() });
  expect(wrapsRequested(yes)).toBe(true);
  yes.drop();

  const no = new Wrap('Inner', { _0: established() });
  expect(wrapsRequested(no)).toBe(false);
  no.drop();

  const other = new Wrap('Other', {});
  expect(wrapsRequested(other)).toBe(false);
  other.drop();
});

test('a nameless payload that asks nothing still asks nothing', () => {
  const any = established();
  expect(isAnything(any)).toBe(true);
  any.drop();
  expect(isAnything(null)).toBe(false);
});

test('two keys a separator would have merged are two keys', () => {
  // Joined by `|`, both of these hashed to `s:x|s:y|s:z`.
  const first = new Pair('x|s:y', 'z');
  const second = new Pair('x', 'y|s:z');
  expect(first.hash()).not.toBe(second.hash());

  const map = new HashMap<Pair, number>();
  map.set(first, 1);
  map.set(second, 2);
  expect(map.size).toBe(2);

  const askFirst = new Pair('x|s:y', 'z');
  const askSecond = new Pair('x', 'y|s:z');
  expect(map.get(askFirst)).toBe(1);
  expect(map.get(askSecond)).toBe(2);
  askFirst.drop();
  askSecond.drop();
  map.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
