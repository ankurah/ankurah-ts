# Port Maintainability Analysis: ankurah-ts

**Date**: 2026-02-11
**Scope**: Evaluation of whether the current annotations, mappings, rules, and conventions are sufficient to enable automatic agentic updating as the authoritative Rust ankurah library evolves.

---

## 1. Executive Summary

The ankurah-ts port has an exceptionally well-designed framework for automated maintenance. The `port-rules.md` specification is thorough and mechanical, covering file naming, identifier conventions, type mappings, exception rules, and validation checklists. The `audit-port.ts` script provides a real-time compliance signal. The annotation convention (`MIRRORS` / `TS-ONLY` on line 1) is machine-parseable and consistently applied across all 86 existing TS source files. However, there are specific gaps that would impede a fully automated update workflow: the audit script lacks drift detection (timestamp/hash comparison), some filenames violate the stated snake_case convention, test file annotations include suffixes that fail the regex parser, and there is no formal struct/method-level mapping beyond file-level annotations. With targeted improvements (detailed in Section 9), this port is well-positioned for sustained agentic maintenance.

---

## 2. Annotation Quality

### What works well

- **100% annotation coverage**: Every one of the 86 TS source files has a line 1 annotation (the audit script confirms "All 86 TS source files have valid annotations").
- **Machine-parseable format**: The `// MIRRORS: ankurah/<crate>/src/<path>.rs` format is trivially extractable with a single regex: `/^\/\/\s*MIRRORS:\s*(.+)$/`. The audit script already does this.
- **Consistent structure**: Both `MIRRORS` and `TS-ONLY` annotations follow the documented pattern without deviation.
- **Exception citations**: All 5 detected mapping divergences cite exception rules (e.g., E5 for yrs->yjs, E12 for file-with-submodules).

### Issues found

1. **Test file annotation suffix breaks parsing**: Several test files use annotations like:
   ```
   // MIRRORS: ankurah/core/src/property/backend/lww.rs (tests module)
   // MIRRORS: ankurah/core/src/transaction.rs (tests)
   // MIRRORS: ankurah/core/src/node.rs (tests)
   ```
   The audit script treats `(tests module)` and `(tests)` as part of the Rust file path, causing 4 false-positive orphan/validity failures. The `continue-implementation.md` file (line 288) explicitly states "MIRRORS annotations must be bare paths -- audit script validates," and the agent working notes say "bare path only, NO extra text like `[E2]` or `(tests module)`." This rule is being violated in practice.

2. **Annotation does not encode Rust file hash or timestamp**: The MIRRORS annotation points to a file path but provides no way to detect if the Rust file has changed since the TS file was last updated. An agent seeing `// MIRRORS: ankurah/core/src/entity.rs` cannot determine if the Rust file has been modified.

### Assessment: Strong foundation, minor consistency fixes needed.

---

## 3. Divergence Documentation

### Exception citations (E1-E18)

The exception rule system is well-designed:

- **18 exception rules** cover every known category of Rust-to-TS divergence, from proc macros (E1) to tokio channels (E18).
- **Each exception has**: a Rust pattern, a TS equivalent, and a justification.
- **Inline citation format**: `// Divergence: <description> [E8]` is consistently used. For example, in `entity.ts`:
  ```typescript
  // Divergence: Rust uses Arc<AtomicBool> for trx_alive; TS uses shared { value: boolean } [E8].
  // Divergence: No Arc -- plain class instance (JS single-threaded, GC handles memory) [E8].
  // Divergence: No RwLock needed -- single-threaded JS [E8].
  ```
  This file alone has 11 inline divergence comments, all citing specific E-numbers.

- **In `transaction.ts`**: 5 divergence comments in the class-level doc block, each with an E-number.
- **In `yjs.ts`**: Explicit E5 and E8 citations on line 2 and throughout.
- **In `lww.ts`**: E8 citations for every `RwLock`/`Mutex` replacement.

### Quality of divergence comments

