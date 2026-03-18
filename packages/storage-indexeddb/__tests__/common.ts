// MIRRORS: ankurah/storage/indexeddb-wasm/tests/common/mod.rs
//
// Test helpers for IndexedDB integration tests.
// Divergence: Uses fake-indexeddb instead of browser IndexedDB [E16].

import 'fake-indexeddb/auto';

import { IndexedDBStorageEngine } from '../src/index.ts';
import {
  Node,
  PermissiveAgent,
  defineModel,
  yrsText,
  lww,
  matchArgs,
} from '@ankurah/core';
import type { ChangeSet, ChangeKind, ItemChange, ViewInstance } from '@ankurah/core';

// ── Models ────────────────────────────────────────────────────────────

/** Mirrors: common.rs `struct Album { name: String, year: String }` */
export const Album = defineModel('album', {
  name: lww<string>(),
  year: yrsText(),
});

/** Mirrors: common.rs `struct Book { name: String, year: String }` */
export const Book = defineModel('book', {
  name: lww<string>(),
  year: yrsText(),
});

/** Mirrors: common.rs `struct Event { name: String, timestamp: i64, active: bool }` */
export const Event = defineModel('event', {
  name: lww<string>(),
  timestamp: lww<number>(),
  active: lww<boolean>(),
});

/** Mirrors: desc_ordering.rs `struct LogEvent { category: String, timestamp: i64, level: String }` */
export const LogEvent = defineModel('logevent', {
  category: yrsText(),
  timestamp: lww<number>(),
  level: yrsText(),
});

/** Mirrors: desc_ordering.rs `struct Message { room: String, deleted: bool, timestamp: i64, text: String }` */
export const Message = defineModel('message', {
  room: yrsText(),
  deleted: lww<boolean>(),
  timestamp: lww<number>(),
  text: yrsText(),
});

/** Mirrors: json_property.rs `struct Track { name: String, licensing: Json }` */
export const Track = defineModel('track', {
  name: lww<string>(),
  licensing: lww<unknown>(),
});

/** Mirrors: multi_column_order_by.rs `struct Product { category: String, name: String, price: i64, stock: i64 }` */
export const Product = defineModel('product', {
  category: yrsText(),
  name: yrsText(),
  price: lww<number>(),
  stock: lww<number>(),
});

/** Mirrors: predicate_checks.rs `struct QueryTest { label: String, data: Json }` */
export const QueryTest = defineModel('querytest', {
  label: yrsText(),
  data: lww<unknown>(),
});

// ── Helper functions ──────────────────────────────────────────────────

/** Extract names from Album views */
export function names(items: any[]): string[] {
  return items.map((a: any) => a.name());
}

/** Extract names sorted from Album views */
export function sortNames(items: any[]): string[] {
  const n = names(items);
  n.sort();
  return n;
}

/** Extract years from Album views */
export function years(items: any[]): string[] {
  return items.map((a: any) => a.year());
}

/** Extract timestamps from Event/LogEvent views */
export function eventTimestamps(items: any[]): number[] {
  return items.map((e: any) => e.timestamp());
}

/** Assert timestamps are in DESC order */
export function assertDescOrder(timestamps: number[], context: string): void {
  for (let i = 0; i < timestamps.length - 1; i++) {
    if (timestamps[i] < timestamps[i + 1]) {
      throw new Error(`${context}: Expected DESC order, got ${JSON.stringify(timestamps)}`);
    }
  }
}

/** Assert timestamps are in ASC order */
export function assertAscOrder(timestamps: number[], context: string): void {
  for (let i = 0; i < timestamps.length - 1; i++) {
    if (timestamps[i] > timestamps[i + 1]) {
      throw new Error(`${context}: Expected ASC order, got ${JSON.stringify(timestamps)}`);
    }
  }
}

// ── Test helper: create Node with IndexedDB engine ────────────────────

let dbCounter = 0;

export async function createIndexedDBNode(): Promise<{ node: Node; dbName: string }> {
  const dbName = `test_db_${Date.now()}_${dbCounter++}`;
  const engine = await IndexedDBStorageEngine.open(dbName);
  const node = new Node({
    storageEngine: engine,
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  // Mirrors Rust: node.system.create().await?
  // Must wait for the system catalog to load + create before using the node.
  // This ensures IndexedDB index upgrades complete before test operations begin.
  await node.system.create();
  return { node, dbName };
}

// ── Batch create helpers ─────────────────────────────────────────────

export async function createAlbums(ctx: any, albums: [string, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [name, year] of albums) {
    await trx.create(Album, { name, year });
  }
  await trx.commit();
}

export async function createBooks(ctx: any, books: [string, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [name, year] of books) {
    await trx.create(Book, { name, year });
  }
  await trx.commit();
}

export async function createEvents(ctx: any, events: [string, number, boolean][]): Promise<void> {
  const trx = ctx.begin();
  for (const [name, timestamp, active] of events) {
    await trx.create(Event, { name, timestamp, active });
  }
  await trx.commit();
}

export async function createLogEvents(ctx: any, events: [string, number, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [category, timestamp, level] of events) {
    await trx.create(LogEvent, { category, timestamp, level });
  }
  await trx.commit();
}

export async function createMessages(ctx: any, messages: [string, boolean, number, string][]): Promise<void> {
  const trx = ctx.begin();
  for (const [room, deleted, timestamp, text] of messages) {
    await trx.create(Message, { room, deleted, timestamp, text });
  }
  await trx.commit();
}

export async function createProducts(ctx: any, products: [string, string, number, number][]): Promise<void> {
  const trx = ctx.begin();
  for (const [category, name, price, stock] of products) {
    await trx.create(Product, { category, name, price, stock });
  }
  await trx.commit();
}

export async function createTracks(ctx: any, tracks: [string, unknown][]): Promise<void> {
  const trx = ctx.begin();
  for (const [name, licensing] of tracks) {
    await trx.create(Track, { name, licensing });
  }
  await trx.commit();
}

// Re-export for convenience
export { matchArgs, IndexedDBStorageEngine };
