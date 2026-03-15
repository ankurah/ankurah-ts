# User Feedback Compliance Review: memory-model.md

**Reviewer**: Feedback Compliance Agent
**Date**: 2026-03-12
**Spec under review**: `/Users/daniel/ak/ankurah-ts/specs/memory-model.md`

---

## Verdict: PASS WITH NOTES

The spec faithfully reflects all 23 user decisions. No decisions are missing, and none are contradicted. Two decisions (21 and 22) are organizational/process directives rather than spec content, so they are noted but not penalized. One decision (14, inheritance exception for Disposable) is reflected as implicit capability rather than explicit guidance, which could be made more prominent.

---

## Decision-by-Decision Compliance Table

| # | User Decision | Reflected? | Section | Relevant Spec Text | Discrepancy |
|---|--------------|-----------|---------|-------------------|-------------|
| 1 | **Correctness vs ergonomics** is the key axis for classifying cleanup patterns | YES | Section 3, Axis 1 | "The key question: **what happens if cleanup doesn't run?**" with two categories: "Correctness-critical" vs "Resource hygiene" | None. The spec frames this precisely as the user intended: the classification axis is about severity of missed cleanup (correctness vs waste), not about mechanism. |
| 2 | **Transaction** = ergonomic/waste only. Nobody hurt if no rollback. FR rollback fine. `using` optional convenience. | YES | Section 3a | "**Why resource hygiene, not correctness-critical?** If a transaction is abandoned without `commit()` or `rollback()`, nobody is hurt -- just a bit of wasted memory for the forked entities. FinalizationRegistry rolling back an abandoned transaction is perfectly acceptable. Auto-rollback via `using` is a nice convenience, not a correctness requirement." | None. Exact match to user's framing. |
| 3 | **ResultSetWrite** = correctness-critical. MUST complete at specific time. FR should **crash the app hard** if not completed. | YES | Section 3b, Section 8 | Section 3b: "This is **correctness-critical**: if the broadcast doesn't fire, observers silently see stale data." Section 8: "**Correctness-critical** (ResultSetWrite): Crash the app hard. Throw an unhandled exception via `queueMicrotask`." | None. The spec uses the user's exact language ("crash the app hard") and correctly explains why. |
| 4 | **Subscriptions** (ReactorSubscription, LiveQuery) = waste/resource hygiene, NOT correctness | YES | Section 3c, Classification Table | Section 3c: "**Why resource hygiene?** Subscriptions that are not cleaned up waste resources: CPU evaluating dead predicates, memory for watcher entries, and network bandwidth... However, no observer sees incorrect data -- the problem is waste, not wrongness." | None. The "waste, not wrongness" framing matches the user's intent precisely. |
| 5 | **Weak->WeakRef** should be the default safe approach, like-for-like mapping | YES | Section 7 | "Rust `Weak<T>` maps to TS `WeakRef<T>` 1:1. This is the default safe approach -- use them like-for-like." | None. Verbatim match: "default safe approach" and "like-for-like" both appear. |
| 6 | **Mutex** was dismissed too hastily -- still needed for **application invariant enforcement** | YES | Section 6 (RefCell), Section 13 (PromiseMutex) | Section 6: "You need application invariant enforcement (not memory safety, but ensuring side effects like broadcasts happen correctly)." Section 13 extensively covers PromiseMutex for async serialization and when std::sync::Mutex cannot be eliminated. | None. The spec correctly rehabilitates Mutex/RefCell as needed for invariant enforcement, not just thread safety. The "when std::sync::Mutex CANNOT be eliminated" subsection directly addresses the "dismissed too hastily" concern. |
| 7 | Need a **unified design pattern** for do-work-on-drop | YES | Sections 3-5 | The two-axis classification (Section 3), Disposable base class (Section 4), DisposeGuard (Section 5), and RefCell (Section 6) together form a unified design pattern. Section 11 "Rules for New Code" provides a single decision tree. | None. The spec delivers a unified pattern with clear decision points. |
| 8 | **Fail loud + fail early** is a net quality improver | YES | Sections 3b, 8 | Section 3b: "Fail loud + fail early is a net improver of quality." Section 8: "Crash the app hard" for correctness-critical types. | None. The exact phrase "fail loud + fail early is a net improver of quality" appears verbatim. |
| 9 | `using` is fine for both Transaction and ResultSetWrite (with Mutex/RefCell enforcement for the latter) | YES | Sections 3a, 3b, 9 | Transaction: code example shows `using trx = await node.begin()`. ResultSetWrite: uses RefCell/withMut (scope-guaranteed, no `using` needed because it's not a long-lived object). Section 9 covers `using` platform support. | Minor note: ResultSetWrite doesn't use `using` directly because the RefCell/withMut pattern supersedes it -- the scoped callback pattern is strictly better for this case. The spec correctly explains why (Section 3b: "There is no long-lived object to dispose"). This is consistent with the user's intent since RefCell IS the enforcement mechanism they wanted. |
| 10 | The real concern with `using` is the **escape hatch** -- `let bar; {using foo = ..; bar = foo;}` leaks a disposed object | YES | Section 12 "The `using` Escape Hatch" | "The biggest concern with `using` is the escape-hatch pattern: assigning a `using`-scoped object to an outer-scope variable leaks a reference to a disposed object." Code example: `let leaked: MySubscription; { using sub = ...; leaked = sub; // BAD }` | None. The spec uses the user's exact scenario and calls it "the biggest concern." The mitigation (`assertNotDisposed()`) is also specified. |
| 11 | Forgetting to call done/commit is solvable by FinalizationRegistry | YES | Sections 3a, 8 | Section 3a: "FinalizationRegistry MUST warn with creation stack trace (file+line) if GC'd without commit/rollback." Section 8 defines the two-tier FR behavior. | None. The spec correctly positions FR as the safety net for forgotten cleanup calls. |
| 12 | **using/dispose + guards + refcell** preferred over `withWrite(fn)` if platform support exists | YES | Section 9 | "Metro handles this transform with the above plugin. This is the key platform support delineator -- `using`/`dispose` + guards + RefCell is the preferred approach provided this transform works across all target platforms." | None. Verbatim match of the preference and the platform-support caveat. |
| 13 | **Metro/Hermes platform support** is the key delineator for using/dispose | YES | Section 9 | "The `using` declaration syntax requires transpilation on runtimes that do not support it natively." Hermes fallback: `Symbol.for('Symbol.dispose')`. "This is the key platform support delineator." | None. Section 9 explicitly addresses Metro, Hermes, and the Babel plugin needed. |
| 14 | Open to **inheritance exception** (base class) for Disposable if the pattern is common enough | PARTIALLY | Section 4 | Section 4: "class T extends Disposable" is the primary pattern. Section 5 provides DisposeGuard as the composition alternative for types that already extend another class. | The spec implicitly supports this by making Disposable a base class (inheritance IS the default), and DisposeGuard exists for the composition case. However, the spec does not explicitly discuss the inheritance-vs-composition trade-off or state that inheritance is acceptable/preferred when the pattern is common. This is a minor gap -- the user's decision is reflected in practice but not stated as an explicit design rationale. |
| 15 | Mandatory RAII applies to types with **vicarious drop** (owning Drop-implementing fields), not just direct `impl Drop` | YES | Section 10 | "A struct that does not have its own `impl Drop` but owns a field whose type does will still see that field's `drop()` called. We call this **vicarious RAII**. This is distinct from resource-preserving RAII -- vicarious RAII applies to types that own Drop-implementing fields even if they have no Drop impl themselves." Section 11: "No `impl Drop`, but owns fields whose types have `impl Drop` (vicarious RAII): The TS type MUST extend `Disposable`" | None. The spec dedicates an entire section (10) to vicarious RAII with exhaustive tables. |
| 16 | Mandatory vicarious RAII is **distinct from** resource-preserving RAII | YES | Section 10 | "This is distinct from resource-preserving RAII -- vicarious RAII applies to types that own Drop-implementing fields even if they have no Drop impl themselves." | None. The distinction is explicitly stated. |
| 17 | Names must convey **WHY** (must-complete vs nice-to-clean-up), not just what | YES | Section 3, Axis 1 | The two severity levels are named "Correctness-critical" and "Resource hygiene" -- these names convey WHY cleanup matters, not what cleanup does. The classification table maps each type to a severity level with an explanation of what goes wrong. | None. The naming scheme reflects the user's intent: "correctness-critical" = must-complete, "resource hygiene" = nice-to-clean-up. |
| 18 | FinalizationRegistry errors should include **file and line number** | YES | Section 8 | "Log a `console.error` with the creation stack trace, including file and line number." For correctness-critical: "The creation stack trace with file and line number helps developers find the bug quickly." Code example includes `${creationStackTrace}`. Section 3a: "FinalizationRegistry MUST warn with creation stack trace (file+line)." | None. Mentioned three times across sections 3 and 8. |
| 19 | The goal is a **consistent and correct mapping** from Rust's memory model | YES | Section 1 (Purpose) | "This document specifies how Rust's automatic memory management (RAII via the `Drop` trait, borrow checker, `Arc`/`Weak`, `Mutex`/`RwLock`, `RefCell`, lifetimes) maps to TypeScript's explicit disposal model." The entire spec is structured as a mapping from Rust concepts to TS equivalents. | None. The spec is fundamentally organized as a Rust->TS mapping document. |
| 20 | Mutex/RefCell is "close enough" to memory model concerns to include in the spec | YES | Sections 6, 13 | RefCell gets its own section (6) with full API, borrowing rules, and constraints. PromiseMutex gets its own section (13) with rationale, pattern, and coverage table. Both are listed in the "Authoritative for" line at the top. | None. Both are first-class sections, not afterthoughts. The "Authoritative for" line explicitly lists them. |
| 21 | Test files should be tracked similarly to source files in the port manifest | N/A (process) | -- | This is a process/tooling decision about how test files are tracked in the port manifest, not a memory-model spec concern. | Not a discrepancy. This decision applies to the port manifest, not the memory-model spec. The spec correctly does not address port manifest concerns. |
| 22 | Supporting code should be written directly, not as a TODO list | N/A (process) | -- | This is a process directive about implementation approach, not spec content. | Not a discrepancy. The spec provides implementation patterns and code examples rather than TODO lists, which is consistent with this directive. |
| 23 | Always delegate to background agents; supervisor should coordinate, not implement | N/A (process) | -- | This is a workflow directive for agents, not spec content. | Not a discrepancy. This is an operational instruction, not a memory-model design decision. |

---

## Contradiction / Softening Analysis

No contradictions found. The spec does not soften any of the user's explicit decisions.

Specific checks:

1. **Transaction severity**: The user was emphatic that Transaction is resource-hygiene only ("nobody is hurt"). The spec says exactly this -- "nobody is hurt" appears verbatim. Not softened.

2. **ResultSetWrite crash behavior**: The user wanted the app to "crash hard." The spec says "Crash the app hard" -- verbatim. Not softened.

3. **Mutex rehabilitation**: The user said Mutex was "dismissed too hastily." The spec includes extensive Mutex/RefCell content (Sections 6 and 13) as first-class patterns, not footnotes. The RefCell section explicitly mentions "application invariant enforcement" as a use case. Not softened.

4. **Fail loud + fail early**: The spec uses this exact phrase and applies it consistently to correctness-critical types. Not softened.

5. **The `using` escape hatch**: The user identified this as "the real concern." The spec calls it "the biggest concern" and dedicates a subsection to it. Not softened.

---

## Missing Decisions

No user decisions are entirely missing from the spec.

Decisions 21, 22, and 23 are process/workflow directives that correctly do not appear in a technical spec about memory model mapping. They would belong in a CLAUDE.md or workflow guide, not in the memory-model spec.

---

## Tone / Framing Check

The spec's language is consistent with how the user talks about these patterns:

- Uses "waste, not wrongness" (user's framing for resource hygiene)
- Uses "nobody is hurt" (user's language for Transaction severity)
- Uses "crash the app hard" (user's language for correctness-critical FR behavior)
- Uses "fail loud + fail early is a net improver of quality" (user's exact phrase)
- Uses "the biggest concern" for the `using` escape hatch (mirrors user's emphasis)
- Uses "like-for-like" for WeakRef mapping (user's term)
- Uses "key platform support delineator" for Metro/Hermes (user's framing)
- Uses "application invariant enforcement" for RefCell/Mutex purpose (user's reasoning)
- Uses "default safe approach" for Weak->WeakRef (user's phrasing)

The tone is direct, technical, and opinionated -- consistent with the user's communication style. The spec does not hedge where the user was definitive, and it does not add unnecessary caveats.

---

## Notes

1. **Decision 14 (inheritance exception)**: The spec implements Disposable as a base class (inheritance is the default path), which implicitly reflects the user's openness to inheritance. However, the spec could benefit from a brief note in Section 4 explicitly stating that using `extends Disposable` is the preferred approach when a type has no other base class, and that this is a deliberate design choice (not just a convenience). This is a minor enhancement, not a gap.

2. **Decisions 21-23**: These are process/operational directives, not memory-model design decisions. They are correctly absent from the spec. If a separate process document exists or is planned, these should be tracked there.

3. **Strong consistency**: The spec maintains consistent terminology throughout. The two-axis classification (severity x mechanism) provides a coherent framework that naturally organizes all the user's decisions about individual types (Transaction, ResultSetWrite, subscriptions). This is well-structured.
