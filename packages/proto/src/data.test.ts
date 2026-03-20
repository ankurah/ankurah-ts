// MIRRORS: ankurah/proto/src/data.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EventId } from './data';
import { BincodeWriter, BincodeReader } from './codec';

describe('data unit tests', () => {
  test('test_event_id_json_serialization', () => {
    // Rust serde_json uses custom Serialize → base64url no-pad
    using id = EventId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    const base64 = id.toBase64();
    expect(base64).toEqual('AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA');
    using roundTrip = EventId.fromBase64(base64);
    expect(id.equals(roundTrip)).toBe(true);
  });

  test('test_event_id_bincode_serialization', () => {
    // Rust bincode uses custom Serialize → raw 32 bytes
    using id = EventId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    const writer = new BincodeWriter();
    id.encode(writer);
    const bytes = writer.finish();
    expect(Array.from(bytes)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    const reader = new BincodeReader(bytes);
    using decoded = EventId.decode(reader);
    expect(id.equals(decoded)).toBe(true);
  });
});
