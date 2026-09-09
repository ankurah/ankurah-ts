// Runs the emitted a_local_typed_inside_an_adaptor against the runtime.
//
// Against 8ab991f's engine the closure's parameter is a projection nothing
// settles, so `kept` has no type at its `let` and no guard: the path that
// leaves before the loop releases none of the `Tag`s it holds.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { run } from './input.ts';

test('a local filled from inside an adaptor is released on the path that leaves early', () => {
  expect(run()).toBe(5n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
