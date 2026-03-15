// MIRRORS: ankurah/signals/src/porcelain.rs
// Exception E12: file-with-submodules pattern

// Rust: pub use subscribe::{DynSubscribe, GetAndDynSubscribe, Subscribe, SubscriptionGuard};
// Divergence: DynSubscribe is a blanket impl over Subscribe in Rust (Box<dyn Fn> vs generic F).
// In TS there's no distinction — Subscribe covers both cases. Not re-exported separately [E8].
// Divergence: GetAndDynSubscribe is a trait alias (Get + Peek + DynSubscribe) in Rust.
// In TS, use intersection type `Get<T> & Peek<T> & Subscribe<T>` at call sites [E8].
export { type Subscribe, SubscriptionGuard } from './subscribe.ts';

// Rust: pub use wait::Wait;
export { type Wait, type WaitResultValue, waitValue, waitFor } from './wait.ts';
