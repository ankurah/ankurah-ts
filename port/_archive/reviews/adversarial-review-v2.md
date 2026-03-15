# Adversarial Red-Team Review v2: Restructured Memory Model Spec

**Reviewer**: Adversarial Red-Team Agent
**Date**: 2026-03-13
**Spec under review**: Three-file restructured spec:
- `specs/memory-model/overview.md` (rulebook)
- `specs/memory-model/decisions.md` (ankurah-specific classifications)
- `specs/memory-model/provided-types.md` (API docs)

**Prior review**: `specs/reviews/adversarial-review.md` (v1, against monolithic spec)
**Methodology**: Can an implementer following ONLY these three documents make a mistake the old monolithic spec would have caught? Are the rules clear enough to apply correctly without ankurah-specific type lists?

---

## Verdict: PASS WITH CONDITIONS

The restructuring is a significant improvement. Separating rules from type-specific decisions is architecturally sound and aligns with the "rulebook for ongoing translation" philosophy. However, the split introduced several gaps where information that was co-located in the monolithic spec now requires cross-referencing between documents, and some critical guidance was lost in the restructure. I found 2 HIGH issues, 3 MODERATE issues, and 3 LOW issues.

---

## Part 1: Information Loss -- What the Monolith Caught That the Split Doesn't

### Issue 1: Missing Guidance on onDispose() Throw Ordering

**Severity**: HIGH
**V1 reference**: Scenario 12 (onDispose throws)
**Old spec coverage**: Not covered in v1 either, but flagged in the v1 review recommendations.

**Problem**: `provided-types.md` documents `Disposable.dispose()` as "idempotent; calls onDispose() once" but says nothing about what happens if `onDispose()` throws. The current implementation (disposable.ts:84-89) sets `#disposed = true` and unregisters from FR *before* calling `onDispose()`. If `onDispose()` throws:

1. `#disposed` is permanently `true`
2. FR is unregistered (no leak warning will ever fire)
3. Actual cleanup never happened
4. Calling `dispose()` again is a silent no-op

An implementer reading only the spec would assume `dispose()` is safe to call and that either cleanup ran or will run. The spec needs to state the error contract:

- Does onDispose throwing mean the object is "disposed" or "wedged"?
- Should callers wrap dispose() in try/catch?
- Should the implementation change the ordering (call onDispose first, then set disposed)?

**Attack**: A class whose `onDispose()` does network cleanup (e.g., unsubscribe from a remote server) can fail under transient network errors, permanently wedging the object with no recourse and no diagnostic.

**Recommendation**: Add an "Error Handling" subsection to the Disposable section of `provided-types.md` specifying the contract. The simplest fix: call `onDispose()` first, only set `#disposed = true` if it succeeds, keeping FR registered as a backstop.

---

### Issue 2: RefCell async Detection is Specified but Not Shown in the API

**Severity**: HIGH
**V1 reference**: Scenario 5 (async callback in withMut)

**Problem**: `overview.md` line 85 states: "`withMut()` MUST detect async callbacks at runtime (`result instanceof Promise`) and throw. Fail loud, fail early." This is the right rule. However, `provided-types.md` line 103 shows the `withMut` signature as:

```typescript
withMut<R>(fn: (value: T) => R): R;
```

The "Runtime Enforcement" section at line 134 says: "If the callback returns a Promise, withMut() throws immediately (result instanceof Promise check)."

But the actual implementation (disposable.ts:220-234) does NOT perform this check:

```typescript
withMut<R>(fn: (value: T) => R): R {
    // ... borrow check ...
    this.#state = { kind: 'mut_borrowed' };
    try {
      return fn(this.#value);  // <-- No Promise check!
    } finally {
      this.#state = { kind: 'not_borrowed' };
      this.#onMutRelease?.();
    }
}
```

The spec documents a guard that does not exist in the implementation. An implementer reading the spec would believe they're protected. An automatic translator seeing the spec would not add its own guard because the spec says RefCell already handles it. This is a correctness-critical gap: an async callback inside `withMut` causes `onMutRelease` (broadcast) to fire before mutation completes, then allows concurrent `withMut` to run in parallel, violating the single-writer invariant.

**Attack**: Same as v1 Scenario 5 -- an async callback in `withMut()` silently breaks borrow tracking and can cause stale broadcasts.

**Recommendation**: Either (a) add the `instanceof Promise` check to the implementation and verify the spec matches, or (b) add a prominent warning in `provided-types.md` that this guard is "specified but not yet implemented" so implementers know they need to add it.

---

### Issue 3: Broadcast Error Isolation Still Absent

