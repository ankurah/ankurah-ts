// MIRRORS: ankurah/proto/src/clock.rs
import { Result, dropOwned } from '@ankurah/base';
import { Clock } from './clock.provided';
import { EventId } from './data';
import { DecodeError } from './error';
export { Clock };

export function Vec_Vec_U8_tryInto(self: Uint8Array[]): Result<Clock, Error> {
  let _moved0 = false;
  const ids = [];
  try {
    for (const idBytes of self) {
      const _r1 = idBytes.tryInto().mapErr((_) => DecodeError.InvalidLength);
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      const bytes = _r1.unwrap();
      let _moved2 = false;
      const id = EventId.fromBytes(bytes);
      try {
        _moved2 = true;
        ids.push(id);
      } finally {
        if (!_moved2) id.drop();
      }
    }
    _moved0 = true;
    return Result.Ok(new Clock([...ids]));
  } finally {
    if (!_moved0) dropOwned(ids);
  }
}

export function Vec_EventId_from(self: EventId[], clock: Clock): Self {
  return clock._0.toVec();
}

