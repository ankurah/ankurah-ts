// MIRRORS: ankurah/storage/common/tests/planner_fixtures.rs
//
// Nothing in storage/common crosses a wire, so there is no encoding to prove here.
// What `plans.json` proves is planner *agreement*: given the same query, primary key
// and engine capability config, does the port choose the same plans, in the same
// order? Order is part of the contract — `Planner::plan` returns a deduplicated
// candidate list with the table scan appended last, and callers pick from that list,
// so the same set in a different order is not equivalent.
//
// The fixture's plans are a hand-written projection of the Rust `Plan` tree, chosen
// so a port never has to reproduce Rust's `Debug` rendering. The `project*`
// functions below are the TypeScript half of that same projection, following the
// schema in `storage/common/test_fixtures/README.md`. `Predicate`, `OrderByItem` and
// `Value` are not projected: they use their real serde shapes, the same ones the
// proto and core fixtures use.
//
// Each case is checked along three independent axes, each its own test:
//   parse   — the port's parser builds the fixture's `selection_ast`, so a parser
//             divergence is not mistaken for a planner divergence
//   plans   — the projected plans, in order
//   bounds  — `normalize` on a hand-built bound set

import { describe, test, expect } from 'bun:test';

import { parseSelection } from '@ankurah/ankql';
import type { OrderByItem, Selection } from '@ankurah/ankql';
import type { IndexKeyPart, Value, ValueType as ValueTypeT } from '@ankurah/core';
import { EntityId } from '@ankurah/proto';

import {
  Planner,
  plannerConfigIndexeddb,
  plannerConfigFullSupport,
  normalize,
  Endpoint,
  KeyDatum,
  KeyBoundComponent,
  KeyBounds,
  Plan,
} from '../src/index';
import type { PlannerConfig } from '../src/index';

import { readSidecar } from '../../proto/__tests__/support/fixtures';
import { coreValueToSerde, toSerde } from '../../proto/__tests__/support/serde';

const fixture = readSidecar('storage/common/test_fixtures/plans.json') as {
  plan_case_count: number;
  bounds_case_count: number;
  plan_cases: PlanCase[];
  bounds_cases: BoundsCase[];
};

interface PlanCase {
  query: string;
  primary_key: string;
  config: { name: 'indexeddb' | 'full_support'; supports_desc_indexes: boolean };
  note: string;
  selection_ast: unknown;
  plan_count: number;
  plans: unknown[];
}

interface BoundsCase {
  label: string;
  note: string;
  bounds: any[];
  normalized: unknown;
}

// ── Projection: the port's Plan tree into the fixture's schema ───────────────

function projectKeypart(kp: IndexKeyPart): unknown {
  return {
    column: kp.column,
    sub_path: kp.subPath ?? null,
    direction: kp.direction as string,
    value_type: kp.valueType as string,
    nulls: (kp.nulls ?? null) as string | null,
    collation: kp.collation ?? null,
  };
}

function projectDatum(datum: KeyDatum): unknown {
  return datum.match({
    Val: (v) => ({ kind: 'Val', value: coreValueToSerde(v.value) }),
    NegInfinity: (v) => ({ kind: 'NegInfinity', value_type: v.valueType as string }),
    PosInfinity: (v) => ({ kind: 'PosInfinity', value_type: v.valueType as string }),
  });
}

function projectEndpoint(endpoint: Endpoint): unknown {
  return endpoint.match({
    UnboundedLow: (v) => ({ kind: 'UnboundedLow', value_type: v.valueType as string }),
    UnboundedHigh: (v) => ({ kind: 'UnboundedHigh', value_type: v.valueType as string }),
    Value: (v) => ({ kind: 'Value', datum: projectDatum(v.datum), inclusive: v.inclusive }),
  });
}

function projectBounds(bounds: KeyBounds): unknown[] {
  return bounds.keyparts.map((b) => ({
    column: b.column,
    low: projectEndpoint(b.low),
    high: projectEndpoint(b.high),
  }));
}

function projectOrderBy(items: OrderByItem[]): unknown[] {
  return items.map((item) => toSerde(item));
}

function projectOrderBySpill(spill: { presort: OrderByItem[]; spill: OrderByItem[]; isSatisfied(): boolean; isGlobalSpill(): boolean }): unknown {
  return {
    presort: projectOrderBy(spill.presort),
    spill: projectOrderBy(spill.spill),
    is_satisfied: spill.isSatisfied(),
    is_global_spill: spill.isGlobalSpill(),
  };
}

