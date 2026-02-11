// MIRRORS: ankurah/storage/common/src/bounds.rs

import type { Value } from '@ankurah/core';
import type { CanonicalRange, KeyBounds } from './types.ts';

/**
 * Normalize IndexBounds to a CanonicalRange shape shared across KV engines.
 *
 * Rust: `pub fn normalize(bounds: &KeyBounds) -> (CanonicalRange, usize, Vec<Value>)`
 */
export function normalize(bounds: KeyBounds): [CanonicalRange, number, Value[]] {
  const lowerTuple: Value[] = [];
  const upperTuple: Value[] = [];
  let lowerOpen = false;
  let upperOpen = false;
  let eqPrefixLen = 0;
  const eqPrefixValues: Value[] = [];

  for (const bound of bounds.keyparts) {
    // Check for equality: both endpoints are inclusive Value with same Val datum
    if (
      bound.low.type === 'Value' && bound.high.type === 'Value' &&
      bound.low.datum.type === 'Val' && bound.high.datum.type === 'Val' &&
      bound.low.inclusive && bound.high.inclusive &&
      valuesEqual(bound.low.datum.value, bound.high.datum.value)
    ) {
      lowerTuple.push(bound.low.datum.value);
      upperTuple.push(bound.high.datum.value);
      eqPrefixValues.push(bound.low.datum.value);
      eqPrefixLen += 1;
      continue;
    }

    // Process low endpoint
    if (bound.low.type === 'Value' && bound.low.datum.type === 'Val') {
      lowerTuple.push(bound.low.datum.value);
      lowerOpen = !bound.low.inclusive;
    } else if (bound.low.type === 'UnboundedLow') {
      // No-op for unbounded low
    } else {
      break;
    }

    // Process high endpoint
    if (bound.high.type === 'Value' && bound.high.datum.type === 'Val') {
      upperTuple.push(bound.high.datum.value);
      upperOpen = !bound.high.inclusive;
    } else if (bound.high.type === 'UnboundedHigh') {
      return [
        { lower: lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, upper: null },
        eqPrefixLen,
        eqPrefixValues,
      ];
    } else {
      return [
        { lower: lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, upper: null },
        eqPrefixLen,
        eqPrefixValues,
      ];
    }

    break;
  }

  if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen === 1) {
    return [
      { lower: lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, upper: null },
      eqPrefixLen,
      eqPrefixValues,
    ];
  }

  const canonicalRange: CanonicalRange = {
    lower: lowerTuple.length === 0 ? null : [lowerTuple, lowerOpen],
    upper: upperTuple.length === 0 ? null : [upperTuple, upperOpen],
  };

  return [canonicalRange, eqPrefixLen, eqPrefixValues];
}

/**
 * Simple deep-equality check for two Value objects.
 * Mirrors Rust `low_val == high_val` comparison in bounds.rs.
 */
function valuesEqual(a: Value, b: Value): boolean {
  if (a.type !== b.type) return false;
  switch (a.type) {
    case 'I16':
    case 'I32':
    case 'I64':
    case 'F64':
    case 'Bool':
    case 'String':
      return a.value === (b as typeof a).value;
    default:
      return JSON.stringify(a.value) === JSON.stringify(b.value);
  }
}
