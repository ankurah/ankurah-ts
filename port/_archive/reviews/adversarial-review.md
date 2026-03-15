# Adversarial Red-Team Review: memory-model.md

**Reviewer**: Adversarial Red-Team Agent
**Date**: 2026-03-12
**Spec under review**: `/ankurah-ts/specs/memory-model.md`
**Source files traced**: All 14 implementation files listed in brief

---

## Verdict: PASS WITH NOTES

The spec is thorough and addresses the most dangerous failure modes. However, several adversarial attack vectors succeed against the current **implementation** even though the spec identifies them conceptually. The gap is between what the spec *prescribes* and what the code *enforces*. I found 3 CRITICAL issues (all related to known gaps the spec acknowledges but where code has no guard), 4 MODERATE issues, and 5 LOW issues.

---

## Adversarial Scenarios

### Scenario 1: MutableBorrow Use-After-Commit (The Zombie Mutator)

**Severity**: CRITICAL

**Adversarial code (TypeScript)**:
```typescript
const trx = await ctx.begin();
const record = await trx.create(Video, { title: 'test' });
const mutable = record.inner; // MutableInstance extracted

await trx.commit(); // alive = false

// Zombie: mutable still holds a reference to the forked Entity
// The Entity's kind.trxAlive.value is now false, but...
const handle = mutable.title(); // calls entity.getActiveHandle('title', 'lww')
// handle is { backend, fieldName, entity } -- a raw object with NO guards
handle.backend.set('title', { type: 'String', value: 'HACKED' });
// This mutates the FORKED entity's backend directly, bypassing all checks
```

**Rust behavior**: Compile error. `MutableBorrow<'rec, T>` has lifetime `'rec` tied to the transaction. After `trx.commit()` consumes `self`, the borrow checker prevents any use of `record` or `mutable`.

**TS behavior (traced through real code)**:
1. `trx.commit()` calls `commitLocalTrx` which sets `trx.alive.value = false` (node.ts:265)
2. `record.inner` is a `GeneratedMutableClass` holding a reference to the forked `Entity` (define-model.ts:366-394)
3. `mutable.title()` calls `entity.getActiveHandle('title', 'lww')` (define-model.ts:408-409)
4. `getActiveHandle` returns `{ backend, fieldName, entity: this }` (entity.ts:278-279) -- a **raw object with no guards**
5. The caller can directly call `handle.backend.set(...)` on the LWW backend
6. This succeeds because `LWWBackend.set()` does NOT check `entity.isWritable()`

**Does the spec address this?** YES -- Section 14 (Lifetime Enforcement) explicitly states: "Property value types (`LWW.set()`, `YrsString.insert()`, etc.) MUST check `entity.isWritable()` before any mutation" and "The `defineModel()` generated Mutable class MUST return properly guarded property value instances (not raw `{backend, fieldName, entity}` handles)".

**Is the mitigation sufficient?** The spec is correct, but the **implementation violates it**. `entity.getActiveHandle()` at entity.ts:275-279 returns raw `{ backend, fieldName, entity }`. The `defineModel()` Mutable getter at define-model.ts:408-409 passes this through with no wrapping. Neither `LWWBackend.set()` nor the returned handle checks `isWritable()`. The spec identifies this as a known attack vector but the code has not been updated to match.

---

### Scenario 2: ResultSetWrite Leak via Forgotten done()

**Severity**: CRITICAL

**Adversarial code (TypeScript)**:
```typescript
// Attacker gets a write guard and never calls done()
const resultset = EntityResultSet.empty();
const writer = resultset.write();
writer.add(someEntity);
// writer goes out of scope without done() -- broadcast NEVER fires
// All observers silently see stale data
```

**Rust behavior**: `ResultSetWrite` has `impl Drop` that broadcasts changes. The broadcast fires automatically when the write guard is dropped, regardless of whether the user explicitly calls anything.

