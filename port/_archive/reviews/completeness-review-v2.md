# Completeness Review v2: Restructured Memory Model Spec

**Reviewer**: Completeness Reviewer Agent
**Date**: 2026-03-14
**Spec version reviewed**: Three-file split (`overview.md`, `decisions.md`, `provided-types.md`)
**Prior review**: `completeness-review.md` (2026-03-12, single-file version)

---

## Verdict: PASS

The restructured spec covers everything the monolithic version covered. Nothing was lost in the split. The two minor gaps from the v1 review (ReactObserver vicarious RAII, PNCounter Weak) remain — they were not addressed in the restructuring, but they remain low-severity and were already PASS WITH NOTES in v1.

The restructured spec is actually *better* for completeness because the separation of concerns makes it clearer which document is authoritative for which question: rules in `overview.md`, ankurah-specific classifications in `decisions.md`, API contracts in `provided-types.md`.

---

## Traceability: v1 Patterns Through the Three-File Split

### 1. `impl Drop` Patterns

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Drop -> Disposable/DisposeGuard | `overview.md` Core Mapping Table, Disposal Rules |
| Drop severity classification | `overview.md` Two-Axis Classification |
| Result set mutation broadcast (correctness-critical) | `decisions.md` Classification of Major Subsystems |
| Reactor subscriptions / live queries (resource hygiene) | `decisions.md` Classification of Major Subsystems |
| Signal listener guards (resource hygiene) | `decisions.md` Classification of Major Subsystems |
| Transactions (decision-required) | `decisions.md` Classification of Major Subsystems |
| NodeInner Drop (logging only) | `overview.md` Core Mapping Table ("logging only -> Nothing needed") |

**Status**: COMPLETE. All 6 meaningful Drop types from v1 Section 1 are covered by rules + classifications.

### 2. `Arc<T>` Usage

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Arc -> plain reference rule | `overview.md` Core Mapping Table |

**Status**: COMPLETE. The rule is clear and applies to all 21 Arc usages inventoried in v1. Per the spec's design principle (rulebook, not remediation plan), individual Arc instances don't need enumeration.

### 3. `Weak<T>` Usage

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Weak -> WeakRef rule | `overview.md` Core Mapping Table, WeakRef Rules section |
| Strong reference holder requirement | `overview.md` WeakRef Rules |
| `deref()` undefined handling | `overview.md` WeakRef Rules |
| Map cleanup with FR | `overview.md` WeakRef Rules + FinalizationRegistry Policy |
| NodeLikeAdapter strong ref rule | `decisions.md` Known Architectural Gotchas |
| WeakRef timing non-determinism | `overview.md` Inherent Limitations |

**Status**: COMPLETE. The general rules cover all 9 Weak usages from v1. The v1 Gap 2 (PNCounter backend Weak not listed) remains, but this is a narrow type-specific adjudication that belongs as a source annotation per the spec's design principle.

### 4. `Mutex<T>` / `RwLock<T>` Usage

| v1 item | Where in restructured spec |
|---------|---------------------------|
| std::sync::Mutex -> eliminated | `overview.md` Core Mapping Table, Async Serialization Rules |
| tokio::sync::Mutex -> PromiseMutex | `overview.md` Core Mapping Table, Async Serialization Rules |
| Elimination criteria (no .await crossings) | `overview.md` Async Serialization Rules ("When std::sync::Mutex can be eliminated") |
| Fire-and-forget exception | `overview.md` Async Serialization Rules ("When std::sync::Mutex CANNOT be eliminated") |
| Reactor notify_lock -> PromiseMutex | `decisions.md` Async Serialization Decisions |
| WatcherSet gap-fill serialization | `decisions.md` Async Serialization Decisions |
| LiveQuery activation race | `decisions.md` Async Serialization Decisions |

**Status**: COMPLETE. All 14 Mutex and 16 RwLock usages from v1 are covered by the elimination rule + the three specific decisions.

### 5. `RefCell<T>` Usage

| v1 item | Where in restructured spec |
|---------|---------------------------|
| RefCell rules and constraints | `overview.md` RefCell Rules |
| No async inside withMut | `overview.md` RefCell Rules constraint 1, `provided-types.md` Runtime Enforcement |
| onMutRelease for Drop-on-release | `provided-types.md` RefCell API + Usage example |
| Re-entrancy protection | `provided-types.md` Borrowing Rules table |
| Reference escape limitation | `overview.md` RefCell Rules constraint 2, Inherent Limitations |
| Result set mutation -> RefCell/withMut | `decisions.md` Classification (Correctness-Critical, Scope-Guaranteed) |

**Status**: COMPLETE.

### 6. Lifetime Parameters

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Lifetime -> runtime alive flags | `overview.md` Core Mapping Table, Lifetime Rules |
| Move semantics -> alive flag | `overview.md` Core Mapping Table, Inherent Limitations (No move semantics) |
| Transaction alive gap | `decisions.md` Known Architectural Gotchas |
| Property writability checks | `overview.md` Lifetime Rules |

**Status**: COMPLETE.

### 7. Vicarious RAII

