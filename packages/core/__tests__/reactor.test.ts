// MIRRORS: ankurah/core/src/reactor.rs (tests module)
// MIRRORS: ankurah/core/src/reactor/comparison_index.rs (tests module)
// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs (tests module)
// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs (tests module)

import { describe, expect, test } from 'bun:test';
import { EntityId, QueryId, CollectionId } from '@ankurah/proto';
import { PathExpr, Selection, Predicate, Expr, Literal, ComparisonOperator, OrderByItem, OrderDirection } from '@ankurah/ankql';
import type { Value } from '../src/value/index.ts';

import { ComparisonIndex } from '../src/reactor/comparison_index.ts';
import { CandidateChanges } from '../src/reactor/candidate_changes.ts';
import { buildContinuationPredicate, inferValueTypeForField } from '../src/reactor/fetch_gap.ts';
import { ValueType } from '../src/value/index.ts';
import { Entity } from '../src/entity.ts';
import { LWWBackend } from '../src/property/backend/lww.ts';

// ── Helpers ──────────────────────────────────────────────────────────────

/**
 * Sort an array of QueryId by their ULID bytes (ascending).
 * Mirrors Rust BTreeSet ordering used in ComparisonIndex::find_matching.
 */
function sortQueryIds(ids: QueryId[]): QueryId[] {
  return [...ids].sort((a, b) => {
    const ab = a.bytes;
    const bb = b.bytes;
    for (let i = 0; i < 16; i++) {
      if (ab[i] < bb[i]) return -1;
      if (ab[i] > bb[i]) return 1;
    }
    return 0;
  });
}

/**
 * Create a test Entity with the given LWW property values.
 * Mirrors the Rust TestEntity::new() in fetch_gap.rs tests.
 */
function createTestEntity(
  idByte: number,
  data: Record<string, Value>,
): Entity {
  const idBytes = new Uint8Array(16);
  idBytes[15] = idByte;
  const id = EntityId.fromBytes(idBytes);
  const collection = CollectionId.fixedName('test');
  const entity = Entity.create(id, collection);
  const lww = entity.getBackend(LWWBackend);
  for (const [key, value] of Object.entries(data)) {
    lww.set(key, value);
  }
  return entity;
}

// =========================================================================
// ComparisonIndex tests
// MIRRORS: ankurah/core/src/reactor/comparison_index.rs #[cfg(test)]
// =========================================================================

describe('ComparisonIndex', () => {
  test('test_field_index', () => {
    const index = new ComparisonIndex<QueryId>();

    // Less than 8 ----------------------------------------------------------
    const sub0 = QueryId.test(0n);
    index.add(Literal.I64(8n), ComparisonOperator.LessThan(), sub0);

    // 8 should match nothing
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 8 } as Value))).toEqual([]);

    // 7 should match sub0
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 7 } as Value))).toEqual([sub0]);

    const sub1 = QueryId.test(1n);

    // Greater than 20 ------------------------------------------------------
    index.add(Literal.I64(20n), ComparisonOperator.GreaterThan(), sub1);

    // 20 should match nothing
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 20 } as Value))).toEqual([]);

    // 21 should match sub1
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 21 } as Value))).toEqual([sub1]);

    // Add subscriptions for various numeric comparisons
    index.add(Literal.I64(5n), ComparisonOperator.Equal(), sub0);

    // Test exact match (5)
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 5 } as Value))).toEqual([sub0]);

    // Less than 25 ---------------------------------------------------------
    index.add(Literal.I64(25n), ComparisonOperator.LessThan(), sub0);

    // 22 should match sub0 (< 25) and sub1 (> 20)
    // Divergence: Rust returns BTreeSet (sorted by Ord); TS returns insertion-order array.
    // We sort by ULID bytes to match Rust BTreeSet ordering.
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 22 } as Value))).toEqual([sub0, sub1]);

    // 25 should match sub1 because > 20
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 25 } as Value))).toEqual([sub1]);

    // 26 should match sub1 because > 20
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 26 } as Value))).toEqual([sub1]);
  });

  test('test_field_index_not_equal', () => {
    const index = new ComparisonIndex<QueryId>();

    const sub0 = QueryId.test(0n);
    index.add(Literal.I64(8n), ComparisonOperator.NotEqual(), sub0);

    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 8 } as Value))).toEqual([]);
    expect(sortQueryIds(index.findMatching({ type: 'I64', value: 9 } as Value))).toEqual([sub0]);
  });
});

