// MIRRORS: ankurah/storage/indexeddb-wasm/src/planner_integration.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { normalize, planBoundsToIdbRange, planBoundsToIdbRangeSyntax, scanDirectionToCursorDirection } from './planner_integration';
import { dropOwned, dropUnbound } from '@ankurah/base';
import { Predicate } from '@ankurah/ankql';
import { IndexKeyPart, KeySpec, Value, ValueType } from '@ankurah/core';
import { Endpoint, KeyBoundComponent, KeyBounds, OrderByComponents, Plan, ScanDirection } from '@ankurah/storage-common';

describe('planner_integration unit tests', () => {
  test('test_plan_index_spec_name', () => {
    const plan = new Plan('Index', { indexSpec: KeySpec.new([IndexKeyPart.asc('__collection', new ValueType('String', {})), IndexKeyPart.asc('age', new ValueType('I32', {})), IndexKeyPart.asc('score', new ValueType('I32', {}))]), scanDirection: new ScanDirection('Forward', {}), bounds: KeyBounds.new([]), remainingPredicate: new Predicate('True', {}), orderBySpill: OrderByComponents.default() });
    return plan.intoMatch({
      Index: (v) => {
        const indexSpec = v.indexSpec;
        try {
          try {
            const indexName = indexSpec.nameWith('', '__');
            expect(indexName).toEqual('__collection asc__age asc__score asc');
          } finally {
            indexSpec.drop();
          }
        } finally {
          dropUnbound(v, ['indexSpec']);
        }
      },
      TableScan: (v) => {
        try {
        } finally {
          dropUnbound(v, []);
        }
      },
      EmptyScan: () => {},
    });
  });

  test('test_scan_direction_to_cursor_direction', () => {
    const ascDirection = scanDirectionToCursorDirection(new ScanDirection('Forward', {}));
    const descDirection = scanDirectionToCursorDirection(new ScanDirection('Reverse', {}));
    expect(ascDirection).not.toEqual(descDirection);
    expect(ascDirection).toEqual(webSys.IdbCursorDirection.Next);
    expect(descDirection).toEqual(webSys.IdbCursorDirection.Prev);
  });

  test('test_normalize_equality_only', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'album' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'album' }));
      let _moved4 = false;
      const _b3 = Endpoint.incl(new Value('I32', { _0: 30 }));
      try {
        const _b5 = Endpoint.incl(new Value('I32', { _0: 30 }));
        _moved1 = true;
        _moved4 = true;
        const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2), new KeyBoundComponent('age', _b3, _b5)]);
        try {
          const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
          expect(eqPrefixLen).toEqual(2);
          const _t6 = [new Value('String', { _0: 'album' }), new Value('I32', { _0: 30 })];
          try {
            expect(eqPrefixValues).toEqual(_t6);
          } finally {
            dropOwned(_t6);
          }
          expect(canonicalRange.lower).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 30 })], false]);
          expect(canonicalRange.upper).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 31 })], true]);
        } finally {
          bounds.drop();
        }
      } finally {
        if (!_moved4) dropOwned(_b3);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

  test('test_normalize_with_inequality', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'album' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'album' }));
      let _moved4 = false;
      const _b3 = Endpoint.excl(new Value('I32', { _0: 25 }));
      try {
        const _b5 = new Endpoint('UnboundedHigh', { _0: new ValueType('I32', {}) });
        _moved1 = true;
        _moved4 = true;
        const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2), new KeyBoundComponent('age', _b3, _b5)]);
        try {
          const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
          expect(eqPrefixLen).toEqual(1);
          const _t6 = [new Value('String', { _0: 'album' })];
          try {
            expect(eqPrefixValues).toEqual(_t6);
          } finally {
            dropOwned(_t6);
          }
          expect(canonicalRange.lower).toEqual([[new Value('String', { _0: 'album' }), new Value('I32', { _0: 25 })], true]);
          expect(canonicalRange.upper).toEqual(null);
        } finally {
          bounds.drop();
        }
      } finally {
        if (!_moved4) dropOwned(_b3);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

  test('test_plan_bounds_to_idb_range', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'album' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'album' }));
      _moved1 = true;
      const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2)]);
      try {
        const result = planBoundsToIdbRange(bounds, new ScanDirection('Forward', {}));
        if (!(result.isOk())) throw new Error('assertion failed');
        const [_idbRange, upperOpenEnded, eqPrefixLen, eqPrefixValues] = result.unwrap();
        if (!(!upperOpenEnded)) throw new Error('assertion failed');
        expect(eqPrefixLen).toEqual(1);
        const _t3 = [new Value('String', { _0: 'album' })];
        try {
          expect(eqPrefixValues).toEqual(_t3);
        } finally {
          dropOwned(_t3);
        }
      } finally {
        bounds.drop();
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

  test('test_plan_bounds_to_idb_range_syntax', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'connectionevent' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'connectionevent' }));
      let _moved4 = false;
      const _b3 = Endpoint.incl(new Value('String', { _0: 'AZoegTHj_4vcBoJ5FfY-Xw' }));
      try {
        const _b5 = Endpoint.incl(new Value('String', { _0: 'AZoegTHj_4vcBoJ5FfY-Xw' }));
        let _moved7 = false;
        const _b6 = Endpoint.excl(new Value('I64', { _0: 1761455267792n }));
        try {
          const _b8 = Endpoint.excl(new Value('I64', { _0: 1761456167793n }));
          _moved1 = true;
          _moved4 = true;
          _moved7 = true;
          const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2), new KeyBoundComponent('user_id', _b3, _b5), new KeyBoundComponent('timestamp', _b6, _b8)]);
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
        } finally {
          if (!_moved7) dropOwned(_b6);
        }
      } finally {
        if (!_moved4) dropOwned(_b3);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

  test('test_plan_bounds_to_idb_range_syntax_equality_only', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'album' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'album' }));
      _moved1 = true;
      const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2)]);
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
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

  test('test_plan_bounds_to_idb_range_syntax_multi_equality', () => {
    let _moved1 = false;
    const _b0 = Endpoint.incl(new Value('String', { _0: 'album' }));
    try {
      const _b2 = Endpoint.incl(new Value('String', { _0: 'album' }));
      let _moved4 = false;
      const _b3 = Endpoint.incl(new Value('String', { _0: '2000' }));
      try {
        const _b5 = Endpoint.incl(new Value('String', { _0: '2000' }));
        _moved1 = true;
        _moved4 = true;
        const bounds = KeyBounds.new([new KeyBoundComponent('__collection', _b0, _b2), new KeyBoundComponent('year', _b3, _b5)]);
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
      } finally {
        if (!_moved4) dropOwned(_b3);
      }
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  });

});
