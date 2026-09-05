// The real driver for this golden is the EMITTED `input.test.ts` beside it:
// `batch` writes it into the same directory, bun runs it, and at the parent
// every one of its async tests threw — on `Ok` as well as on `Err` — because
// `unwrap` was asked of the promise rather than of the value inside it. This
// file checks the same precedence from the other side, and that a failing
// async test still fails.

import { expect, test } from 'bun:test';
import { parse, parseNow } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an awaited Result answers its own methods', async () => {
  const answer = await parse('abc');
  expect(answer.isOk()).toBe(true);
  expect(answer.unwrap()).toBe(3);
});

test('and an awaited refusal is still a refusal', async () => {
  const answer = await parse('');
  expect(answer.isErr()).toBe(true);
  expect(answer.unwrapErr()).toBe('empty');
});

test('the sync form is unchanged', () => {
  const answer = parseNow('ab');
  expect(answer.unwrap()).toBe(2);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
