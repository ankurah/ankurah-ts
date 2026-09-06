// TS-ONLY: the one decoder every reader of text in the port goes through.
//
// A Rust `String` is UTF-8 by construction, so a byte run that is not valid
// UTF-8 could not have come from one and Rust's own readers error there. The
// host's `TextDecoder` answers U+FFFD instead — a different string that then
// flows on as though it had been read, which is a silent corruption where Rust
// reports. `decodeUtf8` is the fatal decode, and every site that reads bytes
// into a string calls it: `serde_json.fromSlice`, and the bincode reader in
// each package's own `codec.ts`.
//
// The table below is what "fatal" has to mean, case by case. It lives here
// rather than beside each reader because it is a fact about TEXT, not about any
// one reader (port/ownership.md, "Text crossing into the port is UTF-8").

import { describe, expect, test } from 'bun:test';
import { decodeUtf8, decodeUtf8Lossy } from '../src/std/utf8.ts';

describe('decodeUtf8 refuses every byte run serde refuses', () => {
  const cases: Record<string, number[]> = {
    'a continuation byte with no leader': [0x80],
    'a leader with no continuation': [0xc3],
    'a truncated three-byte sequence': [0xe2, 0x82],
    'an overlong encoding of "/"': [0xc0, 0xaf],
    'a byte no UTF-8 sequence starts with': [0xff],
  };
  for (const [what, bytes] of Object.entries(cases)) {
    test(what, () => {
      expect(decodeUtf8(Uint8Array.from(bytes))).toBeNull();
    });
  }

  test('and valid UTF-8 reads', () => {
    expect(decodeUtf8(Uint8Array.from([0xe2, 0x82, 0xac]))).toBe('€');
    expect(decodeUtf8(Uint8Array.from([]))).toBe('');
    expect(decodeUtf8(Uint8Array.from([0x68, 0x69]))).toBe('hi');
  });

  test('a lone surrogate cannot be encoded in UTF-8, so no byte run decodes to one', () => {
    // The three-byte run a naive encoder would write for U+D800.
    expect(decodeUtf8(Uint8Array.from([0xed, 0xa0, 0x80]))).toBeNull();
  });
});

describe('decodeUtf8Lossy substitutes where decodeUtf8 refuses', () => {
  // The same table, read the other way: Rust has both answers, and which one a
  // site takes is the source's choice. `String::from_utf8_lossy` is what
  // `core/src/value/mod.rs:266` writes an arbitrary byte value out with, where
  // refusing would take the whole query down.
  const cases: Record<string, number[]> = {
    'a continuation byte with no leader': [0x80],
    'a leader with no continuation': [0xc3],
    'a truncated three-byte sequence': [0xe2, 0x82],
    'an overlong encoding of "/"': [0xc0, 0xaf],
    'a byte no UTF-8 sequence starts with': [0xff],
  };
  for (const [what, bytes] of Object.entries(cases)) {
    test(what, () => {
      const found = decodeUtf8Lossy(Uint8Array.from(bytes));
      expect(decodeUtf8(Uint8Array.from(bytes))).toBeNull();
      expect(found).toContain('\uFFFD');
    });
  }

  test('and valid UTF-8 reads the same either way', () => {
    for (const bytes of [[0xe2, 0x82, 0xac], [], [0x68, 0x69]]) {
      const run = Uint8Array.from(bytes);
      expect(decodeUtf8Lossy(run)).toBe(decodeUtf8(run) as string);
    }
  });

  test('the bytes around an invalid run are kept', () => {
    expect(decodeUtf8Lossy(Uint8Array.from([0x68, 0xff, 0x69]))).toBe('h\uFFFDi');
  });
});

describe('a leading byte-order mark', () => {
  // R2: `TextDecoder`'s default is `ignoreBOM: false`, whose name says the
  // opposite of what it does — it REMOVES a leading EF BB BF from the answer.
  // Rust's `from_utf8` and `from_utf8_lossy` keep U+FEFF as an ordinary
  // character, so the default silently dropped one from every string whose
  // bytes began with a mark. Both decoders now ask to keep it.
  const withMark = Uint8Array.from([0xef, 0xbb, 0xbf, 0x68, 0x69]);

  test('is kept by the fatal decoder, as `from_utf8` keeps it', () => {
    expect(decodeUtf8(withMark)).toBe('﻿hi');
  });

  test('is kept by the lossy decoder, as `from_utf8_lossy` keeps it', () => {
    expect(decodeUtf8Lossy(withMark)).toBe('﻿hi');
  });

  test('and one in the MIDDLE was never at risk, which says what changed', () => {
    const inTheMiddle = Uint8Array.from([0x68, 0xef, 0xbb, 0xbf, 0x69]);
    expect(decodeUtf8(inTheMiddle)).toBe('h﻿i');
    expect(decodeUtf8Lossy(inTheMiddle)).toBe('h﻿i');
  });

  test('a mark on its own is one character, not an empty string', () => {
    const alone = Uint8Array.from([0xef, 0xbb, 0xbf]);
    expect(decodeUtf8(alone)).toBe('﻿');
    expect(decodeUtf8Lossy(alone)).toBe('﻿');
  });
});
