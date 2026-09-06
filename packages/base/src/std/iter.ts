// TS-ONLY: the iterator adaptors Rust answers an `Option` with, whose nearest
// JavaScript spelling answers something else.
//
// The port writes `Option<T>` as `T | null` (R5), and a caller that has been
// handed a `None` tests it with `== null`. Two JavaScript sentinels break that
// test in two different ways:
//
//   - `Array.prototype.findIndex` answers `-1`, and `-1 != null` is TRUE. The
//     emitted `if (pos != null) entries.splice(pos, 1)` therefore ran for a
//     watcher that was NOT in the list, and `splice(-1, 1)` deleted the last
//     one instead — the reactor unsubscribed a live watcher every time it was
//     asked to unsubscribe one that had already gone.
//   - `Array.prototype.find` and `Array.prototype.at` answer `undefined`, which
//     `!= null` reads as absent by accident but which is not the value the
//     port's declared `T | null` promises: a caller writing `x === null`, a
//     `JSON.stringify` of a struct field, and `tsc` all tell the two apart.
//
// So every Option-returning adaptor goes through a helper here, and the helper
// answers `null`. Each takes the sequence as its first argument so the emitter
// writes the receiver exactly once: a receiver written twice is evaluated
// twice, and the second evaluation of a call with an effect is a defect the
// self-review scans for.
//
// None of these helpers participates in ownership. They neither clone nor
// release: they read the sequence they are handed and hand back one of its
// elements (or an index, or whatever the caller's own closure built). The
// emitted site around them keeps whatever release the sequence already had.

/**
 * What every helper here reads.
 *
 * `ArrayLike` rather than `T[]`, because a `Vec<u8>` is a `Uint8Array` in the
 * port and a `Uint8Array` is not a `number[]`. Every helper walks by index for
 * the same reason.
 */
type Seq<T> = ArrayLike<T>;

/**
 * Rust's `a..b` — the half-open range, as the sequence of its values.
 *
 * The port has no `Range` type, and a range used as a VALUE was written
 * `undefined`: `for attempt in 0..MAX_RETRIES` emitted
 * `for (const attempt of undefined)`, which raises `undefined is not iterable`
 * the first time the loop is reached. `Entity::commit`'s retry loop is one of
 * them. Materialising it is what makes every other adaptor work on it — `rev`,
 * `map`, `filter`, `contains` are all array operations here — and the corpus's
 * ranges are small: `0..16`, `0..MAX_RETRIES`, `0..bytes.length`.
 *
 * An empty or reversed range is empty, as Rust's is.
 */
export function range(from: number, to: number): number[] {
  const out: number[] = [];
  for (let n = from; n < to; n++) out.push(n);
  return out;
}

/** Rust's `a..=b`, which includes the last value. */
export function rangeIncl(from: number, to: number): number[] {
  const out: number[] = [];
  for (let n = from; n <= to; n++) out.push(n);
  return out;
}

/**
 * Rust's `iter.filter_map(f)`: the closure's `Some` results, in order.
 *
 * The closure's `Option<U>` is `U | null` here, so this keeps what is not
 * `null`. Written as the camelCase of its Rust name it was `xs.filterMap(..)`,
 * a method no array declares — twelve emitted sites.
 */
export function iterFilterMap<T, U>(xs: Seq<T>, f: (x: T) => U | null): U[] {
  const out: U[] = [];
  for (let i = 0; i < xs.length; i++) {
    const got = f(xs[i] as T);
    if (got != null) out.push(got);
  }
  return out;
}

/** Rust's `iter.position(p)`: the index of the first match, or `None`. */
export function iterPosition<T>(xs: Seq<T>, p: (x: T) => boolean): number | null {
  for (let i = 0; i < xs.length; i++) {
    if (p(xs[i]!)) return i;
  }
  return null;
}

/** Rust's `iter.rposition(p)`: the index of the LAST match, or `None`. */
export function iterRposition<T>(xs: Seq<T>, p: (x: T) => boolean): number | null {
  for (let i = xs.length - 1; i >= 0; i--) {
    if (p(xs[i]!)) return i;
  }
  return null;
}

/** Rust's `iter.find(p)`: the first matching element, or `None`. */
export function iterFind<T>(xs: Seq<T>, p: (x: T) => boolean): T | null {
  for (let i = 0; i < xs.length; i++) {
    if (p(xs[i]!)) return xs[i]!;
  }
  return null;
}

/**
 * Rust's `iter.find_map(f)`: the first `Some` the closure answers, or `None`.
 *
 * The closure's own `Option<U>` is `U | null` here, so "the first `Some`" is
 * "the first result that is not `null`".
 */
