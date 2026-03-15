// MIRRORS: ankurah/signals/src/lib.rs
//
// @ankurah/signals -- Reactive signals and subscriptions.
//
// This package provides Broadcast, Signal trait, Subscribe, and ListenerGuard.
// Used by core for reactive updates and by @ankurah/react for UI bindings.
//
// Rust crate: ankurah-signals

// Rust: pub mod broadcast;
// Rust: pub use broadcast::BroadcastId;
export {
  BroadcastId,
  Broadcast,
  BroadcastRef,
  ListenerGuard as BroadcastListenerGuard,
  type BroadcastListener,
  type TListenerGuard,
} from './broadcast.ts';

// Rust: mod context; pub use context::*;
export { CurrentObserver } from './context.ts';

// Rust: pub mod observer; pub use observer::*;
export type { Observer } from './observer/index.ts';
export { CallbackObserver } from './observer/callback_observer.ts';

// Rust: pub mod signal; pub use signal::*;
export {
  Mut,
  Read,
  ListenerGuard,
  type Signal,
  type Get,
  type Peek,
  type With,
  type GetReadCell,
  type Listener,
} from './signal/index.ts';
export { Calculated } from './signal/calculated.ts';
export { Map } from './signal/map.ts';
export { Memo } from './signal/memo.ts';

// Rust: mod value; (private module, but TS exports for use by other packages)
export { ValueCell, ReadValueCell } from './value.ts';

// Rust: pub mod porcelain; pub use porcelain::*;
export { type Subscribe, SubscriptionGuard } from './porcelain/index.ts';
export { type Wait, type WaitResultValue, waitValue, waitFor } from './porcelain/wait.ts';

// Feature-gated modules — all skipped:
// reactive_graph [E14], react [E9], react_native [E15], jsvalue [E9]
