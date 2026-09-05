// Runs the emitted format_strings against the real runtime. What is under test
// is what each placeholder renders: `{}` is Display and `{:?}` is Debug, and
// the two are different strings for the same value. A placeholder names its
// argument by position, by name, or — since Rust 2021 — by naming a variable.

import { expect, test } from 'bun:test';
import { Lines, Parts, Peer, Size, absent, braces, captured, debugged, greeting, named, positional, refuse } from './input.ts';
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

test('a Display that writes several times composes all of them', () => {
  // `write!(f, "a")?; write!(f, "b")` answered `"b"`: only the statement form
  // with a `?` appended, and the tail write replaced the accumulator.
  const parts = new Parts('head', 'tail');
  expect(parts.toString()).toBe('[head|tail]');
  parts.drop();
});

test('a Display ending in Ok(()) answers what it wrote, including the semicolon form', () => {
  const lines = new Lines('first');
  expect(lines.toString()).toBe('first\nend');
  lines.drop();
});

test('a placeholder with no argument is written as undefined and reported', () => {
  expect(absent(1)).toBe('1 undefined');
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});

test('a `return write!(..)` inside a Display appends and then answers', () => {
  const small = new Size(7);
  expect(small.toString()).toBe('Size(7)');
  small.drop();
  // The defective path: the early `return` used to make what it wrote the whole
  // answer, so this was `big)` and the `Size(` before it was thrown away.
  const big = new Size(200);
  expect(big.toString()).toBe('Size(big)');
  big.drop();
});
