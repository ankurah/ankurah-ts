// MIRRORS: ankurah/storage/indexeddb-wasm/src/util/require.rs
import { Result, AnyhowError } from '@ankurah/base';
import { Event } from '@ankurah/proto';

export interface WBGRequire<T> {
  require(err: string): Result<T, Error>;
}

export function Result_JsValue_require<T>(self: Result<T, unknown>, err: string): Result<T, Error> {
  const _r0 = self.mapErr((e) => AnyhowError.msg(`${err} - ${extractMessage(e)}`));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  return Result.Ok(_r0.unwrap());
}

export function Option_require<T>(self: T | null, err: string): Result<T, Error> {
  const _m0 = AnyhowError.msg(`${err} is None`);
  const _r1 = (self != null ? Result.Ok(self!) : Result.Err(_m0));
  if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
  return Result.Ok(_r1.unwrap());
}

export function Result_Option_JsValue_require<T>(self: Result<T | null, unknown>, err: string): Result<T, Error> {
  if (self.isOk()) {
    const _v = self.unwrap();
    if (_v != null) {
      const res = _v;
      return Result.Ok(res);
    }
    {
      const _v1 = _v;
      return Result.Err(AnyhowError.msg(`${err} is None`));
    }
  } else {
    const e = self.unwrapErr();
    return Result.Err(AnyhowError.msg(`${err} Err: ${e}`));
  }
}

export function Result_Event_require<T>(self: Result<T, Event>, err: string): Result<T, Error> {
  return self.mapErr((e) => {
    {
      const _v3 = e.target();
      if (_v3 != null) {
        const target = _v3;
        {
          const _v2 = target.dynInto();
          if (_v2.isOk()) {
            const request = _v2.unwrap();
            {
              const _v1 = request.error();
              if (_v1.isOk()) {
                const error = _v1.unwrap();
                {
                  const _v = error;
                  if (_v != null) {
                    const domException = _v;
                    return AnyhowError.msg(`${err}: ${domException.message()} (code: ${domException.code()})`);
                  } else {
                  return AnyhowError.msg(`${err}: Unknown error`);
                }
                }
              } else {
              return AnyhowError.msg(`${err}: No error object`);
            }
            }
          } else {
          return AnyhowError.msg(`${err}: Event type ${e.type()}`);
        }
        }
      } else {
      return AnyhowError.msg(`${err}: No event target`);
    }
    }
  });
}

