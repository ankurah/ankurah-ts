// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs

import type { QueryId } from '@ankurah/proto';

// ---------------------------------------------------------------------------
// QueryCandidate
// ---------------------------------------------------------------------------

/**
 * A query-specific view of candidates that borrows from CandidateChanges.
 *
 * Rust: `pub struct QueryCandidate<'a, C>`
 */
export class QueryCandidate<C> {
  readonly queryId: QueryId;
  private readonly changes: readonly C[];
  private readonly offsets: readonly number[];

  constructor(queryId: QueryId, changes: readonly C[], offsets: readonly number[]) {
    this.queryId = queryId;
    this.changes = changes;
    this.offsets = offsets;
  }

  /** Iterate over the candidate changes for this query. */
  iter(): C[] {
    return this.offsets.map((offset) => this.changes[offset]);
  }
}

// ---------------------------------------------------------------------------
// CandidateChanges
// ---------------------------------------------------------------------------

/**
 * Wraps a shared list of changes with per-query and per-entity offsets to avoid
 * cloning events during the three-phase notification pipeline.
 *
 * Rust: `pub struct CandidateChanges<C>`
 *
 * Divergences from Rust:
 * - `Arc<Vec<C>>` is just a readonly array (JS passes arrays by reference).
 * - `IVec<usize, 8>` (small-vec) is a plain `number[]`.
 * - `HashMap<QueryId, ...>` uses a string-keyed `Map` internally because JS
 *   Maps use reference equality for object keys, while Rust HashMaps use
 *   Hash+Eq. The ULID string of each QueryId serves as the stable key.
 */
export class CandidateChanges<C> {
  private readonly changes: readonly C[];

  /**
   * Internal map keyed by QueryId's ULID string for value-equality lookups.
   * Each entry stores the original QueryId alongside its offset list.
   */
  private readonly queryOffsets: Map<string, { queryId: QueryId; offsets: number[] }> = new Map();

  private readonly entityOffsets: number[] = [];

  constructor(changes: readonly C[]) {
    this.changes = changes;
  }

  // ── Mutators ────────────────────────────────────────────────────────

  /** Add an offset for an entity-level subscription (not tied to any query). */
  addEntity(offset: number): void {
    this.entityOffsets.push(offset);
  }

  /** Add an offset to the candidate list for a specific query. */
  addQuery(queryId: QueryId, offset: number): void {
    const key = queryId.toUlidString();
    let entry = this.queryOffsets.get(key);
    if (entry === undefined) {
      entry = { queryId, offsets: [] };
      this.queryOffsets.set(key, entry);
    }
    entry.offsets.push(offset);
  }

  // ── Accessors ───────────────────────────────────────────────────────

  /** Returns true if there are no candidates (neither query nor entity level). */
  isEmpty(): boolean {
    return this.queryOffsets.size === 0 && this.entityOffsets.length === 0;
  }

  /** Returns the number of distinct queries with candidates. */
  queryCount(): number {
    return this.queryOffsets.size;
  }

  /** Iterate over query candidates. */
  queryIter(): QueryCandidate<C>[] {
    const result: QueryCandidate<C>[] = [];
    for (const { queryId, offsets } of this.queryOffsets.values()) {
      result.push(new QueryCandidate<C>(queryId, this.changes, offsets));
    }
    return result;
  }

  /** Iterate over entity-level candidates. */
  entityIter(): C[] {
    return this.entityOffsets.map((offset) => this.changes[offset]);
  }

  /** Get direct access to the shared changes array. */
  getChanges(): readonly C[] {
    return this.changes;
  }
}
