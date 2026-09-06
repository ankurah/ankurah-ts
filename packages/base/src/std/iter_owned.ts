// TS-ONLY: the iterator terminals that TAKE the sequence's elements.
//
// For: Rust's consuming terminals own what they walk. `into_iter().find(p)`
// hands the caller the element it selected and DROPS every element it passed
// and every element it never reached; `max_by_key` drops every loser;
// `position` moves each element into the closure, which drops it. The reading
// family in `iter.ts` does none of that, because a chain built with `iter()`
// holds borrows and the sequence belongs to somebody else.
//
// Written with the reading family, a consuming chain got one of two wrong
// answers. Where the emitter had also given the sequence a release of its own
// — `const _t0 = [...tokens]; try { .. } finally { dropOwned(_t0) }` — the
// element the terminal handed back was released under the caller's feet, and a
// closure that dropped its own element hit `OwnershipFatal` on the second drop.
// Where it had not, every element the terminal did not return simply leaked:
// `iterMaxByKey([a, b, c], key)` returned the winner and released neither loser.
//
// So ownership is part of the lowering. The emitter writes these names when the
// resolved terminal comes through `Iterator` — `slice::last(&self)` is a
// borrow, `Iterator::last(self)` is not — and the elements have drop glue.
//
// **The cursor discipline.** Each helper keeps one index meaning "the elements
// from here on are still the iterator's", advances it as it disposes of them,
// and releases whatever is left in a `finally`. That is what Rust's own unwind
// does: the closure that was handed an element by value drops it, and the
// iterator drops what it had not yet produced. Every helper releases its
// callback in the same `finally`, because Rust's terminals take `F` by value.

import { compareKeys, type Seq } from './iter.ts';
import { invokeRef, type Invocable } from '../closure.ts';
import { dropOwned } from '../object.ts';

/** Release `xs[from..]` — what the iterator still held when it was dropped. */
function dropFrom<T>(xs: Seq<T>, from: number): void {
  for (let i = from; i < xs.length; i++) dropOwned(xs[i]);
}

/**
 * Rust's `into_iter().find(p)`: the first element the predicate accepts, or
 * `None`.
 *
 * `p` takes a REFERENCE, so an element it rejects is still the iterator's and
 * is dropped as the walk advances past it. The accepted one is the caller's.
 */
export function iterFindOwned<T>(xs: Seq<T>, p: Invocable<[T], boolean>): T | null {
  let at = 0;
  try {
    for (let i = 0; i < xs.length; i++) {
      if (invokeRef(p, xs[i] as T)) {
        at = i + 1;
        return xs[i] as T;
      }
      at = i + 1;
      dropOwned(xs[i]);
    }
    return null;
  } finally {
    dropFrom(xs, at);
    dropOwned(p);
  }
}

/**
 * Rust's `into_iter().position(p)`: the index of the first match, or `None`.
 *
 * `p` takes the element BY VALUE, so from the call onwards the element is the
 * closure's and this helper never releases it — on a normal return or on a
 * throw. What is left is everything the walk never reached.
 */
export function iterPositionOwned<T>(xs: Seq<T>, p: Invocable<[T], boolean>): number | null {
  let at = 0;
  try {
    for (let i = 0; i < xs.length; i++) {
      at = i + 1;
      if (invokeRef(p, xs[i] as T)) return i;
    }
    return null;
  } finally {
    dropFrom(xs, at);
    dropOwned(p);
  }
}

/** The same walked from the end: `rposition`. What is left is the front. */
export function iterRpositionOwned<T>(xs: Seq<T>, p: Invocable<[T], boolean>): number | null {
  let end = xs.length;
  try {
    for (let i = xs.length - 1; i >= 0; i--) {
      end = i;
      if (invokeRef(p, xs[i] as T)) return i;
    }
    return null;
  } finally {
    for (let i = 0; i < end; i++) dropOwned(xs[i]);
    dropOwned(p);
  }
}

/**
 * Rust's `into_iter().find_map(f)`: the first `Some` the closure answers.
 *
 * `f` takes the element BY VALUE, so what it was handed is its to keep or drop;
 * what is left is everything after the element that answered.
 */
export function iterFindMapOwned<T, U>(xs: Seq<T>, f: Invocable<[T], U | null>): U | null {
  let at = 0;
  try {
    for (let i = 0; i < xs.length; i++) {
      at = i + 1;
      const got = invokeRef(f, xs[i] as T);
      if (got != null) return got;
    }
    return null;
  } finally {
    dropFrom(xs, at);
    dropOwned(f);
  }
}

/**
 * Rust's `into_iter().last()`: the final element, with every earlier one
 * dropped as the walk passes it.
 *
 * `slice::last(&self)` is a different method and stays in the reading family:
 * it borrows, and the sequence is somebody else's.
 */
