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
// Every helper here READS a sequence somebody else owns. It neither clones nor
// releases an element: it hands back one of them (or an index, or whatever the
// caller's own closure built), and the emitted site around it keeps whatever
// release the sequence already had. That is the right reading for a chain built
// with `iter()`, whose elements are borrows.
//
// A chain built with `into_iter()` is the other half, and it lives in
// `iter_owned.ts`: Rust's consuming terminals TAKE the elements, so the one
// they select is transferred to the caller and every other one is dropped where
// Rust drops it. Which family the emitter writes is decided by the resolved
// method and the element type, not by the name.
//
// Every callback is `Invocable` and is called through `invokeRef`, and every
// helper that takes one releases it when the call ends however it ends: Rust's
// terminals take their `F` BY VALUE and drop it there, and R10 says a closure
// the emitter wrapped is never called as a bare function. A plain arrow passes
// through both unchanged.

import { invokeRef, type Invocable } from '../closure.ts';
import { dropOwned } from '../object.ts';

/**
 * What every helper here reads.
 *
 * `ArrayLike` rather than `T[]`, because a `Vec<u8>` is a `Uint8Array` in the
 * port and a `Uint8Array` is not a `number[]`. Every helper walks by index for
 * the same reason.
 */
export type Seq<T> = ArrayLike<T>;

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

/**
 * How a terminal took its callback, which decides whether it releases it.
 *
 * Rust's terminals take their `F` BY VALUE and drop it where the call ends, so
 * `xs.into_iter().find(p)` consumes `p`. `find(&mut p)` type-checks through the
 * `impl FnMut for &mut F` blanket: what the terminal takes by value is then the
 * REFERENCE, dropping a reference does nothing, and `p` is still the caller's
 * to call again. The port has no `&mut`, so the closure object itself is handed
 * over and the terminal has to be told which of the two happened — the call's
 * own text cannot say, and a terminal that released a borrowed callback left
 * the next call reading captures that were gone and the caller's own release
 * dropping it a second time.
 */
export type CallbackMode = 'own' | 'borrow';

/** Release a callback the terminal took by value; leave a borrowed one alone. */
export function releaseCallback(f: unknown, mode: CallbackMode): void {
  if (mode === 'own') dropOwned(f);
}

/**
 * Rust's `iter.step_by(n)`: every `n`th element, starting with the first.
 *
 * The port materialises an iterator as an array, and no array declares
 * `stepBy`, so `(0..10).step_by(2)` came out as `(range(0, 10)).stepBy(2)` — a
 * `TypeError` with no diagnostic beside it (E7). Rust panics on a step of zero,
 * which is what an infinite sequence would be, and so does this.
 */
export function stepBy<T>(xs: Seq<T>, step: number): T[] {
  if (!Number.isInteger(step) || step <= 0) {
    throw new RangeError('step_by: the step must be a positive integer, as Rust requires');
  }
  const out: T[] = [];
  for (let at = 0; at < xs.length; at += step) out.push(xs[at] as T);
  return out;
}

/**
 * Rust's `range.contains(&item)`, from the range's BOUNDS.
 *
 * `Range::contains` is a comparison against the two ends, and it is what a
 * range of a type the port cannot count still answers — a float range is not an
 * iterator in Rust either, and neither is an unbounded one. Written inline as
 * `start <= item && item < end` the port evaluated the ITEM twice, so
 * `(0u32..n).contains(&side())` called `side()` twice, and it evaluated the end
 * only when the first comparison held, where Rust builds the whole range before
 * it looks at the item at all (O7). The arguments here stand in Rust's own
 * order: start, then end, then the item.
 *
 * `null` for a bound is Rust's `Unbounded`: `(a..)`, `(..b)`, `(..=b)` and
 * `(..)` are all this, and each of them used to fall to the materialisation
 * hole even though their answers are `a <= x`, `x < b`, `x <= b` and `true`
 * (O8).
 */
