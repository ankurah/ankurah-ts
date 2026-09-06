// MIRRORS: ankurah/core/src/value/mod.rs
import { Enum, decodeUtf8Lossy, Result, JsonError, serde_json, OwnershipFatal, UnsupportedShape, valueEquals, unsupported, debugString } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from '../codec';
import { PropertyError } from '../property/traits';
import { Literal } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';
export * from './cast_predicate';
export { CastError } from './cast';

export type ValueV = {
  I16: { _0: number };
  I32: { _0: number };
  I64: { _0: bigint };
  F64: { _0: number };
  Bool: { _0: boolean };
  String: { _0: string };
  EntityId: { _0: EntityId };
  Object: { _0: Uint8Array };
  Binary: { _0: Uint8Array };
  Json: { _0: unknown };
};

export class Value extends Enum<ValueV> {

  static json<T extends Serialize>(value: T): Result<Value, Error> {
    const _r0 = serde_json.toValue(value);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    return Result.Ok(new Value('Json', { _0: _r0.unwrap() }));
  }

  parseAsJson<T extends DeserializeOwned>(): Result<T, PropertyError> {
    return this.match({
      Json: (v) => {
        const json = v._0;
        const _r0 = unsupported('`serde_json::from_value` answers the type the caller names, and the port reads a JSON value back through that type\'s own `fromJson`; a free function has no way to be told which type this one is');
        return Result.Ok(_r0);
      },
      Object: (v) => {
        const bytes = v._0;
        const _r1 = serde_json.fromSlice(bytes);
        if (_r1.isErr()) return Result.Err(PropertyError.fromError(_r1.unwrapErr()));
        return Result.Ok(_r1.unwrap());
      },
      Binary: (v) => {
        const bytes = v._0;
        const _r2 = serde_json.fromSlice(bytes);
        if (_r2.isErr()) return Result.Err(PropertyError.fromError(_r2.unwrapErr()));
        return Result.Ok(_r2.unwrap());
      },
      String: (v) => {
        const s = v._0;
        const _r3 = serde_json.parse(s);
        if (_r3.isErr()) return Result.Err(PropertyError.fromError(_r3.unwrapErr()));
        return Result.Ok(_r3.unwrap());
      },
      I16: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      I32: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      I64: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      F64: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      Bool: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      EntityId: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
    });
  }

  parseAsString<T extends FromStr>(): Result<T, PropertyError> {
    return this.match({
      String: (v) => {
        const s = v._0;
        return s.parse().mapErr((_) => new PropertyError('InvalidValue', { value: s, ty: any.typeName() }));
      },
      I16: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      I32: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      I64: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      F64: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      Bool: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      EntityId: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      Object: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      Binary: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
      Json: () => {
        const other = this;
        return Result.Err(new PropertyError('InvalidVariant', { given: other.clone(), ty: any.typeName() }));
      },
    });
  }

  extractAtPath(path: string[]): Value | null {
    if (path.length === 0) {
      return this.clone();
    }
    return this.match({
      Json: (v) => {
        const json = v._0;
        let current = json;
        for (const key of path) {
          const _r0 = ((current as Record<string, unknown>)?.[key] ?? null);
          if (_r0 == null) return null;
          current = _r0;
        }
        return jsonValueToValue(current);
      },
      Binary: (v) => {
        const bytes = v._0;
        const _r1 = serde_json.fromSlice(bytes).ok();
        if (_r1 == null) return null;
        const json = _r1;
        let current = json;
        for (const key of path) {
          const _r2 = ((current as Record<string, unknown>)?.[key] ?? null);
          if (_r2 == null) return null;
          current = _r2;
        }
        return jsonValueToValue(current);
      },
      String: (v) => {
        const s = v._0;
        const _r3 = serde_json.parse(s).ok();
        if (_r3 == null) return null;
        const json = _r3;
        let current = json;
        for (const key of path) {
          const _r4 = ((current as Record<string, unknown>)?.[key] ?? null);
          if (_r4 == null) return null;
          current = _r4;
        }
        return jsonValueToValue(current);
      },
      I16: () => null,
      I32: () => null,
      I64: () => null,
      F64: () => null,
      Bool: () => null,
      EntityId: () => null,
      Object: () => null,
    });
  }

