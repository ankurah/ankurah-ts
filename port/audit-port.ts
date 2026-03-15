// TS-ONLY: Programmatic audit script validating bidirectional mapping between Rust and TS repos
//
// Usage: bun run port/audit-port.ts
// Env:   ANKURAH_RS_PATH (default: ../ankurah)

import { createHash } from "crypto";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "fs";
import { basename, dirname, join, relative, resolve } from "path";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const RS_PATH = resolve(process.env.ANKURAH_RS_PATH ?? join(__dirname, "..", "..", "ankurah"));
const TS_ROOT = resolve(join(__dirname, ".."));
const MANIFEST_PATH = join(TS_ROOT, "port", ".rust-source-hashes.json");

/** Crate-path (relative to RS_PATH) -> array of TS package paths (relative to TS_ROOT/packages/) */
const CRATE_TO_PACKAGES: Record<string, string[]> = {
  "proto":                       ["proto"],
  "core":                        ["core"],
  "signals":                     ["signals"],
  "ankql":                       ["ankql"],
  "storage/common":              ["storage-common"],
  "storage/sqlite":              ["storage-expo-sqlite", "storage-better-sqlite3"],
  "connectors/websocket-client": ["connector-websocket"],
  "connectors/local-process":    ["connector-local"],
};

/** Crates that are completely out of scope -- not present in CRATE_TO_PACKAGES at all.
 *  Listed here for documentation; the script only iterates over CRATE_TO_PACKAGES keys. */
const DESCOPED_CRATES = new Set([
  "derive",
  "storage/postgres",
  "storage/sled",
  "storage/indexeddb-wasm",
  "connectors/websocket-server",
  "connectors/websocket-client-wasm",
  "ankurah",  // facade crate -- re-exports only
]);

/** Individual Rust filenames to skip (basename only, no path) -- WASM-only (E9) */
const WASM_ONLY_FILES = new Set(["wasm.rs", "tsify.rs", "jsvalue.rs"]);

/** Individual Rust filenames to skip -- feature-gated (E10) */
const FEATURE_GATED_FILES = new Set(["postgres.rs"]);

/** Individual Rust filenames to skip -- de-scoped */
const DESCOPED_FILES = new Set(["pn_counter.rs"]);

/** Specific crate-relative paths to skip with their exception rule */
const SKIP_PATHS: Record<string, string> = {
  // E15: React hooks replaced by @ankurah/react
  "signals/src/react.rs":          "E15",
  "signals/src/react_native.rs":   "E15",
  // E14: Rust-only reactive_graph integration
  "signals/src/reactive_graph.rs": "E14",
};

/** Filename mapping exceptions: Rust basename -> TS basename */
const FILENAME_EXCEPTIONS: Record<string, { tsName: string; exception: string }> = {
  "yrs.rs":  { tsName: "yjs.ts", exception: "E5" },
  "mod.rs":  { tsName: "index.ts", exception: "E2" },
  "lib.rs":  { tsName: "index.ts", exception: "E2" },
};

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

type Severity = "PASS" | "FAIL" | "WARN";

interface ReportItem {
  severity: Severity;
  check: string;
  message: string;
  file?: string;
}

const report: ReportItem[] = [];

