// Runs the emitted a_capture_named_only_inside_a_macro against the real runtime.
// Against the parent engine the block released both `Arc`s on the way out and
// `greet()` read one: `BUG: Arc<Name> was used after being moved`.

import { expect, test } from 'bun:test';
import { greeter } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the closure still holds what the block handed it', () => {
  const held = greeter('Alice', 'Smith');
  expect(held.greet()).toBe('Alice Smith');
  held.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
