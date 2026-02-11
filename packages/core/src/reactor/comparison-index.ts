// MIRRORS: ankurah/core/src/reactor/comparison_index.rs

import type { ComparisonOperator, Literal } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import {
  valueFromLiteral,
  valueCollatableToBytes,
  valueCollatableSuccessorBytes,
  valueCollatablePredecessorBytes,
} from '../value/index.ts';

// ── Byte-array helpers ──────────────────────────────────────────────

/** Hex-encode a Uint8Array so it can be used as a Map key (JS Maps use reference equality for objects). */
function bytesToKey(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i++) {
    s += bytes[i].toString(16).padStart(2, '0');
  }
  return s;
}

/** Lexicographic comparison of two Uint8Arrays. Returns -1, 0, or 1. */
function compareBytes(a: Uint8Array, b: Uint8Array): number {
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  if (a.length < b.length) return -1;
  if (a.length > b.length) return 1;
  return 0;
}

// ── Literal → collatable bytes helpers ──────────────────────────────

/** Convert a Literal to collatable bytes (Literal → Value → bytes). */
function literalToCollatableBytes(literal: Literal): Uint8Array {
  return valueCollatableToBytes(valueFromLiteral(literal));
}

/** Get the predecessor bytes for a Literal (Literal → Value → predecessor bytes). */
function literalPredecessorBytes(literal: Literal): Uint8Array | null {
  return valueCollatablePredecessorBytes(valueFromLiteral(literal));
}

/** Get the successor bytes for a Literal (Literal → Value → successor bytes). */
function literalSuccessorBytes(literal: Literal): Uint8Array | null {
  return valueCollatableSuccessorBytes(valueFromLiteral(literal));
}

// ── ComparisonIndex ─────────────────────────────────────────────────

/**
 * An index for a specific field and comparison operator.
 * Used for storage engines that don't offer watchable indexes.
 *
 * This is a naive implementation that uses Maps for each operator.
 * Not efficient for large datasets — if this ends up being used in production
 * we should consider a more efficient index structure like a B+ tree with
 * subscription registrations on intermediate nodes for range comparisons.
 */
export class ComparisonIndex<T> {
  /** Exact-match: collated-bytes-key → subscribers */
  private eq: Map<string, T[]> = new Map();

  /** Not-equal: collated-bytes-key → subscribers */
  private ne: Map<string, T[]> = new Map();

  /**
   * Greater-than: entries sorted by collated bytes.
   * Each entry is [raw bytes, hex key, subscribers].
   * Kept sorted in ascending byte order for range scans.
   */
  private gt: Array<{ bytes: Uint8Array; key: string; subs: T[] }> = [];

  /**
   * Less-than: entries sorted by collated bytes.
   * Each entry is [raw bytes, hex key, subscribers].
   * Kept sorted in ascending byte order for range scans.
   */
  private lt: Array<{ bytes: Uint8Array; key: string; subs: T[] }> = [];

  // ── Private helpers ─────────────────────────────────────────────

  /** Get or create an entry in a sorted array (gt or lt), maintaining sort order. */
  private getOrInsertSorted(
    arr: Array<{ bytes: Uint8Array; key: string; subs: T[] }>,
    bytes: Uint8Array,
  ): T[] {
    const key = bytesToKey(bytes);

    // Binary search for the position
    let lo = 0;
    let hi = arr.length;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      const cmp = compareBytes(arr[mid].bytes, bytes);
      if (cmp < 0) {
        lo = mid + 1;
      } else if (cmp > 0) {
        hi = mid;
      } else {
        // Found existing entry
        return arr[mid].subs;
      }
    }

