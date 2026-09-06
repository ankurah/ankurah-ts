// MIRRORS: ankurah/ankql/src/ast/json_as_bytes
import { Result, serde_json } from '@ankurah/base';

export function encode<S>(value: unknown, serializer: S): Result<Ok, Error> {
  const _r0 = serde_json.toVec(value).mapErr(serde.ser.Error.custom);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const bytes = _r0.unwrap();
  return bytes.encode(serializer);
}

export function decode<D>(deserializer: D): Result<unknown, Error> {
  const _r0 = Vec.deserialize(deserializer);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const bytes = _r0.unwrap();
  return serde_json.fromSlice(bytes).mapErr(serde.de.Error.custom);
}

