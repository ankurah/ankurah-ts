// MIRRORS: ankurah/proto/src/data.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { EventId } from './data';
import { BincodeWriter, BincodeReader } from './codec';

describe('data unit tests', () => {
  test('test_event_id_json_serialization', () => {
    const id = EventId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    try {
      const json = JSON.stringify(id);
      expect(json).toEqual('"AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA"');
      expect(id).toEqual(JSON.parse(json));
    } finally {
      id.drop();
    }
  });

  test('test_event_id_bincode_serialization', () => {
    const id = EventId.fromBytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    try {
      const bytes = (() => { const _w = new BincodeWriter(); id.encode(_w); return _w.finish(); })();
      expect(bytes).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
      expect(id).toEqual((() => { const _r = new BincodeReader(bytes); return /* TODO: need type */ _r; })());
    } finally {
      id.drop();
    }
  });

});
