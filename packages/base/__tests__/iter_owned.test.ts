// TS-ONLY: the consuming iterator terminals, and who releases what. F1.
//
// Rust's `into_iter().find(p)` owns the sequence it walks: the element it
// selects becomes the caller's and every other one is dropped, including the
// ones the walk never reached. `max_by_key` drops both losers and hands back
// the winner. `position` moves each element into the closure, which is then the
// only thing that can drop it.
//
// Every test below is written against a droppable element that counts its own
// drops, because the number of drops IS the property: one for every element
// that did not reach the caller, and none for the one that did.
//
// A terminal that hands the element to a callback BY VALUE is tested through
// `generated`, which is the shape the emitter writes for such a callback: the
// body runs inside a scope that releases the parameter however the invocation
// is left. Hand-written callbacks that dropped their argument as the test
// author saw fit hid the emitter's own gap for a whole slice — the emitted
// callback released nothing, and the tests still passed because they were
// doing the emitter's work by hand. O1.

import { describe, expect, test } from 'bun:test';
import { Drop, OwnedClosure } from '../src/index.ts';
import {
  iterFindMapOwned,
  iterFindOwned,
  iterLastOwned,
  iterMaxByKeyOwned,
  iterMaxByOwned,
  iterMinByKeyOwned,
  iterMinByOwned,
  iterPositionOwned,
  iterReduceOwned,
  iterRpositionOwned,
  iterFirstOwned,
  filterOwned,
  skipOwned,
  takeOwned,
  stepByOwned,
  SeqCursor,
} from '../src/std/iter_owned.ts';
import { iterMaxByKey } from '../src/std/iter.ts';

/** An element that says when it was dropped, and refuses a second drop. */
class Token extends Drop {
  static dropped: number[] = [];
  constructor(readonly n: number) {
    super();
  }
  protected override onDrop(): void {
    Token.dropped.push(this.n);
  }
}

function tokens(...ns: number[]): Token[] {
  Token.dropped = [];
  return ns.map((n) => new Token(n));
}

/** What is still alive, by the numbers the elements were built with. */
function dropped(): number[] {
  return [...Token.dropped].sort((a, b) => a - b);
}

/**
 * A callback in the shape the emitter writes for a by-value parameter: the
 * body's answer, with the parameter released however the invocation is left.
 *
 * Rust drops a by-value closure parameter at the end of every call, on the
 * normal return and while an unwind passes through, because it is a local of
 * the closure's body. A callback that does anything else is a callback the
 * emitter does not write, and testing the terminals against one says nothing
 * about the code the port produces.
 */
function generated<T extends Drop, R>(f: (x: T) => R): (x: T) => R {
  return (x: T) => {
    try {
      return f(x);
    } finally {
      x.drop();
    }
  };
}

