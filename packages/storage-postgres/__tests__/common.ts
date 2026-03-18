// MIRRORS: ankurah/storage/postgres/tests/common/mod.rs
//
// Common utilities for Postgres storage integration tests.
// Uses testcontainers to spin up an ephemeral Postgres container per test,
// matching the Rust test infrastructure exactly.

import { GenericContainer, Wait, type StartedTestContainer } from 'testcontainers';
import pg from 'pg';
import { Postgres, type PostgresPool, type PostgresClient, type PostgresQueryResult } from '../src/index.ts';
import {
  Node,
  PermissiveAgent,
  defineModel,
  yrsText,
  lww,
} from '@ankurah/core';
import type { ChangeSet, ChangeKind, ItemChange, ViewInstance } from '@ankurah/core';

// Re-export for tests
export { Postgres };

// ── pg driver adapter ─────────────────────────────────────────────────

/** Wrap a node-postgres Pool as the PostgresPool interface expected by our Postgres engine.
 * Each getClient() call checks out a connection that is automatically released after each query.
 * This mirrors Rust bb8 pool behavior where connections are returned when dropped. */
function pgPoolAdapter(pool: pg.Pool): PostgresPool {
  return {
    async getClient(): Promise<PostgresClient> {
      // Return a client that acquires/releases a connection per query operation.
      // This prevents connection leaks that would cause pool.end() to hang.
      return {
        async query(sql: string, params?: unknown[]): Promise<PostgresQueryResult> {
          const client = await pool.connect();
          try {
            const result = await client.query(sql, params ?? []);
            return { rows: result.rows, rowCount: result.rowCount ?? 0 };
          } finally {
            client.release();
          }
        },
        async queryOne(sql: string, params?: unknown[]): Promise<Record<string, unknown>> {
          const client = await pool.connect();
          try {
            const result = await client.query(sql, params ?? []);
            if (result.rows.length === 0) {
              throw new Error('query returned an unexpected number of rows');
            }
            return result.rows[0];
          } finally {
            client.release();
          }
        },
      };
    },
  };
}

// ── pg_init.sql ───────────────────────────────────────────────────────
// Mirrors: ankurah/storage/postgres/tests/pg_init.sql
// The Rust tests use this to initialize extensions. We run the same SQL
// after container startup.

const PG_INIT_SQL = `
  CREATE EXTENSION IF NOT EXISTS hstore;
  CREATE EXTENSION IF NOT EXISTS citext;
  CREATE EXTENSION IF NOT EXISTS ltree;
`;

// ── Container helper ──────────────────────────────────────────────────

export interface PostgresTestContext {
  container: StartedTestContainer;
  engine: Postgres;
  pool: pg.Pool;
}

/**
 * Create a Postgres container + storage engine for integration tests.
 * Mirrors: common::create_postgres_container() in Rust.
 */
export async function createPostgresContainer(): Promise<PostgresTestContext> {
  const container = await new GenericContainer('postgres:16')
    .withEnvironment({
      POSTGRES_USER: 'postgres',
      POSTGRES_PASSWORD: 'postgres',
      POSTGRES_DB: 'ankurah',
    })
    .withExposedPorts(5432)
    .withWaitStrategy(Wait.forLogMessage('database system is ready to accept connections', 2))
    .start();

  const host = container.getHost();
  const port = container.getMappedPort(5432);

  const pool = new pg.Pool({
    host,
    port,
    database: 'ankurah',
    user: 'postgres',
    password: 'postgres',
  });

  // Run pg_init.sql
  const client = await pool.connect();
  try {
    await client.query(PG_INIT_SQL);
  } finally {
    client.release();
  }

  const engine = new Postgres(pgPoolAdapter(pool));

  return { container, engine, pool };
}

/**
 * Stop a container and drain its pool.
 */
export async function stopPostgresContainer(ctx: PostgresTestContext): Promise<void> {
  if (!ctx) return;
  await ctx.pool.end();
  await ctx.container.stop();
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

/** Mirrors: undefined_column.rs `struct Task { name: String, status: String, created: String }` */
export const Task = defineModel('task', {
  name: yrsText(),
  status: yrsText(),
  created: yrsText(),
});

/** Mirrors: property_backends.rs `struct Video { title: YrsString, description: Option<String>, visibility: LWW, attribution: Option<String> }` */
export const Video = defineModel('video', {
  title: yrsText(),
  description: lww<string | null>(),
  visibility: lww<string>(),
  attribution: lww<string | null>(),
});

/** Mirrors: predicate_checks.rs `struct QueryTest { label: String, data: Json }` */
export const QueryTest = defineModel('query_test', {
  label: yrsText(),
  data: lww<unknown>(),
});

// ── Test helper: create Node with Postgres engine ─────────────────────

export function createPostgresNode(engine: Postgres): Node {
  const node = new Node({
    storageEngine: engine,
    policyAgent: new PermissiveAgent(),
    durable: true,
  });
  return node;
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
