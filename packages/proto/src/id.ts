// MIRRORS: ankurah/proto/src/id.rs
import { Result } from '@ankurah/base';
import { EntityId } from './id.provided';
import { DecodeError } from './error';
import { Expr, Literal } from '@ankurah/ankql';
export { EntityId };

export function String_fromEntityId(id: EntityId): string {
  return id.toBase64();
}

export function Vec_U8_tryInto(self: Uint8Array): Result<EntityId, DecodeError> {
  const _r0 = self.tryInto().mapErr((_) => new DecodeError('InvalidLength', {}));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const bytes = _r0.unwrap();
  return Result.Ok(new EntityId(Ulid.fromBytes(bytes)));
}

export function Ulid_fromEntityId(id: EntityId): Ulid {
  return id._0;
}

export function Expr_fromEntityId(id: EntityId): Expr {
  return new Expr('Literal', { _0: new Literal('EntityId', { _0: id.toUlid() }) });
}

