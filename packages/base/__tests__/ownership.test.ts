// TS-ONLY: Tests for @ankurah/base ownership primitives
import { describe, test, expect } from 'bun:test';
import { AkObject, Struct, Enum, Drop, DropGuard, Arc, Weak, Borrow, BorrowMut, Mutex, RefCell, AsyncMutex, disposeSymbol } from '../src/index.ts';

// ── Test helpers ──

class SimpleStruct extends Struct {
  dropCount = 0;
}

class Inner extends Drop {
  dropCount = 0;
  drop(): void { this.dropCount++; }
}

class Owner extends Drop {
  inner: Inner;
  constructor() { super(); this.inner = new Inner(); }
  drop(): void { /* custom cleanup */ }
}

class BorrowOwner extends Drop {
  borrowed: Borrow<Inner>;
  constructor(inner: Inner) { super(); this.borrowed = new Borrow(inner); }
  drop(): void {}
}

type TestEnumV = {
  Empty: {};
  WithData: { inner: Inner };
  WithPrimitive: { count: number };
};

class TestEnum extends Enum<TestEnumV> {
  static Empty = () => new TestEnum('Empty', {});
  static WithData = (inner: Inner) => new TestEnum('WithData', { inner });
  static WithPrimitive = (count: number) => new TestEnum('WithPrimitive', { count });
}

// ── AkObject ──

describe('AkObject', () => {
  test('auto-cascade drops owned fields', () => {
    const owner = new Owner();
    expect(owner.inner.dropCount).toBe(0);
    owner[disposeSymbol]();
    // Owner.drop() runs, then cascade calls inner[disposeSymbol]() which calls inner.drop()
    expect(owner.inner.dropCount).toBe(1);
  });

  test('dispose is idempotent', () => {
    const owner = new Owner();
    owner[disposeSymbol]();
    owner[disposeSymbol]();
    expect(owner.inner.dropCount).toBe(1);
  });

  test('isDropped reflects state', () => {
    const s = new SimpleStruct();
    expect(s.isDropped).toBe(false);
    s[disposeSymbol]();
    expect(s.isDropped).toBe(true);
  });

  test('assertNotDropped throws after drop', () => {
    class Guarded extends Struct {
      check(): void { this.assertNotDropped(); }
    }
    const g = new Guarded();
    g.check(); // should not throw
    g[disposeSymbol]();
    expect(() => g.check()).toThrow('has already been dropped');
  });
});

// ── Borrow ──