  gt(other: Value): boolean {
    return valueEquals(this.partialCompareTo(other), 1);
  }

  ge(other: Value): boolean {
    return ((_v) => {
      if (!(_v != null && ((_v === 1) || (_v === 0)))) return false;
      return true;
    })(this.partialCompareTo(other));
  }

  lt(other: Value): boolean {
    return valueEquals(this.partialCompareTo(other), -1);
  }

  le(other: Value): boolean {
    return ((_v) => {
      if (!(_v != null && ((_v === -1) || (_v === 0)))) return false;
      return true;
    })(this.partialCompareTo(other));
  }

  partialCompareTo(other: Value): number | null {
    const _v = [this, other];
    if ((_v[0].is('I16')) && (_v[1].is('I16'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('I32')) && (_v[1].is('I32'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('I64')) && (_v[1].is('I64'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('F64')) && (_v[1].is('F64'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('Bool')) && (_v[1].is('Bool'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('String')) && (_v[1].is('String'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a, b);
    } else if ((_v[0].is('EntityId')) && (_v[1].is('EntityId'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return a.toBytes().compareTo(b.toBytes());
    } else if ((_v[0].is('Object')) && (_v[1].is('Object'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return a.compareTo(b);
    } else if ((_v[0].is('Binary')) && (_v[1].is('Binary'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return a.compareTo(b);
    } else if ((_v[0].is('Json')) && (_v[1].is('Json'))) {
      const { _0: a } = _v[0].value;
      const { _0: b } = _v[1].value;
      return (($a, $b) => $a < $b ? -1 : $a > $b ? 1 : 0)(a.toString(), b.toString());
    } else {
      return null;
    }
  }

  toString(): string {
    return this.match({
      I16: (v) => {
        const int = v._0;
        return `${String(int)}`;
      },
      I32: (v) => {
        const int = v._0;
        return `${String(int)}`;
      },
      I64: (v) => {
        const int = v._0;
        return `${String(int)}`;
      },
      F64: (v) => {
        const float = v._0;
        return `${(($f) => Number.isFinite($f) ? (Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)) : ($f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'))(float)}`;
      },
      Bool: (v) => {
        const bool = v._0;
        return `${String(bool)}`;
      },
      String: (v) => {
        const string = v._0;
        return `${debugString(string)}`;
      },
      EntityId: (v) => {
        const entityId = v._0;
        return `${entityId}`;
      },
      Object: (v) => {
        const object = v._0;
        return `${`[${Array.from(object).map((e) => String(e)).join(', ')}]`}`;
      },
      Binary: (v) => {
        const binary = v._0;
        return `${`[${Array.from(binary).map((e) => String(e)).join(', ')}]`}`;
      },
      Json: (v) => {
        const json = v._0;
        return `${json}`;
      },
    });
  }

  static fromAstLiteral(literal: Literal): Value {
    try {
      return literal.match({
        I16: (v) => {
          const i = v._0;
          return new Value('I16', { _0: i });
        },
        I32: (v) => {
          const i = v._0;
          return new Value('I32', { _0: i });
        },
        I64: (v) => {
          const i = v._0;
          return new Value('I64', { _0: i });
        },
        F64: (v) => {
          const f = v._0;
          return new Value('F64', { _0: f });
        },
        Bool: (v) => {
          const b = v._0;
          return new Value('Bool', { _0: b });
        },
        String: (v) => {
          const s = v._0;
          return new Value('String', { _0: s });
        },
        EntityId: (v) => {
          const ulid = v._0;
          return new Value('EntityId', { _0: EntityId.fromUlid(ulid) });
        },
        Object: (v) => {
          const object = v._0;
          return new Value('Object', { _0: object });
        },
        Binary: (v) => {
          const binary = v._0;
          return new Value('Binary', { _0: binary });
        },
        Json: (v) => {
          const json = v._0;
          return new Value('Json', { _0: json });
        },
      });
    } finally {
      literal.drop();
    }
  }

  static fromRefAstLiteral(literal: Literal): Value {
    return literal.match({
      I16: (v) => {
        const i = v._0;
        return new Value('I16', { _0: i });
      },
      I32: (v) => {
        const i = v._0;
        return new Value('I32', { _0: i });
      },
      I64: (v) => {
        const i = v._0;
        return new Value('I64', { _0: i });
      },
      F64: (v) => {
        const f = v._0;
        return new Value('F64', { _0: f });
      },
      Bool: (v) => {
        const b = v._0;
        return new Value('Bool', { _0: b });
      },
      String: (v) => {
        const s = v._0;
        return new Value('String', { _0: s });
      },
      EntityId: (v) => {
        const ulid = v._0;
        return new Value('EntityId', { _0: EntityId.fromUlid(ulid) });
      },
      Object: (v) => {
        const object = v._0;
        return new Value('Object', { _0: object.clone() });
      },
      Binary: (v) => {
        const binary = v._0;
        return new Value('Binary', { _0: binary.clone() });
      },
      Json: (v) => {
        const json = v._0;
        return new Value('Json', { _0: structuredClone(json) });
      },
    });
  }

  clone(): Value {
    return this.match({
      I16: (v) => new Value('I16', { _0: v._0 }),
      I32: (v) => new Value('I32', { _0: v._0 }),
      I64: (v) => new Value('I64', { _0: v._0 }),
      F64: (v) => new Value('F64', { _0: v._0 }),
      Bool: (v) => new Value('Bool', { _0: v._0 }),
      String: (v) => new Value('String', { _0: v._0 }),
      EntityId: (v) => new Value('EntityId', { _0: v._0.clone() }),
      Object: (v) => new Value('Object', { _0: new Uint8Array(v._0) }),
      Binary: (v) => new Value('Binary', { _0: new Uint8Array(v._0) }),
      Json: (v) => new Value('Json', { _0: v._0.clone() }),
    });
  }

  equals(other: Value): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'I16': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'I32': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'I64': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'F64': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'Bool': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'String': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'EntityId': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
      case 'Object': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if ((this.value as any)._0[i] !== (other.value as any)._0[i]) return false; } }
        break;
      }
      case 'Binary': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if ((this.value as any)._0[i] !== (other.value as any)._0[i]) return false; } }
        break;
      }
      case 'Json': {
        if (!(this.value as any)._0.equals((other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      I16: (v) => `I16(${String(v._0)})`,
      I32: (v) => `I32(${String(v._0)})`,
      I64: (v) => `I64(${String(v._0)})`,
      F64: (v) => `F64(${(($f) => Number.isFinite($f) ? (Number.isInteger($f) ? (Object.is($f, -0) ? '-0.0' : $f.toFixed(1)) : String($f)) : ($f !== $f ? 'NaN' : $f > 0 ? 'inf' : '-inf'))(v._0)})`,
      Bool: (v) => `Bool(${String(v._0)})`,
      String: (v) => `String(${debugString(v._0)})`,
      EntityId: (v) => `EntityId(${v._0})`,
      Object: (v) => `Object(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Binary: (v) => `Binary(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Json: (v) => `Json(${v._0})`,
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      I16: (v) => {
        writer.writeVariant(0);
        writer.writeI16(v._0);
      },
      I32: (v) => {
        writer.writeVariant(1);
        writer.writeI32(v._0);
      },
      I64: (v) => {
        writer.writeVariant(2);
        writer.writeI64(v._0);
      },
      F64: (v) => {
        writer.writeVariant(3);
        writer.writeF64(v._0);
      },
      Bool: (v) => {
        writer.writeVariant(4);
        writer.writeBool(v._0);
      },
      String: (v) => {
        writer.writeVariant(5);
        writer.writeString(v._0);
      },
      EntityId: (v) => {
        writer.writeVariant(6);
        v._0.encode(writer);
      },
      Object: (v) => {
        writer.writeVariant(7);
        writer.writeByteVec(v._0);
      },
      Binary: (v) => {
        writer.writeVariant(8);
        writer.writeByteVec(v._0);
      },
      Json: (v) => {
        writer.writeVariant(9);
        writer.writeByteVec(new TextEncoder().encode(JSON.stringify(v._0)));
      },
    });
  }

  static decode(reader: BincodeReader): Value {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        const _0 = reader.readI16();
        return new Value('I16', { _0 });
      }
      case 1: {
        const _0 = reader.readI32();
        return new Value('I32', { _0 });
      }
      case 2: {
        const _0 = reader.readI64();
        return new Value('I64', { _0 });
      }
      case 3: {
        const _0 = reader.readF64();
        return new Value('F64', { _0 });
      }
      case 4: {
        const _0 = reader.readBool();
        return new Value('Bool', { _0 });
      }
      case 5: {
        const _0 = reader.readString();
        return new Value('String', { _0 });
      }
      case 6: {
        const _0 = EntityId.decode(reader);
        return new Value('EntityId', { _0 });
      }
      case 7: {
        const _0 = reader.readByteVec();
        return new Value('Object', { _0 });
      }
      case 8: {
        const _0 = reader.readByteVec();
        return new Value('Binary', { _0 });
      }
      case 9: {
        const _0 = serde_json.fromSlice(reader.readByteVec()).unwrap();
        return new Value('Json', { _0 });
      }
      default: throw new Error(`Unknown Value variant: ${variant}`);
    }
  }
}

export type ValueTypeV = {
  I16: {};
  I32: {};
  I64: {};
  F64: {};
  Bool: {};
  String: {};
  EntityId: {};
  Object: {};
  Binary: {};
  Json: {};
};

export class ValueType extends Enum<ValueTypeV> {

  static of(v: Value): ValueType {
    return v.match({
      I16: (v) => new ValueType('I16', {}),
      I32: (v) => new ValueType('I32', {}),
      I64: (v) => new ValueType('I64', {}),
      F64: (v) => new ValueType('F64', {}),
      Bool: (v) => new ValueType('Bool', {}),
      String: (v) => new ValueType('String', {}),
      EntityId: (v) => new ValueType('EntityId', {}),
      Object: (v) => new ValueType('Object', {}),
      Binary: (v) => new ValueType('Binary', {}),
      Json: (v) => new ValueType('Json', {}),
    });
  }

  clone(): ValueType {
    return new ValueType(this.type, { ...this.value });
  }

  equals(other: ValueType): boolean {
    if (this.type !== other.type) return false;
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return String(this.type);
  }

  debug(): string {
    return this.match({
      I16: () => 'I16',
      I32: () => 'I32',
      I64: () => 'I64',
      F64: () => 'F64',
      Bool: () => 'Bool',
      String: () => 'String',
      EntityId: () => 'EntityId',
      Object: () => 'Object',
      Binary: () => 'Binary',
      Json: () => 'Json',
    });
  }

  encode(writer: BincodeWriter): void {
    this.match({
      I16: (v) => {
        writer.writeVariant(0);
      },
      I32: (v) => {
        writer.writeVariant(1);
      },
      I64: (v) => {
        writer.writeVariant(2);
      },
      F64: (v) => {
        writer.writeVariant(3);
      },
      Bool: (v) => {
        writer.writeVariant(4);
      },
      String: (v) => {
        writer.writeVariant(5);
      },
      EntityId: (v) => {
        writer.writeVariant(6);
      },
      Object: (v) => {
        writer.writeVariant(7);
      },
      Binary: (v) => {
        writer.writeVariant(8);
      },
      Json: (v) => {
        writer.writeVariant(9);
      },
    });
  }

  static decode(reader: BincodeReader): ValueType {
    const variant = reader.readVariant();
    switch (variant) {
      case 0: {
        return new ValueType('I16', {});
      }
      case 1: {
        return new ValueType('I32', {});
      }
      case 2: {
        return new ValueType('I64', {});
      }
      case 3: {
        return new ValueType('F64', {});
      }
      case 4: {
        return new ValueType('Bool', {});
      }
      case 5: {
        return new ValueType('String', {});
      }
      case 6: {
        return new ValueType('EntityId', {});
      }
      case 7: {
        return new ValueType('Object', {});
      }
      case 8: {
        return new ValueType('Binary', {});
      }
      case 9: {
        return new ValueType('Json', {});
      }
      default: throw new Error(`Unknown ValueType variant: ${variant}`);
    }
  }

  toJSON(): unknown {
    return this.match<unknown>({
      I16: () => 'I16',
      I32: () => 'I32',
      I64: () => 'I64',
      F64: () => 'F64',
      Bool: () => 'Bool',
      String: () => 'String',
      EntityId: () => 'EntityId',
      Object: () => 'Object',
      Binary: () => 'Binary',
      Json: () => 'Json',
    });
  }

  static fromJson(value: unknown): Result<ValueType, JsonError> {
    try {
      if (typeof value === 'string') {
        switch (value) {
          case 'I16': return Result.Ok(new ValueType('I16', {}));
          case 'I32': return Result.Ok(new ValueType('I32', {}));
          case 'I64': return Result.Ok(new ValueType('I64', {}));
          case 'F64': return Result.Ok(new ValueType('F64', {}));
          case 'Bool': return Result.Ok(new ValueType('Bool', {}));
          case 'String': return Result.Ok(new ValueType('String', {}));
          case 'EntityId': return Result.Ok(new ValueType('EntityId', {}));
          case 'Object': return Result.Ok(new ValueType('Object', {}));
          case 'Binary': return Result.Ok(new ValueType('Binary', {}));
          case 'Json': return Result.Ok(new ValueType('Json', {}));
        }
      }
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected a variant of `ValueType`'));
      }
      const o = value as Record<string, unknown>;
      return Result.Err(JsonError.custom('no variant of `ValueType` matches this JSON'));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

function jsonValueToValue(json: unknown): Value {
  return json.match({
    Null: () => new Value('Json', { _0: null }),
    Bool: (v) => {
      const b = v._0;
      return new Value('Bool', { _0: b });
    },
    Number: (v) => {
      const n = v._0;
      {
        const _v1 = n.asI64();
        if (_v1 != null) {
          const i = _v1;
          return new Value('I64', { _0: i });
        } else {
        const _v = n.asF64();
        if (_v != null) {
          const f = _v;
          return new Value('F64', { _0: f });
        } else {
        return new Value('String', { _0: n.toString() });
      }
      }
      }
    },
    String: (v) => {
      const s = v._0;
      return new Value('String', { _0: s });
    },
    Array: (v) => new Value('Json', { _0: structuredClone(json) }),
    Object: (v) => new Value('Json', { _0: structuredClone(json) }),
  });
}

export function Literal_fromValue(value: Value): Literal {
  try {
    return value.match({
      I16: (v) => {
        const i = v._0;
        return new Literal('I16', { _0: i });
      },
      I32: (v) => {
        const i = v._0;
        return new Literal('I32', { _0: i });
      },
      I64: (v) => {
        const i = v._0;
        return new Literal('I64', { _0: i });
      },
      F64: (v) => {
        const f = v._0;
        return new Literal('F64', { _0: f });
      },
      Bool: (v) => {
        const b = v._0;
        return new Literal('Bool', { _0: b });
      },
      String: (v) => {
        const s = v._0;
        return new Literal('String', { _0: s });
      },
      EntityId: (v) => {
        const entityId = v._0;
        return new Literal('EntityId', { _0: entityId.toUlid() });
      },
      Object: (v) => {
        const bytes = v._0;
        return new Literal('String', { _0: decodeUtf8Lossy(bytes) });
      },
      Binary: (v) => {
        const bytes = v._0;
        return new Literal('String', { _0: decodeUtf8Lossy(bytes) });
      },
      Json: (v) => {
        const json = v._0;
        return new Literal('Json', { _0: json });
      },
    });
  } finally {
    value.drop();
  }
}

export function Literal_fromRefValue(value: Value): Literal {
  return value.match({
    I16: (v) => {
      const i = v._0;
      return new Literal('I16', { _0: i });
    },
    I32: (v) => {
      const i = v._0;
      return new Literal('I32', { _0: i });
    },
    I64: (v) => {
      const i = v._0;
      return new Literal('I64', { _0: i });
    },
    F64: (v) => {
      const f = v._0;
      return new Literal('F64', { _0: f });
    },
    Bool: (v) => {
      const b = v._0;
      return new Literal('Bool', { _0: b });
    },
    String: (v) => {
      const s = v._0;
      return new Literal('String', { _0: s });
    },
    EntityId: (v) => {
      const entityId = v._0;
      return new Literal('EntityId', { _0: entityId.toUlid() });
    },
    Object: (v) => {
      const bytes = v._0;
      return new Literal('String', { _0: decodeUtf8Lossy(bytes) });
    },
    Binary: (v) => {
      const bytes = v._0;
      return new Literal('String', { _0: decodeUtf8Lossy(bytes) });
    },
    Json: (v) => {
      const json = v._0;
      return new Literal('Json', { _0: structuredClone(json) });
    },
  });
}

