// TS-ONLY: Symbol.dispose polyfill, leak detection, and the one place this
// package reports a fatal ownership bug.
//
// Every fatal is thrown as an OwnershipFatal, which emitted code must never
// swallow: a `catch` that handles a Rust error type rethrows it unconditionally,
// because the runtime has already found a bug that Rust would not have compiled
// and nothing after it can be trusted.

export const disposeSymbol: typeof Symbol.dispose =
  (Symbol.dispose ?? Symbol.for('Symbol.dispose')) as typeof Symbol.dispose;

// ── Fatal reporting ──────────────────────────────────────────────────────

/**
 * What the runtime does when it finds an ownership bug. The default throws,
 * which unwinds emitted code the way a Rust panic aborts. A host that has to
 * stop differently — killing a worker, failing one request — replaces it.
 */
export type FatalHandler = (message: string) => void;

/**
 * The error every fatal is thrown as. Emitted catch blocks test for it and
 * rethrow: an ownership bug is not a Rust error value and must not be handled
 * as one.
 */
export class OwnershipFatal extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'OwnershipFatal';
  }
}

let onFatal: FatalHandler = (message) => { throw new OwnershipFatal(message); };
let poisoned = false;

export function setOnFatal(handler: FatalHandler): void {
  onFatal = handler;
}

/**
 * Stop the program on an ownership bug — a state the Rust compiler would have
 * refused to build, so nothing downstream can be trusted to mean what its author
 * wrote. There is deliberately no softer path: every fatal comes through here,
 * so there is one severity to reason about.
 *
 * Reporting one also poisons the runtime, because a host can swallow a throw —
 * an uncaughtException handler, a browser page that keeps painting — and the
 * program would then run on over corrupted ownership. Every liveness check reads
 * the latch first, so the next one refuses instead of continuing.
 */
export function fatal(message: string): never {
  poisoned = true;
  onFatal(message);
  // Reached only when a host's handler returns instead of terminating.
  throw new OwnershipFatal(message);
}

/** True once any fatal has been reported. */
export function isPoisoned(): boolean {
  return poisoned;
}

export function assertNotPoisoned(): void {
  if (poisoned) {
    throw new OwnershipFatal(
      'BUG: the ownership runtime already reported a fatal bug and something\n' +
      'swallowed it. Every value is suspect from that point on, so this\n' +
      'operation is refused rather than run over corrupted ownership.',
    );
  }
}

/**
 * Clear the poison latch. Exists for tests that provoke a fatal on purpose and
 * then go on asserting — emitted code must never call it.
 */
export function clearFatalLatch(): void {
  poisoned = false;
}

// ── The conditions Rust rejects at compile time ──────────────────────────

export function fatalDoubleDrop(label: string): never {
  fatal(
    `BUG: ${label} was dropped twice.\n` +
    `Rust drops a value exactly once, so a second drop means two owners hold one\n` +
    `value, or the same drop was emitted twice.`,
  );
}

export function fatalUseAfterDrop(label: string): never {
  fatal(
    `BUG: ${label} was used after being dropped.\n` +
    `Its owner released it, and Rust's lifetimes would have rejected this read.`,
  );
}

export function fatalUseAfterMove(label: string): never {
  fatal(
    `BUG: ${label} was used after being moved.\n` +
    `A method taking self already handed its payload to somebody else, so Rust\n` +
    `would reject this use at compile time.`,
  );
}

export function fatalOutstandingGuard(container: string, guard: string): never {
  fatal(
    `BUG: ${container} was dropped while a ${guard} is still outstanding.\n` +
    `Rust's borrow checker makes this impossible, so the emitted drop scope is wrong.`,
  );
}

export function fatalSelfAssignment(label: string): never {
  fatal(
    `BUG: ${label} was assigned the value it already holds.\n` +
    `\`*guard = *guard\` does not compile for a non-Copy type, so this is one\n` +
    `value with two owners — and the assignment would drop what it stores.`,
  );
}

export function fatalNonExhaustiveMatch(label: string, variant: string): never {
  fatal(
    `BUG: match on ${label} has no arm for '${variant}'.\n` +
    `A non-exhaustive match does not compile in Rust, so an arm was dropped on\n` +
    `the way out of the emitter.`,
  );
}

// ── Leak detection ───────────────────────────────────────────────────────

export interface LeakInfo {
  label: string;
  creationStack: string;
}

/** The slice of FinalizationRegistry this package uses. */
interface LeakRegistry {
  register(target: object, info: LeakInfo, token: object): void;
  unregister(token: object): void;
}

function reportLeak(info: LeakInfo): void {
  const message =
    `BUG: ${info.label} was garbage collected without being dropped.\n` +
    `Something owned it and never called drop().\n` +
    `Allocated at:\n${info.creationStack}`;
  // A FinalizationRegistry callback has no caller to throw to, so this one
  // fatal is deferred to a microtask. Everything else throws where it happens.
  queueMicrotask(() => fatal(message));
}

const finalizationRegistryAvailable = typeof FinalizationRegistry === 'function';

export const leakRegistry: LeakRegistry = finalizationRegistryAvailable
  ? new FinalizationRegistry<LeakInfo>(reportLeak)
  : { register() {}, unregister() {} };

if (!finalizationRegistryAvailable) {
  // Hermes support is unverified, and the port has to load there regardless.
  // Say so once and loudly: every other ownership check still works, but a value
  // that is simply forgotten will now go unreported.
  console.warn(
    'ankurah: this host has no FinalizationRegistry, so leak detection is OFF.\n' +
    'Drop errors are still reported; values that are never dropped are not.',
  );
}

// ── Allocation stacks ────────────────────────────────────────────────────
//
// A leak report is worth little without the site that allocated the value, but
// capturing a stack costs about a microsecond per construction, which is real
// money for a type built in a loop. Labels are always recorded; stacks are not.

const productionBuild =
  typeof process !== 'undefined' && process?.env?.NODE_ENV === 'production';

let captureStacks = !productionBuild;

export function setCaptureStacks(enabled: boolean): void {
  captureStacks = enabled;
}

export function creationStack(): string {
  return captureStacks
    ? new Error().stack ?? ''
    : '(stack capture is off — call setCaptureStacks(true) to record allocation sites)';
}
