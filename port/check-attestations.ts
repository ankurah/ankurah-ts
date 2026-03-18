// TS-ONLY: Automated Rust→TS parity checker with commit-hash attestations
//
// 1. Extracts every item (fn, struct, enum, trait, impl) from Rust source
// 2. Maps names automatically: snake_case → camelCase + static mapping table
// 3. Finds the TS counterpart in the mirrored file
// 4. Checks for // @<hash> attestation on the preceding line
// 5. Compares hash against latest commit that touched the Rust file
//
// Usage: bun run port/check-attestations.ts [package-name]

import { execSync } from 'child_process';
import { existsSync, readdirSync, readFileSync } from 'fs';
import { join, relative, resolve } from 'path';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const RS_PATH = resolve(process.env.ANKURAH_RS_PATH ?? join(__dirname, '..', '..', 'ankurah-ts-support'));
const TS_ROOT = resolve(join(__dirname, '..'));

// ---------------------------------------------------------------------------
// Static name mapping for Rust→TS names that aren't simple snake→camel
// ---------------------------------------------------------------------------

const STATIC_NAME_MAP: Record<string, string> = {
  // Display trait
  fmt: 'toString',
  // Serde
  serialize: 'encode',
  deserialize: 'decode',
  // Equality
  eq: 'equals',
  partial_eq: 'equals',
  // Hashing
  hash: 'hash',
  // Clone
  clone: 'clone',
  // Default
  default: 'default',
  // Drop
  drop: 'drop',
  // Conversion
  from: 'from',
  try_from: 'tryFrom',
  into: 'into',
  try_into: 'tryInto',
  // Iterator
  next: 'next',
  into_iter: 'iter',
  // Deref
  deref: 'deref',
  deref_mut: 'derefMut',
  // Constructor
  new: 'new',
};

// ---------------------------------------------------------------------------
// Name conversion
// ---------------------------------------------------------------------------

function snakeToCamel(s: string): string {
  return s.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
}

/** Convert a Rust name to its expected TS name */
function rustNameToTs(rustName: string): string {
  if (STATIC_NAME_MAP[rustName] !== undefined) {
    return STATIC_NAME_MAP[rustName];
  }
  return snakeToCamel(rustName);
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ItemKind = 'fn' | 'struct' | 'enum' | 'trait' | 'impl';

interface RustItem {
  kind: ItemKind;
  rustName: string;      // Original Rust name
  tsName: string;        // Expected TS name (auto-mapped)
  lineNumber: number;
  lineCount: number;
  inWasmBlock: boolean;
  // For impl blocks:
  implTarget?: string;   // e.g. "Node" for "impl Node"
  implTrait?: string;    // e.g. "Display" for "impl Display for Node"
}

interface TsItem {
  kind: 'class' | 'interface' | 'type' | 'function' | 'method' | 'const';
  name: string;
  lineNumber: number;
  lineCount: number;
  hasAttestation: boolean;  // Has // @<hash> on preceding line
  attestHash: string | null;
  parentClass?: string;     // For methods: which class they belong to
}

interface FileReport {
  tsFile: string;
  rustPath: string;
  rustItems: RustItem[];
  matched: { rust: RustItem; ts: TsItem }[];
  unmatched: RustItem[];    // Rust items with no TS counterpart
  unattested: { rust: RustItem; ts: TsItem }[]; // Matched but no // @hash
  stale: { rust: RustItem; ts: TsItem; attestHash: string; currentHash: string }[];
  sizeWarnings: { rust: RustItem; ts: TsItem; pctShorter: number }[];
}

// ---------------------------------------------------------------------------
// Git helpers
// ---------------------------------------------------------------------------

function getLatestCommitHash(filePath: string): string | null {
  try {
    const hash = execSync(`git -C "${RS_PATH}" log -1 --format=%h -- "${relative(RS_PATH, filePath)}"`, {
      encoding: 'utf-8',
      timeout: 5000,
    }).trim();
    return hash || null;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

function walkDir(dir: string, ext: string): string[] {
  const results: string[] = [];
  if (!existsSync(dir)) return results;
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkDir(full, ext));
    } else if (entry.isFile() && entry.name.endsWith(ext)) {
      results.push(full);
    }
  }
  return results;
}

