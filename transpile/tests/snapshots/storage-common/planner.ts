// MIRRORS: ankurah/storage/common/src/planner.rs
import { Struct, dropOwned, unsupported, iterFind, iterFindMap, iterFirst, HashSet } from '@ankurah/base';
import { ComparisonOperator, Predicate, OrderByItem, Selection } from '@ankurah/ankql';
import { IndexKeyPart, KeySpec, Value, ValueType } from '@ankurah/core';
import { ConjunctFinder } from './predicate';
import { Endpoint, KeyBoundComponent, KeyBounds, KeyDatum, OrderByComponents, Plan, ScanDirection } from './types';

export class PlannerConfig extends Struct {
  readonly supportsDescIndexes: boolean;

  constructor(supportsDescIndexes: boolean) {
    super();
    this.supportsDescIndexes = supportsDescIndexes;
  }

  static new(supportsDescIndexes: boolean): PlannerConfig {
    return new PlannerConfig(supportsDescIndexes);
  }

  static indexeddb(): PlannerConfig {
    return PlannerConfig.new(false);
  }

  static fullSupport(): PlannerConfig {
    return PlannerConfig.new(true);
  }

  clone(): PlannerConfig {
    return new PlannerConfig(this.supportsDescIndexes);
  }

  debug(): string {
    return `PlannerConfig { supportsDescIndexes: ${String(this.supportsDescIndexes)} }`;
  }
}

export class Planner extends Struct {
  config: PlannerConfig;

  constructor(config: PlannerConfig) {
    super();
    this.config = config;
  }

  static new(config: PlannerConfig): Planner {
    return new Planner(config);
  }

  plan(selection: Selection, primaryKey: string): Plan[] {
    const conjuncts = ConjunctFinder.find(selection.predicate);
    try {
      const [equalities, inequalities] = this.categorizeConjunctsExcludingPrimaryKey(conjuncts, primaryKey);
      const hasPrimaryKeyRanges = this.hasPrimaryKeyRangePredicates(conjuncts, primaryKey);
      const hasPrimaryKeyOrderBy = this.hasPrimaryKeyOrderBy(selection.orderBy, primaryKey);
      const hasNonPrimaryPredicates = [...conjuncts].some((pred) => !pred.is('True') && !this.isPrimaryKeyPredicate(pred, primaryKey));
      if ((hasPrimaryKeyRanges || hasPrimaryKeyOrderBy) && !hasNonPrimaryPredicates) {
        const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
        return [tableScan];
      }
      let plans = [];
      {
        const _v2 = selection.orderBy;
        if (_v2 != null) {
          const orderBy = _v2;
          if (!(orderBy.length === 0)) {
            {
              const _v = this.buildOrderFirstPlan(equalities, inequalities, orderBy, conjuncts);
              if (_v != null) {
                const plan = _v;
                plans.push(plan);
              }
            }
            const coveredIneq = [...orderBy].some((item) => (item.path.isSimple() ? inequalities.containsKey(item.path.first()) : false));
            if (!coveredIneq) {
              if (!inequalities.isEmpty()) {
                {
                  const _v1 = this.buildIneqFirstPlan(equalities, inequalities, orderBy, conjuncts);
                  if (_v1 != null) {
                    const plan = _v1;
                    plans.push(plan);
                  }
                }}}
            let _moved0 = false;
            const deduplicatedPlans = this.deduplicatePlans(plans);
            try {
              const hasEmptyScan = [...deduplicatedPlans].some((plan) => plan.is('EmptyScan'));
              if (!hasEmptyScan) {
                _moved0 = true;
                let finalPlans = deduplicatedPlans;
                const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
                finalPlans.push(tableScan);
                return finalPlans;
              } else {
                _moved0 = true;
                return deduplicatedPlans;
              }
            } finally {
              if (!_moved0) dropOwned(deduplicatedPlans);
            }
          }
        }
      }
      if (!inequalities.isEmpty()) {
        for (const [field, ] of inequalities) {
          {
            const _v3 = this.generateInequalityPlanWithOrderBy(equalities, field, inequalities, conjuncts, selection.orderBy);
            if (_v3 != null) {
              const plan = _v3;
              plans.push(plan);
            }
          }
        }
      } else if (!(equalities.length === 0)) {
        {
          const _v4 = this.generateEqualityPlan(equalities, conjuncts);
          if (_v4 != null) {
            const plan = _v4;
            plans.push(plan);
          }
        }
      }
      let _moved1 = false;
      const deduplicatedPlans = this.deduplicatePlans(plans);
      try {
        const hasEmptyScan = [...deduplicatedPlans].some((plan) => plan.is('EmptyScan'));
        if (!hasEmptyScan) {
          _moved1 = true;
          let finalPlans = deduplicatedPlans;
          const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
          finalPlans.push(tableScan);
          return finalPlans;
        } else {
          _moved1 = true;
          return deduplicatedPlans;
        }
      } finally {
        if (!_moved1) dropOwned(deduplicatedPlans);
      }
    } finally {
      dropOwned(conjuncts);
    }
  }

