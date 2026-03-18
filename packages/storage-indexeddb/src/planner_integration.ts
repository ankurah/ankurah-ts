// MIRRORS: ankurah/storage/indexeddb-wasm/src/planner_integration.rs

import type { Value } from '@ankurah/core';
import { CanonicalRange, type KeyBounds, ScanDirection } from '@ankurah/storage-common';
import { valueToIdb } from './idb_value.ts';

// ── next_upper_bound ────────────────────────────────────────────────────

function nextUpperBound(value: Value): [Value, boolean] | null {
  switch (value.type) {
    case 'Bool':
      return value.value === false
        ? [{ type: 'Bool', value: true }, true]
        : [{ type: 'I32', value: 2 }, true];
    case 'I16':
      // Divergence: Rust uses saturating_add. JS numbers don't overflow the same way. [E8]
      return [{ type: 'I16', value: value.value + 1 }, true];
    case 'I32':
      return [{ type: 'I32', value: value.value + 1 }, true];
    case 'I64':
      return [{ type: 'I64', value: value.value + 1 }, true];
    case 'F64': {
      const v = value.value;
      if (isNaN(v) || !isFinite(v)) return null;
      // Use nextafter-like logic: add a small epsilon scaled to the magnitude
      const epsilon = Math.max(Number.EPSILON, Math.abs(v) * Number.EPSILON);
      return [{ type: 'F64', value: v + epsilon }, true];
    }
    case 'String': {
      return [{ type: 'String', value: value.value + '\u{0000}' }, true];
    }
    case 'EntityId': {
      const bumped = value.value.toBase64() + '\u{0000}';
      return [{ type: 'String', value: bumped }, true];
    }
    case 'Object':
    case 'Binary':
    case 'Json':
      return null;
  }
}

// ── normalize ─────────────────────────────────────────────────────────

/**
 * Normalize KeyBounds to CanonicalRange following the playbook algorithm.
 * Returns [CanonicalRange, eq_prefix_len, eq_prefix_values].
 *
 * This is the IndexedDB-specific normalize that includes next_upper_bound
 * for equality-only cases, distinct from the shared normalize in storage-common.
 *
 * Mirrors Rust `normalize(bounds: &KeyBounds) -> (CanonicalRange, usize, Vec<Value>)`
 * in planner_integration.rs (NOT bounds.rs).
 */
export function normalize(bounds: KeyBounds): [CanonicalRange, number, Value[]] {
  const lowerTuple: Value[] = [];
  const upperTuple: Value[] = [];
  let lowerOpen = false;
  let upperOpen = false;
  let eqPrefixLen = 0;
  const eqPrefixValues: Value[] = [];

  // Step 1: Accumulate equality prefix
  for (const bound of bounds.keyparts) {
    // Check if this is an equality constraint (low == high and both inclusive)
    if (
      bound.low.is('Value') && bound.high.is('Value') &&
      bound.low.value.datum.is('Val') && bound.high.value.datum.is('Val') &&
      bound.low.value.inclusive && bound.high.value.inclusive &&
      valuesEqual(bound.low.value.datum.value.value, bound.high.value.datum.value.value)
    ) {
      lowerTuple.push(bound.low.value.datum.value.value);
      upperTuple.push(bound.high.value.datum.value.value);
      eqPrefixLen += 1;
      eqPrefixValues.push(bound.low.value.datum.value.value);
      continue;
    }

    // Step 2: At first non-equality column, process lower and upper sides

    // Process lower side
    if (bound.low.is('UnboundedLow')) {
      // Keep lower at equality prefix; do not break. We'll process upper next.
    } else if (bound.low.is('Value') && bound.low.value.datum.is('Val')) {
      lowerTuple.push(bound.low.value.datum.value.value);
      lowerOpen = !bound.low.value.inclusive;
    } else {
      break; // Other cases like infinity values
    }

    // Process upper side
    if (bound.high.is('UnboundedHigh')) {
      // upper = None (open-ended)
      return [
        new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
        eqPrefixLen,
        eqPrefixValues,
      ];
    } else if (bound.high.is('Value') && bound.high.value.datum.is('Val')) {
      upperTuple.push(bound.high.value.datum.value.value);
      upperOpen = !bound.high.value.inclusive;
    } else {
      // Other cases - treat as open-ended
      return [
        new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
        eqPrefixLen,
        eqPrefixValues,
      ];
    }

    // Stop after processing first inequality column
    break;
  }

  // Equality-only bounds: if we have equality on all leading columns
  // and no inequality processed, prefer a prefix-open scan to cover full suffix.
  if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen > 0) {
    const lastValue = lowerTuple[lowerTuple.length - 1];
    if (lastValue !== undefined) {
      const bump = nextUpperBound(lastValue);
      if (bump !== null) {
        const [nextValue, bumpUpperOpen] = bump;
        const upperWithBump = [...lowerTuple];
        upperWithBump[upperWithBump.length - 1] = nextValue;
        return [
          new CanonicalRange([lowerTuple, lowerOpen], [upperWithBump, bumpUpperOpen]),
          eqPrefixLen,
          eqPrefixValues,
        ];
      }
    }
    return [
      new CanonicalRange(lowerTuple.length > 0 ? [lowerTuple, lowerOpen] : null, null),
      eqPrefixLen,
      eqPrefixValues,
    ];
  }

  // Build final CanonicalRange for inequality cases
  const canonicalRange = new CanonicalRange(
    lowerTuple.length === 0 ? null : [lowerTuple, lowerOpen],
    upperTuple.length === 0 ? null : [upperTuple, upperOpen],
  );

  return [canonicalRange, eqPrefixLen, eqPrefixValues];
}

