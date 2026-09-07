// Runs the emitted reserved_names against the real runtime.
//
// Against the parent's engine this file does not LOAD: `export function r#in`
// is not JavaScript, and `export function with` is not TypeScript. Written so
// that it did, `refuses()` would answer 7 — the user's own function — instead of
// throwing, because the declaration shadowed the hole helper and base's was
// never imported.

import { expect, test } from 'bun:test';
import { Holder, callsThem, in_, refuses, unsupported_, with_ } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a reserved word is escaped at the declaration as well as the reference', () => {
  expect(with_(1)).toBe(2);
  expect(in_(1)).toBe(3);
  expect(callsThem(1)).toBe(2 + 3 + 7);
});

test('the user function that spells the hole helper keeps its own answer', () => {
  expect(unsupported_('nothing')).toBe(7);
});

test('and the hole beside it still throws', () => {
  expect(() => refuses([1, 2])).toThrow('the port has no translation for this');
});

test('a METHOD called `with` is a property and is not renamed', () => {
  const holder = new Holder(4);
  expect(holder.with(3)).toBe(7);
  holder.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
