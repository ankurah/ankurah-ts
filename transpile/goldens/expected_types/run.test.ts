// Runs the emitted expected_types against the real runtime. The point is the
// runtime types the position chose: a `Vec<u8>` is a `Uint8Array` and not a
// JavaScript array, and the two compare unequal, so a literal written into one
// of those positions has to come out as bytes. The widths themselves are
// invisible in JavaScript; what is visible is which container was built.

import { expect, test } from 'bun:test';
import { Header, lengths, nextLength, preamble, tag } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

test('a struct literal builds with the widths its fields declare', () => {
  const header = Header.first();
  expect(header.version).toBe(1);
  expect(header.length).toBe(512);
  header.drop();
});

test('a sequence literal expected as bytes is built as bytes', () => {
  const bytes = preamble();
  expect(bytes).toBeInstanceOf(Uint8Array);
  expect(bytes).toEqual(new Uint8Array([1, 2, 3, 4]));
});

test('an array literal behind a byte annotation is built as bytes too', () => {
  const bytes = tag();
  expect(bytes).toBeInstanceOf(Uint8Array);
  expect(bytes).toEqual(new Uint8Array([7, 8, 9, 10]));
});

test('an annotated sum keeps its own width', () => {
  const header = Header.first();
  expect(nextLength(header)).toBe(513);
  header.drop();
});

test('a hole the position closes still collects into an ordinary array', () => {
  const headers = [new Header(1, 10), new Header(2, 20)];
  const out = lengths(headers);
  expect(out).toEqual([10, 20]);
  expect(out).not.toBeInstanceOf(Uint8Array);
  for (const header of headers) header.drop();
});

test('nothing leaked and nothing was reported', async () => {
  await expectNoOwnershipReports();
});
