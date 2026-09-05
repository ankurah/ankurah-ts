// MIRRORS: ankurah/ankql/tests/parser_fixtures.rs
//
// `parse_cases.json` records what the Rust `parse_selection` actually does to a
// corpus of query strings — not what SQL intuition says it should do, and the two
// differ in places that matter: `NOT IN` silently becomes `IN`, `AND` does not bind
// tighter than `OR`, `i32::MAX` widens to `I64` while `i32::MAX - 1` does not, and
// negative and fractional literals do not parse at all. A port has to land on the
// same side of every one of those lines or the two implementations disagree about
// what a stored query means.
//
// Each case is checked along four independent axes, each its own test, so a
// divergence in one does not hide the others:
//
//   parse      — the tree the parser builds, against the sidecar's `ast_json`
//   bincode    — that tree's encoding, against `ast_bincode_hex`. A port that builds
//                a semantically equal but structurally different tree passes the
//                `ast_json` check loosely and fails here.
//   SQL        — `generate_selection_sql(predicate)` against `predicate_sql`, and
//                `Display for Selection` against `roundtrip_sql`
//   reject     — the queries the parser refuses, and which ParseError it refuses with
//
// `error_message` is null for `SyntaxError` on purpose: that variant wraps pest's
// rendered diagnostic, which is pest's text and no port could reproduce it. The
// variant itself is the distinction worth implementing — `SyntaxError` means the
// grammar refused the text, every other variant means the grammar accepted it and
// AST construction then refused it.
//
// `parse_selection` and `generate_selection_sql` both answer a `Result`, so every
// case below asks the answer `isOk()` / `isErr()` rather than catching a throw.

import { describe, test, expect } from 'bun:test';

import { parseSelection } from '../src/parser';
import { generateSelectionSql } from '../src/selection/sql';
import { BincodeWriter } from '../src/codec';
import type { Selection } from '../src/ast';
import type { ParseError } from '../src/error';

import { readSidecar, toHex } from '../../proto/__tests__/support/fixtures';
import { toSerde } from '../../proto/__tests__/support/serde';

interface AcceptCase {
  query: string;
  note: string;
  ast_json: unknown;
  ast_bincode_hex: string;
  predicate_sql: string | null;
  predicate_sql_error: string | null;
  roundtrip_sql: string;
}

interface RejectCase {
  query: string;
  note: string;
  error_variant: string;
  error_message: string | null;
}

const cases = readSidecar('ankql/test_fixtures/parse_cases.json') as {
  accept_count: number;
  reject_count: number;
  accept: AcceptCase[];
  reject: RejectCase[];
};

/** A query as a test name: quoted, with whitespace visible so blank cases differ. */
function caseName(query: string): string {
  return JSON.stringify(query);
}

/** Render a parsed tree for a failure message. Bigints survive; JSON.stringify does not. */
function render(value: unknown): string {
  return JSON.stringify(value, (_k, v) => (typeof v === 'bigint' ? `${v}n` : v));
}

/** The Selection an accept case must parse to, with the Err reported rather than swallowed. */
function accepted(query: string): Selection {
  const result = parseSelection(query);
  if (result.isErr()) {
    using error = result.unwrapErr();
    throw new Error(`expected ${caseName(query)} to parse, got ${String(error)}`);
  }
  return result.unwrap();
}

test('the fixture case counts match', () => {
  expect(cases.accept.length).toBe(cases.accept_count);
  expect(cases.reject.length).toBe(cases.reject_count);
});

// ── accept: the tree the parser builds ──────────────────────────────────────

describe('parse: AST matches the fixture', () => {
  for (const c of cases.accept) {
    test(`${caseName(c.query)} — ${c.note}`, () => {
      using selection = accepted(c.query);
      expect(toSerde(selection)).toEqual(c.ast_json as any);
    });
  }
});

// ── accept: how that tree encodes ───────────────────────────────────────────

describe('bincode: AST encoding matches the fixture hex', () => {
  for (const c of cases.accept) {
    test(`${caseName(c.query)} — ${c.note}`, () => {
      using selection = accepted(c.query);
      const writer = new BincodeWriter();
      selection.encode(writer);
      expect(toHex(writer.finish())).toBe(c.ast_bincode_hex);
    });
  }
});

// ── accept: SQL generation ──────────────────────────────────────────────────

describe('SQL: generate_selection_sql matches the fixture', () => {
  for (const c of cases.accept) {
    test(`${caseName(c.query)} — ${c.note}`, () => {
      using selection = accepted(c.query);
      const result = generateSelectionSql(selection.predicate, null);

      if (c.predicate_sql_error !== null) {
        // The fixture says SQL generation fails here, with this text.
        if (result.isOk()) {
          throw new Error(`expected SQL generation to fail with ${JSON.stringify(c.predicate_sql_error)}, got ${render(result.unwrap())}`);
        }
        using error = result.unwrapErr();
        expect(String(error)).toBe(c.predicate_sql_error);
        return;
      }

      if (result.isErr()) {
        using error = result.unwrapErr();
        throw new Error(`expected SQL, got ${String(error)}`);
      }
      expect(result.unwrap()).toBe(c.predicate_sql as string);
    });
  }
});

describe('SQL: Display for Selection matches roundtrip_sql', () => {
  for (const c of cases.accept) {
    test(`${caseName(c.query)} — ${c.note}`, () => {
      // Rust's `Display for Predicate` swallows generation errors and prints
      // "SQL Error: …" in their place, so `roundtrip_sql` is not always valid SQL.
      using selection = accepted(c.query);
      expect(selection.toString()).toBe(c.roundtrip_sql);
    });
  }
});

// ── reject ──────────────────────────────────────────────────────────────────

describe('reject: the parser refuses, with the fixture variant', () => {
  for (const c of cases.reject) {
    test(`${caseName(c.query)} — ${c.note}`, () => {
      const result = parseSelection(c.query);
      if (result.isOk()) {
        using selection = result.unwrap();
        throw new Error(`expected ${c.error_variant}, but the query parsed to ${render(toSerde(selection))}`);
      }
      using error = result.unwrapErr();
      expect(error.type).toBe(c.error_variant as ParseError['type']);
    });
  }
});

describe('reject: the error text matches the fixture', () => {
  for (const c of cases.reject) {
    // SyntaxError carries pest's rendered diagnostic, which the fixture
    // deliberately does not pin. The variant check above still covers those.
    if (c.error_message === null) continue;
    test(`${caseName(c.query)} — ${c.note}`, () => {
      const result = parseSelection(c.query);
      if (result.isOk()) {
        using selection = result.unwrap();
        throw new Error(`expected an error, but the query parsed to ${render(toSerde(selection))}`);
      }
      using error = result.unwrapErr();
      expect(String(error)).toBe(c.error_message as string);
    });
  }
});
