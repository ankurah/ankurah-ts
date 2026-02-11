// MIRRORS: ankurah/storage/common/src/types.rs

import type { Predicate, OrderByItem } from '@ankurah/ankql';
import type { Value, KeySpec } from '@ankurah/core';
import { ValueType, valueType } from '@ankurah/core';

// --- ORDER BY Components (partition-aware sorting) ----------------------------------------

/**
 * Describes how ORDER BY should be handled by the execution engine.
 *
 * When an index can only partially satisfy ORDER BY (e.g., mixed directions on IndexedDB),
 * results arrive pre-sorted by `presort` columns but need in-memory sorting by `spill` columns
 * within each partition (group of rows with identical `presort` values).
 *
 * See specs/pushdown/order_by.md for detailed documentation.
 *
 * Rust: `pub struct OrderByComponents { presort, spill }`
 */
export interface OrderByComponents {
  /**
   * ORDER BY columns satisfied by the index scan direction.
   * These define "partition boundaries" - when these values change,
   * we're in a new partition that needs independent sorting.
   * Empty if the entire ORDER BY must be spilled (global sort).
   */
  presort: OrderByItem[];

  /**
   * ORDER BY columns requiring in-memory sort.
   * Empty if the index fully satisfies the ORDER BY.
   */
  spill: OrderByItem[];
}

/** Create a new OrderByComponents. */
export function orderByComponentsNew(presort: OrderByItem[], spill: OrderByItem[]): OrderByComponents {
  return { presort, spill };
}

/** Default (empty) OrderByComponents. */
export function orderByComponentsDefault(): OrderByComponents {
  return { presort: [], spill: [] };
}

/** Returns true if no sorting is needed (index satisfies entire ORDER BY). */
export function orderByComponentsIsSatisfied(obc: OrderByComponents): boolean {
  return obc.spill.length === 0;
}

/** Returns true if the entire ORDER BY must be spilled (global sort). */
export function orderByComponentsIsGlobalSpill(obc: OrderByComponents): boolean {
  return obc.presort.length === 0 && obc.spill.length > 0;
}

// --- Plan (similar to PG IndexScan/IndexOnlyScan inputs) --------------------------------

export type Plan =
  | {
      type: 'Index';
      indexSpec: KeySpec;
      scanDirection: ScanDirection;
      bounds: KeyBounds;
      remainingPredicate: Predicate;
      orderBySpill: OrderByComponents;
    }
  | {
      type: 'TableScan';
      bounds: KeyBounds;
      scanDirection: ScanDirection;
      remainingPredicate: Predicate;
      orderBySpill: OrderByComponents;
    }
  | { type: 'EmptyScan' };

export type ScanDirection = 'Forward' | 'Reverse';

// --- Types & sentinels -------------------------------------------------------

/**
 * Planner-only atom for a single column position (PG: like a Datum + flags).
 *
 * Rust: `pub enum KeyDatum { Val(Value), NegInfinity(ValueType), PosInfinity(ValueType) }`
 */
export type KeyDatum =
  | { type: 'Val'; value: Value }
  | { type: 'NegInfinity'; valueType: ValueType }
  | { type: 'PosInfinity'; valueType: ValueType };

/** Get the ValueType of a KeyDatum. Rust: `pub fn ty(&self)` */
export function keyDatumType(kd: KeyDatum): ValueType {
  switch (kd.type) {
    case 'Val': return valueType(kd.value);
    case 'NegInfinity': return kd.valueType;
    case 'PosInfinity': return kd.valueType;
  }
}

/** Create a KeyDatum from a Value. Rust: `impl From<Value> for KeyDatum` */
export function keyDatumFromValue(v: Value): KeyDatum {
  return { type: 'Val', value: v };
}

// --- Endpoints & per-column bounds (PG: per-column ScanKey / bound) ----------

/**
 * Endpoint for one side of a column bound (PG: strategy + flags collapsed).
 *
 * Rust: `pub enum Endpoint { UnboundedLow(ValueType), UnboundedHigh(ValueType), Value { datum, inclusive } }`
 */
export type Endpoint =
  | { type: 'UnboundedLow'; valueType: ValueType }
  | { type: 'UnboundedHigh'; valueType: ValueType }
  | { type: 'Value'; datum: KeyDatum; inclusive: boolean };

/** Create an inclusive Value endpoint. Rust: `Endpoint::incl(v)` */
export function endpointIncl(v: Value): Endpoint {
  return { type: 'Value', datum: { type: 'Val', value: v }, inclusive: true };
}

/** Create an exclusive Value endpoint. Rust: `Endpoint::excl(v)` */
export function endpointExcl(v: Value): Endpoint {
  return { type: 'Value', datum: { type: 'Val', value: v }, inclusive: false };
}

// --- Per-column bound --------------------------------------------------------

/**
 * Bound for a single index column, in index key order (PG: per keypart).
 *
 * Rust: `pub struct KeyBoundComponent { column, low, high }`
 */
export interface KeyBoundComponent {
  column: string;
  low: Endpoint;
  high: Endpoint;
}

// --- Multi-column bounds (PG: IndexBounds) -----------------------------------

/**
 * Full multi-column bounds for an index scan (PG: IndexBounds).
 *
 * Rust: `pub struct KeyBounds { keyparts: Vec<KeyBoundComponent> }`
 */
export interface KeyBounds {
  keyparts: KeyBoundComponent[];
}

/** Create new KeyBounds. Rust: `KeyBounds::new(keyparts)` */
export function keyBoundsNew(keyparts: KeyBoundComponent[]): KeyBounds {
  return { keyparts };
}

/** Create empty KeyBounds. Rust: `KeyBounds::empty()` */
export function keyBoundsEmpty(): KeyBounds {
  return { keyparts: [] };
}

// --- Canonical, lexicographic interval after normalization -------------------

/**
 * Canonical lexicographic interval (possibly open-ended) ready for lowering.
 * lower/upper: [tuple, open?] where open===true means exclusive.
 *
 * Rust: `pub struct CanonicalRange { lower, upper }`
 */
export interface CanonicalRange {
  /** None (null) => unbounded low */
  lower: [Value[], boolean] | null;
  /** None (null) => unbounded high */
  upper: [Value[], boolean] | null;
}
