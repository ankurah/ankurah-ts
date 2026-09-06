// Runs the emitted unit_struct_value against the real runtime. What is under
// test is that a unit struct written as a value reaches its own methods: the
// class object does not, and handing one to a parameter that calls a trait
// method is `TypeError: g.greeting is not a function`.

import { expect, test } from 'bun:test';
import { Loud, Mock, aMock, greetWith } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a unit struct in value position is a value of that type', () => {
  const mock = aMock();
  expect(mock).toBeInstanceOf(Mock);
  expect(mock.greeting()).toBe('mock');
  mock.drop();
});

test('and it reaches the trait method it was handed over for', () => {
  const mock = aMock();
  expect(greetWith(mock)).toBe('mock');
  mock.drop();
  const loud = new Loud();
  expect(greetWith(loud)).toBe('LOUD');
  loud.drop();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
