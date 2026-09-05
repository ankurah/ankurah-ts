// MIRRORS: ankurah/storage/common/src/bounds.rs
import { dropOwned, checkedAdd } from '@ankurah/base';
import { Value } from '@ankurah/core';
import { CanonicalRange, KeyBounds } from './types';

export function normalize(bounds: KeyBounds): [CanonicalRange, number, Value[]] {
  let lowerTuple = [];
  let upperTuple = [];
  let lowerOpen = false;
  let upperOpen = false;
  let eqPrefixLen = 0;
  let eqPrefixValues = [];
  const _seq1 = bounds.keyparts;
  let _at2 = 0;
  try {
    while (_at2 < _seq1.length) {
      const bound = _seq1[_at2++];
      try {
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
                      eqPrefixLen = checkedAdd(eqPrefixLen, 1, 'i32');
                      continue;
                    }}}  }
            }  }
        }
        if (bound.low.is('Value') && (bound.low.value.datum.is('Val'))) {
          const { inclusive } = bound.low.value;
          const { _0: val } = bound.low.value.datum.value;
          (() => {
            lowerTuple.push(val.clone());
            lowerOpen = !inclusive;
          })()
        } else if (bound.low.is('UnboundedLow')) {
          (() => {
          })()
        } else {
          break
        }
        const _m0 = bound.high.match<any>({
          Value: (v) => {
            const { _0: val } = v.datum.value;
            const inclusive = v.inclusive;
            upperTuple.push(val.clone());
            upperOpen = !inclusive;
          },
          UnboundedHigh: (v) => {
            return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
          },
          UnboundedLow: () => {
            return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
          },
        });
        if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
        break;
      } finally {
        bound.drop();
      }
    }
  } finally {
    dropOwned(_seq1.slice(_at2));
  }
  if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen === 1) {
    return [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues];
  }
  let _moved3 = false;
  const canonicalRange = new CanonicalRange(lowerTuple.length === 0 ? null : [lowerTuple, lowerOpen], upperTuple.length === 0 ? null : [upperTuple, upperOpen]);
  try {
    _moved3 = true;
    return [canonicalRange, eqPrefixLen, eqPrefixValues];
  } finally {
    if (!_moved3) canonicalRange.drop();
  }
}

