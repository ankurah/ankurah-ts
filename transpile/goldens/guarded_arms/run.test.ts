// Runs the emitted guarded_arms against the real runtime. A guard used to be
// reported and DROPPED, so its arm ran for every value its pattern matched:
// core's `node.rs:621` answered the event-bridge path for an empty bridge, and
// `context.rs:187` lost its cached-and-no-durable-peers arm entirely. These are
// the answers Rust gives.

import { expect, test } from 'bun:test';
import { Result } from '@ankurah/base';
import { Detail, Guarded, Refusal, Rich, Token, Weight, bridge, count, describe, guardedConsuming, heaviest, settle, settleRich } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a guard on a consuming arm decides, and the arm below it takes the rest', () => {
  expect(guardedConsuming(new Guarded('Same', { _0: Token.new(5), _1: true }))).toBe(1);
  // The guard fails: the arm below runs, and the token it binds is the same one.
  expect(guardedConsuming(new Guarded('Same', { _0: Token.new(0), _1: true }))).toBe(2);
  expect(guardedConsuming(new Guarded('Same', { _0: Token.new(9), _1: false }))).toBe(2);
  expect(guardedConsuming(new Guarded('Other', {}))).toBe(0);
});

test('a guard on a borrowed arm falls through to the arm below', () => {
  const big = new Weight('Light', { _0: 42 });
  expect(heaviest(big)).toBe(10);
  big.drop();
  const small = new Weight('Light', { _0: 3 });
  expect(heaviest(small)).toBe(3);
  small.drop();
  const heavy = new Weight('Heavy', { _0: 7 });
  expect(heaviest(heavy)).toBe(99);
  heavy.drop();
});

test('a guarded Err arm is not lost (core context.rs:187)', () => {
  // cached: the guard holds and the arm answers Ok.
  expect(settle(Result.Err(new Refusal('Empty', {})), true).unwrap()).toBe(0);
  // not cached: the guard fails and the arm below hands the error on.
  const passed = settle(Result.Err(new Refusal('Empty', {})), false);
  expect(passed.isErr()).toBe(true);
  passed.unwrapErr().drop();
  // a different error: the pattern does not match, so the arm below runs.
  const other = settle(Result.Err(new Refusal('Late', {})), true);
  expect(other.isErr()).toBe(true);
  other.unwrapErr().drop();
  expect(settle(Result.Ok(4), true).unwrap()).toBe(4);
});

test('an empty event bridge takes the snapshot path (core node.rs:621)', () => {
  expect(bridge(3)).toBe(1);
  // The defective answer: an EMPTY bridge used to take the bridge path.
  expect(bridge(0)).toBe(0);
});

test('a guarded arm whose body does not return stops the arms below', () => {
  const into: number[] = [];
  const big = new Weight('Light', { _0: 9 });
  count(big, into);
  big.drop();
  const small = new Weight('Light', { _0: 1 });
  count(small, into);
  small.drop();
  const heavy = new Weight('Heavy', { _0: 4 });
  count(heavy, into);
  heavy.drop();
  expect(into).toEqual([9, 0, 8]);
});

test('a guard in a value position', () => {
  expect(describe(new Weight('Light', { _0: 0 }))).toBe('nothing');
  expect(describe(new Weight('Light', { _0: 4 }))).toBe('light 4');
  expect(describe(new Weight('Heavy', { _0: 4 }))).toBe('heavy 4');
});

test('an arm that tests inside the payload AND names part of it refuses, in the branch', () => {
  // The refusal stands where the ARM would have run, so a value its pattern
  // does not match still reaches the arm below it (R12, and D2's rule that a
  // hole never stands in a condition).
  const late = settleRich(Result.Err(new Rich('Late', { _0: new Detail('late') })), true);
  expect(late.isErr()).toBe(true);
  late.unwrapErr().drop();
  // The refusal releases the payload it was handed before it throws (R12 does
  // not license abandoning what the branch owns), so the driver owes nothing.
  const empty = new Rich('Empty', { _0: new Detail('empty') });
  expect(() => settleRich(Result.Err(empty), true)).toThrow();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
