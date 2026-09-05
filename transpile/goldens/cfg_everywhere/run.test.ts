// Runs the emitted cfg_everywhere against the real runtime. The point under
// test is arity and dispatch: a field the build leaves out must not stand in
// the constructor, a variant it leaves out must not stand in the match, and the
// `let` that survives must be the debug one — which is the branch the
// `debug_assertions = true` ruling keeps, and the branch the IndexedDB prefix
// guard was silently losing.

import { expect, test } from 'bun:test';
import { Bucket, Mode } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('the constructor takes only the fields this build has', () => {
  // Two parameters, not three: `never_here` is not a field here at all.
  expect(Bucket.length).toBe(2);
  const bucket = Bucket.new(4);
  expect(bucket.prefixLen).toBe(4);
  expect(bucket.guardDisabled).toBe(false);
  bucket.drop();
});

test('the `let` that survives is the debug one, and it reads the guard', () => {
  const guarded = new Bucket(4, true);
  // The debug branch answers 0 because the guard is disabled; the release
  // branch, which used to be the one read, would answer 4.
  expect(guarded.effective(true)).toBe(0);
  guarded.drop();
  const open = new Bucket(4, false);
  expect(open.effective(true)).toBe(4);
  expect(open.effective(false)).toBe(0);
  open.drop();
});

test('the method this build has is the debug one', () => {
  const bucket = Bucket.new(4);
  expect(bucket.checked()).toBe(5);
  bucket.drop();
});

test('a variant the build leaves out is neither in the union nor in the match', () => {
  const bucket = Bucket.new(1);
  expect(bucket.describe(new Mode('Fast', {}))).toBe(0);
  expect(bucket.describe(new Mode('Checked', {}))).toBe(1);
  bucket.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
