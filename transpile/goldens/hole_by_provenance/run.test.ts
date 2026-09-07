// Runs the emitted hole_by_provenance against the real runtime. What is under
// test is whether the port can tell its own hole from a user function that
// happens to be called `unsupported`.
//
// `askedMissing()` takes the path the `?` exists for. The parent's engine read
// the rendered characters — `unsupported('missing')`, which is what a call with
// a string LITERAL looks like — called it a hole, and dropped the null test, so
// the emitted body ran `checkedAdd(null, 1, 'u32')` on a valid program.

import { expect, test } from 'bun:test';
import { askedMissing, askedPresent, unsupported } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the user function is the one that answers', () => {
  expect(unsupported('anything')).toBe(3);
  expect(unsupported('missing')).toBe(null);
});

test('the ? hands back the sum where the callee answered a value', () => {
  expect(askedPresent()).toBe(4);
});

test('the ? leaves with null where the callee answered none', () => {
  expect(askedMissing()).toBe(null);
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
