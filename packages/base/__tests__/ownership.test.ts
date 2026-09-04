// TS-ONLY: Tests for @ankurah/base ownership primitives
import { describe, test, expect } from 'bun:test';
import {
  Struct, Enum, Result, Drop, DropGuard, Arc, Borrow, BorrowMut,
  Mutex, RefCell, RwLock, AsyncMutex, ThreadLocal, disposeSymbol, clearFatalLatch,
  OwnershipFatal,
} from '../src/index.ts';
import { installOwnershipTestHooks } from '../src/testing.ts';

// ── Test helpers ──

// A fatal poisons the runtime: everything afterwards refuses to run, because a
// host that swallowed the throw must not carry on over corrupted ownership.
// These hooks give the suite a per-test reset, and fail any test that raised a
// fatal and swallowed it rather than letting it poison the tests after it.
installOwnershipTestHooks();

/** Assert a fatal, and clear the latch so the test can keep going. */
function expectFatal(body: () => unknown, message: string): void {
  expect(body).toThrow(message);
  clearFatalLatch();
}

class SimpleStruct extends Struct {
  dropCount = 0;
}

// Custom cleanup goes in onDrop(), which AkObject.drop() calls while the fields
// are still alive — the order Rust runs `impl Drop` in. Nothing overrides drop().
class Inner extends Drop {
  dropCount = 0;
  protected override onDrop(): void { this.dropCount++; }
}

class Owner extends Drop {
  inner: Inner;
  constructor() { super(); this.inner = new Inner(); }
  protected override onDrop(): void { /* custom cleanup */ }
}

class BorrowOwner extends Drop {
  borrowed: Borrow<Inner>;
  constructor(inner: Inner) { super(); this.borrowed = new Borrow(inner); }
  protected override onDrop(): void {}
}

type TestEnumV = {
  Empty: {};
  WithData: { inner: Inner };
  WithPrimitive: { count: number };
};

class TestEnum extends Enum<TestEnumV> {
  static Empty = () => new TestEnum('Empty', {});
  static WithData = (v: TestEnumV['WithData']) => new TestEnum('WithData', v);
  static WithPrimitive = (v: TestEnumV['WithPrimitive']) => new TestEnum('WithPrimitive', v);
}

// ── AkObject ──

describe('AkObject', () => {
  test('auto-cascade drops owned fields', () => {
    const owner = new Owner();
    expect(owner.inner.dropCount).toBe(0);
    owner[disposeSymbol]();
    expect(owner.inner.dropCount).toBe(1);
  });

  test('dropping twice is fatal', () => {
    const owner = new Owner();
    owner[disposeSymbol]();
    expectFatal(() => owner[disposeSymbol](), 'BUG: Owner was dropped twice');
    expect(owner.inner.dropCount).toBe(1);
  });

  test('isDropped reflects state', () => {
    const s = new SimpleStruct();
    expect(s.isDropped).toBe(false);
    expect(s.isMoved).toBe(false);
    s[disposeSymbol]();
    expect(s.isDropped).toBe(true);
  });

  test('assertNotDropped is fatal after drop', () => {
    class Guarded extends Struct {
      check(): void { this.assertNotDropped(); }
    }
    const g = new Guarded();
    g.check(); // should not throw
    g[disposeSymbol]();
    expectFatal(() => g.check(), 'BUG: Guarded was used after being dropped');
  });

  test('onDrop runs while the fields are still alive', () => {
    // Rust runs `impl Drop` before dropping fields, so a cleanup body that reads
    // a field must still find it usable.
    let fieldWasAlive = false;
    class Holder extends Drop {
      child: Arc<Inner>;
      constructor(inner: Inner) { super(); this.child = Arc.new(inner); }
      protected override onDrop(): void { fieldWasAlive = !this.child.value.isDropped; }
    }
    const inner = new Inner();
    new Holder(inner).drop();
    expect(fieldWasAlive).toBe(true);
    expect(inner.dropCount).toBe(1); // and the field is dropped afterwards
  });

  test('fields drop in the order the constructor assigned them', () => {
    // Rust drops a struct's fields in declaration order, so the emitter must
    // assign them in declaration order for this to match.
    class Tracer extends Drop {
      readonly tag: string;
      readonly log: string[];
      constructor(tag: string, log: string[]) { super(); this.tag = tag; this.log = log; }
      protected override onDrop(): void { this.log.push(this.tag); }
    }
    class Holder extends Struct {
      first: Tracer;
      second: Tracer;
      third: Tracer;
      constructor(log: string[]) {
        super();
        this.first = new Tracer('first', log);
        this.second = new Tracer('second', log);
        this.third = new Tracer('third', log);
      }
    }
    const log: string[] = [];
    new Holder(log).drop();
    expect(log).toEqual(['first', 'second', 'third']);
  });

  test('a fatal is an OwnershipFatal, so an emitted catch can rethrow it', () => {
    // Emitted code catches Rust error types. An ownership bug is not one of
    // them, so the catch has to be able to tell them apart and rethrow.
    const s = new Struct();
    s.drop();
    let caught: unknown;
    try {
      s.drop();
    } catch (thrown) {
      caught = thrown;
    }
    clearFatalLatch();
    expect(caught).toBeInstanceOf(OwnershipFatal);
    expect(caught).toBeInstanceOf(Error);
    expect((caught as Error).name).toBe('OwnershipFatal');
  });

  test('a field exposed both ways is dropped once', () => {
    // A subclass that hands back a payload ownedFields() already reaches by
    // walking properties would otherwise drop it twice, which is fatal.
    class DoubleExposed extends Struct {
      payload: Inner;
      constructor(payload: Inner) { super(); this.payload = payload; }
      protected override ownedFields(): unknown[] {
        return [...super.ownedFields(), this.payload];
      }
    }
    const inner = new Inner();
    new DoubleExposed(inner).drop();
    expect(inner.dropCount).toBe(1);
  });

  test('a cleanup body that throws still drops the fields', () => {
    class Angry extends Drop {
      child: Inner;
      constructor(child: Inner) { super(); this.child = child; }
      protected override onDrop(): void { throw new Error('cleanup blew up'); }
    }
    const child = new Inner();
    expect(() => new Angry(child).drop()).toThrow('cleanup blew up');
    expect(child.dropCount).toBe(1);
  });
});

// ── Borrow ──

