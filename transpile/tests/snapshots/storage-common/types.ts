// MIRRORS: ankurah/storage/common/src/types.rs
import { Struct, Enum, HashMap, HashSet } from '@ankurah/base';
import { KeySpec, Value, ValueType } from '@ankurah/core';
import { OrderByItem, Predicate } from '@ankurah/ankql';

export class OrderByComponents extends Struct {
  readonly presort: OrderByItem[];
  readonly spill: OrderByItem[];

  constructor(presort: OrderByItem[], spill: OrderByItem[]) {
    super();
    this.presort = presort;
    this.spill = spill;
  }

  static new(presort: OrderByItem[], spill: OrderByItem[]): OrderByComponents {
    return new OrderByComponents(presort, spill);
  }

  isSatisfied(): boolean {
    return this.spill.length === 0;
  }

  isGlobalSpill(): boolean {
    return this.presort.length === 0 && !(this.spill.length === 0);
  }

  equals(other: OrderByComponents): boolean {
    { if (this.presort.length !== other.presort.length) return false; for (let i = 0; i < this.presort.length; i++) { if (!this.presort[i].equals(other.presort[i])) return false; } }
    { if (this.spill.length !== other.spill.length) return false; for (let i = 0; i < this.spill.length; i++) { if (!this.spill[i].equals(other.spill[i])) return false; } }
    return true;
  }

  clone(): OrderByComponents {
    return new OrderByComponents(this.presort.map(e => e.clone()), this.spill.map(e => e.clone()));
  }

  static default(): OrderByComponents {
    return new OrderByComponents([], []);
  }

  debug(): string {
    return `OrderByComponents { presort: ${`[${Array.from(this.presort).map((e) => e.debug()).join(', ')}]`}, spill: ${`[${Array.from(this.spill).map((e) => e.debug()).join(', ')}]`} }`;
  }
}

export class KeyBoundComponent extends Struct {
  readonly column: string;
  readonly low: Endpoint;
  readonly high: Endpoint;

  constructor(column: string, low: Endpoint, high: Endpoint) {
    super();
    this.column = column;
    this.low = low;
    this.high = high;
  }

  equals(other: KeyBoundComponent): boolean {
    if (this.column !== other.column) return false;
    if (!this.low.equals(other.low)) return false;
    if (!this.high.equals(other.high)) return false;
    return true;
  }

  clone(): KeyBoundComponent {
    return new KeyBoundComponent(this.column, this.low.clone(), this.high.clone());
  }

  debug(): string {
    return `KeyBoundComponent { column: ${JSON.stringify(this.column)}, low: ${this.low.debug()}, high: ${this.high.debug()} }`;
  }
}

export class KeyBounds extends Struct {
  readonly keyparts: KeyBoundComponent[];

  constructor(keyparts: KeyBoundComponent[]) {
    super();
    this.keyparts = keyparts;
  }

  static new(keyparts: KeyBoundComponent[]): KeyBounds {
    return new KeyBounds(keyparts);
  }

  static empty(): KeyBounds {
    return new KeyBounds([]);
  }

  equals(other: KeyBounds): boolean {
    { if (this.keyparts.length !== other.keyparts.length) return false; for (let i = 0; i < this.keyparts.length; i++) { if (!this.keyparts[i].equals(other.keyparts[i])) return false; } }
    return true;
  }

  clone(): KeyBounds {
    return new KeyBounds(this.keyparts.map(e => e.clone()));
  }

  debug(): string {
    return `KeyBounds { keyparts: ${`[${Array.from(this.keyparts).map((e) => e.debug()).join(', ')}]`} }`;
  }
}

export class CanonicalRange extends Struct {
  readonly lower: [Value[], boolean] | null;
  readonly upper: [Value[], boolean] | null;

  constructor(lower: [Value[], boolean] | null, upper: [Value[], boolean] | null) {
    super();
    this.lower = lower;
    this.upper = upper;
  }

  equals(other: CanonicalRange): boolean {
    { if ((this.lower == null) !== (other.lower == null)) return false; if (this.lower != null) { { { if (this.lower![0].length !== other.lower![0].length) return false; for (let i1 = 0; i1 < this.lower![0].length; i1++) { if (!this.lower![0][i1].equals(other.lower![0][i1])) return false; } } if (this.lower![1] !== other.lower![1]) return false; } } }
    { if ((this.upper == null) !== (other.upper == null)) return false; if (this.upper != null) { { { if (this.upper![0].length !== other.upper![0].length) return false; for (let i1 = 0; i1 < this.upper![0].length; i1++) { if (!this.upper![0][i1].equals(other.upper![0][i1])) return false; } } if (this.upper![1] !== other.upper![1]) return false; } } }
    return true;
  }

  clone(): CanonicalRange {
    return new CanonicalRange((this.lower != null ? [this.lower[0].map(e1 => e1.clone()), this.lower[1]] as [Value[], boolean] : null), (this.upper != null ? [this.upper[0].map(e1 => e1.clone()), this.upper[1]] as [Value[], boolean] : null));
  }

  debug(): string {
    return `CanonicalRange { lower: ${this.lower}, upper: ${this.upper} }`;
  }
}

export type PlanV = {
  Index: { indexSpec: KeySpec; scanDirection: ScanDirection; bounds: KeyBounds; remainingPredicate: Predicate; orderBySpill: OrderByComponents };
  TableScan: { bounds: KeyBounds; scanDirection: ScanDirection; remainingPredicate: Predicate; orderBySpill: OrderByComponents };
  EmptyScan: {};
};

