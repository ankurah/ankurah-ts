// Runs the emitted bytes_to_text against the real runtime. What is under test is
// that the port keeps Rust's two byte-to-text answers apart: the reader that
// REFUSES a byte run which is not UTF-8, and the one that substitutes U+FFFD
// because the source asked it to.
//
// The rows are the ones `packages/base/__tests__/utf8.test.ts` pins on the
// decoder itself, read here through the callers that use them.

import { expect, test } from 'bun:test';
import { readJson, readLossy } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const invalid: Record<string, number[]> = {
  'a continuation byte with no leader': [0x22, 0x80, 0x22],
  'a leader with no continuation': [0x22, 0xc3, 0x22],
  'a truncated three-byte sequence': [0x22, 0xe2, 0x82, 0x22],
  'an overlong encoding of "/"': [0x22, 0xc0, 0xaf, 0x22],
  'a byte no UTF-8 sequence starts with': [0x22, 0xff, 0x22],
};

for (const [what, bytes] of Object.entries(invalid)) {
  test(`a JSON string around ${what} is not read`, () => {
    // Read through the host's default decoder this is the string U+FFFD, which
    // then flows on as though Rust had read it. Rust answers None.
    expect(readJson(Uint8Array.from(bytes))).toBeNull();
  });

  test(`and the lossy reader answers the replacement character for ${what}`, () => {
    expect(readLossy(Uint8Array.from(bytes))).toContain('�');
  });
}

test('valid UTF-8 reads the same either way', () => {
  const quoted = Uint8Array.from([0x22, 0xe2, 0x82, 0xac, 0x22]);
  expect(readJson(quoted)).toBe('€');
  expect(readLossy(quoted)).toBe('"€"');
});

test('and UTF-8 that is not JSON is still refused', () => {
  expect(readJson(Uint8Array.from([0x7b, 0x7b]))).toBeNull();
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
