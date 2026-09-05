// Runs the emitted hole_releases_payload against the real runtime. The arm
// refuses — which is R12 doing its job — and the question here is what it does
// with the payload on its way out. At the parent it threw and left the `Inner`,
// both its `Token`s and the trailing `Token` to nobody.

import { expect, test } from 'bun:test';
import { UnsupportedShape } from '@ankurah/base';
import { Inner, Token, Wrap, pick } from './input.ts';
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

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