describe('a consuming terminal drops every element it does not hand back', () => {
  test('find: the losers before it, and everything after it', () => {
    const xs = tokens(1, 2, 3, 4);
    const found = iterFindOwned(xs, (t) => t.n === 2);
    expect(found?.n).toBe(2);
    expect(dropped()).toEqual([1, 3, 4]);
    // The one the caller was given is still alive, and is the caller's to drop.
    found!.drop();
    expect(dropped()).toEqual([1, 2, 3, 4]);
  });

  test('find with no match drops all of them and answers null', () => {
    const xs = tokens(1, 2, 3);
    expect(iterFindOwned(xs, () => false)).toBe(null);
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('position: the closure owns what it was handed, so nothing here drops it', () => {
    const xs = tokens(1, 2, 3, 4);
    const at = iterPositionOwned(
      xs,
      generated((t: Token) => t.n === 3),
    );
    expect(at).toBe(2);
    // 1, 2 and 3 went through the callback, which released each of them at the
    // end of its own invocation; 4 was never reached and is the walk's.
    expect(dropped()).toEqual([1, 2, 3, 4]);
  });

  test('rposition walks from the end and leaves the front to be dropped', () => {
    const xs = tokens(1, 2, 3, 4);
    const at = iterRpositionOwned(
      xs,
      generated((t: Token) => t.n === 3),
    );
    expect(at).toBe(2);
    // 4 and 3 went through the callback, which released each of them; 1 and 2
    // were never reached and are the walk's.
    expect(dropped()).toEqual([1, 2, 3, 4]);
  });

  test('find_map answers the first Some and drops what it never reached', () => {
    const xs = tokens(1, 2, 3);
    const got = iterFindMapOwned(
      xs,
      generated((t: Token) => (t.n === 2 ? `n${t.n}` : null)),
    );
    expect(got).toBe('n2');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('last drops every element the walk passed', () => {
    const xs = tokens(1, 2, 3);
    const got = iterLastOwned(xs);
    expect(got?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);
    expect(iterLastOwned([])).toBe(null);
  });

  test('reduce hands every element to the closure and drops none of them', () => {
    // `reduce` hands BOTH the accumulator and the next element over by value.
    // The emitter's shape releases the one the body does not answer with; the
    // one it returns is the next accumulator and is released by nobody here.
    const xs = tokens(1, 2, 3);
    const got = iterReduceOwned(xs, (a: Token, b: Token) => {
      try {
        return a;
      } finally {
        b.drop();
      }
    });
    expect(got?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
    got!.drop();
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('max_by and min_by drop every loser', () => {
    const xs = tokens(2, 3, 1);
    const most = iterMaxByOwned(xs, (a, b) => a.n - b.n);
    expect(most?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);

    const ys = tokens(2, 3, 1);
    const least = iterMinByOwned(ys, (a, b) => a.n - b.n);
    expect(least?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
  });

  test('max_by_key and min_by_key drop every loser', () => {
    const xs = tokens(2, 3, 1);
    const most = iterMaxByKeyOwned(xs, (t) => t.n);
    expect(most?.n).toBe(3);
    expect(dropped()).toEqual([1, 2]);

    const ys = tokens(2, 3, 1);
    const least = iterMinByKeyOwned(ys, (t) => t.n);
    expect(least?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
  });

  test('an empty sequence answers null and drops nothing', () => {
    Token.dropped = [];
    expect(iterMaxByOwned([], (a: Token, b: Token) => a.n - b.n)).toBe(null);
    expect(iterMaxByKeyOwned([], (t: Token) => t.n)).toBe(null);
    expect(iterReduceOwned([], (a: Token) => a)).toBe(null);
    expect(dropped()).toEqual([]);
  });
});

describe('a callback that throws leaves nothing behind', () => {
  test('find: the element the predicate was reading, and everything after it', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterFindOwned(xs, (t) => {
        if (t.n === 2) throw new Error('no');
        return false;
      }),
    ).toThrow('no');
    // 1 was passed and dropped; 2 was only borrowed by the predicate, so it is
    // still the walk's; 3 was never reached.
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('max_by: the accumulator as well as the rest', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterMaxByOwned(xs, () => {
        throw new Error('no');
      }),
    ).toThrow('no');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('max_by_key: the winner so far, its key, and the rest', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterMaxByKeyOwned(xs, (t) => {
        if (t.n === 2) throw new Error('no');
        return t.n;
      }),
    ).toThrow('no');
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('position: the callback releases what it was handed, throw or not', () => {
    const xs = tokens(1, 2, 3);
    expect(() =>
      iterPositionOwned(
        xs,
        generated((t: Token) => {
          if (t.n === 2) throw new Error('no');
          return false;
        }),
      ),
    ).toThrow('no');
    // 1 returned normally and 2 threw; the callback released both, because a
    // Rust unwind drops the closure's locals as it passes through. 3 was never
    // reached and the walk released it.
    expect(dropped()).toEqual([1, 2, 3]);
  });
});

describe('a wrapped closure reaches the terminal through invokeRef', () => {
  // F9/R10: a `move` closure over something with drop glue is written as an
  // `OwnedClosure`, which is not callable as a function. Called as one it
  // raised `TypeError: p is not a function` on the first element.
  test('an OwnedClosure predicate is called and then released', () => {
    const captured = new Token(9);
    Token.dropped = [];
    const xs = [new Token(1), new Token(2)];
    const p = new OwnedClosure([captured], (t: Token) => t.n === captured.n - 7);
    const found = iterFindOwned(xs, p);
    expect(found?.n).toBe(2);
    // The closure was released when the terminal ended, which released its
    // captures — Rust drops the `F` the terminal took by value.
    expect(p.isDropped).toBe(true);
    expect(Token.dropped).toContain(9);
    found!.drop();
  });

  test('and so is a plain arrow, which owns nothing', () => {
    const xs = tokens(1, 2);
    expect(iterFindOwned(xs, (t) => t.n === 1)?.n).toBe(1);
  });

  // O2: `find(&mut p)` type-checks through `impl FnMut for &mut F`, so what
  // the terminal takes by value is the REFERENCE — dropping it does nothing
  // and `p` is still the caller's to call again. The port has no reference to
  // hand over, so the emitter says which of the two happened.
  test('a BORROWED callback survives the terminal and can be called again', () => {
    const captured = new Token(9);
    Token.dropped = [];
    const p = new OwnedClosure([captured], (t: Token) => t.n === 2);

    const first = iterFindOwned([new Token(1), new Token(2)], p, 'borrow');
    expect(first?.n).toBe(2);
    expect(p.isDropped).toBe(false);
    first!.drop();

    // The defective answer: `OwnershipFatal` — the closure was released by the
    // first terminal and this call reads captures that are gone.
    const second = iterFindOwned([new Token(3), new Token(2)], p, 'borrow');
    expect(second?.n).toBe(2);
    second!.drop();

    expect(Token.dropped).not.toContain(9);
    p.drop();
    expect(Token.dropped).toContain(9);
  });

  test('every terminal that takes a callback carries the mode', () => {
    const made: OwnedClosure<never[], never>[] = [];
    const closure = <A extends unknown[], R>(f: (...args: A) => R) => {
      const c = new OwnedClosure<A, R>([], f);
      made.push(c as unknown as OwnedClosure<never[], never>);
      return c;
    };
    Token.dropped = [];
    const kept = [
      iterFindOwned(tokens(1, 2), closure((t: Token) => t.n === 2), 'borrow'),
      iterMaxByOwned(tokens(1, 2), closure((a: Token, b: Token) => a.n - b.n), 'borrow'),
      iterMinByOwned(tokens(1, 2), closure((a: Token, b: Token) => a.n - b.n), 'borrow'),
      iterMaxByKeyOwned(tokens(1, 2), closure((t: Token) => t.n), 'borrow'),
      iterMinByKeyOwned(tokens(1, 2), closure((t: Token) => t.n), 'borrow'),
      iterReduceOwned(tokens(1, 2), closure((a: Token, b: Token) => { b.drop(); return a; }), 'borrow'),
    ];
    for (const c of made) expect(c.isDropped).toBe(false);
    for (const k of kept) k?.drop();
    expect(iterPositionOwned(tokens(1, 2), closure((t: Token) => { t.drop(); return t.n === 2; }), 'borrow')).toBe(1);
    expect(
      iterRpositionOwned(tokens(1, 2), closure((t: Token) => { t.drop(); return t.n === 1; }), 'borrow'),
    ).toBe(0);
    expect(iterFindMapOwned(tokens(1, 2), closure((t: Token) => { const n = t.n; t.drop(); return n === 2 ? n : null; }), 'borrow')).toBe(2);
    for (const c of made) {
      expect(c.isDropped).toBe(false);
      c.drop();
    }
  });
});

describe('an eager adaptor over owned elements releases what it discards', () => {
  // O3/O4: Rust's adaptors are lazy and own what they walk — `Filter` drops the
  // element its predicate rejected, `Skip` the prefix, `Take` the tail with the
  // iterator it wraps, `StepBy` what it stepped over. The port writes them
  // eagerly, so the drops happen here; written as array operations they simply
  // lost the discarded elements, and the consuming terminal below could not
  // release what the adaptor had already erased.
  test('filter drops what its predicate rejects and keeps the rest', () => {
    const xs = tokens(1, 2, 3, 4);
    const kept = filterOwned(xs, (t) => t.n % 2 === 0);
    expect(kept.map((t) => t.n)).toEqual([2, 4]);
    expect(dropped()).toEqual([1, 3]);
    for (const t of kept) t.drop();
  });

  test('and on a throw it keeps nothing: the kept, the thrower and the tail', () => {
    const xs = tokens(1, 2, 3, 4);
    expect(() =>
      filterOwned(xs, (t) => {
        if (t.n === 3) throw new Error('no');
        return true;
      }),
    ).toThrow('no');
    // Nobody receives the answer on that path, so nothing may be left alive.
    expect(dropped()).toEqual([1, 2, 3, 4]);
  });

  test('skip drops the prefix and take drops the tail', () => {
    const xs = tokens(1, 2, 3, 4);
    const rest = skipOwned(xs, 2);
    expect(rest.map((t) => t.n)).toEqual([3, 4]);
    expect(dropped()).toEqual([1, 2]);

    const front = takeOwned(rest, 1);
    expect(front.map((t) => t.n)).toEqual([3]);
    expect(dropped()).toEqual([1, 2, 4]);
    front[0]!.drop();

    Token.dropped = [];
    // Out of range on either side is Rust's answer, not an exception.
    expect(skipOwned([], 3)).toEqual([]);
    expect(takeOwned(tokens(1), 9).length).toBe(1);
    expect(dropped()).toEqual([]);
  });

  test('step_by drops what it stepped over, and refuses a step of zero', () => {
    const xs = tokens(1, 2, 3, 4, 5);
    const every = stepByOwned(xs, 2);
    expect(every.map((t) => t.n)).toEqual([1, 3, 5]);
    expect(dropped()).toEqual([2, 4]);
    for (const t of every) t.drop();
  });

  // U1: `step_by` takes the iterator BY VALUE and then panics on a zero step,
  // so Rust's unwind drops the whole sequence. The helper owns `xs` from the
  // moment it is called, and the old test asserted only the throw — which the
  // defective version passed while leaving every element with no owner.
  test('a step of zero throws AND releases the sequence it was handed', () => {
    const xs = tokens(1, 2, 3);
    expect(() => stepByOwned(xs, 0)).toThrow(RangeError);
    expect(dropped()).toEqual([1, 2, 3]);
    const negative = tokens(4, 5);
    expect(() => stepByOwned(negative, -1)).toThrow(RangeError);
    expect(dropped()).toEqual([4, 5]);
    const fractional = tokens(6);
    expect(() => stepByOwned(fractional, 1.5)).toThrow(RangeError);
    expect(dropped()).toEqual([6]);
  });

  // U4: the candidate's key is the fold's from the moment the closure answered
  // it, and a hand-written `compareTo` may throw. Until the comparison has
  // said which key wins, the candidate's is in neither set — the `catch`
  // releases the accumulator's, the `finally` releases the untouched tail, and
  // neither of them holds this one.
  test('a keyed fold releases the candidate key when the comparison throws', () => {
    const keys: number[] = [];
    class Key extends Drop {
      constructor(readonly n: number) {
        super();
      }
      compareTo(other: Key): number {
        if (other.n === 2 || this.n === 2) throw new Error('no comparison');
        return this.n - other.n;
      }
      protected override onDrop(): void {
        keys.push(this.n);
      }
    }
    const xs = tokens(1, 2, 3);
    expect(() => iterMaxByKeyOwned(xs, (t: Token) => new Key(t.n))).toThrow('no comparison');
    // Both keys made so far are released: the accumulator's by the catch, the
    // candidate's by its own finally.
    expect([...keys].sort((a, b) => a - b)).toEqual([1, 2]);
    // And every element: the two the fold held, and the one it never reached.
    expect(dropped()).toEqual([1, 2, 3]);
  });

  test('the same with borrowed elements, where the fold releases no element', () => {
    const keys: number[] = [];
    class Key extends Drop {
      constructor(readonly n: number) {
        super();
      }
      compareTo(other: Key): number {
        if (other.n === 2 || this.n === 2) throw new Error('no comparison');
        return this.n - other.n;
      }
      protected override onDrop(): void {
        keys.push(this.n);
      }
    }
    const xs = tokens(1, 2, 3);
    expect(() => iterMaxByKey(xs, (t: Token) => new Key(t.n))).toThrow('no comparison');
    expect([...keys].sort((a, b) => a - b)).toEqual([1, 2]);
    // The elements are the caller's here, so none of them is released.
    expect(dropped()).toEqual([]);
    for (const t of xs) t.drop();
  });

  test('next on a sequence nobody else holds answers the head and drops the tail', () => {
    const xs = tokens(1, 2, 3);
    const head = iterFirstOwned(xs);
    expect(head?.n).toBe(1);
    expect(dropped()).toEqual([2, 3]);
    head!.drop();
    expect(iterFirstOwned([])).toBe(null);
  });
});

// ── SeqCursor: an OPAQUE iterator, which is the one shape the array cannot be ──
//
// A generic body that takes `I: Iterator<Item = V>` and calls `next()` is doing
// the one thing the whole-sequence shape cannot express. The cursor holds the
// sequence and the index of the first element it has not handed out; what it
// has handed out belongs to whoever took it, and dropping the cursor releases
// exactly the rest — which is what Rust drops when a part-walked iterator goes
// out of scope.

describe('SeqCursor', () => {
  test('next hands out each element in order and then answers null', () => {
    Token.dropped = [];
    const cursor = new SeqCursor([new Token(1), new Token(2)]);
    const first = cursor.next();
    const second = cursor.next();
    expect(first?.n).toBe(1);
    expect(second?.n).toBe(2);
    expect(cursor.next()).toBe(null);
    expect(Token.dropped).toEqual([]);
    cursor.drop();
    // Nothing was left in the cursor, so dropping it released nothing.
    expect(Token.dropped).toEqual([]);
    first?.drop();
    second?.drop();
    expect(Token.dropped).toEqual([1, 2]);
  });

  test('dropping a part-walked cursor releases only what it still held', () => {
    Token.dropped = [];
    const cursor = new SeqCursor([new Token(1), new Token(2), new Token(3)]);
    const taken = cursor.next();
    expect(taken?.n).toBe(1);
    cursor.drop();
    expect(Token.dropped).toEqual([2, 3]);
    // The one that was handed out is still the caller's.
    taken?.drop();
    expect(Token.dropped).toEqual([2, 3, 1]);
  });

  test('remaining counts what has not been handed out', () => {
    Token.dropped = [];
    const cursor = new SeqCursor([new Token(1), new Token(2)]);
    expect(cursor.remaining).toBe(2);
    cursor.next()?.drop();
    expect(cursor.remaining).toBe(1);
    cursor.drop();
    expect(Token.dropped).toEqual([1, 2]);
  });

  test('takeRest hands the tail over and CONSUMES the cursor', () => {
    Token.dropped = [];
    const cursor = new SeqCursor([new Token(1), new Token(2), new Token(3)]);
    cursor.next()?.drop();
    const rest = cursor.takeRest();
    expect(rest.map((t) => t.n)).toEqual([2, 3]);
    // Every consuming `Iterator` method takes the iterator by value, so the
    // cursor is gone: the frame that declared it must not release it, and
    // touching it again is a use after move.
    expect(cursor.isMoved).toBe(true);
    expect(Token.dropped).toEqual([1]);
    for (const token of rest) token.drop();
    expect(Token.dropped).toEqual([1, 2, 3]);
  });

  test('a cursor over an empty sequence answers null and drops cleanly', () => {
    Token.dropped = [];
    const cursor = new SeqCursor<Token>([]);
    expect(cursor.next()).toBe(null);
    expect(cursor.remaining).toBe(0);
    cursor.drop();
    expect(Token.dropped).toEqual([]);
  });
});
