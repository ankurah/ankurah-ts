# Async Safety Review v2: Restructured Spec

**Reviewer**: Async Safety Reviewer Agent
**Date**: 2026-03-13
**Spec under review**: `specs/memory-model/overview.md`, `specs/memory-model/decisions.md`, `specs/memory-model/provided-types.md`
**Prior review**: `specs/reviews/async-safety-review.md` (v1, 2026-03-12)
**Verdict**: **PASS WITH NOTES**

---

## 1. Executive Summary

The three-file restructuring correctly separates general rules (overview.md), ankurah-specific decisions (decisions.md), and API documentation (provided-types.md). The async serialization content is split across all three files, and the split is **mostly correct** with one structural concern and one API documentation gap.

Key findings:
1. The overview.md / decisions.md split is **correct** in principle but **incomplete in practice** -- decisions.md omits one item from the v1 table (SystemManager lifecycle ops).
2. The PromiseMutex API in provided-types.md documents a `run()` method, but the actual implementation uses `acquire()` / manual release. This mismatch must be resolved.
3. All v1 findings remain valid -- the restructuring has not introduced regressions or lost content.

---

## 2. Is the Overview / Decisions Split Correct?

### What overview.md covers (Async Serialization Rules, lines 127-148)

- The `std::sync::Mutex` vs `tokio::sync::Mutex` distinction
- Rules for when `std::sync::Mutex` can/cannot be eliminated
- The fire-and-forget caveat
- Pointer to provided-types.md for the PromiseMutex API

**Assessment**: This is the right content for a general rulebook. These rules apply regardless of which ankurah subsystem is being translated. A new developer encountering any `tokio::sync::Mutex` in the Rust code can follow these rules without needing to know about specific ankurah subsystems. **CORRECT**.

### What decisions.md covers (Async Serialization Decisions, lines 37-44)