**TS behavior (traced through real code)**:
1. `resultset.write()` returns `new ResultSetWrite(this, this.state)` (resultset.ts:492-493)
2. `ResultSetWrite` has no `Symbol.dispose`, no `Disposable` base class, no FinalizationRegistry registration
3. `done()` at resultset.ts:379-383 is the ONLY way to trigger the broadcast
4. If `done()` is never called, `this.changed` is `true` but `this.resultset._broadcast()` never fires
5. The `ResultSetWrite` is eventually GC'd with zero notification

**Does the spec address this?** YES -- Section 3b explicitly says: "Do NOT create a long-lived `ResultSetWrite` object. The `ResultSetWrite` class should not exist as a public API in TS. All mutation must go through `RefCell.withMut()`." Section 8 says FinalizationRegistry must hard-crash for correctness-critical types like ResultSetWrite.

**Is the mitigation sufficient?** The spec is excellent here, but the **code contradicts it completely**. `EntityResultSet.write()` is a public method (resultset.ts:492) that returns a raw `ResultSetWrite`. There is no `RefCell` wrapping anywhere in `resultset.ts`. The `ResultSetWrite` class has no FinalizationRegistry, no `Symbol.dispose`, and no `try/finally` guarantee. The code in `subscription_state.ts` creates `ResultSetWrite` via `resultset.write()` and calls `.done()` manually in at least 8 places (lines 328/368, 464/466, 669/679, 745/759). Every single one of these is an unguarded `write()`/`done()` pair with no `try/finally` -- if the code between `write()` and `done()` throws, the broadcast is silently lost.

---

### Scenario 3: The `using` Escape Hatch (Disposed Object Leak)

**Severity**: MODERATE

**Adversarial code (TypeScript)**:
```typescript
let leaked: ReactorSubscription;
{
    using sub = reactor.subscribe();
    leaked = sub; // escape the scoped variable
}
// sub[Symbol.dispose]() has been called, inner.disposed = true
// but leaked still holds a reference

leaked.subscribe((update) => {
    console.log('Ghost listener:', update);
});
// Does this throw? Let's trace...
```

**Rust behavior**: Impossible. `sub` is moved into the scope, and after `}` the value is dropped. You cannot assign a moved value to an outer variable -- the borrow checker prevents it.

**TS behavior (traced through real code)**:
1. `reactor.subscribe()` returns a `ReactorSubscription` (reactor/index.ts:183-194)
2. After the `using` block exits, `ReactorSubscription[Symbol.dispose]()` calls `this.dispose()` which calls `this.inner.dispose()` (subscription.ts:152-154)
3. `ReactorSubInner.dispose()` sets `this.disposed = true` and calls `unsubscribeFn(this.subscriptionId)` (subscription.ts:51-56)
4. Now `leaked.subscribe(...)` is called. This calls `this.inner.broadcast.reference().listen(...)` (subscription.ts:115)
5. **No `assertNotDisposed()` check exists** in `ReactorSubscription.subscribe()` or `.listen()`
6. The broadcast still exists (it was not destroyed), so the listener is registered
7. But the subscription was removed from `reactor.subscriptions` Map, so no more updates will be sent through this broadcast
8. The listener is a zombie -- it won't crash, but it will never fire and the `ListenerGuard` will leak

**Does the spec address this?** YES -- Section 12 "The `using` Escape Hatch" describes exactly this pattern and says: "`assertNotDisposed()` guards on public methods catch this at runtime, converting silent failures into loud errors. This is why every Disposable type must call `assertNotDisposed()` at the top of public methods."

**Is the mitigation sufficient?** The spec prescription is correct, but `ReactorSubscription` **does not extend `Disposable`** (subscription.ts:77). It has a manual `dispose()` method but no `assertNotDisposed()` calls in `subscribe()`, `listen()`, `id()`, or `broadcastId()`. The spec's Section 4 checklist says "Public methods call `assertNotDisposed()` at the top" but the class doesn't follow this.