| v1 item | Where in restructured spec |
|---------|---------------------------|
| General rule: onDispose cascades | `overview.md` Core Mapping Table, Disposal Rules |
| Reactive subscription chain | `decisions.md` Vicarious RAII |
| Signal subscription chain | `decisions.md` Vicarious RAII |
| Listener guard chain | `decisions.md` Vicarious RAII |

**Status**: COMPLETE. All 7 explicitly listed vicarious RAII types from v1 are covered by the three chains. The v1 Gap 1 (ReactObserver WASM/RN) remains — per the spec's design principle, this would be a source annotation.

### 8. FinalizationRegistry Policy

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Severity-based FR behavior | `overview.md` FinalizationRegistry Policy |
| Hard crash for correctness-critical | `overview.md` FinalizationRegistry Policy |
| Warn for resource hygiene | `overview.md` FinalizationRegistry Policy |
| FR not a primary cleanup mechanism | `overview.md` FinalizationRegistry Policy |
| Map hygiene (WeakRef cleanup) | `overview.md` FinalizationRegistry Policy |

**Status**: COMPLETE.

### 9. Observer Stack Context

| v1 item | Where in restructured spec |
|---------|---------------------------|
| try/finally for push/pop balance | `decisions.md` Known Architectural Gotchas (Observer stack balance) |

**Status**: COMPLETE.

### 10. Provided Types (API Contracts)

| v1 item | Where in restructured spec |
|---------|---------------------------|
| Disposable API | `provided-types.md` Disposable section |
| DisposeGuard API | `provided-types.md` DisposeGuard section |
| RefCell API + borrowing rules | `provided-types.md` RefCell section |
| PromiseMutex API + implementation | `provided-types.md` PromiseMutex section |
| Symbol.dispose polyfill | `provided-types.md` Symbol.dispose Polyfill section |

**Status**: COMPLETE.

---

## v1 Gaps Status

### Gap 1 (ReactObserver WASM/RN vicarious RAII) — STILL OPEN, LOW SEVERITY

The ReactObserver types own ListenerGuards and are vicarious RAII. They are not mentioned in `decisions.md`. This is acceptable because:
- The TS port uses native React hooks (`useEffect` cleanup), not a port of the Rust observer
- Per the spec's design principle, this is a narrow type-specific adjudication that belongs as a source code annotation

### Gap 2 (PNCounter backend Weak) — STILL OPEN, VERY LOW SEVERITY

PNCounter holds `Weak<PNBackend>` which maps to a plain reference in TS (backend lifetime is tied to Entity, which is GC-managed). Not mentioned in the spec. This is acceptable because:
- The spec's WeakRef Rules provide sufficient guidance
- This is a narrow type-specific adjudication that belongs as a source code annotation

---

## New Observations on the Restructured Spec

### Improvement: Clearer Separation of Concerns

The three-file split resolves an ambiguity in the monolithic version where generic rules and ankurah-specific decisions were interleaved. Now:
- `overview.md` can be read by any implementer without domain knowledge
- `decisions.md` captures the "why" for ankurah-specific choices
- `provided-types.md` serves as a standalone API reference

### Improvement: Design Principle is Explicit

The spec header in `decisions.md` now explicitly states: "Narrow, type-specific adjudications should be annotated in the source code itself, not here." This directly addresses the v1 review's implicit assumption that every type should be enumerated in the spec. The spec is a rulebook, not a checklist.

### Minor Suggestion: Cross-References

The three files cross-reference each other well. One additional cross-reference would be helpful:
- `decisions.md` Vicarious RAII section could link to `overview.md` Disposal Rules for the general cascade rule. Currently the chains are described but the "how" (onDispose cascading) requires the reader to already know the overview.

### The `using` Escape Hatch

`decisions.md` documents the `using` escape hatch gotcha well. The corresponding mitigation (`assertNotDisposed()`) is documented in `overview.md` Disposal Rules checklist and `provided-types.md` Disposable API. Good coverage across all three files.

---

## Cross-Reference Matrix (v2)

| Pattern category | `overview.md` | `decisions.md` | `provided-types.md` |
|-----------------|---------------|-----------------|----------------------|
| Drop/Disposable | Mapping rule, checklist | Classification of subsystems | Disposable/DisposeGuard API |
| Vicarious RAII | Mapping rule, cascade rule | Three ownership chains | onDispose API |
| RefCell | Rules, constraints | Result set classification | API, borrowing rules, enforcement |
| WeakRef | Rules, timing caveat | NodeLikeAdapter gotcha | — |
| Mutex elimination | Rules, criteria | Reactor/WatcherSet/LiveQuery decisions | — |
| PromiseMutex | Mapping rule | Reactor/WatcherSet/LiveQuery decisions | API, implementation |
| Lifetimes | Mapping rule, rules | Transaction alive gap | — |
| FR policy | Severity-based behavior | — | — |
| Observer stack | — | Observer stack balance gotcha | — |
| Move semantics | Mapping rule, limitation | — | — |

Every row has coverage in at least one file. No orphaned patterns.

---

## Final Assessment

**Verdict: PASS**

The restructured three-file spec is a strict superset of the coverage from the monolithic version. No patterns were lost in the split. The two v1 gaps remain open but are correctly scoped as source-level annotations per the spec's own design principle. The separation of concerns is an improvement.
