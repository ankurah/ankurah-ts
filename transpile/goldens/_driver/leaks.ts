// The ownership check every golden driver ends with: collect what the runtime
// reports while the golden runs, and fail the run with it at the end.
//
// A leak check built on FinalizationRegistry is one-sided and always will be. A
// report that fires proves a value was collected without being dropped, and that
// is a defect. Silence proves only that nothing was collected — a garbage
// collector is never obliged to collect anything. Forcing a collection and
// giving the loop a turn to deliver what it found is what makes the silence
// worth something; it does not make it certain.

import { afterAll } from 'bun:test';

import { clearFatalLatch, OwnershipFatal, setOnDiagnostic, setOnFatal } from '@ankurah/base';

const reports: string[] = [];

setOnFatal((message) => {
  reports.push(message);
});

setOnDiagnostic((message, detail) => {
  reports.push(detail === undefined ? message : `${message}\n${String(detail)}`);
});

// The leak registry reports from a microtask, because a FinalizationRegistry
// callback has no caller to throw to. That throw would land on whichever `await`
// the test happened to be sitting at and tear the test down before it could read
// the report. `hostTask` looks `queueMicrotask` up on the global rather than
// capturing it, for a harness to do exactly this: hold the throw here, so the
// message reaches the list above and the check below is what fails.
const hostQueueMicrotask = globalThis.queueMicrotask.bind(globalThis);
globalThis.queueMicrotask = (callback: () => void) => {
  hostQueueMicrotask(() => {
    try {
      callback();
    } catch (thrown) {
      // Only an ownership fatal is already recorded above. Anything else is
      // somebody's real error and belongs on the host's uncaught path.
      if (!(thrown instanceof OwnershipFatal)) throw thrown;
    }
  });
};

/** Force a collection, then let the loop deliver the reports it produced. */
async function settle(): Promise<void> {
  Bun.gc(true);
  await new Promise((resolve) => setTimeout(resolve, 0));
  Bun.gc(true);
  await new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * One report a golden EXPECTS, because the golden stands on a defect that is
 * open and named.
 *
 * A recorded report is a DEBT, not an acceptance, so it names the item that owes
 * the fix: `owes` is required, and a driver that records a leak without saying
 * who is going to fix it does not compile.
 */
export interface RecordedReport {
  /**
   * The first line of the report, as the runtime prints it — `BUG: HashMap was
   * garbage collected without being dropped.` — and matched against exactly
   * that.
   */
  report: string;
  /**
   * The addendum item that owes the fix, named so a reader can find it: the
   * file it is in and what it governs.
   */
  owes: string;
}

/**
 * The reports a golden expects, because the golden stands on a defect that is
 * open and named.
 */
export interface KnownReports {
  /**
   * Why each of these is here, and the whole line the runtime prints. Matched in
   * BOTH directions and as a MULTISET: an expected report that stops appearing
   * fails the golden as loudly as an unexpected one, so a fix cannot pass
   * unnoticed and a papered leak cannot sit here forever — and a golden that
   * leaks TWICE where it recorded once fails too, which set comparison let
   * through.
   */
  except: readonly RecordedReport[];
}

/** The first line of each report, which is what a `KnownReports` entry names. */
function headlines(collected: readonly string[]): string[] {
  return collected.map((report) => report.split('\n')[0]!.trim());
}

/** Read the reports and throw unless they are exactly the expected ones. */
async function check(known: KnownReports | undefined): Promise<void> {
  await settle();
  const collected = reports.slice();
  reports.length = 0;
  // Acknowledging the latch is what the test hooks ask of a suite that has
  // asserted a fatal. Without it one report also fails every test after it, and
  // the message below is the one worth reading.
  if (collected.length > 0) clearFatalLatch();

  // A MULTISET, not a set: each expected line takes one occurrence out of what
  // was seen, so a golden that leaks twice where it recorded once is left with a
  // surplus. Compared as sets, `includes` answered `true` for the second
  // occurrence as readily as the first and the extra leak passed.
  const unmatched = headlines(collected);
  const missing: string[] = [];
  for (const entry of known?.except ?? []) {
    const at = unmatched.indexOf(entry.report);
    if (at === -1) {
      missing.push(entry.report);
      continue;
    }
    unmatched.splice(at, 1);
  }
  const surplus = unmatched;
  if (surplus.length === 0 && missing.length === 0) return;

  const parts: string[] = [];
  if (surplus.length > 0) {
    parts.push(`the ownership runtime reported a problem:\n\n${collected.join('\n\n')}`);
  }
  if (missing.length > 0) {
    parts.push(
      'these reports were expected and did not appear, so the defect they stand for is ' +
        `either fixed — take the line out — or no longer reached:\n${missing.map((l) => `  ${l}`).join('\n')}`,
    );
  }
  throw new Error(parts.join('\n\n'));
}

/**
 * Every check somebody asked for and has not yet taken delivery of. A promise
 * nobody awaits swallows its own rejection: the test callback returns
 * `undefined`, bun records a pass, and the golden's leak claim is never tested.
 * Fourteen drivers did exactly that. A ticket goes in here when the check is
 * asked for and comes out when somebody awaits it; `afterAll` fails the file for
 * whatever is left.
 */
const outstanding = new Map<number, string>();
let issued = 0;

/**
 * A check nobody has started yet. It is a `PromiseLike` rather than a `Promise`
 * because the work must not begin until somebody awaits it: a check that ran on
 * its own would reject into the void, which is the defect this class exists to
 * refuse.
 */
class OwnershipCheck implements PromiseLike<void> {
  readonly #ticket: number;
  readonly #known: KnownReports | undefined;
  #started: Promise<void> | undefined;

  constructor(site: string, known: KnownReports | undefined) {
    issued += 1;
    this.#ticket = issued;
    this.#known = known;
    outstanding.set(this.#ticket, site);
  }

  then<Fulfilled = void, Rejected = never>(
    onFulfilled?: ((value: void) => Fulfilled | PromiseLike<Fulfilled>) | null,
    onRejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
  ): PromiseLike<Fulfilled | Rejected> {
    outstanding.delete(this.#ticket);
    this.#started ??= check(this.#known);
    return this.#started.then(onFulfilled, onRejected);
  }
}

/**
 * Fail unless the runtime stayed quiet. Put this in the last test of a driver,
 * after everything the driver owns has been dropped — and AWAIT it, in an
 * `async` callback. A discarded call fails the file from `afterAll` below.
 *
 * @param known — the reports this golden expects anyway, because it stands on a
 *   named open defect. Matched exactly, in both directions.
 */
export function expectNoOwnershipReports(known?: KnownReports): PromiseLike<void> {
  // The caller's line, so a discarded call says where it was written.
  const site = new Error('expectNoOwnershipReports').stack?.split('\n')[2]?.trim() ?? 'unknown site';
  return new OwnershipCheck(site, known);
}

afterAll(() => {
  if (outstanding.size === 0) return;
  const many = outstanding.size === 1 ? 'once' : `${outstanding.size} times`;
  const sites = [...outstanding.values()].map((site) => `  ${site}`).join('\n');
  outstanding.clear();
  throw new Error(
    `expectNoOwnershipReports() was called ${many} without \`await\`, so the leak check ` +
      `never ran:\n${sites}\n\n` +
      'Write it as `test(..., async () => { await expectNoOwnershipReports(); })`.',
  );
});
