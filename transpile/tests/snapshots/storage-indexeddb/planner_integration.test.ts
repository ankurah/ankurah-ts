// MIRRORS: ankurah/storage/indexeddb-wasm/src/planner_integration.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { normalize, planBoundsToIdbRange, planBoundsToIdbRangeSyntax, scanDirectionToCursorDirection } from './planner_integration';
import { dropOwned } from '@ankurah/base';
import { Predicate } from '@ankurah/ankql';
import { IndexKeyPart, KeySpec, Value, ValueType } from '@ankurah/core';
import { Endpoint, KeyBoundComponent, KeyBounds, OrderByComponents, Plan, ScanDirection } from '@ankurah/storage-common';

describe('planner_integration unit tests', () => {
  test('test_plan_index_spec_name', () => {
    const plan = new Plan('Index', { indexSpec: KeySpec.new([IndexKeyPart.asc('__collection', new ValueType('String', {})), IndexKeyPart.asc('age', new ValueType('I32', {})), IndexKeyPart.asc('score', new ValueType('I32', {}))]), scanDirection: new ScanDirection('Forward', {}), bounds: KeyBounds.new([]), remainingPredicate: new Predicate('True', {}), orderBySpill: OrderByComponents.default() });
    {
      const _v = plan;
      if (_v.is('Index')) {
        const { indexSpec } = _v.value;
        try {
          const indexName = indexSpec.nameWith('', '__');
          expect(indexName).toEqual('__collection asc__age asc__score asc');
        } finally {
          indexSpec.drop();
        }
      } else {
      _v.drop();
    }
    }
  });

  test('test_scan_direction_to_cursor_direction', () => {
    const ascDirection = scanDirectionToCursorDirection(new ScanDirection('Forward', {}));
    const descDirection = scanDirectionToCursorDirection(new ScanDirection('Reverse', {}));
    expect(ascDirection).not.toEqual(descDirection);
    expect(ascDirection).toEqual(webSys.IdbCursorDirection.Next);
    expect(descDirection).toEqual(webSys.IdbCursorDirection.Prev);
  });

  test('test_normalize_equality_only', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'album' })), Endpoint.incl(new Value('String', { _0: 'album' }))), new KeyBoundComponent('age', Endpoint.incl(new Value('I32', { _0: 30 })), Endpoint.incl(new Value('I32', { _0: 30 })))]);
    try {
      const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
      expect(eqPrefixLen).toEqual(2);
      const _t0 = [new Value('String', { _0: 'album' }), new Value('I32', { _0: 30 })];
      try {
        expect(eqPrefixValues).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
      expect(canonicalRange.lower).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 30 })], false]);
      expect(canonicalRange.upper).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 31 })], true]);
    } finally {
      bounds.drop();
    }
  });

  test('test_normalize_with_inequality', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'album' })), Endpoint.incl(new Value('String', { _0: 'album' }))), new KeyBoundComponent('age', Endpoint.excl(new Value('I32', { _0: 25 })), new Endpoint('UnboundedHigh', { _0: new ValueType('I32', {}) }))]);
    try {
      const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
      expect(eqPrefixLen).toEqual(1);
      const _t0 = [new Value('String', { _0: 'album' })];
      try {
        expect(eqPrefixValues).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
      expect(canonicalRange.lower).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 25 })], true]);
      expect(canonicalRange.upper).toEqual(null);
    } finally {
      bounds.drop();
    }
  });

  test('test_plan_bounds_to_idb_range', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'album' })), Endpoint.incl(new Value('String', { _0: 'album' })))]);
    try {
      const result = planBoundsToIdbRange(bounds, new ScanDirection('Forward', {}));
      if (!(result.isOk())) throw new Error('assertion failed');
      const [_idbRange, upperOpenEnded, eqPrefixLen, eqPrefixValues] = result.unwrap();
      if (!(!upperOpenEnded)) throw new Error('assertion failed');
      expect(eqPrefixLen).toEqual(1);
      const _t0 = [new Value('String', { _0: 'album' })];
      try {
        expect(eqPrefixValues).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
    } finally {
      bounds.drop();
    }
  });

  test('test_plan_bounds_to_idb_range_syntax', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'connectionevent' })), Endpoint.incl(new Value('String', { _0: 'connectionevent' }))), new KeyBoundComponent('user_id', Endpoint.incl(new Value('String', { _0: 'AZoegTHj_4vcBoJ5FfY-Xw' })), Endpoint.incl(new Value('String', { _0: 'AZoegTHj_4vcBoJ5FfY-Xw' }))), new KeyBoundComponent('timestamp', Endpoint.excl(new Value('I64', { _0: 1761455267792n })), Endpoint.excl(new Value('I64', { _0: 1761456167793n })))]);
    try {
      const result = planBoundsToIdbRangeSyntax(bounds);
      if (!(result.isOk())) throw new Error('assertion failed');
      const jsSyntax = result.unwrap();
      console.log(`Generated JavaScript syntax: ${jsSyntax}`);
      if (!(jsSyntax.includes('IDBKeyRange.bound'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('"connectionevent"'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('"AZoegTHj_4vcBoJ5FfY-Xw"'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('1761455267792'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('1761456167793'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('true, true'))) throw new Error('assertion failed');
    } finally {
      bounds.drop();
    }
  });

  test('test_plan_bounds_to_idb_range_syntax_equality_only', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'album' })), Endpoint.incl(new Value('String', { _0: 'album' })))]);
    try {
      const result = planBoundsToIdbRangeSyntax(bounds);
      if (!(result.isOk())) throw new Error('assertion failed');
      const jsSyntax = result.unwrap();
      console.log(`Generated JavaScript syntax for single equality: ${jsSyntax}`);
      if (!(jsSyntax.includes('IDBKeyRange.bound'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('"album"'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('], ["album') && jsSyntax.endsWith('"], false, true)'))) throw new Error('assertion failed');
    } finally {
      bounds.drop();
    }
  });

  test('test_plan_bounds_to_idb_range_syntax_multi_equality', () => {
    const bounds = KeyBounds.new([new KeyBoundComponent('__collection', Endpoint.incl(new Value('String', { _0: 'album' })), Endpoint.incl(new Value('String', { _0: 'album' }))), new KeyBoundComponent('year', Endpoint.incl(new Value('String', { _0: '2000' })), Endpoint.incl(new Value('String', { _0: '2000' })))]);
    try {
      const result = planBoundsToIdbRangeSyntax(bounds);
      if (!(result.isOk())) throw new Error('assertion failed');
      const jsSyntax = result.unwrap();
      console.log(`Generated JavaScript syntax for multi-equality: ${jsSyntax}`);
      if (!(jsSyntax.includes('IDBKeyRange.bound'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('"album"'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('"2000"'))) throw new Error('assertion failed');
      if (!(jsSyntax.includes('], ["album", "2000') && jsSyntax.endsWith('"], false, true)'))) throw new Error('assertion failed');
    } finally {
      bounds.drop();
    }
  });

});
