// Runs the emitted a_callback_held_in_a_field against the runtime.
//
// The closure captures a `Tag`, so the emitter wraps it in an `OwnedClosure`,
// the field is declared as the `Invocable` it holds, and the call through it
// goes through the helper.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { overACapture } from './input.ts';

test('a callback that owns its captures answers through the field that holds it', () => {
  expect(overACapture()).toBe(12n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
