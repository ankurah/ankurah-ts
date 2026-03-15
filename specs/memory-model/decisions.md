# Memory Model Decisions

**Ankurah-specific architectural decisions.** These are overarching decisions that aren't derivable from the rules in [overview.md](overview.md) alone. Type-specific severity classifications and disposal patterns should be annotated in the source code, not here.

---

## Async Serialization

**Reactor notification pipeline**: Uses `PromiseMutex` (mirrors Rust's `tokio::sync::Mutex<()> notify_lock`).

**WatcherSet gap-fill**: Fire-and-forget `fillGapsAndNotify()` mutates WatcherSet outside notify lock. Needs awaiting or its own PromiseMutex.

**LiveQuery activation**: Concurrent activations can race (same bug in Rust, issue #146). Needs serialization.

**SystemManager lifecycle**: Low risk, initialization-time only. Consider PromiseMutex if connector porting surfaces races.

---

## Known Gotchas

**NodeLikeAdapter**: Adapters bridging reactor interfaces to Node must hold strong references. WeakRef-only adapters get GC'd while the subscription is still active.

**Transaction alive gap**: `commit()` and `rollback()` set `alive = false` eagerly to close the gap between unreachability and GC.

**`using` escape hatch**: `let bar; { using foo = ..; bar = foo; }` leaks a disposed reference. `assertNotDisposed()` converts this from silent failure to loud error.

**Observer stack balance**: Reactive tracking context push/pop must use try/finally.
