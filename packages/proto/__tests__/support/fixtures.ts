// TS-ONLY: shared locator for the cross-language fixture tree.
//
// The fixtures live in the Rust support tree (`ankurah-ts-support`), which is a
// sibling checkout rather than part of this repo. Every fixture consumer in
// every package resolves it through this module so there is exactly one place
// that knows where that tree is and exactly one error message when it is
// missing.
//
// Resolution order:
//   1. $ANKURAH_SUPPORT — an explicit path, for CI and for checkouts that put
//      the support tree somewhere unusual.
//   2. A walk up from this file looking for `ankurah-ts-support` beside the
//      repo, or the `.claude/worktrees/ankurah-ts-support` symlink that makes
//      the tree reachable from any worktree.
// A tree is accepted only if `proto/test_fixtures` is inside it, so a stale
// empty directory fails here rather than as a confusing per-fixture error.

import { existsSync, readFileSync, readdirSync } from 'fs';
import path from 'path';

const MARKER = path.join('proto', 'test_fixtures');

function isSupportTree(dir: string): boolean {
  return existsSync(path.join(dir, MARKER));
}

function search(): string | null {
  const fromEnv = process.env.ANKURAH_SUPPORT;
  if (fromEnv && fromEnv.length > 0) {
    const resolved = path.resolve(fromEnv);
    if (isSupportTree(resolved)) return resolved;
    throw new Error(
      `ANKURAH_SUPPORT is set to ${resolved} but that directory has no ${MARKER}. ` +
      `Point it at an ankurah-ts-support checkout.`,
    );
  }

  let dir = import.meta.dir;
  for (;;) {
    for (const candidate of [
      path.join(dir, 'ankurah-ts-support'),
      path.join(dir, '.claude', 'worktrees', 'ankurah-ts-support'),
    ]) {
      if (isSupportTree(candidate)) return candidate;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

let cached: string | null | undefined;

/** Absolute path of the ankurah-ts-support checkout. Throws if it cannot be found. */
export function supportRoot(): string {
  if (cached === undefined) cached = search();
  if (cached === null) {
    throw new Error(
      'ankurah-ts-support checkout not found. The cross-language fixtures live there, ' +
      'not in this repo. Either clone it beside this repo (../ankurah-ts-support), ' +
      'link it at .claude/worktrees/ankurah-ts-support, or set ANKURAH_SUPPORT to its path. ' +
      'Regenerate the fixtures inside it with: ' +
      'OVERWRITE_FIXTURES=1 cargo test -p ankurah-proto --test bincode_fixtures',
    );
  }
  return cached;
}

/** Absolute path of a file inside the support tree, e.g. fixturePath('proto/test_fixtures/ids.bin'). */
export function fixturePath(...segments: string[]): string {
  return path.join(supportRoot(), ...segments);
}

/** Read a fixture's bytes. Throws naming the file if it is not there. */
export function readFixtureBytes(...segments: string[]): Uint8Array {
  const file = fixturePath(...segments);
  if (!existsSync(file)) throw new Error(`Fixture not found: ${file}`);
  return new Uint8Array(readFileSync(file));
}

/** List a fixture directory, sorted, so a manifest-driven test enumerates deterministically. */
export function listFixtureDir(...segments: string[]): string[] {
  const dir = fixturePath(...segments);
  if (!existsSync(dir)) throw new Error(`Fixture directory not found: ${dir}`);
  return readdirSync(dir).sort();
}

// ── Sidecar reading ─────────────────────────────────────────────────────────

/**
 * Parse a sidecar's JSON, keeping integers that JavaScript numbers cannot hold
 * exactly as `bigint`. `JSON.parse` silently rounds `9007199254740993` and
 * `18446744073709551615`, which are exactly the values the `integer_widths` and
 * `all_value_types` fixtures exist to catch — reading them through `JSON.parse`
 * would make the fixture agree with a port that has the same bug.
 */
export function parseJsonPreservingBigInts(text: string): unknown {
  let i = 0;

  const fail = (msg: string): never => {
    throw new Error(`sidecar JSON: ${msg} at offset ${i}`);
  };

  const skipWs = (): void => {
    while (i < text.length && (text[i] === ' ' || text[i] === '\n' || text[i] === '\r' || text[i] === '\t')) i++;
  };

  const parseString = (): string => {
    if (text[i] !== '"') fail('expected string');
    i++;
    let out = '';
    for (;;) {
      if (i >= text.length) fail('unterminated string');
      const c = text[i];
      if (c === '"') { i++; return out; }
      if (c === '\\') {
        i++;
        const e = text[i++];
        switch (e) {
          case '"': out += '"'; break;
          case '\\': out += '\\'; break;
          case '/': out += '/'; break;
          case 'b': out += '\b'; break;
          case 'f': out += '\f'; break;
          case 'n': out += '\n'; break;
          case 'r': out += '\r'; break;
          case 't': out += '\t'; break;
          case 'u': out += String.fromCharCode(parseInt(text.slice(i, i + 4), 16)); i += 4; break;
          default: fail(`bad escape \\${e}`);
        }
        continue;
      }
      out += c;
      i++;
    }
  };

  const parseNumber = (): number | bigint => {
    const start = i;
    if (text[i] === '-') i++;
    while (i < text.length && text[i] >= '0' && text[i] <= '9') i++;
    let isInteger = true;
    if (text[i] === '.') { isInteger = false; i++; while (i < text.length && text[i] >= '0' && text[i] <= '9') i++; }
    if (text[i] === 'e' || text[i] === 'E') {
      isInteger = false;
      i++;
      if (text[i] === '+' || text[i] === '-') i++;
      while (i < text.length && text[i] >= '0' && text[i] <= '9') i++;
    }
    const raw = text.slice(start, i);
    if (isInteger) {
      const big = BigInt(raw);
      // Inside the safe range a number is exact, and a number is what every
      // decoder here hands back for small integers, so keep the shapes aligned.
      if (big >= BigInt(Number.MIN_SAFE_INTEGER) && big <= BigInt(Number.MAX_SAFE_INTEGER)) return Number(raw);
      return big;
    }
    return Number(raw);
  };

  const parseValue = (): unknown => {
    skipWs();
    const c = text[i];
    if (c === '{') {
      i++;
      const obj: Record<string, unknown> = {};
      skipWs();
      if (text[i] === '}') { i++; return obj; }
      for (;;) {
        skipWs();
        const key = parseString();
        skipWs();
        if (text[i] !== ':') fail('expected :');
        i++;
        obj[key] = parseValue();
        skipWs();
        if (text[i] === ',') { i++; continue; }
        if (text[i] === '}') { i++; return obj; }
        fail('expected , or }');
      }
    }
    if (c === '[') {
      i++;
      const arr: unknown[] = [];
      skipWs();
      if (text[i] === ']') { i++; return arr; }
      for (;;) {
        arr.push(parseValue());
        skipWs();
        if (text[i] === ',') { i++; continue; }
        if (text[i] === ']') { i++; return arr; }
        fail('expected , or ]');
      }
    }
    if (c === '"') return parseString();
    if (text.startsWith('true', i)) { i += 4; return true; }
    if (text.startsWith('false', i)) { i += 5; return false; }
    if (text.startsWith('null', i)) { i += 4; return null; }
    if (c === '-' || (c >= '0' && c <= '9')) return parseNumber();
    return fail(`unexpected character ${JSON.stringify(c)}`);
  };

  const value = parseValue();
  skipWs();
  if (i !== text.length) fail('trailing content');
  return value;
}

/** Read a sidecar, preserving integers outside the JavaScript safe range. */
export function readSidecar(...segments: string[]): any {
  const file = fixturePath(...segments);
  if (!existsSync(file)) throw new Error(`Sidecar not found: ${file}`);
  return parseJsonPreservingBigInts(readFileSync(file, 'utf8'));
}

// ── Byte helpers ────────────────────────────────────────────────────────────

export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map((b) => b.toString(16).padStart(2, '0')).join('');
}

export function fromHex(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}
