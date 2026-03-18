// TS-ONLY: Function attestation checker — verifies that TS files attest every Rust fn they mirror
//
// Usage: bun run port/check-attestations.ts [package-name]
//   No args = check all packages
//   With arg = check only that package (e.g., bun run port/check-attestations.ts core)

import { existsSync, readdirSync, readFileSync, statSync } from 'fs';
import { basename, dirname, join, relative, resolve } from 'path';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const RS_PATH = resolve(process.env.ANKURAH_RS_PATH ?? join(__dirname, '..', '..', 'ankurah'));
const TS_ROOT = resolve(join(__dirname, '..'));

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface RustFunction {
  name: string;
  lineNumber: number;
  lineCount: number;
  visibility: string; // "pub", "pub(crate)", "" (private)
  isAsync: boolean;
  inTestBlock: boolean;
  inWasmBlock: boolean;
}

interface TsAttestation {
  fnName: string;
  lineNumber: number;
  isSkipped: boolean;
  skipReason?: string;
  tsLineCount: number | null; // null if we can't find the following function
}

interface FileReport {
  tsFile: string;
  rustPath: string; // The MIRRORS annotation value
  rustFunctions: RustFunction[];
  tsAttestations: TsAttestation[];
  attested: string[];
  missing: string[];
  sizeWarnings: { fnName: string; rustLines: number; tsLines: number; pctShorter: number }[];
}

// ---------------------------------------------------------------------------
// Utility: collect all files recursively
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
// Rust function extraction
// ---------------------------------------------------------------------------

/**
 * Find cfg(test) and cfg(feature = "wasm") block ranges in the file.
 * Returns arrays of [startLine, endLine] (0-indexed).
 */
function findCfgBlockRanges(lines: string[]): { testRanges: [number, number][]; wasmRanges: [number, number][] } {
  const testRanges: [number, number][] = [];
  const wasmRanges: [number, number][] = [];

  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed === '#[cfg(test)]' || trimmed === '#[cfg(test)]') {
      // Find the opening brace of the following block
      const blockRange = findBlockRange(lines, i + 1);
      if (blockRange) {
        testRanges.push(blockRange);
      }
    }
    if (trimmed.match(/#\[cfg\(feature\s*=\s*"wasm"\)\]/)) {
      const blockRange = findBlockRange(lines, i + 1);
      if (blockRange) {
        wasmRanges.push(blockRange);
      }
    }
  }

  return { testRanges, wasmRanges };
}

/**
 * Starting from line startIdx, find the brace-delimited block.
 * Returns [startLine, endLine] (0-indexed, inclusive).
 */
function findBlockRange(lines: string[], startIdx: number): [number, number] | null {
  // Skip to the line with the opening brace
  let braceStart = -1;
  for (let i = startIdx; i < lines.length; i++) {
    if (lines[i].includes('{')) {
      braceStart = i;
      break;
    }
    // If we hit a line that looks like an attribute or blank, continue
    const trimmed = lines[i].trim();
    if (trimmed === '' || trimmed.startsWith('#[') || trimmed.startsWith('//')) {
      continue;
    }
    // If there's text but no brace, this might be a single-item cfg
    break;
  }

  if (braceStart === -1) return null;

  // Count braces from braceStart
  let depth = 0;
  for (let i = braceStart; i < lines.length; i++) {
    for (const ch of lines[i]) {
      if (ch === '{') depth++;
      if (ch === '}') depth--;
    }
    if (depth === 0) {
      return [braceStart, i];
    }
  }

  return null;
}

function isInRanges(lineIdx: number, ranges: [number, number][]): boolean {
  for (const [start, end] of ranges) {
    if (lineIdx >= start && lineIdx <= end) return true;
  }
  return false;
}

/**
 * Extract all function declarations from a Rust file.
 */
function extractRustFunctions(filePath: string): RustFunction[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const functions: RustFunction[] = [];

  const { testRanges, wasmRanges } = findCfgBlockRanges(lines);

  // Match function declarations
  // Patterns: pub fn, pub async fn, pub(crate) fn, async fn, fn
  const fnRegex = /^(\s*)(pub(?:\(crate\))?\s+)?(async\s+)?fn\s+(\w+)/;

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(fnRegex);
    if (!match) continue;

    const name = match[4];
    const visibility = match[2]?.trim() ?? '';
    const isAsync = !!match[3];
    const inTestBlock = isInRanges(i, testRanges);
    const inWasmBlock = isInRanges(i, wasmRanges);

    // Count function body lines
    const lineCount = countRustFunctionLines(lines, i);

    functions.push({
      name,
      lineNumber: i + 1,
      lineCount,
      visibility,
      isAsync,
      inTestBlock,
      inWasmBlock,
    });
  }

  return functions;
}

