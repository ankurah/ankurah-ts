// MIRRORS: ankurah/storage/sqlite/tests/json_property.rs
//
// SQLite JSON Property Integration Tests
//
// These tests verify that the `Json` property type works correctly with SQLite storage,
// including:
// - Storing entities with Json properties (as BLOB with JSONB format)
// - Querying with JSON path syntax (e.g., `licensing.territory = 'US'`)
// - SQLite JSONB operator behavior for path traversal

import { describe, test, expect } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import { splitPredicateForSqlite } from '../src/index.ts';
import { parseSelection } from '@ankurah/ankql';
import { createSqliteNode, Track } from './common.ts';

// ── Helpers (mirrors json_property.rs assert_fully_pushes_down / get_predicate_split) ──

function assertFullyPushesDown(query: string): void {
  const selection = parseSelection(query);
  const split = splitPredicateForSqlite(selection.predicate);
  expect(split.needsPostFilter()).toBe(false);
}

describe('json_property SQLite integration', () => {
  test('test_json_property_storage_and_simple_query', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    // Create a track with JSON licensing data
    {
      const trx = ctx.begin();
      await trx.create(Track, {
        name: 'Test Track',
        licensing: { territory: 'US', rights: 'exclusive' },
      });
      await trx.commit();
    }

    // Simple query - should work
    const tracks = await ctx.fetch(Track, matchArgs("name = 'Test Track'"));
    expect(tracks.length).toBe(1);
    expect(tracks[0].name()).toBe('Test Track');
  });

  test('test_json_path_pushdown_verification', () => {
    // All these queries should fully push down to SQLite
    assertFullyPushesDown("licensing.territory = 'US'");
    assertFullyPushesDown("licensing.rights.holder = 'Label'");
    assertFullyPushesDown('licensing.count > 10');
    assertFullyPushesDown("name = 'Test' AND licensing.territory = 'US'");
    assertFullyPushesDown("licensing.territory = 'US' OR licensing.territory = 'UK'");

    // Nested paths should also push down
    assertFullyPushesDown("licensing.nested.deeply.value = 'test'");
  });

  test('test_json_path_query_string_equality', async () => {
    // Verify pushdown before running the actual query
    assertFullyPushesDown("licensing.territory = 'US'");

    const { node } = createSqliteNode();
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

    // First verify the data was stored correctly with a simple query
    const allTracks = await ctx.fetch(Track, matchArgs("name = 'US Track' OR name = 'UK Track'"));
    expect(allTracks.length).toBe(2);

    // Verify the JSON data is accessible
    const usTrack = allTracks.find((t) => t.name() === 'US Track');
    const ukTrack = allTracks.find((t) => t.name() === 'UK Track');
    expect(usTrack).toBeDefined();
    expect(ukTrack).toBeDefined();

    const usLicensing = usTrack!.licensing() as Record<string, unknown>;
    const ukLicensing = ukTrack!.licensing() as Record<string, unknown>;
    expect(usLicensing.territory).toBe('US');
    expect(ukLicensing.territory).toBe('UK');

    // Query by JSON path
    const usTracks = await ctx.fetch(Track, matchArgs("licensing.territory = 'US'"));
    expect(usTracks.length).toBe(1);
    expect(usTracks[0].name()).toBe('US Track');
  });

  test('test_json_path_query_numeric_comparison', async () => {
    const { node } = createSqliteNode();
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

  test('test_json_path_nested_query', async () => {
    const { node } = createSqliteNode();
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

  test('test_json_path_combined_with_regular_field', async () => {
    const { node } = createSqliteNode();
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

  test('test_json_path_query_with_or', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    {
      const trx = ctx.begin();
      await trx.create(Track, { name: 'US Track', licensing: { territory: 'US' } });
      await trx.create(Track, { name: 'UK Track', licensing: { territory: 'UK' } });
      await trx.create(Track, { name: 'CA Track', licensing: { territory: 'CA' } });
      await trx.commit();
    }

    // Query with OR condition on JSON path
    const tracks = await ctx.fetch(Track, matchArgs("licensing.territory = 'US' OR licensing.territory = 'UK'"));
    expect(tracks.length).toBe(2);
    const territories = tracks.map((t) => {
      const lic = t.licensing() as Record<string, unknown>;
      return lic.territory as string;
    });
    expect(territories).toContain('US');
    expect(territories).toContain('UK');
  });

  test('test_json_path_query_numeric_ordering', async () => {
    const { node } = createSqliteNode();
    const ctx = node.context();

    {
      const trx = ctx.begin();
      await trx.create(Track, { name: 'Track 1', licensing: { priority: 1, territory: 'US' } });
      await trx.create(Track, { name: 'Track 2', licensing: { priority: 2, territory: 'US' } });
      await trx.create(Track, { name: 'Track 3', licensing: { priority: 3, territory: 'US' } });
      await trx.commit();
    }

    // Query with numeric comparison - should use numeric comparison, not lexicographic
    const highPriorityTracks = await ctx.fetch(Track, matchArgs('licensing.priority > 1'));
    expect(highPriorityTracks.length).toBe(2);

    // Verify numeric comparison worked
    const priorities = highPriorityTracks.map((t) => {
      const lic = t.licensing() as Record<string, unknown>;
      return lic.priority as number;
    });
    expect(priorities).toContain(2);
    expect(priorities).toContain(3);
    expect(priorities).not.toContain(1);
  });
});
