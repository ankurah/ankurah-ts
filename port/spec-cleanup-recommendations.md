# Spec Cleanup Recommendations

> **HISTORICAL — recommendations made on 2026-03-14 about the archive, kept as a
> record of what was decided about each archived file.** Its recommendations for
> the live docs were written against the ownership spec of the day and are
> superseded: it proposes documenting what `onDispose()` should do when it
> throws, and lists nine lint rules under names (`require-using`,
> `require-disposable`) that no longer exist. `AkObject.drop()` now runs
> `onDrop()` inside a `try` and cascades in the `finally` regardless, and the
> lint plugin is down to six rules after the retractions of 2026-09-02. Read the
> archive assessments; do not act on the live-doc recommendations without
> re-deriving them. Historical text follows unchanged.

**Date**: 2026-03-14
**Reviewer**: Spec Reviewer agent
**Scope**: All files in `port/_archive/` (top-level, `memory-model/`, `_agent-work/`, `reviews/`) assessed against current docs in `port/`.

**Guiding principle**: This is an ongoing, iterative port. The Rust source continues to evolve. "Done-ness" of a subsystem is NOT a reason to delete its spec -- translation rationale retains value when re-porting after Rust changes. However, *exact duplicates*, *actively misleading content*, and *superseded-with-no-unique-content* files are candidates for deletion.

---

## Summary

| Category | Count |
|----------|-------|
| DELETE | 12 files |
| DELETE after applying fixes | 1 file |
| MERGE specific sections into current docs | 3 files |
| KEEP in archive as-is | 7 files + 2 directories |

---

## Part 1: Archive File Assessments

### 1. `_archive/architectural-decisions.md` -- DELETE

**Reason**: Exact duplicate of current `port/decisions.md`. Same content, same date (2026-02-10). Zero unique material.

---

### 2. `_archive/port-rules.md` -- DELETE

**Reason**: Exact duplicate of current `port/translation-rules.md`. Same content, same date.

---

### 3. `_archive/architecture.md` -- DELETE

**Content not covered by current docs**: "Why a TS Port" rationale section, React Native hook API sketches, CLI codegen pipeline sketch.

**Translation rationale for re-porting**: None. This is a high-level overview, not translation guidance.

**Actively misleading**: Yes. `NodeMessageBody` does not exist (it is `NodeMessage` which is an enum, not a struct). Lineage de-scope claim is too broad (lineage types are in-scope per `decisions.md`). Contradicts codegen decision.

**Recommendation**: DELETE. The "Why a TS Port" rationale is historical context with no ongoing value. The module mapping table duplicates `port-runbook.md`. The type correspondences are a less precise version of `translation-rules.md`. The errors make it actively dangerous to consult.

---

### 4. `_archive/ecosystem-research.md` -- KEEP

**Content not covered by current docs**: expo-sqlite API details, CRDT library comparison (Yjs vs Automerge vs Loro viability assessment), Expo Go runtime constraints (no WASM, no native modules), bincode encoding reference (integer widths, string encoding, enum variant encoding, EventId computation).

**Translation rationale for re-porting**: The bincode encoding details serve as a reference when extending the codec for new proto types. The CRDT comparison documents *why* Yjs was chosen -- useful if someone asks "why not Automerge/Loro?"

**Actively misleading**: One error: line 39 says V1 encoding when Rust uses V2 exclusively. Fix this line.

**Recommendation**: KEEP. Fix the V1->V2 error on line 39. Low maintenance burden since it describes external constraints that rarely change. Useful for onboarding and decision rationale.

---

### 5. `_archive/schema-registry-and-codegen.md` -- KEEP

**Content not covered by current docs**: Schema registry design based on Rust PR #236, codegen workflow (`.ankurah` schema files -> `defineModel()` calls), generated TypeScript code templates.

**Translation rationale for re-porting**: This documents a forward-looking pipeline. PR #236 is still in progress. When this work resumes, the spec provides the design intent.

**Actively misleading**: No.

**Recommendation**: KEEP. Forward-looking reference for future Layer 9+ work. Low maintenance burden.

---

### 6. `_archive/structural-mapping-analysis.md` -- MERGE then DELETE

**Content not covered by current docs**: Illustrative code examples showing Rust-to-TS struct/trait/enum/impl mapping side-by-side. The "88% directly mappable" claim is stale, but the code examples are genuinely useful and are NOT in `translation-rules.md`.

**Translation rationale for re-porting**: Yes -- the side-by-side code examples demonstrate how to apply the translation rules concretely. A re-porter encountering a new Rust struct/enum/impl pattern would benefit from seeing these examples.

