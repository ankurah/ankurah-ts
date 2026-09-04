// TS-ONLY: Stand-in for the `tracing` crate's five level macros.
//
// The port needs somewhere for `tracing::info!("connected to {peer}")` to go.
// In Rust the macro builds an *event* — a level, a target, a set of typed
// fields — and hands it to whatever subscriber is installed, which decides how
// to render it. None of that survives the crossing: the transpiler renders the
// format string at the call site and emits one already-rendered string, so what
// arrives here is a level and a line of text. A `tracing::warn!` that carries
// structured fields instead of a format string is refused by the transpiler
// rather than losing its fields quietly here.
//
// So this file is a level, a string, and one place to send them. The sink is
// replaceable because a host decides where its log goes — a test captures it, a
// React Native app forwards it, a browser prints it.
//
// Nothing here is leak-tracked and nothing here owns anything: a rendered
// message is a string, and a string has no drop glue.

/** The five levels the `tracing` macros name, in the order tracing orders them. */
export type Level = 'trace' | 'debug' | 'info' | 'warn' | 'error';

/** Where a rendered event goes. */
export type Sink = (level: Level, message: string) => void;

/**
 * The default sink: one console call per event, on the method that carries the
 * level.
 *
 * DELIBERATE DIFFERENCE: `trace` goes to `console.debug`, not `console.trace`.
 * `console.trace` prints a stack trace, which `tracing::trace!` does not — the
 * level name is the only thing the two spellings share.
 *
 * DELIBERATE DIFFERENCE: an event reaches the console with no subscriber
 * installed. In Rust an event with no subscriber is dropped, and a binary that
 * never calls `tracing_subscriber::fmt::init()` prints nothing. Silence is the
 * worse default in a port whose whole purpose is to be watched while it runs,
 * so the console is the sink until a host replaces it.
 */
export const consoleSink: Sink = (level: Level, message: string): void => {
  switch (level) {
    case 'trace': console.debug(message); return;
    case 'debug': console.debug(message); return;
    case 'info': console.info(message); return;
    case 'warn': console.warn(message); return;
    case 'error': console.error(message); return;
  }
};

let sink: Sink = consoleSink;

/**
 * Replace where the five levels write. A host that has its own log wires it up
 * here; a test replaces the sink, asserts what the code under it recorded, and
 * puts `consoleSink` back.
 */
export function setSink(next: Sink): void {
  sink = next;
}

/** `tracing::trace!` */
export function trace(message: string): void { sink('trace', message); }

/** `tracing::debug!` */
export function debug(message: string): void { sink('debug', message); }

/** `tracing::info!` */
export function info(message: string): void { sink('info', message); }

/** `tracing::warn!` */
export function warn(message: string): void { sink('warn', message); }

/** `tracing::error!` */
export function error(message: string): void { sink('error', message); }
