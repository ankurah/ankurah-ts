// MIRRORS: ankurah/storage/common/src/planner.rs #[cfg(test)]
import { describe, test, expect } from 'bun:test';
import { parseSelection } from '@ankurah/ankql';
import type { Selection } from '@ankurah/ankql';
import { Predicate, PathExpr, OrderDirection, OrderByItem } from '@ankurah/ankql';
import { ValueType, indexKeyPartAscPath, indexKeyPartDescPath, keySpecNew, keySpecEquals, valuePartialCmp } from '@ankurah/core';
import type { KeySpec, IndexKeyPart, Value } from '@ankurah/core';
import { Planner, plannerConfigIndexeddb, plannerConfigFullSupport } from './planner.ts';
import {
  Plan, ScanDirection, Endpoint, KeyDatum, KeyBoundComponent, KeyBounds,
  OrderByComponents,
} from './types.ts';

// ── Helpers matching Rust macros ──

function sel(input: string): Selection {
  return parseSelection(input);
}

function planIndexeddb(input: string): Plan[] {
  const selection = sel(input);
  const planner = new Planner(plannerConfigIndexeddb());
  return planner.plan(selection, 'id');
}

function planFullSupport(input: string): Plan[] {
  const selection = sel(input);
  const planner = new Planner(plannerConfigFullSupport());
  return planner.plan(selection, 'id');
}

// Value constructors
function strVal(s: string): Value { return { type: 'String', value: s }; }
function i32Val(n: number): Value { return { type: 'I32', value: n }; }
function i64Val(n: number): Value { return { type: 'I64', value: n }; }

// OrderByItem constructors
function obyAsc(name: string): OrderByItem {
  return new OrderByItem(PathExpr.simple(name), OrderDirection.Asc());
}
function obyDesc(name: string): OrderByItem {
  return new OrderByItem(PathExpr.simple(name), OrderDirection.Desc());
}

// Endpoint helpers (matching Rust ge/gt/le/lt)
function ge(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), true); }
function gt(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), false); }
function le(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), true); }
function lt(v: Value): Endpoint { return Endpoint.Value(KeyDatum.Val(v), false); }

// Bound builder helpers
function eqBound(col: string, v: Value): KeyBoundComponent {
  return new KeyBoundComponent(col, ge(v), le(v));
}
function rangeBound(col: string, low: Endpoint, high: Endpoint): KeyBoundComponent {
  return new KeyBoundComponent(col, low, high);
}
function unboundedLow(vt: ValueType): Endpoint { return Endpoint.UnboundedLow(vt); }
function unboundedHigh(vt: ValueType): Endpoint { return Endpoint.UnboundedHigh(vt); }

function bounds(...components: KeyBoundComponent[]): KeyBounds {
  return new KeyBounds(components);
}

// ── Structural assertion helpers ──

function assertPlanCount(plans: Plan[], expected: number): void {
  expect(plans.length).toBe(expected);
}

function assertIndex(
  plan: Plan,
  opts: {
    indexSpec: KeySpec;
    scanDirection: 'Forward' | 'Reverse';
    bounds: KeyBounds;
    remainingPredicate: (pred: Predicate) => void;
    orderBySpill: { presort: OrderByItem[]; spill: OrderByItem[] };
  },
): void {
  expect(plan.is('Index')).toBe(true);
  if (!plan.is('Index')) return;
  const v = plan.value;

  // indexSpec
  expect(keySpecEquals(v.indexSpec, opts.indexSpec)).toBe(true);

  // scanDirection
  expect(v.scanDirection.type).toBe(opts.scanDirection);

  // bounds
  assertBoundsEqual(v.bounds, opts.bounds);

  // remainingPredicate
  opts.remainingPredicate(v.remainingPredicate);

  // orderBySpill
  assertOrderByComponents(v.orderBySpill, opts.orderBySpill);
}