**Actively misleading**: The 88% claim is based on stale file counts. The file-level mapping tables are superseded by audit-port.ts.

**Recommendation**: MERGE the Rust-to-TS code examples (struct mapping, trait-to-interface, enum translation, impl-to-class) into `translation-rules.md` as a new "Examples" section. Then DELETE this file. The examples add concrete illustration to the abstract rules.

---

### 7. `_archive/wire-format-interop.md` -- DELETE

**Content not covered by current docs**: Bincode encoding details (integer widths, string length encoding, enum variant encoding), BincodeReader/BincodeWriter skeleton code.

**Translation rationale for re-porting**: The bincode encoding details overlap with `ecosystem-research.md` (which is being kept). The skeleton code is superseded by the actual implementation.

**Actively misleading**: Yes, multiple errors. `NodeMessage` described as a struct (it is an enum). `Operation` struct shown with wrong shape. Recommends a JSON alternative (directly contradicts `decisions.md`).

**Recommendation**: DELETE. The one piece of unique value (bincode encoding rules) is covered more accurately in `ecosystem-research.md`. The errors make this file dangerous.

---

### 8. `_archive/initial-porting-workflow.md` -- DELETE

**Content not covered by current docs**: None. The phase-by-phase guide is entirely superseded by `port-runbook.md`.

**Translation rationale for re-porting**: None. This describes one-time bootstrap phases, not re-porting.

**Actively misleading**: Yes. File inventories undercount actual Rust files. Reactor described as single file (it is a directory). `update.rs` vs `message.rs` confusion.

**Recommendation**: DELETE. Historical, stale, and error-prone.

---

### 9. `_archive/ongoing-maintenance-workflow.md` -- KEEP

**Content not covered by current docs**: CI-based drift detection workflow design (daily diff jobs, automated patch application, sync state tracking). The concepts are partially implemented by `audit-port.ts`, but the CI pipeline design (GitHub Actions workflow, scheduled runs, PR auto-creation) is not documented elsewhere.

**Translation rationale for re-porting**: Directly relevant -- the whole purpose is to detect when Rust changes require re-porting.

**Actively misleading**: No.

**Recommendation**: KEEP. The CI pipeline design is aspirational but still the right approach. Revisit when Layer 9 (integration/CI) is reached. Zero maintenance cost.

---

### 10. `_archive/port-maintainability-analysis.md` -- DELETE

**Content not covered by current docs**: Point-in-time assessment from 2026-02-11 scoring port at 7.5/10 with specific gaps.

**Translation rationale for re-porting**: None. Findings have been acted on or absorbed elsewhere.

**Actively misleading**: Stale scores and percentages.

**Recommendation**: DELETE. The actionable items are captured in `ankurah-rs-spec-cleanup.md` and the audit script.

---

### 11. `_archive/progress-and-parallelism-review.md` -- DELETE

**Content not covered by current docs**: Historical snapshot from 2026-02-11. Reports 309 tests (now 376), Layer 5b as "in progress" (now Layer 7).

**Translation rationale for re-porting**: None.

**Actively misleading**: All numbers are stale.

**Recommendation**: DELETE. `port-runbook.md` is the living status document.

---

### 12. `_archive/yrs-yjs-interop-validation.md` -- DELETE

**Content not covered by current docs**: Yrs/Yjs interop validation test plan.

**Translation rationale for re-porting**: Marginal. The test patterns could inform new Yrs/Yjs interop tests, but the code examples all show V1 API calls when V2 is used exclusively.

**Actively misleading**: Yes. V1 encoding referenced throughout. Would lead a re-porter to use the wrong Yjs API.

**Recommendation**: DELETE. The interop is validated by 10 passing tests and 6 fixture files. The errors make this file actively harmful.

---

### 13. `_archive/watcher-set.md` -- KEEP

**Content not covered by current docs**: Detailed translation rationale for WatcherSet: why JS `Map` key patterns replace Rust `HashMap<(CollectionId, EntityId), ...>` composite keys. Pseudocode with divergence annotations. Concurrency simplification notes.

**Translation rationale for re-porting**: Yes. If Rust changes `WatcherSet` (adds fields, changes the state machine, modifies key patterns), a re-porter would need to understand *why* the TS implementation uses different key strategies. The divergence annotations document decisions that are not obvious from the code alone.

**Actively misleading**: No.

**Recommendation**: KEEP. Translation rationale is the primary value. The code is implemented and tested, but the "why we diverged" rationale retains value for maintenance.

---

### 14. `_archive/fetch-gap-port-spec.md` -- KEEP

