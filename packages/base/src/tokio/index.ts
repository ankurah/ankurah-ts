// TS-ONLY: the `tokio` crate, as far as ankurah uses it.
//
// The transpiler will not translate tokio — it is a Rust runtime, not part of
// the family of code the port carries over — so it maps the crate onto these
// stand-ins by identity. That mapping is a path rewrite and nothing more, which
// is why this file mirrors tokio's module tree: `tokio::sync::mpsc::channel`
// becomes `tokio.sync.mpsc.channel`, `tokio::spawn` becomes `tokio.spawn`, and
// `tokio::select!` becomes `tokio.select`.
//
// Each name is also exported flat from the package root, for the far more
// common `use tokio::sync::Notify;` form. The two spellings are the same
// objects.

export * as sync from './sync.ts';
export * as task from './task.ts';
export * as time from './time.ts';

export { spawn, spawn_local, yield_now } from './task.ts';
export { select } from './select.ts';
export type { SelectBranch, SelectOutcome } from './select.ts';