function assertTableScan(
  plan: Plan,
  opts: {
    bounds: KeyBounds;
    scanDirection: 'Forward' | 'Reverse';
    remainingPredicate: (pred: Predicate) => void;
    orderBySpill: { presort: OrderByItem[]; spill: OrderByItem[] };
  },
): void {
  expect(plan.is('TableScan')).toBe(true);
  if (!plan.is('TableScan')) return;
  const v = plan.value;

  assertBoundsEqual(v.bounds, opts.bounds);
  expect(v.scanDirection.type).toBe(opts.scanDirection);
  opts.remainingPredicate(v.remainingPredicate);
  assertOrderByComponents(v.orderBySpill, opts.orderBySpill);
}

function assertEmptyScan(plan: Plan): void {
  expect(plan.is('EmptyScan')).toBe(true);
}

function assertTrue(pred: Predicate): void {
  expect(pred.is('True')).toBe(true);
}

function assertPredicateMatchesParse(input: string): (pred: Predicate) => void {
  return (pred: Predicate) => {
    // Compare by generating SQL or checking structure — simplest: compare type trees
    // For now, we just verify it's not True (it should be a real predicate)
    // and that parsing the same input gives same structure
    const expected = parseSelection(input).predicate;
    assertPredicatesEqual(pred, expected);
  };
}

function assertPredicatesEqual(a: Predicate, b: Predicate): void {
  // Structural comparison via type + recursive match
  expect(a.type).toBe(b.type);
  if (a.is('True') || a.is('False') || a.is('Placeholder')) return;

  if (a.is('And') && b.is('And')) {
    assertPredicatesEqual(a.value.left, b.value.left);
    assertPredicatesEqual(a.value.right, b.value.right);
  } else if (a.is('Or') && b.is('Or')) {
    assertPredicatesEqual(a.value.left, b.value.left);
    assertPredicatesEqual(a.value.right, b.value.right);
  } else if (a.is('Not') && b.is('Not')) {
    assertPredicatesEqual(a.value.predicate, b.value.predicate);
  } else if (a.is('Comparison') && b.is('Comparison')) {
    expect(a.value.operator.type).toBe(b.value.operator.type);
    // Compare left/right exprs by type
    expect(a.value.left.type).toBe(b.value.left.type);
    expect(a.value.right.type).toBe(b.value.right.type);
    if (a.value.left.is('Path') && b.value.left.is('Path')) {
      expect(a.value.left.value.path.toString()).toBe(b.value.left.value.path.toString());
    }
    if (a.value.right.is('Literal') && b.value.right.is('Literal')) {
      expect(a.value.right.value.literal.type).toBe(b.value.right.value.literal.type);
      expect(a.value.right.value.literal.value.value).toBe(b.value.right.value.literal.value.value);
    }
    if (a.value.right.is('Path') && b.value.right.is('Path')) {
      expect(a.value.right.value.path.toString()).toBe(b.value.right.value.path.toString());
    }
  } else if (a.is('IsNull') && b.is('IsNull')) {
    expect(a.value.expr.type).toBe(b.value.expr.type);
  }
}

function assertBoundsEqual(a: KeyBounds, b: KeyBounds): void {
  expect(a.keyparts.length).toBe(b.keyparts.length);
  for (let i = 0; i < a.keyparts.length; i++) {
    const ak = a.keyparts[i];
    const bk = b.keyparts[i];
    expect(ak.column).toBe(bk.column);
    assertEndpointEqual(ak.low, bk.low);
    assertEndpointEqual(ak.high, bk.high);
  }
}

function assertEndpointEqual(a: Endpoint, b: Endpoint): void {
  expect(a.type).toBe(b.type);
  if (a.is('Value') && b.is('Value')) {
    expect(a.value.inclusive).toBe(b.value.inclusive);
    expect(a.value.datum.type).toBe(b.value.datum.type);
    if (a.value.datum.is('Val') && b.value.datum.is('Val')) {
      expect(a.value.datum.value.value.type).toBe(b.value.datum.value.value.type);
      // Compare the actual values
      const cmp = valuePartialCmp(a.value.datum.value.value, b.value.datum.value.value);
      expect(cmp).toBe(0);
    }
  }
  if (a.is('UnboundedLow') && b.is('UnboundedLow')) {
    expect(a.value.valueType).toBe(b.value.valueType);
  }
  if (a.is('UnboundedHigh') && b.is('UnboundedHigh')) {
    expect(a.value.valueType).toBe(b.value.valueType);
  }
}

