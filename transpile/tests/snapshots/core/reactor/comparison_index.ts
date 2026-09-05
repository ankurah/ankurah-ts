// MIRRORS: ankurah/core/src/reactor/comparison_index.rs
import { Struct, HashMap, HashSet } from '@ankurah/base';
import { Collatable, Collatable_dispatch_predecessorBytes, Collatable_dispatch_successorBytes, Collatable_dispatch_toBytes } from '../collation';
import { ComparisonOperator } from '@ankurah/ankql';

class ComparisonIndex<T extends Clone & Eq & Hash & Ord> extends Struct {
  eq: HashMap<Uint8Array, T[]>;
  ne: HashMap<Uint8Array, T[]>;
  gt: HashMap<Uint8Array, T[]>;
  lt: HashMap<Uint8Array, T[]>;

  constructor(eq: HashMap<Uint8Array, T[]>, ne: HashMap<Uint8Array, T[]>, gt: HashMap<Uint8Array, T[]>, lt: HashMap<Uint8Array, T[]>) {
    super();
    this.eq = eq;
    this.ne = ne;
    this.gt = gt;
    this.lt = lt;
  }

  static new<T>(): ComparisonIndex<T> {
    return ComparisonIndex.Self.default();
  }

  forEntry<F, V>(value: V, op: ComparisonOperator, f: F): void {
    try {
      return op.match({
        Equal: () => {
          const entry = this.eq.entry(Collatable_dispatch_toBytes(value)).orDefault();
          f(entry);
        },
        NotEqual: () => {
          const entry = this.ne.entry(Collatable_dispatch_toBytes(value)).orDefault();
          f(entry);
        },
        GreaterThan: () => {
          const entry = this.gt.entry(Collatable_dispatch_toBytes(value)).orDefault();
          f(entry);
        },
        LessThan: () => {
          const entry = this.lt.entry(Collatable_dispatch_toBytes(value)).orDefault();
          f(entry);
        },
        GreaterThanOrEqual: () => {
          {
            const _v = Collatable_dispatch_predecessorBytes(value);
            if (_v != null) {
              const pred = _v;
              const entry = this.gt.entry(pred).orDefault();
              f(entry);
            } else {
            const entry = this.gt.entry([]).orDefault();
            f(entry);
          }
          }
        },
        LessThanOrEqual: () => {
          {
            const _v1 = Collatable_dispatch_successorBytes(value);
            if (_v1 != null) {
              const succ = _v1;
              const entry = this.lt.entry(succ).orDefault();
              f(entry);
            }
          }
        },
        In: () => {
          throw new Error(`Unsupported operator: ${op.debug()}`)
        },
        Between: () => {
          throw new Error(`Unsupported operator: ${op.debug()}`)
        },
      });
    } finally {
      op.drop();
    }
  }

  add<V extends Collatable>(value: V, op: ComparisonOperator, watcherId: T): void {
    this.forEntry(value, op, (entries) => entries.push(watcherId));
  }

  remove<V extends Collatable>(value: V, op: ComparisonOperator, watcherId: T): void {
    this.forEntry(value, op, (entries) => {
      {
        const _v = [...entries].findIndex((id) => id === watcherId);
        if (_v != null) {
          const pos = _v;
          entries.splice(pos, 1)[0];
        }
      }
    });
  }

  findMatching<V extends Collatable>(value: V): T[] {
    let result = new HashSet();
    const bytes = Collatable_dispatch_toBytes(value);
    {
      const _v = this.eq.get(bytes);
      if (_v != null) {
        const subs = _v;
        result.extend([...[...subs]]);
      }
    }
    for (const [storedBytes, subs] of this.ne) {
      if (bytes !== storedBytes) {
        result.extend([...[...subs]]);
      }
    }
    for (const [_threshold, subs] of this.gt.range(undefined /* range ..bytes.clone() */)) {
      result.extend([...[...subs]]);
    }
    {
      const _v1 = Collatable_dispatch_successorBytes(value);
      if (_v1 != null) {
        const pred = _v1;
        for (const [_threshold, subs] of this.lt.range(undefined /* range pred.. */)) {
          result.extend([...[...subs]]);
        }
      }
    }
    return [...result];
  }

  static default<T>(): ComparisonIndex<T> {
    return new ComparisonIndex(new HashMap(), new HashMap(), new HashMap(), new HashMap());
  }

  debug(): string {
    return `ComparisonIndex { eq: ${this.eq}, ne: ${this.ne}, gt: ${this.gt}, lt: ${this.lt} }`;
  }
}