describe('Borrow', () => {
  test('the cascade steps over a borrow without dropping what it points at', () => {
    const inner = new Inner();
    const owner = new BorrowOwner(inner);
    owner[disposeSymbol]();
    expect(inner.dropCount).toBe(0);
    inner.drop(); // its real owner drops it
  });

  test('value is accessible', () => {
    const inner = new Inner();
    const b = new Borrow(inner);
    expect(b.value).toBe(inner);
    inner.drop();
  });

  test('a borrow is not reported as an unwrapped foreign object', () => {
    // Borrow marks itself nonOwning, so the cascade passes over it in silence
    // rather than warning that it does not know how to release it.
    const warnings: unknown[] = [];
    const realWarn = console.warn;
    console.warn = (...args: unknown[]) => { warnings.push(args); };
    try {
      const inner = new Inner();
      new BorrowOwner(inner)[disposeSymbol]();
      inner.drop();
    } finally {
      console.warn = realWarn;
    }
    expect(warnings).toEqual([]);
  });
});

describe('BorrowMut', () => {
  test('value getter and setter', () => {
    const bm = new BorrowMut(42);
    expect(bm.value).toBe(42);
    bm.value = 99;
    expect(bm.value).toBe(99);
  });

  test('a field holding one is not dropped through', () => {
    const inner = new Inner();
    class Holder extends Struct {
      ref: BorrowMut<Inner>;
      constructor(inner: Inner) { super(); this.ref = new BorrowMut(inner); }
    }
    new Holder(inner)[disposeSymbol]();
    expect(inner.dropCount).toBe(0);
    inner.drop();
  });
});

// ── Arc ──

describe('Arc', () => {
  test('inner drops when last Arc drops', () => {
    const inner = new Inner();
    const a = Arc.new(inner);
    const b = a.clone();
    expect(a.strongCount).toBe(2);

    a.drop();
    expect(inner.dropCount).toBe(0); // b still holds
    expect(b.strongCount).toBe(1);

    b.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('clone increments refcount', () => {
    const a = Arc.new(new Inner());
    expect(a.strongCount).toBe(1);
    const b = a.clone();
    expect(a.strongCount).toBe(2);
    b.drop();
    expect(a.strongCount).toBe(1);
    a.drop();
  });

  test('double drop on the same handle is fatal', () => {
    const inner = new Inner();
    const a = Arc.new(inner);
    a.drop();
    expectFatal(() => a.drop(), 'BUG: Arc<Inner> was dropped twice');
    expect(inner.dropCount).toBe(1);
  });

  test('a released handle is unusable even while clones live', () => {
    // The handle was this scope's owner. Releasing it ends the handle, and in
    // Rust the moved-out binding is simply no longer nameable.
    const inner = new Inner();
    const a = Arc.new(inner);
    const b = a.clone();
    a.drop();

    for (const use of [() => a.value, () => a.clone(), () => a.downgrade(), () => a.strongCount, () => a.asPtr()]) {
      expectFatal(use, 'BUG: Arc<Inner> was used after being moved');
    }
    expect(inner.dropCount).toBe(0); // b is still holding it

    b.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('inner cascade works through Arc', () => {
    const owner = new Owner();
    const a = Arc.new(owner);
    a.drop();
    expect(owner.inner.dropCount).toBe(1);
  });

  test('Arc<Mutex<T>> drops the mutex contents on the last drop', () => {
    const inner = new Inner();
    const a = Arc.new(new Mutex(inner));
    const b = a.clone();

    a.drop();
    expect(inner.dropCount).toBe(0);

    b.drop();
    expect(inner.dropCount).toBe(1);
  });
});

// ── Weak ──

describe('Weak', () => {
  test('upgrade returns Arc when strong refs exist', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    const upgraded = w.upgrade();
    expect(upgraded).not.toBeNull();
    expect(a.strongCount).toBe(2);
    upgraded!.drop();
    a.drop();
    w.drop();
  });

  test('upgrade returns null after all strong refs dropped', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    a.drop();
    expect(w.upgrade()).toBeNull();
    w.drop();
  });

  test('double drop is fatal', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    w.drop();
    expectFatal(() => w.drop(), 'BUG: Weak<Inner> was dropped twice');
    a.drop();
  });

  test('clone tracks the weak count', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    expect(w.weakCount).toBe(1);

    const w2 = w.clone();
    expect(w.weakCount).toBe(2);
    expect(w2.asPtr()).toBe(w.asPtr()); // same allocation

    w2.drop();
    expect(w.weakCount).toBe(1);
    w.drop();
    a.drop();
  });

  test('a dropped Weak is unusable', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    w.drop();
    for (const use of [() => w.upgrade(), () => w.asPtr(), () => w.clone(), () => w.weakCount]) {
      expectFatal(use, 'BUG: Weak<Inner> was used after being dropped');
    }
    a.drop();
  });

  test('the last strong drop lets go of the payload', () => {
    // A Weak must not pin a dropped object graph in memory.
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    a.drop();
    expect(w.upgrade()).toBeNull();
    w.drop();
  });
});

// ── Enum ──