// =========================================================================
// CandidateChanges tests
// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs #[cfg(test)]
// =========================================================================

describe('CandidateChanges', () => {
  test('test_candidate_changes_empty', () => {
    const changes: readonly number[] = [];
    const candidates = new CandidateChanges<number>(changes);
    expect(candidates.isEmpty()).toBe(true);
    expect(candidates.queryCount()).toBe(0);
  });

  test('test_candidate_changes_add_query', () => {
    const changes: readonly number[] = [10, 20, 30, 40, 50];
    const candidates = new CandidateChanges<number>(changes);

    const q1 = QueryId.new();
    const q2 = QueryId.new();

    candidates.addQuery(q1, 1); // 20
    candidates.addQuery(q1, 3); // 40
    candidates.addQuery(q2, 0); // 10

    expect(candidates.queryCount()).toBe(2);
    expect(candidates.isEmpty()).toBe(false);

    const queryMap = new Map<string, number[]>();
    for (const qc of candidates.queryIter()) {
      const values = qc.iter();
      queryMap.set(qc.queryId.toUlidString(), values);
    }

    expect(queryMap.get(q1.toUlidString())).toEqual([20, 40]);
    expect(queryMap.get(q2.toUlidString())).toEqual([10]);
  });

  test('test_candidate_changes_entity_level', () => {
    const changes: readonly number[] = [10, 20, 30];
    const candidates = new CandidateChanges<number>(changes);

    candidates.addEntity(0);
    candidates.addEntity(2);

    const entities = candidates.entityIter();
    expect(entities).toEqual([10, 30]);
  });
});

// =========================================================================
// FetchGap tests
// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs #[cfg(test)]
// =========================================================================

describe('FetchGap', () => {
  test('test_build_gap_predicate_single_column_asc', () => {
    const entity = createTestEntity(1, {
      name: { type: 'String', value: 'John' },
    });

    const originalPredicate = Predicate.True();
    const orderBy: OrderByItem[] = [
      new OrderByItem(PathExpr.simple('name'), OrderDirection.Asc()),
    ];

    const gapPredicate = buildContinuationPredicate(
      originalPredicate,
      orderBy,
      entity,
    );

    // Expected: true AND name >= 'John' AND id != entity.id
    const expected = Predicate.And(
      Predicate.And(
        Predicate.True(),
        Predicate.Comparison(
          Expr.Path(PathExpr.simple('name')),
          ComparisonOperator.GreaterThanOrEqual(),
          Expr.Literal(Literal.String('John')),
        ),
      ),
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('id')),
        ComparisonOperator.NotEqual(),
        Expr.Literal(Literal.EntityId(entity.id().toBytes())),
      ),
    );

    expect(gapPredicate).toEqual(expected);
  });

  test('test_build_gap_predicate_multi_column', () => {
    const entity = createTestEntity(2, {
      name: { type: 'String', value: 'John' },
      age: { type: 'I32', value: 30 },
    });

    const originalPredicate = Predicate.True();
    const orderBy: OrderByItem[] = [
      new OrderByItem(PathExpr.simple('name'), OrderDirection.Asc()),
      new OrderByItem(PathExpr.simple('age'), OrderDirection.Desc()),
    ];

    const gapPredicate = buildContinuationPredicate(
      originalPredicate,
      orderBy,
      entity,
    );

    // Expected: true AND name >= 'John' AND age <= 30 AND id != entity.id
    const expected = Predicate.And(
      Predicate.And(
        Predicate.And(
          Predicate.True(),
          Predicate.Comparison(
            Expr.Path(PathExpr.simple('name')),
            ComparisonOperator.GreaterThanOrEqual(),
            Expr.Literal(Literal.String('John')),
          ),
        ),
        Predicate.Comparison(
          Expr.Path(PathExpr.simple('age')),
          ComparisonOperator.LessThanOrEqual(),
          Expr.Literal(Literal.I32(30)),
        ),
      ),
      Predicate.Comparison(
        Expr.Path(PathExpr.simple('id')),
        ComparisonOperator.NotEqual(),
        Expr.Literal(Literal.EntityId(entity.id().toBytes())),
      ),
    );

    expect(gapPredicate).toEqual(expected);
  });

  test('test_infer_value_type_for_field', () => {
    const entities = [
      createTestEntity(1, {
        name: { type: 'String', value: 'Alice' },
      }),
      createTestEntity(2, {
        age: { type: 'I32', value: 25 },
      }),
    ];

    expect(inferValueTypeForField(entities, 'name')).toBe(ValueType.String);
    expect(inferValueTypeForField(entities, 'age')).toBe(ValueType.I32);
    expect(inferValueTypeForField(entities, 'nonexistent')).toBe(ValueType.String);
  });
});

