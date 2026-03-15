# Spec File Survey

**Date**: 2026-03-14
**Context**: 376 tests passing, Layers 0-6b complete, Layer 7 partially done (NodeApplier done, peer_subscription and node networking remaining). Memory model spec finalized in `specs/memory-model/`.

**Authoritative specs** (not surveyed here, known-good):
- `continue-implementation.md` — status tracker (has stale sections, see bottom of this doc)
- `architectural-decisions.md` — user-confirmed decisions
- `port-rules.md` — structural mapping rules
- `memory-model/` — lifecycle management (newly finalized)

---

## Individual File Assessments

### 1. `architecture.md`
**Purpose**: High-level architecture overview — layer diagram, module mapping, type correspondences, serialization strategy, React Native integration, package structure.
**Verdict: DELETE**
- Superseded by the combination of `continue-implementation.md` (status + package structure), `port-rules.md` (structural mapping), and `architectural-decisions.md` (decisions).
- Contains known errors flagged in `ankurah-rs-spec-cleanup.md`: `NodeMessageBody` does not exist (line 114), lineage de-scope is too broad (lines 227-235), contradicts codegen decision (line 224).
- The module mapping table duplicates `continue-implementation.md` lines 98-118.
- The type correspondences table is a less precise version of what `port-rules.md` provides.
- Nobody needs a third place to look for information that lives more accurately in two other files.

### 2. `ecosystem-research.md`
**Purpose**: Initial research findings on expo-sqlite, CRDT libraries (Yjs viable, Automerge/Loro not), Expo Go constraints, WebSocket API, existing solutions, Rust internals (bincode rules, EventId computation, RON configs, wire protocol).
**Verdict: KEEP (reference)**
- The expo-sqlite API reference, CRDT viability comparison, and Expo Go constraints remain useful context for anyone onboarding.
- The Rust internals section (bincode encoding rules, EventId computation) is accurate reference material.
- Has known V1/V2 error on line 39 (flagged in `ankurah-rs-spec-cleanup.md`). Fix that one line if keeping.
- Low maintenance burden since it describes external constraints that rarely change.

### 3. `fetch-gap-port-spec.md`
**Purpose**: Detailed port spec for `fetch_gap.rs` -> `fetch-gap.ts`. Import mapping, types, interfaces, function-by-function translation.
**Verdict: DELETE**
- Work is done. `fetch_gap.ts` is ported, tested, and committed (Layer 5b).
- A completed port spec has zero future readers. The source code + MIRRORS annotation is the reference now.

### 4. `livequery-port-spec.md`
**Purpose**: Detailed port spec for `livequery.rs` -> `livequery.ts`. Import mapping, ChangeSet type, function translation.
**Verdict: DELETE**
- Work is done. `livequery.ts` is ported (726 lines), tested, and committed (Layer 6a).
- Same reasoning as fetch-gap: completed port specs serve no ongoing purpose.

### 5. `initial-porting-workflow.md`
**Purpose**: 13-phase step-by-step guide for the initial port. Monorepo setup, phase ordering, file lists per phase.
**Verdict: DELETE**
- Entirely historical. The initial port phases are done through Layer 6b.
- Contains multiple known errors: file inventories undercount actual Rust files (flagged in `ankurah-rs-spec-cleanup.md` Section 4), `update.rs` vs `message.rs` confusion, reactor described as single file.
- `continue-implementation.md` is the authoritative sequencing document. This adds nothing.

### 6. `ongoing-maintenance-workflow.md`
**Purpose**: CI-based drift detection workflow, scheduled diff jobs, automated patch application process.
**Verdict: KEEP (aspirational)**
- Describes infrastructure that does not exist yet but will be needed.
- The drift detection CI concept (daily diff of Rust source against MIRRORS annotations) is still the right approach.
- Low maintenance burden. Revisit when Layer 9 (integration) is reached.

### 7. `port-maintainability-analysis.md`
**Purpose**: Assessment of whether annotations, mappings, and conventions are sufficient for automated agentic maintenance. Scores the port 7.5/10 with specific gap analysis.
**Verdict: DELETE**
- Point-in-time analysis from 2026-02-11 when the port was at ~45%.
- Its findings have been acted on or absorbed: test annotation suffix issue is documented, SOURCE-HASH annotations are now in `continue-implementation.md`, structural mapping analysis is captured elsewhere.
- The "7.5/10 score" is stale (the port has advanced significantly since).
- `ankurah-rs-spec-cleanup.md` captures the actionable items better.

### 8. `progress-and-parallelism-review.md`
**Purpose**: Snapshot of progress at 2026-02-11. Package completion percentages, layer status, parallelization opportunities, test counts (309 tests at the time).
**Verdict: DELETE**
- Completely stale. Reports 309 tests (now 376), Layer 5b as "in progress" (now Layer 7), and ~45% core completion (now much higher).
- `continue-implementation.md` is the living status document and is kept up to date.
- Historical snapshots provide no value when the living document exists.

