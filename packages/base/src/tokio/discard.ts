// TS-ONLY: releasing a value where there is no caller to hand a failure to.
//
// Every one of these runs inside a promise reaction — a losing select branch, a
// detached task's result, a value that arrived after its deadline. A throw from
// there becomes a rejection nobody is listening to, so an ownership fatal would
// vanish. That is the one thing this file exists to prevent.

import { dropOwned } from '../object.ts';
import { diagnostic, reportAsyncFatal } from '../drop_registry.ts';

/** Release a value that arrived somewhere nobody can own it. */
export function discardValue(value: unknown): void {
  try {
    dropOwned(value);
  } catch (thrown) {
    reportFault(thrown, 'ankurah: releasing an abandoned async value failed.');
  }
}

/**
 * Report a failure with no caller to raise it to. An ownership fatal is
 * re-raised from a fresh host task, so the latch and the throw both land;
 * anything else goes to the host's diagnostic handler, which does nothing
 * unless setOnDiagnostic() has been called.
 */
export function reportFault(thrown: unknown, message: string): void {
  if (reportAsyncFatal(thrown)) return;
  diagnostic(message, thrown);
}
