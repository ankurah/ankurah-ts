// MIRRORS: ankurah/proto/src/id.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EntityId } from './id';
import { BincodeWriter, BincodeReader } from './codec';

describe('id.rs unit tests', () => {
  test('test_entity_id_json_serialization', () => {
    const id = EntityId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    // JSON serialization uses base64url-no-pad
    const json = JSON.stringify(id.toBase64());
    expect(json).toBe('"AQIDBAUGBwgJCgsMDQ4PEA"');
    // Round-trip: parse the JSON string and reconstruct
    const parsed = JSON.parse(json) as string;
    const id2 = EntityId.fromBase64(parsed);
    expect(id.equals(id2)).toBe(true);
  });

  test('test_entity_id_bincode_serialization', () => {
    const id = EntityId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    // Bincode serialization: raw 16 bytes, no length prefix
    const writer = new BincodeWriter();
    id.encode(writer);
    const bytes = writer.finish();
    expect(Array.from(bytes)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    // Round-trip: decode and verify equality
    const reader = new BincodeReader(bytes);
    const id2 = EntityId.decode(reader);
    expect(id.equals(id2)).toBe(true);
  });
});
