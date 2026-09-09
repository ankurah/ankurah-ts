// Runs the emitted a_callback_held_in_a_field against the runtime.
//
// The closure captures a `Tag`, so the emitter wraps it in an `OwnedClosure`.
// Against 8ab991f's engine the field is written as a bare arrow and the call
// through it is a direct call, which an `OwnedClosure` is not.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import { overACapture } from './input.ts';

test('a callback that owns its captures answers through the field that holds it', () => {
  expect(overACapture()).toBe(12n);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