describe('Borrow', () => {
  test('does not cascade drop to borrowed value', () => {
    const inner = new Inner();
    const owner = new BorrowOwner(inner);
    owner[disposeSymbol]();
    // Borrow's [Symbol.dispose] is a no-op — inner should NOT be dropped
    expect(inner.dropCount).toBe(0);
  });

  test('value is accessible', () => {
    const inner = new Inner();
    const b = new Borrow(inner);
    expect(b.value).toBe(inner);
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
    expect(inner.dropCount).toBe(1); // last Arc dropped
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

  test('double drop on same handle is idempotent', () => {
    const inner = new Inner();
    const a = Arc.new(inner);
    a.drop();
    a.drop(); // should not decrement again
    expect(inner.dropCount).toBe(1);
  });

  test('value access throws after drop', () => {
    const a = Arc.new(new Inner());
    a.drop();
    expect(() => a.value).toThrow('has been dropped');
  });

  test('clone throws after drop', () => {
    const a = Arc.new(new Inner());
    a.drop();
    expect(() => a.clone()).toThrow('cannot clone');
  });

  test('inner cascade works through Arc', () => {
    // Owner has an Inner field. Arc wraps Owner. When last Arc drops,
    // Owner's drop glue should cascade to Inner.
    const owner = new Owner();
    const a = Arc.new(owner);
    a.drop();
    expect(owner.inner.dropCount).toBe(1);
  });

  test('using support', () => {
    const inner = new Inner();
    {
      using a = Arc.new(inner);
      expect(a.strongCount).toBe(1);
    }
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
  });

  test('upgrade returns null after all strong refs dropped', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    a.drop();
    expect(w.upgrade()).toBeNull();
  });

  test('double drop is idempotent', () => {
    const a = Arc.new(new Inner());
    const w = a.downgrade();
    w.drop();
    w.drop(); // should not crash
    a.drop();
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
  });

  test('match with data variant', () => {
    const inner = new Inner();
    const e = TestEnum.WithData(inner);
    const result = e.match({
      Empty: () => null,
      WithData: (v) => v.inner,
      WithPrimitive: () => null,
    });
    expect(result).toBe(inner);
  });

  test('is() narrows type', () => {
    const e = TestEnum.WithPrimitive(42);
    expect(e.is('WithPrimitive')).toBe(true);
    expect(e.is('Empty')).toBe(false);
  });

  test('cascade drops variant value fields', () => {
    const inner = new Inner();
    const e = TestEnum.WithData(inner);
    e[disposeSymbol]();
    expect(inner.dropCount).toBe(1);
  });

  test('unit variant cascade is harmless', () => {
    const e = TestEnum.Empty();
    e[disposeSymbol](); // should not throw
    expect(e.isDropped).toBe(true);
  });

  test('primitive variant fields are skipped', () => {
    const e = TestEnum.WithPrimitive(42);
    e[disposeSymbol](); // number has no disposeSymbol — should not throw
    expect(e.isDropped).toBe(true);
  });

  test('toString', () => {
    const e = TestEnum.Empty();
    expect(e.toString()).toBe('TestEnum::Empty');
  });
});

// ── Mutex ──

describe('Mutex', () => {
  test('lock and access value', () => {
    const m = new Mutex({ x: 1 });
    {
      using guard = m.lock();
      expect(guard.value.x).toBe(1);
      guard.value.x = 2;
    }
    {
      using guard = m.lock();
      expect(guard.value.x).toBe(2);
    }
  });

  test('double lock throws', () => {
    const m = new Mutex(0);
    const g = m.lock();
    expect(() => m.lock()).toThrow('already locked');
    g[disposeSymbol]();
  });
});

// ── RefCell ──

describe('RefCell', () => {
  test('shared borrows allow multiple readers', () => {
    const cell = new RefCell({ x: 1 });
    {
      using r1 = cell.borrow();
      using r2 = cell.borrow();
      expect(r1.value.x).toBe(1);
      expect(r2.value.x).toBe(1);
    }
  });

  test('mut borrow is exclusive', () => {
    const cell = new RefCell({ x: 1 });
    {
      using w = cell.borrow_mut();
      expect(() => cell.borrow()).toThrow();
      expect(() => cell.borrow_mut()).toThrow();
    }
    // After release, borrow works again
    {
      using r = cell.borrow();
      expect(r.value.x).toBe(1);
    }
  });
});

// ── Composition ──

describe('Composition', () => {
  test('Struct owning Arc with Drop inner — full cascade', () => {
    class MyStruct extends Struct {
      sub: Arc<Inner>;
      constructor(inner: Inner) {
        super();
        this.sub = Arc.new(inner);
      }
    }
    const inner = new Inner();
    const s = new MyStruct(inner);
    s[disposeSymbol]();
    // Cascade: MyStruct dispose → sub[disposeSymbol]() → Arc.drop() → Inner drop glue
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
    const s = new MyStruct(owned, borrowed);
    s[disposeSymbol]();
    expect(owned.dropCount).toBe(1);
    expect(borrowed.dropCount).toBe(0);
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
    const a = new A(inner);
    a[disposeSymbol]();
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
    const h1 = new Holder(a1);
    h1[disposeSymbol]();
    expect(inner.dropCount).toBe(0); // a2 still alive
    a2.drop();
    expect(inner.dropCount).toBe(1);
  });

  test('Arc wrapping non-Drop AkObject cascades fields', () => {
    class Plain extends Struct {
      child: Inner;
      constructor() { super(); this.child = new Inner(); }
    }
    const p = new Plain();
    const childRef = p.child;
    const arc = Arc.new(p);
    arc.drop();
    expect(p.isDropped).toBe(true);
    expect(childRef.dropCount).toBe(1);
  });
});

// ── BorrowMut ──

describe('BorrowMut', () => {
  test('value getter and setter', () => {
    const bm = new BorrowMut(42);
    expect(bm.value).toBe(42);
    bm.value = 99;
    expect(bm.value).toBe(99);
  });

  test('dispose is no-op — does not propagate', () => {
    const inner = new Inner();
    class Holder extends Struct {
      ref: BorrowMut<Inner>;
      constructor(inner: Inner) { super(); this.ref = new BorrowMut(inner); }
    }
    const h = new Holder(inner);
    h[disposeSymbol]();
    expect(inner.dropCount).toBe(0);
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
    h.check(); // should not throw
    h.cleanup();
    expect(h.guard.isDropped).toBe(true);
    expect(() => h.check()).toThrow('has already been dropped');
  });

  test('markDropped is idempotent', () => {
    class Host {
      guard = new DropGuard(this);
      cleanup(): void { this.guard.markDropped(this); }
    }
    const h = new Host();
    h.cleanup();
    h.cleanup(); // should not throw
    expect(h.guard.isDropped).toBe(true);
  });
});

// ── AsyncMutex ──

describe('AsyncMutex', () => {
  test('acquire and release', async () => {
    const m = new AsyncMutex();
    const release = await m.acquire();
    release();
  });

  test('serializes async operations', async () => {
    const m = new AsyncMutex();
    const order: number[] = [];

    const op = async (id: number, delay: number) => {
      const release = await m.acquire();
      order.push(id);
      await new Promise(r => setTimeout(r, delay));
      order.push(id * 10);
      release();
    };

    await Promise.all([op(1, 20), op(2, 10)]);
    // op1 acquires first, runs to completion (1, 10), then op2 runs (2, 20)
    expect(order).toEqual([1, 10, 2, 20]);
  });

  test('re-acquire after release', async () => {
    const m = new AsyncMutex();
    const r1 = await m.acquire();
    r1();
    const r2 = await m.acquire();
    r2();
  });
});

// ── Guard post-dispose ──

describe('Guard post-dispose throws', () => {
  test('MutexGuard.value throws after dispose', () => {
    const m = new Mutex({ x: 1 });
    const g = m.lock();
    g[disposeSymbol]();
    expect(() => g.value).toThrow('has already been dropped');
  });

  test('Ref.value throws after dispose', () => {
    const cell = new RefCell({ x: 1 });
    const r = cell.borrow();
    r[disposeSymbol]();
    expect(() => r.value).toThrow('has already been dropped');
  });

  test('RefMut.value throws after dispose', () => {
    const cell = new RefCell({ x: 1 });
    const w = cell.borrow_mut();
    w[disposeSymbol]();
    expect(() => w.value).toThrow('has already been dropped');
  });
});

// ── RefCell extras ──

describe('RefCell extras', () => {
  test('onMutRelease callback fires after borrow_mut release', () => {
    let released = false;
    const cell = new RefCell({ x: 1 }, { onMutRelease: () => { released = true; } });
    {
      using w = cell.borrow_mut();
      expect(released).toBe(false);
    }
    expect(released).toBe(true);
  });

  test('borrow_mut after borrow release works', () => {
    const cell = new RefCell({ x: 1 });
    { using r = cell.borrow(); }
    { using w = cell.borrow_mut(); w.value.x = 2; }
    { using r = cell.borrow(); expect(r.value.x).toBe(2); }
  });
});

// ── Weak lifecycle ──

describe('Weak lifecycle', () => {
  test('upgrade, use, drop — full pattern', () => {
    const inner = new Inner();
    const arc = Arc.new(inner);
    const weak = arc.downgrade();

    // Upgrade while alive
    const upgraded = weak.upgrade();
    expect(upgraded).not.toBeNull();
    expect(upgraded!.value).toBe(inner);
    expect(arc.strongCount).toBe(2);

    // Drop original
    arc.drop();
    expect(inner.dropCount).toBe(0); // upgraded still holds

    // Drop upgraded
    upgraded!.drop();
    expect(inner.dropCount).toBe(1); // last strong ref gone

    // Weak upgrade returns null
    expect(weak.upgrade()).toBeNull();
    weak.drop();
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
  });
});
