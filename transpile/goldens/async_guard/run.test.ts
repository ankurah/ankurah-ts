// Runs the emitted async_guard against the real runtime.
//
// What this driver can check: the guard survives the await and is released
// whichever way the block leaves, so a later lock is granted rather than parked
// forever; two overlapping calls serialize instead of interleaving; and `race`
// releases both receivers and both branch futures, so neither the winner nor the
// loser is left for the leak registry.
//
// What it cannot check, and does not pretend to:
//
//   * Cancellation. Rust's `select!` drops the losing futures, which cancels
//     them; a losing Promise here runs to completion and does whatever it was
//     going to do. There is no observation that separates the two here.
//   * Which branch wins when both are ready. This runtime arbitrates by source
//     order at one checkpoint; tokio's unbiased `select!` picks at random among
//     the ready branches. Asserting "left wins" would pin the runtime's
//     determinism, not the Rust semantics, so the ready-at-once case below only
//     asserts that *some* branch won and that nothing leaked either way.

import { expect, test } from 'bun:test';
import { AsyncMutex, dropOwned, mpsc } from '@ankurah/base';
import { Gate, race, step } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('step is an ordinary async function that owns nothing', async () => {
  expect(await step()).toBe(1);
});

test('bump holds the guard across the await and releases it at the end', async () => {
  const gate = new Gate(new AsyncMutex(0));
  expect(await gate.bump()).toBe(1);
  // A second call is granted the lock, which is only possible because the first
  // guard was released — a leaked guard would park this call forever.
  expect(await gate.bump()).toBe(2);
  gate.drop();
});

test('two overlapping bumps serialize on the mutex', async () => {
  const gate = new Gate(new AsyncMutex(0));
  const seen = await Promise.all([gate.bump(), gate.bump(), gate.bump()]);
  // Each call read the counter after its own increment, so the three answers
  // are 1, 2, 3 in some order — never two calls seeing the same number.
  expect([...seen].sort()).toEqual([1, 2, 3]);
  gate.drop();
});

test('race takes the branch whose channel already has a message', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  // send() answers with a Result the caller owns, exactly as in Rust.
  dropOwned(await leftTx.send(7));
  expect(await race(leftRx, rightRx)).toBe(1);
  // race owns both receivers and released them; the senders are still ours.
  leftTx.drop();
  rightTx.drop();
});

test('race takes the other branch when that is the one with a message', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await rightTx.send(9));
  expect(await race(leftRx, rightRx)).toBe(2);
  leftTx.drop();
  rightTx.drop();
});

test('race with both branches ready picks one and leaks neither', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await leftTx.send(1));
  dropOwned(await rightTx.send(2));
  const winner = await race(leftRx, rightRx);
  expect(winner === 1 || winner === 2).toBe(true);
  leftTx.drop();
  rightTx.drop();
});

test('race resolves when a sender is dropped and the branch reports None', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  // Dropping the last sender on the left ends its receives with Rust's None,
  // which is still a branch that produced a value.
  leftTx.drop();
  const winner = await race(leftRx, rightRx);
  expect(winner).toBe(1);
  rightTx.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
