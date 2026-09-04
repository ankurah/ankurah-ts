// Runs the emitted impl_drop against the real runtime. What is under test is
// the cleanup body: `AkObject.drop()` must call it, and must call it while the
// fields are still alive, so the body sees `live` before the cascade releases
// anything.

import { expect, test } from 'bun:test';
import { Subscription } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a Subscription carries the fields the constructor was given', () => {
  const subscription = new Subscription('feed', true);
  expect(subscription.name).toBe('feed');
  expect(subscription.live).toBe(true);
  subscription.drop();
});

test('dropping a Subscription runs the cleanup body', () => {
  const subscription = new Subscription('feed', true);
  subscription.drop();
  expect(subscription.live).toBe(false);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