export class Plan extends Enum<PlanV> {

  clone(): Plan {
    return this.match({
      Index: (v) => new Plan('Index', { indexSpec: v.indexSpec.clone(), scanDirection: v.scanDirection.clone(), bounds: v.bounds.clone(), remainingPredicate: v.remainingPredicate.clone(), orderBySpill: v.orderBySpill.clone() }),
      TableScan: (v) => new Plan('TableScan', { bounds: v.bounds.clone(), scanDirection: v.scanDirection.clone(), remainingPredicate: v.remainingPredicate.clone(), orderBySpill: v.orderBySpill.clone() }),
      EmptyScan: () => new Plan('EmptyScan', {}),
    });
  }

  equals(other: Plan): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Index': {
        if (!(this.value as any).indexSpec.equals((other.value as any).indexSpec)) return false;
        if (!(this.value as any).scanDirection.equals((other.value as any).scanDirection)) return false;
        if (!(this.value as any).bounds.equals((other.value as any).bounds)) return false;
        if (!(this.value as any).remainingPredicate.equals((other.value as any).remainingPredicate)) return false;
        if (!(this.value as any).orderBySpill.equals((other.value as any).orderBySpill)) return false;
        break;
      }
      case 'TableScan': {
        if (!(this.value as any).bounds.equals((other.value as any).bounds)) return false;
        if (!(this.value as any).scanDirection.equals((other.value as any).scanDirection)) return false;
        if (!(this.value as any).remainingPredicate.equals((other.value as any).remainingPredicate)) return false;
        if (!(this.value as any).orderBySpill.equals((other.value as any).orderBySpill)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Index: (v) => `Index { indexSpec: ${v.indexSpec.debug()}, scanDirection: ${v.scanDirection.debug()}, bounds: ${v.bounds.debug()}, remainingPredicate: ${v.remainingPredicate.debug()}, orderBySpill: ${v.orderBySpill.debug()} }`,
      TableScan: (v) => `TableScan { bounds: ${v.bounds.debug()}, scanDirection: ${v.scanDirection.debug()}, remainingPredicate: ${v.remainingPredicate.debug()}, orderBySpill: ${v.orderBySpill.debug()} }`,
      EmptyScan: () => 'EmptyScan',
    });
  }
}

export type ScanDirectionV = {
  Forward: {};
  Reverse: {};
};

export class ScanDirection extends Enum<ScanDirectionV> {

  clone(): ScanDirection {
    return new ScanDirection(this.type, { ...this.value });
  }

  equals(other: ScanDirection): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      Forward: () => 'Forward',
      Reverse: () => 'Reverse',
    });
  }
}

export type KeyDatumV = {
  Val: { _0: Value };
  NegInfinity: { _0: ValueType };
  PosInfinity: { _0: ValueType };
};

export class KeyDatum extends Enum<KeyDatumV> {

  ty(): ValueType {
    return this.match({
      Val: (_v) => {
        const v = _v._0;
        return ValueType.of(v);
      },
      NegInfinity: (v) => {
        const t = v._0;
        return t;
      },
      PosInfinity: (v) => {
        const t = v._0;
        return t;
      },
    });
  }

  static fromValue(v: Value): KeyDatum {
    return new KeyDatum('Val', { _0: v });
  }

  clone(): KeyDatum {
    return this.match({
      Val: (v) => new KeyDatum('Val', { _0: v._0.clone() }),
      NegInfinity: (v) => new KeyDatum('NegInfinity', { _0: v._0.clone() }),
      PosInfinity: (v) => new KeyDatum('PosInfinity', { _0: v._0.clone() }),
    });
  }

  equals(other: KeyDatum): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Val': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'NegInfinity': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'PosInfinity': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Val: (v) => `Val(${v._0.debug()})`,
      NegInfinity: (v) => `NegInfinity(${v._0.debug()})`,
      PosInfinity: (v) => `PosInfinity(${v._0.debug()})`,
    });
  }
}

export type EndpointV = {
  UnboundedLow: { _0: ValueType };
  UnboundedHigh: { _0: ValueType };
  Value: { datum: KeyDatum; inclusive: boolean };
};

export class Endpoint extends Enum<EndpointV> {

  static incl(v: Value): Endpoint {
    return new Endpoint('Value', { datum: new KeyDatum('Val', { _0: v }), inclusive: true });
  }

  static excl(v: Value): Endpoint {
    return new Endpoint('Value', { datum: new KeyDatum('Val', { _0: v }), inclusive: false });
  }

  clone(): Endpoint {
    return this.match({
      UnboundedLow: (v) => new Endpoint('UnboundedLow', { _0: v._0.clone() }),
      UnboundedHigh: (v) => new Endpoint('UnboundedHigh', { _0: v._0.clone() }),
      Value: (v) => new Endpoint('Value', { datum: v.datum.clone(), inclusive: v.inclusive }),
    });
  }

  equals(other: Endpoint): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'UnboundedLow': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'UnboundedHigh': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Value': {
        if (!(this.value as any).datum.equals((other.value as any).datum)) return false;
        if ((this.value as any).inclusive !== (other.value as any).inclusive) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      UnboundedLow: (v) => `UnboundedLow(${v._0.debug()})`,
      UnboundedHigh: (v) => `UnboundedHigh(${v._0.debug()})`,
      Value: (v) => `Value { datum: ${v.datum.debug()}, inclusive: ${String(v.inclusive)} }`,
    });
  }
}

