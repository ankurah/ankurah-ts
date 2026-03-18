// MIRRORS: ankurah/storage/common/src/planner.rs

import { Predicate } from '@ankurah/ankql';
import type { ComparisonOperator, OrderByItem, Selection } from '@ankurah/ankql';
import type { IndexKeyPart, KeySpec, Value } from '@ankurah/core';
import { indexKeyPartAscPath, indexKeyPartDescPath, keySpecNew, valueFromLiteral, valuePartialCmp, valueType, ValueType } from '@ankurah/core';
import { ConjunctFinder } from './predicate.ts';
import {
  Plan,
  ScanDirection,
  KeyBoundComponent,
  Endpoint,
  KeyDatum,
  endpointIncl,
  endpointExcl,
  keyBoundsNew,
  keyBoundsEmpty,
  orderByComponentsNew,
  orderByComponentsDefault,
} from './types.ts';
import type {
  KeyBounds,
  OrderByComponents,
} from './types.ts';

// ── PlannerConfig ────────────────────────────────────────────────────

/**
 * Rust: `pub struct PlannerConfig { supports_desc_indexes: bool }`
 */
export interface PlannerConfig {
  /** Whether the storage backend supports descending indexes. false for IndexedDB, true for engines with real DESC indexes. */
  supportsDescIndexes: boolean;
}

/** IndexedDB configuration. Rust: `PlannerConfig::indexeddb()` */
export function plannerConfigIndexeddb(): PlannerConfig {
  return { supportsDescIndexes: false };
}

/** Generic storage with full index support. Rust: `PlannerConfig::full_support()` */
export function plannerConfigFullSupport(): PlannerConfig {
  return { supportsDescIndexes: true };
}

// ── Planner ──────────────────────────────────────────────────────────

/**
 * Query planner that generates execution plans.
 *
 * Rust: `pub struct Planner { config: PlannerConfig }`
 */
export class Planner {
  private config: PlannerConfig;

  constructor(config: PlannerConfig) {
    this.config = config;
  }

  /**
   * Generate all possible plans for a query.
   *
   * Input: Selection with predicate, primary key field name
   * Output: Vector of all viable plans (index plans + table scan fallback)
   *
   * Rust: `pub fn plan(&self, selection: &Selection, primary_key: &str) -> Vec<Plan>`
   */
  plan(selection: Selection, primaryKey: string): Plan[] {
    const conjuncts = ConjunctFinder.find(selection.predicate);

    // Separate conjuncts into equalities and inequalities, filtering out primary key predicates
    const [equalities, inequalities] = this.categorizeConjunctsExcludingPrimaryKey(conjuncts, primaryKey);

    // Check if we should skip index generation for primary key-only queries
    const hasPrimaryKeyRanges = this.hasPrimaryKeyRangePredicates(conjuncts, primaryKey);
    const hasPrimaryKeyOrderBy = this.hasPrimaryKeyOrderBy(selection.orderBy, primaryKey);
    const hasNonPrimaryPredicates = conjuncts.some(
      (pred) => !pred.is('True') && !this.isPrimaryKeyPredicate(pred, primaryKey),
    );

    // If we have primary key predicates/ORDER BY but NO other meaningful predicates, skip index generation
    if ((hasPrimaryKeyRanges || hasPrimaryKeyOrderBy) && !hasNonPrimaryPredicates) {
      const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
      return [tableScan];
    }

    let plans: Plan[] = [];

    // New ORDER BY strategies
    if (selection.orderBy !== null && selection.orderBy.length > 0) {
      const orderBy = selection.orderBy;

      const orderFirstPlan = this.buildOrderFirstPlan(equalities, inequalities, orderBy, conjuncts);
      if (orderFirstPlan !== null) {
        plans.push(orderFirstPlan);
      }

      // If an ORDER BY field has inequalities (covered inequality), do NOT emit INEQ-FIRST
      const coveredIneq = orderBy.some(
        (item) => item.path.isSimple() && inequalities.has(item.path.first()),
      );
      if (!coveredIneq && inequalities.size > 0) {
        const ineqFirstPlan = this.buildIneqFirstPlan(equalities, inequalities, orderBy, conjuncts);
        if (ineqFirstPlan !== null) {
          plans.push(ineqFirstPlan);
        }
      }

      // Apply the same TableScan fallback logic as the main path
      const deduplicatedPlans = this.deduplicatePlans(plans);
      const hasEmptyScan = deduplicatedPlans.some((plan) => plan.is('EmptyScan'));
      if (!hasEmptyScan) {
        const finalPlans = [...deduplicatedPlans];
        const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
        finalPlans.push(tableScan);
        return finalPlans;
      } else {
        return deduplicatedPlans;
      }
    }

    // If we have inequalities, generate plans for each inequality field
    if (inequalities.size > 0) {
      for (const [field] of inequalities) {
        const plan = this.generateInequalityPlanWithOrderBy(
          equalities,
          field,
          inequalities,
          conjuncts,
          selection.orderBy,
        );
        if (plan !== null) {
          plans.push(plan);
        }
      }
    } else if (equalities.length > 0) {
      // Generate equality-only plan if we have equalities but no inequalities
      const plan = this.generateEqualityPlan(equalities, conjuncts);
      if (plan !== null) {
        plans.push(plan);
      }
    }

    // Deduplicate plans based on index_fields and scan_direction
    const deduplicatedPlans = this.deduplicatePlans(plans);

    // Add table scan as fallback ONLY if there's no EmptyScan
    const hasEmptyScan = deduplicatedPlans.some((plan) => plan.is('EmptyScan'));
    if (!hasEmptyScan) {
      const finalPlans = [...deduplicatedPlans];
      const tableScan = this.buildTableScanPlan(conjuncts, primaryKey, selection.orderBy);
      finalPlans.push(tableScan);
      return finalPlans;
    } else {
      return deduplicatedPlans;
    }
  }

