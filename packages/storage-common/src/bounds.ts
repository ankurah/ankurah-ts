// MIRRORS: ankurah/storage/common/src/bounds.rs

import type { Value } from '@ankurah/core';
import { CanonicalRange, KeyBounds } from './types.ts';

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
    // Rust: if let (Endpoint::Value { datum: low_datum, inclusive: low_incl },
    //              Endpoint::Value { datum: high_datum, inclusive: high_incl }) = (&bound.low, &bound.high)
    //       && let (KeyDatum::Val(low_val), KeyDatum::Val(high_val)) = (low_datum, high_datum)
    //       && low_val == high_val && *low_incl && *high_incl
    if (
      bound.low.is('Value') && bound.high.is('Value') &&
      bound.low.value.datum.is('Val') && bound.high.value.datum.is('Val') &&
      bound.low.value.inclusive && bound.high.value.inclusive &&
      valuesEqual(bound.low.value.datum.value.value, bound.high.value.datum.value.value)
    ) {
      lowerTuple.push(bound.low.value.datum.value.value);
      upperTuple.push(bound.high.value.datum.value.value);
      eqPrefixValues.push(bound.low.value.datum.value.value);
      eqPrefixLen += 1;
      continue;
    }

    // Process low endpoint
    // Rust: match &bound.low { Endpoint::Value { datum: KeyDatum::Val(val), inclusive } => ...
    let lowHandled = false;
    if (bound.low.is('Value') && bound.low.value.datum.is('Val')) {
      lowerTuple.push(bound.low.value.datum.value.value);
      lowerOpen = !bound.low.value.inclusive;
      lowHandled = true;
    } else if (bound.low.is('UnboundedLow')) {
      // No-op for unbounded low
      lowHandled = true;
    }
    if (!lowHandled) {
      break;
    }

    // Process high endpoint
    // Rust: match &bound.high { Endpoint::Value { datum: KeyDatum::Val(val), inclusive } => ...
    if (bound.high.is('Value') && bound.high.value.datum.is('Val')) {
      upperTuple.push(bound.high.value.datum.value.value);
      upperOpen = !bound.high.value.inclusive;
    } else if (bound.high.is('UnboundedHigh')) {
      return [
        new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
        eqPrefixLen,
        eqPrefixValues,
      ];
    } else {
      return [
        new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
        eqPrefixLen,
        eqPrefixValues,
      ];
    }

    break;
  }

  if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen === 1) {
    return [
      new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
      eqPrefixLen,
      eqPrefixValues,
    ];
  }

  const canonicalRange = new CanonicalRange(
    lowerTuple.length === 0 ? null : [lowerTuple, lowerOpen],
    upperTuple.length === 0 ? null : [upperTuple, upperOpen],
  );

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