function projectPlan(plan: Plan): unknown {
  return plan.match({
    Index: (v) => ({
      kind: 'Index',
      index_spec: v.indexSpec.keyparts.map(projectKeypart),
      scan_direction: v.scanDirection.type as string,
      bounds: projectBounds(v.bounds),
      remaining_predicate: toSerde(v.remainingPredicate),
      order_by_spill: projectOrderBySpill(v.orderBySpill),
    }),
    TableScan: (v) => ({
      kind: 'TableScan',
      bounds: projectBounds(v.bounds),
      scan_direction: v.scanDirection.type as string,
      remaining_predicate: toSerde(v.remainingPredicate),
      order_by_spill: projectOrderBySpill(v.orderBySpill),
    }),
    EmptyScan: () => ({ kind: 'EmptyScan' }),
  });
}

// ── The reverse direction, for the bounds cases' hand-built inputs ───────────

function valueFromSerde(json: any): Value {
  const [variant, payload] = Object.entries(json)[0] as [string, any];
  switch (variant) {
    case 'I16': case 'I32': case 'I64': case 'F64':
      return { type: variant, value: Number(payload) } as Value;
    case 'Bool':
      return { type: 'Bool', value: payload as boolean };
    case 'String':
      return { type: 'String', value: payload as string };
    case 'EntityId':
      return { type: 'EntityId', value: EntityId.fromBase64(payload as string) };
    case 'Object': case 'Binary':
      return { type: variant, value: new Uint8Array(payload as number[]) } as Value;
    case 'Json':
      return { type: 'Json', value: JSON.parse(new TextDecoder().decode(new Uint8Array(payload as number[]))) };
    default:
      throw new Error(`unknown Value variant in the fixture: ${variant}`);
  }
}

function datumFromSerde(json: any): KeyDatum {
  switch (json.kind) {
    case 'Val': return KeyDatum.Val(valueFromSerde(json.value));
    case 'NegInfinity': return KeyDatum.NegInfinity(json.value_type as ValueTypeT);
    case 'PosInfinity': return KeyDatum.PosInfinity(json.value_type as ValueTypeT);
    default: throw new Error(`unknown KeyDatum kind: ${json.kind}`);
  }
}

function endpointFromSerde(json: any): Endpoint {
  switch (json.kind) {
    case 'UnboundedLow': return Endpoint.UnboundedLow(json.value_type as ValueTypeT);
    case 'UnboundedHigh': return Endpoint.UnboundedHigh(json.value_type as ValueTypeT);
    case 'Value': return Endpoint.Value(datumFromSerde(json.datum), json.inclusive as boolean);
    default: throw new Error(`unknown Endpoint kind: ${json.kind}`);
  }
}

function boundsFromSerde(json: any[]): KeyBounds {
  return new KeyBounds(json.map((b) => new KeyBoundComponent(b.column, endpointFromSerde(b.low), endpointFromSerde(b.high))));
}

function configFor(name: 'indexeddb' | 'full_support'): PlannerConfig {
  return name === 'indexeddb' ? plannerConfigIndexeddb() : plannerConfigFullSupport();
}

/** A plan case as a test name: query, config and primary key, since query alone repeats. */
function planCaseName(c: PlanCase): string {
  return `${JSON.stringify(c.query)} [${c.config.name}, pk=${c.primary_key}] — ${c.note}`;
}

test('the fixture case counts match', () => {
  expect(fixture.plan_cases.length).toBe(fixture.plan_case_count);
  expect(fixture.bounds_cases.length).toBe(fixture.bounds_case_count);
});

// ── The parser half, separated so a parser bug is not read as a planner bug ──

describe('plan cases: the parser builds the fixture selection', () => {
  for (const c of fixture.plan_cases) {
    test(planCaseName(c), () => {
      expect(toSerde(parseSelection(c.query))).toEqual(c.selection_ast as any);
    });
  }
});

// ── The planner ─────────────────────────────────────────────────────────────

describe('plan cases: the planner produces the fixture plans, in order', () => {
  for (const c of fixture.plan_cases) {
    test(planCaseName(c), () => {
      const planner = new Planner(configFor(c.config.name));
      expect(configFor(c.config.name).supportsDescIndexes).toBe(c.config.supports_desc_indexes);

      const selection: Selection = parseSelection(c.query);
      const plans = planner.plan(selection, c.primary_key);

      expect(plans.length).toBe(c.plan_count);
      expect(plans.map(projectPlan)).toEqual(c.plans as any);
    });
  }
});

// ── bounds::normalize ───────────────────────────────────────────────────────

describe('bounds cases: normalize produces the fixture range', () => {
  for (const c of fixture.bounds_cases) {
    test(`${c.label} — ${c.note}`, () => {
      const [range, eqPrefixLen, eqPrefixValues] = normalize(boundsFromSerde(c.bounds));

      const projectSide = (side: [Value[], boolean] | null): unknown =>
        side === null ? null : { tuple: side[0].map(coreValueToSerde), open: side[1] };

      expect({
        canonical_range: { lower: projectSide(range.lower), upper: projectSide(range.upper) },
        eq_prefix_len: eqPrefixLen,
        eq_prefix_values: eqPrefixValues.map(coreValueToSerde),
      }).toEqual(c.normalized as any);
    });
  }
});
