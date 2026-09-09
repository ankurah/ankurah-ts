// Runs the emitted a_local_typed_inside_an_adaptor against the runtime.
//
// The closure's parameter at an adaptor is the item the receiver iterates, so
// `kept` has a type at its `let` and a guard, and the path that leaves before
// the loop releases the `Tag`s it holds.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { run } from './input.ts';

test('a local filled from inside an adaptor is released on the path that leaves early', () => {
  expect(run()).toBe(5n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
