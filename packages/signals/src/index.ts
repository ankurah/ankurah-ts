// MIRRORS: ankurah/signals/src/lib.rs

// Rust: pub mod broadcast;
// Rust: mod context;
// Rust: pub mod observer;
// Rust: pub mod porcelain;
// Rust: pub mod signal;
// Rust: mod value;

// Rust: pub use broadcast::BroadcastId;
export {
  BroadcastId,
  Broadcast,
  BroadcastRef,
  ListenerGuard as BroadcastListenerGuard,
  type BroadcastListener,
  type TListenerGuard,
} from './broadcast.ts';

// Rust: pub use context::*;
export { CurrentObserver } from './context.ts';

// Rust: pub use observer::*;
export type { Observer } from './observer/index.ts';
export { CallbackObserver } from './observer/index.ts';

// Rust: pub use porcelain::*;
export { type Subscribe, SubscriptionGuard } from './porcelain/index.ts';
export { type Wait, type WaitResultValue, waitValue, waitFor } from './porcelain/wait.ts';

// Rust: pub use signal::*;
export {
  Calculated,
  Map,
  Memo,
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

// Rust: mod value; (private module, but TS exports for use by other packages)
// Divergence: Rust keeps value module private; TS exports it for cross-package use [E8]
export { ValueCell, ReadValueCell } from './value.ts';

// Feature-gated modules — all skipped:
// reactive_graph [E14], react [E9], react_native [E15], jsvalue [E9]