function assertOrderByComponents(
  actual: OrderByComponents,
  expected: { presort: OrderByItem[]; spill: OrderByItem[] },
): void {
  expect(actual.presort.length).toBe(expected.presort.length);
  for (let i = 0; i < actual.presort.length; i++) {
    expect(actual.presort[i].path.toString()).toBe(expected.presort[i].path.toString());
    expect(actual.presort[i].direction.type).toBe(expected.presort[i].direction.type);
  }
  expect(actual.spill.length).toBe(expected.spill.length);
  for (let i = 0; i < actual.spill.length; i++) {
    expect(actual.spill[i].path.toString()).toBe(expected.spill[i].path.toString());
    expect(actual.spill[i].direction.type).toBe(expected.spill[i].direction.type);
  }
}

// Shorthand for no ORDER BY spill
const noOb = { presort: [] as OrderByItem[], spill: [] as OrderByItem[] };

// ── Tests ──

describe('order_by_tests', () => {
  test('basic_order_by', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY foo, bar");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('foo', ValueType.String),
        indexKeyPartAscPath('bar', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('foo'), obyAsc('bar')], spill: [] },
    });
    assertTableScan(plans[1], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("__collection = 'album'"),
      orderBySpill: { presort: [], spill: [obyAsc('foo'), obyAsc('bar')] },
    });
  });

  test('order_by_with_covered_inequality', () => {
    const plans = planIndexeddb("__collection = 'album' AND foo > 10 ORDER BY foo, bar");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('foo', ValueType.String),
        indexKeyPartAscPath('bar', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('foo', gt(i32Val(10)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('foo'), obyAsc('bar')], spill: [] },
    });
    assertTableScan(plans[1], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("__collection = 'album' AND foo > 10"),
      orderBySpill: { presort: [], spill: [obyAsc('foo'), obyAsc('bar')] },
    });
  });

  test('no_collection_field', () => {
    const plans = planIndexeddb('age = 30 ORDER BY foo, bar');
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('age', ValueType.I32),
        indexKeyPartAscPath('foo', ValueType.String),
        indexKeyPartAscPath('bar', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('age', i32Val(30))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('foo'), obyAsc('bar')], spill: [] },
    });
    assertTableScan(plans[1], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse('age = 30'),
      orderBySpill: { presort: [], spill: [obyAsc('foo'), obyAsc('bar')] },
    });
  });

  test('order_by_with_equality', () => {
    const plans = planIndexeddb("__collection = 'album' AND age = 30 ORDER BY foo, bar");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
        indexKeyPartAscPath('foo', ValueType.String),
        indexKeyPartAscPath('bar', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('age', i32Val(30))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('foo'), obyAsc('bar')], spill: [] },
    });
    assertTableScan(plans[1], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("__collection = 'album' AND age = 30"),
      orderBySpill: { presort: [], spill: [obyAsc('foo'), obyAsc('bar')] },
    });
  });

  test('order_by_desc_single_field', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY name DESC");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name')], spill: [] },
    });
    assertTableScan(plans[1], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("__collection = 'album'"),
      orderBySpill: { presort: [], spill: [obyDesc('name')] },
    });
  });

  test('order_by_all_desc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY name DESC, year DESC");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('year', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name'), obyDesc('year')], spill: [] },
    });
  });

  test('order_by_mixed_directions_asc_first', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY name ASC, year DESC");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('name')], spill: [obyDesc('year')] },
    });
  });

  test('order_by_mixed_directions_desc_first', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY name DESC, year ASC");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name')], spill: [obyAsc('year')] },
    });
  });

  // 3-column direction patterns
  test('order_by_three_asc_asc_asc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b ASC, c ASC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
        indexKeyPartAscPath('b', ValueType.String),
        indexKeyPartAscPath('c', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('a'), obyAsc('b'), obyAsc('c')], spill: [] },
    });
  });

  test('order_by_three_asc_asc_desc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b ASC, c DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
        indexKeyPartAscPath('b', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('a'), obyAsc('b')], spill: [obyDesc('c')] },
    });
  });

  test('order_by_three_asc_desc_asc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b DESC, c ASC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('a')], spill: [obyDesc('b'), obyAsc('c')] },
    });
  });

  test('order_by_three_asc_desc_desc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b DESC, c DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('a')], spill: [obyDesc('b'), obyDesc('c')] },
    });
  });

  test('order_by_three_desc_asc_asc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a DESC, b ASC, c ASC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('a')], spill: [obyAsc('b'), obyAsc('c')] },
    });
  });

  test('order_by_three_desc_asc_desc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a DESC, b ASC, c DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('a')], spill: [obyAsc('b'), obyDesc('c')] },
    });
  });

  test('order_by_three_desc_desc_asc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a DESC, b DESC, c ASC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
        indexKeyPartAscPath('b', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('a'), obyDesc('b')], spill: [obyAsc('c')] },
    });
  });

  test('order_by_three_desc_desc_desc', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a DESC, b DESC, c DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('a', ValueType.String),
        indexKeyPartAscPath('b', ValueType.String),
        indexKeyPartAscPath('c', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('a'), obyDesc('b'), obyDesc('c')], spill: [] },
    });
  });

  test('order_by_with_equality_and_desc', () => {
    const plans = planIndexeddb("__collection = 'album' AND status = 'active' ORDER BY name DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('status', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('status', strVal('active'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name')], spill: [] },
    });
  });

  test('order_by_with_inequality_and_desc', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 25 ORDER BY age DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', gt(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('age')], spill: [] },
    });
  });
});

