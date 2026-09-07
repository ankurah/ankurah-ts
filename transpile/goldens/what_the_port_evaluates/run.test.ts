// Runs the emitted what_the_port_evaluates against the real runtime.
//
// Against the parent's engine, `throughADeref` on a dropped handle and
// `reordered` with a `None` each leak the token the frame still owned, and
// `reassigned` leaks the value the local held before the assignment.

import { expect, test } from 'bun:test';
import { Handle, Token, reassigned, reordered, throughADeref } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

// A dropped `Handle` cannot be handed to `throughADeref` from here: the
// parameter is by value, so the function drops it too, and the double drop is
// the driver's mistake rather than the port's. What the deref changes is where
// the flag STANDS, which `ownership::lift_tests::
// an_auto_deref_beside_a_move_makes_the_move_conditional` reads off the text.
test('a live handle hands both values to the constructor', () => {
  const built = throughADeref(new Token(1n), Handle.new(3n));
  expect(built.n).toBe(3n);
  built.drop();
});

test('the first field written is evaluated first', () => {
  expect(() => reordered(new Token(2n), null)).toThrow();
});

test('and the values still reach the constructor in declaration order', () => {
  const built = reordered(new Token(2n), 9n);
  expect(built.token.n).toBe(2n);
  expect(built.n).toBe(9n);
  built.drop();
});

test('an assignment releases the value the local held', () => {
  expect(reassigned(new Token(4n), true, 5n)).toBe(5n);
});

test('and leaves it alone when the branch does not run', () => {
  expect(reassigned(new Token(4n), false, 5n)).toBe(5n);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
