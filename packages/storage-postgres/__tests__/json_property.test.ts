// MIRRORS: ankurah/storage/postgres/tests/json_property.rs

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import { splitPredicateForPostgres } from '../src/sql_builder.ts';
import { parseSelection } from '@ankurah/ankql';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Track,
  type PostgresTestContext,
} from './common.ts';

// ── Helpers (these don't need PG connection) ─────────────────────────

/// Assert that a query predicate fully pushes down to PostgreSQL (no post-filtering required).
function assertFullyPushesDown(query: string): void {
  const selection = parseSelection(query);
  const split = splitPredicateForPostgres(selection.predicate);
  expect(split.needsPostFilter()).toBe(false);
}

// ── Tests that DON'T need a PG connection ────────────────────────────

describe('json_property pushdown verification', () => {
  // Rust: fn test_json_path_pushdown_verification
  test('test_json_path_pushdown_verification', () => {
    // All these queries should fully push down to PostgreSQL
    assertFullyPushesDown("licensing.territory = 'US'");
    assertFullyPushesDown("licensing.rights.holder = 'Label'");
    assertFullyPushesDown('licensing.count > 10');
    assertFullyPushesDown("name = 'Test' AND licensing.territory = 'US'");
    assertFullyPushesDown("licensing.territory = 'US' OR licensing.territory = 'UK'");

    // Nested paths should also push down
    assertFullyPushesDown("licensing.nested.deeply.value = 'test'");
  });
});

// ── Tests that NEED a PG connection ──────────────────────────────────

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

import { beforeEach } from 'bun:test';

describe('json_property integration', () => {
  // Each Rust test creates a fresh container. Since we share one container,
  // clean all tables before each test so system.create() works fresh.
  beforeEach(async () => {
    await pgCtx.engine.deleteAllCollections();
  });
  // Rust: fn test_json_property_storage_and_simple_query
  test('test_json_property_storage_and_simple_query', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create Track with JSON licensing data
    {
      const trx = ctx.begin();
      await trx.create(Track, {
        name: 'Test Track',
        licensing: { territory: 'US', rights: 'exclusive' },
      });
      await trx.commit();
    }

    // Query by name = 'Test Track'
    const tracks = await ctx.fetch(Track, matchArgs("name = 'Test Track'"));
    expect(tracks.length).toBe(1);
    expect(tracks[0].name()).toBe('Test Track');
  });

  // Rust: fn test_bytea_jsonb_operator_behavior
  test('test_bytea_jsonb_operator_behavior', async () => {
    // Raw SQL test: JSONB operator (->) on bytea column should error
    const client = await pgCtx.pool.connect();
    try {
      // Create a table with bytea column (simulating old Json storage)
      await client.query('CREATE TABLE IF NOT EXISTS test_bytea (id SERIAL PRIMARY KEY, data BYTEA)');

      // Insert JSON as raw bytes
      const jsonBytes = Buffer.from(JSON.stringify({ territory: 'US' }));
      await client.query('INSERT INTO test_bytea (data) VALUES ($1)', [jsonBytes]);

      // Using JSONB operator (->) on bytea column should ERROR
      let threw = false;
      try {
        await client.query("SELECT data->'territory' FROM test_bytea");
      } catch (err: unknown) {
        threw = true;
        const msg = String(err);
        expect(
          msg.includes('operator does not exist') || msg.includes('type') || msg.includes('bytea'),
        ).toBe(true);
      }
      expect(threw).toBe(true);

      // Verify the column is indeed bytea
      const colInfo = await client.query(
        "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'test_bytea'",
      );
      const dataCol = colInfo.rows.find((r: Record<string, unknown>) => r.column_name === 'data');
      expect(dataCol!.data_type).toBe('bytea');
    } finally {
      client.release();
    }
  });

  // Rust: fn test_json_path_query_string_equality
  test('test_json_path_query_string_equality', async () => {
    assertFullyPushesDown("licensing.territory = 'US'");

    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create tracks with different licensing territories
    {
      const trx = ctx.begin();
      await trx.create(Track, {
        name: 'US Track',
        licensing: { territory: 'US', rights: 'exclusive' },
      });
      await trx.create(Track, {
        name: 'UK Track',
        licensing: { territory: 'UK', rights: 'non-exclusive' },
      });
      await trx.commit();
    }

    // Query by JSON path
    const usTracks = await ctx.fetch(Track, matchArgs("licensing.territory = 'US'"));
    expect(usTracks.length).toBe(1);
    expect(usTracks[0].name()).toBe('US Track');
  });

  // Rust: fn test_json_path_query_numeric_comparison
  test('test_json_path_query_numeric_comparison', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    {
      const trx = ctx.begin();
      await trx.create(Track, {
        name: 'Short Track',
        licensing: { duration: 120, territory: 'US' },
      });
      await trx.create(Track, {
        name: 'Long Track',
        licensing: { duration: 300, territory: 'US' },
      });
      await trx.commit();
    }

    // Query with numeric comparison
    const longTracks = await ctx.fetch(Track, matchArgs('licensing.duration > 200'));
    expect(longTracks.length).toBe(1);
    expect(longTracks[0].name()).toBe('Long Track');
  });

  // Rust: fn test_json_path_nested_query
  test('test_json_path_nested_query', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    {
      const trx = ctx.begin();
      await trx.create(Track, {
        name: 'Label A Track',
        licensing: { rights: { holder: 'Label A', type: 'exclusive' } },
      });
      await trx.create(Track, {
        name: 'Label B Track',
        licensing: { rights: { holder: 'Label B', type: 'non-exclusive' } },
      });
      await trx.commit();
    }

    // Query nested JSON path
    const labelATracks = await ctx.fetch(Track, matchArgs("licensing.rights.holder = 'Label A'"));
    expect(labelATracks.length).toBe(1);
    expect(labelATracks[0].name()).toBe('Label A Track');
  });

  // Rust: fn test_json_path_combined_with_regular_field
  test('test_json_path_combined_with_regular_field', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    {
      const trx = ctx.begin();
      await trx.create(Track, { name: 'Track A', licensing: { territory: 'US' } });
      await trx.create(Track, { name: 'Track B', licensing: { territory: 'US' } });
      await trx.create(Track, { name: 'Track C', licensing: { territory: 'UK' } });
      await trx.commit();
    }

    // Combined query: regular field AND JSON path
    const tracks = await ctx.fetch(Track, matchArgs("name = 'Track A' AND licensing.territory = 'US'"));
    expect(tracks.length).toBe(1);
    expect(tracks[0].name()).toBe('Track A');
  });
});
