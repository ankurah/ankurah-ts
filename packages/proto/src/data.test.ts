// MIRRORS: ankurah/proto/src/data.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EventId } from './id';
import { BincodeWriter, BincodeReader } from './codec';

describe('data.rs unit tests', () => {
  test('test_event_id_json_serialization', () => {
    const id = EventId.fromBytes([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
      17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ]);
    // JSON serialization uses base64url-no-pad
    const json = JSON.stringify(id.toBase64());
    expect(json).toBe('"AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA"');
    // Round-trip: parse the JSON string and reconstruct
    const parsed = JSON.parse(json) as string;
    const id2 = EventId.fromBase64(parsed);
    expect(id.equals(id2)).toBe(true);
  });

  test('test_event_id_bincode_serialization', () => {
    const id = EventId.fromBytes([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
      17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ]);
    // Bincode serialization: raw 32 bytes, no length prefix
    const writer = new BincodeWriter();
    id.encode(writer);
    const bytes = writer.finish();
    expect(Array.from(bytes)).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
      17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ]);
    // Round-trip: decode and verify equality
    const reader = new BincodeReader(bytes);
    const id2 = EventId.decode(reader);
    expect(id.equals(id2)).toBe(true);
  });
});
