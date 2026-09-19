// Runs the emitted a_cloned_value_is_not_an_alias against the real runtime.
//
// Each function writes to what the copy handed back and then reads the map.
// Rust answers 10 in all three, because the write landed on the copy. Against
// the parent engine all three answer 5: the write reached the map's own value.

import { expect, test } from 'bun:test';
import { HashMap } from '@ankurah/base';
import {
  Dot,
  writeThroughACollection,
  writeThroughACopyStruct,
  writeThroughASequence,
} from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a write on a cloned sequence does not reach the map', () => {
  const map = new HashMap<bigint, bigint[]>();
  map.insert(1n, [10n, 20n]);
  expect(writeThroughASequence(map, 1n)).toBe(10n);
  map.drop();
});

test('a write on a copied struct does not reach the map', () => {
  const map = new HashMap<bigint, Dot>();
  map.insert(1n, new Dot(10n));
  expect(writeThroughACopyStruct(map, 1n)).toBe(10n);
  map.drop();
});

test('a write on one element of a copied collection does not reach the map', () => {
  const map = new HashMap<bigint, Dot>();
  map.insert(1n, new Dot(10n));
  expect(writeThroughACollection(map, 1n)).toBe(10n);
  map.drop();
});

test('nothing leaked beyond the copies a Copy type owes nobody', async () => {
  await expectNoOwnershipReports({
    except: [
      {
        report: 'BUG: Dot was garbage collected without being dropped.',
        owes: 'a `Copy` crate type is emitted as a tracked object while the engine answers ' +
          'that it owns nothing, so no scope releases a copy of one. An explicit `.clone()` ' +
          'of the same struct has always reported this way; the copy did not introduce it',
      },
      {
        report: 'BUG: Dot was garbage collected without being dropped.',
        owes: 'the second of the two copies: the collected form makes one per value',
      },
    ],
  });
});