The inline comments go beyond simple citations -- they describe *what* changed and *why*:
```typescript
// Divergence: Rust uses RwLock<BTreeMap<PropertyName, ValueEntry>>; TS uses plain Map [E8].
```
This is highly valuable for an agent, because it provides the semantic context needed to understand whether a Rust change in this area would require a corresponding TS update.

### Gaps

- Some files (like `candidate-changes.ts`) have divergence documentation only in the class-level JSDoc, not with inline E-number citations. The style is: "Divergences from Rust: `Arc<Vec<C>>` is just a readonly array..." -- this is readable but uses a non-standard format (no `[E8]` tag). An agent scanning for `[E\d+]` would miss these.
- The `parser.ts` file cites E6 on line 2 but has no further divergence annotations even though the entire implementation is structurally different from Rust (hand-written parser vs Pest-generated). This is arguably correct since the whole file is a divergence, but an agent might not know what level of structural tracking to expect.

### Assessment: Excellent. The exception citation system is the strongest aspect of the port's maintainability framework.

---

## 4. Structural Fidelity

### File-level mapping

Comparing Rust and TS file structures:

| Aspect | Fidelity | Notes |
|--------|----------|-------|
| File names | High | 1:1 mapping with documented exceptions (E2, E5, E12) |
| Directory structure | High | `proto/src/` -> `packages/proto/src/`, `core/src/property/backend/` -> same |
| Struct/class names | High | `Entity`, `Transaction`, `LWWBackend`, `YjsBackend`, `ComparisonIndex` all match |
| Method names | High | `generate_commit_event` -> `generateCommitEvent`, `to_state` -> `toState` (camelCase) |
| Field names | High | `entity_id` -> `entityId`, `trx_alive` -> `trxAlive` (camelCase) |
| Enum variants | High | `EntityKind::Primary` -> `{ type: 'Primary' }`, `EntityKind::Transacted` -> `{ type: 'Transacted', ... }` |
| Field order in structs | High | Proto types maintain identical field order for bincode compatibility |

### Detailed comparison: `entity.rs` vs `entity.ts`

The Rust file has:
- `Entity(Arc<EntityInner>)` -- TS has `class Entity` with fields inlined
- `EntityInnerState { head, backends }` -- TS has `interface EntityInnerState { head, backends }`
- `EntityKind { Primary, Transacted { trx_alive, upstream } }` -- TS has discriminated union type
- `impl Entity { create, from_state, head, is_writable, to_state, generate_commit_event, snapshot, view, get_backend }` -- TS has all matching methods
- `WeakEntitySet(Arc<RwLock<BTreeMap<EntityId, WeakEntity>>>)` -- TS has `class WeakEntitySet` with `Map<string, WeakRef<Entity>>`
- `TemporaryEntity` -- not yet ported (noted in Rust TODO comments)

Key differences that are well-documented:
- `Arc<AtomicBool>` -> `{ value: boolean }` [E8]
- `Arc<dyn PropertyBackend>` -> plain interface reference [E8]
- `RwLock<EntityInnerState>` -> direct property access [E8]

Key differences that might cause drift problems:
- The Rust `apply_event` is significantly more complex (full lineage comparison with retry loop, TOCTOU protection via `try_mutate`). The TS version is simplified with `// Note: Full lineage comparison deferred`. When the lineage module is ported, this will need updating -- but there is no tracking mechanism for this deferred work at the annotation level.
- The Rust `WeakEntitySet::with_state` takes a `Retrieve` generic and does storage retrieval. The TS version is simplified. Again, no annotation tracks this simplification.

### Detailed comparison: `transaction.rs` vs `transaction.ts`

Very close structural match:
- Both have `dyncontext`, `id`, `entities`, `alive`, `created_entity_ids` fields
- Both have `create`, `get`, `edit`, `commit`, `rollback` methods
- Method signatures differ only by language conventions (lifetimes removed, `Arc<AtomicBool>` -> shared ref)
- The TS file adds `values` parameter to `create()` that the Rust version doesn't have (Rust uses `model.initialize_new_entity` which is macro-generated). This difference is undocumented.

