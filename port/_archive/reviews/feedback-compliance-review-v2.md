# User Feedback Compliance Review v2: Restructured Memory Model Spec

**Reviewer**: Feedback Compliance Agent
**Date**: 2026-03-13
**Spec under review**: Three-file restructure:
- `specs/memory-model/overview.md` — Quick-reference rulebook
- `specs/memory-model/decisions.md` — Overarching architectural decisions
- `specs/memory-model/provided-types.md` — API docs for utility types

**Prior review**: `specs/reviews/feedback-compliance-review.md` (v1, single-file spec)

---

## Verdict: PASS

All 23 user decisions remain reflected in the restructured spec. No decisions were lost, softened, or contradicted by the restructure. The three-file split is clean: rules in overview.md, ankurah-specific classifications in decisions.md, and API surface in provided-types.md. The restructure also correctly follows the user's meta-directive that "decisions should be overarching and narrow cases should move into code comments over time" — decisions.md explicitly states this in its header.

---

## Decision-by-Decision Compliance Table

| # | User Decision | Reflected? | Where (v1) | Where (v2) | Discrepancy |
|---|--------------|-----------|------------|------------|-------------|
| 1 | **Correctness vs ergonomics** is the key axis for classifying cleanup patterns | YES | Section 3, Axis 1 | overview.md: "Two-Axis Classification", Axis 1 | None. "what happens if cleanup doesn't run?" framing preserved. |
| 2 | **Transaction** = ergonomic/waste only. Nobody hurt if no rollback. FR rollback fine. `using` optional convenience. | YES | Section 3a | decisions.md: "Resource Hygiene (Decision-Required)" — Transactions | None. "nobody is hurt — just wasted memory for forked entities. FinalizationRegistry rolling back an abandoned transaction is acceptable. `using` provides auto-rollback as a convenience." All three user points preserved verbatim. |
| 3 | **ResultSetWrite** = correctness-critical. MUST complete at specific time. FR should **crash the app hard** if not completed. | YES | Section 3b, Section 8 | decisions.md: "Correctness-Critical (Scope-Guaranteed)" + overview.md: FR Policy "Crash the app hard" | None. decisions.md: "the broadcast MUST fire at a specific time. If it doesn't, observers silently see stale data." overview.md FR Policy: "Crash the app hard via `queueMicrotask(() => { throw ... })`". Both halves preserved. |
| 4 | **Subscriptions** (ReactorSubscription, LiveQuery) = waste/resource hygiene, NOT correctness | YES | Section 3c | decisions.md: "Resource Hygiene (User-Managed)" — Reactor subscriptions and live queries | None. "no observer sees wrong data — the problem is waste, not wrongness." Verbatim preservation of "waste, not wrongness" framing. |
| 5 | **Weak->WeakRef** should be the default safe approach, like-for-like mapping | YES | Section 7 | overview.md: "WeakRef Rules" | None. "Rust `Weak<T>` maps to TS `WeakRef<T>` 1:1. This is the default safe approach — use them like-for-like." Verbatim. |
| 6 | **Mutex** was dismissed too hastily -- still needed for **application invariant enforcement** | YES | Sections 6, 13 | overview.md: RefCell Rules ("application invariant enforcement"), Async Serialization Rules ("When `std::sync::Mutex` CANNOT be eliminated") + provided-types.md: RefCell and PromiseMutex APIs | None. RefCell Rules explicitly list "application invariant enforcement" as a use case. The "CANNOT be eliminated" subsection directly addresses the "dismissed too hastily" concern. Both RefCell and PromiseMutex are first-class sections, not footnotes. |
| 7 | Need a **unified design pattern** for do-work-on-drop | YES | Sections 3-5 | overview.md: Two-Axis Classification + Disposal Rules + RefCell Rules + provided-types.md: Disposable/DisposeGuard/RefCell | None. The unified pattern flows: classify (overview Axis 1+2) → pick mechanism (Disposable/DisposeGuard/RefCell) → implement (provided-types API). |
| 8 | **Fail loud + fail early** is a net quality improver | YES | Sections 3b, 8 | overview.md: Axis 1 ("Fail loud, fail early"), FR Policy, RefCell Rules ("Fail loud, fail early") | None. The phrase appears twice in overview.md — in the Axis 1 correctness-critical definition and in the RefCell async detection constraint. |
| 9 | `using` is fine for both Transaction and ResultSetWrite (with Mutex/RefCell enforcement for the latter) | YES | Sections 3a, 3b, 9 | overview.md: Core Mapping Table ("using declaration or explicit dispose()"), Disposal Rules + decisions.md: Transactions "`using` provides auto-rollback" + provided-types.md: Symbol.dispose Polyfill, Babel plugin | None. Transaction uses `using` for convenience (decisions.md). ResultSetWrite uses RefCell/withMut which is scope-guaranteed (overview.md RefCell Rules). The `using` platform support is in provided-types.md. |
| 10 | The real concern with `using` is the **escape hatch** -- `let bar; {using foo = ..; bar = foo;}` leaks a disposed object | YES | Section 12 | decisions.md: "The `using` escape hatch" under Known Architectural Gotchas | None. Exact scenario: "`let bar; { using foo = ..; bar = foo; }` leaks a disposed reference." Mitigation: "`assertNotDisposed()` guards on public methods convert this from a silent failure into a loud error." |
| 11 | Forgetting to call done/commit is solvable by FinalizationRegistry | YES | Sections 3a, 8 | overview.md: FR Policy + decisions.md: Transactions ("FinalizationRegistry rolling back an abandoned transaction is acceptable") | None. FR as safety net is clearly stated in both files. |
| 12 | **using/dispose + guards + refcell** preferred over `withWrite(fn)` if platform support exists | PARTIALLY | Section 9 | provided-types.md: Symbol.dispose Polyfill section covers platform support (Babel plugin, Hermes fallback) | Minor change. The v1 spec had a sentence explicitly stating "using/dispose + guards + RefCell is the preferred approach provided this transform works across all target platforms." The v2 spec covers the platform support mechanics (polyfill, Babel plugin, Hermes) but does not restate the explicit preference sentence. The preference is implicit from the overall structure (Disposable/DisposeGuard/RefCell are the primary patterns), but the explicit "preferred over withWrite(fn)" comparison is absent. See Notes below. |
| 13 | **Metro/Hermes platform support** is the key delineator for using/dispose | PARTIALLY | Section 9 | provided-types.md: Symbol.dispose Polyfill section | Minor change. The polyfill section covers Metro, Hermes, and the Babel plugin, but the v1 phrasing "This is the key platform support delineator" is absent. The information is all present — the framing of its importance is softer. See Notes below. |
| 14 | Open to **inheritance exception** (base class) for Disposable if the pattern is common enough | YES | Section 4 | overview.md: Disposal Rules (when to use Disposable vs DisposeGuard) + provided-types.md: Disposable API | None. Inheritance (`extends Disposable`) is the default; composition (DisposeGuard) is the fallback for types that already extend another class. Same status as v1 — the inheritance-is-default design choice is implicit in the structure. |
| 15 | Mandatory RAII applies to types with **vicarious drop** (owning Drop-implementing fields), not just direct `impl Drop` | YES | Section 10 | overview.md: Core Mapping Table ("Vicarious RAII" row) + Disposal Rules ("Type owns fields whose Rust types have `impl Drop`") + decisions.md: "Vicarious RAII" section with ownership chains | None. Vicarious RAII appears in three places: the mapping table, the disposal rules, and a dedicated section in decisions.md with concrete ownership chains (reactive subscription, signal subscription, listener guard). Better coverage than v1. |
| 16 | Mandatory vicarious RAII is **distinct from** resource-preserving RAII | YES | Section 10 | overview.md: Core Mapping Table has separate rows for direct Drop and vicarious RAII, both mapping to the same TS treatment ("Same as direct Drop") | Subtle. The v1 spec had an explicit statement: "This is distinct from resource-preserving RAII." The v2 spec communicates the same idea structurally (separate table row, same treatment) but does not include the explicit distinction statement. The mapping table's "Same as direct Drop" comment makes the equivalence clear, which is arguably the more important point — vicarious RAII gets the same treatment. The distinction is preserved in practice. |
| 17 | Names must convey **WHY** (must-complete vs nice-to-clean-up), not just what | YES | Section 3, Axis 1 | overview.md: Axis 1 names are "Correctness-critical" and "Resource hygiene" | None. These names convey severity (why cleanup matters), not mechanism (what cleanup does). |
| 18 | FinalizationRegistry errors should include **file and line number** | YES | Section 8 | overview.md: FR Policy — "creation stack trace (file+line)" appears in both resource hygiene and correctness-critical bullets | None. Mentioned in both severity tiers. |
| 19 | The goal is a **consistent and correct mapping** from Rust's memory model | YES | Section 1 (Purpose) | overview.md: title "Memory Model: Rust Ownership to TypeScript GC" + Core Mapping Table | None. The entire overview.md is structured as a mapping rulebook. The core mapping table is the first content section. |
| 20 | Mutex/RefCell is "close enough" to memory model concerns to include in the spec | YES | Sections 6, 13 | overview.md: RefCell Rules + Async Serialization Rules + Core Mapping Table (rows for Mutex, tokio::sync::Mutex, RefCell) + provided-types.md: RefCell and PromiseMutex APIs | None. Both are first-class sections in both overview.md and provided-types.md. Three rows in the core mapping table. |
| 21 | Test files should be tracked similarly to source files in the port manifest | N/A (process) | -- | -- | Not a spec concern. Same as v1. |
| 22 | Supporting code should be written directly, not as a TODO list | N/A (process) | -- | -- | Not a spec concern. Same as v1. |
| 23 | Always delegate to background agents; supervisor should coordinate, not implement | N/A (process) | -- | -- | Not a spec concern. Same as v1. |

