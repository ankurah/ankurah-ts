// MIRRORS: ankurah/core/src/reactor/comparison_index.rs

import { ComparisonOperator, Literal } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import {
  valueFromLiteral,
  valueCollatableToBytes,
  valueCollatableSuccessorBytes,
  valueCollatablePredecessorBytes,
} from '../value/index.ts';

// ── Byte-array helpers ──────────────────────────────────────────────
// Divergence: Rust uses Vec<u8> as HashMap/BTreeMap keys directly;
// JS Maps use reference equality for objects, so we hex-encode to string keys [E8]

/** Hex-encode a Uint8Array so it can be used as a Map key. */
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
// Divergence: Rust Collatable trait is called directly on Literal/Value;
// TS uses free functions since Literal/Value are not class-based [E7]

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
 *
 * Rust: `pub(crate) struct ComparisonIndex<T>`
 * Divergence: HashMap → Map<string,...>, BTreeMap → sorted array with binary search [E8]
 */
export class ComparisonIndex<T> {
  /** Exact-match: collated-bytes-key → subscribers. Rust: `eq: HashMap<Vec<u8>, Vec<T>>` */
  private eq: Map<string, T[]> = new Map();

  /** Not-equal: collated-bytes-key → subscribers. Rust: `ne: HashMap<Vec<u8>, Vec<T>>` */
  private ne: Map<string, T[]> = new Map();

  /**
   * Greater-than: entries sorted by collated bytes.
   * Rust: `gt: BTreeMap<Vec<u8>, Vec<T>>`
   * Divergence: BTreeMap → sorted array with binary search [E8]
   */
  private gt: Array<{ bytes: Uint8Array; key: string; subs: T[] }> = [];

  /**
   * Less-than: entries sorted by collated bytes.
   * Rust: `lt: BTreeMap<Vec<u8>, Vec<T>>`
   * Divergence: BTreeMap → sorted array with binary search [E8]
   */
  private lt: Array<{ bytes: Uint8Array; key: string; subs: T[] }> = [];

  // ── impl Default ──

  // Default constructor (fields initialized inline above)

  // ── impl ComparisonIndex ──

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
   *
   * Rust: `fn for_entry<F, V>(&mut self, value: V, op: ast::ComparisonOperator, f: F)`
   */
  private forEntry(
    collatableBytes: Uint8Array,
    op: ComparisonOperator,
    f: (entries: T[]) => void,
    literal: Literal,
  ): void {
    switch (op.type) {
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
        break;
      }
      default:
        throw new Error(`Unsupported operator: ${op.type}`);
    }
  }

  /** Rust: `pub fn add<V: Collatable>(&mut self, value: V, op: ast::ComparisonOperator, watcher_id: T)` */
  add(literal: Literal, op: ComparisonOperator, subscriberId: T): void {
    const bytes = literalToCollatableBytes(literal);
    this.forEntry(bytes, op, (entries) => entries.push(subscriberId), literal);
  }

  /** Rust: `pub fn remove<V: Collatable>(&mut self, value: V, op: ast::ComparisonOperator, watcher_id: T)` */
  remove(literal: Literal, op: ComparisonOperator, subscriberId: T): void {
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
   * Rust: `pub fn find_matching<V: Collatable>(&self, value: V) -> BTreeSet::IntoIter<T>`
   * Divergence: Returns sorted, deduplicated array instead of BTreeSet iterator [E8]
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

    // Check exact matches
    const eqSubs = this.eq.get(probeKey);
    if (eqSubs) {
      for (const id of eqSubs) {
        addUnique(id);
      }
    }

    // Check not equal - iterate through all != conditions
    for (const [storedKey, subs] of this.ne) {
      if (probeKey !== storedKey) {
        for (const id of subs) {
          addUnique(id);
        }
      }
    }

    // Check greater than matches (x > threshold)
    // gt array is sorted ascending by threshold bytes.
    // All entries with threshold bytes < probeBytes match.
    for (const entry of this.gt) {
      if (compareBytes(entry.bytes, probeBytes) < 0) {
        for (const id of entry.subs) {
          addUnique(id);
        }
      } else {
        break;
      }
    }

    // Check less than matches (x < threshold)
    // lt array is sorted ascending by threshold bytes.
    // All entries with threshold bytes > probeBytes match.
    const probeSucc = valueCollatableSuccessorBytes(probeValue);
    if (probeSucc !== null) {
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
