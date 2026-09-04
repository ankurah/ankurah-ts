// Runs the emitted select_value against the real runtime.
//
// What this driver checks is where the winning arm's value ends up. `firstOf`
// binds it, `doubled` passes it to a call, and `answer` returns from inside the
// arm — three positions that used to lose it or emit something that did not
// parse. Each call also hands both receivers to the function, so a branch
// future or a receiver the emitter forgot shows up as a leak.
//
// What it cannot check, as `async_guard`'s driver already says: a losing branch
// is not cancelled here the way tokio cancels a dropped future, and which
// branch wins when both are ready is this runtime's arbitration order rather
// than tokio's. Every case below makes exactly one branch ready.

import { expect, test } from 'bun:test';
import { dropOwned, mpsc } from '@ankurah/base';
import { answer, doubled, firstOf, lastWord, twice } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('twice is an ordinary function that owns nothing', () => {
  expect(twice(3)).toBe(6);
});

test('the value the winning arm produced reaches the let that binds it', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  // send() answers with a Result the caller owns, exactly as in Rust.
  dropOwned(await leftTx.send(5));
  // The left arm produces 1, and the body multiplies what it bound by ten.
  expect(await firstOf(leftRx, rightRx)).toBe(10);
  leftTx.drop();
  rightTx.drop();
});

test('the other arm binds its own value', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await rightTx.send(5));
  expect(await firstOf(leftRx, rightRx)).toBe(20);
  leftTx.drop();
  rightTx.drop();
});

test('the value reaches a call written around the select', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await leftTx.send(5));
  // The left arm produces 3, and twice doubles it.
  expect(await doubled(leftRx, rightRx)).toBe(6);
  leftTx.drop();
  rightTx.drop();
});

test('the value reaches the caller when the select is the last thing in the body', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await rightTx.send(5));
  expect(await lastWord(leftRx, rightRx)).toBe(6);
  leftTx.drop();
  rightTx.drop();
});

test('an arm that returns leaves the function around the select, not the arm', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await leftTx.send(5));
  // 7 is the arm's own return; 0 is what the function says if it falls through.
  expect(await answer(leftRx, rightRx)).toBe(7);
  leftTx.drop();
  rightTx.drop();
});

test('the other returning arm leaves the same way', async () => {
  const [leftTx, leftRx] = mpsc.channel<number>(1);
  const [rightTx, rightRx] = mpsc.channel<number>(1);
  dropOwned(await rightTx.send(5));
  expect(await answer(leftRx, rightRx)).toBe(8);
  leftTx.drop();
  rightTx.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
