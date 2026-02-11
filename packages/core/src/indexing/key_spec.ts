// MIRRORS: ankurah/core/src/indexing/key_spec.rs

import type { PathExpr } from '@ankurah/ankql';
import { ValueType } from '../value/index.ts';

// ── IndexDirection ───────────────────────────────────────────────────
// Rust: `pub enum IndexDirection { Asc, Desc }`

export enum IndexDirection {
  Asc = 'Asc',
  Desc = 'Desc',
}

// ── NullsOrder ───────────────────────────────────────────────────────
// Rust: `pub enum NullsOrder { First, Last }`

export enum NullsOrder {
  First = 'First',
  Last = 'Last',
}

// ── IndexKeyPart ─────────────────────────────────────────────────────
// Rust: `pub struct IndexKeyPart { column, sub_path, direction, value_type, nulls, collation }`

export interface IndexKeyPart {
  column: string;
  /** Optional path within property value (for JSON, future Ref, etc.) */
  subPath: string[] | null;
  direction: IndexDirection;
  valueType: ValueType;
  nulls: NullsOrder | null;
  collation: string | null;
}

// ── IndexKeyPart factory methods ─────────────────────────────────────
// Rust: `impl IndexKeyPart { pub fn asc(...), pub fn desc(...), ... }`

export function indexKeyPartAsc(column: string, valueType: ValueType): IndexKeyPart {
  return { column, subPath: null, direction: IndexDirection.Asc, valueType, nulls: null, collation: null };
}

export function indexKeyPartDesc(column: string, valueType: ValueType): IndexKeyPart {
  return { column, subPath: null, direction: IndexDirection.Desc, valueType, nulls: null, collation: null };
}

/** Create from a PathExpr (handles multi-step paths). Rust: `pub fn from_path(...)` */
export function indexKeyPartFromPath(path: PathExpr, direction: IndexDirection, valueType: ValueType): IndexKeyPart {
  let column: string;
  let subPath: string[] | null;
  if (path.steps.length === 1) {
    column = path.steps[0];
    subPath = null;
  } else {
    column = path.steps[0];
    subPath = path.steps.slice(1);
  }
  return { column, subPath, direction, valueType, nulls: null, collation: null };
}

/** Full path as a flat string (e.g., "context.session_id"). Rust: `pub fn full_path(&self)` */
export function indexKeyPartFullPath(keypart: IndexKeyPart): string {
  if (keypart.subPath === null) {
    return keypart.column;
  }
  return [keypart.column, ...keypart.subPath].join('.');
}

/** Create from a flat path string (e.g., "context.session_id"). Rust: `pub fn from_flat_path(...)` */
export function indexKeyPartFromFlatPath(path: string, direction: IndexDirection, valueType: ValueType): IndexKeyPart {
  const parts = path.split('.');
  let column: string;
  let subPath: string[] | null;
  if (parts.length === 1) {
    column = parts[0];
    subPath = null;
  } else {
    column = parts[0];
    subPath = parts.slice(1);
  }
  return { column, subPath, direction, valueType, nulls: null, collation: null };
}

/** Create ascending keypart from flat path. Rust: `pub fn asc_path(...)` */
export function indexKeyPartAscPath(path: string, valueType: ValueType): IndexKeyPart {
  return indexKeyPartFromFlatPath(path, IndexDirection.Asc, valueType);
}

/** Create descending keypart from flat path. Rust: `pub fn desc_path(...)` */
export function indexKeyPartDescPath(path: string, valueType: ValueType): IndexKeyPart {
  return indexKeyPartFromFlatPath(path, IndexDirection.Desc, valueType);
}

// ── IndexDirection helpers ───────────────────────────────────────────

/** Rust: `pub fn is_desc(&self) -> bool` */
export function isDesc(direction: IndexDirection): boolean {
  return direction === IndexDirection.Desc;
}

// ── KeySpec ──────────────────────────────────────────────────────────
// Rust: `pub struct KeySpec { pub keyparts: Vec<IndexKeyPart> }`

export interface KeySpec {
  keyparts: IndexKeyPart[];
}

