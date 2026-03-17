// MIRRORS: ankurah/tests/tests/predicate_checks.rs
//
// Predicate Checks: MemoryStorageEngine vs Filterable
//
// Verifies predicate evaluation consistency between storage backend queries
// and in-memory Filterable evaluation. Test cases loaded from predicate_cases.json.
//
// Divergence: Uses MemoryStorageEngine instead of SledStorageEngine [E8].
// Divergence: JSON file loaded via fs.readFileSync instead of include_str! [E1].

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'fs';
import { join } from 'path';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { parseSelection } from '@ankurah/ankql';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText, lww } from '../../src/define-model.ts';
import { evaluatePredicate } from '../../src/selection/filter.ts';
import type { Filterable } from '../../src/selection/filter.ts';
import { TypeResolver } from '../../src/type_resolver.ts';
import type { Value } from '../../src/value/index.ts';

// ── Test Case Types ──
// Mirrors: predicate_checks.rs PredicateCases, TestSuite, TestCase, TestEntity, Expectation

interface PredicateCases {
  suites: TestSuite[];
}

interface TestSuite {
  name: string;
  cases: TestCaseData[];
}

interface TestCaseData {
  name: string;
  entities: TestEntity[];
  expectations: Expectation[];
}

interface TestEntity {
  label: string;
  data: unknown;
}

interface Expectation {
  query: string;
  matches: string[];
}

// ── Load test cases ──
// Mirrors: predicate_checks.rs `const PREDICATE_CASES_JSON: &str = include_str!("../predicate_cases.json");`
// Divergence: Fixture copied from Rust repo into TS test fixtures [E1].

const predicateCasesJson = readFileSync(join(__dirname, '../fixtures/predicate_cases.json'), 'utf-8');
const predicateCases: PredicateCases = JSON.parse(predicateCasesJson);

function allTestCases(): TestCaseData[] {
  return predicateCases.suites.flatMap((s) => s.cases);
}

// ── MockFilterable ──
// Mirrors: predicate_checks.rs MockFilterable

class MockFilterable implements Filterable {
  private readonly _collection: string;
  private readonly values: Map<string, Value>;

  constructor(collectionName: string) {
    this._collection = collectionName;
    this.values = new Map();
  }

  withJson(name: string, json: unknown): MockFilterable {
    this.values.set(name, { type: 'Json', value: json });
    return this;
  }

  collection(): string {
    return this._collection;
  }

  value(name: string): Value | null {
    return this.values.get(name) ?? null;
  }
}

// ── Filterable verification ──
// Mirrors: predicate_checks.rs verify_filterable

function verifyFilterable(testCase: TestCaseData): void {
  const typeResolver = new TypeResolver();
  for (const entity of testCase.entities) {
    const f = new MockFilterable('QueryTest').withJson('data', entity.data);
    for (const exp of testCase.expectations) {
      const sel = parseSelection(exp.query);
      // Apply type resolver to convert literals for JSON path comparisons
      const resolvedSel = typeResolver.resolveSelectionTypes(sel);
      const matches = evaluatePredicate(f, resolvedSel.predicate);
      const should = exp.matches.includes(entity.label);
      expect(matches).toBe(
        should,
        // Divergence: Bun expect doesn't support custom message as second arg to toBe,
        // so the assertion context is in the test name instead.
      );
    }
  }
}

// ── Test Model ──
// Mirrors: predicate_checks.rs `struct QueryTest { label: String, data: Json }`
// Divergence: Uses defineModel() instead of #[derive(Model)] [E1].
// Divergence: Json field stored as lww<unknown>() [E8].

const QueryTest = defineModel('QueryTest', {
  label: yrsText(),
  data: lww<unknown>(),
});

// ── Tests ──

describe('predicate_checks', () => {
  // Mirrors: predicate_checks.rs verify_filterable (called inline within test_sled_predicate_checks)
  // This is separated into its own test for clarity.
  test('verify_filterable matches predicate_cases.json', () => {
    const cases = allTestCases();
    for (const testCase of cases) {
      verifyFilterable(testCase);
    }
  });

  // Mirrors: predicate_checks.rs test_sled_predicate_checks
  // Divergence: Uses MemoryStorageEngine instead of SledStorageEngine [E8].
  test('test_storage_predicate_checks', async () => {
    const cases = allTestCases();
    const node = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });
    const ctx = node.context();

    for (const testCase of cases) {
      // Also verify filterable for each case (mirrors Rust which calls verify_filterable inline)
      verifyFilterable(testCase);

      const trx = ctx.begin();
      for (const entity of testCase.entities) {
        await trx.create(QueryTest, {
          label: entity.label,
          data: entity.data,
        });
      }
      await trx.commit();

      for (const exp of testCase.expectations) {
        const expected = [...exp.matches].sort();
        const results = await ctx.fetch(QueryTest, matchArgs(exp.query));
        // Rust: results.iter().map(|r| r.label().unwrap()).collect()
        const actual = results
          .map((r: any) => r.label() as string)
          .sort();
        expect(actual).toEqual(expected);
      }
    }
  });
});