describe('full_support_tests', () => {
  test('full_support_single_desc', () => {
    const plans = planFullSupport("__collection = 'album' ORDER BY name DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name')], spill: [] },
    });
  });

  test('full_support_mixed_directions', () => {
    const plans = planFullSupport("__collection = 'album' ORDER BY name ASC, year DESC, score ASC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('year', ValueType.String),
        indexKeyPartAscPath('score', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('name'), obyDesc('year'), obyAsc('score')], spill: [] },
    });
  });

  test('full_support_all_desc', () => {
    const plans = planFullSupport("__collection = 'album' ORDER BY name DESC, year DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('year', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('name'), obyDesc('year')], spill: [] },
    });
  });

  test('full_support_with_equality_and_mixed_order', () => {
    const plans = planFullSupport("__collection = 'album' AND status = 'active' ORDER BY name ASC, year DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('status', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('year', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('status', strVal('active'))),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('name'), obyDesc('year')], spill: [] },
    });
  });
});

describe('inequality_tests', () => {
  test('single_inequality_plan_structure', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 25");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', gt(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('multiple_inequalities_same_field', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 25 AND age < 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', gt(i32Val(25)), lt(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('multiple_inequalities_different_fields', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 25 AND score < 100");
    assertPlanCount(plans, 3);
    // Plan 1: age index
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', gt(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertPredicateMatchesParse('score < 100'),
      orderBySpill: noOb,
    });
    // Plan 2: score index
    assertIndex(plans[1], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('score', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('score', unboundedLow(ValueType.I32), lt(i32Val(100))),
      ),
      remainingPredicate: assertPredicateMatchesParse('age > 25'),
      orderBySpill: noOb,
    });
  });

  test('greater_or_equal_inclusive_lower_bound', () => {
    const plans = planIndexeddb("__collection = 'album' AND age >= 25");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', ge(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('less_than_exclusive_upper_bound', () => {
    const plans = planIndexeddb("__collection = 'album' AND age < 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', unboundedLow(ValueType.I32), lt(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('less_or_equal_inclusive_upper_bound', () => {
    const plans = planIndexeddb("__collection = 'album' AND age <= 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', unboundedLow(ValueType.I32), le(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('range_inclusive_both', () => {
    const plans = planIndexeddb("__collection = 'album' AND age >= 25 AND age <= 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', ge(i32Val(25)), le(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('range_mixed_gte_lt', () => {
    const plans = planIndexeddb("__collection = 'album' AND age >= 25 AND age < 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', ge(i32Val(25)), lt(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('range_mixed_gt_lte', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 25 AND age <= 50");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', gt(i32Val(25)), le(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('gte_with_desc_order_by', () => {
    const plans = planIndexeddb("__collection = 'album' AND age >= 25 ORDER BY age DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', ge(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('age')], spill: [] },
    });
  });

  test('lte_with_desc_order_by', () => {
    const plans = planIndexeddb("__collection = 'album' AND age <= 50 ORDER BY age DESC");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.String),
      ]),
      scanDirection: 'Reverse',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', unboundedLow(ValueType.I32), le(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('age')], spill: [] },
    });
  });
});