function pass(check: string, message: string, file?: string) {
  report.push({ severity: "PASS", check, message, file });
}
function fail(check: string, message: string, file?: string) {
  report.push({ severity: "FAIL", check, message, file });
}
function warn(check: string, message: string, file?: string) {
  report.push({ severity: "WARN", check, message, file });
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
// Utility: read first line of a file
// ---------------------------------------------------------------------------

function readFirstLine(filePath: string): string | null {
  try {
    const buf = readFileSync(filePath, "utf-8");
    const nl = buf.indexOf("\n");
    return nl === -1 ? buf : buf.slice(0, nl);
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Utility: check if Rust file contains test markers
// ---------------------------------------------------------------------------

function rustFileHasTests(filePath: string): boolean {
  try {
    const content = readFileSync(filePath, "utf-8");
    return (
      content.includes("#[cfg(test)]") ||
      content.includes("#[test]") ||
      content.includes("#[tokio::test]")
    );
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Utility: compute SHA-256 hash of a file
// ---------------------------------------------------------------------------

function sha256File(filePath: string): string | null {
  try {
    const content = readFileSync(filePath);
    return createHash("sha256").update(content).digest("hex");
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Manifest: Rust source file hash tracking for drift detection
// ---------------------------------------------------------------------------

/** Maps Rust file paths (relative to RS_PATH, e.g. "proto/src/id.rs") to their SHA-256 hashes */
type HashManifest = Record<string, string>;

function loadManifest(): HashManifest {
  try {
    const raw = readFileSync(MANIFEST_PATH, "utf-8");
    return JSON.parse(raw) as HashManifest;
  } catch {
    return {};
  }
}

function saveManifest(manifest: HashManifest): void {
  // Sort keys for stable, diffable output
  const sorted: HashManifest = {};
  for (const key of Object.keys(manifest).sort()) {
    sorted[key] = manifest[key];
  }
  writeFileSync(MANIFEST_PATH, JSON.stringify(sorted, null, 2) + "\n", "utf-8");
}

// ---------------------------------------------------------------------------
// Utility: extract MIRRORS annotation Rust path from a TS file
// ---------------------------------------------------------------------------

/** Returns the Rust relative path (e.g. "core/src/entity.rs") from a MIRRORS annotation, or null */
function extractMirrorsPath(tsFile: string): string | null {
  const firstLine = readFirstLine(tsFile);
  if (!firstLine) return null;
  const match = firstLine.match(/^\/\/\s*MIRRORS:\s*(.+)$/);
  if (!match) return null;
  // Strip the "ankurah/" prefix if present
  return match[1].trim().replace(/^ankurah\//, "");
}

// ---------------------------------------------------------------------------
// Map a Rust source file path to expected TS file path(s)
// ---------------------------------------------------------------------------

interface MappingResult {
  tsFiles: string[];        // expected TS file paths (absolute)
  skipped: boolean;         // if true, this Rust file should be skipped
  skipReason?: string;      // why it was skipped
  exception?: string;       // exception rule that applies to the filename mapping
  isFileWithSubmodules: boolean;
}

function mapRustFileToTs(
  rsFileAbs: string,
  cratePath: string,
  tsPackages: string[],
): MappingResult {
  const crateRoot = join(RS_PATH, cratePath);
  const srcDir = join(crateRoot, "src");
  const relToSrc = relative(srcDir, rsFileAbs); // e.g. "property/backend/yrs.rs"
  const crateRelPath = `${cratePath}/src/${relToSrc}`;

  // Check specific skip paths (keyed by crate-relative path like "signals/src/react.rs")
  if (SKIP_PATHS[crateRelPath]) {
    return { tsFiles: [], skipped: true, skipReason: `Exception ${SKIP_PATHS[crateRelPath]}`, isFileWithSubmodules: false };
  }

  const rsBasename = basename(rsFileAbs);

  // Check WASM-only skips (E9)
  if (WASM_ONLY_FILES.has(rsBasename)) {
    return { tsFiles: [], skipped: true, skipReason: "Exception E9: WASM-only module", isFileWithSubmodules: false };
  }

  // Check feature-gated skips (E10)
  if (FEATURE_GATED_FILES.has(rsBasename)) {
    return { tsFiles: [], skipped: true, skipReason: "Exception E10: feature-gated module", isFileWithSubmodules: false };
  }

  // Check de-scoped files
  if (DESCOPED_FILES.has(rsBasename)) {
    return { tsFiles: [], skipped: true, skipReason: "De-scoped (pn_counter)", isFileWithSubmodules: false };
  }

  // Determine TS filename
  let tsBasename: string;
  let exception: string | undefined;

  if (FILENAME_EXCEPTIONS[rsBasename]) {
    tsBasename = FILENAME_EXCEPTIONS[rsBasename].tsName;
    exception = FILENAME_EXCEPTIONS[rsBasename].exception;
  } else {
    // Direct mapping: foo_bar.rs -> foo_bar.ts
    tsBasename = rsBasename.replace(/\.rs$/, ".ts");
  }

  // Check for file-with-submodules pattern (E12):
  // Rust has both foo.rs AND foo/ directory -- TS becomes foo/index.ts
  let isFileWithSubmodules = false;
  if (rsBasename !== "mod.rs" && rsBasename !== "lib.rs") {
    const potentialDir = rsFileAbs.replace(/\.rs$/, "");
    if (existsSync(potentialDir) && statSync(potentialDir).isDirectory()) {
      isFileWithSubmodules = true;
      // foo.rs with foo/ directory -> foo/index.ts
      const dirName = basename(potentialDir);
      const parentRelToSrc = relative(srcDir, dirname(rsFileAbs));
      const tsRelPath = parentRelToSrc
        ? join(parentRelToSrc, dirName, "index.ts")
        : join(dirName, "index.ts");
      exception = "E12";
      return {
        tsFiles: tsPackages.map((pkg) => join(TS_ROOT, "packages", pkg, "src", tsRelPath)),
        skipped: false,
        exception,
        isFileWithSubmodules: true,
      };
    }
  }

  // Build path relative to src/ in the TS package
  const dirRelToSrc = relative(srcDir, dirname(rsFileAbs));
  const tsRelPath = dirRelToSrc ? join(dirRelToSrc, tsBasename) : tsBasename;

  return {
    tsFiles: tsPackages.map((pkg) => join(TS_ROOT, "packages", pkg, "src", tsRelPath)),
    skipped: false,
    exception,
    isFileWithSubmodules: false,
  };
}

// ---------------------------------------------------------------------------
// Check 1: Rust file coverage
// ---------------------------------------------------------------------------

function checkRustFileCoverage() {
  const CHECK = "Rust file coverage";

  if (!existsSync(RS_PATH)) {
    fail(CHECK, `Rust repo not found at ${RS_PATH}. Set ANKURAH_RS_PATH.`);
    return;
  }

  let totalRsFiles = 0;
  let coveredFiles = 0;
  let skippedFiles = 0;
  let missingFiles = 0;
  let warnedFiles = 0;

  for (const [cratePath, tsPackages] of Object.entries(CRATE_TO_PACKAGES)) {
    const srcDir = join(RS_PATH, cratePath, "src");
    if (!existsSync(srcDir)) {
      warn(CHECK, `Rust crate source dir not found: ${cratePath}/src/`, srcDir);
      continue;
    }

    const rsFiles = walkDir(srcDir, ".rs");

    for (const rsFile of rsFiles) {
      totalRsFiles++;
      const mapping = mapRustFileToTs(rsFile, cratePath, tsPackages);

      if (mapping.skipped) {
        skippedFiles++;
        continue;
      }

      // Check if at least one TS package has the file
      let anyExists = false;
      let anyPackageExists = false;

      for (const tsFile of mapping.tsFiles) {
        const pkgDir = join(TS_ROOT, "packages", relative(join(TS_ROOT, "packages"), tsFile).split("/")[0]);
        if (existsSync(pkgDir)) {
          anyPackageExists = true;
        }
        if (existsSync(tsFile)) {
          anyExists = true;
        }
      }

      if (anyExists) {
        coveredFiles++;
      } else if (!anyPackageExists) {
        // Package doesn't exist yet -- warn, not fail
        warnedFiles++;
        const relRs = relative(RS_PATH, rsFile);
        const expectedTs = mapping.tsFiles.map((f) => relative(TS_ROOT, f)).join(" OR ");
        warn(CHECK, `Package not scaffolded yet: ${relRs} -> ${expectedTs}`, rsFile);
      } else {
        missingFiles++;
        const relRs = relative(RS_PATH, rsFile);
        const expectedTs = mapping.tsFiles.map((f) => relative(TS_ROOT, f)).join(" OR ");
        fail(CHECK, `Missing TS file: ${relRs} -> ${expectedTs}`, rsFile);
      }
    }
  }

  if (missingFiles === 0 && totalRsFiles > 0) {
    pass(
      CHECK,
      `All ${coveredFiles} in-scope Rust files covered (${skippedFiles} skipped, ${warnedFiles} warned)`,
    );
  }
}

// ---------------------------------------------------------------------------
// Check 2 & 3 & 4 & 6 & 7: TS file annotations & validity
// ---------------------------------------------------------------------------

function checkTsAnnotations() {
  const CHECK_ANNOTATION = "TS file annotations";
  const CHECK_MIRRORS_VALID = "MIRRORS validity";
  const CHECK_TSONLY_VALID = "TS-ONLY validity";
  const CHECK_ORPHAN = "Orphan detection";
  const CHECK_EXCEPTION = "Exception citations";

  const packagesDir = join(TS_ROOT, "packages");
  if (!existsSync(packagesDir)) {
    warn(CHECK_ANNOTATION, "packages/ directory does not exist yet -- skipping annotation checks");
    return;
  }

  const pkgDirs = readdirSync(packagesDir, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name);

  if (pkgDirs.length === 0) {
    warn(CHECK_ANNOTATION, "No packages found in packages/ -- skipping annotation checks");
    return;
  }

  let totalTs = 0;
  let annotated = 0;
  let mirrorsValid = 0;
  let mirrorsInvalid = 0;
  let tsOnlyValid = 0;
  let tsOnlyInvalid = 0;
  let missingAnnotation = 0;
  let exceptionOk = 0;
  let exceptionMissing = 0;

  for (const pkg of pkgDirs) {
    const srcDir = join(packagesDir, pkg, "src");
    if (!existsSync(srcDir)) continue;

    // Include all .ts files in src/ (source AND test files) -- G5 says test files need annotations too
    const tsFiles: string[] = walkDir(srcDir, ".ts");

    // Also check __tests__/ directory if it exists
    const testsDir = join(packagesDir, pkg, "__tests__");
    if (existsSync(testsDir)) {
      tsFiles.push(...walkDir(testsDir, ".ts"));
    }

    for (const tsFile of tsFiles) {
      totalTs++;
      const firstLine = readFirstLine(tsFile);

      if (!firstLine) {
        missingAnnotation++;
        fail(CHECK_ANNOTATION, `Empty file or unreadable`, tsFile);
        continue;
      }

      const mirrorsMatch = firstLine.match(/^\/\/\s*MIRRORS:\s*(.+)$/);
      const tsOnlyMatch = firstLine.match(/^\/\/\s*TS-ONLY:\s*(.+)$/);

      if (!mirrorsMatch && !tsOnlyMatch) {
        missingAnnotation++;
        fail(
          CHECK_ANNOTATION,
          `Line 1 must be "// MIRRORS: ..." or "// TS-ONLY: ...", got: ${firstLine.slice(0, 80)}`,
          tsFile,
        );
        continue;
      }

      annotated++;

      // ------ MIRRORS validity (Check 3 & 6) ------
      if (mirrorsMatch) {
        const rsRelPath = mirrorsMatch[1].trim(); // e.g. "ankurah/core/src/entity.rs"
        const rsAbsPath = join(RS_PATH, rsRelPath.replace(/^ankurah\//, ""));

        if (!existsSync(rsAbsPath)) {
          mirrorsInvalid++;
          fail(
            CHECK_MIRRORS_VALID,
            `MIRRORS annotation points to non-existent Rust file: ${rsRelPath}`,
            tsFile,
          );
          // Also counts as orphan
          fail(CHECK_ORPHAN, `Orphaned: claims to mirror ${rsRelPath} which does not exist`, tsFile);
        } else {
          mirrorsValid++;

          // ------ Exception citation check (Check 7) ------
          // Detect divergences that need exception citations
          const rsBasename = basename(rsAbsPath);
          const tsBasename = basename(tsFile);

          // Check yrs.rs -> yjs.ts exception
          if (rsBasename === "yrs.rs" && tsBasename === "yjs.ts") {
            // Should cite E5
            const content = readFileSync(tsFile, "utf-8");
            if (content.includes("E5") || content.includes("Exception E5")) {
              exceptionOk++;
            } else {
              exceptionMissing++;
              fail(
                CHECK_EXCEPTION,
                `yrs.rs -> yjs.ts mapping divergence must cite Exception E5`,
                tsFile,
              );
            }
          }

          // Check mod.rs -> index.ts (E2)
          if (
            (rsBasename === "mod.rs" || rsBasename === "lib.rs") &&
            tsBasename === "index.ts"
          ) {
            // E2 is so standard we don't require inline citation for it
          }

          // Check file-with-submodules E12
          if (tsBasename === "index.ts" && rsBasename !== "mod.rs" && rsBasename !== "lib.rs") {
            // This is an E12 case -- the MIRRORS path should be to a .rs file
            // and the TS file is in a directory named after that .rs file
            const content = readFileSync(tsFile, "utf-8");
            if (content.includes("E12") || content.includes("Exception E12")) {
              exceptionOk++;
            } else {
              exceptionMissing++;
              fail(
                CHECK_EXCEPTION,
                `File-with-submodules mapping (${rsBasename} -> dir/index.ts) must cite Exception E12`,
                tsFile,
              );
            }
          }
        }
      }

      // ------ TS-ONLY validity (Check 4) ------
      if (tsOnlyMatch) {
        // Check if there actually IS a corresponding Rust file (in which case it should be MIRRORS)
        // Only check source files in src/, not test files in __tests__/ or .test.ts files
        const isInSrc = tsFile.startsWith(join(packagesDir, pkg, "src"));
        const isTestFile = basename(tsFile).endsWith(".test.ts") || basename(tsFile).endsWith(".spec.ts");

        if (isInSrc && !isTestFile) {
          const tsRelToSrc = relative(join(packagesDir, pkg, "src"), tsFile);
          const hasCorrespondingRust = checkIfRustFileExistsForTsFile(pkg, tsRelToSrc);

          if (hasCorrespondingRust) {
            tsOnlyInvalid++;
            fail(
              CHECK_TSONLY_VALID,
              `TS-ONLY file has a corresponding Rust file -- should use MIRRORS annotation instead`,
              tsFile,
            );
          } else {
            tsOnlyValid++;
          }
        } else {
          // Test files and __tests__/ files marked TS-ONLY are fine
          tsOnlyValid++;
        }
      }
    }
  }

  if (totalTs > 0) {
    if (missingAnnotation === 0) {
      pass(CHECK_ANNOTATION, `All ${annotated} TS source files have valid annotations`);
    }
    if (mirrorsInvalid === 0 && mirrorsValid > 0) {
      pass(CHECK_MIRRORS_VALID, `All ${mirrorsValid} MIRRORS annotations point to existing Rust files`);
    }
    if (tsOnlyInvalid === 0 && tsOnlyValid > 0) {
      pass(CHECK_TSONLY_VALID, `All ${tsOnlyValid} TS-ONLY files verified (no corresponding Rust file)`);
    }
    if (mirrorsInvalid === 0) {
      pass(CHECK_ORPHAN, `No orphaned TS files found`);
    }
    if (exceptionMissing === 0 && exceptionOk > 0) {
      pass(CHECK_EXCEPTION, `All ${exceptionOk} mapping divergences cite exception rules`);
    } else if (exceptionMissing === 0 && exceptionOk === 0) {
      pass(CHECK_EXCEPTION, `No mapping divergences detected (nothing to cite)`);
    }
  } else {
    warn(CHECK_ANNOTATION, "No TS source files found in packages/*/src/ -- nothing to check");
  }
}

// ---------------------------------------------------------------------------
// Helper: given a TS package name and relative path, check if a Rust file exists
// ---------------------------------------------------------------------------

/** Reverse-map a TS package + relative path to a Rust file path and check existence */
function checkIfRustFileExistsForTsFile(tsPkg: string, tsRelPath: string): boolean {
  // Find which crate(s) map to this package
  for (const [cratePath, packages] of Object.entries(CRATE_TO_PACKAGES)) {
    if (!packages.includes(tsPkg)) continue;

    // Reverse the filename mapping
    let rsRelPath = tsRelPath;

    // index.ts could be lib.rs, mod.rs, or file-with-submodules
    const tsBasename = basename(tsRelPath);
    if (tsBasename === "index.ts") {
      const parentDir = dirname(tsRelPath);
      // Check if this is a top-level index.ts (lib.rs)
      if (parentDir === ".") {
        const libPath = join(RS_PATH, cratePath, "src", "lib.rs");
        if (existsSync(libPath)) return true;
      }
      // Check mod.rs
      const modPath = join(RS_PATH, cratePath, "src", parentDir === "." ? "" : parentDir, "mod.rs");
      if (existsSync(modPath)) return true;
      // Check file-with-submodules (E12): parent-dir-name.rs
      if (parentDir !== ".") {
        const dirName = basename(parentDir);
        const grandParent = dirname(parentDir);
        const fileWithSubmodules = join(
          RS_PATH,
          cratePath,
          "src",
          grandParent === "." ? "" : grandParent,
          `${dirName}.rs`,
        );
        if (existsSync(fileWithSubmodules)) return true;
      }
    } else {
      // Direct name mapping
      let rsBasename = tsBasename.replace(/\.ts$/, ".rs");
      // Reverse E5: yjs.ts -> yrs.rs
      if (tsBasename === "yjs.ts") {
        rsBasename = "yrs.rs";
      }
      const rsPath = join(RS_PATH, cratePath, "src", dirname(tsRelPath), rsBasename);
      if (existsSync(rsPath)) return true;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Check 5: Test coverage
// ---------------------------------------------------------------------------

function checkTestCoverage() {
  const CHECK = "Test coverage";

  if (!existsSync(RS_PATH)) {
    fail(CHECK, `Rust repo not found at ${RS_PATH}`);
    return;
  }

  let totalRsWithTests = 0;
  let coveredTests = 0;
  let missingTests = 0;
  let warnedTests = 0;

  for (const [cratePath, tsPackages] of Object.entries(CRATE_TO_PACKAGES)) {
    const srcDir = join(RS_PATH, cratePath, "src");
    if (!existsSync(srcDir)) continue;

    const rsFiles = walkDir(srcDir, ".rs");

    for (const rsFile of rsFiles) {
      // Skip files that are out of scope
      const mapping = mapRustFileToTs(rsFile, cratePath, tsPackages);
      if (mapping.skipped) continue;

      if (!rustFileHasTests(rsFile)) continue;
      totalRsWithTests++;

      // Determine expected test file(s)
      // For each expected TS file, there should be a .test.ts adjacent
      let anyTestExists = false;
      let anyPackageExists = false;

      for (const tsFile of mapping.tsFiles) {
        const pkgName = relative(join(TS_ROOT, "packages"), tsFile).split("/")[0];
        const pkgDir = join(TS_ROOT, "packages", pkgName);

        if (existsSync(pkgDir)) {
          anyPackageExists = true;
        }

        // Test file: same path but .test.ts instead of .ts
        const testFile = tsFile.replace(/\.ts$/, ".test.ts");
        if (existsSync(testFile)) {
          anyTestExists = true;
        }

        // Also check __tests__/ directory pattern
        const srcRelPath = relative(join(pkgDir, "src"), tsFile);
        const testInDir = join(pkgDir, "__tests__", srcRelPath.replace(/\.ts$/, ".test.ts"));
        if (existsSync(testInDir)) {
          anyTestExists = true;
        }
      }

      if (anyTestExists) {
        coveredTests++;
      } else if (!anyPackageExists) {
        warnedTests++;
        const relRs = relative(RS_PATH, rsFile);
        warn(CHECK, `Package not scaffolded yet for test: ${relRs}`, rsFile);
      } else {
        missingTests++;
        const relRs = relative(RS_PATH, rsFile);
        const expectedTests = mapping.tsFiles
          .map((f) => relative(TS_ROOT, f).replace(/\.ts$/, ".test.ts"))
          .join(" OR ");
        fail(CHECK, `Missing test file for ${relRs} -> ${expectedTests}`, rsFile);
      }
    }

    // Also check integration tests in crate's tests/ directory
    const testsDir = join(RS_PATH, cratePath, "tests");
    if (existsSync(testsDir)) {
      const testRsFiles = walkDir(testsDir, ".rs");
      for (const testRsFile of testRsFiles) {
        const testBasename = basename(testRsFile);
        // Skip common.rs (test helpers, not actual test files)
        if (testBasename === "common.rs" || testBasename === "main.rs") continue;

        totalRsWithTests++;
        const testRelPath = relative(testsDir, testRsFile);

        let anyTestExists = false;
        let anyPackageExists = false;

        for (const tsPkg of tsPackages) {
          const pkgDir = join(TS_ROOT, "packages", tsPkg);
          if (existsSync(pkgDir)) {
            anyPackageExists = true;
          }

          // Look for corresponding test in __tests__/ directory
          const tsTestName = testRelPath.replace(/\.rs$/, ".test.ts");
          const testInDir = join(pkgDir, "__tests__", tsTestName);
          if (existsSync(testInDir)) {
            anyTestExists = true;
          }

          // Also check src/ adjacent pattern
          const testInSrc = join(pkgDir, "src", tsTestName);
          if (existsSync(testInSrc)) {
            anyTestExists = true;
          }
        }

        if (anyTestExists) {
          coveredTests++;
        } else if (!anyPackageExists) {
          warnedTests++;
          const relTest = relative(RS_PATH, testRsFile);
          warn(CHECK, `Package not scaffolded yet for integration test: ${relTest}`, testRsFile);
        } else {
          missingTests++;
          const relTest = relative(RS_PATH, testRsFile);
          fail(CHECK, `Missing integration test file for ${relTest}`, testRsFile);
        }
      }
    }
  }

  if (totalRsWithTests === 0) {
    warn(CHECK, "No Rust test files detected (unexpected -- check Rust repo path)");
  } else if (missingTests === 0) {
    pass(
      CHECK,
      `All ${coveredTests} Rust test modules covered (${warnedTests} warned, packages not scaffolded)`,
    );
  }
}

// ---------------------------------------------------------------------------
// Check 8: Rust source drift detection (hash manifest)
// ---------------------------------------------------------------------------

function checkRustSourceDrift() {
  const CHECK = "Rust source drift";

  const manifest = loadManifest();
  const manifestKeys = Object.keys(manifest);

  if (manifestKeys.length === 0) {
    warn(CHECK, "No hash manifest found at scripts/rust-source-hashes.json -- run with --backpopulate to create one");
    return;
  }

  let totalChecked = 0;
  let upToDate = 0;
  let drifted = 0;
  let missingRs = 0;

  for (const rsRelPath of manifestKeys) {
    const rsAbsPath = join(RS_PATH, rsRelPath);

    if (!existsSync(rsAbsPath)) {
      missingRs++;
      warn(CHECK, `Manifest references Rust file that no longer exists: ${rsRelPath}`);
      continue;
    }

    totalChecked++;
    const currentHash = sha256File(rsAbsPath);
    if (!currentHash) {
      warn(CHECK, `Could not hash Rust file: ${rsRelPath}`);
      continue;
    }

    if (currentHash !== manifest[rsRelPath]) {
      drifted++;
      warn(
        CHECK,
        `Rust file has changed since last port: ${rsRelPath} (manifest: ${manifest[rsRelPath].slice(0, 12)}... current: ${currentHash.slice(0, 12)}...)`,
      );
    } else {
      upToDate++;
    }
  }

  if (drifted === 0 && totalChecked > 0) {
    pass(CHECK, `All ${upToDate} tracked Rust files match their manifest hashes (no drift)`);
  } else if (drifted > 0) {
    warn(
      CHECK,
      `${drifted} of ${totalChecked} tracked Rust files have drifted since last port -- TS files may need updating`,
    );
  }

  if (missingRs > 0) {
    warn(CHECK, `${missingRs} Rust files in manifest no longer exist (deleted or moved)`);
  }
}

// ---------------------------------------------------------------------------
// Back-population: scan all MIRRORS annotations and build hash manifest
// ---------------------------------------------------------------------------

function backpopulateManifest(): void {
  const packagesDir = join(TS_ROOT, "packages");
  if (!existsSync(packagesDir)) {
    console.error("ERROR: packages/ directory does not exist.");
    process.exit(1);
  }

  const manifest: HashManifest = {};
  let scanned = 0;
  let hashed = 0;
  let skippedMissing = 0;

  const pkgDirs = readdirSync(packagesDir, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name);

  for (const pkg of pkgDirs) {
    const srcDir = join(packagesDir, pkg, "src");
    if (!existsSync(srcDir)) continue;

    const tsFiles = walkDir(srcDir, ".ts");

    // Also check __tests__/ directory
    const testsDir = join(packagesDir, pkg, "__tests__");
    if (existsSync(testsDir)) {
      tsFiles.push(...walkDir(testsDir, ".ts"));
    }

    for (const tsFile of tsFiles) {
      const rsRelPath = extractMirrorsPath(tsFile);
      if (!rsRelPath) continue;

      scanned++;
      const rsAbsPath = join(RS_PATH, rsRelPath);

      if (!existsSync(rsAbsPath)) {
        skippedMissing++;
        console.log(`  SKIP (missing): ${rsRelPath} (referenced by ${relative(TS_ROOT, tsFile)})`);
        continue;
      }

      const hash = sha256File(rsAbsPath);
      if (hash) {
        manifest[rsRelPath] = hash;
        hashed++;
      }
    }
  }

  saveManifest(manifest);

  console.log("");
  console.log("Back-population complete:");
  console.log(`  Scanned:  ${scanned} MIRRORS annotations`);
  console.log(`  Hashed:   ${hashed} Rust source files`);
  if (skippedMissing > 0) {
    console.log(`  Skipped:  ${skippedMissing} (Rust file not found)`);
  }
  console.log(`  Written:  ${MANIFEST_PATH}`);
  console.log("");
}

// ---------------------------------------------------------------------------
// Update manifest: recompute hashes for all tracked Rust files
// ---------------------------------------------------------------------------

function updateManifest(): void {
  const manifest = loadManifest();
  const keys = Object.keys(manifest);

  if (keys.length === 0) {
    console.error("ERROR: No existing manifest to update. Run with --backpopulate first.");
    process.exit(1);
  }

  let updated = 0;
  let removed = 0;

  for (const rsRelPath of keys) {
    const rsAbsPath = join(RS_PATH, rsRelPath);
    if (!existsSync(rsAbsPath)) {
      delete manifest[rsRelPath];
      removed++;
      console.log(`  REMOVED (file gone): ${rsRelPath}`);
      continue;
    }

    const hash = sha256File(rsAbsPath);
    if (hash && hash !== manifest[rsRelPath]) {
      console.log(`  UPDATED: ${rsRelPath} (${manifest[rsRelPath].slice(0, 12)}... -> ${hash.slice(0, 12)}...)`);
      manifest[rsRelPath] = hash;
      updated++;
    }
  }

  saveManifest(manifest);

  console.log("");
  console.log("Manifest update complete:");
  console.log(`  Checked:  ${keys.length} entries`);
  console.log(`  Updated:  ${updated}`);
  if (removed > 0) {
    console.log(`  Removed:  ${removed} (Rust files no longer exist)`);
  }
  console.log(`  Written:  ${MANIFEST_PATH}`);
  console.log("");
}

// ---------------------------------------------------------------------------
// Print report
// ---------------------------------------------------------------------------

function printReport() {
  const RESET = "\x1b[0m";
  const GREEN = "\x1b[32m";
  const RED = "\x1b[31m";
  const YELLOW = "\x1b[33m";
  const BOLD = "\x1b[1m";
  const DIM = "\x1b[2m";

  // Detect if color is supported
  const useColor = process.stdout.isTTY !== false;
  const c = (code: string, text: string) => (useColor ? `${code}${text}${RESET}` : text);

  console.log("");
  console.log(c(BOLD, "========================================"));
  console.log(c(BOLD, "  ankurah-ts Port Audit Report"));
  console.log(c(BOLD, "========================================"));
  console.log("");
  console.log(c(DIM, `Rust repo:  ${RS_PATH}`));
  console.log(c(DIM, `TS repo:    ${TS_ROOT}`));
  console.log("");

  // Group by check
  const checks = new Map<string, ReportItem[]>();
  for (const item of report) {
    const existing = checks.get(item.check) ?? [];
    existing.push(item);
    checks.set(item.check, existing);
  }

  let totalPass = 0;
  let totalFail = 0;
  let totalWarn = 0;

  // Print FAIL items first, then WARN, then PASS summary
  const failItems = report.filter((r) => r.severity === "FAIL");
  const warnItems = report.filter((r) => r.severity === "WARN");
  const passItems = report.filter((r) => r.severity === "PASS");

  if (failItems.length > 0) {
    console.log(c(RED + BOLD, "--- FAILURES ---"));
    console.log("");
    for (const item of failItems) {
      totalFail++;
      const prefix = c(RED, "[FAIL]");
      const filePart = item.file ? c(DIM, ` (${relative(process.cwd(), item.file)})`) : "";
      console.log(`  ${prefix} ${c(BOLD, item.check)}: ${item.message}${filePart}`);
    }
    console.log("");
  }

  if (warnItems.length > 0) {
    console.log(c(YELLOW + BOLD, "--- WARNINGS ---"));
    console.log("");
    for (const item of warnItems) {
      totalWarn++;
      const prefix = c(YELLOW, "[WARN]");
      const filePart = item.file ? c(DIM, ` (${relative(process.cwd(), item.file)})`) : "";
      console.log(`  ${prefix} ${c(BOLD, item.check)}: ${item.message}${filePart}`);
    }
    console.log("");
  }

  if (passItems.length > 0) {
    console.log(c(GREEN + BOLD, "--- PASSED ---"));
    console.log("");
    for (const item of passItems) {
      totalPass++;
      const prefix = c(GREEN, "[PASS]");
      console.log(`  ${prefix} ${c(BOLD, item.check)}: ${item.message}`);
    }
    console.log("");
  }

  console.log(c(BOLD, "========================================"));
  console.log(
    `  ${c(GREEN, `${totalPass} passed`)}  ${c(YELLOW, `${totalWarn} warnings`)}  ${c(RED, `${totalFail} failures`)}`,
  );
  console.log(c(BOLD, "========================================"));
  console.log("");

  return totalFail;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);

  // Verify Rust repo exists
  if (!existsSync(RS_PATH)) {
    console.error(
      `ERROR: Rust repo not found at ${RS_PATH}\n` +
        `Set ANKURAH_RS_PATH environment variable or ensure ../ankurah exists.`,
    );
    process.exit(1);
  }

  // Handle special modes
  if (args.includes("--backpopulate")) {
    console.log("Back-populating hash manifest from current MIRRORS annotations...");
    backpopulateManifest();
    return;
  }

  if (args.includes("--update-manifest")) {
    console.log("Updating hash manifest with current Rust file hashes...");
    updateManifest();
    return;
  }

  if (args.includes("--help") || args.includes("-h")) {
    console.log(`
Usage: bun run scripts/audit-port.ts [OPTIONS]

Options:
  (no args)           Run all audit checks including drift detection
  --backpopulate      Scan all MIRRORS annotations, compute SHA-256 of each
                      referenced Rust file, and write scripts/rust-source-hashes.json
  --update-manifest   Recompute hashes for all files already in the manifest
                      (use after reviewing/porting drifted files)
  --help, -h          Show this help message

Environment:
  ANKURAH_RS_PATH     Path to Rust ankurah repo (default: ../ankurah)
`);
    return;
  }

  // Run all checks
  checkRustFileCoverage();
  checkTsAnnotations();
  checkTestCoverage();
  checkRustSourceDrift();

  // Print report and exit
  const failures = printReport();
  process.exit(failures > 0 ? 1 : 0);
}

main();
