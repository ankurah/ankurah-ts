// MIRRORS: ankurah/lifted_exit/src/input.rs
import { Struct, Enum, Result, checkedAdd } from '@ankurah/base';

export class Entity extends Struct {
  readonly name: string;

  constructor(name: string) {
    super();
    this.name = name;
  }

  apply(ok: boolean): Result<boolean, ApplyError> {
    if (ok) {
      return Result.Ok(true);
    } else {
      return Result.Err(new ApplyError('Refused', {}));
    }
  }
}

export type ApplyErrorV = {
  Refused: {};
};

export class ApplyError extends Enum<ApplyErrorV> {

  clone(): ApplyError {
    return new ApplyError(this.type, { ...this.value });
  }

  equals(other: ApplyError): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  debug(): string {
    return this.match({
      Refused: () => 'Refused',
    });
  }
}

export type StepV = {
  Skip: {};
  Apply: { _0: boolean };
};

export class Step extends Enum<StepV> {
}

export function commit(entity: Entity, already: boolean, ok: boolean): Result<number, ApplyError> {
  const _m1 = (() => {
    if (already) {
      return true;
    } else {
      const _r0 = entity.apply(ok);
      if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
      return _r0.unwrap();
    }
  })();
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  const applied = (_m1 as any);
  if (applied) {
    return Result.Ok(1);
  } else {
    return Result.Ok(0);
  }
}

export function commitBlock(entity: Entity, ok: boolean): Result<number, ApplyError> {
  const _m1 = (() => {
    {
      const _r0 = entity.apply(ok);
      if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
      const a = _r0.unwrap();
      return a;
    }
  })();
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  const applied = (_m1 as any);
  if (applied) {
    return Result.Ok(1);
  } else {
    return Result.Ok(0);
  }
}

export function commitEarly(entity: Entity, stop: boolean, ok: boolean): Result<number, ApplyError> {
  const _m1 = (() => {
    if (stop) {
      return { $jump: 'return', $value: Result.Ok(7) };
    } else {
      const _r0 = entity.apply(ok);
      if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
      return _r0.unwrap();
    }
  })();
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  const applied = (_m1 as any);
  if (applied) {
    return Result.Ok(1);
  } else {
    return Result.Ok(0);
  }
}

export function run(entity: Entity, step: Step): Result<number, ApplyError> {
  try {
    let count = 0;
    const _m1 = step.match<any>({
      Skip: () => {},
      Apply: (v) => {
        const ok = v._0;
        const _r0 = entity.apply(ok);
        if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
        const applied = _r0.unwrap();
        if (applied) {
          count = checkedAdd(count, 1, 'u32');
        }
      },
    });
    if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
    return Result.Ok(count);
  } finally {
    step.drop();
  }
}