  buildOrderFirstPlan(equalities: [string, Value][], inequalities: IndexMap<string, [ComparisonOperator, Value][], RandomState>, orderBy: OrderByItem[], conjuncts: Predicate[]): Plan | null {
    if (orderBy.length === 0) {
      return null;
    }
    let _moved0 = false;
    let indexKeyparts = [...equalities].map(([f, v]) => IndexKeyPart.ascPath(f, ValueType.of(v)));
    try {
      if (this.config.supportsDescIndexes) {
        for (const item of orderBy) {
          if (item.path.isSimple()) {
            const name = item.path.first();
            indexKeyparts.push(item.direction.match({
              Asc: () => IndexKeyPart.asc(name, new ValueType('String', {})),
              Desc: () => IndexKeyPart.desc(name, new ValueType('String', {})),
            }));
          }
        }
      } else {
        const firstDir = orderBy[0].direction.clone();
        try {
          let broke = false;
          for (const item of orderBy) {
            if (item.path.isSimple()) {
              const name = item.path.first();
              if (!broke && item.direction.equals(firstDir)) {
                indexKeyparts.push(IndexKeyPart.asc(name, new ValueType('String', {})));
              } else {
                broke = true;
              }
            }
          }
        } finally {
          firstDir.drop();
        }
      }
      const appliedIneq = iterFindMap([...orderBy], (item) => {
        if (item.path.isSimple()) {
          const name = item.path.first();
          return inequalities.getKeyValue(name).map(([k, v]) => [k.asStr(), v]);
        } else {
          return null;
        }
      });
      const _m3 = (() => {
        if (appliedIneq != null) {
          const [field, vec] = appliedIneq;
          const _r1 = this.buildBounds(equalities, [field, vec], indexKeyparts);
          if (_r1 == null) return { $jump: 'return', $value: null };
          return _r1;
        } else {
          const _r2 = this.buildBounds(equalities, null, indexKeyparts);
          if (_r2 == null) return { $jump: 'return', $value: null };
          return _r2;
        }
      })();
      if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
      let _moved4 = false;
      const bounds = (_m3 as any);
      try {
        if (this.isEmptyBounds(bounds)) {
          return new Plan('EmptyScan', {});
        }
        let _moved5 = false;
        const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, (appliedIneq != null ? (([f, ]) => f)(appliedIneq!) : null));
        try {
          const scanDirection = (this.config.supportsDescIndexes ? new ScanDirection('Forward', {}) : orderBy[0].direction.match({
            Desc: () => new ScanDirection('Reverse', {}),
            Asc: () => new ScanDirection('Forward', {}),
          }));
          let _moved6 = false;
          const orderBy_1 = (() => {
            if (!this.config.supportsDescIndexes) {
              const firstDir = orderBy[0].direction.clone();
              try {
                let presort = [];
                let spill = [];
                let broke = false;
                for (const item of orderBy) {
                  if (item.path.isSimple()) {
                    if (!broke && item.direction.equals(firstDir)) {
                      presort.push(item.clone());
                    } else {
                      broke = true;
                      spill.push(item.clone());
                    }
                  }
                }
                return OrderByComponents.new(presort, spill);
              } finally {
                firstDir.drop();
              }
            } else {
              return OrderByComponents.new(orderBy.map((e) => e.clone()), []);
            }
          })();
          try {
            _moved4 = true;
            _moved5 = true;
            _moved6 = true;
            _moved0 = true;
            return new Plan('Index', { indexSpec: KeySpec.new(indexKeyparts), scanDirection: scanDirection, bounds: bounds, remainingPredicate: remainingPredicate, orderBySpill: orderBy_1 });
          } finally {
            if (!_moved6) orderBy_1.drop();
          }
        } finally {
          if (!_moved5) remainingPredicate.drop();
        }
      } finally {
        if (!_moved4) bounds.drop();
      }
    } finally {
      if (!_moved0) dropOwned(indexKeyparts);
    }
  }

  buildIneqFirstPlan(equalities: [string, Value][], inequalities: IndexMap<string, [ComparisonOperator, Value][], RandomState>, orderBy: OrderByItem[], conjuncts: Predicate[]): Plan | null {
    const _r1 = iterFindMap([...orderBy], (item) => {
      if (item.path.isSimple()) {
        const name = item.path.first();
        return inequalities.getKeyValue(name).map(([k, v]) => [k.asStr(), v]);
      } else {
        return null;
      }
    }).orElse(() => {
      const _m0 = unsupported('`next` advances an iterator\'s cursor, and the port writes an iterator as the whole sequence with no cursor to advance');
      return (_m0 != null ? (([k, v]) => [k, v])(_m0!) : null);
    });
    const primary = _r1;
    let _moved2 = false;
    let indexKeyparts = [...equalities].map(([f, v]) => IndexKeyPart.ascPath(f, ValueType.of(v)));
    try {
      const primaryValue = primary._1[0]._1;
      indexKeyparts.push(IndexKeyPart.ascPath(primary._0, ValueType.of(primaryValue)));
      const _r3 = this.buildBounds(equalities, primary, indexKeyparts);
      if (_r3 == null) return null;
      let _moved4 = false;
      const bounds = _r3;
      try {
        if (this.isEmptyBounds(bounds)) {
          return new Plan('EmptyScan', {});
        }
        let _moved5 = false;
        const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, primary._0);
        try {
          const scanDirection = (this.config.supportsDescIndexes ? new ScanDirection('Forward', {}) : orderBy[0].direction.match({
            Desc: () => new ScanDirection('Reverse', {}),
            Asc: () => new ScanDirection('Forward', {}),
          }));
          let covered = new HashSet();
          covered.extend([...equalities].map(([f, ]) => f));
          covered.insert(primary._0);
          let presort = [];
          let spill = [];
          for (const item of orderBy) {
            if (item.path.isSimple()) {
              const name = item.path.first();
              if (covered.includes(name)) {
                presort.push(item.clone());
              } else {
                spill.push(item.clone());
              }
            }
          }
          let _moved6 = false;
          const orderBy_1 = OrderByComponents.new(presort, spill);
          try {
            _moved4 = true;
            _moved5 = true;
            _moved6 = true;
            _moved2 = true;
            return new Plan('Index', { indexSpec: KeySpec.new(indexKeyparts), scanDirection: scanDirection, bounds: bounds, remainingPredicate: remainingPredicate, orderBySpill: orderBy_1 });
          } finally {
            if (!_moved6) orderBy_1.drop();
          }
        } finally {
          if (!_moved5) remainingPredicate.drop();
        }
      } finally {
        if (!_moved4) bounds.drop();
      }
    } finally {
      if (!_moved2) dropOwned(indexKeyparts);
    }
  }

  categorizeConjunctsExcludingPrimaryKey(conjuncts: Predicate[], primaryKey: string): [[string, Value][], IndexMap<string, [ComparisonOperator, Value][], RandomState>] {
    let equalities = [];
    let inequalities = IndexMap.new();
    for (const conjunct of conjuncts) {
      {
        const _v = this.extractComparison(conjunct);
        if (_v != null) {
          const [field, op, value] = _v;
          let _moved0 = false;
          let _moved1 = false;
          try {
            try {
              if (field === primaryKey) {
                continue;
              }
              return op.match({
                Equal: () => {
                  _moved1 = true;
                  equalities.push([field, value]);
                },
                GreaterThan: () => {
                  _moved0 = true;
                  _moved1 = true;
                  inequalities.entry(field).orDefault().push([op, value]);
                },
                GreaterThanOrEqual: () => {
                  _moved0 = true;
                  _moved1 = true;
                  inequalities.entry(field).orDefault().push([op, value]);
                },
                LessThan: () => {
                  _moved0 = true;
                  _moved1 = true;
                  inequalities.entry(field).orDefault().push([op, value]);
                },
                LessThanOrEqual: () => {
                  _moved0 = true;
                  _moved1 = true;
                  inequalities.entry(field).orDefault().push([op, value]);
                },
                NotEqual: () => {},
                In: () => {},
                Between: () => {},
              });
            } finally {
              if (!_moved1) value.drop();
            }
          } finally {
            if (!_moved0) op.drop();
          }
        }
      }
    }
    return [equalities, inequalities];
  }

  extractComparison(predicate: Predicate): [string, ComparisonOperator, Value] | null {
    return predicate.match({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        const _m0 = (() => {
          return left.asRef().match<any>({
            Path: (v) => {
              const path = v._0;
              return path.steps.join('.');
            },
            Literal: () => {
              return { $jump: 'return', $value: null };
            },
            Predicate: () => {
              return { $jump: 'return', $value: null };
            },
            InfixExpr: () => {
              return { $jump: 'return', $value: null };
            },
            ExprList: () => {
              return { $jump: 'return', $value: null };
            },
            Placeholder: () => {
              return { $jump: 'return', $value: null };
            },
          });
        })();
        if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
        const fieldPath = (_m0 as any);
        const _m1 = (() => {
          return right.asRef().match<any>({
            Literal: (v) => {
              const literal = v._0;
              return literal;
            },
            Path: () => {
              return { $jump: 'return', $value: null };
            },
            Predicate: () => {
              return { $jump: 'return', $value: null };
            },
            InfixExpr: () => {
              return { $jump: 'return', $value: null };
            },
            ExprList: () => {
              return { $jump: 'return', $value: null };
            },
            Placeholder: () => {
              return { $jump: 'return', $value: null };
            },
          });
        })();
        if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
        const value = (_m1 as any);
        return [fieldPath, operator.clone(), value];
      },
      IsNull: () => null,
      And: () => null,
      Or: () => null,
      Not: () => null,
      True: () => null,
      False: () => null,
      Placeholder: () => null,
    });
  }

  generateInequalityPlanWithOrderBy(equalities: [string, Value][], inequalityField: string, inequalities: IndexMap<string, [ComparisonOperator, Value][], RandomState>, conjuncts: Predicate[], orderBy: OrderByItem[] | null): Plan | null {
    let indexKeyparts = [];
    for (const [field, value] of equalities) {
      indexKeyparts.push(IndexKeyPart.ascPath(field, ValueType.of(value)));
    }
    const _r0 = inequalities.get(inequalityField);
    if (_r0 == null) return null;
    const inequalityValues = _r0;
    let _moved1 = false;
    const firstInequalityValue = inequalityValues[0][1];
    try {
      _moved1 = true;
      indexKeyparts.push(IndexKeyPart.ascPath(inequalityField, ValueType.of(firstInequalityValue)));
      let _moved2 = false;
      const bounds = this.buildBounds(equalities, [inequalityField, inequalityValues], indexKeyparts);
      try {
        _moved2 = true;
        const _m4 = (() => {
          const _v = bounds;
          if (_v != null) {
            const bounds = _v;
            let _moved3 = false;
            try {
              {
                if (this.isEmptyBounds(bounds)) {
                  return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
                }
                _moved3 = true;
                return bounds;
              }
            } finally {
              if (!_moved3) bounds.drop();
            }
          } else {
            return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
          }
        })();
        if ((_m4 as any)?.$jump === 'return') return (_m4 as any).$value;
        let _moved5 = false;
        const bounds_1 = (_m4 as any);
        try {
          let _moved6 = false;
          const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, inequalityField);
          try {
            let _moved7 = false;
            const orderBySpill = (() => {
              {
                const _v1 = orderBy;
                if (_v1 != null) {
                  const orderByItems = _v1;
                  const coveredFields = HashSet.from([...[...equalities].map(([f, ]) => f), ...once(inequalityField)]);
                  let presort = [];
                  let spill = [];
                  for (const item of orderByItems) {
                    if (item.path.isSimple()) {
                      const name = item.path.first();
                      if (coveredFields.has(name)) {
                        presort.push(item.clone());
                      } else {
                        spill.push(item.clone());
                      }
                    }
                  }
                  return OrderByComponents.new(presort, spill);
                } else {
                return OrderByComponents.default();
              }
              }
            })();
            try {
              let _moved8 = false;
              const indexSpec = KeySpec.new(indexKeyparts);
              try {
                _moved8 = true;
                _moved5 = true;
                _moved6 = true;
                _moved7 = true;
                return new Plan('Index', { indexSpec: indexSpec, scanDirection: new ScanDirection('Forward', {}), bounds: bounds_1, remainingPredicate: remainingPredicate, orderBySpill: orderBySpill });
              } finally {
                if (!_moved8) indexSpec.drop();
              }
            } finally {
              if (!_moved7) orderBySpill.drop();
            }
          } finally {
            if (!_moved6) remainingPredicate.drop();
          }
        } finally {
          if (!_moved5) dropOwned(bounds_1);
        }
      } finally {
        if (!_moved2) dropOwned(bounds);
      }
    } finally {
      if (!_moved1) firstInequalityValue.drop();
    }
  }

  generateEqualityPlan(equalities: [string, Value][], conjuncts: Predicate[]): Plan | null {
    let indexKeyparts = [];
    for (const [field, value] of equalities) {
      indexKeyparts.push(IndexKeyPart.ascPath(field, ValueType.of(value)));
    }
    const bounds = this.buildBounds(equalities, null, indexKeyparts);
    const _m1 = (() => {
      const _v = bounds;
      if (_v != null) {
        const bounds = _v;
        let _moved0 = false;
        try {
          {
            if (this.isEmptyBounds(bounds)) {
              return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
            }
            _moved0 = true;
            return bounds;
          }
        } finally {
          if (!_moved0) bounds.drop();
        }
      } else {
        return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
      }
    })();
    if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
    let _moved2 = false;
    const bounds_1 = (_m1 as any);
    try {
      let _moved3 = false;
      const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, null);
      try {
        let _moved4 = false;
        const indexSpec = KeySpec.new(indexKeyparts);
        try {
          _moved4 = true;
          _moved2 = true;
          _moved3 = true;
          return new Plan('Index', { indexSpec: indexSpec, scanDirection: new ScanDirection('Forward', {}), bounds: bounds_1, remainingPredicate: remainingPredicate, orderBySpill: OrderByComponents.default() });
        } finally {
          if (!_moved4) indexSpec.drop();
        }
      } finally {
        if (!_moved3) remainingPredicate.drop();
      }
    } finally {
      if (!_moved2) dropOwned(bounds_1);
    }
  }

  buildBounds(equalities: [string, Value][], inequality: [string, [ComparisonOperator, Value][]] | null, indexKeyparts: IndexKeyPart[]): KeyBounds | null {
    let keypartBounds = [];
    for (const keypart of indexKeyparts) {
      const fullPath = keypart.fullPath();
      const _m0 = iterFind([...equalities], ([field, ]) => field === fullPath);
      const equalityValue = (_m0 != null ? (([, value]) => value)(_m0!) : null);
      {
        const _v1 = equalityValue;
        if (_v1 != null) {
          const value = _v1;
          keypartBounds.push(new KeyBoundComponent(fullPath, Endpoint.incl(value.clone()), Endpoint.incl(value.clone())));
        } else {
        const _v = inequality;
        if (_v != null) {
          const [ineqField, inequalities] = _v;
          if (ineqField === fullPath) {
            let low = new Endpoint('UnboundedLow', { _0: ValueType.of(inequalities[0][1]) });
            let high = new Endpoint('UnboundedHigh', { _0: ValueType.of(inequalities[0][1]) });
            for (const [op, value] of inequalities) {
              op.match({
                GreaterThan: () => {
                  let _moved1 = false;
                  const candidate = Endpoint.excl(value.clone());
                  try {
                    if (this.isMoreRestrictiveLower(candidate, low)) {
                      _moved1 = true;
                      const _a2 = candidate;
                      low.drop();
                      low = _a2;
                    }
                  } finally {
                    if (!_moved1) candidate.drop();
                  }
                },
                GreaterThanOrEqual: () => {
                  let _moved3 = false;
                  const candidate = Endpoint.incl(value.clone());
                  try {
                    if (this.isMoreRestrictiveLower(candidate, low)) {
                      _moved3 = true;
                      const _a4 = candidate;
                      low.drop();
                      low = _a4;
                    }
                  } finally {
                    if (!_moved3) candidate.drop();
                  }
                },
                LessThan: () => {
                  let _moved5 = false;
                  const candidate = Endpoint.excl(value.clone());
                  try {
                    if (this.isMoreRestrictiveUpper(candidate, high)) {
                      _moved5 = true;
                      const _a6 = candidate;
                      high.drop();
                      high = _a6;
                    }
                  } finally {
                    if (!_moved5) candidate.drop();
                  }
                },
                LessThanOrEqual: () => {
                  let _moved7 = false;
                  const candidate = Endpoint.incl(value.clone());
                  try {
                    if (this.isMoreRestrictiveUpper(candidate, high)) {
                      _moved7 = true;
                      const _a8 = candidate;
                      high.drop();
                      high = _a8;
                    }
                  } finally {
                    if (!_moved7) candidate.drop();
                  }
                },
                Equal: () => {},
                NotEqual: () => {},
                In: () => {},
                Between: () => {},
              });
            }
            keypartBounds.push(new KeyBoundComponent(fullPath, low, high));
            break;
          } else {
            break;
          }
        } else {
        break;
      }
      }
      }
    }
    return KeyBounds.new(keypartBounds);
  }

  isMoreRestrictiveLower(candidate: Endpoint, current: Endpoint): boolean {
    const _v = [candidate, current];
    if ((_v[0].is('Value')) && (_v[1].is('UnboundedLow'))) {
      return true;
    } else if ((_v[0].is('UnboundedLow')) && (_v[1].is('Value'))) {
      return false;
    } else if ((_v[0].is('Value')) && (_v[1].is('Value'))) {
      const { datum: candDatum, inclusive: candIncl } = _v[0].value;
      const { datum: currDatum, inclusive: currIncl } = _v[1].value;
      const _v1 = [candDatum, currDatum];
      if ((_v1[0].is('Val')) && (_v1[1].is('Val'))) {
        const { _0: candVal } = _v1[0].value;
        const { _0: currVal } = _v1[1].value;
        const _v2 = candVal.partialCompareTo(currVal);
        if (_v2 != null && (_v2 === 1)) {
          return true;
        } else if (_v2 != null && (_v2 === 0)) {
          return !candIncl && currIncl;
        } else if (_v2 != null && (_v2 === -1)) {
          return false;
        } else {
          return false;
        }
      } else {
        return false;
      }
    } else {
      return false;
    }
  }

  isMoreRestrictiveUpper(candidate: Endpoint, current: Endpoint): boolean {
    const _v = [candidate, current];
    if ((_v[0].is('Value')) && (_v[1].is('UnboundedHigh'))) {
      return true;
    } else if ((_v[0].is('UnboundedHigh')) && (_v[1].is('Value'))) {
      return false;
    } else if ((_v[0].is('Value')) && (_v[1].is('Value'))) {
      const { datum: candDatum, inclusive: candIncl } = _v[0].value;
      const { datum: currDatum, inclusive: currIncl } = _v[1].value;
      const _v1 = [candDatum, currDatum];
      if ((_v1[0].is('Val')) && (_v1[1].is('Val'))) {
        const { _0: candVal } = _v1[0].value;
        const { _0: currVal } = _v1[1].value;
        const _v2 = candVal.partialCompareTo(currVal);
        if (_v2 != null && (_v2 === -1)) {
          return true;
        } else if (_v2 != null && (_v2 === 0)) {
          return !candIncl && currIncl;
        } else if (_v2 != null && (_v2 === 1)) {
          return false;
        } else {
          return false;
        }
      } else {
        return false;
      }
    } else {
      return false;
    }
  }

  isEmptyBounds(bounds: KeyBounds): boolean {
    for (const bound of bounds.keyparts) {
      const _v = [bound.low, bound.high];
      if ((_v[0].is('Value')) && (_v[1].is('Value'))) {
        const { datum: lowDatum, inclusive: lowIncl } = _v[0].value;
        const { datum: highDatum, inclusive: highIncl } = _v[1].value;
        const _v1 = [lowDatum, highDatum];
        if ((_v1[0].is('Val')) && (_v1[1].is('Val'))) {
          const { _0: lowVal } = _v1[0].value;
          const { _0: highVal } = _v1[1].value;
          const _v2 = lowVal.partialCompareTo(highVal);
          if (_v2 != null && (_v2 === 1)) {
            return true;
          } else if (_v2 != null && (_v2 === 0)) {
            if (!lowIncl && !highIncl) {
              return true;
            }
          } else if (_v2 != null && (_v2 === -1)) {
            {
            }
          } else {
            {
            }
          }
        } else {
          {
          }
        }
      } else {

      }
    }
    return false;
  }

  calculateRemainingPredicate(conjuncts: Predicate[], consumedEqualities: [string, Value][], consumedInequalityField: string | null): Predicate {
    let remainingConjuncts = [];
    for (const conjunct of conjuncts) {
      let consumed = false;
      {
        const _v1 = this.extractComparison(conjunct);
        if (_v1 != null) {
          const [field, , ] = _v1;
          for (const [eqField, ] of consumedEqualities) {
            if (field === eqField) {
              consumed = true;
              break;
            }
          }
          if (!consumed) {
            {
              const _v = consumedInequalityField;
              if (_v != null) {
                const ineqField = _v;
                if (field === ineqField) {
                  consumed = true;
                }  }
            }}
        }
      }
      if (!consumed) {
        remainingConjuncts.push(conjunct.clone());
      }
    }
    if (remainingConjuncts.length === 0) {
      return new Predicate('True', {});
    } else if (remainingConjuncts.length === 1) {
      return [...remainingConjuncts].next();
    } else {
      let result = remainingConjuncts[0].clone();
      for (const conjunct of [...remainingConjuncts].slice(1)) {
        result = new Predicate('And', { _0: result, _1: conjunct });
      }
      return result;
    }
  }

  deduplicatePlans(plans: Plan[]): Plan[] {
    let uniquePlans = [];
    let seen = new HashSet();
    const _seq1 = plans;
    let _at2 = 0;
    try {
      while (_at2 < _seq1.length) {
        const plan = _seq1[_at2++];
        let _moved0 = false;
        try {
          plan.match({
            Index: (v) => {
              const indexSpec = v.indexSpec;
              const scanDirection = v.scanDirection;
              const key = [indexSpec.keyparts.map((e) => e.clone()), scanDirection];
              if (seen.insert(key)) {
                _moved0 = true;
                uniquePlans.push(plan);
              }
            },
            EmptyScan: () => {
              _moved0 = true;
              uniquePlans.push(plan);
            },
            TableScan: () => {
              _moved0 = true;
              uniquePlans.push(plan);
            },
          });
        } finally {
          if (!_moved0) plan.drop();
        }
      }
    } finally {
      dropOwned(_seq1.slice(_at2));
    }
    return uniquePlans;
  }

  buildTableScanPlan(conjuncts: Predicate[], primaryKey: string, orderBy: OrderByItem[] | null): Plan {
    const bounds = this.extractEntityIdRange(conjuncts, primaryKey);
    const remainingPredicate = [...conjuncts].fold(new Predicate('True', {}), (acc, pred) => {
      if (acc.is('True')) {
        return pred.clone();
      } else {
        return new Predicate('And', { _0: acc, _1: pred.clone() });
      }
    });
    const [scanDirection, orderBySpill] = (() => {
      {
        const _v1 = orderBy;
        if (_v1 != null) {
          const orderItems = _v1;
          {
            const _v = iterFirst(orderItems);
            if (_v != null) {
              const firstItem = _v;
              if (firstItem.path.isSimple() && firstItem.path.first() === primaryKey) {
                const direction = firstItem.direction.match({
                  Asc: () => new ScanDirection('Forward', {}),
                  Desc: () => new ScanDirection('Reverse', {}),
                });
                const presort = [firstItem.clone()];
                const spill = orderItems.slice(1).map((e) => e.clone());
                return [direction, OrderByComponents.new(presort, spill)];
              } else {
                return [new ScanDirection('Forward', {}), OrderByComponents.new([], orderItems.map((e) => e.clone()))];
              }
            } else {
            return [new ScanDirection('Forward', {}), OrderByComponents.default()];
          }
          }
        } else {
        return [new ScanDirection('Forward', {}), OrderByComponents.default()];
      }
      }
    })();
    return new Plan('TableScan', { bounds: bounds, scanDirection: scanDirection, remainingPredicate: remainingPredicate, orderBySpill: orderBySpill });
  }

  extractEntityIdRange(conjuncts: Predicate[], primaryKey: string): KeyBounds {
    let primaryKeyBounds = [];
    for (const predicate of conjuncts) {
      {
        const _v = this.extractPrimaryKeyBound(predicate, primaryKey);
        if (_v != null) {
          const bound = _v;
          primaryKeyBounds.push(bound);
        }
      }
    }
    if (primaryKeyBounds.length === 0) {
      return KeyBounds.empty();
    }
    if (primaryKeyBounds.length === 1) {
      return new KeyBounds(primaryKeyBounds);
    } else {
      const intersectedBound = this.intersectPrimaryKeyBounds(primaryKeyBounds, primaryKey);
      return new KeyBounds([intersectedBound]);
    }
  }

  extractPrimaryKeyBound(predicate: Predicate, primaryKey: string): KeyBoundComponent | null {
    {
      const _v1 = predicate;
      if (_v1.is('Comparison')) {
        const { left, operator, right } = _v1.value;
        const _m0 = (() => {
          const _v = [left.asRef(), right.asRef()];
          if ((_v[0].is('Path')) && (_v[1].is('Literal'))) {
            const { _0: path } = _v[0].value;
            const { _0: literal } = _v[1].value;
            if (path.isSimple() && path.first() === primaryKey) {
              return Value.fromRefAstLiteral(literal);
            }
          }
          if ((_v[0].is('Literal')) && (_v[1].is('Path'))) {
            const { _0: literal } = _v[0].value;
            const { _0: path } = _v[1].value;
            if (path.isSimple() && path.first() === primaryKey) {
              return Value.fromRefAstLiteral(literal);
            }
          }
          {
            return { $jump: 'return', $value: null };
          }
        })();
        if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
        const value = (_m0 as any);
        const _m1 = (() => {
          return operator.match<any>({
            Equal: () => [new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true }), new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value }), inclusive: true })] as any,
            GreaterThan: () => [new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: false }), new Endpoint('UnboundedHigh', { _0: ValueType.of(value) })] as any,
            GreaterThanOrEqual: () => [new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true }), new Endpoint('UnboundedHigh', { _0: ValueType.of(value) })] as any,
            LessThan: () => [new Endpoint('UnboundedLow', { _0: ValueType.of(value) }), new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: false })] as any,
            LessThanOrEqual: () => [new Endpoint('UnboundedLow', { _0: ValueType.of(value) }), new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true })] as any,
            NotEqual: () => {
              return { $jump: 'return', $value: null };
            },
            In: () => {
              return { $jump: 'return', $value: null };
            },
            Between: () => {
              return { $jump: 'return', $value: null };
            },
          });
        })();
        if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
        const [low, high] = (_m1 as any);
        return new KeyBoundComponent(primaryKey, low, high);
      } else {
      return null;
    }
    }
  }

  intersectPrimaryKeyBounds(bounds: KeyBoundComponent[], primaryKey: string): KeyBoundComponent {
    let resultLow = new Endpoint('UnboundedLow', { _0: new ValueType('String', {}) });
    let resultHigh = new Endpoint('UnboundedHigh', { _0: new ValueType('String', {}) });
    const _seq2 = bounds;
    let _at3 = 0;
    try {
      while (_at3 < _seq2.length) {
        const bound = _seq2[_at3++];
        try {
          const _a0 = this.intersectLowerBounds(resultLow, bound.low);
          resultLow.drop();
          resultLow = _a0;
          const _a1 = this.intersectUpperBounds(resultHigh, bound.high);
          resultHigh.drop();
          resultHigh = _a1;
        } finally {
          bound.drop();
        }
      }
    } finally {
      dropOwned(_seq2.slice(_at3));
    }
    return new KeyBoundComponent(primaryKey, resultLow, resultHigh);
  }

  intersectLowerBounds(left: Endpoint, right: Endpoint): Endpoint {
    const _v = [left, right];
    if (((_v[0].is('UnboundedLow'))) || ((_v[1].is('UnboundedLow')))) {
      const other = (((_v[0].is('UnboundedLow')))) ? _v[1] : (((_v[1].is('UnboundedLow')))) ? _v[0] : undefined;
      return other.clone();
    } else if ((_v[0].is('Value') && (_v[0].value.datum.is('Val'))) && (_v[1].is('Value') && (_v[1].value.datum.is('Val')))) {
      const { inclusive: incA } = _v[0].value;
      const { _0: a } = _v[0].value.datum.value;
      const { inclusive: incB } = _v[1].value;
      const { _0: b } = _v[1].value.datum.value;
      const _v1 = a.partialCompareTo(b);
      if (_v1 != null && (_v1 === 1)) {
        return left.clone();
      } else if (_v1 != null && (_v1 === -1)) {
        return right.clone();
      } else if (_v1 != null && (_v1 === 0)) {
        return new Endpoint('Value', { datum: new KeyDatum('Val', { _0: a.clone() }), inclusive: incA && incB });
      } else {
        return left.clone();
      }
    } else {
      return left.clone();
    }
  }

  intersectUpperBounds(left: Endpoint, right: Endpoint): Endpoint {
    const _v = [left, right];
    if (((_v[0].is('UnboundedHigh'))) || ((_v[1].is('UnboundedHigh')))) {
      const other = (((_v[0].is('UnboundedHigh')))) ? _v[1] : (((_v[1].is('UnboundedHigh')))) ? _v[0] : undefined;
      return other.clone();
    } else if ((_v[0].is('Value') && (_v[0].value.datum.is('Val'))) && (_v[1].is('Value') && (_v[1].value.datum.is('Val')))) {
      const { inclusive: incA } = _v[0].value;
      const { _0: a } = _v[0].value.datum.value;
      const { inclusive: incB } = _v[1].value;
      const { _0: b } = _v[1].value.datum.value;
      const _v1 = a.partialCompareTo(b);
      if (_v1 != null && (_v1 === -1)) {
        return left.clone();
      } else if (_v1 != null && (_v1 === 1)) {
        return right.clone();
      } else if (_v1 != null && (_v1 === 0)) {
        return new Endpoint('Value', { datum: new KeyDatum('Val', { _0: a.clone() }), inclusive: incA && incB });
      } else {
        return left.clone();
      }
    } else {
      return left.clone();
    }
  }

  isPrimaryKeyPredicate(predicate: Predicate, primaryKey: string): boolean {
    {
      const _v1 = predicate;
      if (_v1.is('Comparison')) {
        const { left } = _v1.value;
        const _v = left.asRef();
        if (_v.is('Path')) {
          const { _0: path } = _v.value;
          if (path.isSimple()) {
            return path.first() === primaryKey;
          }
        }
        {
          return false;
        }
      } else {
      return false;
    }
    }
  }

  hasPrimaryKeyOrderBy(orderBy: OrderByItem[] | null, primaryKey: string): boolean {
    {
      const _v = orderBy;
      if (_v != null) {
        const orderItems = _v;
        {
          const _v1 = iterFirst(orderItems);
          if (_v1 != null) {
            const firstItem = _v1;
            if (firstItem.path.isSimple()) {
              return firstItem.path.first() === primaryKey;
            }  }
        }  }
    }
    return false;
  }

  hasPrimaryKeyRangePredicates(conjuncts: Predicate[], primaryKey: string): boolean {
    return [...conjuncts].some((predicate) => {
      {
        const _v2 = predicate;
        if (_v2.is('Comparison')) {
          const { left, operator } = _v2.value;
          const isPrimaryKeyField = (() => {
            const _v1 = left.asRef();
            if (_v1.is('Path')) {
              const { _0: path } = _v1.value;
              if (path.isSimple()) {
                return path.first() === primaryKey;
              }
            }
            {
              return false;
            }
          })();
          if (isPrimaryKeyField) {
            return (operator.is('Equal')) || (operator.is('GreaterThan')) || (operator.is('GreaterThanOrEqual')) || (operator.is('LessThan')) || (operator.is('LessThanOrEqual'));
          } else {
            return false;
          }
        } else {
        return false;
      }
      }
    });
  }
}