export function iterFindMap<T, U>(xs: Seq<T>, f: (x: T) => U | null): U | null {
  for (let i = 0; i < xs.length; i++) {
    const got = f(xs[i]!);
    if (got != null) return got;
  }
  return null;
}

/** Rust's `iter.last()`: the final element, or `None` for an empty sequence. */
export function iterLast<T>(xs: Seq<T>): T | null {
  return xs.length === 0 ? null : xs[xs.length - 1]!;
}

/**
 * Rust's `slice.first()`: the leading element, or `None`.
 *
 * Not an iterator adaptor, but the same sentinel: `xs[0]` on an empty array is
 * `undefined`, and the port's `Option<T>` is `T | null`.
 */
export function iterFirst<T>(xs: Seq<T>): T | null {
  return xs.length === 0 ? null : xs[0]!;
}

/** Rust's `slice.get(i)`: the element at `i`, or `None` for an index past the end. */
export function iterGet<T>(xs: Seq<T>, i: number): T | null {
  return xs[i] ?? null;
}

/**
 * Rust's `iter.max_by(cmp)`: the maximum, or `None` for an empty sequence.
 *
 * Rust returns the LAST element among several that compare equal at the
 * maximum, and the FIRST among several at the minimum. That asymmetry is
 * deliberate in `std` and observable whenever the elements carry anything the
 * comparison does not read, so the two loops below are not symmetrical either.
 */
export function iterMaxBy<T>(xs: Seq<T>, cmp: (a: T, b: T) => number): T | null {
  if (xs.length === 0) return null;
  let best = xs[0]!;
  for (let i = 1; i < xs.length; i++) {
    if (cmp(xs[i]!, best) >= 0) best = xs[i]!;
  }
  return best;
}

/** Rust's `iter.min_by(cmp)`: the minimum, or `None`. The first of a tie wins. */
export function iterMinBy<T>(xs: Seq<T>, cmp: (a: T, b: T) => number): T | null {
  if (xs.length === 0) return null;
  let best = xs[0]!;
  for (let i = 1; i < xs.length; i++) {
    if (cmp(xs[i]!, best) < 0) best = xs[i]!;
  }
  return best;
}

/** Rust's `iter.max_by_key(f)`: the maximum by key, or `None`. The last of a tie wins. */
export function iterMaxByKey<T, K>(xs: Seq<T>, f: (x: T) => K): T | null {
  return iterMaxBy(xs, (a, b) => compareKeys(f(a), f(b)));
}

/** Rust's `iter.min_by_key(f)`: the minimum by key, or `None`. The first of a tie wins. */
export function iterMinByKey<T, K>(xs: Seq<T>, f: (x: T) => K): T | null {
  return iterMinBy(xs, (a, b) => compareKeys(f(a), f(b)));
}

/**
 * Rust's `iter.reduce(f)`: the fold with no initial value, or `None` for an
 * empty sequence.
 *
 * `Array.prototype.reduce` with no initial value THROWS on an empty array
 * rather than answering absence, so it cannot stand in for this one.
 */
export function iterReduce<T>(xs: Seq<T>, f: (acc: T, x: T) => T): T | null {
  if (xs.length === 0) return null;
  // No `!` on the elements: `T` may itself admit null — Rust's `Option<T>` is
  // `T | null` here — and `xs[i]!` narrows to `NonNullable<T>`, which is not
  // what the fold takes.
  let acc: T = xs[0] as T;
  for (let i = 1; i < xs.length; i++) acc = f(acc, xs[i] as T);
  return acc;
}

/**
 * The `Ord` a `max_by_key`/`min_by_key` key is compared with.
 *
 * A key is whatever the closure built, so the comparison is the value's own
 * surface: a type that declares `compareTo` is ordered by it, and a number,
 * bigint, string or boolean by `<`. Anything else has no order the port can
 * read, and answering `0` there would silently pick the first element, so it
 * raises instead.
 */
function compareKeys(a: unknown, b: unknown): number {
  if (a !== null && typeof a === 'object' && typeof (a as { compareTo?: unknown }).compareTo === 'function') {
    return (a as { compareTo: (o: unknown) => number }).compareTo(b);
  }
  if (typeof a === 'number' && typeof b === 'number') return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === 'bigint' && typeof b === 'bigint') return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === 'string' && typeof b === 'string') return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === 'boolean' && typeof b === 'boolean') return a === b ? 0 : a ? 1 : -1;
  throw new TypeError(
    `max_by_key/min_by_key: a key of type ${typeof a} declares no order (no compareTo, not a primitive)`,
  );
}