  // ── ORDER-FIRST strategy ──────────────────────────────────────────

  /**
   * ORDER-FIRST: [EQ ...] + maximal OB prefix (capability-aware). Bounds: EQ only.
   *
   * Rust: `fn build_order_first_plan(...)`
   */
  private buildOrderFirstPlan(
    equalities: [string, Value][],
    inequalities: Map<string, [ComparisonOperator, Value][]>,
    orderBy: OrderByItem[],
    conjuncts: Predicate[],
  ): Plan | null {
    if (orderBy.length === 0) return null;

    // Keyparts: EQ prefix
    const indexKeyparts: IndexKeyPart[] = equalities.map(([f, v]) =>
      indexKeyPartAscPath(f, valueType(v)),
    );

    // Append ORDER BY fields per capability
    if (this.config.supportsDescIndexes) {
      for (const item of orderBy) {
        if (item.path.isSimple()) {
          const name = item.path.first();
          indexKeyparts.push(
            item.direction.is('Asc')
              ? indexKeyPartAscPath(name, ValueType.String)
              : indexKeyPartDescPath(name, ValueType.String),
          );
        }
      }
    } else {
      // IndexedDB: ASC-only index parts, keep longest same-direction prefix
      const firstDir = orderBy[0].direction;
      let broke = false;
      for (const item of orderBy) {
        if (item.path.isSimple()) {
          const name = item.path.first();
          if (!broke && item.direction.type === firstDir.type) {
            indexKeyparts.push(indexKeyPartAscPath(name, ValueType.String));
          } else {
            broke = true;
          }
        }
      }
    }

    // Bounds: equalities + (optional) bounds on the first ORDER BY field that has inequalities
    let appliedIneq: [string, [ComparisonOperator, Value][]] | null = null;
    for (const item of orderBy) {
      if (item.path.isSimple()) {
        const name = item.path.first();
        const ineqVec = inequalities.get(name);
        if (ineqVec !== undefined) {
          appliedIneq = [name, ineqVec];
          break;
        }
      }
    }

    const bounds = appliedIneq !== null
      ? this.buildBounds(equalities, [appliedIneq[0], appliedIneq[1]], indexKeyparts)
      : this.buildBounds(equalities, null, indexKeyparts);
    if (bounds === null) return Plan.EmptyScan();
    if (this.isEmptyBounds(bounds)) return Plan.EmptyScan();

    // Remaining predicate excludes the applied OB inequality if any
    const remainingPredicate = this.calculateRemainingPredicate(
      conjuncts,
      equalities,
      appliedIneq !== null ? appliedIneq[0] : null,
    );

    // Scan direction
    let scanDirection: ScanDirection;
    if (this.config.supportsDescIndexes) {
      scanDirection = ScanDirection.Forward();
    } else {
      scanDirection = orderBy[0].direction.is('Desc') ? ScanDirection.Reverse() : ScanDirection.Forward();
    }

    // Build OrderByComponents: presort (satisfied by index) and spill (needs in-memory sort)
    let orderByComponents: OrderByComponents;
    if (!this.config.supportsDescIndexes) {
      const firstDir = orderBy[0].direction;
      const presort: OrderByItem[] = [];
      const spill: OrderByItem[] = [];
      let broke = false;
      for (const item of orderBy) {
        if (item.path.isSimple()) {
          if (!broke && item.direction.type === firstDir.type) {
            presort.push(item);
          } else {
            broke = true;
            spill.push(item);
          }
        }
      }
      orderByComponents = orderByComponentsNew(presort, spill);
    } else {
      // DESC indexes supported - entire ORDER BY satisfied by index
      orderByComponents = orderByComponentsNew([...orderBy], []);
    }

    return Plan.Index(keySpecNew(indexKeyparts), scanDirection, bounds, remainingPredicate, orderByComponents);
  }

