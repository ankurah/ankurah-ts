// TS-ONLY: the derived Ord on the provided ids.
//
// Every one of the six carries `#[derive(Ord, PartialOrd)]` in Rust. For the five
// ULID-backed ones — EntityId, TransactionId, RequestId, QueryId, UpdateId — the derive
// orders the inner `Ulid`, whose own Ord is over its `u128`; `Ulid::to_bytes` is
// `self.0.to_be_bytes()`, so the 16 bytes each of these stores are that integer
// big-endian and comparing them in order is the same comparison. EventId wraps `[u8; 32]`
// directly, and array Ord is lexicographic over unsigned bytes. Both come out as the same
// loop, which is what `compareTo` is.
//
// The independent check is the ULID string: Crockford Base32's alphabet is in ascending
// order and every ULID string is 26 characters, so string order is numeric order — and it
// is computed by code that shares nothing with compareTo.

import { describe, test, expect } from 'bun:test';
import {
  EntityId,
  EventId,
  QueryId,
  RequestId,
  TransactionId,
  UpdateId,
} from '../src/index';
import { ulidBytesToString } from '../src/id.provided';

const sign = (n: number): number => (n < 0 ? -1 : n > 0 ? 1 : 0);

/** An id of `width` bytes, `value` at index `at` and zeroes everywhere else. */
function oneByte(width: number, at: number, value: number): Uint8Array {
  const bytes = new Uint8Array(width);
  bytes[at] = value;
  return bytes;
}

interface Ordered {
  name: string;
  width: number;
  make(bytes: Uint8Array): { compareTo(other: never): number; equals(other: never): boolean; drop(): void };
}

// The five ULID-backed ids, plus EventId over its 32 bytes.
const ULID_BACKED: Ordered[] = [
  { name: 'EntityId', width: 16, make: (b) => EntityId.fromBytes(b) },
  { name: 'TransactionId', width: 16, make: (b) => TransactionId.fromBytes(b) },
  { name: 'RequestId', width: 16, make: (b) => RequestId.fromBytes(b) },
  { name: 'QueryId', width: 16, make: (b) => QueryId.fromBytes(b) },
  { name: 'UpdateId', width: 16, make: (b) => UpdateId.fromBytes(b) },
];
const ALL: Ordered[] = [...ULID_BACKED, { name: 'EventId', width: 32, make: (b) => EventId.fromBytes(b) }];

/** Compare two ids built from these bytes, and release both. */
function order(kind: Ordered, left: Uint8Array, right: Uint8Array): number {
  const a = kind.make(left);
  const b = kind.make(right);
  try {
    return sign(a.compareTo(b as never));
  } finally {
    a.drop();
    b.drop();
  }
}

describe('id ordering', () => {
  for (const kind of ALL) {
    test(`${kind.name} lets the most significant byte decide`, () => {
      // A one in the first byte beats 255s in every byte after it.
      const high = oneByte(kind.width, 0, 1);
      const low = new Uint8Array(kind.width).fill(255);
      low[0] = 0;
      expect(order(kind, high, low)).toBe(1);
      expect(order(kind, low, high)).toBe(-1);
    });

    test(`${kind.name} compares its bytes as unsigned`, () => {
      // 0x80 is -128 read as a signed byte, and 128 read as Rust reads a u8.
      expect(order(kind, oneByte(kind.width, 3, 0x80), oneByte(kind.width, 3, 0x7f))).toBe(1);
      expect(order(kind, oneByte(kind.width, 3, 0xff), oneByte(kind.width, 3, 0x01))).toBe(1);
    });

    test(`${kind.name} answers 0 for equal ids and only for those`, () => {
      const same = oneByte(kind.width, 5, 9);
      expect(order(kind, same, same)).toBe(0);
      expect(order(kind, same, oneByte(kind.width, 5, 10))).toBe(-1);
      // compareTo and equals agree, which is what Ord and Eq being derived together means.
      const a = kind.make(same);
      const b = kind.make(new Uint8Array(same));
      expect([a.equals(b as never), sign(a.compareTo(b as never))]).toEqual([true, 0]);
      a.drop();
      b.drop();
    });

    test(`${kind.name} orders a last-byte difference too`, () => {
      const last = kind.width - 1;
      expect(order(kind, oneByte(kind.width, last, 2), oneByte(kind.width, last, 1))).toBe(1);
    });
  }

  test('sorting the ULID-backed ids agrees with their ULID strings', () => {
    // Bytes chosen to exercise the high byte, a middle byte and the low byte, with
    // values on both sides of 0x7f.
    const samples = [
      Uint8Array.from([0x01, 0x8f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02]),
      Uint8Array.from([0x01, 0x8f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]),
      Uint8Array.from([0x7f, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
      Uint8Array.from([0x80, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
      Uint8Array.from([0x00, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ];
    const byString = [...samples].sort((a, b) =>
      ulidBytesToString(a) < ulidBytesToString(b) ? -1 : ulidBytesToString(a) > ulidBytesToString(b) ? 1 : 0,
    );

    for (const kind of ULID_BACKED) {
      const ids = samples.map((bytes) => ({ bytes, id: kind.make(bytes) }));
      ids.sort((a, b) => a.id.compareTo(b.id as never));
      expect([kind.name, ids.map((e) => ulidBytesToString(e.bytes))])
        .toEqual([kind.name, byString.map((b) => ulidBytesToString(b))]);
      for (const entry of ids) entry.id.drop();
    }
  });
});
