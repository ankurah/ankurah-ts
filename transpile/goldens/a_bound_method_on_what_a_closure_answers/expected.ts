// MIRRORS: ankurah/a_bound_method_on_what_a_closure_answers/src/input.rs
import { invokeRef, Invocable, dropOwned } from '@ankurah/base';

export interface Check {
  met(): boolean;
}

export function anyMet<R>(values: bigint[], predicate: Invocable<[bigint], R>): boolean {
  try {
    for (const value of values) {
      if (Check_dispatch_met(invokeRef(predicate, value))) {
        return true;
      }
    }
    return false;
  } finally {
    dropOwned(predicate);
  }
}

export function Bool_met(self: boolean): boolean {
  return self;
}

export function Option_met<T>(self: T | null): boolean {
  return (self != null);
}

export function Check_dispatch_met(self: unknown): boolean {
  if (typeof self === 'boolean') return Bool_met(self as any);
  return Option_met(self as any);
}