### Detailed comparison: `comparison_index.rs` vs `comparison-index.ts`

Close structural match despite different data structures:
- Rust uses `HashMap` for eq/ne, `BTreeMap` for gt/lt. TS uses `Map` for eq/ne, sorted arrays for gt/lt.
- The TS file includes a binary search implementation for sorted array maintenance that replaces BTreeMap's built-in ordered iteration.
- `find_matching` logic is functionally equivalent: same eq/ne/gt/lt scanning with same boundary conditions.
- The `for_entry` method structure is preserved 1:1 including the >= -> predecessor and <= -> successor transformations.

### Could a diff tool propagate changes?

For most changes -- **yes**, with caveats:
- **Adding a new field to a struct**: An agent could detect the new field in Rust and add it to the corresponding TS class. The MIRRORS annotation provides the file mapping.
- **Adding a new method**: The agent could identify the new `impl` block method and add a corresponding TS method.
- **Changing method logic**: This requires understanding whether the change is in "universal logic" or "Rust-specific logic" (e.g., concurrency). The divergence comments help but are not exhaustive.
- **Adding a new file**: The audit script would immediately flag it as a missing TS file, creating a clear signal.

### Assessment: High fidelity. The structural mapping is close enough for automated propagation of most changes. The main risk is in areas with deferred/simplified implementations that lack explicit tracking annotations.

---

## 5. Naming Convention Compliance

### File naming

Port-rules.md rule A1 states: "Rust `foo_bar.rs` maps to TS `foo_bar.ts` (preserve snake_case filenames for 1:1 mapping)."

**Violations found**: 3 files in `packages/core/src/reactor/` use kebab-case instead of snake_case:

| Rust filename | Expected TS filename | Actual TS filename | Status |
|---------------|---------------------|-------------------|--------|
| `comparison_index.rs` | `comparison_index.ts` | `comparison-index.ts` | VIOLATION |
| `candidate_changes.rs` | `candidate_changes.ts` | `candidate-changes.ts` | VIOLATION |
| `property_path.rs` | `property_path.ts` | `property-path.ts` | VIOLATION |
| `fetch_gap.rs` | `fetch_gap.ts` | `fetch_gap.ts` | CORRECT |
| `update.rs` | `update.ts` | `update.ts` | CORRECT |

These three files were created at an earlier timestamp (Feb 10 17:45-17:46) compared to `fetch_gap.ts` (Feb 11 09:51), suggesting the convention was clarified between the two batches. Notably, the MIRRORS annotations inside these files point to the correct Rust paths (`ankurah/core/src/reactor/comparison_index.rs`) but the filenames themselves don't match, so the audit script correctly flags them as "Missing TS file" even though the file exists under a different name.

The audit script reports these as missing because it constructs the expected path using the Rust filename convention. This is correct behavior -- the audit script works as intended, it's the files that are wrong.

### Identifier naming

The snake_case to camelCase mapping (A2) is consistently applied:
- `generate_commit_event` -> `generateCommitEvent`
- `to_state` -> `toState`
- `is_writable` -> `isWritable`
- `entity_id` -> `entityId`
- `trx_alive` -> `trxAlive`
- `created_entity_ids` -> `createdEntityIds`
- `property_backend_name` -> `propertyBackendName`
- `from_state_buffer` -> `fromStateBuffer`

No identifier naming violations were found across all sampled files.

### Type naming

PascalCase is preserved correctly in both languages: `Entity`, `Transaction`, `LWWBackend`, `YjsBackend`, `ComparisonIndex`, `CandidateChanges`, `QueryCandidate`, `EntityKind`, `MutableBorrow`.

### Assessment: The identifier mapping is flawless. The file naming has 3 specific violations that should be fixed (simple renames).

---

## 6. Audit Script Coverage

### What it checks

The audit script (`scripts/audit-port.ts`, 795 lines) performs 7 categories of checks:

