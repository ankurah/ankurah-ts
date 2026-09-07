// Runs the emitted hole_by_provenance against the real runtime. What is under
// test is whether the port can tell its own hole from a user function that
// happens to be called `unsupported` — and which the port RENAMES, because
// `unsupported` is the one name it writes into a body without the source asking
// for it (BB1). Left as it was, the declaration shadowed base's helper and every
// hole in the file answered a value instead of throwing.
//
// `askedMissing()` takes the path the `?` exists for. The parent's engine read
// the rendered characters — `unsupported('missing')`, which is what a call with
// a string LITERAL looks like — called it a hole, and dropped the null test, so
// the emitted body ran `checkedAdd(null, 1, 'u32')` on a valid program.

import { expect, test } from 'bun:test';
import { askedMissing, askedPresent, unsupported_ } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the user function is the one that answers', () => {
  expect(unsupported_('anything')).toBe(3);
  expect(unsupported_('missing')).toBe(null);
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
