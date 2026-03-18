// MIRRORS: ankurah/storage/postgres/tests/common/mod.rs
//
// Common utilities for Postgres storage integration tests.
//
// Divergence: Rust uses testcontainers with Docker for ephemeral Postgres instances.
// TS expects a running Postgres instance via POSTGRES_URL env var [E8].
//
// To run integration tests:
//   POSTGRES_URL="host=localhost port=5432 user=postgres password=postgres dbname=ankurah_test" bun test packages/storage-postgres/__tests__/

import { Postgres, type PostgresPool, type PostgresClient, type PostgresQueryResult } from '../src/index.ts';

// Re-export for tests
export { Postgres };

/**
 * Get the Postgres connection URL from environment.
 * Returns null if POSTGRES_URL is not set (tests should skip).
 */
export function getPostgresUrl(): string | null {
  return process.env['POSTGRES_URL'] ?? null;
}

/**
 * Check if Postgres integration tests should run.
 * Tests are skipped unless POSTGRES_URL is set.
 */
export function shouldRunPgTests(): boolean {
  return getPostgresUrl() !== null;
}

/**
 * Create a Postgres storage engine connected to the test database.
 * Requires POSTGRES_URL environment variable.
 *
 * Divergence: Rust creates a Docker container. TS connects to an existing
 * Postgres instance via environment variable [E8].
 */
export async function createPostgresStorage(): Promise<Postgres> {
  const url = getPostgresUrl();
  if (!url) {
    throw new Error('POSTGRES_URL environment variable is required for integration tests');
  }

  // Divergence: TS requires a PostgresPool adapter for the chosen PG driver.
  // This is a placeholder that must be connected to an actual PG client [E8].
  throw new Error(
    'createPostgresStorage: not yet implemented — needs a concrete PostgresPool adapter for pg/postgres.js driver',
  );
}
