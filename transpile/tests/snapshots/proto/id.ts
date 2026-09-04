// MIRRORS: ankurah/proto/src/id.rs
import { Result } from '@ankurah/base';
import { EntityId } from './id.provided';
import { DecodeError } from './error';
export { EntityId };

export function String_from(self: string, id: EntityId): string {
  return id.toBase64();
}

export function Vec_U8_tryInto(self: Uint8Array): Result<EntityId, Error> {
  const _r0 = self.tryInto().mapErr((_) => DecodeError.InvalidLength);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const bytes = _r0.unwrap();
  return Result.Ok(new EntityId(Ulid.fromBytes(bytes)));
}

export function Ulid_from(self: Ulid, id: EntityId): Self {
  return id._0;
}

export function Expr_from(self: Expr, id: EntityId): Expr {
  return ankql.ast.Expr.Literal(ankql.ast.Literal.EntityId(id.toUlid()));
}