describe('Enum', () => {
  test('match exhaustive', () => {
    const e = TestEnum.Empty();
    const result = e.match({
      Empty: () => 'empty',
      WithData: (v) => `data:${v.inner}`,
      WithPrimitive: (v) => `count:${v.count}`,
    });
    expect(result).toBe('empty');
    e.drop();
  });

  test('match borrows, so the enum is still usable afterwards', () => {
    // The emitter uses match in borrow position, where the Rust source matches
    // on a reference.
    const e = TestEnum.WithPrimitive({ count: 7 });
    expect(e.match({ Empty: () => 0, WithData: () => 0, WithPrimitive: (v) => v.count })).toBe(7);
    expect(e.match({ Empty: () => 0, WithData: () => 0, WithPrimitive: (v) => v.count })).toBe(7);
    expect(e.type).toBe('WithPrimitive');
    e.drop();
  });

  test('a match with a missing arm is fatal', () => {
    const e = TestEnum.Empty();
    expectFatal(
      () => (e as any).match({ WithData: () => 1, WithPrimitive: () => 2 }),
      "BUG: match on TestEnum has no arm for 'Empty'",
    );
    e.drop();
  });

  test('is() narrows type', () => {
    const e = TestEnum.WithPrimitive({ count: 42 });
    expect(e.is('WithPrimitive')).toBe(true);
    expect(e.is('Empty')).toBe(false);
    e.drop();
  });

  test('cascade drops variant value fields', () => {
    const inner = new Inner();
    const e = TestEnum.WithData({ inner });
    e[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('unit and primitive variants cascade harmlessly', () => {
    const empty = TestEnum.Empty();
    empty[disposeSymbol]();
    expect(empty.isDropped).toBe(true);

    const primitive = TestEnum.WithPrimitive({ count: 42 });
    primitive[disposeSymbol]();
    expect(primitive.isDropped).toBe(true);
  });

  test('every accessor is closed after the enum is dropped', () => {
    const e = TestEnum.Empty();
    e.drop();
    for (const use of [() => e.type, () => e.value, () => e.is('Empty'),
                       () => e.match({ Empty: () => 1, WithData: () => 2, WithPrimitive: () => 3 })]) {
      expectFatal(use, 'BUG: TestEnum was used after being dropped');
    }
  });

  test('toString renders the state instead of raising a second fault', () => {
    // Rendering a value is what panic messages and debuggers do, and both run
    // when something has already gone wrong.
    const dropped = TestEnum.Empty();
    dropped.drop();
    expect(dropped.toString()).toBe('TestEnum::Empty (dropped)');

    const moved = Result.Ok<number, string>(1);
    moved.unwrap();
    expect(moved.toString()).toBe('Result::Ok (moved)');

    const alive = TestEnum.Empty();
    expect(alive.toString()).toBe('TestEnum::Empty');
    alive.drop();
  });

  test('toString', () => {
    const e = TestEnum.Empty();
    expect(e.toString()).toBe('TestEnum::Empty');
    e.drop();
  });
});

// ── Result ──
//
// Every Rust Result method that takes `self` consumes it: the payload moves to
// the caller, to a callback, or into the Result the method returns, and the
// original is gone. Methods taking &self borrow and leave it whole.

describe('Result move semantics', () => {
  test('unwrap hands the payload to the caller and moves the Result', () => {
    const payload = new Inner();
    const r = Result.Ok<Inner, string>(payload);

    expect(r.unwrap()).toBe(payload);
    expectFatal(() => r.isOk(), 'BUG: Result was used after being moved');

    payload.drop(); // the caller owns it now
    expect(payload.dropCount).toBe(1);
  });

  test('unwrapErr hands the error out', () => {
    const err = new Inner();
    const r = Result.Err<number, Inner>(err);
    expect(r.unwrapErr()).toBe(err);
    err.drop();
  });

  test('unwrap on an Err names the payload through its own toString', () => {
    const r = Result.Err<number, TestEnum>(TestEnum.Empty());
    expect(() => r.unwrap()).toThrow('called unwrap() on Err: TestEnum::Empty');
  });

  test('a payload that cannot be rendered does not bury the panic', () => {
    // describe() runs on the panic path, so a throwing toString must not
    // replace the error it was called to describe.
    const hostile = { toString(): string { throw new Error('cannot render me'); } };
    expect(() => Result.Err<number, typeof hostile>(hostile).unwrap())
      .toThrow('called unwrap() on Err:');
  });

  test('map moves the payload into the Result it returns', () => {
    const payload = new Inner();
    const mapped = Result.Ok<Inner, string>(payload).map((v) => v);
    expect(payload.dropCount).toBe(0);
    mapped.drop();
    expect(payload.dropCount).toBe(1);
  });

  test('map on an Err carries the error across untouched', () => {
    const err = new Inner();
    const mapped = Result.Err<number, Inner>(err).map((v) => v);
    expect(err.dropCount).toBe(0);
    mapped.drop();
    expect(err.dropCount).toBe(1);
  });

  test('mapErr carries the Ok payload across untouched', () => {
    const payload = new Inner();
    const mapped = Result.Ok<Inner, string>(payload).mapErr((e) => e);
    expect(payload.dropCount).toBe(0);
    mapped.drop();
    expect(payload.dropCount).toBe(1);
  });

  test('a callback that throws drops the payload it was given', () => {
    // Rust's unwind drops the value the callback owned; leaving it unowned here
    // would surface later as a leak nobody can explain.
    const payload = new Inner();
    expect(() => Result.Ok<Inner, string>(payload).map(() => { throw new Error('boom'); }))
      .toThrow('boom');
    expect(payload.dropCount).toBe(1);

    const err = new Inner();
    expect(() => Result.Err<number, Inner>(err).mapErr(() => { throw new Error('boom'); }))
      .toThrow('boom');
    expect(err.dropCount).toBe(1);

    const third = new Inner();
    expect(() => Result.Err<number, Inner>(third).unwrapOrElse(() => { throw new Error('boom'); }))
      .toThrow('boom');
    expect(third.dropCount).toBe(1);
  });

  test('unwrapOr drops the default it did not need', () => {
    const payload = new Inner();
    const unused = new Inner();
    expect(Result.Ok<Inner, string>(payload).unwrapOr(unused)).toBe(payload);
    expect(unused.dropCount).toBe(1); // nobody else owns it
    payload.drop();
  });

  test('unwrapOr drops the discarded error and returns the default', () => {
    const err = new Inner();
    const fallback = new Inner();
    expect(Result.Err<Inner, Inner>(err).unwrapOr(fallback)).toBe(fallback);
    expect(err.dropCount).toBe(1);
    fallback.drop();
  });

  test('unwrapOrElse moves the error into the fallback', () => {
    const err = new Inner();
    const handed: Inner[] = [];
    const replacement = new Inner();

    const got = Result.Err<Inner, Inner>(err).unwrapOrElse((e) => { handed.push(e); return replacement; });

    expect(handed[0]).toBe(err);
    expect(err.dropCount).toBe(0); // the callback owns it now
    err.drop();
    expect(got).toBe(replacement);
    replacement.drop();
  });

  test('expect and expectErr', () => {
    const payload = new Inner();
    expect(Result.Ok<Inner, string>(payload).expect('should be ok')).toBe(payload);
    payload.drop();

    expect(() => Result.Err<number, string>('nope').expect('wanted a number'))
      .toThrow('wanted a number: nope');

    expect(Result.Err<number, string>('boom').expectErr('wanted an error')).toBe('boom');
    expect(() => Result.Ok<number, string>(1).expectErr('wanted an error'))
      .toThrow('wanted an error: 1');
  });

  test('andThen chains, moving the payload into the next Result', () => {
    const payload = new Inner();
    const chained = Result.Ok<Inner, string>(payload).andThen((v) => Result.Ok<Inner, string>(v));
    expect(payload.dropCount).toBe(0);
    chained.drop();
    expect(payload.dropCount).toBe(1);

    const err = Result.Err<Inner, string>('bad').andThen(() => Result.Ok<Inner, string>(new Inner()));
    expect(err.isErr()).toBe(true);
    err.drop();
  });

  test('orElse replaces an Err and passes an Ok through', () => {
    const recovered = Result.Err<number, string>('bad').orElse(() => Result.Ok<number, string>(7));
    expect(recovered.unwrap()).toBe(7);

    const kept = Result.Ok<number, string>(1).orElse(() => Result.Ok<number, string>(9));
    expect(kept.unwrap()).toBe(1);
  });

  test('ok() and err() map to the port Option, which is T | null', () => {
    const payload = new Inner();
    expect(Result.Ok<Inner, string>(payload).ok()).toBe(payload);
    payload.drop();

    const discarded = new Inner();
    expect(Result.Ok<Inner, string>(discarded).err()).toBeNull();
    expect(discarded.dropCount).toBe(1); // err() discarded the Ok payload

    const err = new Inner();
    expect(Result.Err<number, Inner>(err).err()).toBe(err);
    err.drop();

    const dropped = new Inner();
    expect(Result.Err<number, Inner>(dropped).ok()).toBeNull();
    expect(dropped.dropCount).toBe(1);
  });

  test('borrowing accessors leave the Result usable', () => {
    const payload = new Inner();
    const r = Result.Ok<Inner, string>(payload);

    expect(r.isOk()).toBe(true);
    expect(r.isErr()).toBe(false);
    expect(r.isOk()).toBe(true);

    r.drop();
    expect(payload.dropCount).toBe(1);
  });

  test('use after move is fatal for an accessor, a second consume, and drop', () => {
    const payload = new Inner();
    const r = Result.Ok<Inner, string>(payload);
    r.unwrap();

    expectFatal(() => r.isOk(), 'BUG: Result was used after being moved');
    expectFatal(() => r.unwrap(), 'BUG: Result was used after being moved');
    expectFatal(() => r.drop(), 'BUG: Result was used after being moved');

    payload.drop();
  });
});

// ── Containers ──
//
// Dropping a Mutex<T>, RwLock<T> or RefCell<T> in Rust drops the T inside it.
// Each keeps its T in a #private field the owner's cascade cannot see, so the
// container hands it over itself.

describe('Container drop drops its contents', () => {
  const containers: Array<[string, (inner: Inner) => { drop(): void }]> = [
    ['Mutex', (inner) => new Mutex(inner)],
    ['RwLock', (inner) => new RwLock(inner)],
    ['RefCell', (inner) => new RefCell(inner)],
  ];

  for (const [name, make] of containers) {
    test(`${name}.drop() drops the contained object`, () => {
      const inner = new Inner();
      make(inner).drop();
      expect(inner.dropCount).toBe(1);
      expect(inner.isDropped).toBe(true);
    });

    test(`${name} leaves the contained object alone while it lives`, () => {
      const inner = new Inner();
      const container = make(inner);
      expect(inner.dropCount).toBe(0);
      container.drop();
      expect(inner.dropCount).toBe(1);
    });

    test(`${name}.drop() twice is fatal`, () => {
      const inner = new Inner();
      const container = make(inner);
      container.drop();
      expectFatal(() => container.drop(), `BUG: ${name} was dropped twice`);
      expect(inner.dropCount).toBe(1);
    });

    test(`an owning struct's cascade reaches through a ${name} field`, () => {
      class Holder extends Struct {
        field: { drop(): void };
        constructor(inner: Inner) { super(); this.field = make(inner); }
      }
      const inner = new Inner();
      new Holder(inner)[disposeSymbol]();
      expect(inner.dropCount).toBe(1);
    });
  }

  test('contents with nothing to release are simply let go', () => {
    expect(() => new Mutex(42).drop()).not.toThrow();
    expect(() => new RwLock('text').drop()).not.toThrow();
    expect(() => new RefCell({ x: 1 }).drop()).not.toThrow();
  });
});

describe('Container basics', () => {
  test('Mutex: lock, read and write through the guard', () => {
    const m = new Mutex({ x: 1 });
    const first = m.lock();
    expect(first.value.x).toBe(1);
    first.value.x = 2;
    first.drop();

    const second = m.lock();
    expect(second.value.x).toBe(2);
    second.drop();
    m.drop();
  });

  test('Mutex: re-locking throws, which is a deadlock in Rust', () => {
    const m = new Mutex(0);
    const g = m.lock();
    expect(() => m.lock()).toThrow('already locked');
    g.drop();
    m.drop();
  });

  test('RefCell: shared borrows allow multiple readers', () => {
    const cell = new RefCell({ x: 1 });
    const r1 = cell.borrow();
    const r2 = cell.borrow();
    expect(r1.value.x).toBe(1);
    expect(r2.value.x).toBe(1);
    r1.drop();
    r2.drop();
    cell.drop();
  });

  test('RefCell: a mut borrow is exclusive', () => {
    const cell = new RefCell({ x: 1 });
    const w = cell.borrowMut();
    expect(() => cell.borrow()).toThrow('already mutably borrowed');
    expect(() => cell.borrowMut()).toThrow('already mutably borrowed');
    w.drop();

    const r = cell.borrow();
    expect(r.value.x).toBe(1);
    r.drop();
    cell.drop();
  });

  test('RefCell: onMutRelease fires when the borrow is released', () => {
    let released = false;
    const cell = new RefCell({ x: 1 }, { onMutRelease: () => { released = true; } });
    const w = cell.borrowMut();
    expect(released).toBe(false);
    w.drop();
    expect(released).toBe(true);
    cell.drop();
  });

  test('a container refuses to hand out a guard after it is dropped', () => {
    const m = new Mutex({ x: 1 });
    m.drop();
    expectFatal(() => m.lock(), 'BUG: Mutex was used after being dropped');

    const lock = new RwLock({ x: 1 });
    lock.drop();
    expectFatal(() => lock.read(), 'BUG: RwLock was used after being dropped');
    expectFatal(() => lock.write(), 'BUG: RwLock was used after being dropped');

    const cell = new RefCell({ x: 1 });
    cell.drop();
    expectFatal(() => cell.borrow(), 'BUG: RefCell was used after being dropped');
    expectFatal(() => cell.borrowMut(), 'BUG: RefCell was used after being dropped');
  });
});

// ── Dropping a container out from under a guard ──

describe('Container drop with an outstanding guard is fatal', () => {
  const held: Array<[string, string, (inner: Inner) => [{ drop(): void }, { drop(): void }]]> = [
    ['Mutex', 'MutexGuard', (inner) => { const c = new Mutex(inner); return [c, c.lock()]; }],
    ['RwLock', 'RwLockReadGuard', (inner) => { const c = new RwLock(inner); return [c, c.read()]; }],
    ['RwLock', 'RwLockWriteGuard', (inner) => { const c = new RwLock(inner); return [c, c.write()]; }],
    ['RefCell', 'Ref', (inner) => { const c = new RefCell(inner); return [c, c.borrow()]; }],
    ['RefCell', 'RefMut', (inner) => { const c = new RefCell(inner); return [c, c.borrowMut()]; }],
  ];

  for (const [container, guard, take] of held) {
    test(`${container}.drop() holding a ${guard} is fatal, and fine once released`, () => {
      const inner = new Inner();
      const [c, g] = take(inner);

      expectFatal(() => c.drop(), `BUG: ${container} was dropped while a ${guard} is still outstanding.`);
      expect(inner.isDropped).toBe(false); // the refused drop released nothing

      g.drop();
      c.drop();
      expect(inner.dropCount).toBe(1);
    });
  }
});

// ── Guards ──

describe('Guard liveness', () => {
  type Held = [{ drop(): void }, { drop(): void; readonly value: Inner }];
  const guards: Array<[string, (c: Inner) => Held]> = [
    ['MutexGuard on Mutex', (inner) => { const c = new Mutex(inner); return [c, c.lock()]; }],
    ['Ref on RefCell', (inner) => { const c = new RefCell(inner); return [c, c.borrow()]; }],
    ['RefMut on RefCell', (inner) => { const c = new RefCell(inner); return [c, c.borrowMut()]; }],
    ['RwLockReadGuard on RwLock', (inner) => { const c = new RwLock(inner); return [c, c.read()]; }],
    ['RwLockWriteGuard on RwLock', (inner) => { const c = new RwLock(inner); return [c, c.write()]; }],
  ];

  for (const [label, take] of guards) {
    test(`${label}: value is closed after the guard is dropped`, () => {
      const inner = new Inner();
      const [container, g] = take(inner);
      g.drop();
      expectFatal(() => g.value, `BUG: ${label} was used after being dropped`);
      container.drop();
      expect(inner.dropCount).toBe(1);
    });
  }
});

// A guard's drop() releases the borrow it holds. Dropping the same guard twice
// must release once, or the second release corrupts state that now belongs to
// somebody else's guard. Guards are the one type where this is deliberate.

describe('Guard drop is idempotent', () => {
  test('Ref: double drop decrements the shared count once', () => {
    const cell = new RefCell({ x: 1 });
    const r1 = cell.borrow();
    const r2 = cell.borrow();
    r1.drop();
    r1.drop();
    expect(() => cell.borrowMut()).toThrow('already shared-borrowed (count: 1)');
    r2.drop();
    const w = cell.borrowMut();
    expect(w.value.x).toBe(1);
    w.drop();
    cell.drop();
  });

  test('RefMut: double drop fires onMutRelease once', () => {
    let releases = 0;
    const cell = new RefCell({ x: 1 }, { onMutRelease: () => { releases++; } });
    const w = cell.borrowMut();
    w.drop();
    w.drop();
    expect(releases).toBe(1);
    cell.drop();
  });

  test('RwLockReadGuard: double drop decrements the reader count once', () => {
    const lock = new RwLock({ x: 1 });
    const r1 = lock.read();
    const r2 = lock.read();
    r1.drop();
    r1.drop();
    expect(() => lock.write()).toThrow('read locks held');
    r2.drop();
    const w = lock.write();
    expect(w.value.x).toBe(1);
    w.drop();
    lock.drop();
  });

  test('MutexGuard: a stale guard does not release the next holder', () => {
    const m = new Mutex({ x: 1 });
    const g = m.lock();
    g.drop();
    g.drop();
    const g2 = m.lock();
    g.drop(); // the lock now belongs to g2 — this must not clear it
    expect(() => m.lock()).toThrow('already locked');
    g2.drop();
    m.drop();
  });

  test('RwLockWriteGuard: a stale guard does not release the next writer', () => {
    const lock = new RwLock({ x: 1 });
    const w = lock.write();
    w.drop();
    w.drop();
    const w2 = lock.write();
    w.drop(); // the write lock now belongs to w2 — this must not clear it
    expect(() => lock.write()).toThrow('write lock held');
    w2.drop();
    lock.drop();
  });
});

// A Rust guard borrows the value it points at; the container that holds the
// value is its owner and drops it.

describe('Guard drop does not drop the guarded value', () => {
  const acquire: Array<[string, (inner: Inner) => [{ drop(): void }, { drop(): void }]]> = [
    ['Ref', (inner) => { const c = new RefCell(inner); return [c, c.borrow()]; }],
    ['RefMut', (inner) => { const c = new RefCell(inner); return [c, c.borrowMut()]; }],
    ['MutexGuard', (inner) => { const c = new Mutex(inner); return [c, c.lock()]; }],
    ['RwLockReadGuard', (inner) => { const c = new RwLock(inner); return [c, c.read()]; }],
    ['RwLockWriteGuard', (inner) => { const c = new RwLock(inner); return [c, c.write()]; }],
  ];

  for (const [name, take] of acquire) {
    test(`${name}.drop() leaves the guarded object undropped`, () => {
      const inner = new Inner();
      const [container, guard] = take(inner);
      guard.drop();
      expect(inner.dropCount).toBe(0);
      container.drop(); // the owner drops it, not the guard
      expect(inner.dropCount).toBe(1);
    });
  }
});

// ── Assignment through a write guard ──

describe('Assignment through a write guard', () => {
  type Writer = { container: { drop(): void }; take: () => { value: Inner; drop(): void } };
  const writers: Array<[string, (inner: Inner) => Writer]> = [
    ['Mutex', (inner) => { const c = new Mutex(inner); return { container: c, take: () => c.lock() }; }],
    ['RwLock', (inner) => { const c = new RwLock(inner); return { container: c, take: () => c.write() }; }],
    ['RefCell', (inner) => { const c = new RefCell(inner); return { container: c, take: () => c.borrowMut() }; }],
  ];

  for (const [name, open] of writers) {
    test(`${name}: assigning replaces the contents and drops what was there`, () => {
      const original = new Inner();
      const replacement = new Inner();
      const { container, take } = open(original);

      const first = take();
      first.value = replacement;
      expect(original.dropCount).toBe(1); // dropped before the new value is stored
      expect(replacement.dropCount).toBe(0);
      first.drop();

      const second = take();
      expect(second.value).toBe(replacement);
      second.drop();

      container.drop();
      expect(replacement.dropCount).toBe(1);
    });

    test(`${name}: assigning the value already held is fatal`, () => {
      // `*guard = *guard` does not compile for a non-Copy type: that is one
      // value with two owners.
      const inner = new Inner();
      const { container, take } = open(inner);
      const guard = take();

      expectFatal(() => { guard.value = guard.value; }, 'was assigned the value it already holds');
      expect(inner.dropCount).toBe(0);

      guard.drop();
      container.drop();
      expect(inner.dropCount).toBe(1);
    });
  }

  test('re-storing a value with no drop glue is fine', () => {
    // A Copy type cannot implement Drop, so storing one over itself releases
    // nothing and is legal in Rust. The emitter gives Copy types no drop glue,
    // whatever class shape it picks, so that is the test — not "is it a
    // primitive".
    const m = new Mutex(7);
    const g = m.lock();
    g.value = 7;
    expect(g.value).toBe(7);
    g.drop();
    m.drop();

    const record = { n: 1 };
    const cell = new RefCell(record);
    const w = cell.borrowMut();
    w.value = record;
    expect(w.value).toBe(record);
    w.drop();
    cell.drop();
  });

  test('a read guard refuses assignment', () => {
    const cell = new RefCell({ x: 1 });
    const r = cell.borrow();
    expect(() => { (r as unknown as { value: unknown }).value = { x: 9 }; }).toThrow();
    expect(r.value.x).toBe(1);
    r.drop();
    cell.drop();

    const lock = new RwLock({ x: 1 });
    const read = lock.read();
    expect(() => { (read as unknown as { value: unknown }).value = { x: 9 }; }).toThrow();
    expect(read.value.x).toBe(1);
    read.drop();
    lock.drop();
  });

  test('a read guard sees what the container holds now, not a snapshot', () => {
    const lock = new RwLock(1);
    const writer = lock.write();
    writer.value = 2;
    writer.drop();

    const reader = lock.read();
    expect(reader.value).toBe(2);
    expect(reader.deref()).toBe(2);
    reader.drop();
    lock.drop();
  });
});

// ── Cascade depth ──

describe('Cascade depth', () => {
  test('an array nested three deep is dropped all the way down', () => {
    class Holder extends Struct {
      rows: Inner[][][];
      constructor(rows: Inner[][][]) { super(); this.rows = rows; }
    }
    const deepest = new Inner();
    const rows = [[[new Inner(), new Inner()]], [[deepest]]];
    new Holder(rows)[disposeSymbol]();

    for (const plane of rows) for (const row of plane) for (const inner of row) {
      expect(inner.dropCount).toBe(1);
    }
    expect(deepest.isDropped).toBe(true);
  });

  test('a Map drops its keys as well as its values', () => {
    const key = new Inner();
    const value = new Inner();
    new Mutex(new Map([[key, value]])).drop();
    expect(key.dropCount).toBe(1);
    expect(value.dropCount).toBe(1);
  });

  test('a Map of arrays drops every element', () => {
    class Holder extends Struct {
      byKey: Map<string, Inner[]>;
      constructor(byKey: Map<string, Inner[]>) { super(); this.byKey = byKey; }
    }
    const first = new Inner();
    const second = new Inner();
    new Holder(new Map([['a', [first]], ['b', [second]]]))[disposeSymbol]();
    expect(first.dropCount).toBe(1);
    expect(second.dropCount).toBe(1);
  });

  test('a Set drops its members', () => {
    class Holder extends Struct {
      members: Set<Inner>;
      constructor(members: Set<Inner>) { super(); this.members = members; }
    }
    const inner = new Inner();
    new Holder(new Set([inner]))[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('a plain object field is walked like the record it stands for', () => {
    class Holder extends Struct {
      record: { a: Inner; b: Inner[] };
      constructor(record: { a: Inner; b: Inner[] }) { super(); this.record = record; }
    }
    const a = new Inner();
    const b = new Inner();
    new Holder({ a, b: [b] })[disposeSymbol]();
    expect(a.dropCount).toBe(1);
    expect(b.dropCount).toBe(1);
  });

  test('Arc<Mutex<Vec<AkObject>>> drops the elements on the last Arc drop', () => {
    const first = new Inner();
    const second = new Inner();
    const arc = Arc.new(new Mutex([first, second]));
    const clone = arc.clone();

    arc.drop();
    expect(first.dropCount).toBe(0);

    clone.drop();
    expect(first.dropCount).toBe(1);
    expect(second.dropCount).toBe(1);
  });

  test('a container inside a container inside a struct field is reached', () => {
    class Holder extends Struct {
      cell: RefCell<Mutex<Inner>>;
      constructor(inner: Inner) { super(); this.cell = new RefCell(new Mutex(inner)); }
    }
    const inner = new Inner();
    new Holder(inner)[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('an object with no drop glue is reported once, not dropped silently', () => {
    class Foreign { held = new Inner(); }
    class Holder extends Struct {
      foreign: Foreign;
      constructor(foreign: Foreign) { super(); this.foreign = foreign; }
    }
    const warnings: string[] = [];
    const realWarn = console.warn;
    console.warn = (message: string) => { warnings.push(message); };
    try {
      const foreign = new Foreign();
      new Holder(foreign)[disposeSymbol]();
      expect(foreign.held.dropCount).toBe(0); // nothing knew how to release it
      foreign.held.drop();
    } finally {
      console.warn = realWarn;
    }
    expect(warnings.length).toBe(1);
    expect(warnings[0]).toContain('reached a Foreign, which has no drop glue');
  });
});

// ── Composition ──

describe('Composition', () => {
  test('Struct owning Arc with Drop inner — full cascade', () => {
    class MyStruct extends Struct {
      sub: Arc<Inner>;
      constructor(inner: Inner) { super(); this.sub = Arc.new(inner); }
    }
    const inner = new Inner();
    new MyStruct(inner)[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('Struct with Borrow does not drop borrowed', () => {
    class MyStruct extends Struct {
      owned: Inner;
      borrowed: Borrow<Inner>;
      constructor(owned: Inner, borrowed: Inner) {
        super();
        this.owned = owned;
        this.borrowed = new Borrow(borrowed);
      }
    }
    const owned = new Inner();
    const borrowed = new Inner();
    new MyStruct(owned, borrowed)[disposeSymbol]();
    expect(owned.dropCount).toBe(1);
    expect(borrowed.dropCount).toBe(0);
    borrowed.drop();
  });

  test('Nested: Struct A owns Struct B owns Arc<Inner>', () => {
    class B extends Struct {
      arc: Arc<Inner>;
      constructor(inner: Inner) { super(); this.arc = Arc.new(inner); }
    }
    class A extends Struct {
      b: B;
      constructor(inner: Inner) { super(); this.b = new B(inner); }
    }
    const inner = new Inner();
    new A(inner)[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('Multi-owner Arc — dispose one clone, inner survives', () => {
    const inner = new Inner();
    const a1 = Arc.new(inner);
    const a2 = a1.clone();
    class Holder extends Struct {
      ref: Arc<Inner>;
      constructor(arc: Arc<Inner>) { super(); this.ref = arc; }
    }
    new Holder(a1)[disposeSymbol]();
    expect(inner.dropCount).toBe(0); // a2 still alive
    a2.drop();
    expect(inner.dropCount).toBe(1);
  });
});

// ── DropGuard ──

describe('DropGuard', () => {
  test('markDropped and assertNotDropped', () => {
    class Host {
      guard = new DropGuard(this);
      check(): void { this.guard.assertNotDropped(); }
      cleanup(): void { this.guard.markDropped(this); }
    }
    const h = new Host();
    h.check();
    h.cleanup();
    expect(h.guard.isDropped).toBe(true);
    expectFatal(() => h.check(), 'BUG: Host was used after being dropped');
  });

  test('markDropped is idempotent', () => {
    class Host {
      guard = new DropGuard(this);
      cleanup(): void { this.guard.markDropped(this); }
    }
    const h = new Host();
    h.cleanup();
    h.cleanup();
    expect(h.guard.isDropped).toBe(true);
  });
});

// ── AsyncMutex ──

describe('AsyncMutex', () => {
  test('serializes async operations', async () => {
    const m = new AsyncMutex();
    const order: number[] = [];

    const op = async (id: number, delay: number) => {
      const guard = await m.acquire();
      order.push(id);
      await new Promise((r) => setTimeout(r, delay));
      order.push(id * 10);
      guard.drop();
    };

    await Promise.all([op(1, 20), op(2, 10)]);
    expect(order).toEqual([1, 10, 2, 20]);
    m.drop();
  });

  test('a critical section that throws still releases the mutex', async () => {
    // With a bare release closure a throw skipped the release and the mutex
    // deadlocked forever. An emitted finally drops the guard instead.
    const m = new AsyncMutex();
    const guard = await m.acquire();
    try {
      throw new Error('critical section blew up');
    } catch {
      // swallowed on purpose — the emitted finally is what has to save us
    } finally {
      guard.drop();
    }

    const second = await Promise.race([
      m.acquire(),
      new Promise<null>((r) => setTimeout(() => r(null), 250)),
    ]);
    expect(second).not.toBeNull();
    (second as { drop(): void }).drop();
    m.drop();
  });

  test('it owns its contents like the synchronous Mutex', async () => {
    const inner = new Inner();
    const m = new AsyncMutex(inner);

    const guard = await m.acquire();
    expect(guard.value).toBe(inner);
    guard.drop();

    m.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('dropping it while a guard is outstanding is fatal', async () => {
    const inner = new Inner();
    const m = new AsyncMutex(inner);
    const guard = await m.acquire();

    expectFatal(
      () => m.drop(),
      'BUG: AsyncMutex was dropped while a AsyncMutexGuard is still outstanding.',
    );
    expect(inner.isDropped).toBe(false);

    guard.drop();
    m.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('it refuses a second drop, and refuses to hand out a guard afterwards', async () => {
    const m = new AsyncMutex();
    m.drop();
    expectFatal(() => m.drop(), 'BUG: AsyncMutex was dropped twice');
    await expect(m.acquire()).rejects.toThrow('BUG: AsyncMutex was used after being dropped');
    clearFatalLatch();
  });

  test('dropping it with acquirers queued is fatal rather than stranding them', async () => {
    const inner = new Inner();
    const m = new AsyncMutex(inner);

    const held = await m.acquire();
    const first = m.acquire();   // parks on its turn
    const second = m.acquire();  // parks behind it
    held.drop(); // the queue can move now, but neither waiter has resumed yet

    expectFatal(
      () => m.drop(),
      'BUG: AsyncMutex was dropped while a queued acquire() is still outstanding.',
    );
    expect(inner.isDropped).toBe(false);

    (await first).drop();
    (await second).drop();
    m.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('the guard releases once, even if dropped twice', async () => {
    const m = new AsyncMutex();
    const first = await m.acquire();
    first.drop();
    first.drop(); // a guard's second drop is deliberately a no-op

    const second = await Promise.race([
      m.acquire(),
      new Promise<null>((r) => setTimeout(() => r(null), 250)),
    ]);
    expect(second).not.toBeNull(); // the extra drop did not corrupt the queue
    (second as { drop(): void }).drop();
    m.drop();
  });
});

// ── ThreadLocal ──

describe('ThreadLocal', () => {
  test('with() hands the value to the callback', () => {
    const local = new ThreadLocal(41);
    expect(local.with((v) => v + 1)).toBe(42);
  });

  test('holds one value for the life of the module', () => {
    const local = new ThreadLocal<number[]>([]);
    local.with((v) => v.push(1));
    local.with((v) => v.push(2));
    expect(local.with((v) => [...v])).toEqual([1, 2]);
  });

  test('is not leak-tracked, so a module-level static is not a leak', async () => {
    // A thread_local! lives for the whole program. It is reachable from module
    // scope forever, so the registry would never fire for it anyway — but it is
    // also never registered, which is what keeps the contract honest.
    const reports = await leakReportsDuring(() => {
      new LeakProbe();
      new ThreadLocal(1); // abandoned, and still never reported
    });
    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);
    expect(reports.filter((r) => r.startsWith('BUG: ThreadLocal'))).toEqual([]);
  });
});

// ── Leak registry ──

/** Deliberately leaked, to prove the registry actually fired during the window. */
class LeakProbe extends Drop {
  protected override onDrop(): void {}
}

/**
 * Runs `body`, forces a collection, and returns the leak-registry messages
 * raised while it ran.
 *
 * drop_registry.ts reports a fatal leak by throwing from a queueMicrotask
 * callback. Bun's test runner claims 'uncaughtException' first, so the only
 * place to intercept is queueMicrotask itself.
 */
async function leakReportsDuring(body: () => void): Promise<string[]> {
  const reports: string[] = [];
  const realQueueMicrotask = globalThis.queueMicrotask;
  globalThis.queueMicrotask = (cb: () => void) => {
    realQueueMicrotask(() => {
      try { cb(); } catch (e) { reports.push(String((e as Error).message)); }
    });
  };
  try {
    body();
    Bun.gc(true);
    for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  } finally {
    globalThis.queueMicrotask = realQueueMicrotask;
    clearFatalLatch(); // the reports above poisoned the runtime on purpose
  }
  return reports;
}

// Every tracked class, proved twice over in one collection window: an instance
// that was abandoned IS reported, and an otherwise identical instance that was
// dropped is NOT. Asserting only the first would pass while the registry
// reported everything indiscriminately.

class LeakedStruct extends Struct {}
class DroppedStruct extends Struct {}
class LeakedEnum extends Enum<{ V: {} }> {}
class DroppedEnum extends Enum<{ V: {} }> {}
class LeakedResult extends Result<number, string> {}
class DroppedResult extends Result<number, string> {}
class LeakedArcInner extends Struct {}
class DroppedArcInner extends Struct {}
class LeakedWeakInner extends Struct {}
class DroppedWeakInner extends Struct {}

describe('Leak registry', () => {
  test('every tracked class reports an abandoned instance and not a dropped one', async () => {
    const cases: Array<{ kind: string; leaked: string; dropped: string; abandon: () => void; release: () => void }> = [
      {
        kind: 'Struct', leaked: 'LeakedStruct', dropped: 'DroppedStruct',
        abandon: () => { new LeakedStruct(); },
        release: () => { new DroppedStruct().drop(); },
      },
      {
        kind: 'Enum', leaked: 'LeakedEnum', dropped: 'DroppedEnum',
        abandon: () => { new LeakedEnum('V', {}); },
        release: () => { new DroppedEnum('V', {}).drop(); },
      },
      {
        kind: 'Result', leaked: 'LeakedResult', dropped: 'DroppedResult',
        abandon: () => { new LeakedResult('Ok', { _0: 1 }); },
        release: () => { new DroppedResult('Ok', { _0: 1 }).drop(); },
      },
      {
        kind: 'Mutex', leaked: 'LeakedMutex', dropped: 'DroppedMutex',
        abandon: () => { new Mutex(1, 'LeakedMutex'); },
        release: () => { new Mutex(1, 'DroppedMutex').drop(); },
      },
      {
        kind: 'RwLock', leaked: 'LeakedRwLock', dropped: 'DroppedRwLock',
        abandon: () => { new RwLock(1, 'LeakedRwLock'); },
        release: () => { new RwLock(1, 'DroppedRwLock').drop(); },
      },
      {
        kind: 'RefCell', leaked: 'LeakedRefCell', dropped: 'DroppedRefCell',
        abandon: () => { new RefCell(1, { label: 'LeakedRefCell' }); },
        release: () => { new RefCell(1, { label: 'DroppedRefCell' }).drop(); },
      },
      {
        kind: 'MutexGuard', leaked: 'MutexGuard on LeakedLockA', dropped: 'MutexGuard on DroppedLockA',
        abandon: () => { new Mutex(1, 'LeakedLockA').lock(); },
        release: () => { const m = new Mutex(1, 'DroppedLockA'); m.lock().drop(); m.drop(); },
      },
      {
        kind: 'Ref', leaked: 'Ref on LeakedCellA', dropped: 'Ref on DroppedCellA',
        abandon: () => { new RefCell(1, { label: 'LeakedCellA' }).borrow(); },
        release: () => { const c = new RefCell(1, { label: 'DroppedCellA' }); c.borrow().drop(); c.drop(); },
      },
      {
        kind: 'RefMut', leaked: 'RefMut on LeakedCellB', dropped: 'RefMut on DroppedCellB',
        abandon: () => { new RefCell(1, { label: 'LeakedCellB' }).borrowMut(); },
        release: () => { const c = new RefCell(1, { label: 'DroppedCellB' }); c.borrowMut().drop(); c.drop(); },
      },
      {
        kind: 'RwLockReadGuard', leaked: 'RwLockReadGuard on LeakedLockB', dropped: 'RwLockReadGuard on DroppedLockB',
        abandon: () => { new RwLock(1, 'LeakedLockB').read(); },
        release: () => { const l = new RwLock(1, 'DroppedLockB'); l.read().drop(); l.drop(); },
      },
      {
        kind: 'RwLockWriteGuard', leaked: 'RwLockWriteGuard on LeakedLockC', dropped: 'RwLockWriteGuard on DroppedLockC',
        abandon: () => { new RwLock(1, 'LeakedLockC').write(); },
        release: () => { const l = new RwLock(1, 'DroppedLockC'); l.write().drop(); l.drop(); },
      },
      {
        kind: 'AsyncMutex', leaked: 'LeakedAsyncMutex', dropped: 'DroppedAsyncMutex',
        abandon: () => { new AsyncMutex(1, 'LeakedAsyncMutex'); },
        release: () => { new AsyncMutex(1, 'DroppedAsyncMutex').drop(); },
      },
      {
        kind: 'Arc', leaked: 'Arc<LeakedArcInner>', dropped: 'Arc<DroppedArcInner>',
        abandon: () => { Arc.new(new LeakedArcInner()); },
        release: () => { Arc.new(new DroppedArcInner()).drop(); },
      },
      {
        kind: 'Weak', leaked: 'Weak<LeakedWeakInner>', dropped: 'Weak<DroppedWeakInner>',
        abandon: () => { Arc.new(new LeakedWeakInner()).downgrade(); },
        release: () => {
          const a = Arc.new(new DroppedWeakInner());
          a.downgrade().drop();
          a.drop();
        },
      },
    ];

    const reports = await leakReportsDuring(() => {
      new LeakProbe(); // never dropped
      for (const c of cases) {
        c.abandon();
        c.release();
      }
    });

    // Without this the test would pass vacuously whenever the collection or the
    // registry callback did not run at all.
    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);

    const reported = (label: string) => reports.some((r) => r.startsWith(`BUG: ${label} was garbage collected`));
    for (const c of cases) {
      expect([c.kind, reported(c.leaked)]).toEqual([c.kind, true]);
      expect([c.kind, reported(c.dropped)]).toEqual([c.kind, false]);
    }
  });

  test('a moved Result is not reported as a leak', async () => {
    const reports = await leakReportsDuring(() => {
      new LeakProbe();
      const payload = new Inner();
      Result.Ok<Inner, string>(payload).unwrap();
      payload.drop();
    });

    expect(reports.some((r) => r.startsWith('BUG: LeakProbe was'))).toBe(true);
    expect(reports.filter((r) => r.startsWith('BUG: Result was'))).toEqual([]);
  });
});

// ── Private field limitation ──

describe('Private field limitation', () => {
  test('#private fields are invisible to cascade (documented limitation)', () => {
    class WithPrivate extends Struct {
      #secret: Inner;
      public exposed: Inner;
      constructor() {
        super();
        this.#secret = new Inner();
        this.exposed = new Inner();
      }
      getSecret(): Inner { return this.#secret; }
    }
    const w = new WithPrivate();
    const secret = w.getSecret();
    const exposed = w.exposed;
    w[disposeSymbol]();
    expect(exposed.dropCount).toBe(1); // cascade reached public field
    expect(secret.dropCount).toBe(0);  // cascade missed #private field
    secret.drop(); // a type keeping private state overrides ownedFields()
  });
});