/** Create a new KeySpec. Rust: `pub fn new(...)` */
export function keySpecNew(keyparts: IndexKeyPart[]): KeySpec {
  return { keyparts };
}

/** Simple name generator. Rust: `pub fn name_with(&self, prefix, delim)` */
export function keySpecNameWith(keySpec: KeySpec, prefix: string, delim: string): string {
  const fields: string[] = keySpec.keyparts.map((k) => {
    const dir = k.direction === IndexDirection.Asc ? 'asc' : 'desc';
    const colName = indexKeyPartFullPath(k);
    if (k.collation !== null || k.nulls !== null) {
      const extras: string[] = [];
      if (k.collation !== null) {
        extras.push(`collate=${k.collation}`);
      }
      if (k.nulls !== null) {
        extras.push(`nulls=${k.nulls.toLowerCase()}`);
      }
      return `${colName} ${dir}(${extras.join(',')})`;
    }
    return `${colName} ${dir}`;
  });

  if (prefix === '') {
    return fields.join(delim);
  }
  return `${prefix}${delim}${fields.join(delim)}`;
}

// ── IndexSpecMatch ───────────────────────────────────────────────────
// Rust: `pub enum IndexSpecMatch { Match, Inverse }`

export enum IndexSpecMatch {
  Match = 'Match',
  Inverse = 'Inverse',
}

/**
 * Checks if this KeySpec can be satisfied by another KeySpec.
 * Returns Match if this is a prefix subset of other.
 * Returns Inverse if this is a prefix subset of other with all directions flipped.
 * Returns null if neither condition is met.
 *
 * Rust: `pub fn matches(&self, other: &KeySpec) -> Option<IndexSpecMatch>`
 */
export function keySpecMatches(self: KeySpec, other: KeySpec): IndexSpecMatch | null {
  if (self.keyparts.length > other.keyparts.length) {
    return null;
  }

  let directMatch = true;
  let inverseMatch = true;

  for (let i = 0; i < self.keyparts.length; i++) {
    const selfKeypart = self.keyparts[i];
    const otherKeypart = other.keyparts[i];

    // Both column and subPath must match
    if (selfKeypart.column !== otherKeypart.column) {
      return null;
    }
    // Compare subPath
    if (selfKeypart.subPath === null && otherKeypart.subPath !== null) return null;
    if (selfKeypart.subPath !== null && otherKeypart.subPath === null) return null;
    if (selfKeypart.subPath !== null && otherKeypart.subPath !== null) {
      if (selfKeypart.subPath.length !== otherKeypart.subPath.length) return null;
      for (let j = 0; j < selfKeypart.subPath.length; j++) {
        if (selfKeypart.subPath[j] !== otherKeypart.subPath[j]) return null;
      }
    }

    if (selfKeypart.direction !== otherKeypart.direction) {
      directMatch = false;
    }

    if (selfKeypart.direction === otherKeypart.direction) {
      inverseMatch = false;
    }
  }

  if (directMatch) {
    return IndexSpecMatch.Match;
  } else if (inverseMatch) {
    return IndexSpecMatch.Inverse;
  }
  return null;
}

/** Deep equality check for KeySpec. Rust: derives PartialEq. */
export function keySpecEquals(a: KeySpec, b: KeySpec): boolean {
  if (a.keyparts.length !== b.keyparts.length) return false;
  for (let i = 0; i < a.keyparts.length; i++) {
    const ak = a.keyparts[i];
    const bk = b.keyparts[i];
    if (ak.column !== bk.column) return false;
    if (ak.direction !== bk.direction) return false;
    if (ak.valueType !== bk.valueType) return false;
    if (ak.nulls !== bk.nulls) return false;
    if (ak.collation !== bk.collation) return false;
    // Compare subPath
    if (ak.subPath === null && bk.subPath !== null) return false;
    if (ak.subPath !== null && bk.subPath === null) return false;
    if (ak.subPath !== null && bk.subPath !== null) {
      if (ak.subPath.length !== bk.subPath.length) return false;
      for (let j = 0; j < ak.subPath.length; j++) {
        if (ak.subPath[j] !== bk.subPath[j]) return false;
      }
    }
  }
  return true;
}
