// Runs the emitted a_wildcard_local against the real runtime.
//
// Against the parent engine each of the three functions leaks the ticket: the
// two that build one bound it to a `_` nothing releases, and the third was read
// as handing its local away, so the block released nothing either.

import { expect, test } from 'bun:test';
import { Desk, issueAndForget, readWithoutMoving, serveAndForget } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('an awaited value nobody binds is released at the semicolon', async () => {
  const desk = new Desk(0n);
  await serveAndForget(desk, 4n);
  expect(desk.issued).toBe(1n);
  desk.drop();
});

test('a built value nobody binds is released at the semicolon', () => {
  const desk = new Desk(0n);
  issueAndForget(desk, 5n);
  expect(desk.issued).toBe(1n);
  desk.drop();
});

test('a local read through a wildcard still belongs to the block, and is released once', () => {
  const desk = new Desk(0n);
  // A second read would throw `used after being dropped` if the wildcard had
  // taken the ticket away.
  expect(readWithoutMoving(desk)).toBe(7n);
  desk.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
