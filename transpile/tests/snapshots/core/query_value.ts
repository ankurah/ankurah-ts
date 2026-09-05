// MIRRORS: ankurah/core/src/query_value.rs
import { Enum, Result } from '@ankurah/base';
import { EntityId } from '@ankurah/proto';
import { Expr, Literal, ParseError } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

export type QueryValueV = {
  String: { _0: string };
  Int: { _0: bigint };
  Float: { _0: number };
  Bool: { _0: boolean };
  EntityId: { _0: string };
};

export class QueryValue extends Enum<QueryValueV> {

  static fromString(s: string): QueryValue {
    return new QueryValue('String', { _0: s });
  }

  static fromBigint(i: bigint): QueryValue {
    return new QueryValue('Int', { _0: i });
  }

  static fromNumber(i: number): QueryValue {
    return new QueryValue('Int', { _0: BigInt(i) });
  }

  static fromBoolean(b: boolean): QueryValue {
    return new QueryValue('Bool', { _0: b });
  }

  static fromEntityId(id: EntityId): QueryValue {
    return new QueryValue('EntityId', { _0: id.toBase64() });
  }

  clone(): QueryValue {
    return new QueryValue(this.type, { ...this.value });
  }

  debug(): string {
    return this.match({
      String: (v) => `String(${JSON.stringify(v._0)})`,
      Int: (v) => `Int(${String(v._0)})`,
      Float: (v) => `Float(${(($f) => Number.isFinite($f) ? (Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)) : ($f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'))(v._0)})`,
      Bool: (v) => `Bool(${String(v._0)})`,
      EntityId: (v) => `EntityId(${JSON.stringify(v._0)})`,
    });
  }
}

export function Expr_tryFromQueryValue(value: QueryValue): Result<Expr, ParseError> {
  try {
    const _m1 = (() => {
      return value.match<any>({
        String: (v) => {
          const s = v._0;
          return new Expr('Literal', { _0: new Literal('String', { _0: s }) });
        },
        Int: (v) => {
          const i = v._0;
          return new Expr('Literal', { _0: new Literal('I64', { _0: i }) });
        },
        Float: (v) => {
          const f = v._0;
          return new Expr('Literal', { _0: new Literal('F64', { _0: f }) });
        },
        Bool: (v) => {
          const b = v._0;
          return new Expr('Literal', { _0: new Literal('Bool', { _0: b }) });
        },
        EntityId: (v) => {
          const s = v._0;
          const _r0 = EntityId.fromBase64(s).mapErr((e) => new ParseError('InvalidPredicate', { _0: `Invalid EntityId: ${e}` }));
          if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
          const id = _r0.unwrap();
          return new Expr('Literal', { _0: new Literal('EntityId', { _0: id.toUlid() }) });
        },
      });
    })();
    if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
    return Result.Ok((_m1 as any));
  } finally {
    value.drop();
  }
}

