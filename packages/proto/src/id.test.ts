// MIRRORS: ankurah/proto/src/id.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EntityId } from './id';
import { BincodeWriter, BincodeReader } from './codec';

describe('id unit tests', () => {
  test('test_entity_id_json_serialization', () => {
    const id = EntityId.fromBytes(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
    const json = JSON.stringify(id);
    expect(json).toEqual('"AQIDBAUGBwgJCgsMDQ4PEA"');
    expect(id).toEqual(EntityId.fromJson(JSON.parse(json)).unwrap());
  });

  test('test_entity_id_bincode_serialization', () => {
    const id = EntityId.fromBytes(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
    const bytes = (() => { const _w = new BincodeWriter(); id.encode(_w); return _w.finish(); })();
    expect(bytes).toEqual(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
    expect(id).toEqual((() => { const _r = new BincodeReader(bytes); return EntityId.decode(_r); })());
  });

});