**Severity**: MODERATE
**V1 reference**: Scenario 7 (broadcast listener throws)

**Problem**: The v1 review identified that `Broadcast.send()` iterates listeners in a `for...of` loop with no try/catch (broadcast.ts:141-150). If any listener throws, subsequent listeners never fire. The monolithic spec didn't cover this, and the restructured spec still doesn't.

This matters because the spec positions broadcasts as the primary notification mechanism. `overview.md` says `onMutRelease` fires in the `finally` block after `withMut` completes, and `decisions.md` classifies result set mutation broadcasts as correctness-critical. But if the broadcast itself is fragile (one bad listener kills the chain), then the "correctness-critical" guarantee is undermined.

Neither `overview.md` nor `decisions.md` specifies error isolation semantics for broadcast delivery. An implementer could write a listener that throws and silently break all downstream observers.

**Recommendation**: Add a rule to `overview.md` under a "Broadcast Delivery" section or within the existing RefCell / Disposal rules: "Broadcast listeners MUST be called in isolation. A throwing listener MUST NOT prevent other listeners from receiving the notification. Implementations should catch exceptions per-listener and log them."

---

### Issue 4: DisposeGuard markDisposed(host) API Footgun

**Severity**: MODERATE
**V1 reference**: Scenario 6 (DisposeGuard host mismatch)

**Problem**: `provided-types.md` shows `markDisposed(host: object)` taking a `host` parameter. The constructor also takes `host`. If the caller passes a different object to `markDisposed` than was passed to the constructor, the FR unregisters the wrong object, causing a false-positive leak warning.

The spec shows correct usage (`this.#guard.markDisposed(this)`) but doesn't warn about the mismatch case. The implementation could simply store the host internally and not require it as a parameter to `markDisposed()`.

The bigger concern: an automatic translator might generate incorrect `markDisposed(someOtherRef)` calls because the API surface accepts any object.

**Recommendation**: In `provided-types.md`, either (a) change the API to `markDisposed()` (no parameter -- use stored host), or (b) add a warning that the host passed to `markDisposed` MUST be the same object reference passed to the constructor.

---

### Issue 5: Disposal Order in Vicarious RAII Not Specified Precisely Enough

**Severity**: MODERATE
**V1 reference**: New issue

**Problem**: `overview.md` line 67 says: "`onDispose()` disposes all owned Disposable fields (reverse construction order)." `decisions.md` describes three ownership chains (reactive subscription, signal subscription, listener guard). But neither document specifies what happens if disposing one field fails (throws) -- do you continue disposing the rest?

In Rust, `Drop` for each field runs independently. If one field's `Drop` panics, the remaining fields are still dropped (with potential double-panic abort). In TS, if `onDispose()` calls `this.fieldA.dispose()` and it throws, `this.fieldB.dispose()` and `this.fieldC.dispose()` never run.

For vicarious RAII chains like `LiveQuery -> EntityLiveQuery -> ReactorSubscription -> ReactorSubInner`, a throw at any level silently leaks all downstream disposals.

**Recommendation**: Add to the Disposal Rules section: "When disposing multiple owned fields, each `dispose()` call SHOULD be in its own try/catch to ensure partial failure does not prevent remaining fields from being disposed."

---

### Issue 6: Two-Axis Classification Doesn't Cover "Read" Misuse

**Severity**: LOW
**V1 reference**: Scenario 11 (stale read from committed fork)

