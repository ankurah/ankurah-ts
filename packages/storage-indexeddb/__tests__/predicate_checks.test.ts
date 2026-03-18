// MIRRORS: ankurah/storage/indexeddb-wasm/tests/predicate_checks.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, QueryTest,
  matchArgs, IndexedDBStorageEngine,
} from './common.ts';

// Load test cases from shared fixture
import predicateCases from '../../core/__tests__/fixtures/predicate_cases.json';

interface TestEntity {
  label: string;
  data: unknown;
}

interface Expectation {
  query: string;
  matches: string[];
}

interface TestCase {
  name: string;
  entities: TestEntity[];
  expectations: Expectation[];
}

function allTestCases(): TestCase[] {
  return (predicateCases as any).suites.flatMap((s: any) => s.cases);
}

describe('predicate_checks', () => {
  test('test_indexeddb_predicate_checks', async () => {
    const cases = allTestCases();
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    for (const testCase of cases) {
      // Create entities for this test case
      const trx = ctx.begin();
      for (const entity of testCase.entities) {
        await trx.create(QueryTest, { label: entity.label, data: entity.data });
      }
      await trx.commit();

      // Verify each expectation
      for (const exp of testCase.expectations) {
        const results = await ctx.fetch(QueryTest, matchArgs(exp.query));
        const actual = results.map((r: any) => r.label() as string).sort();
        const expected = [...exp.matches].sort();
        expect(actual).toEqual(expected);
      }
    }

    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