---

## Contradiction / Softening Analysis

No contradictions found. Two minor softenings noted:

1. **Decision 12 (preference statement)**: The explicit sentence "using/dispose + guards + RefCell is the preferred approach provided this transform works across all target platforms" from v1 Section 9 does not appear in the v2 restructure. The preference is implicit from the overall architecture (these are the only patterns documented), but the explicit comparison against `withWrite(fn)` alternatives is gone. **Severity: Low.** The preference is structurally obvious — no alternative pattern is documented, so there is nothing to prefer it over.

2. **Decision 13 ("key platform support delineator" framing)**: The v1 phrasing "This is the key platform support delineator" is absent from provided-types.md's polyfill section. The technical content (Metro, Hermes, Babel plugin) is all present. **Severity: Low.** The editorial emphasis is gone but the substance is intact.

Neither of these rises to the level of a decision being lost or contradicted. They are framing/emphasis changes, not content changes.

---

## Structural Assessment

The three-file split is well-executed:

| File | Purpose | Content Quality |
|------|---------|----------------|
| overview.md | Rulebook for implementers | Clean decision tree structure. Core mapping table is excellent as a quick reference. Two-axis classification is preserved faithfully. |
| decisions.md | Ankurah-specific classifications | Correctly scoped to overarching decisions. The header explicitly states "Narrow, type-specific adjudications should be annotated in the source code itself, not here" — directly reflecting the user's meta-directive. |
| provided-types.md | API docs | Complete API surfaces for all four utility types (Disposable, DisposeGuard, RefCell, PromiseMutex) plus polyfill details. Code examples are clear. |

