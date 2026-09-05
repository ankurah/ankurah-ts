// MIRRORS: ankurah/core/src/util/mod.rs
//
// Two things, because mod.rs holds two: the module index, and five
// `#[macro_export] macro_rules!` whose bodies are `tracing` calls dressed in ANSI
// escapes. The macros are what makes this module `[[provided]]` — the engine does
// not expand macro_rules — so they are written out here as functions.
//
// `#[macro_export]` puts a macro at the CRATE root in Rust, not under
// `crate::util`, so `ankurah_core::action_info!` is its real name. It reaches the
// package root here too, by way of core's own index writing `export * from
// './util'` — so the two spellings agree without the file having to move.
//
// What a caller passes: the port renders a format string at the call site and
// hands the result over as one string, the way `tracing.*` already receives its
// message, so the variadic `$($arg),+` tail arrives as a single rendered
// `context` argument rather than a format string and its operands.

export * from './cast.ts';
export * from './expand_states.ts';
export * from './iterable.ts';
export * from './ivec.ts';
export * from './ready_chunks.ts';
export * from './safemap.ts';
export * from './safeset.ts';

import { tracing } from '@ankurah/base';

/** Bold blue — the thing that performed the action. */
const THING = '\x1b[1;34m';
/** Green — the action's name. */
const ACTION = '\x1b[32m';
/** Dimmed — the additional context. */
const CONTEXT = '\x1b[2m';
/** Bold yellow — a notice, which stands alone rather than naming an actor. */
const NOTICE = '\x1b[1;33m';
const OFF = '\x1b[0m';

/**
 * The line every action macro writes: the actor in bold blue, an arrow, the
 * action in green, and — when the caller gave one — the rest dimmed after it.
 */
function actionLine(thing: unknown, action: string, context: string | undefined): string {
  const head = `${THING}${thing}${OFF} → ${ACTION}${action}${OFF}`;
  return context === undefined ? head : `${head} ${CONTEXT}${context}${OFF}`;
}

/** Rust: `action_info!` */
export function actionInfo(thing: unknown, action: string, context?: string): void {
  tracing.info(actionLine(thing, action, context));
}

/** Rust: `action_debug!` */
export function actionDebug(thing: unknown, action: string, context?: string): void {
  tracing.debug(actionLine(thing, action, context));
}

/** Rust: `action_warn!` */
export function actionWarn(thing: unknown, action: string, context?: string): void {
  tracing.warn(actionLine(thing, action, context));
}

/** Rust: `action_error!` */
export function actionError(thing: unknown, action: string, context?: string): void {
  tracing.error(actionLine(thing, action, context));
}

/** Rust: `notice_info!` — one message, bold yellow, so it stands out in the log. */
export function noticeInfo(message: string): void {
  tracing.info(`${NOTICE}${message}${OFF}`);
}
