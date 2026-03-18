// MIRRORS: ankurah/storage/sqlite/tests/common/mod.rs
//
// Test helpers for SQLite integration tests.
// Divergence: Uses bun:sqlite instead of rusqlite [E16].

import { Database } from 'bun:sqlite';
import type { SqliteDriver } from '../src/index.ts';
import { SqliteStorageEngine } from '../src/index.ts';
import {
  Node,
  PermissiveAgent,
  defineModel,
  yrsText,
  lww,
} from '@ankurah/core';
import type { ChangeSet, ChangeKind, ItemChange, ViewInstance } from '@ankurah/core';

// ── bun:sqlite driver adapter ─────────────────────────────────────────

export function bunSqliteDriver(path: string = ':memory:'): SqliteDriver {
  const db = new Database(path);

  // Performance optimizations matching Rust connection.rs
  db.exec(
    `PRAGMA journal_mode=WAL;
     PRAGMA synchronous=NORMAL;
     PRAGMA foreign_keys=ON;
     PRAGMA cache_size=-64000;
     PRAGMA mmap_size=268435456;
     PRAGMA temp_store=MEMORY;`,
  );

  return {
    execute(sql: string, params: unknown[] = []): number {
      const stmt = db.prepare(sql);
      const result = stmt.run(...params as any[]);
      return result.changes;
    },
    query<T = Record<string, unknown>>(sql: string, params: unknown[] = []): T[] {
      const stmt = db.prepare(sql);
      return stmt.all(...params as any[]) as T[];
    },
    queryOne<T = Record<string, unknown>>(sql: string, params: unknown[] = []): T | null {
      const stmt = db.prepare(sql);
      const row = stmt.get(...params as any[]);
      return (row as T) ?? null;
    },
    close(): void {
      db.close();
    },
  };
}

// ── Models ────────────────────────────────────────────────────────────

/** Mirrors: common.rs `struct Album { name: String, year: String }` */
export const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

/** Mirrors: json_property.rs `struct Track { name: String, licensing: Json }` */
export const Track = defineModel('track', {
  name: yrsText(),
  licensing: lww<unknown>(),
});

/** Mirrors: sqlite_undefined_column.rs `struct Task { name: String, status: String, created: String }` */
export const Task = defineModel('task', {
  name: yrsText(),
  status: yrsText(),
  created: yrsText(),
});

// ── Test helper: create Node with SQLite engine ───────────────────────

export function createSqliteNode(): { node: Node; driver: SqliteDriver } {
  const driver = bunSqliteDriver();
  const engine = new SqliteStorageEngine(driver);
  const node = new Node({
    storageEngine: engine,
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  return { node, driver };
}

// ── TestWatcher (changeset) ────────────────────────────────────────────
// Mirrors: common.rs TestWatcher<ChangeSet<R>, Vec<(EntityId, ChangeKind)>>

export class TestWatcher {
  private batches: Array<Array<[string, ChangeKind]>> = [];
  private resolvers: Array<() => void> = [];

  listener(): (changeset: ChangeSet<ViewInstance>) => void {
    return (changeset: ChangeSet<ViewInstance>) => {
      const batch: Array<[string, ChangeKind]> = changeset.changes.map(
        (change: ItemChange<ViewInstance>) => [change.item.id().toBase64(), change.kind] as [string, ChangeKind],
      );
      this.batches.push(batch);
      for (const resolve of this.resolvers) resolve();
      this.resolvers = [];
    };
  }

  async wait(timeoutMs = 10000): Promise<boolean> {
    if (this.batches.length > 0) return true;
    return new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), timeoutMs);
      this.resolvers.push(() => {
        clearTimeout(timer);
        resolve(true);
      });
    });
  }

  count(): number {
    return this.batches.length;
  }

  drain(): Array<Array<[string, ChangeKind]>> {
    return this.batches.splice(0);
  }

  async takeOne(timeoutMs = 10000): Promise<Array<[string, ChangeKind]>> {
    if (this.batches.length > 0) {
      return this.batches.splice(0, 1)[0];
    }
    const ok = await this.wait(timeoutMs);
    if (!ok || this.batches.length === 0) {
      throw new Error(`takeOne() timed out waiting (waited ${timeoutMs}ms, got ${this.batches.length} items)`);
    }
    return this.batches.splice(0, 1)[0];
  }

  async quiesce(): Promise<number> {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return this.batches.length;
  }
}
