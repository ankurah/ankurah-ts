// TS-ONLY: `tokio::sync`, assembled so the path a Rust `use` names is the path
// the emitted import names.
//
// `tokio::sync::Mutex` is the AsyncMutex the runtime already had — it is that
// type's whole reason for existing — and `tokio::sync::RwLock` is AsyncRwLock
// for the same reason. Both keep their distinct TS names at the package's top
// level, because `std::sync::Mutex` and `std::sync::RwLock` are also ported and
// a flat `Mutex` would be ambiguous. Under this namespace there is no ambiguity,
// so here they are spelled the way tokio spells them.

export { Notify, Notified } from './notify.ts';
export { AsyncMutex as Mutex, AsyncMutexGuard as MutexGuard } from '../std/async_mutex.ts';
export {
  AsyncRwLock as RwLock,
  AsyncRwLockReadGuard as RwLockReadGuard,
  AsyncRwLockWriteGuard as RwLockWriteGuard,
} from './rwlock.ts';
export { TryLockError } from './lock_error.ts';
export * as oneshot from './oneshot.ts';
export * as mpsc from './mpsc.ts';