---

### Scenario 4: Transaction Double-Create Race

**Severity**: MODERATE

**Adversarial code (TypeScript)**:
```typescript
const trx = new Transaction(ctx);

// Two concurrent creates for the same "logical" entity
const [a, b] = await Promise.all([
    trx.create(Video, { title: 'A' }),
    trx.create(Video, { title: 'B' }),
]);
// Both succeed -- each creates a separate entity. Not really an attack,
// but what about get()?

const [c, d] = await Promise.all([
    trx.get(Video, existingId),
    trx.get(Video, existingId),
]);
// Race condition in getTrxEntity + addEntity
```

**Rust behavior**: `Transaction` methods take `&self` (shared reference). `AppendOnlyVec` is lock-free for concurrent appends. `get_trx_entity` iterates the vec safely. The Rust code has the same race check pattern (re-examine after await).

**TS behavior (traced through real code)**:
1. Both `trx.get()` calls execute concurrently
2. First call: `getTrxEntity(id)` returns null, proceeds to `await this.dyncontext.getEntity(...)` (transaction.ts:150)
3. Second call: also gets null from `getTrxEntity(id)`, also awaits `getEntity`
4. First call returns, does race check at line 153: `getTrxEntity` still returns null (second call hasn't finished yet), calls `addEntity` with snapshot
5. Second call returns, race check at line 153: NOW `getTrxEntity` returns the entity added by the first call
6. Second call returns the existing fork -- **crisis averted by the race check**

**Does the spec address this?** Not explicitly. The race check is in the code but the spec doesn't discuss concurrent async operations within a single transaction.

**Is the mitigation sufficient?** The race check works correctly for this case. However, `Transaction` does not check `this.alive.value` at the top of `create()`, `get()`, or `edit()` (transaction.ts:112-187). The spec's Section 14 explicitly requires these checks. A committed transaction's methods will proceed without error until they hit a downstream failure.

---

### Scenario 5: RefCell Async Callback (The Async Borrow Escape)

**Severity**: CRITICAL

**Adversarial code (TypeScript)**:
```typescript
const cell = new RefCell(state, {
    onMutRelease: () => broadcast.send(),
    label: 'ResultSet',
});

cell.withMut(async (state) => {
    state.add(entity1);
    await fetch('/api/validate'); // <-- AWAIT inside withMut!
    state.add(entity2);           // state is still accessible but borrow was "released"
});
// onMutRelease fires at the first await (when the async function yields)
// broadcast fires BEFORE entity2 is added
// THEN entity2 is added silently with no broadcast
```

**Rust behavior**: `RefCell::borrow_mut()` returns a `RefMut<T>` with a lifetime tied to the cell. The Rust `RefCell` is not `Send`, so it cannot be held across `.await` points in async contexts. The compiler enforces this.

**TS behavior (traced through real code)**:
1. `withMut` at disposable.ts:220 sets `#state = { kind: 'mut_borrowed' }`
2. Calls `fn(this.#value)` -- `fn` is an async function that returns a Promise
3. The `try` block returns the Promise immediately (async functions return on first await)
4. The `finally` block runs: `#state = { kind: 'not_borrowed' }`, `onMutRelease()` fires
5. The broadcast fires with only `entity1` added
6. The await resolves, `state.add(entity2)` runs -- but the borrow tracking says "not borrowed"
7. Another `withMut` could now execute concurrently, violating single-writer semantics
8. entity2 is added with no broadcast notification

**Does the spec address this?** YES -- Section 6 constraints: "No async inside `withMut()`: The callback MUST be synchronous. If an async function is passed, the `finally` block runs at the first `await`, releasing the borrow while the callback is still running."

**Is the mitigation sufficient?** The spec documents the limitation but the code **does not enforce it**. `withMut<R>(fn: (value: T) => R)` accepts any function. It could detect async callbacks by checking if the return value is a Promise (i.e., `if (result instanceof Promise) throw new Error(...)` in the `try` block before the `finally`). Without runtime enforcement, this is a documentation-only guard against a correctness-critical failure.

---

### Scenario 6: DisposeGuard Host Mismatch

**Severity**: LOW

**Adversarial code (TypeScript)**:
```typescript
class Evil {
    guard = new DisposeGuard(this, 'Evil');

    dispose(): void {
        // Pass a DIFFERENT object to markDisposed
        this.guard.markDisposed({}); // different object than `this`
    }
}

const evil = new Evil();
evil.dispose();
// The FinalizationRegistry still tracks `this` (the original host)
// but we unregistered a different object
// When `evil` is GC'd, the leak warning fires even though dispose() was called
```

**Rust behavior**: N/A -- this pattern doesn't exist in Rust (Drop is automatic).

**TS behavior (traced through real code)**:
1. `DisposeGuard` constructor registers `host` with `leakRegistry.register(host, info, host)` (disposable.ts:144)
2. The unregister token is `host` (the third arg)
3. `markDisposed({})` calls `leakRegistry.unregister({})` -- unregisters with a **different** object
4. The original `host` is still registered -- GC will fire the false-positive leak warning
5. `#disposed` is set to `true`, so `assertNotDisposed()` works correctly
6. The only symptom is a spurious console.error

**Does the spec address this?** Section 5 shows the pattern with `this.#guard.markDisposed(this)` passing `this` correctly, but doesn't warn about the mismatch case.

**Is the mitigation sufficient?** This is a minor API footgun. The `markDisposed` method could be changed to not take a `host` parameter at all -- instead, capture the host in a private field during construction and use it automatically. Low severity since the worst case is a false diagnostic.

---

### Scenario 7: Broadcast Listener Throws (Cascading Failure)

**Severity**: MODERATE

**Adversarial code (TypeScript)**:
```typescript
const broadcast = new Broadcast<void>();
const ref = broadcast.reference();

const guard1 = ref.listen({ type: 'NotifyOnly', callback: () => {
    throw new Error('Listener 1 explodes');
}});
const guard2 = ref.listen({ type: 'NotifyOnly', callback: () => {
    console.log('Listener 2 should fire but might not');
}});

broadcast.send(); // Listener 1 throws -- does Listener 2 fire?
```

**Rust behavior**: In Rust, the broadcast iterates listeners and calls each one. If a listener panics, `catch_unwind` or the default panic handler deals with it. Typically each listener runs independently.

**TS behavior (traced through real code)**:
1. `broadcast.send()` at broadcast.ts:136-151 clones listeners to an array, then iterates with a `for...of` loop
2. Listener 1's callback throws
3. The `for` loop is NOT wrapped in try/catch
4. **Listener 2 never fires** -- the exception propagates up
5. Any code after the `broadcast.send()` call also does not execute

**Does the spec address this?** No. The spec discusses broadcast semantics only in terms of when they fire (Section 3b, Section 6), not error handling within listener callbacks.

**Is the mitigation sufficient?** No mitigation exists. This is a moderate issue because a single misbehaving listener can prevent all downstream listeners from receiving notifications. The `send()` method should wrap each listener call in a try/catch and log errors without preventing other listeners from firing.

---

### Scenario 8: fillGapsAndNotify Fire-and-Forget Race

**Severity**: MODERATE

**Adversarial code (TypeScript)**:
```typescript
// This isn't adversarial user code -- it's an inherent race in the implementation.
// subscription_state.ts:532 calls this.fillGapsAndNotify() without await:
//   this.fillGapsAndNotify(updateItems, gapsToFill);
// This is fire-and-forget. Meanwhile, the notifyLock in Reactor is released.
// Another notifyChange() can now execute, potentially mutating the same
// resultsets that fillGapsAndNotify is about to write to.
```

**Rust behavior**: In Rust, `fill_gaps_and_notify` is spawned as a separate task with `crate::task::spawn`. The `notify_lock` (tokio::sync::Mutex) is released before the spawned task runs, but the spawned task acquires its own locks on the state it needs. The Rust WatcherSet is protected by `std::sync::Mutex`.

**TS behavior (traced through real code)**:
1. `evaluateChanges` at subscription_state.ts:429-538 is called within the `notifyLock` in reactor/index.ts:434
2. At line 532: `this.fillGapsAndNotify(updateItems, gapsToFill)` -- called WITHOUT `await`
3. The promise is returned but not awaited -- it's fire-and-forget
4. `evaluateChanges` returns `watcherChanges` to the caller
5. The `notifyLock` is released in reactor/index.ts:477
6. Now another `notifyChange()` can proceed while `fillGapsAndNotify` is still running
7. `fillGapsAndNotify` calls `resultset.write()` / `.add()` / `.done()` (subscription_state.ts:669-679)
8. Concurrently, the new `notifyChange` may also call `resultset.write()` via `evaluateChanges`
9. Both are writing to the same `ResultSetState` object with no synchronization

**Does the spec address this?** YES -- Section 13 explicitly identifies this: "WatcherSet mutation from gap-fill... fire-and-forget `fillGapsAndNotify()` mutates WatcherSet outside the notifyLock. Must either await gap fill within `evaluateChanges` or add WatcherSet-level PromiseMutex." It is listed as **MISSING** protection.

**Is the mitigation sufficient?** The spec correctly identifies the problem and notes it as a gap. The code has no fix yet. This is a real race condition that can cause corrupted result sets.

---

### Scenario 9: WeakEntitySet Phantom Entity Resurrection

**Severity**: LOW

**Adversarial code (TypeScript)**:
```typescript
// Create an entity, let it go out of scope
let entityId: EntityId;
{
    const entity = entitySet.create(someCollection);
    entityId = entity.id();
    // entity goes out of scope
}
// GC hasn't run yet -- WeakRef still returns the entity
const zombie = entitySet.get(entityId); // succeeds!

// Now withState is called with a NEW state for the same ID
const [changed, entity] = entitySet.withState(entityId, collection, newState);
// withState checks: const existing = this.get(id) -- returns the zombie!
// existing.applyState(state) -- applies state to the about-to-be-GC'd entity
// changed = true, entity = zombie (same object)
// Caller gets back a "live" entity that has no strong references anywhere
// Next GC pass: entity is collected, WeakRef goes dead
// FinalizationRegistry cleans up the map entry
// The caller's reference keeps it alive... or does it?
```

**Rust behavior**: `Weak::upgrade()` returns `None` immediately when the last `Arc` is dropped. The entity set would create a new entity from state.

**TS behavior (traced through real code)**:
1. `WeakEntitySet.get()` at entity.ts:494-504 calls `ref_.deref()` which may return the object even after all strong refs are gone (per ECMAScript spec for WeakRef within same microtask)
2. `withState()` at entity.ts:542-554 gets the "zombie" entity and applies state to it
3. The caller gets back a reference to this entity, which keeps it alive via the returned reference
4. Actually this is fine -- the caller now holds a strong reference, so GC won't collect it

**Does the spec address this?** YES -- Section 7 "Timing difference from Rust" explains this: "JS WeakRef is 'stronger' than Rust Weak within a single synchronous execution slice."

**Is the mitigation sufficient?** Yes. The spec correctly identifies this as not-a-bug. The TS code is actually more permissive than Rust here, and the caller's returned reference keeps the entity alive. No real harm.

---

### Scenario 10: SystemManager Concurrent joinSystem Calls

**Severity**: MODERATE (data integrity)

**Adversarial code (TypeScript)**:
```typescript
// Two peer connections both try to join the system simultaneously
const state1 = makeAttestedState(clock1);
const state2 = makeAttestedState(clock2);

// Both fire concurrently (e.g., from two WebSocket message handlers)
Promise.all([
    systemManager.joinSystem(state1),
    systemManager.joinSystem(state2),
]);
```

**Rust behavior**: `joinSystem` acquires the `RwLock` on internal state, serializing the two calls. One succeeds, the other sees the existing root and either matches or resets.

**TS behavior (traced through real code)**:
1. Both calls hit `await this.waitLoaded()` (system.ts:303) -- both proceed after loading
2. Call 1: `this.root()` returns null (line 313), proceeds to line 342
3. Call 2: also gets null from `this.root()`, also proceeds to line 342
4. Both calls proceed to `await storage.setState(state)` (line 347)
5. Call 1 completes: sets `_root = state1`, sets `systemReady = true`, resolves deferred
6. Call 2 completes: **overwrites** `_root = state2`, sets `systemReady = true`
7. The deferred was already resolved, so the second `resolve()` is a no-op
8. System is now in an inconsistent state -- `_root` is `state2` but storage might have `state1`'s data

**Does the spec address this?** YES -- Section 13 "Required PromiseMutex Coverage" table lists: "SystemManager lifecycle ops... `joinSystem()`, `create()`, `hardReset()` need serialization. Two concurrent `joinSystem` calls can race through lifecycle operations." It is listed as **MISSING**.

**Is the mitigation sufficient?** The spec correctly identifies the gap but the code has no fix. Real-world impact: ephemeral nodes receiving system state from multiple peers simultaneously could corrupt their root.

---

### Scenario 11: Stale Read from Committed Transaction Fork

**Severity**: LOW

**Adversarial code (TypeScript)**:
```typescript
const trx = await ctx.begin();
const record = await trx.create(Video, { title: 'original' });
const mutable = record.inner;

await trx.commit();
// Commit applies event to upstream (canonical) entity
// The forked entity's backends are now STALE -- they have the old state

// Reading from the mutable gives the FORK's state, not the canonical state
const view = mutable.read(); // calls GeneratedViewClass.fromEntity(this._entity)
const title = view.title();  // reads from the forked entity's backend
// title === 'original' -- this is the fork's value, which happens to be correct
// BUT: if another transaction committed changes to this entity between our
// create and our commit, the canonical entity would have merged state that
// the fork doesn't have
```

**Rust behavior**: After `commit()` consumes the transaction, `mutable` is no longer accessible (moved value). No stale reads possible.

**TS behavior (traced through real code)**:
1. After commit, `mutable._entity` is the forked entity (a snapshot)
2. The fork's backends were created at snapshot time and only have local changes
3. `mutable.read()` at define-model.ts:392 creates a View from the fork entity
4. The View reads from fork backends, not canonical backends
5. This gives stale data if concurrent changes occurred

**Does the spec address this?** YES -- Section 14: "Reading from a committed transaction's fork entity returns stale data... Property value read methods SHOULD check `isWritable()` and warn or throw to prevent reading stale forked state after commit."

**Is the mitigation sufficient?** The spec says "SHOULD" (not "MUST") for read checks. The code does not implement this -- `getPropertyValue()` and View getters have no alive/writable checks. Severity is LOW because this is a read-only issue and the stale data is from the user's own transaction.

---

### Scenario 12: Disposable.onDispose() Throws (Unregistered but Not Disposed)

**Severity**: LOW

**Adversarial code (TypeScript)**:
```typescript
class BadSubscription extends Disposable {
    constructor() { super('BadSubscription'); }
    protected onDispose(): void {
        throw new Error('cleanup failed!');
    }
}

const sub = new BadSubscription();
try {
    sub.dispose();
} catch (e) {
    // caught
}

// What state is `sub` in now?
console.log(sub.isDisposed); // true -- #disposed was set BEFORE onDispose()
// FinalizationRegistry was ALSO unregistered BEFORE onDispose() threw
// So: #disposed = true, FR unregistered, but actual cleanup never happened
// Calling dispose() again is a no-op (idempotent check at line 85)
// The resource is permanently leaked with no diagnostic
```

**Rust behavior**: If `Drop::drop()` panics, it's a double-panic abort. Rust strongly discourages panicking in Drop. The resources would be cleaned up by the OS process terminating.

**TS behavior (traced through real code)**:
1. `dispose()` at disposable.ts:84: checks `#disposed` (false), sets `#disposed = true`
2. Line 87: `leakRegistry.unregister(this)` -- FR unregistered
3. Line 88: `this.onDispose()` -- throws
4. Exception propagates to caller
5. `#disposed` is `true`, FR is unregistered, but cleanup never completed
6. Calling `dispose()` again returns immediately at line 85

**Does the spec address this?** Not explicitly. Section 4 says `onDispose()` is "called exactly once, inside dispose()" but doesn't address what happens if it throws.

**Is the mitigation sufficient?** The ordering in `dispose()` should be: (1) call `onDispose()`, (2) only if it succeeds, set `#disposed = true` and unregister from FR. Or: wrap `onDispose()` in try/catch and always unregister but log the error. Current ordering means a throwing `onDispose()` permanently wedges the object.

---

## Summary of Attack Results

| # | Scenario | Severity | Attack Succeeds? | Spec Addresses? | Code Enforces? |
|---|----------|----------|-------------------|-----------------|----------------|
| 1 | MutableBorrow zombie mutator | CRITICAL | YES | YES (S14) | NO -- raw handles returned |
| 2 | ResultSetWrite forgotten done() | CRITICAL | YES | YES (S3b) | NO -- no RefCell wrapping |
| 3 | `using` escape hatch on ReactorSubscription | MODERATE | Partial | YES (S12) | NO -- no assertNotDisposed |
| 4 | Transaction double-get race | MODERATE | NO (race check works) | NO | YES (by code) |
| 5 | Async callback in RefCell.withMut | CRITICAL | YES | YES (S6) | NO -- no runtime check |
| 6 | DisposeGuard host mismatch | LOW | Spurious warning only | Partially | NO |
| 7 | Broadcast listener throws | MODERATE | YES | NO | NO |
| 8 | fillGapsAndNotify race | MODERATE | YES | YES (S13) | NO -- listed as MISSING |
| 9 | WeakRef phantom resurrection | LOW | Not harmful | YES (S7) | N/A |
| 10 | SystemManager concurrent join | MODERATE | YES | YES (S13) | NO -- listed as MISSING |
| 11 | Stale read from committed fork | LOW | YES | YES (S14) | NO |
| 12 | onDispose() throws | LOW | YES | NO | NO |

### Attacks that succeed against the current spec+code:

**3 CRITICAL attacks succeed**: Scenarios 1, 2, and 5. All three are identified by the spec but not enforced by the code.

**4 MODERATE attacks succeed**: Scenarios 3, 7, 8, and 10. Scenarios 8 and 10 are explicitly listed as "MISSING" in the spec. Scenario 7 (broadcast listener throws) is not addressed by the spec at all.

**The spec itself is sound.** The prescriptions in Sections 3b, 6, 12, 13, and 14 would prevent all CRITICAL attacks if implemented. The primary gap is implementation fidelity, not spec design.

### Recommendations for spec additions:

1. **Add a section on error handling in Broadcast.send()** -- listener callbacks that throw should not prevent other listeners from firing.
2. **Add a note about Disposable.onDispose() error handling** -- specify the ordering of `#disposed`/unregister/onDispose and behavior when onDispose throws.
3. **Strengthen Section 14 "SHOULD" to "MUST"** for read checks on committed fork entities.
4. **Add Transaction alive checks** to the Section 14 table for `create()`, `get()`, `edit()` (currently documented but not in the code).
5. **Consider runtime enforcement for async-in-withMut** -- check `result instanceof Promise` and throw.