// ---------------------------------------------------------------------------
// Rust extraction
// ---------------------------------------------------------------------------

function findWasmBlockRanges(lines: string[]): [number, number][] {
  const ranges: [number, number][] = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim().match(/#\[cfg\(feature\s*=\s*"wasm"\)\]/)) {
      const range = findBlockRange(lines, i + 1);
      if (range) ranges.push(range);
    }
  }
  return ranges;
}

function findBlockRange(lines: string[], startIdx: number): [number, number] | null {
  let braceStart = -1;
  for (let i = startIdx; i < lines.length; i++) {
    if (lines[i].includes('{')) { braceStart = i; break; }
    const trimmed = lines[i].trim();
    if (trimmed === '' || trimmed.startsWith('#[') || trimmed.startsWith('//')) continue;
    break;
  }
  if (braceStart === -1) return null;
  let depth = 0;
  for (let i = braceStart; i < lines.length; i++) {
    for (const ch of lines[i]) {
      if (ch === '{') depth++;
      if (ch === '}') depth--;
    }
    if (depth === 0) return [braceStart, i];
  }
  return null;
}

function isInRanges(line: number, ranges: [number, number][]): boolean {
  return ranges.some(([s, e]) => line >= s && line <= e);
}

function countBodyLines(lines: string[], startIdx: number): number {
  let depth = 0, started = false;
  for (let i = startIdx; i < lines.length; i++) {
    for (const ch of lines[i]) {
      if (ch === '{') { depth++; started = true; }
      if (ch === '}') depth--;
    }
    if (started && depth === 0) return i - startIdx + 1;
  }
  return lines.length - startIdx;
}

