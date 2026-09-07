// Runs the emitted loop_frame against the real runtime. What is under test is
// the scope of what a consuming loop records about the sequence it took.
//
// `shadowed` refuses at the `collect`, and the vector standing under the same
// NAME as the loop's sequence has to be released as the hole throws past it.
// Against the parent's engine that vector and every token in it were released
// by nobody, and the collector reported each one.

import { expect, test } from 'bun:test';
import { Token, shadowed, twice } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the replacement is released when the refusal throws past it', () => {
  expect(() => shadowed([new Token(1), new Token(2)], [new Token(3)])).toThrow('collect');
});

test('the loop above the refusal still releases every element it handed out', () => {
  expect(() => shadowed([new Token(1)], [new Token(2), new Token(3)])).toThrow('collect');
});

test('two loops over one spelling each release their own elements', () => {
  expect(twice([new Token(1), new Token(2)], [new Token(3)])).toBe(6);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
