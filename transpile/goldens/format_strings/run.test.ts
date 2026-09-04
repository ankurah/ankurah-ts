// Runs the emitted format_strings against the real runtime. What is under test
// is what each placeholder renders: `{}` is Display and `{:?}` is Debug, and
// the two are different strings for the same value. A placeholder names its
// argument by position, by name, or — since Rust 2021 — by naming a variable.

import { expect, test } from 'bun:test';
import { Peer, braces, captured, debugged, greeting, named, positional, refuse } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('{} renders the value itself', () => {
  const peer = new Peer(3, 'ada');
  expect(greeting(peer)).toBe('hello ada');
  peer.drop();
});

test('a positional placeholder may be written more than once', () => {
  expect(positional(1, 2)).toBe('1 then 2, and 1 again');
});

test('a named argument is read by its name', () => {
  const peer = new Peer(9, 'grace');
  expect(named(peer)).toBe('grace is 9');
  peer.drop();
});

test('Rust 2021 captures the variable a placeholder names', () => {
  expect(captured('here')).toBe('captured here');
});

test('{:?} renders through Debug, which is not Display', () => {
  const peer = new Peer(4, 'lin');
  expect(debugged(peer)).toBe('peer Peer { id: 4, name: "lin" }');
  expect(peer.toString()).toBe('lin#4');
  peer.drop();
});

test('escaped braces stay text', () => {
  expect(braces(8)).toBe('{8}');
});

test('panic! carries the same rendering', () => {
  expect(refuse(2)).toBe(2);
  expect(() => refuse(0)).toThrow('refusing 0');
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
