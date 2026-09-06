// MIRRORS: ankurah/core/src/indexing/encoding.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { encodeComponentTyped } from './encoding';
import { Value, ValueType } from '../value/index';

describe('encoding unit tests', () => {
  test('test_desc_ordering', () => {
    const _t0 = new Value('String', { _0: 'a' });
    try {
      const a = encodeComponentTyped(_t0, new ValueType('String', {}), true).unwrap();
      const _t1 = new Value('String', { _0: 'b' });
      try {
        const b = encodeComponentTyped(_t1, new ValueType('String', {}), true).unwrap();
        if (!(a > b)) throw new Error('assertion failed');
      } finally {
        _t1.drop();
      }
    } finally {
      _t0.drop();
    }
  });

  test('test_asc_ordering', () => {
    const _t0 = new Value('String', { _0: 'a' });
    try {
      const a = encodeComponentTyped(_t0, new ValueType('String', {}), false).unwrap();
      const _t1 = new Value('String', { _0: 'b' });
      try {
        const b = encodeComponentTyped(_t1, new ValueType('String', {}), false).unwrap();
        if (!(a < b)) throw new Error('assertion failed');
      } finally {
        _t1.drop();
      }
    } finally {
      _t0.drop();
    }
  });

});