### 9. `schema-registry-and-codegen.md`
**Purpose**: Design for schema registry system based on Rust PR #236. Schema flow from Rust struct to registry to TS codegen.
**Verdict: KEEP (future reference)**
- PR #236 is still in progress on the Rust side. This spec will be needed when Layer 9+ work reaches codegen.
- Content is forward-looking and not yet implementable.
- Low maintenance burden since it tracks an external dependency.

### 10. `storage-memory-impl-spec.md`
**Purpose**: Implementation spec for `@ankurah/storage-memory` package. Class design, imports, method signatures.
**Verdict: DELETE**
- Work is done. `storage-memory` is implemented, tested, and committed (Layer 6b).
- Completed implementation specs serve no ongoing purpose.

### 11. `structural-mapping-analysis.md`
**Purpose**: Detailed file-level Rust->TS structural mapping. Tables showing every file pair, mapping quality, and notes.
**Verdict: DELETE**
- Superseded by `port-rules.md` (which codifies the rules) and the MIRRORS annotations in the source code (which are the ground truth).
- Contains stale claim of "~88% directly mappable" based on incomplete file count (flagged in `ankurah-rs-spec-cleanup.md`).
- The audit script (`scripts/audit-port.ts`) provides live mapping validation, making a static analysis document redundant.

### 12. `system-port-spec.md`
**Purpose**: Detailed port spec for `system.rs` -> `system.ts`. Field mappings, type translations, method signatures.
**Verdict: DELETE**
- Work is done. `system.ts` is ported with tests (Layer 6a/6b).
- Same reasoning as other completed port specs.

### 13. `watcher-set.md`
**Purpose**: Detailed port spec for `watcherset.rs` -> `watcher_set.ts`. Registry design, imports, prerequisite types.
**Verdict: DELETE**
- Work is done. `watcher_set.ts` is ported and tested (Layer 5b, reactor fully complete).
- Same reasoning as other completed port specs.

### 14. `wire-format-interop.md`
**Purpose**: Wire protocol compatibility strategy. Bincode encoding details, hybrid approach recommendation, reference test suite design.
**Verdict: DELETE**
- Contains multiple known errors (flagged in `ankurah-rs-spec-cleanup.md`): fake `NodeMessage` struct shape, wrong `Operation` type, wrong `Event.operations` type.
- The bincode codec is already implemented and validated via fixture parity tests (24 tests, 244 assertions).
- `ecosystem-research.md` covers the bincode encoding rules accurately. The proto fixture tests are the living validation.

### 15. `yrs-yjs-interop-validation.md`
**Purpose**: Plan for validating Yrs/Yjs encoding compatibility. Test strategy, known compat status, potential gotchas.
**Verdict: DELETE**
- Work is done. Yrs/Yjs V2 interop is validated with 10 tests across 6 fixture files.
- Contains known V1/V2 errors throughout (flagged in `ankurah-rs-spec-cleanup.md`).
- The interop tests in the codebase are the living validation.

### 16. `ankurah-rs-spec-cleanup.md`
**Purpose**: Checklist of factual errors across specs, organized by category: V1/V2 errors, duplicate proto definitions, incorrect type descriptions, stale file inventories, scope corrections, TODO specs.
**Verdict: PARTIAL — depends on deletions**
- If the files it references are deleted (architecture.md, wire-format-interop.md, yrs-yjs-interop-validation.md, initial-porting-workflow.md, structural-mapping-analysis.md), most of its checklist items become moot.
- The surviving items would be: V1/V2 fix in `ecosystem-research.md` line 39, V1/V2 fix in `architectural-decisions.md` line 30, V2 fix in `continue-implementation.md` line 92.
- **Recommendation**: Apply the three surviving fixes, then delete this file too.

### 17. `_agent-work/` (directory)
**Purpose**: Detailed research outputs from background agents during initial analysis phase.
**Contents** (9 files):
- `bun-monorepo-research.md` — Bun workspace setup research
- `domcorder-analysis.md` — Bincode patterns from domcorder reference project
- `expo-rn-constraints.md` — Expo/React Native platform constraints
- `node-context-gap-analysis.md` — Node/Context implementation gap analysis
- `reactor-main-spec.md` — Reactor subsystem detailed spec
- `remaining-questions.md` — Open questions from early analysis
- `rust-architecture-findings.md` — Rust codebase architecture analysis
- `spec-cross-check.md` — Cross-check of specs against Rust source (basis for ankurah-rs-spec-cleanup.md)
- `yrs-yjs-interop-findings.md` — Yrs/Yjs interop research
**Verdict: KEEP (archive)**
- These are read-only reference material. None are actively maintained.
- `reactor-main-spec.md` may still be useful for the remaining peer_subscription work.
- Low maintenance burden (zero — nobody updates these).
- Could be deleted in bulk once Layer 7 is complete, but no urgency.

