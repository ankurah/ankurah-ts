// Runs the emitted default_and_expectation against the real runtime. Against
// the parent engine (c2e2b2d) `toBytes` answers a `number[]` from one arm and a
// `Uint8Array` from the other; every `unwrapOrDefault` is a `TypeError`,
// because nothing declares one; and `serde_json.Value` is a name
// `@ankurah/base` does not export at all.

import { expect, test } from 'bun:test';
import { Lit, bytesOrEmpty, countOrZero, jsonBytes, jsonNull, jsonOf, textOrEmpty, toBytes } from './input.ts';
import { Result } from '@ankurah/base';
import { expectNoOwnershipReports } from './leaks.ts';

test('every arm of one match answers what the match is expected to answer', () => {
  // `to_bytes` BORROWS, so both values stay the driver's.
  const flag = new Lit('Bool', { _0: true });
  const bits = toBytes(flag);
  // The defective answer: `[1]`, a plain array, from this arm alone.
  expect(bits).toBeInstanceOf(Uint8Array);
  expect([...bits]).toEqual([1]);
  flag.drop();
  const written = new Lit('Text', { _0: 'hi' });
  const text = toBytes(written);
  expect(text).toBeInstanceOf(Uint8Array);
  expect([...text]).toEqual([0, 1]);
  written.drop();
});

test('unwrap_or_default on a nullable is the payload’s default', () => {
  expect(textOrEmpty('x')).toBe('x');
  expect(textOrEmpty(null)).toBe('');
  expect(countOrZero(7)).toBe(7);
  expect(countOrZero(null)).toBe(0);
});

test('and on a Result it is unwrapOr, which releases the wrapper', () => {
  const ok = bytesOrEmpty(Result.Ok(new Uint8Array([1, 2])));
  expect([...ok]).toEqual([1, 2]);
  const err = bytesOrEmpty(Result.Err('no'));
  expect([...err]).toEqual([]);
});

test('a serde_json Value variant is the plain JavaScript value', () => {
  expect(jsonOf(true)).toBe(true);
  expect(jsonNull()).toBe(null);
  expect([...jsonBytes({ a: 1 })]).toEqual([...new TextEncoder().encode('{"a":1}')]);
  expect([...jsonBytes(jsonNull())]).toEqual([...new TextEncoder().encode('null')]);
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
