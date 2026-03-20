// MIRRORS: ankurah/proto/src/id.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EntityId } from './id';
import { BincodeWriter, BincodeReader } from './codec';

describe('id unit tests', () => {
  test('test_entity_id_json_serialization', () => {
    // Rust serde_json uses custom Serialize → base64url no-pad
    const id = EntityId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    const base64 = id.toBase64();
    expect(base64).toEqual('AQIDBAUGBwgJCgsMDQ4PEA');
    const roundTrip = EntityId.fromBase64(base64);
    expect(id.equals(roundTrip)).toBe(true);
    roundTrip.drop();
    id.drop();
  });

  test('test_entity_id_bincode_serialization', () => {
    // Rust bincode uses custom Serialize → raw 16 bytes
    const id = EntityId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    const writer = new BincodeWriter();
    id.encode(writer);
    const bytes = writer.finish();
    expect(Array.from(bytes)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    const reader = new BincodeReader(bytes);
    const decoded = EntityId.decode(reader);
    expect(id.equals(decoded)).toBe(true);
    decoded.drop();
    id.drop();
  });
});