**Problem**: The two-axis classification system in `overview.md` focuses on cleanup (what happens if cleanup doesn't run). It does not address use-after-dispose for *reads*. The old monolithic spec (Section 14) said property read methods "SHOULD" check `isWritable()` and warn to prevent stale data after commit.

The restructured spec's Lifetime Rules section (line 157) says "Property value types MUST check a writability flag before any mutation" but says nothing about reads. An implementer would correctly add mutation guards but leave reads unguarded, allowing stale data from committed transaction forks to be silently returned.

**Recommendation**: Add to Lifetime Rules: "Property value read methods SHOULD check the owning scope's alive/writable flag and warn if the scope is dead, as reads from a committed transaction fork may return stale data."

---

### Issue 7: Transaction alive Checks Not Mentioned in Rules

**Severity**: LOW
**V1 reference**: Scenario 4 (transaction double-get race)

**Problem**: `overview.md` Lifetime Rules (line 157) says "Types that use `fn method(self)` (move semantics) in Rust must set an `alive` flag to `false`" and talks about property value types checking writability. But it never explicitly says that the Transaction type's own methods (`create()`, `get()`, `edit()`) should check `this.alive.value` at entry.

The current implementation (transaction.ts) does NOT check `alive` in `create()`, `get()`, or `edit()`. After `commit()` sets `alive = false`, these methods still execute until they hit a downstream failure (or succeed silently). The `decisions.md` Transaction section mentions "commit() and rollback() set alive = false eagerly to close this gap" but doesn't say the methods themselves must check it.

An implementer following the spec would add `alive` flag management to commit/rollback but wouldn't know to add entry-point guards to `create/get/edit`.

**Recommendation**: In `decisions.md` under "Transaction alive gap", add: "Transaction methods (`create()`, `get()`, `edit()`) MUST check `this.alive.value` at entry and throw if the transaction has been committed or rolled back."

---

### Issue 8: FinalizationRegistry Severity Level Cross-Reference Gap

**Severity**: LOW
**V1 reference**: New issue

**Problem**: `overview.md` defines two FR severity levels (correctness-critical: crash; resource hygiene: warn). `decisions.md` classifies subsystems into these categories. But the current implementation uses a single `leakRegistry` (disposable.ts:37-43) that always `console.error` -- it never crashes. The `liveQueryRegistry` (livequery.ts:39-44) is a separate FR that does nothing (TODO stub).

An implementer reading the spec would need to:
1. Read `overview.md` to learn there are two severity levels
2. Read `decisions.md` to learn which types are correctness-critical
3. Read `provided-types.md` to see the Disposable API

But `provided-types.md` doesn't mention severity levels at all. The Disposable constructor just takes a `label`. There's no mechanism in the API to specify whether a type is correctness-critical or resource-hygiene. An implementer would have to modify the Disposable base class to support both behaviors, but the spec doesn't guide this.

**Recommendation**: Add a `severity` parameter to the Disposable constructor in `provided-types.md` (or a separate registration mechanism), and document how the FR callback switches behavior based on it.

---

## Part 2: Can the Split Spec Be Followed Correctly?

### Test 1: New Type Classification (PASS)

I attempted to classify a hypothetical new type `CursorPosition` (tracks a user's cursor in a collaborative document) using only `overview.md`.

1. "If cleanup never runs, does anyone see wrong data?" -- No, other users just see a stale cursor position. -> **Resource hygiene.**
2. "Can cleanup be structurally guaranteed by a scoped callback?" -- No, cursors live as long as the user's session. -> **User-managed.**

Classification: resource-hygiene / user-managed. This is correct and unambiguous. The two questions are clear decision points.

### Test 2: Implementing a New Disposable Type (PASS WITH NOTE)

I attempted to implement a new `DocumentSession` type using only `provided-types.md`.

The Disposable API docs and checklist are clear. I could implement the type correctly. However, I would not know what FR behavior to use (crash vs warn) because `provided-types.md` doesn't mention severity levels (see Issue 8).

### Test 3: Deciding Whether to Use PromiseMutex (PASS)

I attempted to decide whether a new `syncToServer()` method needs PromiseMutex using only `overview.md`.

The decision tree at lines 136-147 is clear: "If the Rust code uses tokio::sync::Mutex, you need PromiseMutex." The fire-and-forget task rule is also clear. This is well-structured.

### Test 4: Cross-Document Reference Flow (PARTIAL PASS)

I traced the path an implementer would follow for "How do I handle ResultSet mutations?"

1. `overview.md` Core Mapping Table -> row for "Mutex<T> / RwLock<T> (thread safety)" says "Eliminated -- plain field"
2. But ResultSet mutations need broadcast-on-release, which is a `RefCell` pattern
3. `overview.md` RefCell Rules say "Use RefCell when... Mutex/RwLock primarily for Drop-on-release semantics"
4. `decisions.md` classifies ResultSet as "correctness-critical / scope-guaranteed"
5. `provided-types.md` shows RefCell API with `onMutRelease`

This flow works but requires consulting all three documents. The old monolithic spec had this in one place (Section 3b). The split is justified by the "rulebook not remediation" philosophy, but it does add friction. Not a problem for a human who reads all three once; potentially a problem for an AI translator that may not cross-reference.

### Test 5: Identifying the ResultSetWrite Problem (FAIL)

Key question: would an implementer reading only these three documents realize that the current `ResultSetWrite` class should not exist as a public API?

- `overview.md` says to use RefCell for scope-guaranteed mutex patterns
- `decisions.md` says ResultSet mutation is correctness-critical / scope-guaranteed
- But neither document explicitly says "the existing ResultSetWrite class is wrong and should be replaced by RefCell.withMut()"

The old monolithic spec (Section 3b) explicitly said: "Do NOT create a long-lived ResultSetWrite object. The ResultSetWrite class should not exist as a public API in TS." That directive is gone. An implementer seeing the existing `resultset.write()` method would not know it contradicts the spec because the spec no longer calls out this specific anti-pattern.

This is exactly the kind of information that the "rulebook not remediation" philosophy says should be in source-code annotations, not the spec. The correct remedy is a code annotation on `ResultSetWrite` saying "// SPEC-VIOLATION: This class should be replaced by RefCell.withMut() per memory-model/overview.md RefCell Rules." Without that annotation, the gap exists.

---

## Part 3: V1 Scenario Re-Evaluation

How do the 12 v1 attack scenarios fare against the restructured spec?

| V1 # | Scenario | Restructured spec coverage | Status |
|-------|----------|---------------------------|--------|
| 1 | Zombie mutator | Lifetime Rules in overview.md: "MUST check writability flag before mutation" | COVERED (rule is there) |
| 2 | ResultSetWrite forgotten done() | RefCell Rules + decisions.md classification | PARTIALLY COVERED (see Test 5 above) |
| 3 | using escape hatch | Disposal checklist: "Public methods call assertNotDisposed()" | COVERED |
| 4 | Transaction double-get race | Not covered | NOT COVERED (see Issue 7) |
| 5 | Async callback in withMut | overview.md + provided-types.md | COVERED (but not implemented -- see Issue 2) |
| 6 | DisposeGuard host mismatch | provided-types.md shows correct usage | PARTIALLY COVERED (see Issue 4) |
| 7 | Broadcast listener throws | Not covered | NOT COVERED (see Issue 3) |
| 8 | fillGapsAndNotify race | decisions.md async serialization section | COVERED |
| 9 | WeakRef phantom resurrection | overview.md WeakRef timing note | COVERED |
| 10 | SystemManager concurrent join | decisions.md async serialization section | COVERED |
| 11 | Stale read from committed fork | Lifetime Rules mention mutation guards only | PARTIALLY COVERED (see Issue 6) |
| 12 | onDispose throws | Not covered | NOT COVERED (see Issue 1) |

**Coverage summary**: 6 fully covered, 3 partially covered, 3 not covered. The restructured spec covers the same major attack vectors as the monolith. The three uncovered scenarios are lower-severity issues that the monolith also didn't cover (they were identified by the v1 review as recommendations).

---

## Summary

| # | Issue | Severity | Category |
|---|-------|----------|----------|
| 1 | onDispose() throw ordering unspecified | HIGH | Information loss (API contract) |
| 2 | RefCell async detection specified but not implemented or API-visible | HIGH | Spec-implementation divergence |
| 3 | Broadcast error isolation not specified | MODERATE | Missing rule |
| 4 | DisposeGuard markDisposed(host) API footgun | MODERATE | API design |
| 5 | Vicarious RAII partial-failure behavior unspecified | MODERATE | Missing rule |
| 6 | Two-axis classification doesn't cover read misuse | LOW | Gap in classification |
| 7 | Transaction alive checks not mentioned | LOW | Missing guidance |
| 8 | FR severity level not surfaced in Disposable API | LOW | Cross-reference gap |

### Overall Assessment

The restructuring successfully separates rules from type-specific decisions, which is the right architecture for an ongoing translation rulebook. The two-axis classification system is clear and correctly applicable to new types. The three-document structure (rules / decisions / API) is logical.

The main risks are:
1. **Spec-implementation divergence** (Issue 2): The spec documents a guard (async detection in withMut) that doesn't exist. This is dangerous because it creates false confidence.
2. **Error contract gaps** (Issues 1, 3, 5): The spec covers the happy path well but doesn't specify what happens when things go wrong (onDispose throws, broadcast listeners throw, partial disposal failure). These "error-path" scenarios are where real-world bugs live.
3. **Annotation debt**: The "rulebook not remediation" philosophy is correct, but it creates a dependency on source-code annotations. If those annotations don't exist (e.g., `ResultSetWrite` is not annotated as a spec violation), implementers won't know the code contradicts the spec. The spec should either include a "migration notes" appendix or ensure annotations are added to violating code.

### Recommendations (Priority Order)

1. **Add `instanceof Promise` check to `RefCell.withMut()`** and verify provided-types.md matches the implementation
2. **Specify the onDispose() error contract** in provided-types.md
3. **Add broadcast error isolation rule** to overview.md
4. **Add source-code annotations** to types that currently violate the spec (ResultSetWrite, ReactorSubscription missing assertNotDisposed, etc.)
5. **Add FR severity parameter** to Disposable API or document how severity is configured
6. **Add Transaction alive entry-point checks** to decisions.md