export function rangeContains<T>(
  start: T | null,
  end: T | null,
  inclusive: boolean,
  item: T,
): boolean {
  if (start != null && compareKeys(start, item) > 0) return false;
  if (end == null) return true;
  const at = compareKeys(item, end);
  return inclusive ? at <= 0 : at < 0;
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
export function iterFilterMap<T, U>(xs: Seq<T>, f: Invocable<[T], U | null>, mode: CallbackMode = 'own'): U[] {
  const out: U[] = [];
  try {
    for (let i = 0; i < xs.length; i++) {
      const got = invokeRef(f, xs[i] as T);
      if (got != null) out.push(got);
    }
  } finally {
    releaseCallback(f, mode);
  }
  return out;
}

/** Rust's `iter.position(p)`: the index of the first match, or `None`. */
export function iterPosition<T>(xs: Seq<T>, p: Invocable<[T], boolean>, mode: CallbackMode = 'own'): number | null {
  try {
    for (let i = 0; i < xs.length; i++) {
      if (invokeRef(p, xs[i]!)) return i;
    }
    return null;
  } finally {
    releaseCallback(p, mode);
  }
}

/** Rust's `iter.rposition(p)`: the index of the LAST match, or `None`. */
export function iterRposition<T>(xs: Seq<T>, p: Invocable<[T], boolean>, mode: CallbackMode = 'own'): number | null {
  try {
    for (let i = xs.length - 1; i >= 0; i--) {
      if (invokeRef(p, xs[i]!)) return i;
    }
    return null;
  } finally {
    releaseCallback(p, mode);
  }
}

/** Rust's `iter.find(p)`: the first matching element, or `None`. */
export function iterFind<T>(xs: Seq<T>, p: Invocable<[T], boolean>, mode: CallbackMode = 'own'): T | null {
  try {
    for (let i = 0; i < xs.length; i++) {
      if (invokeRef(p, xs[i]!)) return xs[i]!;
    }
    return null;
  } finally {
    releaseCallback(p, mode);
  }
}

/**
 * Rust's `iter.find_map(f)`: the first `Some` the closure answers, or `None`.
 *
 * The closure's own `Option<U>` is `U | null` here, so "the first `Some`" is
 * "the first result that is not `null`".
 */
export function iterFindMap<T, U>(xs: Seq<T>, f: Invocable<[T], U | null>, mode: CallbackMode = 'own'): U | null {
  try {
    for (let i = 0; i < xs.length; i++) {
      const got = invokeRef(f, xs[i]!);
      if (got != null) return got;
    }
    return null;
  } finally {
    releaseCallback(f, mode);
  }
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
 * The fold is Rust's, and so is the ORDER it hands its two values to the
 * comparator: `Iterator::max_by` is `fold1(|best, candidate| cmp::max_by(best,
 * candidate, compare))`, and `cmp::max_by` asks `compare(&best, &candidate)`.
 * The port asked `cmp(candidate, best)`. An antisymmetric comparator hides that
 * in the WINNER and nothing else does: a comparator with a side effect sees the
 * pairs reversed — over `[1, 2, 3]` Rust logs `(1,2)` then `(2,3)` — and one
 * whose two arguments mean different things (a needle and a haystack, a query
 * and a row) answers about the wrong one.
 *
 * `Less` or `Equal` takes the candidate, which is what makes the LAST of
 * several equal elements the maximum; `min_by` takes it only on `Greater`,
 * keeping the FIRST. That asymmetry is deliberate in `std` and is observable
 * whenever the elements carry anything the comparison does not read.
 */
export function iterMaxBy<T>(
  xs: Seq<T>,
  cmp: Invocable<[T, T], number>,
  mode: CallbackMode = 'own',
): T | null {
  return foldBest(xs, cmp, (ordering) => ordering <= 0, mode);
}

/** Rust's `iter.min_by(cmp)`: the minimum, or `None`. The first of a tie wins. */
export function iterMinBy<T>(
  xs: Seq<T>,
  cmp: Invocable<[T, T], number>,
  mode: CallbackMode = 'own',
): T | null {
  return foldBest(xs, cmp, (ordering) => ordering > 0, mode);
}

/** The one fold both comparator readers are: `cmp(best, candidate)`, in order. */
function foldBest<T>(
  xs: Seq<T>,
  cmp: Invocable<[T, T], number>,
  takeCandidate: (ordering: number) => boolean,
  mode: CallbackMode,
): T | null {
  if (xs.length === 0) {
    releaseCallback(cmp, mode);
    return null;
  }
  try {
    let best = xs[0]!;
    for (let i = 1; i < xs.length; i++) {
      if (takeCandidate(invokeRef(cmp, best, xs[i]!))) best = xs[i]!;
    }
    return best;
  } finally {
    releaseCallback(cmp, mode);
  }
}

/**
 * Rust's `iter.max_by_key(f)`: the maximum by key, or `None`. The last of a tie
 * wins.
 *
 * Rust writes this as `self.map(|x| (f(&x), x)).max_by(|a, b| a.0.cmp(&b.0))`,
 * which is LAZY: the key closure is called exactly once per element, in element
 * order, interleaved with the comparisons. Calling it inside the comparator
 * instead called it twice per comparison and in the reverse order, which an
 * `FnMut` key with a side effect — a counter, a cache, a log — can see.
 */
export function iterMaxByKey<T, K>(
  xs: Seq<T>,
  f: Invocable<[T], K>,
  mode: CallbackMode = 'own',
): T | null {
  return foldByKey(xs, f, (ordering) => ordering <= 0, mode, 'borrow');
}

/** Rust's `iter.min_by_key(f)`: the minimum by key, or `None`. The first of a tie wins. */
export function iterMinByKey<T, K>(
  xs: Seq<T>,
  f: Invocable<[T], K>,
  mode: CallbackMode = 'own',
): T | null {
  return foldByKey(xs, f, (ordering) => ordering > 0, mode, 'borrow');
}

/**
 * Who owns the elements a fold or an adaptor walks.
 *
 * A chain built with `iter()` holds borrows and the sequence is somebody
 * else's; one built with `into_iter()` owns every element it walks, and what it
 * discards it drops.
 */
export type ElementMode = 'own' | 'borrow';

/**
 * The one fold `max_by_key` and `min_by_key` are, in both ownership modes.
 *
 * Rust writes both as `self.map(|x| (f(&x), x)).max_by(|a, b| a.0.cmp(&b.0))`,
 * which settles three things at once. The key closure is called exactly ONCE
 * per element, in element order, interleaved with the comparisons — called
 * inside the comparator instead it runs twice per comparison and in the reverse
 * order, which an `FnMut` key with a side effect can see. The loser's PAIR is
 * dropped, so its key goes with it. And the winner's pair is destructured: the
 * element comes out and its key is dropped there.
 *
 * The keys are the borrowed fold's business too — `f` builds them out of
 * whatever it was given, and Rust drops them for `iter()` exactly as it does
 * for `into_iter()`. The reading fold released none of them, which is the only
 * thing `elements` does NOT decide: what that flag settles is whether the
 * ELEMENTS the fold passes over are its to release.
 */
export function foldByKey<T, K>(
  xs: Seq<T>,
  f: Invocable<[T], K>,
  takeCandidate: (ordering: number) => boolean,
  mode: CallbackMode,
  elements: ElementMode,
): T | null {
  const owns = elements === 'own';
  if (xs.length === 0) {
    releaseCallback(f, mode);
    return null;
  }
  let at = 0;
  let have = false;
  let best!: T;
  let bestKey!: K;
  try {
    for (let i = 0; i < xs.length; i++) {
      const candidate = xs[i] as T;
      const key = invokeRef(f, candidate);
      if (!have) {
        best = candidate;
        bestKey = key;
        have = true;
      } else if (takeCandidate(compareKeys(bestKey, key))) {
        dropOwned(bestKey);
        if (owns) dropOwned(best);
        best = candidate;
        bestKey = key;
      } else {
        dropOwned(key);
        if (owns) dropOwned(candidate);
      }
      at = i + 1;
    }
    // The winner's key goes with the pair Rust destructures; only the element
    // reaches the caller.
    dropOwned(bestKey);
    return best;
  } catch (thrown) {
    // The accumulator and its key are nobody else's, and the element the key
    // closure was reading is still in `xs[at..]`.
    if (have) {
      dropOwned(bestKey);
      if (owns) dropOwned(best);
    }
    throw thrown;
  } finally {
    if (owns) for (let i = at; i < xs.length; i++) dropOwned(xs[i]);
    releaseCallback(f, mode);
  }
}

/**
 * Rust's `iter.reduce(f)`: the fold with no initial value, or `None` for an
 * empty sequence.
 *
 * `Array.prototype.reduce` with no initial value THROWS on an empty array
 * rather than answering absence, so it cannot stand in for this one.
 */
export function iterReduce<T>(
  xs: Seq<T>,
  f: Invocable<[T, T], T>,
  mode: CallbackMode = 'own',
): T | null {
  if (xs.length === 0) {
    releaseCallback(f, mode);
    return null;
  }
  try {
    // No `!` on the elements: `T` may itself admit null — Rust's `Option<T>` is
    // `T | null` here — and `xs[i]!` narrows to `NonNullable<T>`, which is not
    // what the fold takes.
    let acc: T = xs[0] as T;
    for (let i = 1; i < xs.length; i++) acc = invokeRef(f, acc, xs[i] as T);
    return acc;
  } finally {
    releaseCallback(f, mode);
  }
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
export function compareKeys(a: unknown, b: unknown): number {
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
