# Ongoing Maintenance Workflow

## Overview

This document describes the process for keeping ankurah-ts in sync with the Rust ankurah codebase as it evolves. It is designed to be executable by agentic workflows with minimal human oversight.

## Repository Layout Assumption

ankurah-ts assumes a **sibling checkout** of ankurah/ for:
- Bincode reference fixtures: `../ankurah/proto/tests/fixtures/bincode/`
- Schema export: `../ankurah/` (cargo build required)
- Structural comparison: `../ankurah/` file tree

```
ak/                          # Parent directory
├── ankurah/                 # Rust implementation (git repo)
├── ankurah-ts/              # TypeScript port (git repo)
│   ├── specs/
│   ├── packages/
│   ├── cli/
│   └── ...
```

The ankurah-ts CI should:
1. Clone both repos as siblings
2. Build Rust fixtures before running TS tests
3. Validate structural correspondence

## Change Detection

### Automated: CI-based Drift Detection

A scheduled CI job (daily or on ankurah push) detects when the Rust codebase has changed in ways that affect the TS port.

```yaml
# .github/workflows/drift-detection.yml
name: Drift Detection
on:
  schedule:
    - cron: '0 6 * * *'  # Daily at 6am
  workflow_dispatch:

jobs:
  detect-drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          repository: ankurah/ankurah
          path: ankurah
          fetch-depth: 0

      - uses: actions/checkout@v4
        with:
          path: ankurah-ts

      - name: Get last synced commit
        run: cat ankurah-ts/.ankurah-sync-commit

      - name: Detect changes since last sync
        run: |
          cd ankurah
          LAST_SYNC=$(cat ../ankurah-ts/.ankurah-sync-commit)
          git diff --name-only $LAST_SYNC HEAD -- \
            proto/src/ \
            core/src/ \
            signals/src/ \
            ankql/src/ \
            storage/common/src/ \
            storage/sqlite/src/ \
            connectors/websocket-client/src/ \
            connectors/local-process/src/ \
            tests/ \
          > ../drift-report.txt

      - name: Classify changes
        run: |
          # For each changed Rust file, identify the corresponding TS file
          # and check if the TS file was updated after the Rust change
          python3 ankurah-ts/scripts/classify-drift.py \
            --rust-changes drift-report.txt \
            --mapping ankurah-ts/file-mapping.json \
            --output drift-classification.json

      - name: Report
        if: always()
        run: |
          # Post to GitHub issue or Slack if drift detected
```

### File Mapping Registry

A machine-readable mapping between Rust and TS files:

```json
// ankurah-ts/file-mapping.json
{
  "mappings": [
    {
      "rust": "proto/src/lib.rs",
      "ts": "packages/proto/src/index.ts",
      "quality": "direct",
      "notes": "Module exports, EntityId, CollectionId"
    },
    {
      "rust": "proto/src/data.rs",
      "ts": "packages/proto/src/data.ts",
      "quality": "direct",
      "notes": "Event, EntityState, Operation types"
    },
    {
      "rust": "core/src/entity.rs",
      "ts": "packages/core/src/entity.ts",
      "quality": "direct",
      "notes": "Entity class"
    },
    {
      "rust": "core/src/property/backend/yrs.rs",
      "ts": "packages/core/src/property/backend/yjs.ts",
      "quality": "near-direct",
      "notes": "Uses Yjs API instead of Yrs API"
    }
  ]
}
```

### Sync State Tracking

```
# ankurah-ts/.ankurah-sync-commit
# This file tracks the last ankurah commit that ankurah-ts is synced to.
# Updated by the sync workflow after successful validation.
abc123def456...
```

## Update Workflow

When drift is detected, an agentic workflow processes the changes:

### Step 1: Analyze Rust Diff

```
Input: Git diff of changed Rust files since last sync
Output: Classified change list
```

For each changed Rust file:
1. Identify the corresponding TS file from `file-mapping.json`
2. Classify the change type:
   - **Type change**: Struct/enum field added/removed/renamed
   - **Method change**: Impl method signature or body changed
   - **New file**: New module added
   - **Deleted file**: Module removed
   - **Refactor**: Internal restructuring (same API)
   - **Test change**: Test added/modified
   - **Dependency change**: New crate dependency

### Step 2: Generate TS Patches

For each classified change, generate the corresponding TS modification:

**Type change example:**
```
Rust diff: Added field `pub score: f64` to struct UserState
→ TS action: Add `score: number` field to UserState interface in packages/proto/src/data.ts
```

**Method change example:**
```
Rust diff: Changed `fn get_property_value` to accept `PropertyId` instead of `&str`
→ TS action: Update `getPropertyValue` method signature in packages/core/src/entity.ts
```

**New file example:**
```
Rust diff: New file core/src/property/backend/yrs_map.rs
→ TS action: Create packages/core/src/property/backend/yjs-map.ts with equivalent implementation
```

### Step 3: Apply and Validate