/**
 * Count lines of a Rust function body starting from the fn declaration line.
 * Counts from the fn line to the closing brace at the same or lower indent level.
 */
function countRustFunctionLines(lines: string[], startIdx: number): number {
  let depth = 0;
  let started = false;

  for (let i = startIdx; i < lines.length; i++) {
    for (const ch of lines[i]) {
      if (ch === '{') {
        depth++;
        started = true;
      }
      if (ch === '}') {
        depth--;
      }
    }
    if (started && depth === 0) {
      return i - startIdx + 1;
    }
  }

  // If we never found the closing brace, count to end of file
  return lines.length - startIdx;
}

// ---------------------------------------------------------------------------
// TS attestation extraction
// ---------------------------------------------------------------------------

/**
 * Extract all `// Rust: fn <name>` attestation comments from a TS file,
 * and measure the line count of the TS function that follows.
 */
function extractTsAttestations(filePath: string): TsAttestation[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const attestations: TsAttestation[] = [];

  // Match: // Rust: fn <name> or // Rust: pub fn <name> or // Rust: pub async fn <name>
  // Also match SKIP variant: // Rust: fn <name> — SKIP: <reason>
  const attestRegex = /\/\/\s*Rust:\s*(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+(\w+)/;
  const skipRegex = /\/\/\s*Rust:\s*(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+\w+.*(?:—|--)\s*SKIP:\s*(.*)/;

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(attestRegex);
    if (!match) continue;

    const fnName = match[1];
    const skipMatch = lines[i].match(skipRegex);
    const isSkipped = !!skipMatch;
    const skipReason = skipMatch?.[1]?.trim();

    // Find the TS function that follows this comment and measure its size
    const tsLineCount = isSkipped ? null : findFollowingTsFunctionSize(lines, i + 1);

    attestations.push({
      fnName,
      lineNumber: i + 1,
      isSkipped,
      skipReason,
      tsLineCount,
    });
  }

  return attestations;
}

/**
 * Starting after an attestation comment, find the next function/method declaration
 * and count its lines.
 */
function findFollowingTsFunctionSize(lines: string[], startIdx: number): number | null {
  // Look for the next function-like declaration within a reasonable range (20 lines)
  // Patterns: function name(, async name(, name(, get name(, set name(
  const fnStartRegex = /^\s*(?:export\s+)?(?:async\s+)?(?:function\s+\w+|(?:get|set)\s+\w+|\w+)\s*(?:<[^>]*>)?\s*\(/;
  // Also match class method patterns and arrow functions
  const methodRegex = /^\s*(?:(?:public|private|protected|static|readonly|async|override)\s+)*(?:get\s+|set\s+)?\w+\s*(?:<[^>]*>)?\s*\(/;
  const arrowRegex = /^\s*(?:export\s+)?(?:const|let)\s+\w+\s*=\s*(?:async\s+)?(?:\([^)]*\)|[^=])*=>/;

  for (let i = startIdx; i < Math.min(startIdx + 20, lines.length); i++) {
    const trimmed = lines[i].trim();
    // Skip blank lines and comments
    if (trimmed === '' || trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('*')) {
      continue;
    }

    if (fnStartRegex.test(lines[i]) || methodRegex.test(lines[i]) || arrowRegex.test(lines[i])) {
      return countTsFunctionLines(lines, i);
    }

    // If we hit something that's not a function declaration, stop looking
    break;
  }

  return null;
}

/**
 * Count lines of a TS function body starting from the function declaration line.
 */
function countTsFunctionLines(lines: string[], startIdx: number): number {
  let depth = 0;
  let started = false;

  for (let i = startIdx; i < lines.length; i++) {
    // Skip string contents to avoid counting braces in strings
    const line = lines[i];
    for (let j = 0; j < line.length; j++) {
      const ch = line[j];
      if (ch === '{') {
        depth++;
        started = true;
      }
      if (ch === '}') {
        depth--;
      }
    }
    if (started && depth === 0) {
      return i - startIdx + 1;
    }
  }

  return lines.length - startIdx;
}

// ---------------------------------------------------------------------------
// MIRRORS annotation parsing
// ---------------------------------------------------------------------------

interface MirrorsInfo {
  tsFile: string;
  rustRelPath: string; // e.g. "core/src/node.rs"
  isTestMirror: boolean; // e.g. "core/src/node.rs #[cfg(test)]" or "(tests module)"
}

/**
 * Scan all TS files in a package for MIRRORS annotations.
 */
function findMirrorsFiles(packageDir: string): MirrorsInfo[] {
  const results: MirrorsInfo[] = [];

  const srcDir = join(packageDir, 'src');
  const testsDir = join(packageDir, '__tests__');

  const tsFiles: string[] = [];
  if (existsSync(srcDir)) tsFiles.push(...walkDir(srcDir, '.ts'));
  if (existsSync(testsDir)) tsFiles.push(...walkDir(testsDir, '.ts'));

  for (const tsFile of tsFiles) {
    const content = readFileSync(tsFile, 'utf-8');
    // Find all MIRRORS annotations (there can be multiple in a file)
    const mirrorsRegex = /\/\/\s*MIRRORS:\s*ankurah\/(.+)/g;
    let match;
    while ((match = mirrorsRegex.exec(content)) !== null) {
      const rawPath = match[1].trim();
      // Check if this is a test mirror
      const isTestMirror = rawPath.includes('#[cfg(test)]') || rawPath.includes('(tests module)') || rawPath.includes('(tests)');
      // Extract just the file path
      const rustRelPath = rawPath.replace(/\s+#\[cfg\(test\)\].*$/, '').replace(/\s+\(tests?\s*(?:module)?\).*$/, '').trim();

      results.push({ tsFile, rustRelPath, isTestMirror });
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Check a single file pair
// ---------------------------------------------------------------------------

function checkFile(tsFile: string, rustRelPath: string, isTestMirror: boolean): FileReport | null {
  const rustAbsPath = join(RS_PATH, rustRelPath);

  if (!existsSync(rustAbsPath)) {
    return null; // Rust file doesn't exist — handled by audit-port.ts
  }

  const allRustFunctions = extractRustFunctions(rustAbsPath);

  // Filter: skip test functions and wasm functions
  let rustFunctions: RustFunction[];
  if (isTestMirror) {
    // For test mirror files, we only care about functions IN test blocks
    rustFunctions = allRustFunctions.filter((f) => f.inTestBlock && !f.inWasmBlock);
  } else {
    // For source mirror files, we skip test functions and wasm functions
    rustFunctions = allRustFunctions.filter((f) => !f.inTestBlock && !f.inWasmBlock);
  }

  const tsAttestations = extractTsAttestations(tsFile);
  const attestedNames = new Set(tsAttestations.map((a) => a.fnName));

  const attested: string[] = [];
  const missing: string[] = [];
  const sizeWarnings: FileReport['sizeWarnings'] = [];

  for (const rustFn of rustFunctions) {
    if (attestedNames.has(rustFn.name)) {
      attested.push(rustFn.name);

      // Check size comparison
      const tsAttest = tsAttestations.find((a) => a.fnName === rustFn.name);
      if (tsAttest && !tsAttest.isSkipped && tsAttest.tsLineCount !== null && rustFn.lineCount > 2) {
        const pctShorter = Math.round(((rustFn.lineCount - tsAttest.tsLineCount) / rustFn.lineCount) * 100);
        if (pctShorter > 50) {
          sizeWarnings.push({
            fnName: rustFn.name,
            rustLines: rustFn.lineCount,
            tsLines: tsAttest.tsLineCount,
            pctShorter,
          });
        }
      }
    } else {
      missing.push(rustFn.name);
    }
  }

  return {
    tsFile,
    rustPath: rustRelPath,
    rustFunctions,
    tsAttestations,
    attested,
    missing,
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
const BOLD = '\x1b[1m';
const DIM = '\x1b[2m';

const useColor = process.stdout.isTTY !== false;
function c(code: string, text: string): string {
  return useColor ? `${code}${text}${RESET}` : text;
}

function printFileReport(report: FileReport): void {
  const tsRel = relative(TS_ROOT, report.tsFile);
  console.log(`\n${c(BOLD, `=== ${tsRel}`)} ${c(DIM, `(MIRRORS: ankurah/${report.rustPath})`)} ===`);

  if (report.rustFunctions.length === 0) {
    console.log(`  ${c(DIM, '(no functions found in Rust file)')}`);
    return;
  }

  // Sort: attested first, then missing, then size warnings inline
  for (const name of report.attested) {
    const sizeWarn = report.sizeWarnings.find((w) => w.fnName === name);
    const attest = report.tsAttestations.find((a) => a.fnName === name);
    if (sizeWarn) {
      console.log(
        `  ${c(YELLOW, '\u26a0')} fn ${name} — attested but TS is ${sizeWarn.tsLines} lines vs Rust ${sizeWarn.rustLines} lines (${sizeWarn.pctShorter}% shorter)`,
      );
    } else if (attest?.isSkipped) {
      console.log(`  ${c(GREEN, '\u2713')} fn ${name} — skipped: ${attest.skipReason ?? '(no reason given)'}`);
    } else {
      console.log(`  ${c(GREEN, '\u2713')} fn ${name} — attested`);
    }
  }

  for (const name of report.missing) {
    console.log(`  ${c(RED, '\u2717')} fn ${name} — ${c(RED, 'NOT ATTESTED')}`);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);
  const filterPackage = args.find((a) => !a.startsWith('-'));

  if (args.includes('--help') || args.includes('-h')) {
    console.log(`
Usage: bun run port/check-attestations.ts [package-name]

Checks that every function in Rust source files has a corresponding
  // Rust: fn <name>
attestation comment in the mirroring TS file.

Options:
  (no args)         Check all packages
  <package-name>    Check only the given package (e.g., "core")
  --help, -h        Show this help message
`);
    return;
  }

  if (!existsSync(RS_PATH)) {
    console.error(
      `ERROR: Rust repo not found at ${RS_PATH}\nSet ANKURAH_RS_PATH environment variable or ensure ../ankurah exists.`,
    );
    process.exit(1);
  }

  const packagesDir = join(TS_ROOT, 'packages');
  if (!existsSync(packagesDir)) {
    console.error('ERROR: packages/ directory does not exist.');
    process.exit(1);
  }

  // Discover packages
  const pkgDirs = readdirSync(packagesDir, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name)
    .filter((name) => !filterPackage || name === filterPackage);

  if (pkgDirs.length === 0) {
    console.error(`ERROR: No package found matching "${filterPackage}".`);
    process.exit(1);
  }

  let totalAttested = 0;
  let totalMissing = 0;
  let totalSizeWarnings = 0;
  let totalFiles = 0;
  let totalFunctions = 0;

  for (const pkg of pkgDirs) {
    const pkgDir = join(packagesDir, pkg);
    const mirrorsFiles = findMirrorsFiles(pkgDir);

    if (mirrorsFiles.length === 0) continue;

    for (const mirror of mirrorsFiles) {
      const report = checkFile(mirror.tsFile, mirror.rustRelPath, mirror.isTestMirror);
      if (!report) continue;
      if (report.rustFunctions.length === 0) continue;

      totalFiles++;
      totalFunctions += report.rustFunctions.length;
      totalAttested += report.attested.length;
      totalMissing += report.missing.length;
      totalSizeWarnings += report.sizeWarnings.length;

      printFileReport(report);
    }
  }

  // Summary
  console.log('');
  console.log(c(BOLD, '========================================'));
  console.log(c(BOLD, '  Function Attestation Summary'));
  console.log(c(BOLD, '========================================'));
  console.log(`  Files checked:    ${totalFiles}`);
  console.log(`  Total functions:  ${totalFunctions}`);
  console.log(`  ${c(GREEN, `Attested:         ${totalAttested}`)}`);
  if (totalMissing > 0) {
    console.log(`  ${c(RED, `Missing:          ${totalMissing}`)}`);
  } else {
    console.log(`  Missing:          0`);
  }
  if (totalSizeWarnings > 0) {
    console.log(`  ${c(YELLOW, `Size warnings:    ${totalSizeWarnings}`)}`);
  }
  console.log(c(BOLD, '========================================'));
  console.log('');

  if (totalMissing > 0) {
    process.exit(1);
  }
}

main();