1. **Rust file coverage**: For every in-scope Rust `.rs` file, verifies a corresponding TS file exists. Handles filename exceptions (E2, E5, E12), WASM-only skips (E9), feature-gated skips (E10), de-scoped files, and file-with-submodules detection.

2. **TS file annotations**: Every TS file in `packages/*/src/` and `packages/*/__tests__/` must have a valid `// MIRRORS:` or `// TS-ONLY:` on line 1.

3. **MIRRORS validity**: Every `MIRRORS` annotation must point to an existing Rust file. Tests this by resolving the path against `ANKURAH_RS_PATH`.

4. **TS-ONLY validity**: Verifies that TS-ONLY files don't have a corresponding Rust file (which would mean they should use MIRRORS instead). Includes reverse-mapping logic for index.ts -> lib.rs/mod.rs and E5/E12 exceptions.

5. **Orphan detection**: Identifies TS files whose MIRRORS annotation points to a non-existent Rust file (subsumes check 3).

6. **Exception citations**: For files with known mapping divergences (yrs->yjs, file-with-submodules), verifies the exception rule is cited in the file content.

7. **Test coverage**: For every in-scope Rust file containing `#[cfg(test)]`, `#[test]`, or `#[tokio::test]`, verifies a corresponding `.test.ts` file exists.

### What it does NOT check

1. **Drift detection**: No comparison of file modification timestamps or content hashes. If the Rust `entity.rs` changes, the audit script has no way to flag that `entity.ts` needs updating.

2. **Struct/method-level mapping**: The script operates at file granularity. It cannot detect if a new struct field was added in Rust but not in TS, or if a method signature changed.

3. **Naming convention compliance**: Does not verify that TS filenames use snake_case matching Rust (the 3 kebab-case violations go undetected as missing files, but the script doesn't explain *why* they're missing).

4. **Import path correctness**: Does not validate that cross-package imports (`@ankurah/proto`, `@ankurah/core`) or intra-package imports use the correct paths per port-rules.md Section F.

5. **Re-export chain validation**: Does not verify that `index.ts` re-exports match `lib.rs`/`mod.rs` re-exports.

6. **Divergence comment coverage**: Does not check whether inline `// Divergence:` comments are present where they should be.

7. **Content hash tracking**: Does not store or compare any hash of either Rust or TS file contents for ongoing monitoring.

### Current audit results

The audit currently reports:
- **2 passed** (annotations, exception citations)
- **0 warnings**
- **102 failures** (56 missing TS files for not-yet-ported Rust files, 4 annotation parsing errors from test file suffixes, 2 TS-ONLY/MIRRORS mismatches in storage packages, ~40 missing test files)

Most failures are expected -- they represent work-in-progress (Layers 5b onwards not yet implemented). The 4 annotation parsing errors and 3 filename convention violations are genuine bugs.

### Assessment: The audit script is solid for its current scope (file-level bidirectional mapping) but lacks the drift detection capabilities needed for ongoing automated maintenance.

---

## 7. Drift Detection Feasibility

### Current state: No drift detection

The current system has no mechanism to detect when a Rust file changes. The MIRRORS annotation is a static pointer -- it says "this TS file corresponds to that Rust file" but provides no versioning.

### Proposed drift detection approaches

**Approach A: File hash manifest** (Recommended)

Maintain a `.port-manifest.json` file tracking the last-synced state of each Rust file:

```json
{
  "ankurah/core/src/entity.rs": {
    "rustHash": "sha256:abc123...",
    "tsFile": "packages/core/src/entity.ts",
    "tsHash": "sha256:def456...",
    "lastSynced": "2026-02-10T17:45:00Z"
  }
}
```

The audit script could then:
1. Hash every in-scope Rust file
2. Compare against the manifest
3. Report any Rust files whose hash has changed since last sync

**Approach B: Git-based diff tracking**

