// Runs the emitted hole_releases_payload against the real runtime. The arm
// refuses — which is R12 doing its job — and the question here is what it does
// with the payload on its way out. At the parent it threw and left the `Inner`,
// both its `Token`s and the trailing `Token` to nobody.

import { expect, test } from 'bun:test';
import { UnsupportedShape } from '@ankurah/base';
import { Counts, Inner, Name, Token, Wrap, pick } from './input.ts';
import { HashMap, Result } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';

test('the arm refuses, and releases everything it was handed first', () => {
  const w = new Wrap('Held', {
    _0: new Inner('A', { _0: [Token.new(1), Token.new(2)] }),
    _1: Token.new(3),
  });
  expect(() => pick(w)).toThrow(UnsupportedShape);
});

test('and a value the refusing arm does not match reaches the arm below it', () => {
  // D2: the refusal stands in the BRANCH, so the TEST still decides. Written
  // where the test goes, this would have thrown here too.
  expect(pick(new Wrap('Empty', {}))).toBe(0);
});

// J4: the same rule at a CALL. `or_default()` on a value type with no default
// is refused, and the hole throws before the entry is ever made — so the key
// the call would have moved into the map is still the block's. At the parent
// the key was marked moved, nothing released it, and the leak check reported it.
test('a refused call releases what it would have consumed', () => {
  const counts = new Counts(new HashMap<Name, Result<number, number>>());
  const key = new Name('a');
  expect(() => counts.finish(key)).toThrow(UnsupportedShape);
  // The key was released on the way out, so a second drop is a double drop.
  expect(key.isDropped).toBe(true);
  counts.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
