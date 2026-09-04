// MIRRORS: ankurah/proto/src/id.rs
import { Result } from '@ankurah/base';
import { EntityId } from './id.provided';
import { DecodeError } from './error';
export { EntityId };

export function String_fromEntityId(self: string, id: EntityId): string {
  return id.toBase64();
}

export function Vec_U8_tryInto(self: Uint8Array): Result<EntityId, DecodeError> {
  const _r0 = self.tryInto().mapErr((_) => new DecodeError('InvalidLength', {}));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const bytes = _r0.unwrap();
  return Result.Ok(new EntityId(Ulid.fromBytes(bytes)));
}

export function Ulid_fromEntityId(self: Ulid, id: EntityId): Ulid {
  return id._0;
}

export function Expr_fromEntityId(self: Expr, id: EntityId): Expr {
  return ankql.ast.Expr.Literal(ankql.ast.Literal.EntityId(id.toUlid()));
}

