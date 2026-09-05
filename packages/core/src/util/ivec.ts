// MIRRORS: ankurah/core/src/util/ivec.rs
//
// A growable element buffer. `add` is the operation it exists for: push a value
// only if the buffer does not already hold an equal one, which is what the
// reactor's per-query offset lists need and what a HashSet would cost too much
// for at these sizes.
//
// THE DIVERGENCE THIS FILE IS PROVIDED FOR. Rust writes that buffer as
// `enum IVec<T, const N: usize> { Small { data: [MaybeUninit<T>; N], len }, Large(Vec<T>) }`
// — the first N elements live inline in an uninitialised array written through
// raw pointers, and the N+1st push copies them out into a Vec. Eight `unsafe`
// blocks carry that: `assume_init_read`, `assume_init_ref`, `assume_init_drop`
// and a `from_raw_parts` slice. None of it survives: a JavaScript array is
// already a growable buffer, so transpiling the pointer arithmetic would turn it
// into ordinary array writes and quietly claim to be doing something it is not.
//
// What is dropped with the mechanism, and why nothing observes it:
//   - the const generic N and the Small/Large split. Every public method — len,
//     is_empty, push, iter, as_slice, contains, add, clone — answers identically
//     in both variants, so no caller can tell which one it holds. The Rust tests
//     assert the variant with `matches!`, and that is the only place it is read.
//   - `#[derive(Debug)]`, which would print `Small { data: .., len: 3 }`. debug()
//     below prints what this port actually holds instead of naming a variant that
//     does not exist here.
//
// What is kept exactly: `impl Drop` — the buffer owns its elements and releasing
// it releases them, which the cascade does by walking `data`. `add` drops the
// value it refuses, because Rust's `false` branch takes it by value and lets it
// fall out of scope.

import { Struct, dropOwned } from '@ankurah/base';

/** `PartialEq` for an element: the port spells it `equals()`, primitives use `===`. */
function elementsEqual<T>(a: T, b: T): boolean {
  const lhs = a as { equals?: (other: T) => boolean } | null;
  return typeof lhs?.equals === 'function' ? lhs.equals(b) : a === b;
}

/** `Clone` for an element: a ported value clones itself, a primitive is its own copy. */
function cloneElement<T>(value: T): T {
  const v = value as { clone?: () => T } | null;
  return typeof v?.clone === 'function' ? v.clone() : value;
}

export class IVec<T> extends Struct {
  /** The elements, owned. The drop cascade walks this and releases each one,
   *  which is `impl Drop for IVec`. */
  readonly data: T[] = [];

  /** Rust: `IVec::new` */
  static new<T>(): IVec<T> {
    return new IVec<T>();
  }

  /** Rust: `impl Default for IVec` */
  static default<T>(): IVec<T> {
    return IVec.new<T>();
  }

  /** Rust: `len` */
  len(): number {
    return this.data.length;
  }

  /** Rust: `is_empty` */
  isEmpty(): boolean {
    return this.len() === 0;
  }

  /** Rust: `push` */
  push(value: T): void {
    this.data.push(value);
  }

  /** Rust: `iter` — the elements as borrows. Dropping what this hands back would
   *  release values the buffer still owns. */
  iter(): readonly T[] {
    return this.data;
  }

  /** Rust: `as_slice` — a borrow of the whole buffer, with the same caveat. */
  asSlice(): readonly T[] {
    return this.data;
  }

  [Symbol.iterator](): Iterator<T> {
    return this.data[Symbol.iterator]();
  }

  /** Rust: `contains` */
  contains(value: T): boolean {
    return this.data.some((held) => elementsEqual(held, value));
  }

  /** Rust: `add` — push unless an equal element is already held. The refused
   *  value is dropped: Rust takes it by value and never gives it back. */
  add(value: T): boolean {
    if (this.contains(value)) {
      dropOwned(value);
      return false;
    }
    this.push(value);
    return true;
  }

  /** Rust: `impl Clone for IVec<T: Clone>` */
  clone(): IVec<T> {
    const copy = IVec.new<T>();
    for (const value of this.data) copy.push(cloneElement(value));
    return copy;
  }

  debug(): string {
    return `IVec [${this.data.map((v) => String(v)).join(', ')}]`;
  }
}
