// MIRRORS: ankurah/option_combinators/src/input.rs
import { Struct, Result, checkedAdd, HashMap } from '@ankurah/base';

export class Entry extends Struct {
  readonly weight: number;

  constructor(weight: number) {
    super();
    this.weight = weight;
  }
}

export class Registry extends Struct {
  readonly entries: HashMap<number, Entry>;
  calls: number;

  constructor(entries: HashMap<number, Entry>, calls: number) {
    super();
    this.entries = entries;
    this.calls = calls;
  }

  static new(): Registry {
    return new Registry(new HashMap<number, Entry>(), 0);
  }

  put(id: number, weight: number): void {
    this.entries.set(id, new Entry(weight));
  }

  take(id: number): Result<Entry, string> {
    this.calls = checkedAdd(this.calls, 1, 'u32');
    const _m0 = this.entries.remove(id);
    const _m1 = 'no entry';
    return (_m0 != null ? Result.Ok(_m0!) : Result.Err(_m1));
  }

  weightless(id: number): boolean {
    const _m0 = this.entries.get(id);
    return (_m0 != null ? ((e) => e.weight)(_m0!) : 0) === 0;
  }

  weightOf(id: number): number | null {
    const _m0 = this.entries.get(id);
    return (_m0 != null ? ((e) => e.weight)(_m0!) : null);
  }

  heavyWeight(id: number): number | null {
    const _m0 = this.entries.get(id);
    return (_m0 != null ? ((e) => e.weight > 2 ? e.weight : null)(_m0!) : null);
  }

  isHeavy(id: number): boolean {
    const _m0 = this.entries.get(id);
    return (_m0 != null && ((e) => e.weight > 2)(_m0!));
  }

  weightOrFail(id: number): Result<number, string> {
    const _m0 = this.entries.get(id);
    const _m1 = (_m0 != null ? ((e) => e.weight)(_m0!) : null);
    return (_m1 != null ? Result.Ok(_m1!) : Result.Err((() => `no ${id}`)()));
  }
}