    // Not found — insert at `lo`
    const entry = { bytes: new Uint8Array(bytes), key, subs: [] as T[] };
    arr.splice(lo, 0, entry);
    return entry.subs;
  }

  /**
   * Access the subscriber list for a given operator + collated bytes,
   * creating it if necessary. Calls `f` with the mutable subscriber list.
   */
  private forEntry(
    collatableBytes: Uint8Array,
    op: ComparisonOperator,
    f: (entries: T[]) => void,
    literal: Literal,
  ): void {
    switch (op) {
      case 'Equal': {
        const key = bytesToKey(collatableBytes);
        let entries = this.eq.get(key);
        if (!entries) {
          entries = [];
          this.eq.set(key, entries);
        }
        f(entries);
        break;
      }
      case 'NotEqual': {
        const key = bytesToKey(collatableBytes);
        let entries = this.ne.get(key);
        if (!entries) {
          entries = [];
          this.ne.set(key, entries);
        }
        f(entries);
        break;
      }
      case 'GreaterThan': {
        const entries = this.getOrInsertSorted(this.gt, collatableBytes);
        f(entries);
        break;
      }
      case 'LessThan': {
        const entries = this.getOrInsertSorted(this.lt, collatableBytes);
        f(entries);
        break;
      }
      case 'GreaterThanOrEqual': {
        // x >= threshold is equivalent to x > predecessor(threshold)
        const pred = literalPredecessorBytes(literal);
        if (pred !== null) {
          const entries = this.getOrInsertSorted(this.gt, pred);
          f(entries);
        } else {
          // No predecessor (value is minimum) — matches everything
          const entries = this.getOrInsertSorted(this.gt, new Uint8Array(0));
          f(entries);
        }
        break;
      }
      case 'LessThanOrEqual': {
        // x <= threshold is equivalent to x < successor(threshold)
        const succ = literalSuccessorBytes(literal);
        if (succ !== null) {
          const entries = this.getOrInsertSorted(this.lt, succ);
          f(entries);
        }
        // If no successor exists, the condition can never be satisfied
        // beyond the threshold, so we don't add anything.
        break;
      }
      default:
        throw new Error(`Unsupported operator: ${op}`);
    }
  }

  // ── Public API ──────────────────────────────────────────────────

  /** Add a subscriber for the given comparison operator and literal threshold. */
  add(literal: Literal, op: ComparisonOperator, subscriberId: T): void {
    const bytes = literalToCollatableBytes(literal);
    this.forEntry(bytes, op, (entries) => entries.push(subscriberId), literal);
  }

  /** Remove a subscriber for the given comparison operator and literal threshold. */
  remove(
    literal: Literal,
    op: ComparisonOperator,
    subscriberId: T,
  ): void {
    const bytes = literalToCollatableBytes(literal);
    this.forEntry(
      bytes,
      op,
      (entries) => {
        const pos = entries.indexOf(subscriberId);
        if (pos !== -1) {
          entries.splice(pos, 1);
        }
      },
      literal,
    );
  }

  /**
   * Find all subscribers whose conditions match the given probe value.
   *
   * Logic:
   * - eq: exact byte match on probe
   * - ne: all entries EXCEPT the one whose bytes match probe
   * - gt: all entries where threshold < probe (threshold bytes < probe bytes)
   * - lt: all entries where threshold > probe (threshold bytes > probe bytes)
   */
  findMatching(probeValue: Value): T[] {
    const probeBytes = valueCollatableToBytes(probeValue);
    const probeKey = bytesToKey(probeBytes);
    const seen = new Set<T>();
    const result: T[] = [];

    const addUnique = (id: T) => {
      if (!seen.has(id)) {
        seen.add(id);
        result.push(id);
      }
    };

    // Check exact matches (eq)
    const eqSubs = this.eq.get(probeKey);
    if (eqSubs) {
      for (const id of eqSubs) {
        addUnique(id);
      }
    }

    // Check not-equal — iterate all != conditions; include those whose
    // stored bytes differ from probe
    for (const [storedKey, subs] of this.ne) {
      if (probeKey !== storedKey) {
        for (const id of subs) {
          addUnique(id);
        }
      }
    }

    // Check greater-than (x > threshold): subscriber matches when
    // probe > threshold, i.e. threshold < probe.
    // gt array is sorted ascending by threshold bytes.
    // All entries with threshold bytes < probeBytes match.
    for (const entry of this.gt) {
      if (compareBytes(entry.bytes, probeBytes) < 0) {
        for (const id of entry.subs) {
          addUnique(id);
        }
      } else {
        // Since gt is sorted ascending, once we hit threshold >= probe
        // all remaining will also be >=, so we can break.
        break;
      }
    }

    // Check less-than (x < threshold): subscriber matches when
    // probe < threshold, i.e. threshold > probe.
    // lt array is sorted ascending by threshold bytes.
    // All entries with threshold bytes > probeBytes match.
    // We need successor bytes of probe to find the start of matching range.
    const probeSucc = valueCollatableSuccessorBytes(probeValue);
    if (probeSucc !== null) {
      // Find the first entry in lt where threshold >= probeSucc
      // (since threshold > probe is equivalent to threshold >= successor(probe))
      for (const entry of this.lt) {
        if (compareBytes(entry.bytes, probeSucc) >= 0) {
          for (const id of entry.subs) {
            addUnique(id);
          }
        }
      }
    }

    return result;
  }
}