**Content not covered by current docs**: Detailed translation decisions for GapFetcher: continuation predicate construction, interface mapping, algorithm pseudocode with divergence annotations.

**Translation rationale for re-porting**: Yes. The divergence annotations explain why TS uses different patterns for async iteration and predicate construction. When Rust changes the gap-fetch algorithm, a re-porter needs this context.

**Actively misleading**: No.

**Recommendation**: KEEP. Same rationale as watcher-set.md -- translation rationale for an implemented subsystem retains value for ongoing maintenance.

---

### 15. `_archive/livequery-port-spec.md` -- KEEP

**Content not covered by current docs**: Detailed translation decisions for LiveQuery: EntityLiveQuery inner state management, activation pattern, Signal/Get/Peek/Subscribe implementations, async pattern decisions.

**Translation rationale for re-porting**: Yes. LiveQuery is one of the most complex subsystems (726 lines in TS). The spec documents multiple divergence decisions around async activation, signal wiring, and state machine transitions.

**Actively misleading**: No.

**Recommendation**: KEEP. The complexity of LiveQuery makes the translation rationale especially valuable for re-porting.

---

### 16. `_archive/storage-memory-impl-spec.md` -- DELETE

**Content not covered by current docs**: Implementation spec for `@ankurah/storage-memory`. Contains entityStateAsFilterable adapter pattern, fetchStates algorithm.

**Translation rationale for re-porting**: Minimal. `storage-memory` is a TS-only package with no Rust counterpart. Changes to Rust storage traits would be captured by `storage-common`, not this.

**Actively misleading**: No.

**Recommendation**: DELETE. TS-only implementations don't need re-porting rationale since there is no Rust source to track.

---

### 17. `_archive/system-port-spec.md` -- MERGE then DELETE

**Content not covered by current docs**: Detailed translation rationale for SystemManager: `sys::Item` JSON round-trip decisions, Node integration patterns, field mapping justifications.

**Translation rationale for re-porting**: Yes, but the spec is 30KB -- most of it is implementation detail that is now in the code. The unique value is the `sys::Item` variant mapping decisions and the `#[serde(other)]` handling rationale.

**Actively misleading**: No.

**Recommendation**: MERGE the `sys::Item` variant mapping rationale and `#[serde(other)]` handling into a comment block at the top of the relevant TS source file (`system.ts`). Then DELETE this file. The rationale belongs close to the code it explains, not in a separate spec.

---

### 18. `_archive/continue-implementation.md` -- DELETE

**Content not covered by current docs**: Large (~51KB) continuation document with supervision model, layer-by-layer status, implementation priorities.

**Translation rationale for re-porting**: None. This is a project management artifact, not translation rationale.

**Actively misleading**: Partially stale. References `specs/memory-model.md` (singular file, now restructured). Contains stale status percentages and outdated layer completion markers.

**Recommendation**: DELETE. `port-runbook.md` is the living status document. The memory model references point to files that have been reorganized.

---

### 19. `_archive/ankurah-rs-spec-cleanup.md` -- DELETE after applying fixes

**Content not covered by current docs**: Meta-tracker of known errors across archived specs. Organized by category: V1/V2 errors, duplicate proto definitions, incorrect type descriptions, stale file inventories.

**Translation rationale for re-porting**: None. This tracks errors in other files.

**Actively misleading**: No -- it *identifies* misleading content in other files.

**Recommendation**: Apply the three fixes that survive the deletions above:
1. Fix V1->V2 in `ecosystem-research.md` line 39 (if keeping that file)
2. Any surviving V2 references in current docs

Then DELETE this file. Once the referenced files are deleted or fixed, this meta-tracker has no purpose.

---

### 20. `_archive/memory-model.md` (index) -- DELETE

**Reason**: Index file pointing to `memory-model/` subdirectory. The subdirectory contents are assessed below.

---

### 21. `_archive/memory-model/decisions.md` -- DELETE

**Reason**: Content has been merged into current `port/decisions.md` (Async Serialization and Known Gotchas sections).

---

### 22. `_archive/memory-model/provided-types.md` -- DELETE

**Reason**: Exact duplicate of current `port/ownership/provided-types.md`.

---

### 23. `_archive/memory-model/lint-rules.md` -- MERGE then DELETE

**Content not covered by current docs**: 9 ESLint rules across 3 tiers for ownership enforcement. Tier 1 (must-have): `require-disposable`, `require-using`, `require-alive-check`, `no-dispose-in-loop`. Tier 2 (important): `require-assert-not-disposed`, `require-dispose-cascade`. Tier 3 (nice-to-have): `no-async-in-with-mut`, `require-reverse-dispose-order`, `require-weak-deref-check`.