/**
 * Simple deep-equality check for two Value objects.
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

// ── idb_key_tuple ───────────────────────────────────────────────────────

/**
 * Build IDB key array from Value array.
 * Uses canonical IdbValue encoding for symmetric handling across storage, ranges, and guards.
 *
 * Mirrors Rust `idb_key_tuple(parts: &[Value]) -> Result<JsValue>`.
 */
function idbKeyTuple(parts: Value[]): unknown[] {
  return parts.map(p => valueToIdb(p));
}

// ── to_idb_keyrange ─────────────────────────────────────────────────────

/**
 * Convert CanonicalRange to IDBKeyRange.
 * Returns [IDBKeyRange, upper_open_ended_flag].
 *
 * Mirrors Rust `to_idb_keyrange(canonical_range: &CanonicalRange) -> Result<(IdbKeyRange, bool)>`.
 */
export function toIdbKeyrange(canonicalRange: CanonicalRange): [IDBKeyRange, boolean] {
  const lower = canonicalRange.lower;
  const upper = canonicalRange.upper;

  if (lower !== null && upper === null) {
    // Upper = None (open-ended)
    const lowerJs = idbKeyTuple(lower[0]);
    const range = IDBKeyRange.lowerBound(lowerJs, lower[1]);
    return [range, true]; // upper_open_ended = true
  }

  if (lower !== null && upper !== null) {
    // Finite lower & upper
    const lowerJs = idbKeyTuple(lower[0]);
    const upperJs = idbKeyTuple(upper[0]);
    const range = IDBKeyRange.bound(lowerJs, upperJs, lower[1], upper[1]);
    return [range, false]; // upper_open_ended = false
  }

  if (lower === null && upper !== null) {
    // Only upper bound
    const upperJs = idbKeyTuple(upper[0]);
    const range = IDBKeyRange.upperBound(upperJs, upper[1]);
    return [range, false]; // upper_open_ended = false
  }

  // Completely unbounded
  throw new Error('Cannot create IDBKeyRange for completely unbounded range');
}

// ── plan_bounds_to_idb_range ────────────────────────────────────────────

/**
 * Convert Plan bounds to IndexedDB IDBKeyRange using the IR pipeline.
 * Returns [IDBKeyRange, upper_open_ended_flag, eq_prefix_len, eq_prefix_values].
 *
 * The `scanDirection` parameter is critical for handling DESC ordering correctly:
 * - For Reverse (DESC) scans with open-ended lower bounds (e.g., timestamp >= X),
 *   we must cap the upper bound at the equality prefix boundary to prevent the
 *   cursor from starting outside the intended key range.
 *
 * Mirrors Rust `plan_bounds_to_idb_range(bounds, scan_direction)`.
 */