---

## `continue-implementation.md` — Stale Sections

The memory model sections need updating now that `specs/memory-model/` is finalized and simplified.

### Section: "Architectural decisions" item 16 (line 96)
**Stale content**: References "memory-model.md" (singular file), describes "three patterns: scope-guaranteed (RefCell for correctness-critical), user-managed (Disposable for resource hygiene), decision-required (Transaction)" and "Two FinalizationRegistry severities: hard-fail for correctness-critical, warn for resource hygiene."
**What changed**: The memory model spec was finalized into `memory-model/` (a directory with 4 files). The final spec uses a simpler framing: Disposable base class with severity classification (correctness-critical vs resource-hygiene), not "three patterns." The terminology "scope-guaranteed" and "decision-required" are not in the final spec.
**Fix**: Update to reference `specs/memory-model/` and use the final spec's framing.

### Section: "Memory model integration (pending)" (lines 324-328)
**Stale content**: Lists four pending tasks:
1. `disposable.ts severity parameter` — Add `severity: 'fatal' | 'warning'`
2. `ResultSetWrite -> RefCell` — Refactor to use `RefCell.withMut()`
3. `Existing types -> Disposable` — Wire ReactorSubscription, EntityLiveQuery, etc.
4. `Category naming` — Finalize naming for severity levels

**What changed**: The memory model spec is finalized. The naming is settled (correctness-critical / resource-hygiene). The provided-types spec in `memory-model/provided-types.md` defines the API. Whether these integration tasks are done or still pending needs checking against the actual source code, but the spec uncertainty is resolved.
**Fix**: Remove "Category naming" item (resolved). Update remaining items to reference the finalized spec. Check source code to see if any have been completed.

### Section: "Rust Ownership -> JS GC Translation (CRITICAL)" (lines 404-458)
**Stale content**: This entire section is a pre-finalization summary that says "See specs/memory-model.md for the complete design. The patterns below are summarized here for quick reference." It then provides its own mapping table, layered defense strategy, known gotchas, and rules for new code.
**What changed**: The finalized `memory-model/` spec is now the authoritative source. This section duplicates and may contradict it in terminology.
**Fix**: Replace the mapping table and strategy sections with a brief pointer to `specs/memory-model/overview.md`. Keep the "Known Gotchas" subsection (lines 436-449) — those are implementation war stories that are genuinely useful and don't belong in the spec. Keep the "Rules for New Code" (lines 451-458) or move them to `memory-model/overview.md` if they aren't already there.

### Section: Spec inventory table (line 181)
**Stale content**: Lists `memory-model.md` as a single file.
**Fix**: Update to `memory-model/` (directory) with updated description.

---

## Summary

| File | Verdict | Reason |
|------|---------|--------|
| `architecture.md` | DELETE | Superseded by continue-impl + port-rules + arch-decisions; has known errors |
| `ecosystem-research.md` | KEEP | Useful onboarding reference; fix V1->V2 on line 39 |
| `fetch-gap-port-spec.md` | DELETE | Work completed |
| `livequery-port-spec.md` | DELETE | Work completed |
| `initial-porting-workflow.md` | DELETE | Entirely historical, has known errors |
| `ongoing-maintenance-workflow.md` | KEEP | Aspirational but still valid design |
| `port-maintainability-analysis.md` | DELETE | Stale point-in-time assessment |
| `progress-and-parallelism-review.md` | DELETE | Stale snapshot, continue-impl is the living doc |
| `schema-registry-and-codegen.md` | KEEP | Future work, tracks external dependency |
| `storage-memory-impl-spec.md` | DELETE | Work completed |
| `structural-mapping-analysis.md` | DELETE | Superseded by port-rules + audit script + MIRRORS annotations |
| `system-port-spec.md` | DELETE | Work completed |
| `watcher-set.md` | DELETE | Work completed |
| `wire-format-interop.md` | DELETE | Has known errors; proto fixtures are the living validation |
| `yrs-yjs-interop-validation.md` | DELETE | Work completed; has known V1/V2 errors |
| `ankurah-rs-spec-cleanup.md` | DELETE after fixes | Apply 3 surviving fixes, then delete |
| `_agent-work/` | KEEP | Read-only archive, zero maintenance cost |

**Score**: 11 files to delete, 3 to keep, 1 to delete after applying fixes, 1 directory to keep as archive. Plus 4 stale sections in `continue-implementation.md` to update.