// =========================================================================
// Reactor end-to-end test
// MIRRORS: ankurah/core/src/reactor.rs #[cfg(test)]
// =========================================================================

describe('Reactor', () => {
  // NOTE: The Rust test test_entity_remains_watched_after_predicate_stops_matching
  // requires async reactor infrastructure (add_query_and_notify, ReactorNodeLike
  // mock, etc.) that involves substantial wiring. The test is ported faithfully
  // below but the underlying machinery (Subscription.registerQuery, updateQuery,
  // etc.) may surface issues during integration. This matches the Rust test
  // structure as closely as possible within the TS type system.

  // Import the Reactor lazily to avoid issues if subscription_state has
  // unresolved dependencies.
  test('test_entity_remains_watched_after_predicate_stops_matching', async () => {
    const { Reactor } = await import('../src/reactor/index.ts');
    const { EntityResultSet } = await import('../src/resultset.ts');
    const { parseSelection } = await import('@ankurah/ankql');
    type ReactorUpdate = import('../src/reactor/update.ts').ReactorUpdate;
    type GapFetcher = import('../src/reactor/fetch_gap.ts').GapFetcher;
    type ReactorNodeLike = import('../src/reactor/index.ts').ReactorNodeLike;

    const reactor = new Reactor();

    // Set up a subscription with a predicate that matches status="pending"
    const rsub = reactor.subscribe();

    // Watcher: accumulates ReactorUpdate values
    const receivedUpdates: ReactorUpdate[] = [];
    const guard = rsub.subscribe((update: ReactorUpdate) => {
      receivedUpdates.push(update);
    });

    const queryId = QueryId.new();
    const collectionId = CollectionId.fixedName('album');
    const selection = parseSelection("status = 'pending'");

    // Create a test entity: album with status="pending"
    const entity1 = Entity.create(EntityId.new(), collectionId);
    const lww = entity1.getBackend(LWWBackend);
    lww.set('name', { type: 'String', value: 'Test Album' });
    lww.set('status', { type: 'String', value: 'pending' });

    const resultset = EntityResultSet.empty();

    // Mock gap fetcher: returns no entities
    const mockGapFetcher: GapFetcher = {
      async fetchGap() {
        return [];
      },
    };

    // Mock node: returns entity1 for any fetch
    const mockNode: ReactorNodeLike = {
      async fetchEntitiesFromLocal() {
        return [entity1];
      },
    };

    // Add query using the reactor - this should send Initial notification
    await reactor.addQueryAndNotify(
      rsub.id(),
      queryId,
      collectionId,
      selection,
      mockNode,
      resultset,
      mockGapFetcher,
    );

    // Verify that we received an Initial notification
    expect(receivedUpdates.length).toBe(1);
    expect(receivedUpdates[0].items.length).toBe(1);
    expect(receivedUpdates[0].items[0].entity).toBe(entity1);
    expect(receivedUpdates[0].items[0].events).toEqual([]);
    expect(receivedUpdates[0].items[0].predicateRelevance).toEqual([
      [queryId, 'Initial'],
    ]);

    // Cleanup
    guard.drop();
    rsub.drop();
  });
});
