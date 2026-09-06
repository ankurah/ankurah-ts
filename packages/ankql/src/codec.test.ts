// TS-ONLY: what THIS reader does with a byte run that is not UTF-8.
//
// ankql carries its own copy of the reader — proto depends on ankql, so ankql
// cannot depend on proto — and owes the same answer as proto's.
//
// Which byte runs are refused, and why refusing them is the port's job, is
// `packages/base/__tests__/utf8.test.ts` — one table for every site that reads
// text. What is this reader's own is the rest: it turns the refusal into an
// error naming the offset the bad bytes were read from, rather than handing a
// string with U+FFFD in it to whatever asked for the field.

import { describe, test, expect } from 'bun:test';
import { BincodeReader, BincodeWriter } from './codec.ts';

/** A bincode string field: a u64 length, then the bytes. */
function encodedString(bytes: number[]): Uint8Array {
  const writer = new BincodeWriter();
  writer.writeLength(bytes.length);
  const head = writer.finish();
  const out = new Uint8Array(head.length + bytes.length);
  out.set(head, 0);
  out.set(Uint8Array.from(bytes), head.length);
  return out;
}

describe('BincodeReader.readString refuses bytes that are not UTF-8', () => {
  test('the refusal names the offset the bytes were read from', () => {
    const reader = new BincodeReader(encodedString([0xc0, 0xaf]));
    expect(() => reader.readString()).toThrow('2 bytes at offset 8 are not valid UTF-8');
  });

  test('and valid UTF-8 still reads', () => {
    const reader = new BincodeReader(encodedString([0xe2, 0x82, 0xac]));
    expect(reader.readString()).toBe('€');
  });
});
