// MIRRORS: ankurah/storage/common/src/bounds.rs
import { dropOwned, checkedAdd } from '@ankurah/base';
import { Value } from '@ankurah/core';
import { CanonicalRange, KeyBounds } from './types';

export function normalize(bounds: KeyBounds): [CanonicalRange, number, Value[]] {
  let _moved0 = false;
  let lowerTuple = [];
  try {
    let _moved1 = false;
    let upperTuple = [];
    try {
      let lowerOpen = false;
      let upperOpen = false;
      let eqPrefixLen = 0;
      let _moved2 = false;
      let eqPrefixValues = [];
      try {
        for (const bound of bounds.keyparts) {
          {
            const _v = [bound.low, bound.high];
            if ((_v[0].is('Value')) && (_v[1].is('Value'))) {
              const { datum: lowDatum, inclusive: lowIncl } = _v[0].value;
              const { datum: highDatum, inclusive: highIncl } = _v[1].value;
              {
                const _v1 = [lowDatum, highDatum];
                if ((_v1[0].is('Val')) && (_v1[1].is('Val'))) {
                  const { _0: lowVal } = _v1[0].value;
                  const { _0: highVal } = _v1[1].value;
                  if (lowVal.equals(highVal)) {
                    if (lowIncl) {
                      if (highIncl) {
                        lowerTuple.push(lowVal.clone());
                        upperTuple.push(highVal.clone());
                        eqPrefixValues.push(lowVal.clone());
                        eqPrefixLen = checkedAdd(eqPrefixLen, 1, 'usize');
                        continue;
                      }}}  }
              }  }
          }
          if (bound.low.is('Value') && (bound.low.value.datum.is('Val'))) {
            const { inclusive } = bound.low.value;
            const { _0: val } = bound.low.value.datum.value;
            lowerTuple.push(val.clone());
            lowerOpen = !inclusive;
          } else if (bound.low.is('UnboundedLow')) {

          } else {
            break
          }
          const _m3 = bound.high.match<any>({
            Value: (v) => {
              if (v.datum.is('Val')) {
                const { _0: val } = v.datum.value;
                const inclusive = v.inclusive;
                upperTuple.push(val.clone());
                upperOpen = !inclusive;
              } else {
                _moved2 = true;
                _moved0 = true;
                return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
              }
            },
            UnboundedHigh: (v) => {
              _moved2 = true;
              _moved0 = true;
              return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
            },
            UnboundedLow: () => {
              _moved2 = true;
              _moved0 = true;
              return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
            },
          });
          if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
          break;
        }
        if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen === 1) {
          _moved2 = true;
          _moved0 = true;
          return [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues];
        }
        let _moved4 = false;
        const canonicalRange = new CanonicalRange((() => {
          if (lowerTuple.length === 0) {
            return null;
          } else {
            _moved0 = true;
            return [lowerTuple, lowerOpen];
          }
        })(), (() => {
          if (upperTuple.length === 0) {
            return null;
          } else {
            _moved1 = true;
            return [upperTuple, upperOpen];
          }
        })());
        try {
          _moved4 = true;
          _moved2 = true;
          return [canonicalRange, eqPrefixLen, eqPrefixValues];
        } finally {
          if (!_moved4) canonicalRange.drop();
        }
      } finally {
        if (!_moved2) dropOwned(eqPrefixValues);
      }
    } finally {
      if (!_moved1) dropOwned(upperTuple);
    }
  } finally {
    if (!_moved0) dropOwned(lowerTuple);
  }
}

