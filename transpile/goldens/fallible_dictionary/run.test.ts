// Runs the dictionaries the emitted call sites wrote (spec 4.4b).
//
// Three properties, each of which the engine got wrong before this golden was
// written. A concrete site whose conversion is a genuine `TryFrom` calls that
// impl and hands its `Result` over as it stands — the site used to ask the
// table for a `From`, find none, and write a hole that threw the moment `pick`
// read the dictionary. A concrete site whose conversion is an infallible `From`
// still has its answer wrapped. And a site the engine cannot name a type for
// throws, where it used to hand over the conversion belonging to the argument
// AFTER the one it could not type.

import { expect, test } from 'bun:test';
import { Sel, counted, parsed, refused, shifted } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a fallible concrete site calls the TryFrom impl and reads its Result', () => {
  const sel = parsed();
  expect(sel).toBeInstanceOf(Sel);
  expect(sel!.text).toBe('year >= 2020');
  sel!.drop();
});

test('and the same impl answers Err where Rust does', () => {
  expect(refused()).toBe(null);
});

test('an infallible concrete site still has its answer wrapped', () => {
  const sel = counted();
  expect(sel!.text).toBe('7');
  sel!.drop();
});

test('a conversion the engine cannot name is a hole that throws', () => {
  // At the parent this returned 7n: the block's type was dropped from the
  // argument list, the `7i64` after it moved into its place, and the call was
  // handed `From<i64>` — a conversion for a type standing nowhere in it.
  expect(() => shifted()).toThrow();
});

test('nothing leaked', async () => {
  await expectNoOwnershipReports();
});
