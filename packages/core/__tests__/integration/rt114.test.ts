// MIRRORS: ankurah/tests/tests/rt114.rs
//
// Regression test for RT-114: After resubscribing, entities that no longer match
// the predicate (due to server-side edits while unsubscribed) should not appear
// in the LiveQuery results.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';
import { CollectionId } from '@ankurah/proto';
import { Node, nocache } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';

// ── Model ──
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

describe('rt114', () => {
  // Mirrors: rt114.rs rt114
  test('rt114', async () => {
    // Set up server (durable) and client (ephemeral)
    const server = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });
    await server.system.create();
    const clientStorage = new MemoryStorageEngine();
    const client = new Node({
      storageEngine: clientStorage,
      policyAgent: new PermissiveAgent(),
      durable: false,
    });
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();

    // Create two albums on the server, both initially matching year >= 2020
    let album1Id, album2Id;
    {
      const trx = serverCtx.begin();
      const album1 = await trx.create(Album, { name: 'Test Album 1', year: '2020' });
      album1Id = album1.inner.id();
      const album2 = await trx.create(Album, { name: 'Test Album 2', year: '2020' });
      album2Id = album2.inner.id();
      await trx.commit();
    }

    const clientCollection = await clientStorage.collection(new CollectionId('album'));
    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // before subscribe
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // before subscribe

    // Subscribe on the client with predicate year >= 2020
    const clientQuery = await clientCtx.queryWait(Album, nocache("year >= '2020'"));
    expect(clientQuery.peek().map((p: any) => p.year()).sort()).toEqual(['2020', '2020']);

    // actually zero events because we receive a state from ItemChange::Initial
    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // after subscribe
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // after subscribe

    // Unsubscribe (drop the LiveQuery)
    clientQuery.drop();

    // wait for the unsubscribe to be propagated to the server
    await new Promise(resolve => setTimeout(resolve, 200));

    // Make changes on the server while client is unsubscribed
    // Album2: change to 2019 (no longer matches year >= 2020)
    {
      const trx = serverCtx.begin();
      const serverAlbum2 = await trx.get(Album, album2Id);
      const yjs = serverAlbum2.inner.entity().getBackend(YjsBackend);
      yjs.delete('year', 0, 4); // delete "2020"
      yjs.insert('year', 0, '2019');
      await trx.commit();
    }

    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // after edits
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // after edits

    await new Promise(resolve => setTimeout(resolve, 200));

    // Resubscribe on the client
    const clientQuery2 = await clientCtx.queryWait(Album, nocache("year >= '2020'"));

    // The client should receive only album1 with the correct state (year = "2020")
    // Album2 should not be returned since it no longer matches (year = "2019")
    expect(clientQuery2.peek().map((p: any) => p.year())).toEqual(['2020']);

    clientQuery2.drop();
    conn.destroy();
  });

  // Mirrors: rt114.rs rt114_b
  // Same scenario as rt114 but using fetch() instead of LiveQuery.
  test('rt114_b', async () => {
    // Set up server (durable) and client (ephemeral)
    const server = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });
    await server.system.create();
    const clientStorage = new MemoryStorageEngine();
    const client = new Node({
      storageEngine: clientStorage,
      policyAgent: new PermissiveAgent(),
      durable: false,
    });
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    const serverCtx = await server.contextAsync();
    const clientCtx = await client.contextAsync();

    // Create two albums on the server, both initially matching year >= 2020
    let album1Id, album2Id;
    {
      const trx = serverCtx.begin();
      const album1 = await trx.create(Album, { name: 'Test Album 1', year: '2020' });
      album1Id = album1.inner.id();
      const album2 = await trx.create(Album, { name: 'Test Album 2', year: '2020' });
      album2Id = album2.inner.id();
      await trx.commit();
    }

    const clientCollection = await clientStorage.collection(new CollectionId('album'));
    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // before fetch
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // before fetch

    // Fetch on the client with predicate year >= 2020
    const initialFetch = await clientCtx.fetch(Album, nocache("year >= '2020'"));
    const initialYears = initialFetch.map((album: any) => album.year()).sort();
    expect(initialYears).toEqual(['2020', '2020']);

    // actually zero events because we receive states directly
    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // after fetch
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // after fetch

    // Make changes on the server while client has cached data
    // Album2: change to 2019 (no longer matches year >= 2020)
    {
      const trx = serverCtx.begin();
      const serverAlbum2 = await trx.get(Album, album2Id);
      const yjs = serverAlbum2.inner.entity().getBackend(YjsBackend);
      yjs.delete('year', 0, 4); // delete "2020"
      yjs.insert('year', 0, '2019');
      await trx.commit();
    }

    expect((await clientCollection.dumpEntityEvents(album1Id)).length).toBe(0); // after edits
    expect((await clientCollection.dumpEntityEvents(album2Id)).length).toBe(0); // after edits

    // Fetch still needs sleeps for now
    await new Promise(resolve => setTimeout(resolve, 200));

    // Fetch again on the client
    const refetch = await clientCtx.fetch(Album, nocache("year >= '2020'"));
    const refetchYears = refetch.map((album: any) => album.year());

    // The client should receive only album1 with the correct state (year = "2020")
    // Album2 should not be returned since it no longer matches (year = "2019")
    expect(refetchYears).toEqual(['2020']);

    conn.destroy();
  });
});