  // ── INEQ-FIRST strategy ──────────────────────────────────────────

  /**
   * INEQ-FIRST: [EQ ...] + primary INEQ (bounded). Do NOT append ORDER BY columns; always spill them.
   *
   * Rust: `fn build_ineq_first_plan(...)`
   */
  private buildIneqFirstPlan(
    equalities: [string, Value][],
    inequalities: Map<string, [ComparisonOperator, Value][]>,
    orderBy: OrderByItem[],
    conjuncts: Predicate[],
  ): Plan | null {
    // Pick primary inequality: prefer first OB field with ineq, else first ineq in map order
    let primary: [string, [ComparisonOperator, Value][]] | null = null;

    for (const item of orderBy) {
      if (item.path.isSimple()) {
        const name = item.path.first();
        const ineqVec = inequalities.get(name);
        if (ineqVec !== undefined) {
          primary = [name, ineqVec];
          break;
        }
      }
    }

    if (primary === null) {
      // Fall back to first inequality in map order
      const firstEntry = inequalities.entries().next();
      if (firstEntry.done) return null;
      primary = [firstEntry.value[0], firstEntry.value[1]];
    }

    // Keyparts: EQ + primary INEQ
    const indexKeyparts: IndexKeyPart[] = equalities.map(([f, v]) =>
      indexKeyPartAscPath(f, valueType(v)),
    );
    const primaryValue = primary[1][0][1]; // Get Value from first inequality
    indexKeyparts.push(indexKeyPartAscPath(primary[0], valueType(primaryValue)));

    // Bounds: EQ + primary INEQ (most-restrictive)
    const bounds = this.buildBounds(equalities, [primary[0], primary[1]], indexKeyparts);
    if (bounds === null) return Plan.EmptyScan();
    if (this.isEmptyBounds(bounds)) return Plan.EmptyScan();

    // Remaining predicate: all inequalities except the primary one
    const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, primary[0]);

    // Scan direction
    let scanDirection: ScanDirection;
    if (this.config.supportsDescIndexes) {
      scanDirection = ScanDirection.Forward();
    } else {
      scanDirection = orderBy[0].direction.is('Desc') ? ScanDirection.Reverse() : ScanDirection.Forward();
    }

    // Build OrderByComponents: presort (EQ columns + pivot) and spill (everything else)
    const covered = new Set<string>();
    for (const [f] of equalities) {
      covered.add(f);
    }
    covered.add(primary[0]);

    const presort: OrderByItem[] = [];
    const spill: OrderByItem[] = [];
    for (const item of orderBy) {
      if (item.path.isSimple()) {
        const name = item.path.first();
        if (covered.has(name)) {
          presort.push(item);
        } else {
          spill.push(item);
        }
      }
    }
    const orderByComponents = orderByComponentsNew(presort, spill);