**Translation rationale for re-porting**: Not directly, but this documents the enforcement strategy for the ownership model, which IS relevant to ongoing maintenance. The ESLint plugin (`@ankurah/eslint-plugin`) implements 8 rules, and this spec documents the design rationale for rule selection and tiering.

**Actively misleading**: No.

**Recommendation**: MERGE the rule inventory and tier rationale into the ESLint plugin's own documentation (e.g., `packages/eslint-plugin/README.md` or a doc within the plugin package). Then DELETE from archive. The lint rules belong with the linter, not in the ownership spec archive.

---

### 24. `_archive/_agent-work/` (directory, 9 files) -- KEEP

Files: `bun-monorepo-research.md`, `domcorder-analysis.md`, `expo-rn-constraints.md`, `node-context-gap-analysis.md`, `reactor-main-spec.md`, `remaining-questions.md`, `rust-architecture-findings.md`, `spec-cross-check.md`, `yrs-yjs-interop-findings.md`.

**Translation rationale for re-porting**: `reactor-main-spec.md` and `node-context-gap-analysis.md` may be useful for the remaining Layer 7 work (peer_subscription, node networking). `domcorder-analysis.md` documents bincode patterns from the reference implementation. `rust-architecture-findings.md` captures Entity/Model/View/Mutable lifecycle analysis.

**Actively misleading**: `remaining-questions.md` contains questions that are all resolved in `decisions.md` -- it could confuse someone who doesn't realize the answers exist. But since these are clearly labeled as agent work products, the risk is low.

**Recommendation**: KEEP as read-only archive. Zero maintenance cost. `reactor-main-spec.md` specifically retains value for Layer 7 completion. Consider deleting in bulk once Layer 7 is fully complete.

---

### 25. `_archive/reviews/` (directory, 11 files) -- KEEP

Files: 5 v1 reviews (feedback-compliance, completeness, semantic-soundness, async-safety, adversarial), 5 v2 reviews (same reviewers), 1 spec-survey.

**Assessment**: These are comprehensive review artifacts that document the quality assurance process for the ownership/memory-model spec. The v2 reviews confirm the restructured spec passes all checks. The adversarial review identifies 12 attack scenarios with severity ratings. The async-safety review provides detailed interleaving analysis.

**Translation rationale for re-porting**: The adversarial review's attack scenarios serve as a regression test checklist. The async-safety review's PromiseMutex coverage table identifies known race conditions. These are reference material for anyone modifying the async or ownership infrastructure.

**Content not covered by current docs**: The spec-survey.md contains per-file verdicts that overlap with this recommendations file but from a different perspective (it was written before this review and uses the older `specs/` directory layout).

**Actively misleading**: References to `specs/memory-model.md` (now `port/ownership/`) are outdated path references, but the content analysis is sound.

**Recommendation**: KEEP. These reviews document the reasoning behind the current spec structure. The adversarial scenarios and async analysis have ongoing reference value. Zero maintenance cost.

---

## Part 2: Current Doc Improvements

### 1. `port/translation-rules.md` -- Add code examples

**Issue**: The translation rules are comprehensive but abstract. Side-by-side code examples showing Rust-to-TS struct/trait/enum/impl mapping would make the rules concrete and actionable.

**Source**: `_archive/structural-mapping-analysis.md` contains good examples that are not present in the current doc.

**Action**: Add an "Examples" section to `translation-rules.md` with side-by-side Rust/TS code for: struct mapping, trait-to-interface, enum variants, impl-to-class methods.

### 2. `port/decisions.md` -- Add SystemManager lifecycle note

**Issue**: The Async Serialization section lists 3 decisions (reactor notify_lock, WatcherSet gap-fill, LiveQuery activation) but omits SystemManager lifecycle ops. The async-safety reviews (both v1 and v2) identify a real TOCTOU race in `SystemManager.create()`.

**Action**: Add a brief entry noting the race and the low-risk assessment. Whether serialization is added is a code decision, but the spec should document that the race exists and was assessed.

### 3. `port/decisions.md` -- Add Transaction alive check guidance

**Issue**: The Known Architectural Gotchas section mentions "Transaction alive gap" and that `commit()`/`rollback()` set `alive = false` eagerly. But it doesn't state that `Transaction.create()`, `.get()`, `.edit()` MUST check `alive` at entry. The semantic-soundness review flagged this as UNSOUND.

**Action**: Add to the Transaction alive gap section: "Transaction methods (create, get, edit) MUST check alive at entry and throw if the transaction has been committed or rolled back."

