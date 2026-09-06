// MIRRORS: ankurah/storage/sqlite/src/value.rs
import { Enum } from '@ankurah/base';
import { Value } from '@ankurah/core';

export type SqliteValueV = {
  Text: { _0: string };
  Integer: { _0: bigint };
  Real: { _0: number };
  Blob: { _0: Uint8Array };
  Jsonb: { _0: unknown };
  Null: {};
};

export class SqliteValue extends Enum<SqliteValueV> {

  sqliteType(): string {
    return this.match({
      Text: (v) => 'TEXT',
      Integer: (v) => 'INTEGER',
      Real: (v) => 'REAL',
      Blob: (v) => 'BLOB',
      Jsonb: (v) => 'BLOB',
      Null: () => 'TEXT',
    });
  }

  isJsonb(): boolean {
    return this.is('Jsonb');
  }

  asJsonString(): string | null {
    return this.match({
      Jsonb: (v) => {
        const j = v._0;
        return j.toString();
      },
      Text: () => null,
      Integer: () => null,
      Real: () => null,
      Blob: () => null,
      Null: () => null,
    });
  }

  toSql(): Value {
    return this.match({
      Text: (v) => {
        const s = v._0;
        return new rusqlite.types.Value('Text', { _0: s });
      },
      Integer: (v) => {
        const i = v._0;
        return new rusqlite.types.Value('Integer', { _0: i });
      },
      Real: (v) => {
        const f = v._0;
        return new rusqlite.types.Value('Real', { _0: f });
      },
      Blob: (v) => {
        const b = v._0;
        return new rusqlite.types.Value('Blob', { _0: b.clone() });
      },
      Jsonb: (v) => {
        const j = v._0;
        return new rusqlite.types.Value('Text', { _0: j.toString() });
      },
      Null: () => rusqlite.types.Value.Null,
    });
  }

  static fromValue(value: Value): SqliteValue {
    try {
      return value.match({
        String: (v) => {
          const s = v._0;
          return new SqliteValue('Text', { _0: s });
        },
        I16: (v) => {
          const i = v._0;
          return new SqliteValue('Integer', { _0: BigInt(i) });
        },
        I32: (v) => {
          const i = v._0;
          return new SqliteValue('Integer', { _0: BigInt(i) });
        },
        I64: (v) => {
          const i = v._0;
          return new SqliteValue('Integer', { _0: i });
        },
        F64: (v) => {
          const f = v._0;
          return new SqliteValue('Real', { _0: f });
        },
        Bool: (v) => {
          const b = v._0;
          return new SqliteValue('Integer', { _0: (b ? 1 : 0) });
        },
        EntityId: (v) => {
          const id = v._0;
          return new SqliteValue('Text', { _0: id.toBase64() });
        },
        Object: (v) => {
          const bytes = v._0;
          return new SqliteValue('Blob', { _0: bytes });
        },
        Binary: (v) => {
          const bytes = v._0;
          return new SqliteValue('Blob', { _0: bytes });
        },
        Json: (v) => {
          const json = v._0;
          return new SqliteValue('Jsonb', { _0: json });
        },
      });
    } finally {
      value.drop();
    }
  }

  static from(value: Value | null): SqliteValue {
    if (value != null) {
      const v = value;
      return SqliteValue.fromValue(v);
    } else {
      return new SqliteValue('Null', {});
    }
  }

  static fromTypesValue(value: Value): SqliteValue {
    return value.match({
      Null: () => new SqliteValue('Null', {}),
      Integer: (v) => {
        const i = v._0;
        return new SqliteValue('Integer', { _0: i });
      },
      Real: (v) => {
        const f = v._0;
        return new SqliteValue('Real', { _0: f });
      },
      Text: (v) => {
        const s = v._0;
        return new SqliteValue('Text', { _0: s });
      },
      Blob: (v) => {
        const b = v._0;
        return new SqliteValue('Blob', { _0: b });
      },
    });
  }

  clone(): SqliteValue {
    return this.match({
      Text: (v) => new SqliteValue('Text', { _0: v._0 }),
      Integer: (v) => new SqliteValue('Integer', { _0: v._0 }),
      Real: (v) => new SqliteValue('Real', { _0: v._0 }),
      Blob: (v) => new SqliteValue('Blob', { _0: new Uint8Array(v._0) }),
      Jsonb: (v) => new SqliteValue('Jsonb', { _0: v._0.clone() }),
      Null: () => new SqliteValue('Null', {}),
    });
  }

  debug(): string {
    return this.match({
      Text: (v) => `Text(${JSON.stringify(v._0)})`,
      Integer: (v) => `Integer(${String(v._0)})`,
      Real: (v) => `Real(${(($f) => Number.isFinite($f) ? (Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)) : ($f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'))(v._0)})`,
      Blob: (v) => `Blob(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Jsonb: (v) => `Jsonb(${v._0})`,
      Null: () => 'Null',
    });
  }
}