Three entries:
1. Reactor notification pipeline -- uses PromiseMutex (mirrors `tokio::sync::Mutex<()> notify_lock`)
2. WatcherSet mutation from gap-fill -- fire-and-forget `fillGapsAndNotify()`, needs serialization
3. LiveQuery activation ordering -- concurrent activations can race (issue #146)

**Assessment**: These are ankurah-specific adjudications that apply the general rules from overview.md to specific subsystems. This is the correct level of abstraction for decisions.md. **CORRECT** in principle.

### What is missing from decisions.md

**SystemManager lifecycle ops**: The v1 review identified a real TOCTOU race in `SystemManager.create()` (items check at line 252, items push at line 285, with two await points in between). The v1 spec's Section 13 table had this as an entry. It is absent from decisions.md.

Whether this omission is intentional or accidental matters:
- If intentional (because the risk is low and the same race exists in Rust): decisions.md should briefly note the decision NOT to serialize, so future translators don't re-discover the race and wonder if it's a bug.
- If accidental: it should be added back.

**Recommendation**: Add a brief entry:

```markdown
**SystemManager lifecycle ops**: `create()` and `joinSystem()` have a TOCTOU window
between the items check and items push (two await points in between). Same race
exists in Rust (RwLock guards dropped before `.await`). Low risk — initialization-time
only. No PromiseMutex currently; a synchronous "creating" flag is sufficient if
serialization is desired.
```

---

## 3. Is the PromiseMutex API in provided-types.md Complete?

### API mismatch: `run()` vs `acquire()`

provided-types.md documents this API:

```typescript
class PromiseMutex {
    async run<T>(fn: () => Promise<T>): Promise<T>;
}
```

The actual implementation in `packages/core/src/reactor/index.ts` (lines 122-135) uses a different API:

```typescript
class PromiseMutex {
    private queue: Promise<void> = Promise.resolve();
    async acquire(): Promise<() => void>;
}
```

The `acquire()` pattern returns a release function that the caller must invoke manually. The `run()` pattern takes a callback and handles release in `finally`. These are semantically equivalent but have important ergonomic and safety differences:

| Aspect | `acquire()` + manual release | `run()` + callback |
|--------|-----------------------------|--------------------|
| Forgetting to release | Silent deadlock (all subsequent `run`/`acquire` calls hang forever) | Impossible (try/finally) |
| Matching Rust idiom | Closer to `let _guard = mutex.lock().await` (but Rust has Drop) | Closer to a scoped lock |
| Usage in `notifyChange` | Currently used: `const release = await this.notifyLock.acquire(); try { ... } finally { release(); }` | Would be: `await this.notifyLock.run(async () => { ... })` |

**Assessment**: The `run()` API documented in provided-types.md is **the better design** -- it eliminates the "forgot to release" class of bugs entirely, which is exactly the kind of compile-time guarantee that Rust provides via Drop and that TS must enforce at the API level. This is consistent with the spec's own philosophy (RefCell uses `withMut` for the same reason).

However, the spec must acknowledge the current implementation mismatch and either:
1. Update the implementation to use `run()` (preferred -- consistent with RefCell's `withMut` philosophy), or
2. Update the spec to document `acquire()` and note it should be migrated to `run()`.

**Recommendation**: The spec should document `run()` as the target API (as it currently does) AND note that the current implementation uses `acquire()` and should be migrated. This makes the spec a forward-looking rulebook rather than a snapshot of current code.

### Missing: `acquire()` documentation

If `acquire()` is kept (even temporarily), provided-types.md should document it for completeness. Currently only `run()` is documented.

### Missing: Error handling semantics

The `run()` implementation in provided-types.md correctly propagates errors:

```typescript
try {
    return await fn();
} finally {
    resolve!();
}
```

This means: if `fn()` throws, the mutex is still released (via `finally`), and the error propagates to the caller. This is correct and important. The API docs should explicitly state: "If the callback throws, the mutex is released and the error propagates."

### Missing: Non-async callback support

The `run()` signature is `fn: () => Promise<T>`. Should it also accept synchronous callbacks? The current signature allows:

```typescript
await lock.run(async () => syncOperation()); // works but unnecessary async wrapper
```

This is fine. Forcing `async` makes intent clear. No change needed.

### Missing: Deadlock detection / timeout

Neither the spec nor the implementation has any deadlock detection or timeout. If a callback never resolves, all subsequent `run()` calls hang forever. This is consistent with Rust's `tokio::sync::Mutex` (which also has no timeout by default). **No change needed** for the spec, but worth noting as a known limitation.

---

## 4. Cross-File Consistency Check

### overview.md references

- "See [provided-types.md](provided-types.md) for the PromiseMutex API" (line 148) -- **CORRECT**, link exists and target has content.

### decisions.md references

- "Uses `PromiseMutex` (mirrors Rust's `tokio::sync::Mutex<()> notify_lock`)" (line 39) -- **CORRECT**, consistent with overview.md rule and provided-types.md API.
- "Needs either awaiting within the serialized pipeline or its own PromiseMutex" (line 41) -- **CORRECT**, this is an adjudication that applies the overview.md rules.
- "Needs serialization or coalescing" (line 43) -- **CORRECT**, properly defers implementation choice.

### provided-types.md references

- "Equivalent to Rust's `tokio::sync::Mutex<()>` -- serializes async operations that must not interleave" (line 141) -- **CORRECT**.

### No circular or broken references detected.

---

## 5. Revisiting v1 Findings Against the Restructured Spec

### v1 Entry 1 (Reactor notification pipeline) -- RESOLVED
Covered in decisions.md line 39. Implementation verified in `reactor/index.ts:170` (`notifyLock`). **No change needed.**

### v1 Entry 2 (WatcherSet gap-fill race) -- COVERED
Covered in decisions.md lines 41-42. The fire-and-forget pattern at `subscription_state.ts:532` remains unchanged. The spec correctly identifies this needs fixing. **No change needed in spec.**

The code still has the fire-and-forget call:
```
subscription_state.ts:532: this.fillGapsAndNotify(updateItems, gapsToFill);
```
This remains the highest-priority async safety issue.

### v1 Entry 3 (SystemManager lifecycle ops) -- MISSING
Not in decisions.md. See recommendation in Section 2 above.

### v1 Entry 4 (LiveQuery activation ordering) -- COVERED
Covered in decisions.md line 43. **No change needed.**

### v1 fire-and-forget inventory (Section 5) -- IMPLICITLY COVERED
The overview.md fire-and-forget rule (lines 140-146) correctly captures the general principle. Individual instances are covered by decisions.md entries. **No change needed.**

### v1 std::sync::Mutex elimination table (Section 6) -- IMPLICITLY COVERED
overview.md lines 136-138 state the elimination rule. The individual eliminations are implementation details, not spec-level decisions. **Correct not to duplicate them in decisions.md.**

---

## 6. Structural Observations

### The split is well-motivated

The three-file structure avoids the problem the v1 review implicitly had: Section 13 mixed general rules with ankurah-specific entries in a single table. Now:
- A translator encountering a new `tokio::sync::Mutex` reads overview.md for the rule.
- A translator working on a specific subsystem reads decisions.md for the adjudication.
- A translator implementing PromiseMutex reads provided-types.md for the API.

This layering matches the spec's stated philosophy of being a rulebook, not a remediation plan.

### decisions.md async section is thin

Only three bullet points. This is appropriate for now -- there are only three subsystems with async serialization concerns. As the codebase grows, this section may need sub-headings, but premature structure would be worse.

---

## 7. Summary of Required Changes

| Priority | Item | File | Action |
|----------|------|------|--------|
| **HIGH** | PromiseMutex API mismatch | provided-types.md | Note that current impl uses `acquire()`, target API is `run()`. Or update impl. |
| **MEDIUM** | SystemManager lifecycle ops missing | decisions.md | Add brief entry noting the TOCTOU race and low-risk assessment |
| **LOW** | Error propagation semantics | provided-types.md | Add one sentence: "If the callback throws, the mutex is released and the error propagates." |

---

## 8. Verdict

**PASS WITH NOTES**. The three-file restructuring is sound. The async serialization content is correctly distributed across the files, with overview.md handling the general rules, decisions.md handling ankurah-specific adjudications, and provided-types.md handling the API. The two substantive issues (PromiseMutex API mismatch, missing SystemManager entry) are straightforward to fix and do not affect the structural soundness of the split.

All v1 findings remain valid and are properly covered by the restructured spec (except the SystemManager omission). No regressions introduced by the restructuring.