### 4. `port/ownership.md` -- Specify onDispose() error behavior

**Issue**: Neither the ownership mapping doc nor provided-types.md specify what happens when `onDispose()` throws. The current implementation sets `#disposed = true` and unregisters from FR *before* calling `onDispose()`, meaning a throwing `onDispose()` permanently wedges the object with no diagnostic. The adversarial review (v1 scenario 12, v2 issue 1) flagged this as HIGH severity.

**Action**: Add an "Error Behavior" subsection to the Disposable section in `port/ownership/provided-types.md` specifying the contract. Document the ordering: call `onDispose()` first, set `#disposed = true` on success, keep FR registered as backstop on failure. Also note that multi-field `onDispose()` should use try/finally to ensure all fields are disposed even if an earlier one throws.

### 5. `port/ownership.md` -- Add broadcast error isolation rule

**Issue**: The adversarial review (v1 scenario 7, v2 issue 3) identified that `Broadcast.send()` has no try/catch around individual listener callbacks. A throwing listener prevents all subsequent listeners from receiving notifications. This undermines the "correctness-critical" guarantee for result set broadcasts.

**Action**: Add a rule to `port/ownership.md`: "Broadcast listeners MUST be called in isolation. A throwing listener MUST NOT prevent other listeners from receiving the notification."

### 6. Cross-reference consistency

**Issue**: The reviews reference paths like `specs/memory-model/overview.md` and `specs/memory-model/decisions.md`. The current structure uses `port/ownership.md`, `port/ownership/provided-types.md`, and `port/decisions.md`. This suggests a reorganization happened between the reviews and the current state.

**Action**: No changes needed to current docs (they have the correct paths). But be aware that archived review files reference the old `specs/` layout. This is expected and does not need fixing -- the reviews are reference artifacts, not living docs.

---

## Part 3: Deletion/Merge Order

Execute in this order to avoid dangling references:

1. Apply surviving fixes from `ankurah-rs-spec-cleanup.md`:
   - Fix V1->V2 in `_archive/ecosystem-research.md` line 39
2. Merge code examples from `_archive/structural-mapping-analysis.md` into `port/translation-rules.md`
3. Merge `sys::Item` mapping rationale from `_archive/system-port-spec.md` into source code comments in `system.ts`
4. Merge lint rule inventory from `_archive/memory-model/lint-rules.md` into `packages/eslint-plugin/` documentation
5. Delete the 12 DELETE files:
   - `_archive/architectural-decisions.md`
   - `_archive/port-rules.md`
   - `_archive/architecture.md`
   - `_archive/wire-format-interop.md`
   - `_archive/initial-porting-workflow.md`
   - `_archive/port-maintainability-analysis.md`
   - `_archive/progress-and-parallelism-review.md`
   - `_archive/yrs-yjs-interop-validation.md`
   - `_archive/storage-memory-impl-spec.md`
   - `_archive/continue-implementation.md`
   - `_archive/memory-model.md` (index)
   - `_archive/memory-model/decisions.md`
6. Delete the 3 MERGE-then-DELETE files (after merges are confirmed):
   - `_archive/structural-mapping-analysis.md`
   - `_archive/system-port-spec.md`
   - `_archive/memory-model/lint-rules.md`
7. Delete `_archive/memory-model/provided-types.md` (exact duplicate)
8. Delete `_archive/ankurah-rs-spec-cleanup.md` (after fixes applied)
9. Apply current-doc improvements (Part 2, items 1-5)

---

## Final State After Cleanup

### Files remaining in `_archive/`:
- `ecosystem-research.md` (with V1->V2 fix applied)
- `schema-registry-and-codegen.md`
- `ongoing-maintenance-workflow.md`
- `watcher-set.md`
- `fetch-gap-port-spec.md`
- `livequery-port-spec.md`
- `_agent-work/` (9 files)
- `reviews/` (11 files)

### Rationale for keeping these:
- **ecosystem-research.md**: Onboarding reference, CRDT decision rationale, bincode encoding details
- **schema-registry-and-codegen.md**: Forward-looking design for future work (PR #236)
- **ongoing-maintenance-workflow.md**: CI pipeline design for drift detection (aspirational but valid)
- **watcher-set.md, fetch-gap-port-spec.md, livequery-port-spec.md**: Translation rationale with divergence annotations -- the primary value for ongoing re-porting
- **_agent-work/**: Read-only research artifacts, zero maintenance cost, some still useful for Layer 7
- **reviews/**: Quality assurance documentation, adversarial scenarios serve as regression checklist