Since the Rust repo is a sibling checkout, the audit script could:
1. Run `git log --since="<last-sync-date>" --name-only` in the Rust repo
2. Intersect changed files with the MIRRORS mapping
3. Report which TS files need review

**Approach C: MIRRORS annotation with commit hash**

Extend the MIRRORS annotation to include the Rust file's git commit hash:
```typescript
// MIRRORS: ankurah/core/src/entity.rs @ abc1234
```

The audit script could then verify the hash matches the current HEAD of the Rust file.

### Feasibility assessment

All three approaches are technically straightforward. Approach A is the most self-contained and doesn't require git access. Approach C is the most elegant but requires updating annotations on every sync. Approach B is the most lightweight for CI.

### Assessment: Drift detection is feasible and should be the highest-priority enhancement.

---

## 8. Automated Update Workflow

Given the current annotation and mapping infrastructure, here is a proposed workflow for how an agent could handle Rust-to-TS updates:

### Phase 1: Detection

1. **Run drift detection** (once implemented): Compare Rust file hashes against manifest. Produce a list of changed Rust files.
2. **Run audit script**: Identify any new Rust files that lack TS counterparts.
3. **Prioritize**: Categorize changes as:
   - **New file**: Rust file exists, no TS counterpart -> needs full port
   - **Modified file**: Rust file changed, TS counterpart exists -> needs update
   - **Deleted file**: Rust file removed, TS counterpart exists -> needs removal

### Phase 2: Analysis (per changed file)

1. **Diff the Rust file**: `git diff <last-synced-commit>..HEAD -- <rust-file>`
2. **Categorize changes**:
   - **Structural**: New struct/enum/field/method additions -> likely needs TS update
   - **Logic**: Changed method bodies -> may need TS update (check if in a divergence zone)
   - **Concurrency**: Added/changed `Arc`/`RwLock`/`Mutex` -> likely TS can ignore [E8]
   - **Feature-gated**: Changes inside `#[cfg(feature = "wasm")]` -> TS can ignore [E9]
   - **Tests**: Changes inside `#[cfg(test)]` -> TS test file needs update [E3]
3. **Read existing TS file**: Check inline divergence comments to understand what was intentionally different.

### Phase 3: Update

1. **Apply structural changes**: Add new fields, methods, types. Follow naming convention (A2).
2. **Apply logic changes**: Port method body changes, respecting documented divergences.
3. **Add divergence comments**: For any new divergences, cite the appropriate E-number.
4. **Update MIRRORS hash**: If using Approach C for drift detection.

### Phase 4: Verification

1. **Run `tsc --noEmit`**: Type-check all packages.
2. **Run `bun test`**: Execute all tests.
3. **Run audit script**: Verify no new failures.
4. **Update manifest**: Record new Rust file hashes.

### Confidence assessment

This workflow is viable for:
- **New struct fields**: High confidence (mechanical)
- **New methods**: High confidence (mechanical + naming convention)
- **Changed method logic**: Medium confidence (requires understanding divergences)
- **New files**: Medium confidence (needs to understand which package, exceptions, etc.)
- **Architectural changes** (e.g., new trait, reorganized modules): Low confidence (requires human review)

### Assessment: The current infrastructure supports an automated update workflow for the majority of changes. The main gap is drift detection (Phase 1).

---

## 9. Gaps and Recommendations

### Critical (blocks automated maintenance)

1. **Implement drift detection in the audit script**. Without this, there is no automated way to know when Rust changes occur. Recommended: file hash manifest approach (see Section 7, Approach A). This is the single most impactful improvement.

2. **Fix the 3 kebab-case filename violations**. Rename:
   - `comparison-index.ts` -> `comparison_index.ts`
   - `candidate-changes.ts` -> `candidate_changes.ts`
   - `property-path.ts` -> `property_path.ts`

   Update all internal imports accordingly. This will immediately resolve 6 audit failures (3 "missing TS file" + 3 corresponding "missing test file").

