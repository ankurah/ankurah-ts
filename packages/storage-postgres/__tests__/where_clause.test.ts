// MIRRORS: ankurah/storage/postgres/tests/where_clause.rs

import { describe, test, expect, beforeAll, afterAll } from 'bun:test';
import { matchArgs } from '@ankurah/core';
import {
  createPostgresContainer,
  stopPostgresContainer,
  createPostgresNode,
  Album,
  type PostgresTestContext,
} from './common.ts';

let pgCtx: PostgresTestContext;

beforeAll(async () => {
  pgCtx = await createPostgresContainer();
}, 60_000);

afterAll(async () => {
  await stopPostgresContainer(pgCtx);
}, 30_000);

describe('where_clause', () => {
  // Rust: fn pg_basic_where_clause
  test('pg_basic_where_clause', async () => {
    const node = createPostgresNode(pgCtx.engine);
    await node.system.create();
    const ctx = node.context();

    // Create 5 albums with different names and years
    {
      const trx = ctx.begin();
      await trx.create(Album, { name: 'Walking on a Dream', year: '2008' });
      await trx.create(Album, { name: 'Death Magnetic', year: '2008' });
      await trx.create(Album, { name: 'Ice on the Dune', year: '2013' });
      await trx.create(Album, { name: 'Two Vines', year: '2016' });
      await trx.create(Album, { name: 'Ask That God', year: '2024' });
      await trx.commit();
    }

    // Query by name = 'Walking on a Dream' — 1 result
    const byName = await ctx.fetch(Album, matchArgs("name = 'Walking on a Dream'"));
    expect(byName.map((a) => a.name())).toEqual(['Walking on a Dream']);

    // Query by year = '2008' — 2 results
    const byYear = await ctx.fetch(Album, matchArgs("year = '2008'"));
    expect(byYear.map((a) => a.name())).toEqual(['Walking on a Dream', 'Death Magnetic']);

    // Query by name AND year = '1800' — 0 results
    const byNameAndOldYear = await ctx.fetch(Album, matchArgs("name = 'Walking on a Dream' AND year = '1800'"));
    expect(byNameAndOldYear.length).toBe(0);

    // Query name IN ('Walking on a Dream', 'Death Magnetic') — 2 results
    const byNameIn = await ctx.fetch(Album, matchArgs("name IN ('Walking on a Dream', 'Death Magnetic')"));
    expect(byNameIn.map((a) => a.name())).toEqual(['Walking on a Dream', 'Death Magnetic']);

    // Query year IN ('2008', '2013') — 3 results
    const byYearIn = await ctx.fetch(Album, matchArgs("year IN ('2008', '2013')"));
    expect(byYearIn.map((a) => a.name())).toEqual(['Walking on a Dream', 'Death Magnetic', 'Ice on the Dune']);
  });
});
