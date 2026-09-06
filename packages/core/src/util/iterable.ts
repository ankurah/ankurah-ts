// MIRRORS: ankurah/core/src/util/iterable.rs
import { HashSet } from '@ankurah/base';
import { Iter } from './ivec';

export interface Iterable<T> {
  iterable(): Iter;
}

export function iterable<T>(self: T): Iter {
  return once(self);
}

export function HashSet_iterable<T, S extends BuildHasher>(self: HashSet<T>): T[] {
  return new HashSet(self);
}

export function Vec_iterable<T>(self: T[]): T[] {
  return [...(self)];
}

export function Iterable_dispatch_iterable<T>(self: unknown): Iter {
  if (self instanceof HashSet) return HashSet_iterable(self as any);
  if (Array.isArray(self)) return Vec_iterable(self as any);
  return iterable(self as any);
}

