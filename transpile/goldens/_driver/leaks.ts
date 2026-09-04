// The ownership check every golden driver ends with: collect what the runtime
// reports while the golden runs, and fail the run with it at the end.
//
// A leak check built on FinalizationRegistry is one-sided and always will be. A
// report that fires proves a value was collected without being dropped, and that
// is a defect. Silence proves only that nothing was collected — a garbage
// collector is never obliged to collect anything. Forcing a collection and
// giving the loop a turn to deliver what it found is what makes the silence
// worth something; it does not make it certain.

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
 * Fail unless the runtime stayed quiet. Put this in the last test of a driver,
 * after everything the driver owns has been dropped.
 */
export async function expectNoOwnershipReports(): Promise<void> {
  await settle();
  if (reports.length === 0) return;
  const collected = reports.join('\n\n');
  reports.length = 0;
  // Acknowledging the latch is what the test hooks ask of a suite that has
  // asserted a fatal. Without it this one report also fails every test after it,
  // and the message below is the one worth reading.
  clearFatalLatch();
  throw new Error(`the ownership runtime reported a problem:\n\n${collected}`);
}