1. Apply the generated TS patches
2. Run type checking: `tsc --noEmit`
3. Run unit tests: `vitest run`
4. Run bincode compatibility tests (re-generate Rust fixtures first)
5. Run integration tests
6. If all pass, create a PR with the changes

### Step 4: Update Sync State

After successful merge:
```
echo "new-commit-hash" > ankurah-ts/.ankurah-sync-commit
```

## Schema Sync Workflow

When Rust model definitions change:

1. **Regenerate schema.json**:
   ```bash
   cd ../ankurah && cargo run --bin ankurah-schema-export > ../ankurah-ts/schema.json
   ```

2. **Regenerate TS model wrappers**:
   ```bash
   cd ankurah-ts && npx @ankurah/cli generate --schema schema.json --output packages/models/src/generated/
   ```

3. **Type-check consuming code** to catch any breaking changes
4. **Update tests** if model shapes changed

## Bincode Fixture Sync

When proto types change:

1. **Regenerate Rust fixtures**:
   ```bash
   cd ../ankurah && cargo test -p ankurah-proto generate_reference_fixtures
   ```

2. **Run TS compatibility tests** (they read from `../ankurah/proto/tests/fixtures/bincode/`):
   ```bash
   cd ankurah-ts && npx vitest run packages/proto/tests/bincode-compat
   ```

3. **If tests fail**: Update the TS bincode encoder/decoder to match new Rust encoding

## Agent Instructions: Applying a Rust Change to TypeScript

When an agent is tasked with syncing a specific Rust change to ankurah-ts, follow this procedure:

### 1. Understand the Rust change

```
Read the Rust diff carefully. Identify:
- What types/traits/impls were modified
- What is the semantic meaning of the change
- What tests were added/modified
```

### 2. Locate the corresponding TS file

```
Consult file-mapping.json to find the TS counterpart.
If no mapping exists (new file), create a new mapping entry.
```

### 3. Apply the equivalent change

```
For each Rust change:
- struct field added → add field to TS interface/class
- method signature changed → update TS method signature
- new impl method → add TS class method
- enum variant added → add to TS discriminated union
- new module → create new TS file with equivalent exports
- test added → write equivalent TS test

Preserve:
- The same method names (camelCase)
- The same field names (camelCase)
- The same type semantics
- The same test names and assertions
```

### 4. Verify

```
1. tsc --noEmit (type checking)
2. vitest run (unit tests)
3. vitest run --grep "bincode" (compatibility tests)
4. vitest run --grep "integration" (integration tests)
```

### 5. Handle special cases

- **Concurrency changes** (Arc, Mutex, etc.): Usually simplify or no-op in TS
- **Lifetime changes**: No equivalent in TS, may need dispose pattern
- **Feature flag changes**: Map to build configuration or runtime checks
- **Macro changes**: Update CLI code generator templates
- **Dependency changes**: Find TS equivalent or port inline

## Agent Instructions: Validating Port Correctness

An agent can validate the port by:

### Structural Validation

```
For each file in file-mapping.json:
1. Read the Rust file, extract public API surface (structs, enums, traits, impl methods)
2. Read the TS file, extract public API surface (interfaces, types, classes, methods)
3. Compare:
   - Every Rust public type has a TS counterpart
   - Every Rust public method has a TS counterpart
   - Field/parameter types are equivalent per the type mapping
4. Report any discrepancies
```

### Behavioral Validation

```
For each integration test in ankurah/tests/:
1. Find the corresponding test in ankurah-ts/tests/
2. Verify they test the same scenarios
3. Run both test suites
4. Compare results (both should pass with equivalent outcomes)
```

### Wire Compatibility Validation

```
1. Build Rust bincode fixtures: cargo test -p ankurah-proto generate_reference_fixtures
2. Run TS bincode tests: vitest run --grep "bincode"
3. All fixtures must decode correctly and re-encode to identical bytes
```

## Automation Opportunities

### GitHub Actions: Auto-sync PR

When ankurah/ pushes to main:
1. Drift detection job runs
2. If changes affect mapped files:
   - Agent generates TS patches
   - Opens PR on ankurah-ts with the patches
   - CI validates the patches
   - Human reviews and merges (or auto-merge if tests pass)

### Pre-commit Hook (ankurah/ repo)

After committing Rust changes, automatically:
1. Re-export schema.json
2. Re-generate bincode fixtures
3. Commit the updated fixtures

### Watch Mode (Development)

For active development where both repos are being modified:
```bash
# In ankurah-ts/
npx chokidar '../ankurah/proto/src/**/*.rs' '../ankurah/core/src/**/*.rs' \
  --command 'node scripts/detect-drift.js'
```

## Metrics and Monitoring

Track over time:
- **Sync lag**: Days since last successful sync
- **Drift count**: Number of Rust files changed without corresponding TS changes
- **Test parity**: Percentage of Rust tests that have TS equivalents
- **API surface coverage**: Percentage of Rust public API reflected in TS
- **Bincode fixture coverage**: Percentage of wire types with reference fixtures