### What improved in the restructure

- **Core Mapping Table** (overview.md lines 9-24): New addition. Provides a single-glance reference for the most common Rust-to-TS translations. Excellent for day-to-day implementer use.
- **Separation of rules from classifications**: overview.md contains portable rules (could apply to any Rust-to-TS translation). decisions.md contains ankurah-specific classifications. This makes the rulebook easier to maintain as the project evolves.
- **Vicarious RAII ownership chains** (decisions.md lines 27-34): The concrete chains (reactive subscription, signal subscription, listener guard) are new and useful. They make the abstract concept actionable.
- **"Narrow cases in code comments" directive** (decisions.md header): Explicitly codifies the user's meta-instruction about where narrow decisions belong.

### What was lost or reduced

- **v1 Section 10 exhaustive vicarious RAII tables**: The v1 spec had detailed per-type tables showing which types have direct Drop vs vicarious RAII. The v2 decisions.md replaces these with higher-level ownership chain descriptions. This is arguably better (overarching vs exhaustive), but implementers may need to reconstruct the per-type details from source code.
- **v1 Section 11 "Rules for New Code" decision tree**: The v1 spec had a step-by-step flowchart for classifying new types. The v2 overview.md has a simpler two-question classification under "How to classify a new type" (lines 44-46). The v2 version is more concise but slightly less prescriptive.
- **Explicit preference/delineator language** (decisions 12, 13): As noted above, two editorial emphasis phrases are absent.

---

## Tone / Framing Check

Key user phrases preserved in v2:

| User Phrase | Present in v2? | Location |
|------------|----------------|----------|
| "waste, not wrongness" | YES | decisions.md line 15 |
| "nobody is hurt" | YES | decisions.md line 21 |
| "crash the app hard" | YES | overview.md line 34, line 119 |
| "fail loud, fail early" | YES | overview.md line 34, line 85 |
| "like-for-like" | YES | overview.md line 101 |
| "default safe approach" | YES | overview.md line 101 |
| "application invariant enforcement" | YES | overview.md line 81 |

Two phrases absent from v2:
| User Phrase | Present in v2? | Location in v1 |
|------------|----------------|----------------|
| "key platform support delineator" | NO | v1 Section 9 |
| "preferred approach provided this transform works" | NO | v1 Section 9 |

---

## Recommendations

1. **Consider restoring the preference statement** (Decision 12): Add a one-line note to provided-types.md's Symbol.dispose Polyfill section: "This platform support is the key delineator — `using`/`dispose` + guards + RefCell is the preferred approach provided the Babel transform works across all target platforms." This restores both decisions 12 and 13 in a single sentence.

2. **No other action needed.** All 23 decisions are reflected. The restructure is faithful to the user's intent and improves the spec's usability as a working reference.
