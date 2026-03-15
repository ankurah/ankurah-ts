# Semantic Soundness Review v2: Restructured Memory Model Spec

**Reviewer**: Semantic Soundness Reviewer Agent
**Date**: 2026-03-13
**Spec reviewed**: `specs/memory-model/overview.md`, `specs/memory-model/decisions.md`, `specs/memory-model/provided-types.md`
**Prior review**: `specs/reviews/semantic-soundness-review.md` (2026-03-12, against monolithic `memory-model.md`)
**Verdict**: Significant improvement over v1. All five prior critical findings have been addressed at the spec level. Two spec-vs-implementation divergences remain, both clearly actionable.

---

## Methodology

1. Re-read the prior review to establish baseline findings
2. Read all three restructured spec files
3. Verified each prior finding against the new spec text
4. Checked the source code (`disposable.ts`, `transaction.ts`) for spec-implementation alignment
5. Assessed completeness of the Inherent Limitations section

---

## Prior Findings: Disposition

### 1. Async detection rule in `withMut()` — RESOLVED IN SPEC, NOT IN CODE

**Prior finding**: The spec mentioned "no async inside `withMut()`" as a constraint but did not include it in the Inherent Limitations section, and the code had no runtime guard.

**New spec (overview.md, RefCell Rules)**: Constraint 1 now explicitly states: "`withMut()` MUST detect async callbacks at runtime (`result instanceof Promise`) and throw. Fail loud, fail early."

**New spec (provided-types.md, Runtime Enforcement)**: Documents the detection mechanism: "If the callback returns a `Promise`, `withMut()` throws immediately (`result instanceof Promise` check). Async callbacks would break the borrow tracking because `finally` runs at the first `await`."

**Status**: The spec is now clear and correct. However, the actual `RefCell.withMut()` implementation in `disposable.ts:220-234` does NOT perform the `instanceof Promise` check. The spec prescribes the right behavior; the code needs to catch up. This is a straightforward one-line fix:

```typescript
// In withMut(), after line 229:
const result = fn(this.#value);
if (result instanceof Promise) {
    throw new Error(`${this.#label}: withMut() callback must be synchronous — returned a Promise`);
}
return result;
```

**Soundness of the detection**: The `instanceof Promise` check is a pragmatic 95% solution. It catches `async function` callbacks (which always return native Promises) and explicit `return new Promise(...)`. It does NOT catch non-Promise thenables or callbacks that schedule work via `setTimeout`/`queueMicrotask` without returning a Promise. The spec should acknowledge this limitation explicitly — but the gap is narrow enough that I consider the rule sound for practical purposes.

### 2. No move semantics — RESOLVED

**Prior finding**: Missing from Inherent Limitations.

**New spec (overview.md, Inherent Limitations)**: Now has a dedicated "No move semantics" section: "Rust's `commit(self)` consumes the value — caller can't use it after. JS has no equivalent. After `trx.commit()`, `trx` is still a valid reference to a dead object. Mitigation: `alive` flag set eagerly, checked at every mutation point."

**New spec (overview.md, Core Mapping Table)**: `fn method(self)` (move/consume) is now mapped to "Runtime `alive` flag check" with a cross-reference to the limitation.

**Status**: Complete and correct.

### 3. FinalizationRegistry two-tier policy — RESOLVED IN SPEC, NOT IN CODE

**Prior finding**: The spec claimed two tiers (warn vs crash) but the implementation only had `console.error`.

**New spec (overview.md, FinalizationRegistry Policy)**: Now explicitly documents both tiers:
- Resource hygiene: `console.error` with creation stack trace
- Correctness-critical: `queueMicrotask(() => { throw ... })` with creation stack trace

**Status**: The spec is clear. The implementation in `disposable.ts:37-43` still only has the single `console.error` path. However, this is less urgent than the async detection gap because:
- The only correctness-critical type (result set mutation) uses `RefCell/withMut` which guarantees cleanup via `try/finally`, making FR irrelevant for the happy path
- FR for correctness-critical types is a backstop for bugs, not a primary mechanism
- The spec correctly states this: "For correctness-critical types, the scope-guaranteed mechanism (RefCell try/finally) makes FR irrelevant"

Still, the implementation should add the hard-crash path to match the spec.

### 4. Transaction alive checks — STATUS UNCHANGED

**Prior finding**: `Transaction.create()`, `get()`, and `edit()` do not check `this.alive.value`.

**New spec (overview.md, Lifetime Rules)**: Correctly requires alive checks. The Transaction class in `transaction.ts` still does not implement them. The spec is correct; the code has not changed.

This remains a developer-experience issue, not a correctness issue (mutations affect orphaned fork entities).

### 5. `write()/done()` vs `RefCell.withMut()` — STATUS UNCHANGED

**Prior finding**: The codebase uses `write()/done()` pattern instead of the prescribed `RefCell.withMut()`.

**New spec (decisions.md)**: Correctly classifies result set mutation as correctness-critical and scope-guaranteed (RefCell/withMut). The code in `resultset.ts` and `subscription_state.ts` still uses `write()/done()`.

The spec is correct; migration is pending.

---

## New Assessment: Restructured Spec

### Structure and Clarity

The three-file split is a clear improvement:

- **overview.md** works as a quick-reference rulebook. A developer translating a new Rust type can follow the classification flowchart and checklists without reading the full rationale.
- **decisions.md** captures the "why" for ankurah-specific choices without cluttering the general rules. The severity classifications are crisp.
- **provided-types.md** is a clean API reference. The borrowing rules table and usage examples are sufficient for implementation.

Cross-references between files are present and correct.

### Completeness of Inherent Limitations (overview.md)

The section now lists five limitations:

| Limitation | Present in v1 spec? | Correct? |
|------------|---------------------|----------|
| No compile-time lifetime enforcement | Yes | Yes |
| RefCell reference escape | Yes | Yes |
| FinalizationRegistry non-determinism | Yes | Yes |
| No move semantics | **No (was missing)** | Yes — now complete |
| WeakRef timing non-determinism | Yes | Yes |

**All five are semantically correct.** The mitigations described are appropriate and honest.

**One omission remains, but it is minor**: The async-callback-in-`withMut()` issue is documented in the RefCell Rules section (constraint 1) and in provided-types.md (Runtime Enforcement), but is NOT listed in the Inherent Limitations section of overview.md. This is borderline — the spec now prescribes a runtime detection mechanism (`instanceof Promise`), which arguably removes it from "inherent limitation" territory and into "enforced constraint" territory. If the runtime check is implemented, the gap shrinks to "non-Promise thenables and side-effect-only async patterns," which is narrow enough that omitting it from Inherent Limitations is defensible.

**Recommendation**: Add a one-line note to Inherent Limitations acknowledging the residual gap: "The `instanceof Promise` check catches `async` callbacks but not non-Promise thenables or callbacks that schedule async work without returning a Promise."

### Semantic Correctness of Mappings

All mappings in the Core Mapping Table are semantically correct. I'll highlight the ones I scrutinized most carefully:

**`fn method(self)` -> Runtime `alive` flag check**: This is the right mapping. The spec correctly notes that this is a mitigation, not a full equivalent — the developer gets a runtime error instead of a compile-time error. The `alive` flag must be set eagerly (in `commit()`/`rollback()`, not deferred to GC), and the spec correctly documents this in decisions.md under "Transaction alive gap."

**`tokio::sync::Mutex` -> `PromiseMutex`**: The spec's rule of thumb ("If the Rust code uses `tokio::sync::Mutex`, you need `PromiseMutex`. If `std::sync::Mutex` and there's a fire-and-forget task, you also need `PromiseMutex`") is correct and actionable. The fire-and-forget case is the subtle one, and the spec handles it well.

**Vicarious RAII**: The decisions.md correctly identifies the three ownership chains (reactive subscription, signal subscription, listener guard). The rule that "each level must explicitly `dispose()` its owned Disposable fields" is correct and matches what Rust does automatically via Drop cascading.

### Decisions.md: Known Architectural Gotchas

All four gotchas are correctly described:

1. **NodeLikeAdapter strong reference rule**: Correct — WeakRef-only adapters can be GC'd while subscriptions are active.
2. **Transaction alive gap**: Correct — eagerly setting `alive = false` in `commit()`/`rollback()` closes the gap between unreachability and GC.
3. **`using` escape hatch**: Correct — `assertNotDisposed()` converts silent failure to loud error.
4. **Observer stack balance**: Correct — `try/finally` for push/pop is necessary.

### Provided-types.md: API Correctness

**Disposable**: API matches the implementation in `disposable.ts`. The `[Symbol.dispose]()` delegation to `dispose()` is correct.

**DisposeGuard**: API matches the implementation. The `host` parameter for FR registration is correct — it ensures the FR tracks the host object, not the guard.

**RefCell**: API matches the implementation EXCEPT for the `instanceof Promise` check (documented in spec, missing in code). The borrowing rules table is correct and matches the state machine in `disposable.ts:193-260`.

**PromiseMutex**: The spec shows a `run()` API; the actual implementation uses `acquire()/release()`. The spec's `run()` pattern is safer (built-in `try/finally`). The provided-types.md documents the `run()` pattern as the target API, which is correct — the implementation should converge to this.

---

## Soundness Issues

### Issue 1: `instanceof Promise` check — spec claims it, code doesn't have it (MEDIUM)

**Location**: provided-types.md line 134, overview.md line 85
**Code**: `disposable.ts:220-234`

The spec states as fact that `withMut()` performs this check. A developer reading the spec would believe the guard exists. If the check is not implemented, an `async` callback silently breaks borrow tracking.

**Risk**: An `async` callback in `withMut()` would release the borrow at the first `await` while the callback continues executing. If another `withMut()` call interleaves, both would succeed — violating the single-writer invariant. For result set mutations, this could cause a broadcast to fire before the mutation is complete (observers see partial state).

**Recommendation**: Implement the check. It is a 3-line change in `withMut()`.

### Issue 2: FinalizationRegistry hard-crash path missing (LOW)

**Location**: overview.md line 119
**Code**: `disposable.ts:37-43`

The spec describes `queueMicrotask(() => { throw ... })` for correctness-critical types. The code only has `console.error`. As noted above, this is low priority because correctness-critical types use scope-guaranteed mechanisms where FR is a backstop.

**Recommendation**: Add the hard-crash path when implementing Disposable subclasses for correctness-critical types. The `Disposable` constructor could accept a `severity` parameter that controls FR behavior.

---

## Summary Table

| Area | v1 Verdict | v2 Verdict | Change |
|------|-----------|-----------|--------|
| Inherent Limitations completeness | INCOMPLETE (missing 2) | COMPLETE (all 5 present) | Fixed |
| Async detection rule | Missing from limitations | Documented as enforced constraint | Fixed (spec level) |
| No move semantics | Missing from limitations | Dedicated section | Fixed |
| Core mapping table | Correct but incomplete | Correct and complete (added `fn method(self)`) | Fixed |
| FR two-tier policy | Spec/code divergence | Spec clear, code unchanged | Spec improved |
| RefCell `instanceof Promise` | Not in spec or code | In spec, not in code | Spec improved, code needs work |
| `write()/done()` migration | Spec/code divergence | Unchanged | Pending |
| Transaction alive checks | Spec/code divergence | Unchanged | Pending |

---

## Conclusion

The restructured spec is a substantial improvement:

1. **Inherent Limitations are now complete.** The two gaps from v1 (no move semantics, async callback detection) are addressed.
2. **The three-file structure works.** Overview is a rulebook, decisions captures the "why," provided-types is an API reference. No redundancy, clear cross-references.
3. **All mappings are semantically correct.** No mapping claims something false about the guarantees provided.
4. **Two spec-vs-code divergences remain** (async detection check, FR hard-crash path), both clearly documented in the spec as requirements. These are implementation tasks, not spec defects.
5. **The spec correctly functions as a rulebook for ongoing translation**, not a one-time remediation plan. A developer encountering a new Rust type with `impl Drop` can follow the classification flowchart and checklists to produce the correct TS equivalent.

The spec is sound. The implementation needs to catch up on two specific points (`instanceof Promise` guard in `withMut()`, FR severity tiers), both of which are straightforward.
