// Runs the emitted option_payload against the real runtime. Against the parent
// engine (c2e2b2d) every one of these bodies is a hole: `readLoosely` still is,
// and the rest throw `UnsupportedShape` where Rust answers.

import { expect, test } from 'bun:test';
import { OwnershipFatal, clearFatalLatch } from '@ankurah/base';
import { Token, Value, read, readExact, readLoosely, peek } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an arm that takes a name out of the payload owns it', () => {
  const t = new Token(4);
  expect(read(new Value('Held', { _0: t }))).toBe(4);
  // `sink` takes the token BY VALUE and releases it at the end of its body.
  expect(t.isDropped).toBe(true);
});

test('the catch-all arm is handed the whole payload', () => {
  expect(read(new Value('Empty', {}))).toBe(7);
});

test('and None is the else', () => {
  expect(read(null)).toBe(0);
  expect(readExact(null)).toBe(0);
});

test('the exact form, with no catch-all', () => {
  const t = new Token(5);
  expect(readExact(new Value('Held', { _0: t }))).toBe(5);
  expect(t.isDropped).toBe(true);
  expect(readExact(new Value('Empty', {}))).toBe(1);
});

test('a borrowed match takes nothing apart', () => {
  const held = new Value('Held', { _0: new Token(1) });
  expect(peek(held)).toBe(1);
  held.drop();
});

test('a `_` that covers both halves at once is still a hole', () => {
  const held = new Value('Held', { _0: new Token(2) });
  expect(() => readLoosely(held)).toThrow(/tests inside the payload/);
  held.drop();
  clearFatalLatch();
});

test('nothing leaked', async () => {
  expect(OwnershipFatal).toBeDefined();
  await expectNoOwnershipReports();
});
