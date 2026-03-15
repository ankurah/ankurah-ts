// MIRRORS: ankurah/storage/common/src/types.rs

import { OrderByItem, Predicate } from '@ankurah/ankql';
import { Struct, Enum } from '@ankurah/base';
import type { Value, KeySpec } from '@ankurah/core';
import { ValueType, valueType } from '@ankurah/core';

// --- ORDER BY Components (partition-aware sorting) ----------------------------------------

/**
 * Describes how ORDER BY should be handled by the execution engine.
 *
 * When an index can only partially satisfy ORDER BY (e.g., mixed directions on IndexedDB),
 * results arrive pre-sorted by `presort` columns but need in-memory sorting by `spill` columns
 * within each partition (group of rows with identical `presort` values).
 */
export class OrderByComponents extends Struct {
  /** ORDER BY columns satisfied by the index scan direction. */
  presort: OrderByItem[];
  /** ORDER BY columns requiring in-memory sort. */
  spill: OrderByItem[];

  constructor(presort: OrderByItem[], spill: OrderByItem[]) {
    super();
    this.presort = presort;
    this.spill = spill;
  }

  /** Default (empty) OrderByComponents. */
  static default(): OrderByComponents {
    return new OrderByComponents([], []);
  }

  /** Returns true if no sorting is needed (index satisfies entire ORDER BY). */
  isSatisfied(): boolean {
    return this.spill.length === 0;
  }

  /** Returns true if the entire ORDER BY must be spilled (global sort). */
  isGlobalSpill(): boolean {
    return this.presort.length === 0 && this.spill.length > 0;
  }
}

// --- Plan (similar to PG IndexScan/IndexOnlyScan inputs) --------------------------------

type PlanV = {
  Index: {
    indexSpec: KeySpec;
    scanDirection: ScanDirection;
    bounds: KeyBounds;
    remainingPredicate: Predicate;
    orderBySpill: OrderByComponents;
  };
  TableScan: {
    bounds: KeyBounds;
    scanDirection: ScanDirection;
    remainingPredicate: Predicate;
    orderBySpill: OrderByComponents;
  };
  EmptyScan: {};
};

export class Plan extends Enum<PlanV> {
  static Index(indexSpec: KeySpec, scanDirection: ScanDirection, bounds: KeyBounds, remainingPredicate: Predicate, orderBySpill: OrderByComponents): Plan {
    return new Plan('Index', { indexSpec, scanDirection, bounds, remainingPredicate, orderBySpill });
  }
  static TableScan(bounds: KeyBounds, scanDirection: ScanDirection, remainingPredicate: Predicate, orderBySpill: OrderByComponents): Plan {
    return new Plan('TableScan', { bounds, scanDirection, remainingPredicate, orderBySpill });
  }
  static EmptyScan(): Plan {
    return new Plan('EmptyScan', {});
  }
}

// --- ScanDirection ---

type ScanDirectionV = {
  Forward: {};
  Reverse: {};
};

export class ScanDirection extends Enum<ScanDirectionV> {
  static Forward(): ScanDirection { return new ScanDirection('Forward', {}); }
  static Reverse(): ScanDirection { return new ScanDirection('Reverse', {}); }
}

// --- Types & sentinels -------------------------------------------------------

/** Planner-only atom for a single column position (PG: like a Datum + flags). */
type KeyDatumV = {
  Val: { value: Value };
  NegInfinity: { valueType: ValueType };
  PosInfinity: { valueType: ValueType };
};

export class KeyDatum extends Enum<KeyDatumV> {
  static Val(value: Value): KeyDatum { return new KeyDatum('Val', { value }); }
  static NegInfinity(vt: ValueType): KeyDatum { return new KeyDatum('NegInfinity', { valueType: vt }); }
  static PosInfinity(vt: ValueType): KeyDatum { return new KeyDatum('PosInfinity', { valueType: vt }); }

  /** Get the ValueType of this KeyDatum. */
  ty(): ValueType {
    return this.match({
      Val: (v) => valueType(v.value),
      NegInfinity: (v) => v.valueType,
      PosInfinity: (v) => v.valueType,
    });
  }

  /** From<Value> for KeyDatum */
  static fromValue(v: Value): KeyDatum {
    return KeyDatum.Val(v);
  }
}

// --- Endpoints & per-column bounds (PG: per-column ScanKey / bound) ----------

/** Endpoint for one side of a column bound (PG: strategy + flags collapsed). */
type EndpointV = {
  UnboundedLow: { valueType: ValueType };
  UnboundedHigh: { valueType: ValueType };
  Value: { datum: KeyDatum; inclusive: boolean };
};

export class Endpoint extends Enum<EndpointV> {
  static UnboundedLow(vt: ValueType): Endpoint { return new Endpoint('UnboundedLow', { valueType: vt }); }
  static UnboundedHigh(vt: ValueType): Endpoint { return new Endpoint('UnboundedHigh', { valueType: vt }); }
  static Value(datum: KeyDatum, inclusive: boolean): Endpoint { return new Endpoint('Value', { datum, inclusive }); }

  static incl(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), true); }
  static excl(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), false); }
}

// --- Per-column bound --------------------------------------------------------

/** Bound for a single index column, in index key order (PG: per keypart). */
export class KeyBoundComponent extends Struct {
  column: string;
  low: Endpoint;
  high: Endpoint;

  constructor(column: string, low: Endpoint, high: Endpoint) {
    super();
    this.column = column;
    this.low = low;
    this.high = high;
  }
}

// --- Multi-column bounds (PG: IndexBounds) -----------------------------------

/** Full multi-column bounds for an index scan (PG: IndexBounds). */
export class KeyBounds extends Struct {
  keyparts: KeyBoundComponent[];

  constructor(keyparts: KeyBoundComponent[]) {
    super();
    this.keyparts = keyparts;
  }

  static empty(): KeyBounds {
    return new KeyBounds([]);
  }
}

// --- Canonical, lexicographic interval after normalization -------------------

/**
 * Canonical lexicographic interval (possibly open-ended) ready for lowering.
 * lower/upper: [tuple, open?] where open===true means exclusive.
 */
export class CanonicalRange extends Struct {
  /** null => unbounded low */
  lower: [Value[], boolean] | null;
  /** null => unbounded high */
  upper: [Value[], boolean] | null;

  constructor(lower: [Value[], boolean] | null, upper: [Value[], boolean] | null) {
    super();
    this.lower = lower;
    this.upper = upper;
  }
}

// --- Backward-compat free functions (delegate to class methods) ---------------

export function orderByComponentsNew(presort: OrderByItem[], spill: OrderByItem[]): OrderByComponents {
  return new OrderByComponents(presort, spill);
}

export function orderByComponentsDefault(): OrderByComponents {
  return OrderByComponents.default();
}

export function orderByComponentsIsSatisfied(obc: OrderByComponents): boolean {
  return obc.isSatisfied();
}

export function orderByComponentsIsGlobalSpill(obc: OrderByComponents): boolean {
  return obc.isGlobalSpill();
}

export function keyDatumType(kd: KeyDatum): ValueType {
  return kd.ty();
}

export function keyDatumFromValue(v: Value): KeyDatum {
  return KeyDatum.fromValue(v);
}

export function endpointIncl(v: Value): Endpoint {
  return Endpoint.incl(v);
}

export function endpointExcl(v: Value): Endpoint {
  return Endpoint.excl(v);
}

export function keyBoundsNew(keyparts: KeyBoundComponent[]): KeyBounds {
  return new KeyBounds(keyparts);
}

export function keyBoundsEmpty(): KeyBounds {
  return KeyBounds.empty();
}
