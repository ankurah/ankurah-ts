// TS-ONLY: the hole an emitted file carries where the port has no lowering.

/**
 * The gap a shape the transpiler cannot lower leaves behind, as something that
 * stops the program.
 *
 * For: a Rust shape the engine has no lowering for used to be reported to
 * whoever ran the transpiler and then emitted ANYWAY, as the nearest thing the
 * engine could write. A dropped `..rest`, a consuming guard the arm chain lost,
 * a refutable arm run for a whole variant — each of those produced code that
 * runs and answers the wrong thing, and a wrong answer at run time is a bug
 * nobody can trace back to a diagnostic printed at build time.
 *
 * So a known-wrong emission is written as a call to this instead. The
 * diagnostic still goes to whoever built the port; this is what the RUNNING
 * program does when it reaches the gap, which is stop and say which Rust shape
 * it is standing on. It is not an error type anything catches or handles: it is
 * the port telling its caller that this path was never translated.
 *
 * Its return type is `never`, so a hole stands wherever an expression stands —
 * an arm's value, a return, an argument — and TypeScript keeps narrowing
 * correctly around it.
 */
export function unsupported(what: string): never {
  throw new UnsupportedShape(what);
}

/**
 * What `unsupported` throws. Named for what it says rather than for a category
 * of failure, because it is not one: it is the absence of a translation.
 */
export class UnsupportedShape extends Error {
  constructor(readonly what: string) {
    super(`the port has no translation for this: ${what}`);
    this.name = 'UnsupportedShape';
  }
}
