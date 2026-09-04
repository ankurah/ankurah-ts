// Runs the emitted blanket_free_fn against the real runtime. Two impls have no
// TypeScript class to be methods of — one written for a bare type parameter and
// one for an `Arc` — and both are emitted as module-level functions taking the
// receiver first. What is under test is that those functions exist, that they
// are reachable under the names the scheme gives them, and that the one taking
// its receiver by value releases it, as `fn into_listener(self)` does in Rust.

import { expect, test } from 'bun:test';
import { Arc } from '@ankurah/base';
import { Arc_Inner_intoListener, Inner, Listener, fromAny, fromWrapped, intoListener } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an impl on a bare parameter is a function named after the method alone', () => {
  const listener = intoListener((tag: number) => tag + 1);
  expect(listener.tag).toBe(2);
  listener.drop();
});

test('an impl on a wrapper is named after its constructors, outside in', () => {
  const inner = Arc.new(new Inner(7));
  const listener = Arc_Inner_intoListener(inner);
  expect(listener.tag).toBe(7);
  listener.drop();
});

test('a call whose receiver the engine could name reaches the right function', () => {
  const inner = Arc.new(new Inner(9));
  const listener = fromWrapped(inner);
  expect(listener).toBeInstanceOf(Listener);
  expect(listener.tag).toBe(9);
  listener.drop();
});

// The two below are what the run-time dispatcher exists for. `fromAny` is
// written for any `L: IntoListener`, and which impl that is cannot be decided
// where the function is emitted — so the emitted call asks the receiver's shape.
// Before the dispatcher, both of these reached the closure impl and the second
// one called an `Arc` as if it were a function.
test('a call through an open bound reaches the closure impl for a closure', () => {
  const listener = fromAny((tag: number) => tag + 10);
  expect(listener.tag).toBe(11);
  listener.drop();
});

test('the same call reaches the Arc impl for an Arc', () => {
  const inner = Arc.new(new Inner(3));
  const listener = fromAny(inner as any);
  expect(listener).toBeInstanceOf(Listener);
  expect(listener.tag).toBe(3);
  listener.drop();
});

test('a receiver no impl is written for is fatal rather than silent', () => {
  expect(() => fromAny(42 as any)).toThrow(/no IntoListener impl/);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