export function iterLastOwned<T>(xs: Seq<T>): T | null {
  if (xs.length === 0) return null;
  for (let i = 0; i < xs.length - 1; i++) dropOwned(xs[i]);
  return xs[xs.length - 1] as T;
}

/**
 * Rust's `into_iter().reduce(f)`: the fold with no initial value.
 *
 * `f` takes BOTH the accumulator and the next element by value and answers the
 * next accumulator, so every element passes through the closure and this helper
 * releases none of them — only what the walk never reached.
 */
export function iterReduceOwned<T>(xs: Seq<T>, f: Invocable<[T, T], T>): T | null {
  if (xs.length === 0) {
    dropOwned(f);
    return null;
  }
  let at = 1;
  try {
    let acc: T = xs[0] as T;
    for (let i = 1; i < xs.length; i++) {
      at = i + 1;
      acc = invokeRef(f, acc, xs[i] as T);
    }
    return acc;
  } finally {
    dropFrom(xs, at);
    dropOwned(f);
  }
}

/**
 * Rust's `into_iter().max_by(cmp)`: the maximum, with every loser dropped.
 *
 * The fold is Rust's own, and so is its argument order: `cmp(best, candidate)`,
 * with the CANDIDATE taken when the answer is `Less` or `Equal` — which is what
 * makes the LAST of several equal elements the maximum. The comparator only
 * borrows, so the element it rejects is still the fold's to drop.
 */
export function iterMaxByOwned<T>(xs: Seq<T>, cmp: Invocable<[T, T], number>): T | null {
  return foldOwned(xs, cmp, (ordering) => ordering <= 0);
}

/**
 * Rust's `into_iter().min_by(cmp)`: the minimum, with every loser dropped. The
 * candidate is taken only where `cmp(best, candidate)` is `Greater`, which
 * keeps the FIRST of several equal elements.
 */
export function iterMinByOwned<T>(xs: Seq<T>, cmp: Invocable<[T, T], number>): T | null {
  return foldOwned(xs, cmp, (ordering) => ordering > 0);
}

/** The one fold both comparator terminals are, with the loser released. */
function foldOwned<T>(
  xs: Seq<T>,
  cmp: Invocable<[T, T], number>,
  takeCandidate: (ordering: number) => boolean,
): T | null {
  if (xs.length === 0) {
    dropOwned(cmp);
    return null;
  }
  let best: T = xs[0] as T;
  let at = 1;
  try {
    for (let i = 1; i < xs.length; i++) {
      const candidate = xs[i] as T;
      if (takeCandidate(invokeRef(cmp, best, candidate))) {
        dropOwned(best);
        best = candidate;
      } else {
        dropOwned(candidate);
      }
      at = i + 1;
    }
    return best;
  } catch (thrown) {
    // The accumulator is nobody else's, and the element the comparator was
    // reading is still in `xs[at..]`.
    dropOwned(best);
    throw thrown;
  } finally {
    dropFrom(xs, at);
    dropOwned(cmp);
  }
}

/**
 * Rust's `into_iter().max_by_key(f)`: the maximum by key, with every loser
 * dropped.
 *
 * Rust writes this as `self.map(|x| (f(&x), x)).max_by(|a, b| a.0.cmp(&b.0))`,
 * which is lazy: the key closure is called exactly ONCE per element, in element
 * order, interleaved with the comparisons. Calling it inside the comparator
 * instead calls it twice per comparison and in the wrong order, which an `FnMut`
 * key with a side effect can see. The losing key is dropped with its element,
 * as the pair Rust builds is.
 */
export function iterMaxByKeyOwned<T, K>(xs: Seq<T>, f: Invocable<[T], K>): T | null {
  return foldByKeyOwned(xs, f, (ordering) => ordering <= 0);
}

/** Rust's `into_iter().min_by_key(f)`. The first of a tie wins. */
export function iterMinByKeyOwned<T, K>(xs: Seq<T>, f: Invocable<[T], K>): T | null {
  return foldByKeyOwned(xs, f, (ordering) => ordering > 0);
}

function foldByKeyOwned<T, K>(
  xs: Seq<T>,
  f: Invocable<[T], K>,
  takeCandidate: (ordering: number) => boolean,
): T | null {
  if (xs.length === 0) {
    dropOwned(f);
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
        dropOwned(best);
        best = candidate;
        bestKey = key;
      } else {
        dropOwned(key);
        dropOwned(candidate);
      }
      at = i + 1;
    }
    // The winner's key goes with the pair Rust destructures; only the element
    // reaches the caller.
    dropOwned(bestKey);
    return best;
  } catch (thrown) {
    if (have) {
      dropOwned(bestKey);
      dropOwned(best);
    }
    throw thrown;
  } finally {
    dropFrom(xs, at);
    dropOwned(f);
  }
}