describe('equality_tests', () => {
  test('single_equality', () => {
    const plans = planIndexeddb("__collection = 'album' AND name = 'Alice'");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('name', strVal('Alice'))),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('multiple_equalities', () => {
    const plans = planIndexeddb("__collection = 'album' AND name = 'Alice' AND age = 30");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('name', strVal('Alice')), eqBound('age', i32Val(30))),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('four_column_equality_prefix', () => {
    const plans = planIndexeddb("__collection = 'album' AND artist = 'Queen' AND year = 1975 AND genre = 'Rock'");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('artist', ValueType.String),
        indexKeyPartAscPath('year', ValueType.I32),
        indexKeyPartAscPath('genre', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        eqBound('artist', strVal('Queen')),
        eqBound('year', i32Val(1975)),
        eqBound('genre', strVal('Rock')),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('three_equality_with_order_by', () => {
    const plans = planIndexeddb("__collection = 'album' AND artist = 'Queen' AND year = 1975 ORDER BY title");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('artist', ValueType.String),
        indexKeyPartAscPath('year', ValueType.I32),
        indexKeyPartAscPath('title', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        eqBound('artist', strVal('Queen')),
        eqBound('year', i32Val(1975)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('title')], spill: [] },
    });
  });

  test('three_equality_with_inequality', () => {
    const plans = planIndexeddb("__collection = 'album' AND artist = 'Queen' AND year = 1975 AND rating > 4");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('artist', ValueType.String),
        indexKeyPartAscPath('year', ValueType.I32),
        indexKeyPartAscPath('rating', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        eqBound('artist', strVal('Queen')),
        eqBound('year', i32Val(1975)),
        rangeBound('rating', gt(i32Val(4)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });
});

describe('mixed_tests', () => {
  test('equality_with_inequality', () => {
    const plans = planIndexeddb("__collection = 'album' AND name = 'Alice' AND age > 25");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        eqBound('name', strVal('Alice')),
        rangeBound('age', gt(i32Val(25)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('equality_with_order_by_and_matching_inequality', () => {
    const plans = planIndexeddb("__collection = 'album' AND score > 50 AND age = 30 ORDER BY score");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
        indexKeyPartAscPath('score', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        eqBound('age', i32Val(30)),
        rangeBound('score', gt(i32Val(50)), unboundedHigh(ValueType.I32)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyAsc('score')], spill: [] },
    });
  });
});

describe('edge_cases', () => {
  test('collection_only_query', () => {
    const plans = planIndexeddb("__collection = 'album'");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([indexKeyPartAscPath('__collection', ValueType.String)]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('unsupported_operators', () => {
    const plans = planIndexeddb("__collection = 'album' AND name != 'Alice'");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([indexKeyPartAscPath('__collection', ValueType.String)]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertPredicateMatchesParse("name != 'Alice'"),
      orderBySpill: noOb,
    });
  });

  test('impossible_range', () => {
    const plans = planIndexeddb("__collection = 'album' AND age > 50 AND age < 30");
    assertPlanCount(plans, 1);
    assertEmptyScan(plans[0]);
  });

  test('or_only_predicate', () => {
    const plans = planIndexeddb("__collection = 'album' AND (age > 25 OR name = 'Alice')");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([indexKeyPartAscPath('__collection', ValueType.String)]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album'))),
      remainingPredicate: assertPredicateMatchesParse("age > 25 OR name = 'Alice'"),
      orderBySpill: noOb,
    });
  });

  test('complex_nested_predicate', () => {
    const plans = planIndexeddb("__collection = 'album' AND score = 100 AND (age > 25 OR name = 'Alice')");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('score', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('score', i32Val(100))),
      remainingPredicate: assertPredicateMatchesParse("age > 25 OR name = 'Alice'"),
      orderBySpill: noOb,
    });
  });

  test('primary_key_only_equality', () => {
    const plans = planIndexeddb("id = '12345678-1234-1234-1234-123456789abc'");
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: bounds(eqBound('id', strVal('12345678-1234-1234-1234-123456789abc'))),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("id = '12345678-1234-1234-1234-123456789abc'"),
      orderBySpill: noOb,
    });
  });

  test('primary_key_only_with_order_by', () => {
    const plans = planIndexeddb("id > '12345678-1234-1234-1234-123456789abc' ORDER BY id DESC");
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: bounds(
        rangeBound('id', gt(strVal('12345678-1234-1234-1234-123456789abc')), unboundedHigh(ValueType.String)),
      ),
      scanDirection: 'Reverse',
      remainingPredicate: assertPredicateMatchesParse("id > '12345678-1234-1234-1234-123456789abc'"),
      orderBySpill: { presort: [obyDesc('id')], spill: [] },
    });
  });

  test('primary_key_with_non_primary_order_by', () => {
    const plans = planIndexeddb("id = '12345678-1234-1234-1234-123456789abc' ORDER BY name ASC");
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: bounds(eqBound('id', strVal('12345678-1234-1234-1234-123456789abc'))),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("id = '12345678-1234-1234-1234-123456789abc'"),
      orderBySpill: { presort: [], spill: [obyAsc('name')] },
    });
  });

  test('primary_key_not_equal', () => {
    const plans = planIndexeddb("id != '12345678-1234-1234-1234-123456789abc'");
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("id != '12345678-1234-1234-1234-123456789abc'"),
      orderBySpill: noOb,
    });
  });

  test('no_predicate_no_order_by', () => {
    const plans = planIndexeddb('true');
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: new KeyBounds([]),
      scanDirection: 'Forward',
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('no_predicate_with_order_by', () => {
    const plans = planIndexeddb('true ORDER BY id DESC');
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: new KeyBounds([]),
      scanDirection: 'Reverse',
      remainingPredicate: assertTrue,
      orderBySpill: { presort: [obyDesc('id')], spill: [] },
    });
  });

  test('primary_key_range_intersection', () => {
    const plans = planIndexeddb("id >= '12345678-1234-1234-1234-123456789aaa' AND id <= '12345678-1234-1234-1234-123456789zzz'");
    assertPlanCount(plans, 1);
    assertTableScan(plans[0], {
      bounds: bounds(
        rangeBound('id',
          ge(strVal('12345678-1234-1234-1234-123456789aaa')),
          le(strVal('12345678-1234-1234-1234-123456789zzz')),
        ),
      ),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("id >= '12345678-1234-1234-1234-123456789aaa' AND id <= '12345678-1234-1234-1234-123456789zzz'"),
      orderBySpill: noOb,
    });
  });

  test('mixed_primary_and_secondary_predicates', () => {
    const plans = planIndexeddb("__collection = 'album' AND id > '12345678-1234-1234-1234-123456789abc' AND name = 'Alice'");
    assertPlanCount(plans, 2);
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('name', strVal('Alice'))),
      remainingPredicate: assertPredicateMatchesParse("id > '12345678-1234-1234-1234-123456789abc'"),
      orderBySpill: noOb,
    });
    assertTableScan(plans[1], {
      bounds: bounds(
        rangeBound('id', gt(strVal('12345678-1234-1234-1234-123456789abc')), unboundedHigh(ValueType.String)),
      ),
      scanDirection: 'Forward',
      remainingPredicate: assertPredicateMatchesParse("__collection = 'album' AND id > '12345678-1234-1234-1234-123456789abc' AND name = 'Alice'"),
      orderBySpill: noOb,
    });
  });

  test('multiple_inequalities_same_field_complex', () => {
    const plans = planIndexeddb("__collection = 'album' AND age >= 25 AND age <= 50 AND age > 20");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('age', ValueType.I32),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('age', ge(i32Val(25)), le(i32Val(50))),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('large_numbers', () => {
    const plans = planIndexeddb("__collection = 'album' AND timestamp > 9223372036854775807");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('timestamp', ValueType.I64),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(
        eqBound('__collection', strVal('album')),
        rangeBound('timestamp', gt(i64Val(9223372036854775807)), unboundedHigh(ValueType.I64)),
      ),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('empty_string_equality', () => {
    const plans = planIndexeddb("__collection = 'album' AND name = ''");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('name', strVal(''))),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });

  test('empty_string_with_other_fields', () => {
    const plans = planIndexeddb("__collection = 'album' AND name = '' AND year = '2000'");
    assertIndex(plans[0], {
      indexSpec: keySpecNew([
        indexKeyPartAscPath('__collection', ValueType.String),
        indexKeyPartAscPath('name', ValueType.String),
        indexKeyPartAscPath('year', ValueType.String),
      ]),
      scanDirection: 'Forward',
      bounds: bounds(eqBound('__collection', strVal('album')), eqBound('name', strVal('')), eqBound('year', strVal('2000'))),
      remainingPredicate: assertTrue,
      orderBySpill: noOb,
    });
  });
});

describe('json_path_tests', () => {
  test('json_path_equality', () => {
    const plans = planFullSupport("context.session_id = 'sess123'");
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      const v = indexPlan.value;
      expect(v.indexSpec.keyparts.length).toBe(1);
      const kp = v.indexSpec.keyparts[0];
      expect(kp.column).toBe('context');
      expect(kp.subPath).toEqual(['session_id']);
      expect(v.bounds.keyparts.length).toBe(1);
      expect(v.bounds.keyparts[0].column).toBe('context.session_id');
    }
  });

  test('json_path_with_order_by', () => {
    const plans = planFullSupport("context.user_id = 'user123' ORDER BY created DESC");
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      const first = indexPlan.value.indexSpec.keyparts[0];
      expect(first.column).toBe('context');
      expect(first.subPath).toEqual(['user_id']);
      if (indexPlan.value.indexSpec.keyparts.length > 1) {
        const second = indexPlan.value.indexSpec.keyparts[1];
        expect(second.column).toBe('created');
        expect(second.subPath).toBeNull();
      }
    }
  });

  test('deep_json_path', () => {
    const plans = planFullSupport("data.nested.field = 'value'");
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      const kp = indexPlan.value.indexSpec.keyparts[0];
      expect(kp.column).toBe('data');
      expect(kp.subPath).toEqual(['nested', 'field']);
    }
  });

  test('json_path_full_pushdown', () => {
    const plans = planFullSupport("context.session_id = 'sess123'");
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      expect(indexPlan.value.remainingPredicate.is('True')).toBe(true);
    }
  });

  test('json_path_inequality', () => {
    const plans = planFullSupport('context.count > 100');
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      const kp = indexPlan.value.indexSpec.keyparts[0];
      expect(kp.column).toBe('context');
      expect(kp.subPath).toEqual(['count']);
      expect(indexPlan.value.remainingPredicate.is('True')).toBe(true);
    }
  });

  test('json_path_mixed_predicates', () => {
    const plans = planFullSupport("status = 'active' AND context.user_id = 'user123'");
    const indexPlan = plans.find(p => p.is('Index'));
    expect(indexPlan).toBeDefined();
    if (indexPlan && indexPlan.is('Index')) {
      expect(indexPlan.value.indexSpec.keyparts.length).toBe(2);
      const jsonKp = indexPlan.value.indexSpec.keyparts.find(kp => kp.subPath !== null);
      expect(jsonKp).toBeDefined();
      if (jsonKp) {
        expect(jsonKp.column).toBe('context');
        expect(jsonKp.subPath).toEqual(['user_id']);
      }
      expect(indexPlan.value.remainingPredicate.is('True')).toBe(true);
    }
  });
});

