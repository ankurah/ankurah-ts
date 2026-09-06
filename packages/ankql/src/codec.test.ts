// TS-ONLY: the reader's own refusals, which no Rust source mirrors.
//
// The same rule as `packages/proto/src/codec.test.ts`, for ankql's own copy of
// the reader: the two codecs read the same bytes and owe the same answer.
//
// A Rust `String` is UTF-8 by construction, so a byte run that is not valid
// UTF-8 could not have come from one and `serde` errors there. A non-fatal
// `TextDecoder` answers U+FFFD instead — a different string that then flows on
// as though it had been read, which is a silent corruption where Rust reports.

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
  // Each of these is a byte run `serde` rejects and a lenient decoder turns
  // into U+FFFD.
  const cases: Record<string, number[]> = {
    'a continuation byte with no leader': [0x80],
    'a leader with no continuation': [0xc3],
    'a truncated three-byte sequence': [0xe2, 0x82],
    'an overlong encoding of "/"': [0xc0, 0xaf],
    'a byte no UTF-8 sequence starts with': [0xff],
  };
  for (const [what, bytes] of Object.entries(cases)) {
    test(what, () => {
      const reader = new BincodeReader(encodedString(bytes));
      expect(() => reader.readString()).toThrow('not valid UTF-8');
    });
  }

  test('and valid UTF-8 still reads', () => {
    const reader = new BincodeReader(encodedString([0xe2, 0x82, 0xac]));
    expect(reader.readString()).toBe('€');
  });
});
