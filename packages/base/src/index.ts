// TS-ONLY: Rust ownership primitives for the ankurah port (see E11)

export { AkObject, dropOwned, type Slot } from './object.ts';
export { Struct } from './struct.ts';
export { Enum } from './enum.ts';
export { Result } from './result.ts';
export { OwnedClosure } from './closure.ts';
export { AnyhowError } from './anyhow.ts';
export * as anyhow from './anyhow.ts';
export { Drop, DropGuard } from './std/drop.ts';
export {
  disposeSymbol,
  OwnershipFatal,
  setOnFatal,
  setOnDiagnostic,
  setCaptureStacks,
  isPoisoned,
  clearFatalLatch,
  type FatalHandler,
  type DiagnosticHandler,
} from './drop_registry.ts';
export { Arc, Weak } from './std/arc.ts';
export { Mutex, MutexGuard } from './std/sync.ts';
export { RwLock, RwLockReadGuard, RwLockWriteGuard } from './std/rwlock.ts';
export { Guard, ReadGuard, WriteGuard } from './std/guard.ts';
export { AsyncMutex, AsyncMutexGuard } from './std/async_mutex.ts';
export { RefCell, Ref, RefMut } from './std/cell.ts';
export { Borrow, BorrowMut } from './std/borrow.ts';
export { ThreadLocal } from './std/thread_local.ts';

// ── tokio ───────────────────────────────────────────────────────────────
//
// Two spellings of one set of objects. `tokio` mirrors the crate's module tree,
// for a path-qualified `tokio::sync::mpsc::channel(1024)`; the flat names below
// are for `use tokio::sync::Notify;`, which is how the corpus almost always
// writes it. tokio's Mutex and RwLock keep the Async- prefix out here, because
// std's Mutex and RwLock are ported too and a bare name would be ambiguous;
// under `tokio.sync` they are spelled Mutex and RwLock as tokio spells them.
export * as tokio from './tokio/index.ts';
export { Notify, Notified } from './tokio/notify.ts';
export { AsyncRwLock, AsyncRwLockReadGuard, AsyncRwLockWriteGuard } from './tokio/rwlock.ts';
export { TryLockError } from './tokio/lock_error.ts';
export { JoinHandle, JoinError, spawn, spawn_local, yield_now, type Spawnable } from './tokio/task.ts';
export { select, type SelectBranch, type SelectOutcome } from './tokio/select.ts';
export { sleep, timeout, Elapsed } from './tokio/time.ts';
export * as oneshot from './tokio/oneshot.ts';
export * as mpsc from './tokio/mpsc.ts';
// Bare, for `use tokio::sync::mpsc::Sender;`. oneshot's two ends keep the
// namespace form — `oneshot.Sender` — because these four have the plain names.
export { Sender, UnboundedSender, Receiver, UnboundedReceiver } from './tokio/mpsc.ts';