describe('order_by_spill_tests', () => {
  test('spill_preserves_column_order', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b DESC, c ASC");
    const indexPlan = plans[0];
    expect(indexPlan.is('Index')).toBe(true);
    if (indexPlan.is('Index')) {
      const obc = indexPlan.value.orderBySpill;
      expect(obc.presort.length).toBe(1);
      expect(obc.presort[0].path.property()).toBe('a');
      expect(obc.spill.length).toBe(2);
      expect(obc.spill[0].path.property()).toBe('b');
      expect(obc.spill[1].path.property()).toBe('c');
    }
  });

  test('spill_preserves_directions', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a ASC, b DESC, c ASC");
    const indexPlan = plans[0];
    expect(indexPlan.is('Index')).toBe(true);
    if (indexPlan.is('Index')) {
      const obc = indexPlan.value.orderBySpill;
      expect(obc.spill.length).toBe(2);
      expect(obc.spill[0].direction.is('Desc')).toBe(true);
      expect(obc.spill[1].direction.is('Asc')).toBe(true);
    }
  });

  test('spill_with_limit', () => {
    const selection = sel("__collection = 'album' ORDER BY a ASC, b DESC LIMIT 10");
    const planner = new Planner(plannerConfigIndexeddb());
    const plans = planner.plan(selection, 'id');
    const indexPlan = plans[0];
    expect(indexPlan.is('Index')).toBe(true);
    if (indexPlan.is('Index')) {
      const obc = indexPlan.value.orderBySpill;
      expect(obc.presort.length).toBe(1);
      expect(obc.presort[0].path.property()).toBe('a');
      expect(obc.spill.length).toBe(1);
      expect(obc.spill[0].path.property()).toBe('b');
      expect(obc.spill[0].direction.is('Desc')).toBe(true);
    }
  });

  test('table_scan_spill_matches_full_order_by', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY x DESC, y ASC, z DESC");
    const tablePlan = plans.find(p => p.is('TableScan'));
    expect(tablePlan).toBeDefined();
    if (tablePlan && tablePlan.is('TableScan')) {
      const obc = tablePlan.value.orderBySpill;
      expect(obc.presort.length).toBe(0);
      expect(obc.spill.length).toBe(3);
      expect(obc.spill[0].path.property()).toBe('x');
      expect(obc.spill[0].direction.is('Desc')).toBe(true);
      expect(obc.spill[1].path.property()).toBe('y');
      expect(obc.spill[1].direction.is('Asc')).toBe(true);
      expect(obc.spill[2].path.property()).toBe('z');
      expect(obc.spill[2].direction.is('Desc')).toBe(true);
    }
  });

  test('no_spill_when_fully_satisfied', () => {
    const plans = planIndexeddb("__collection = 'album' ORDER BY a");
    const indexPlan = plans[0];
    expect(indexPlan.is('Index')).toBe(true);
    if (indexPlan.is('Index')) {
      const obc = indexPlan.value.orderBySpill;
      expect(obc.presort.length).toBe(1);
      expect(obc.presort[0].path.property()).toBe('a');
      expect(obc.spill.length).toBe(0);
    }
  });

  test('equality_prefix_affects_spill', () => {
    const plans = planIndexeddb("__collection = 'album' AND category = 'rock' ORDER BY rating");
    const indexPlan = plans[0];
    expect(indexPlan.is('Index')).toBe(true);
    if (indexPlan.is('Index')) {
      const obc = indexPlan.value.orderBySpill;
      expect(obc.presort.length).toBe(1);
      expect(obc.presort[0].path.property()).toBe('rating');
      expect(obc.spill.length).toBe(0);
    }
  });
});
