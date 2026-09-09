// MIRRORS: ankurah/storage/common/src/planner.rs
import { Struct, dropOwned, unsupported, iterFind, iterFindMap, iterFirst, iterFirstOwned, skipOwned, HashSet } from '@ankurah/base';
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
      let _moved0 = false;
      let plans = [];
      try {
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
              _moved0 = true;
              let _moved1 = false;
              const deduplicatedPlans = this.deduplicatePlans(plans);
              try {
                const hasEmptyScan = [...deduplicatedPlans].some((plan) => plan.is('EmptyScan'));
                if (!hasEmptyScan) {
                  _moved1 = true;
                  let _moved2 = false;
                  let finalPlans = deduplicatedPlans;
                  try {
                    const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
                    finalPlans.push(tableScan);
                    _moved2 = true;
                    return finalPlans;
                  } finally {
                    if (!_moved2) dropOwned(finalPlans);
                  }
                } else {
                  _moved1 = true;
                  return deduplicatedPlans;
                }
              } finally {
                if (!_moved1) dropOwned(deduplicatedPlans);
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
        _moved0 = true;
        let _moved3 = false;
        const deduplicatedPlans = this.deduplicatePlans(plans);
        try {
          const hasEmptyScan = [...deduplicatedPlans].some((plan) => plan.is('EmptyScan'));
          if (!hasEmptyScan) {
            _moved3 = true;
            let _moved4 = false;
            let finalPlans = deduplicatedPlans;
            try {
              const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
              finalPlans.push(tableScan);
              _moved4 = true;
              return finalPlans;
            } finally {
              if (!_moved4) dropOwned(finalPlans);
            }
          } else {
            _moved3 = true;
            return deduplicatedPlans;
          }
        } finally {
          if (!_moved3) dropOwned(deduplicatedPlans);
        }
      } finally {
        if (!_moved0) dropOwned(plans);
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
          let _moved8 = false;
          const orderBy_1 = (() => {
            if (!this.config.supportsDescIndexes) {
              const firstDir = orderBy[0].direction.clone();
              try {
                let _moved6 = false;
                let presort = [];
                try {
                  let _moved7 = false;
                  let spill = [];
                  try {
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
                    _moved6 = true;
                    _moved7 = true;
                    return OrderByComponents.new(presort, spill);
                  } finally {
                    if (!_moved7) dropOwned(spill);
                  }
                } finally {
                  if (!_moved6) dropOwned(presort);
                }
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
            _moved8 = true;
            _moved0 = true;
            return new Plan('Index', { indexSpec: KeySpec.new(indexKeyparts), scanDirection: scanDirection, bounds: bounds, remainingPredicate: remainingPredicate, orderBySpill: orderBy_1 });
          } finally {
            if (!_moved8) orderBy_1.drop();
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
    if (_r1 == null) return null;
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
          covered.add(primary._0);
          let _moved6 = false;
          let presort = [];
          try {
            let _moved7 = false;
            let spill = [];
            try {
              for (const item of orderBy) {
                if (item.path.isSimple()) {
                  const name = item.path.first();
                  if (covered.has(name)) {
                    presort.push(item.clone());
                  } else {
                    spill.push(item.clone());
                  }
                }
              }
              _moved6 = true;
              _moved7 = true;
              let _moved8 = false;
              const orderBy_1 = OrderByComponents.new(presort, spill);
              try {
                _moved4 = true;
                _moved5 = true;
                _moved8 = true;
                _moved2 = true;
                return new Plan('Index', { indexSpec: KeySpec.new(indexKeyparts), scanDirection: scanDirection, bounds: bounds, remainingPredicate: remainingPredicate, orderBySpill: orderBy_1 });
              } finally {
                if (!_moved8) orderBy_1.drop();
              }
            } finally {
              if (!_moved7) dropOwned(spill);
            }
          } finally {
            if (!_moved6) dropOwned(presort);
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
    let _moved0 = false;
    let equalities: [string, Value][] = [];
    try {
      let _moved1 = false;
      let inequalities = IndexMap.new();
      try {
        for (const conjunct of conjuncts) {
          {
            const _v = this.extractComparison(conjunct);
            if (_v != null) {
              const [field, op, value] = _v;
              let _moved2 = false;
              let _moved3 = false;
              try {
                try {
                  if (field === primaryKey) {
                    continue;
                  }
                  return op.match({
                    Equal: () => {
                      _moved3 = true;
                      equalities.push([field, value]);
                    },
                    GreaterThan: () => {
                      _moved2 = true;
                      _moved3 = true;
                      inequalities.entry(field).orDefault().push([op, value]);
                    },
                    GreaterThanOrEqual: () => {
                      _moved2 = true;
                      _moved3 = true;
                      inequalities.entry(field).orDefault().push([op, value]);
                    },
                    LessThan: () => {
                      _moved2 = true;
                      _moved3 = true;
                      inequalities.entry(field).orDefault().push([op, value]);
                    },
                    LessThanOrEqual: () => {
                      _moved2 = true;
                      _moved3 = true;
                      inequalities.entry(field).orDefault().push([op, value]);
                    },
                    NotEqual: () => {},
                    In: () => {},
                    Between: () => {},
                  });
                } finally {
                  if (!_moved3) value.drop();
                }
              } finally {
                if (!_moved2) op.drop();
              }
            }
          }
        }
        _moved0 = true;
        _moved1 = true;
        return [equalities, inequalities];
      } finally {
        if (!_moved1) dropOwned(inequalities);
      }
    } finally {
      if (!_moved0) dropOwned(equalities);
    }
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
    let _moved0 = false;
    let indexKeyparts = [];
    try {
      for (const [field, value] of equalities) {
        indexKeyparts.push(IndexKeyPart.ascPath(field, ValueType.of(value)));
      }
      const _r1 = inequalities.get(inequalityField);
      if (_r1 == null) return null;
      const inequalityValues = _r1;
      let _moved2 = false;
      const firstInequalityValue = inequalityValues[0][1];
      try {
        _moved2 = true;
        indexKeyparts.push(IndexKeyPart.ascPath(inequalityField, ValueType.of(firstInequalityValue)));
        let _moved3 = false;
        const bounds = this.buildBounds(equalities, [inequalityField, inequalityValues], indexKeyparts);
        try {
          const _m5 = (() => {
            const _v = bounds;
            if (_v != null) {
              const bounds = _v;
              let _moved4 = false;
              try {
                {
                  if (this.isEmptyBounds(bounds)) {
                    return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
                  }
                  _moved4 = true;
                  return bounds;
                }
              } finally {
                if (!_moved4) bounds.drop();
              }
            } else {
              return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
            }
          })();
          if ((_m5 as any)?.$jump === 'return') return (_m5 as any).$value;
          _moved3 = true;
          let _moved6 = false;
          const bounds_1 = (_m5 as any);
          try {
            let _moved7 = false;
            const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, inequalityField);
            try {
              let _moved10 = false;
              const orderBySpill = (() => {
                {
                  const _v1 = orderBy;
                  if (_v1 != null) {
                    const orderByItems = _v1;
                    const coveredFields = HashSet.from([...[...equalities].map(([f, ]) => f), ...once(inequalityField)]);
                    let _moved8 = false;
                    let presort = [];
                    try {
                      let _moved9 = false;
                      let spill = [];
                      try {
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
                        _moved8 = true;
                        _moved9 = true;
                        return OrderByComponents.new(presort, spill);
                      } finally {
                        if (!_moved9) dropOwned(spill);
                      }
                    } finally {
                      if (!_moved8) dropOwned(presort);
                    }
                  } else {
                  return OrderByComponents.default();
                }
                }
              })();
              try {
                _moved0 = true;
                let _moved11 = false;
                const indexSpec = KeySpec.new(indexKeyparts);
                try {
                  _moved11 = true;
                  _moved6 = true;
                  _moved7 = true;
                  _moved10 = true;
                  return new Plan('Index', { indexSpec: indexSpec, scanDirection: new ScanDirection('Forward', {}), bounds: bounds_1, remainingPredicate: remainingPredicate, orderBySpill: orderBySpill });
                } finally {
                  if (!_moved11) indexSpec.drop();
                }
              } finally {
                if (!_moved10) orderBySpill.drop();
              }
            } finally {
              if (!_moved7) remainingPredicate.drop();
            }
          } finally {
            if (!_moved6) dropOwned(bounds_1);
          }
        } finally {
          if (!_moved3) dropOwned(bounds);
        }
      } finally {
        if (!_moved2) firstInequalityValue.drop();
      }
    } finally {
      if (!_moved0) dropOwned(indexKeyparts);
    }
  }

  generateEqualityPlan(equalities: [string, Value][], conjuncts: Predicate[]): Plan | null {
    let _moved0 = false;
    let indexKeyparts = [];
    try {
      for (const [field, value] of equalities) {
        indexKeyparts.push(IndexKeyPart.ascPath(field, ValueType.of(value)));
      }
      const bounds = this.buildBounds(equalities, null, indexKeyparts);
      const _m2 = (() => {
        const _v = bounds;
        if (_v != null) {
          const bounds = _v;
          let _moved1 = false;
          try {
            {
              if (this.isEmptyBounds(bounds)) {
                return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
              }
              _moved1 = true;
              return bounds;
            }
          } finally {
            if (!_moved1) bounds.drop();
          }
        } else {
          return { $jump: 'return', $value: new Plan('EmptyScan', {}) };
        }
      })();
      if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
      let _moved3 = false;
      const bounds_1 = (_m2 as any);
      try {
        let _moved4 = false;
        const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, null);
        try {
          _moved0 = true;
          let _moved5 = false;
          const indexSpec = KeySpec.new(indexKeyparts);
          try {
            _moved5 = true;
            _moved3 = true;
            _moved4 = true;
            return new Plan('Index', { indexSpec: indexSpec, scanDirection: new ScanDirection('Forward', {}), bounds: bounds_1, remainingPredicate: remainingPredicate, orderBySpill: OrderByComponents.default() });
          } finally {
            if (!_moved5) indexSpec.drop();
          }
        } finally {
          if (!_moved4) remainingPredicate.drop();
        }
      } finally {
        if (!_moved3) dropOwned(bounds_1);
      }
    } finally {
      if (!_moved0) dropOwned(indexKeyparts);
    }
  }

  buildBounds(equalities: [string, Value][], inequality: [string, [ComparisonOperator, Value][]] | null, indexKeyparts: IndexKeyPart[]): KeyBounds | null {
    let _moved0 = false;
    let keypartBounds = [];
    try {
      for (const keypart of indexKeyparts) {
        const fullPath = keypart.fullPath();
        const _m1 = iterFind([...equalities], ([field, ]) => field === fullPath);
        const equalityValue = (_m1 != null ? (([, value]) => value)(_m1!) : null);
        {
          const _v1 = equalityValue;
          if (_v1 != null) {
            const value = _v1;
            let _moved3 = false;
            const _b2 = Endpoint.incl(value.clone());
            try {
              const _b4 = Endpoint.incl(value.clone());
              _moved3 = true;
              keypartBounds.push(new KeyBoundComponent(fullPath, _b2, _b4));
            } finally {
              if (!_moved3) dropOwned(_b2);
            }
          } else {
          const _v = inequality;
          if (_v != null) {
            const [ineqField, inequalities] = _v;
            if (ineqField === fullPath) {
              let _moved5 = false;
              let low = new Endpoint('UnboundedLow', { _0: ValueType.of(inequalities[0][1]) });
              try {
                let _moved6 = false;
                let high = new Endpoint('UnboundedHigh', { _0: ValueType.of(inequalities[0][1]) });
                try {
                  for (const [op, value] of inequalities) {
                    op.match({
                      GreaterThan: () => {
                        let _moved7 = false;
                        const candidate = Endpoint.excl(value.clone());
                        try {
                          if (this.isMoreRestrictiveLower(candidate, low)) {
                            const _a8 = candidate;
                            if (!_moved5) low.drop();
                            _moved5 = false;
                            _moved7 = true;
                            low = _a8;
                          }
                        } finally {
                          if (!_moved7) candidate.drop();
                        }
                      },
                      GreaterThanOrEqual: () => {
                        let _moved9 = false;
                        const candidate = Endpoint.incl(value.clone());
                        try {
                          if (this.isMoreRestrictiveLower(candidate, low)) {
                            const _a10 = candidate;
                            if (!_moved5) low.drop();
                            _moved5 = false;
                            _moved9 = true;
                            low = _a10;
                          }
                        } finally {
                          if (!_moved9) candidate.drop();
                        }
                      },
                      LessThan: () => {
                        let _moved11 = false;
                        const candidate = Endpoint.excl(value.clone());
                        try {
                          if (this.isMoreRestrictiveUpper(candidate, high)) {
                            const _a12 = candidate;
                            if (!_moved6) high.drop();
                            _moved6 = false;
                            _moved11 = true;
                            high = _a12;
                          }
                        } finally {
                          if (!_moved11) candidate.drop();
                        }
                      },
                      LessThanOrEqual: () => {
                        let _moved13 = false;
                        const candidate = Endpoint.incl(value.clone());
                        try {
                          if (this.isMoreRestrictiveUpper(candidate, high)) {
                            const _a14 = candidate;
                            if (!_moved6) high.drop();
                            _moved6 = false;
                            _moved13 = true;
                            high = _a14;
                          }
                        } finally {
                          if (!_moved13) candidate.drop();
                        }
                      },
                      Equal: () => {},
                      NotEqual: () => {},
                      In: () => {},
                      Between: () => {},
                    });
                  }
                  _moved5 = true;
                  _moved6 = true;
                  keypartBounds.push(new KeyBoundComponent(fullPath, low, high));
                  break;
                } finally {
                  if (!_moved6) high.drop();
                }
              } finally {
                if (!_moved5) low.drop();
              }
            } else {
              break;
            }
          } else {
          break;
        }
        }
        }
      }
      _moved0 = true;
      return KeyBounds.new(keypartBounds);
    } finally {
      if (!_moved0) dropOwned(keypartBounds);
    }
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
    let _moved0 = false;
    let remainingConjuncts = [];
    try {
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
        _moved0 = true;
        return (iterFirstOwned([...remainingConjuncts]) ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
      } else {
        let _moved1 = false;
        let result = remainingConjuncts[0].clone();
        try {
          _moved0 = true;
          for (const conjunct of skipOwned([...remainingConjuncts], 1)) {
            const _a2 = new Predicate('And', { _0: result, _1: conjunct });
            if (!_moved1) result.drop();
            _moved1 = false;
            _moved1 = true;
            result = _a2;
          }
          _moved1 = true;
          return result;
        } finally {
          if (!_moved1) result.drop();
        }
      }
    } finally {
      if (!_moved0) dropOwned(remainingConjuncts);
    }
  }

  deduplicatePlans(plans: Plan[]): Plan[] {
    let _moved0 = false;
    try {
      let _moved1 = false;
      let uniquePlans = [];
      try {
        let seen = new HashSet();
        try {
          _moved0 = true;
          const _seq3 = plans;
          let _at4 = 0;
          try {
            while (_at4 < _seq3.length) {
              const plan = _seq3[_at4++];
              let _moved2 = false;
              try {
                plan.match({
                  Index: (v) => {
                    const indexSpec = v.indexSpec;
                    const scanDirection = v.scanDirection;
                    const key = [indexSpec.keyparts.map((e) => e.clone()), scanDirection];
                    if (seen.insert(key)) {
                      _moved2 = true;
                      uniquePlans.push(plan);
                    }
                  },
                  EmptyScan: () => {
                    _moved2 = true;
                    uniquePlans.push(plan);
                  },
                  TableScan: () => {
                    _moved2 = true;
                    uniquePlans.push(plan);
                  },
                });
              } finally {
                if (!_moved2) plan.drop();
              }
            }
          } finally {
            dropOwned(_seq3.slice(_at4));
          }
          _moved1 = true;
          return uniquePlans;
        } finally {
          dropOwned(seen);
        }
      } finally {
        if (!_moved1) dropOwned(uniquePlans);
      }
    } finally {
      if (!_moved0) dropOwned(plans);
    }
  }

  buildTableScanPlan(conjuncts: Predicate[], primaryKey: string, orderBy: OrderByItem[] | null): Plan {
    let _moved0 = false;
    const bounds = this.extractEntityIdRange(conjuncts, primaryKey);
    try {
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
                  let _moved1 = false;
                  const presort = [firstItem.clone()];
                  try {
                    const spill = orderItems.slice(1).map((e) => e.clone());
                    _moved1 = true;
                    return [direction, OrderByComponents.new(presort, spill)];
                  } finally {
                    if (!_moved1) dropOwned(presort);
                  }
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
      _moved0 = true;
      return new Plan('TableScan', { bounds: bounds, scanDirection: scanDirection, remainingPredicate: remainingPredicate, orderBySpill: orderBySpill });
    } finally {
      if (!_moved0) bounds.drop();
    }
  }

  extractEntityIdRange(conjuncts: Predicate[], primaryKey: string): KeyBounds {
    let _moved0 = false;
    let primaryKeyBounds = [];
    try {
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
        _moved0 = true;
        return new KeyBounds(primaryKeyBounds);
      } else {
        _moved0 = true;
        const intersectedBound = this.intersectPrimaryKeyBounds(primaryKeyBounds, primaryKey);
        return new KeyBounds([intersectedBound]);
      }
    } finally {
      if (!_moved0) dropOwned(primaryKeyBounds);
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
        const _m16 = (() => {
          return operator.match<any>({
            Equal: () => {
              let _moved2 = false;
              const _b1 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true });
              try {
                const _b3 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value }), inclusive: true });
                _moved2 = true;
                return [_b1, _b3];
              } finally {
                if (!_moved2) dropOwned(_b1);
              }
            },
            GreaterThan: () => {
              let _moved5 = false;
              const _b4 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: false });
              try {
                const _b6 = new Endpoint('UnboundedHigh', { _0: ValueType.of(value) });
                _moved5 = true;
                return [_b4, _b6];
              } finally {
                if (!_moved5) dropOwned(_b4);
              }
            },
            GreaterThanOrEqual: () => {
              let _moved8 = false;
              const _b7 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true });
              try {
                const _b9 = new Endpoint('UnboundedHigh', { _0: ValueType.of(value) });
                _moved8 = true;
                return [_b7, _b9];
              } finally {
                if (!_moved8) dropOwned(_b7);
              }
            },
            LessThan: () => {
              let _moved11 = false;
              const _b10 = new Endpoint('UnboundedLow', { _0: ValueType.of(value) });
              try {
                const _b12 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: false });
                _moved11 = true;
                return [_b10, _b12];
              } finally {
                if (!_moved11) dropOwned(_b10);
              }
            },
            LessThanOrEqual: () => {
              let _moved14 = false;
              const _b13 = new Endpoint('UnboundedLow', { _0: ValueType.of(value) });
              try {
                const _b15 = new Endpoint('Value', { datum: new KeyDatum('Val', { _0: value.clone() }), inclusive: true });
                _moved14 = true;
                return [_b13, _b15];
              } finally {
                if (!_moved14) dropOwned(_b13);
              }
            },
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
        if ((_m16 as any)?.$jump === 'return') return (_m16 as any).$value;
        const [low, high] = (_m16 as any);
        return new KeyBoundComponent(primaryKey, low, high);
      } else {
      return null;
    }
    }
  }

  intersectPrimaryKeyBounds(bounds: KeyBoundComponent[], primaryKey: string): KeyBoundComponent {
    let _moved0 = false;
    try {
      let _moved1 = false;
      let resultLow = new Endpoint('UnboundedLow', { _0: new ValueType('String', {}) });
      try {
        let _moved2 = false;
        let resultHigh = new Endpoint('UnboundedHigh', { _0: new ValueType('String', {}) });
        try {
          _moved0 = true;
          const _seq5 = bounds;
          let _at6 = 0;
          try {
            while (_at6 < _seq5.length) {
              const bound = _seq5[_at6++];
              try {
                const _a3 = this.intersectLowerBounds(resultLow, bound.low);
                if (!_moved1) resultLow.drop();
                _moved1 = false;
                resultLow = _a3;
                const _a4 = this.intersectUpperBounds(resultHigh, bound.high);
                if (!_moved2) resultHigh.drop();
                _moved2 = false;
                resultHigh = _a4;
              } finally {
                bound.drop();
              }
            }
          } finally {
            dropOwned(_seq5.slice(_at6));
          }
          _moved1 = true;
          _moved2 = true;
          return new KeyBoundComponent(primaryKey, resultLow, resultHigh);
        } finally {
          if (!_moved2) resultHigh.drop();
        }
      } finally {
        if (!_moved1) resultLow.drop();
      }
    } finally {
      if (!_moved0) dropOwned(bounds);
    }
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