function extractRustItems(filePath: string): RustItem[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const items: RustItem[] = [];
  const wasmRanges = findWasmBlockRanges(lines);

  // Patterns
  const fnRegex = /^\s*(pub(?:\(crate\))?\s+)?(async\s+)?fn\s+(\w+)/;
  const structRegex = /^\s*(pub(?:\(crate\))?\s+)?struct\s+(\w+)/;
  const enumRegex = /^\s*(pub(?:\(crate\))?\s+)?enum\s+(\w+)/;
  const traitRegex = /^\s*(pub(?:\(crate\))?\s+)?trait\s+(\w+)/;
  const implTraitForRegex = /^\s*impl(?:<[^>]*>)?\s+(\w+)(?:<[^>]*>)?\s+for\s+(\w+)/;
  const implRegex = /^\s*impl(?:<[^>]*>)?\s+(\w+)/;

  for (let i = 0; i < lines.length; i++) {
    const inWasm = isInRanges(i, wasmRanges);
    const line = lines[i];
    let match;

    // fn
    if ((match = line.match(fnRegex))) {
      items.push({
        kind: 'fn',
        rustName: match[3],
        tsName: rustNameToTs(match[3]),
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
      continue;
    }

    // struct
    if ((match = line.match(structRegex))) {
      const name = match[2];
      items.push({
        kind: 'struct',
        rustName: name,
        tsName: name, // PascalCase stays
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
      continue;
    }

    // enum
    if ((match = line.match(enumRegex))) {
      const name = match[2];
      items.push({
        kind: 'enum',
        rustName: name,
        tsName: name,
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
      continue;
    }

    // trait
    if ((match = line.match(traitRegex))) {
      const name = match[2];
      items.push({
        kind: 'trait',
        rustName: name,
        tsName: name,
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
      continue;
    }

    // impl Trait for Type (must check before bare impl)
    if ((match = line.match(implTraitForRegex))) {
      items.push({
        kind: 'impl',
        rustName: `${match[1]} for ${match[2]}`,
        tsName: `${match[1]} for ${match[2]}`,
        implTrait: match[1],
        implTarget: match[2],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
      continue;
    }

    // impl Type (inherent)
    if ((match = line.match(implRegex)) && !line.match(/^\s*impl\s*</) ) {
      // Skip if this is just a generic impl without a clear type name
      const name = match[1];
      if (name === 'impl') continue; // malformed
      items.push({
        kind: 'impl',
        rustName: name,
        tsName: name,
        implTarget: name,
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        inWasmBlock: inWasm,
      });
    }
  }

  return items.filter(item => !item.inWasmBlock);
}

// ---------------------------------------------------------------------------
// TS extraction
// ---------------------------------------------------------------------------

function extractTsItems(filePath: string): TsItem[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const items: TsItem[] = [];

  const classRegex = /^\s*(?:export\s+)?(?:abstract\s+)?class\s+(\w+)/;
  const interfaceRegex = /^\s*(?:export\s+)?interface\s+(\w+)/;
  const typeRegex = /^\s*(?:export\s+)?type\s+(\w+)/;
  const funcRegex = /^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)/;
  const constFuncRegex = /^\s*(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s+)?(?:\([^)]*\)|[^=])*=>/;
  const testRegex = /^\s*(?:test|it)\s*\(\s*['"`]([^'"`]+)['"`]/;
  const testSkipRegex = /^\s*(?:test|it)\.skip\s*\(\s*['"`]([^'"`]+)['"`]/;
  const methodRegex = /^\s*(?:(?:public|private|protected|static|readonly|async|override|get|set)\s+)*(\w+)\s*(?:<[^>]*>)?\s*\(/;

  // Track class nesting with a stack
  const classStack: { name: string; depth: number }[] = [];
  let depth = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Count braces BEFORE processing (so class opening brace is counted)
    const prevDepth = depth;
    for (const ch of line) {
      if (ch === '{') depth++;
      if (ch === '}') depth--;
    }

    // Pop class stack when we exit their scope
    while (classStack.length > 0 && depth <= classStack[classStack.length - 1].depth) {
      classStack.pop();
    }

    const currentClass = classStack.length > 0 ? classStack[classStack.length - 1].name : null;

    // Check for // @<hash> on preceding line
    const prevLine = i > 0 ? lines[i - 1].trim() : '';
    const hashMatch = prevLine.match(/^\/\/\s*@([a-f0-9]{7,40})$/);
    const hasAttestation = !!hashMatch;
    const attestHash = hashMatch ? hashMatch[1] : null;

    let match;

    // class
    if ((match = line.match(classRegex))) {
      classStack.push({ name: match[1], depth: prevDepth });
      items.push({
        kind: 'class',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // interface
    if ((match = line.match(interfaceRegex))) {
      items.push({
        kind: 'interface',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // type alias
    if ((match = line.match(typeRegex))) {
      items.push({
        kind: 'type',
        name: match[1],
        lineNumber: i + 1,
        lineCount: 1,
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // standalone function (only outside classes)
    if (!currentClass && (match = line.match(funcRegex))) {
      items.push({
        kind: 'function',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // const arrow function (only outside classes)
    if (!currentClass && (match = line.match(constFuncRegex))) {
      items.push({
        kind: 'const',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // test.skip() call — capture but mark
    if ((match = line.match(testSkipRegex))) {
      items.push({
        kind: 'function',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // test() call
    if ((match = line.match(testRegex))) {
      items.push({
        kind: 'function',
        name: match[1],
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
      });
      continue;
    }

    // Method inside a class
    if (currentClass && (match = trimmed.match(methodRegex))) {
      const methodName = match[1];
      // Skip control flow keywords that match the regex
      const keywords = ['if', 'for', 'while', 'switch', 'catch', 'return', 'throw', 'delete', 'typeof', 'await', 'import', 'export', 'super'];
      if (keywords.includes(methodName)) continue;

      items.push({
        kind: 'method',
        name: methodName === 'constructor' ? 'new' : methodName,
        lineNumber: i + 1,
        lineCount: countBodyLines(lines, i),
        hasAttestation,
        attestHash,
        parentClass: currentClass,
      });
    }
  }

  return items;
}

// ---------------------------------------------------------------------------
// Matching logic
// ---------------------------------------------------------------------------

function matchItems(rustItems: RustItem[], tsItems: TsItem[]): FileReport['matched'] {
  const matched: FileReport['matched'] = [];
  const usedTs = new Set<number>();

  for (const rust of rustItems) {
    let found: TsItem | undefined;

    if (rust.kind === 'fn') {
      // Build a set of candidate TS names to search for
      const candidates = new Set<string>();
      candidates.add(rust.tsName);              // camelCase version
      candidates.add(rust.rustName);            // original snake_case
      // For test functions: "test_foo_bar" → also try "foo bar", "test foo bar", "test_foo_bar"
      if (rust.rustName.startsWith('test_')) {
        const withoutPrefix = rust.rustName.slice(5);
        candidates.add(withoutPrefix.replace(/_/g, ' '));              // "foo bar"
        candidates.add('test ' + withoutPrefix.replace(/_/g, ' '));    // "test foo bar"
        candidates.add(rust.rustName.replace(/_/g, ' '));              // "test foo bar" (full)
      }
      // Also try with underscores replaced by spaces (common test name pattern)
      candidates.add(rust.rustName.replace(/_/g, ' '));

      // Look for function/method/const with any matching name
      found = tsItems.find((ts, idx) =>
        !usedTs.has(idx) &&
        (ts.kind === 'function' || ts.kind === 'method' || ts.kind === 'const') &&
        candidates.has(ts.name)
      );
    } else if (rust.kind === 'struct' || rust.kind === 'enum') {
      found = tsItems.find((ts, idx) =>
        !usedTs.has(idx) && ts.kind === 'class' && ts.name === rust.tsName
      );
      // Also check type aliases and interfaces
      if (!found) {
        found = tsItems.find((ts, idx) =>
          !usedTs.has(idx) &&
          (ts.kind === 'type' || ts.kind === 'interface') &&
          ts.name === rust.tsName
        );
      }
    } else if (rust.kind === 'trait') {
      found = tsItems.find((ts, idx) =>
        !usedTs.has(idx) &&
        (ts.kind === 'interface' || ts.kind === 'class') &&
        ts.name === rust.tsName
      );
    } else if (rust.kind === 'impl') {
      // impl blocks don't have a direct TS counterpart — they're absorbed into classes
      // We verify by checking that the target class exists
      if (rust.implTarget) {
        found = tsItems.find((ts, idx) =>
          !usedTs.has(idx) && ts.kind === 'class' && ts.name === rust.implTarget
        );
      }
    }

    if (found) {
      const idx = tsItems.indexOf(found);
      usedTs.add(idx);
      matched.push({ rust, ts: found });
    }
  }

  return matched;
}

// ---------------------------------------------------------------------------
// MIRRORS parsing
// ---------------------------------------------------------------------------

interface MirrorsInfo {
  tsFile: string;
  rustRelPath: string;
}

function findMirrorsFiles(packageDir: string): MirrorsInfo[] {
  const results: MirrorsInfo[] = [];
  const srcDir = join(packageDir, 'src');
  const testsDir = join(packageDir, '__tests__');

  const tsFiles: string[] = [];
  if (existsSync(srcDir)) tsFiles.push(...walkDir(srcDir, '.ts'));
  if (existsSync(testsDir)) tsFiles.push(...walkDir(testsDir, '.ts'));

  for (const tsFile of tsFiles) {
    const firstLine = readFileSync(tsFile, 'utf-8').split('\n')[0];
    const match = firstLine.match(/\/\/\s*MIRRORS:\s*ankurah\/(.+)/);
    if (match) {
      const rustRelPath = match[1].replace(/\s+\(.*\)/, '').replace(/\s+#\[.*\]/, '').trim();
      results.push({ tsFile, rustRelPath });
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Check a Rust file against ALL TS files that mirror it
// ---------------------------------------------------------------------------

function checkRustFile(rustRelPath: string, tsFiles: string[]): FileReport | null {
  const rustAbsPath = join(RS_PATH, rustRelPath);
  if (!existsSync(rustAbsPath)) return null;

  const rustItems = extractRustItems(rustAbsPath);
  if (rustItems.length === 0) return null;

  // Merge TS items from ALL files that mirror this Rust file
  const allTsItems: TsItem[] = [];
  for (const tsFile of tsFiles) {
    const items = extractTsItems(tsFile);
    // Tag each item with its source file for reporting
    for (const item of items) {
      (item as any)._sourceFile = tsFile;
    }
    allTsItems.push(...items);
  }

  const matched = matchItems(rustItems, allTsItems);
  const matchedKeys = new Set(matched.map(m => m.rust.rustName + ':' + m.rust.kind + ':' + m.rust.lineNumber));

  const unmatched = rustItems.filter(r =>
    !matchedKeys.has(r.rustName + ':' + r.kind + ':' + r.lineNumber)
  );

  const currentHash = getLatestCommitHash(rustAbsPath);

  const unattested = matched.filter(m => !m.ts.hasAttestation);
  const stale = matched
    .filter(m => m.ts.hasAttestation && m.ts.attestHash && currentHash && m.ts.attestHash !== currentHash)
    .map(m => ({ ...m, attestHash: m.ts.attestHash!, currentHash: currentHash! }));

  const sizeWarnings = matched
    .filter(m => m.rust.kind === 'fn' && m.rust.lineCount > 3 && m.ts.lineCount > 0)
    .filter(m => {
      const pct = Math.round(((m.rust.lineCount - m.ts.lineCount) / m.rust.lineCount) * 100);
      return pct > 50;
    })
    .map(m => ({
      rust: m.rust,
      ts: m.ts,
      pctShorter: Math.round(((m.rust.lineCount - m.ts.lineCount) / m.rust.lineCount) * 100),
    }));

  return {
    tsFile: tsFiles.join(', '),
    rustPath: rustRelPath,
    rustItems,
    matched,
    unmatched,
    unattested,
    stale,
    sizeWarnings,
  };
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

const RESET = '\x1b[0m';
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const BOLD = '\x1b[1m';
const DIM = '\x1b[2m';

function c(code: string, text: string): string {
  return process.stdout.isTTY !== false ? `${code}${text}${RESET}` : text;
}

function itemLabel(item: RustItem): string {
  if (item.kind === 'impl' && item.implTrait) {
    return `impl ${item.implTrait} for ${item.implTarget}`;
  }
  if (item.kind === 'impl') {
    return `impl ${item.implTarget}`;
  }
  return `${item.kind} ${item.rustName}`;
}

function printReport(report: FileReport): void {
  const tsRel = relative(TS_ROOT, report.tsFile);
  console.log(`\n${c(BOLD, `=== ${tsRel}`)} ${c(DIM, `(MIRRORS: ankurah/${report.rustPath})`)} ===`);

  // Matched + attested
  for (const m of report.matched) {
    if (m.ts.hasAttestation && !report.stale.some(s => s.rust === m.rust)) {
      console.log(`  ${c(GREEN, '\u2713')} ${itemLabel(m.rust)} → ${m.ts.name} ${c(DIM, `@${m.ts.attestHash}`)}`);
    }
  }

  // Matched but unattested
  for (const m of report.unattested) {
    console.log(`  ${c(YELLOW, '\u26a0')} ${itemLabel(m.rust)} → ${m.ts.name} ${c(YELLOW, '(no @hash)')}`);
  }

  // Stale
  for (const s of report.stale) {
    console.log(`  ${c(CYAN, '\u21bb')} ${itemLabel(s.rust)} → ${s.ts.name} ${c(CYAN, `@${s.attestHash} → ${s.currentHash} STALE`)}`);
  }

  // Unmatched
  for (const u of report.unmatched) {
    console.log(`  ${c(RED, '\u2717')} ${itemLabel(u)} → ${c(RED, `expected TS: ${u.tsName} — NOT FOUND`)}`);
  }

  // Size warnings
  for (const w of report.sizeWarnings) {
    console.log(`  ${c(YELLOW, '\u26a0')} ${itemLabel(w.rust)}: TS ${w.ts.lineCount} lines vs Rust ${w.rust.lineCount} lines (${w.pctShorter}% shorter)`);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);
  const filterPackage = args.find(a => !a.startsWith('-'));

  if (args.includes('--help') || args.includes('-h')) {
    console.log(`
Usage: bun run port/check-attestations.ts [package-name]

Automated Rust→TS parity checker.
- Maps Rust items to TS automatically (snake→camel + static table)
- Checks for // @<commit-hash> attestation on preceding line
- Flags: missing items, unattested items, stale hashes, size mismatches

Options:
  (no args)         Check all packages
  <package-name>    Check only the given package
  --help, -h        Show this help
`);
    return;
  }

  if (!existsSync(RS_PATH)) {
    console.error(`ERROR: Rust repo not found at ${RS_PATH}`);
    process.exit(1);
  }

  const packagesDir = join(TS_ROOT, 'packages');
  const pkgDirs = readdirSync(packagesDir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .filter(name => !filterPackage || name === filterPackage);

  let totalItems = 0, totalMatched = 0, totalUnmatched = 0;
  let totalAttested = 0, totalUnattested = 0, totalStale = 0;
  let totalSizeWarnings = 0, totalFiles = 0;

  for (const pkg of pkgDirs) {
    const pkgDir = join(packagesDir, pkg);
    const mirrors = findMirrorsFiles(pkgDir);
    if (mirrors.length === 0) continue;

    // Group TS files by the Rust file they mirror
    const rustToTs = new Map<string, string[]>();
    for (const { tsFile, rustRelPath } of mirrors) {
      const existing = rustToTs.get(rustRelPath) ?? [];
      existing.push(tsFile);
      rustToTs.set(rustRelPath, existing);
    }

    for (const [rustRelPath, tsFiles] of rustToTs) {
      const report = checkRustFile(rustRelPath, tsFiles);
      if (!report) continue;

      totalFiles++;
      totalItems += report.rustItems.length;
      totalMatched += report.matched.length;
      totalUnmatched += report.unmatched.length;
      totalAttested += report.matched.length - report.unattested.length;
      totalUnattested += report.unattested.length;
      totalStale += report.stale.length;
      totalSizeWarnings += report.sizeWarnings.length;

      // Only print files with issues (or all if verbose)
      if (report.unmatched.length > 0 || report.stale.length > 0 || args.includes('--verbose') || args.includes('-v')) {
        printReport(report);
      }
    }
  }

  console.log('');
  console.log(c(BOLD, '========================================'));
  console.log(c(BOLD, '  Parity Check Summary'));
  console.log(c(BOLD, '========================================'));
  console.log(`  Files checked:    ${totalFiles}`);
  console.log(`  Rust items:       ${totalItems}`);
  console.log(`  ${c(GREEN, `Matched:          ${totalMatched}`)}`);
  if (totalUnmatched > 0) console.log(`  ${c(RED, `Not found in TS:  ${totalUnmatched}`)}`);
  else console.log(`  Not found in TS:  0`);
  console.log(`  ${c(GREEN, `Attested (@hash): ${totalAttested}`)}`);
  if (totalUnattested > 0) console.log(`  ${c(YELLOW, `Unattested:       ${totalUnattested}`)}`);
  if (totalStale > 0) console.log(`  ${c(CYAN, `Stale:            ${totalStale}`)}`);
  if (totalSizeWarnings > 0) console.log(`  ${c(YELLOW, `Size warnings:    ${totalSizeWarnings}`)}`);
  console.log(c(BOLD, '========================================'));

  if (totalUnmatched > 0) process.exit(1);
}

main();