    return Plan.Index(keySpecNew(indexKeyparts), scanDirection, bounds, remainingPredicate, orderByComponents);
  }

  // ── Categorize conjuncts ──────────────────────────────────────────

  /**
   * Categorize conjuncts into equalities and inequalities.
   *
   * Rust: `fn categorize_conjuncts_excluding_primary_key(...)`
   *
   * Divergence: Rust IndexMap → Map (preserves insertion order in ES2015+).
   */
  private categorizeConjunctsExcludingPrimaryKey(
    conjuncts: Predicate[],
    primaryKey: string,
  ): [[string, Value][], Map<string, [ComparisonOperator, Value][]>] {
    const equalities: [string, Value][] = [];
    const inequalities = new Map<string, [ComparisonOperator, Value][]>();

    for (const conjunct of conjuncts) {
      const extracted = this.extractComparison(conjunct);
      if (extracted === null) continue;

      const [field, op, value] = extracted;

      // Skip primary key predicates - they'll be handled by TableScan bounds
      if (field === primaryKey) continue;

      if (op.is('Equal')) {
        equalities.push([field, value]);
      } else if (op.is('GreaterThan') || op.is('GreaterThanOrEqual') || op.is('LessThan') || op.is('LessThanOrEqual')) {
        let vec = inequalities.get(field);
        if (vec === undefined) {
          vec = [];
          inequalities.set(field, vec);
        }
        vec.push([op, value]);
      }
      // NotEqual, In, Between - not supported for index ranges
    }

    return [equalities, inequalities];
  }

  // ── Extract comparison ────────────────────────────────────────────

  /**
   * Extract field path, operator, and value from a comparison predicate.
   * Returns the full path as a dot-separated string (e.g., "context.session_id").
   *
   * Rust: `fn extract_comparison(&self, predicate: &Predicate) -> Option<(String, ComparisonOperator, Value)>`
   */
  private extractComparison(predicate: Predicate): [string, ComparisonOperator, Value] | null {
    if (!predicate.is('Comparison')) return null;
    const comp = predicate.value as { left: any; operator: ComparisonOperator; right: any };

    // Extract field path from left side (supports multi-step paths)
    if (!comp.left.is('Path')) return null;
    const fieldPath = (comp.left.value as { path: any }).path.steps.join('.');

    // Extract value from right side
    if (!comp.right.is('Literal')) return null;
    const value = valueFromLiteral((comp.right.value as { literal: any }).literal);

    return [fieldPath, comp.operator, value];
  }

  // ── Inequality plan (no ORDER BY) ─────────────────────────────────

  /**
   * Rust: `fn generate_inequality_plan_with_order_by(...)`
   */
  private generateInequalityPlanWithOrderBy(
    equalities: [string, Value][],
    inequalityField: string,
    inequalities: Map<string, [ComparisonOperator, Value][]>,
    conjuncts: Predicate[],
    orderBy: OrderByItem[] | null,
  ): Plan | null {
    // Add equality fields first
    const indexKeyparts: IndexKeyPart[] = equalities.map(([field, value]) =>
      indexKeyPartAscPath(field, valueType(value)),
    );

    // Add the inequality field
    const inequalityValues = inequalities.get(inequalityField);
    if (inequalityValues === undefined) return null;
    const firstInequalityValue = inequalityValues[0][1];
    indexKeyparts.push(indexKeyPartAscPath(inequalityField, valueType(firstInequalityValue)));

    // Build bounds
    const bounds = this.buildBounds(equalities, [inequalityField, inequalityValues], indexKeyparts);
    if (bounds === null) return Plan.EmptyScan();
    if (this.isEmptyBounds(bounds)) return Plan.EmptyScan();

    // Calculate remaining predicate (exclude this inequality field)
    const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, inequalityField);

    // Build OrderByComponents
    let orderBySpill: OrderByComponents;
    if (orderBy !== null) {
      const coveredFields = new Set<string>();
      for (const [f] of equalities) {
        coveredFields.add(f);
      }
      coveredFields.add(inequalityField);

      const presort: OrderByItem[] = [];
      const spill: OrderByItem[] = [];
      for (const item of orderBy) {
        if (item.path.isSimple()) {
          const name = item.path.first();
          if (coveredFields.has(name)) {
            presort.push(item);
          } else {
            spill.push(item);
          }
        }
      }
      orderBySpill = orderByComponentsNew(presort, spill);
    } else {
      orderBySpill = orderByComponentsDefault();
    }

    return Plan.Index(keySpecNew(indexKeyparts), ScanDirection.Forward(), bounds, remainingPredicate, orderBySpill);
  }

  // ── Equality-only plan ────────────────────────────────────────────

  /**
   * Generate plan for equality-only queries.
   *
   * Rust: `fn generate_equality_plan(...)`
   */
  private generateEqualityPlan(equalities: [string, Value][], conjuncts: Predicate[]): Plan | null {
    // Add all equality fields
    const indexKeyparts: IndexKeyPart[] = equalities.map(([field, value]) =>
      indexKeyPartAscPath(field, valueType(value)),
    );

    // Build bounds (exact match on all equality values)
    const bounds = this.buildBounds(equalities, null, indexKeyparts);
    if (bounds === null) return Plan.EmptyScan();
    if (this.isEmptyBounds(bounds)) return Plan.EmptyScan();

    // Calculate remaining predicate
    const remainingPredicate = this.calculateRemainingPredicate(conjuncts, equalities, null);

    return Plan.Index(keySpecNew(indexKeyparts), ScanDirection.Forward(), bounds, remainingPredicate, orderByComponentsDefault());
  }

  // ── Build bounds ──────────────────────────────────────────────────

  /**
   * Build bounds based on equalities and optional inequality.
   *
   * Rust: `fn build_bounds(...)`
   */
  private buildBounds(
    equalities: [string, Value][],
    inequality: [string, [ComparisonOperator, Value][]] | null,
    indexKeyparts: IndexKeyPart[],
  ): KeyBounds | null {
    const keypartBounds: KeyBoundComponent[] = [];

    for (const keypart of indexKeyparts) {
      const fullPath = keypart.subPath !== null
        ? [keypart.column, ...keypart.subPath].join('.')
        : keypart.column;

      // Check if this path has an equality constraint
      const equalityEntry = equalities.find(([field]) => field === fullPath);

      if (equalityEntry !== undefined) {
        // Equality constraint: both bounds are the same value, inclusive
        keypartBounds.push(new KeyBoundComponent(fullPath, endpointIncl(equalityEntry[1]), endpointIncl(equalityEntry[1])));
      } else if (inequality !== null) {
        const [ineqField, ineqValues] = inequality;
        if (ineqField === fullPath) {
          // This column has inequality constraints
          let low: Endpoint = Endpoint.UnboundedLow(valueType(ineqValues[0][1]));
          let high: Endpoint = Endpoint.UnboundedHigh(valueType(ineqValues[0][1]));

          // Process all inequalities for this column, choosing most restrictive bounds
          for (const [op, value] of ineqValues) {
            if (op.is('GreaterThan')) {
              const candidate = endpointExcl(value);
              if (this.isMoreRestrictiveLower(candidate, low)) {
                low = candidate;
              }
            } else if (op.is('GreaterThanOrEqual')) {
              const candidate = endpointIncl(value);
              if (this.isMoreRestrictiveLower(candidate, low)) {
                low = candidate;
              }
            } else if (op.is('LessThan')) {
              const candidate = endpointExcl(value);
              if (this.isMoreRestrictiveUpper(candidate, high)) {
                high = candidate;
              }
            } else if (op.is('LessThanOrEqual')) {
              const candidate = endpointIncl(value);
              if (this.isMoreRestrictiveUpper(candidate, high)) {
                high = candidate;
              }
            }
          }

          keypartBounds.push(new KeyBoundComponent(fullPath, low, high));
          break; // Stop at first inequality column
        } else {
          // No constraint on this column - stop here
          break;
        }
      } else {
        // No more constraints - stop here
        break;
      }
    }

    return keyBoundsNew(keypartBounds);
  }

  // ── Bound restrictiveness checks ──────────────────────────────────

  /**
   * Check if candidate lower bound is more restrictive than current.
   *
   * Rust: `fn is_more_restrictive_lower(...)`
   */
  private isMoreRestrictiveLower(candidate: Endpoint, current: Endpoint): boolean {
    if (candidate.is('Value') && current.is('UnboundedLow')) return true;
    if (candidate.is('UnboundedLow') && current.is('Value')) return false;

    if (candidate.is('Value') && current.is('Value')) {
      if (candidate.value.datum.is('Val') && current.value.datum.is('Val')) {
        const cmp = valuePartialCmp(candidate.value.datum.value.value, current.value.datum.value.value);
        if (cmp === null) return false;
        if (cmp > 0) return true;  // Higher value is more restrictive for lower bound
        if (cmp === 0) return !candidate.value.inclusive && current.value.inclusive; // Exclusive is more restrictive
        return false; // Lower value is less restrictive
      }
    }

    return false;
  }

  /**
   * Check if candidate upper bound is more restrictive than current.
   *
   * Rust: `fn is_more_restrictive_upper(...)`
   */
  private isMoreRestrictiveUpper(candidate: Endpoint, current: Endpoint): boolean {
    if (candidate.is('Value') && current.is('UnboundedHigh')) return true;
    if (candidate.is('UnboundedHigh') && current.is('Value')) return false;

    if (candidate.is('Value') && current.is('Value')) {
      if (candidate.value.datum.is('Val') && current.value.datum.is('Val')) {
        const cmp = valuePartialCmp(candidate.value.datum.value.value, current.value.datum.value.value);
        if (cmp === null) return false;
        if (cmp < 0) return true;  // Lower value is more restrictive for upper bound
        if (cmp === 0) return !candidate.value.inclusive && current.value.inclusive; // Exclusive is more restrictive
        return false; // Higher value is less restrictive
      }
    }

    return false;
  }

  // ── Empty bounds check ────────────────────────────────────────────

  /**
   * Check if bounds represent an empty range (impossible to satisfy).
   *
   * Rust: `fn is_empty_bounds(...)`
   */
  private isEmptyBounds(bounds: KeyBounds): boolean {
    for (const bound of bounds.keyparts) {
      if (bound.low.is('Value') && bound.high.is('Value')) {
        if (bound.low.value.datum.is('Val') && bound.high.value.datum.is('Val')) {
          const cmp = valuePartialCmp(bound.low.value.datum.value.value, bound.high.value.datum.value.value);
          if (cmp === null) continue;
          if (cmp > 0) return true; // low > high = empty
          if (cmp === 0) {
            // Equal values but both exclusive = empty
            if (!bound.low.value.inclusive && !bound.high.value.inclusive) {
              return true;
            }
          }
        }
      }
    }
    return false;
  }

  // ── Remaining predicate ───────────────────────────────────────────

  /**
   * Calculate remaining predicate by removing consumed conjuncts.
   *
   * Rust: `fn calculate_remaining_predicate(...)`
   */
  private calculateRemainingPredicate(
    conjuncts: Predicate[],
    consumedEqualities: [string, Value][],
    consumedInequalityField: string | null,
  ): Predicate {
    const remainingConjuncts: Predicate[] = [];

    for (const conjunct of conjuncts) {
      let consumed = false;

      const extracted = this.extractComparison(conjunct);
      if (extracted !== null) {
        const [field] = extracted;

        // Check if it's a consumed equality
        for (const [eqField] of consumedEqualities) {
          if (field === eqField) {
            consumed = true;
            break;
          }
        }

        // Check if it's a consumed inequality
        if (!consumed && consumedInequalityField !== null && field === consumedInequalityField) {
          consumed = true;
        }
      }

      if (!consumed) {
        remainingConjuncts.push(conjunct);
      }
    }

    // Combine remaining conjuncts with AND
    if (remainingConjuncts.length === 0) {
      return Predicate.True();
    } else if (remainingConjuncts.length === 1) {
      return remainingConjuncts[0];
    } else {
      // Build AND chain
      let result: Predicate = remainingConjuncts[0];
      for (let i = 1; i < remainingConjuncts.length; i++) {
        result = Predicate.And(result, remainingConjuncts[i]);
      }
      return result;
    }
  }

  // ── Plan deduplication ────────────────────────────────────────────

  /**
   * Deduplicate plans based on index_spec and scan_direction.
   *
   * Rust: `fn deduplicate_plans(...)`
   */
  private deduplicatePlans(plans: Plan[]): Plan[] {
    const uniquePlans: Plan[] = [];
    const seen = new Set<string>();

    for (const plan of plans) {
      plan.match({
        Index: (v) => {
          // Create a string key for deduplication
          const keypartsStr = v.indexSpec.keyparts
            .map((k) => {
              const fullPath = k.subPath !== null ? [k.column, ...k.subPath].join('.') : k.column;
              return `${fullPath}:${k.direction}:${k.valueType}`;
            })
            .join('|');
          const key = `${keypartsStr}::${v.scanDirection.type}`;
          if (!seen.has(key)) {
            seen.add(key);
            uniquePlans.push(plan);
          }
        },
        EmptyScan: () => {
          // Always include empty scans (they're rare and important)
          uniquePlans.push(plan);
        },
        TableScan: () => {
          // Always include table scans (fallback plan)
          uniquePlans.push(plan);
        },
      });
    }

    return uniquePlans;
  }

  // ── Table scan plan ───────────────────────────────────────────────

  /**
   * Build a table scan plan with optional entity ID range extraction.
   *
   * Rust: `fn build_table_scan_plan(...)`
   */
  private buildTableScanPlan(
    conjuncts: Predicate[],
    primaryKey: string,
    orderBy: OrderByItem[] | null,
  ): Plan {
    // Extract entity ID range from predicates on the primary key
    const bounds = this.extractEntityIdRange(conjuncts, primaryKey);

    // All predicates remain (no index to satisfy any)
    let remainingPredicate: Predicate = Predicate.True();
    for (const pred of conjuncts) {
      if (remainingPredicate.is('True')) {
        remainingPredicate = pred;
      } else {
        remainingPredicate = Predicate.And(remainingPredicate, pred);
      }
    }

    // Determine scan direction and ORDER BY components based on primary key ORDER BY
    let scanDirection: ScanDirection = ScanDirection.Forward();
    let orderBySpill: OrderByComponents = orderByComponentsDefault();

    if (orderBy !== null && orderBy.length > 0) {
      const firstItem = orderBy[0];
      if (firstItem.path.isSimple() && firstItem.path.first() === primaryKey) {
        // Primary key ORDER BY is satisfied by scan direction
        scanDirection = firstItem.direction.is('Desc') ? ScanDirection.Reverse() : ScanDirection.Forward();
        // First item is presort (satisfied by scan), rest is spill
        const presort = [firstItem];
        const spill = orderBy.slice(1);
        orderBySpill = orderByComponentsNew(presort, spill);
      } else {
        // Primary key not in ORDER BY, use forward scan and spill all
        orderBySpill = orderByComponentsNew([], [...orderBy]);
      }
    }

    return Plan.TableScan(bounds, scanDirection, remainingPredicate, orderBySpill);
  }

  // ── Entity ID range extraction ────────────────────────────────────

  /**
   * Extract entity ID range from predicates on the primary key field.
   *
   * Rust: `fn extract_entity_id_range(...)`
   */
  private extractEntityIdRange(conjuncts: Predicate[], primaryKey: string): KeyBounds {
    const primaryKeyBounds: KeyBoundComponent[] = [];

    for (const predicate of conjuncts) {
      const bound = this.extractPrimaryKeyBound(predicate, primaryKey);
      if (bound !== null) {
        primaryKeyBounds.push(bound);
      }
    }

    if (primaryKeyBounds.length === 0) {
      return keyBoundsEmpty();
    }

    if (primaryKeyBounds.length === 1) {
      return keyBoundsNew(primaryKeyBounds);
    }

    // Intersect all bounds to get the most restrictive range
    const intersected = this.intersectPrimaryKeyBounds(primaryKeyBounds, primaryKey);
    return keyBoundsNew([intersected]);
  }

  /**
   * Extract a single primary key bound from a predicate.
   *
   * Rust: `fn extract_primary_key_bound(...)`
   */
  private extractPrimaryKeyBound(predicate: Predicate, primaryKey: string): KeyBoundComponent | null {
    if (!predicate.is('Comparison')) return null;
    const comp = predicate.value as { left: any; operator: ComparisonOperator; right: any };

    // Check if this is a primary key comparison
    let value: Value | null = null;
    if (comp.left.is('Path') && comp.right.is('Literal')) {
      const leftPath = (comp.left.value as { path: any }).path;
      if (leftPath.isSimple() && leftPath.first() === primaryKey) {
        value = valueFromLiteral((comp.right.value as { literal: any }).literal);
      }
    } else if (comp.left.is('Literal') && comp.right.is('Path')) {
      const rightPath = (comp.right.value as { path: any }).path;
      if (rightPath.isSimple() && rightPath.first() === primaryKey) {
        value = valueFromLiteral((comp.left.value as { literal: any }).literal);
      }
    }

    if (value === null) return null;

    // Convert comparison operator to bounds
    let low: Endpoint;
    let high: Endpoint;
    if (comp.operator.is('Equal')) {
      low = Endpoint.Value(KeyDatum.Val(value), true);
      high = Endpoint.Value(KeyDatum.Val(value), true);
    } else if (comp.operator.is('GreaterThan')) {
      low = Endpoint.Value(KeyDatum.Val(value), false);
      high = Endpoint.UnboundedHigh(valueType(value));
    } else if (comp.operator.is('GreaterThanOrEqual')) {
      low = Endpoint.Value(KeyDatum.Val(value), true);
      high = Endpoint.UnboundedHigh(valueType(value));
    } else if (comp.operator.is('LessThan')) {
      low = Endpoint.UnboundedLow(valueType(value));
      high = Endpoint.Value(KeyDatum.Val(value), false);
    } else if (comp.operator.is('LessThanOrEqual')) {
      low = Endpoint.UnboundedLow(valueType(value));
      high = Endpoint.Value(KeyDatum.Val(value), true);
    } else {
      return null; // Skip != and other operators
    }

    return new KeyBoundComponent(primaryKey, low, high);
  }

  /**
   * Intersect multiple primary key bounds to get the most restrictive range.
   *
   * Rust: `fn intersect_primary_key_bounds(...)`
   */
  private intersectPrimaryKeyBounds(bounds: KeyBoundComponent[], primaryKey: string): KeyBoundComponent {
    let resultLow: Endpoint = Endpoint.UnboundedLow(ValueType.String);
    let resultHigh: Endpoint = Endpoint.UnboundedHigh(ValueType.String);

    for (const bound of bounds) {
      resultLow = this.intersectLowerBounds(resultLow, bound.low);
      resultHigh = this.intersectUpperBounds(resultHigh, bound.high);
    }

    return new KeyBoundComponent(primaryKey, resultLow, resultHigh);
  }

  /**
   * Intersect two lower bounds to get the most restrictive (maximum).
   *
   * Rust: `fn intersect_lower_bounds(...)`
   */
  private intersectLowerBounds(left: Endpoint, right: Endpoint): Endpoint {
    if (left.is('UnboundedLow')) return right;
    if (right.is('UnboundedLow')) return left;

    if (left.is('Value') && right.is('Value')) {
      if (left.value.datum.is('Val') && right.value.datum.is('Val')) {
        const cmp = valuePartialCmp(left.value.datum.value.value, right.value.datum.value.value);
        if (cmp === null) return left;
        if (cmp > 0) return left;
        if (cmp < 0) return right;
        // Same value - use the more restrictive inclusivity
        return Endpoint.Value(left.value.datum, left.value.inclusive && right.value.inclusive);
      }
    }

    return left; // Fallback
  }

  /**
   * Intersect two upper bounds to get the most restrictive (minimum).
   *
   * Rust: `fn intersect_upper_bounds(...)`
   */
  private intersectUpperBounds(left: Endpoint, right: Endpoint): Endpoint {
    if (left.is('UnboundedHigh')) return right;
    if (right.is('UnboundedHigh')) return left;

    if (left.is('Value') && right.is('Value')) {
      if (left.value.datum.is('Val') && right.value.datum.is('Val')) {
        const cmp = valuePartialCmp(left.value.datum.value.value, right.value.datum.value.value);
        if (cmp === null) return left;
        if (cmp < 0) return left;
        if (cmp > 0) return right;
        // Same value - use the more restrictive inclusivity
        return Endpoint.Value(left.value.datum, left.value.inclusive && right.value.inclusive);
      }
    }

    return left; // Fallback
  }

  // ── Primary key helpers ───────────────────────────────────────────

  /**
   * Check if a predicate is on the primary key field.
   *
   * Rust: `fn is_primary_key_predicate(...)`
   */
  private isPrimaryKeyPredicate(predicate: Predicate, primaryKey: string): boolean {
    if (!predicate.is('Comparison')) return false;
    const comp = predicate.value as { left: any; operator: any; right: any };
    if (comp.left.is('Path')) {
      const leftPath = (comp.left.value as { path: any }).path;
      if (leftPath.isSimple()) {
        return leftPath.first() === primaryKey;
      }
    }
    return false;
  }

  /**
   * Check if ORDER BY is on the primary key (should skip index generation).
   *
   * Rust: `fn has_primary_key_order_by(...)`
   */
  private hasPrimaryKeyOrderBy(orderBy: OrderByItem[] | null, primaryKey: string): boolean {
    if (orderBy === null || orderBy.length === 0) return false;
    const firstItem = orderBy[0];
    return firstItem.path.isSimple() && firstItem.path.first() === primaryKey;
  }

  /**
   * Check if conjuncts contain primary key range predicates that should skip index generation.
   *
   * Rust: `fn has_primary_key_range_predicates(...)`
   */
  private hasPrimaryKeyRangePredicates(conjuncts: Predicate[], primaryKey: string): boolean {
    return conjuncts.some((predicate) => {
      if (!predicate.is('Comparison')) return false;
      const comp = predicate.value as { left: any; operator: ComparisonOperator; right: any };
      if (!comp.left.is('Path')) return false;
      const leftPath = (comp.left.value as { path: any }).path;
      if (!leftPath.isSimple()) return false;
      if (leftPath.first() !== primaryKey) return false;

      return (
        comp.operator.is('Equal') ||
        comp.operator.is('GreaterThan') ||
        comp.operator.is('GreaterThanOrEqual') ||
        comp.operator.is('LessThan') ||
        comp.operator.is('LessThanOrEqual')
      );
    });
  }
}