export function planBoundsToIdbRange(
  bounds: KeyBounds,
  scanDirection: ScanDirection,
): [IDBKeyRange, boolean, number, Value[]] {
  // Step 1: Normalize IR to CanonicalRange
  const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);

  // Step 2: For Reverse scans with open-ended upper bound and equality prefix,
  // we need to cap the range to stay within the equality prefix.
  let adjustedRange = canonicalRange;
  if (
    scanDirection.is('Reverse') &&
    canonicalRange.upper === null &&
    eqPrefixLen > 0 &&
    canonicalRange.lower !== null
  ) {
    const lastEqValue = eqPrefixValues[eqPrefixValues.length - 1];
    if (lastEqValue !== undefined) {
      const bump = nextUpperBound(lastEqValue);
      if (bump !== null) {
        const [nextValue, isOpen] = bump;
        const upperTuple = eqPrefixValues.slice(0, eqPrefixLen);
        upperTuple[upperTuple.length - 1] = nextValue;
        adjustedRange = {
          lower: canonicalRange.lower,
          upper: [upperTuple, isOpen],
        } as CanonicalRange;
      }
    }
  }

  // Step 3: Convert CanonicalRange to IDBKeyRange
  const [idbRange, upperOpenEnded] = toIdbKeyrange(adjustedRange);

  return [idbRange, upperOpenEnded, eqPrefixLen, eqPrefixValues];
}

// ── scan_direction_to_cursor_direction ───────────────────────────────────

/**
 * Convert scan direction to IndexedDB cursor direction.
 *
 * Mirrors Rust `scan_direction_to_cursor_direction(scan_direction)`.
 */
export function scanDirectionToCursorDirection(scanDirection: ScanDirection): IDBCursorDirection {
  return scanDirection.match({
    Forward: () => 'next' as IDBCursorDirection,
    Reverse: () => 'prev' as IDBCursorDirection,
  });
}

// ── values_to_js_array (debug/syntax helper) ─────────────────────────────

/**
 * Convert Value array to JavaScript array syntax string.
 * Used for debugging and testing.
 *
 * Mirrors Rust `values_to_js_array(values: &[Value]) -> Result<String>`.
 */
export function valuesToJsArray(values: Value[]): string {
  const parts: string[] = [];
  for (const value of values) {
    switch (value.type) {
      case 'String':
        parts.push(`"${value.value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`);
        break;
      case 'I64':
        parts.push(String(value.value));
        break;
      case 'I32':
        parts.push(String(value.value));
        break;
      case 'I16':
        parts.push(String(value.value));
        break;
      case 'F64':
        if (value.value === Math.floor(value.value)) {
          parts.push(String(Math.floor(value.value)));
        } else {
          parts.push(String(value.value));
        }
        break;
      case 'Bool':
        parts.push(value.value ? '1' : '0');
        break;
      case 'EntityId':
        parts.push(`"${value.value.toBase64()}"`);
        break;
      case 'Object':
      case 'Binary':
      case 'Json':
        throw new Error(`Object, Binary and Json values not supported in key syntax generation: ${value.type}`);
    }
  }
  return `[${parts.join(', ')}]`;
}

/**
 * Convert Plan bounds to IDBKeyRange syntax string (for debugging).
 *
 * Mirrors Rust `plan_bounds_to_idb_range_syntax(bounds)`.
 */
export function planBoundsToIdbRangeSyntax(bounds: KeyBounds): string {
  const [canonicalRange, _eqPrefixLen, _eqPrefixValues] = normalize(bounds);

  const lower = canonicalRange.lower;
  const upper = canonicalRange.upper;

  if (lower !== null && upper !== null) {
    return `IDBKeyRange.bound(${valuesToJsArray(lower[0])}, ${valuesToJsArray(upper[0])}, ${lower[1]}, ${upper[1]})`;
  }
  if (lower !== null && upper === null) {
    return `IDBKeyRange.lowerBound(${valuesToJsArray(lower[0])}, ${lower[1]})`;
  }
  if (lower === null && upper !== null) {
    return `IDBKeyRange.upperBound(${valuesToJsArray(upper[0])}, ${upper[1]})`;
  }
  throw new Error('Cannot generate syntax for completely unbounded range');
}
