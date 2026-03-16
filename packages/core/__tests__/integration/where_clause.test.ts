// MIRRORS: ankurah/tests/tests/where_clause.rs
//
// Tests for basic WHERE clause filtering via ctx.fetch().
// Rust uses the fetch! macro which expands to ctx.fetch() with variable interpolation.
// TS uses ctx.fetch() with matchArgs() directly [E1].

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { Node, matchArgs } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';
import { YrsString } from '../../src/property/value/yrs_string.ts';
import type { Entity } from '../../src/entity.ts';

// ── Model ──
// Mirrors: common.rs `struct Album { pub name: String, pub year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── Helpers ──

function createTestNode(): Node {
  return new Node({
    storageEngine: new MemoryStorageEngine(),
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
}

function getYrsStringHandle(entity: Entity, fieldName: string): YrsString {
  const backend = entity.getBackend(YjsBackend);
  return new YrsString(fieldName, backend, entity);
}

// ── Tests ──

describe('where_clause', () => {
  // Mirrors: basic_where_clause
  test('basic_where_clause', async () => {
    // Rust: let node = Node::new_durable(Arc::new(SledStorageEngine::new_test().unwrap()), PermissiveAgent::new());
    // Divergence: MemoryStorageEngine instead of SledStorageEngine [E5]
    const node = createTestNode();
    // Rust: node.system.create().await?;
    // Divergence: SystemManager not yet ported — skip [E8]
    const client = node.context();

    // Create albums
    {
      const trx = client.begin();

      const walkingBorrow = await trx.create(Album, {});
      const walkingEntity = walkingBorrow.inner.entity();
      getYrsStringHandle(walkingEntity, 'name').insert(0, 'Walking on a Dream');
      getYrsStringHandle(walkingEntity, 'year').insert(0, '2008');

      const iceBorrow = await trx.create(Album, {});
      const iceEntity = iceBorrow.inner.entity();
      getYrsStringHandle(iceEntity, 'name').insert(0, 'Ice on the Dune');
      getYrsStringHandle(iceEntity, 'year').insert(0, '2013');

      const twoBorrow = await trx.create(Album, {});
      const twoEntity = twoBorrow.inner.entity();
      getYrsStringHandle(twoEntity, 'name').insert(0, 'Two Vines');
      getYrsStringHandle(twoEntity, 'year').insert(0, '2016');

      const askBorrow = await trx.create(Album, {});
      const askEntity = askBorrow.inner.entity();
      getYrsStringHandle(askEntity, 'name').insert(0, 'Ask That God');
      getYrsStringHandle(askEntity, 'year').insert(0, '2024');

      await trx.commit();
    }

    // Rust: let name = "Walking on a Dream";
    // Rust: let albums: Vec<AlbumView> = fetch!(client, { name }).await?;
    // TS: ctx.fetch(Album, matchArgs("name = 'Walking on a Dream'"))
    const albums = await client.fetch(Album, matchArgs("name = 'Walking on a Dream'"));

    // Rust: assert_eq!(albums.iter().map(|a| a.name().unwrap()).collect::<Vec<String>>(), vec!["Walking on a Dream"]);
    const names = albums.map((a: any) => a.name());
    expect(names).toEqual(['Walking on a Dream']);

    // Test IN with array expansion
    // Rust: let names = vec!["Walking on a Dream", "Ice on the Dune"];
    // Rust: let albums: Vec<AlbumView> = fetch!(client, name IN {names}).await?;
    const albumsIn = await client.fetch(Album, matchArgs("name IN ('Walking on a Dream', 'Ice on the Dune')"));
    let resultNames = albumsIn.map((a: any) => a.name() as string);
    resultNames.sort();
    // Rust: assert_eq!(vec!["Ice on the Dune", "Walking on a Dream"], result);
    expect(resultNames).toEqual(['Ice on the Dune', 'Walking on a Dream']);

    // Test IN with years using array expansion
    // Rust: let years = vec!["2008", "2013"];
    // Rust: let albums: Vec<AlbumView> = fetch!(client, year IN {years}).await?;
    const albumsYears = await client.fetch(Album, matchArgs("year IN ('2008', '2013')"));
    let resultYearNames = albumsYears.map((a: any) => a.name() as string);
    resultYearNames.sort();
    // Rust: assert_eq!(vec!["Ice on the Dune", "Walking on a Dream"], result);
    expect(resultYearNames).toEqual(['Ice on the Dune', 'Walking on a Dream']);
  });

  // Mirrors: test_where_clause_with_id
  test('test_where_clause_with_id', async () => {
    const node = createTestNode();
    const ctx = node.context();

    // Create an album and capture its ID
    let albumId;
    {
      const trx = ctx.begin();
      const walkingBorrow = await trx.create(Album, {});
      const walkingEntity = walkingBorrow.inner.entity();
      getYrsStringHandle(walkingEntity, 'name').insert(0, 'Walking on a Dream');
      getYrsStringHandle(walkingEntity, 'year').insert(0, '2008');
      albumId = walkingBorrow.inner.id();
      await trx.commit();
    }

    // Test querying by ID
    // Rust: let albums: Vec<AlbumView> = fetch!(ctx, id = { album_id }).await?;
    const albums = await ctx.fetch(Album, matchArgs(`id = '${albumId.toBase64()}'`));

    // Rust: assert_eq!(albums.iter().map(|a| a.name().unwrap()).collect::<Vec<String>>(), vec!["Walking on a Dream"]);
    const names = albums.map((a: any) => a.name());
    expect(names).toEqual(['Walking on a Dream']);
  });
});
