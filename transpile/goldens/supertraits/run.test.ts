// Runs the emitted supertraits against the real runtime and, as every golden
// does, past `tsc`. The type check is what this one is really for: a call
// through a bound resolves to the supertrait's method, and without `extends` on
// the emitted interface `tsc` reports TS2339 on the type parameter — code that
// runs and does not compile.

import { expect, test } from 'bun:test';
import { One, ask } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a call through a bound reaches the supertrait method', () => {
  const one = new One();
  expect(ask(one)).toBe(1);
  one.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