3. **Fix test file annotations**. Change from:
   ```
   // MIRRORS: ankurah/core/src/transaction.rs (tests)
   ```
   to:
   ```
   // MIRRORS: ankurah/core/src/transaction.rs
   ```
   Per the documented rule (bare path only). This resolves 4 audit failures.

### Important (significantly improves maintenance quality)

4. **Add a "SIMPLIFIED" or "DEFERRED" annotation for incomplete ports**. Currently, when a TS file simplifies the Rust logic (e.g., `entity.ts` omitting lineage comparison), there is only a free-text comment like "Full lineage comparison deferred." A machine-parseable annotation would help:
   ```typescript
   // DEFERRED: lineage comparison (see ankurah/core/src/lineage.rs)
   ```
   The audit script could then track these and flag them when the deferred Rust file is modified.

5. **Add naming convention validation to the audit script**. Verify that TS filenames match the expected snake_case convention derived from Rust filenames. This would have caught the kebab-case violations automatically.

6. **Standardize divergence comment format**. Some files use `// Divergence: ... [E8]` (consistent, greppable), others use paragraph-style descriptions in JSDoc without E-number tags. Standardize on the `[E<n>]` suffix format for machine extraction.

7. **Add import path validation to the audit script**. Verify that `@ankurah/proto` imports map to `ankurah_proto::` paths in Rust, and that relative imports follow the documented patterns in Section F of port-rules.md.

### Nice-to-have (improves long-term maintainability)

8. **Add struct/method-level mapping comments**. For key types, annotate the Rust source locations:
   ```typescript
   // See Rust: ankurah/core/src/entity.rs:153-169 (generate_commit_event)
   async generateCommitEvent(): Event | null {
   ```
   This already appears in some files (e.g., `EventId.fromParts`). Making it systematic would enable line-level diff tracking.

9. **Document the `Transaction.create()` divergence**. The TS version accepts a `values` parameter that the Rust version doesn't have (Rust uses `model.initialize_new_entity()` which is proc-macro-generated). This undocumented divergence should have a comment explaining why.

10. **Add a "last ported from" comment or manifest entry per file**. Something like:
    ```typescript
    // MIRRORS: ankurah/core/src/entity.rs
    // PORTED-FROM-COMMIT: abc1234def (2026-02-10)
    ```
    This would enable precise git-diff-based change detection.

11. **Consider generating a bidirectional mapping index**. A JSON file that lists every Rust struct/method and its TS counterpart, auto-generated by scanning both codebases. This would make it trivial to answer "what TS code do I need to update when `Entity::apply_event` changes?"

---

## 10. Conclusion

### Overall maintainability score: 7.5/10

**Strengths** (what makes this port highly maintainable):
- Exhaustive, mechanical port rules (`port-rules.md`) covering every mapping scenario
- 100% annotation coverage with machine-parseable `MIRRORS`/`TS-ONLY` format
- Comprehensive exception rule system (E1-E18) with consistent inline citations
- Structural fidelity is high -- an agent can reasonably map Rust changes to TS
- Working audit script that catches file-level compliance issues
- Flawless identifier naming convention compliance (snake_case -> camelCase)
- 309 passing tests provide a strong regression safety net

**Weaknesses** (what needs improvement for full automated maintenance):
- No drift detection (the single biggest gap)
- 3 filename convention violations
- 4 test annotation format errors
- No struct/method-level mapping (file-level only)
- Some divergence comments use non-standard format
- Simplified implementations lack machine-parseable tracking annotations

### Next steps (priority order)

1. Add drift detection via file hash manifest (Section 7, Approach A)
2. Fix 3 kebab-case filenames and 4 test annotations (immediate, mechanical)
3. Add naming convention validation to audit script
4. Standardize divergence comment format with `[E<n>]` suffix
5. Add `DEFERRED` annotation for incomplete implementations
6. Continue porting Layers 5b-5c and remaining files to reduce the 56 missing-file failures

With these improvements, the port would reach a 9/10 maintainability score, making it viable for sustained agentic maintenance with minimal human oversight for routine Rust changes.
