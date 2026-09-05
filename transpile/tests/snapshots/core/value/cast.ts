// MIRRORS: ankurah/core/src/value/cast.rs
import { Enum, Result } from '@ankurah/base';
import { EntityId } from '@ankurah/proto';
import { PropertyError } from '../property/traits';
import { Json } from '../property/value/json';
import { Error } from '../selection/filter';
import { Value, ValueType } from './index';

export type CastErrorV = {
  IncompatibleTypes: { from: ValueType; to: ValueType };
  InvalidFormat: { value: string; targetType: ValueType };
  NumericOverflow: { value: string; targetType: ValueType };
};

export class CastError extends Enum<CastErrorV> {

  toString(): string {
    return this.match({
      IncompatibleTypes: (v) => {
        const from = v.from;
        const to = v.to;
        return `Cannot cast from ${from.debug()} to ${to.debug()}`;
      },
      InvalidFormat: (v) => {
        const value = v.value;
        const targetType = v.targetType;
        return `Invalid format '${value}' for type ${targetType.debug()}`;
      },
      NumericOverflow: (v) => {
        const value = v.value;
        const targetType = v.targetType;
        return `Numeric overflow: '${value}' cannot fit in ${targetType.debug()}`;
      },
    });
  }

  clone(): CastError {
    return this.match({
      IncompatibleTypes: (v) => new CastError('IncompatibleTypes', { from: v.from.clone(), to: v.to.clone() }),
      InvalidFormat: (v) => new CastError('InvalidFormat', { value: v.value, targetType: v.targetType.clone() }),
      NumericOverflow: (v) => new CastError('NumericOverflow', { value: v.value, targetType: v.targetType.clone() }),
    });
  }

  equals(other: CastError): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'IncompatibleTypes': {
        if (!(this.value as any).from.equals((other.value as any).from)) return false;
        if (!(this.value as any).to.equals((other.value as any).to)) return false;
        break;
      }
      case 'InvalidFormat': {
        if ((this.value as any).value !== (other.value as any).value) return false;
        if (!(this.value as any).targetType.equals((other.value as any).targetType)) return false;
        break;
      }
      case 'NumericOverflow': {
        if ((this.value as any).value !== (other.value as any).value) return false;
        if (!(this.value as any).targetType.equals((other.value as any).targetType)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      IncompatibleTypes: (v) => `IncompatibleTypes { from: ${v.from.debug()}, to: ${v.to.debug()} }`,
      InvalidFormat: (v) => `InvalidFormat { value: ${JSON.stringify(v.value)}, targetType: ${v.targetType.debug()} }`,
      NumericOverflow: (v) => `NumericOverflow { value: ${JSON.stringify(v.value)}, targetType: ${v.targetType.debug()} }`,
    });
  }
}

export function PropertyError_fromCastError(err: CastError): PropertyError {
  return new PropertyError('CastError', { _0: err });
}

export function Value_castTo(self: Value, targetType: ValueType): Result<Value, CastError> {
  const sourceType = ValueType.of(self);
  if (sourceType.equals(targetType)) {
    return Result.Ok(self.clone());
  }
  const _v = [self, targetType];
  if ((_v[0].is('String')) && (_v[1].is('EntityId'))) {
    const { _0: s } = _v[0].value;
    const _v1 = EntityId.fromBase64(s);
    if (_v1.isOk()) {
      const entityId = _v1.unwrap();
      return Result.Ok(new Value('EntityId', { _0: entityId }));
    } else {
      const _v2 = _v1.unwrapErr();
      try {
        return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('EntityId', {}) }));
      } finally {
        _v2.drop();
      }
    }
  } else if ((_v[0].is('EntityId')) && (_v[1].is('String'))) {
    const { _0: entityId } = _v[0].value;
    return Result.Ok(new Value('String', { _0: entityId.toBase64() }));
  } else if ((_v[0].is('I16')) && (_v[1].is('I32'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('I32', { _0: n }));
  } else if ((_v[0].is('I16')) && (_v[1].is('I64'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('I64', { _0: BigInt(n) }));
  } else if ((_v[0].is('I16')) && (_v[1].is('F64'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('F64', { _0: n }));
  } else if ((_v[0].is('I32')) && (_v[1].is('I16'))) {
    const { _0: n } = _v[0].value;
    if (n >= -32768 && n <= 32767) {
      return Result.Ok(new Value('I16', { _0: ((n << 16) >> 16) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I16', {}) }));
    }
  } else if ((_v[0].is('I32')) && (_v[1].is('I64'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('I64', { _0: BigInt(n) }));
  } else if ((_v[0].is('I32')) && (_v[1].is('F64'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('F64', { _0: n }));
  } else if ((_v[0].is('I64')) && (_v[1].is('I16'))) {
    const { _0: n } = _v[0].value;
    if (n >= BigInt(-32768) && n <= BigInt(32767)) {
      return Result.Ok(new Value('I16', { _0: Number(BigInt.asIntN(16, n)) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I16', {}) }));
    }
  } else if ((_v[0].is('I64')) && (_v[1].is('I32'))) {
    const { _0: n } = _v[0].value;
    if (n >= BigInt(-2147483648) && n <= BigInt(2147483647)) {
      return Result.Ok(new Value('I32', { _0: Number(BigInt.asIntN(32, n)) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I32', {}) }));
    }
  } else if ((_v[0].is('I64')) && (_v[1].is('F64'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('F64', { _0: Number(n) }));
  } else if ((_v[0].is('F64')) && (_v[1].is('I16'))) {
    const { _0: n } = _v[0].value;
    if (Number.isFinite(n) && n >= -32768 && n <= 32767) {
      return Result.Ok(new Value('I16', { _0: Math.min(Math.max(Math.trunc(n) || 0, -32768), 32767) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I16', {}) }));
    }
  } else if ((_v[0].is('F64')) && (_v[1].is('I32'))) {
    const { _0: n } = _v[0].value;
    if (Number.isFinite(n) && n >= -2147483648 && n <= 2147483647) {
      return Result.Ok(new Value('I32', { _0: Math.min(Math.max(Math.trunc(n) || 0, -2147483648), 2147483647) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I32', {}) }));
    }
  } else if ((_v[0].is('F64')) && (_v[1].is('I64'))) {
    const { _0: n } = _v[0].value;
    if (Number.isFinite(n) && n >= Number(-9223372036854775808n) && n <= Number(9223372036854775807n)) {
      return Result.Ok(new Value('I64', { _0: (($v) => $v < -9223372036854775808n ? -9223372036854775808n : $v > 9223372036854775807n ? 9223372036854775807n : $v)(BigInt(Math.min(Math.max(Math.trunc(n) || 0, -9223372036854775808), 9223372036854775807))) }));
    } else {
      return Result.Err(new CastError('NumericOverflow', { value: n.toString(), targetType: new ValueType('I64', {}) }));
    }
  } else if ((_v[0].is('String')) && (_v[1].is('I16'))) {
    const { _0: s } = _v[0].value;
    const _v3 = s.parse();
    if (_v3.isOk()) {
      const n = _v3.unwrap();
      return Result.Ok(new Value('I16', { _0: n }));
    } else {
      const _v4 = _v3.unwrapErr();
      return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('I16', {}) }));
    }
  } else if ((_v[0].is('String')) && (_v[1].is('I32'))) {
    const { _0: s } = _v[0].value;
    const _v5 = s.parse();
    if (_v5.isOk()) {
      const n = _v5.unwrap();
      return Result.Ok(new Value('I32', { _0: n }));
    } else {
      const _v6 = _v5.unwrapErr();
      return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('I32', {}) }));
    }
  } else if ((_v[0].is('String')) && (_v[1].is('I64'))) {
    const { _0: s } = _v[0].value;
    const _v7 = s.parse();
    if (_v7.isOk()) {
      const n = _v7.unwrap();
      return Result.Ok(new Value('I64', { _0: n }));
    } else {
      const _v8 = _v7.unwrapErr();
      return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('I64', {}) }));
    }
  } else if ((_v[0].is('String')) && (_v[1].is('F64'))) {
    const { _0: s } = _v[0].value;
    const _v9 = s.parse();
    if (_v9.isOk()) {
      const n = _v9.unwrap();
      return Result.Ok(new Value('F64', { _0: n }));
    } else {
      const _v10 = _v9.unwrapErr();
      return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('F64', {}) }));
    }
  } else if ((_v[0].is('String')) && (_v[1].is('Bool'))) {
    const { _0: s } = _v[0].value;
    const _v11 = s.toLowerCase();
    if ((_v11 === 'true') || (_v11 === '1') || (_v11 === 'yes') || (_v11 === 'on')) {
      return Result.Ok(new Value('Bool', { _0: true }));
    } else if ((_v11 === 'false') || (_v11 === '0') || (_v11 === 'no') || (_v11 === 'off')) {
      return Result.Ok(new Value('Bool', { _0: false }));
    } else {
      return Result.Err(new CastError('InvalidFormat', { value: s, targetType: new ValueType('Bool', {}) }));
    }
  } else if ((_v[0].is('I16')) && (_v[1].is('String'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('String', { _0: n.toString() }));
  } else if ((_v[0].is('I32')) && (_v[1].is('String'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('String', { _0: n.toString() }));
  } else if ((_v[0].is('I64')) && (_v[1].is('String'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('String', { _0: n.toString() }));
  } else if ((_v[0].is('F64')) && (_v[1].is('String'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('String', { _0: n.toString() }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('String'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('String', { _0: b.toString() }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('I16'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('I16', { _0: (b ? 1 : 0) }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('I32'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('I32', { _0: (b ? 1 : 0) }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('I64'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('I64', { _0: (b ? 1 : 0) }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('F64'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('F64', { _0: (b ? 1.0 : 0.0) }));
  } else if ((_v[0].is('I16')) && (_v[1].is('Bool'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Bool', { _0: n !== 0 }));
  } else if ((_v[0].is('I32')) && (_v[1].is('Bool'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Bool', { _0: n !== 0 }));
  } else if ((_v[0].is('I64')) && (_v[1].is('Bool'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Bool', { _0: n !== 0n }));
  } else if ((_v[0].is('F64')) && (_v[1].is('Bool'))) {
    const { _0: f } = _v[0].value;
    return Result.Ok(new Value('Bool', { _0: f !== 0.0 }));
  } else if ((_v[0].is('String')) && (_v[1].is('Json'))) {
    const { _0: s } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: serdeJson.Value.String(s) }));
  } else if ((_v[0].is('I64')) && (_v[1].is('Json'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: n }));
  } else if ((_v[0].is('I32')) && (_v[1].is('Json'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: BigInt(n) }));
  } else if ((_v[0].is('I16')) && (_v[1].is('Json'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: BigInt(n) }));
  } else if ((_v[0].is('F64')) && (_v[1].is('Json'))) {
    const { _0: n } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: n }));
  } else if ((_v[0].is('Bool')) && (_v[1].is('Json'))) {
    const { _0: b } = _v[0].value;
    return Result.Ok(new Value('Json', { _0: serdeJson.Value.Bool(b) }));
  } else if ((_v[0].is('Json')) && (_v[1].is('String'))) {
    const { _0: json } = _v[0].value;
    return json.match({
      String: (v) => {
        const s = v._0;
        return Result.Ok(new Value('String', { _0: s }));
      },
      Null: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Bool: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Number: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Array: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Object: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
    });
  } else if ((_v[0].is('Json')) && (_v[1].is('I64'))) {
    const { _0: json } = _v[0].value;
    _match0: {
      if (json.is('Number')) {
        const { _0: n } = json.value;
        if (n.isI64()) {
          return Result.Ok(new Value('I64', { _0: (n.asI64() ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })()) }));
        }
      }
      {
        return Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType }));
      }
    }
  } else if ((_v[0].is('Json')) && (_v[1].is('I32'))) {
    const { _0: json } = _v[0].value;
    _match1: {
      if (json.is('Number')) {
        const { _0: n } = json.value;
        if (n.isI64()) {
          {
            const i = (n.asI64() ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
            if (i >= BigInt(-2147483648) && i <= BigInt(2147483647)) {
              return Result.Ok(new Value('I32', { _0: Number(BigInt.asIntN(32, i)) }));
            } else {
              return Result.Err(new CastError('NumericOverflow', { value: i.toString(), targetType: new ValueType('I32', {}) }));
            }
          }
        }
      }
      {
        return Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType }));
      }
    }
  } else if ((_v[0].is('Json')) && (_v[1].is('I16'))) {
    const { _0: json } = _v[0].value;
    _match2: {
      if (json.is('Number')) {
        const { _0: n } = json.value;
        if (n.isI64()) {
          {
            const i = (n.asI64() ?? (() => { throw new Error('called `Option::unwrap()` on a `None` value'); })());
            if (i >= BigInt(-32768) && i <= BigInt(32767)) {
              return Result.Ok(new Value('I16', { _0: Number(BigInt.asIntN(16, i)) }));
            } else {
              return Result.Err(new CastError('NumericOverflow', { value: i.toString(), targetType: new ValueType('I16', {}) }));
            }
          }
        }
      }
      {
        return Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType }));
      }
    }
  } else if ((_v[0].is('Json')) && (_v[1].is('F64'))) {
    const { _0: json } = _v[0].value;
    return json.match({
      Number: (v) => {
        const n = v._0;
        return Result.Ok(new Value('F64', { _0: n.asF64() ?? 0.0 }));
      },
      Null: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Bool: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      String: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Array: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Object: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
    });
  } else if ((_v[0].is('Json')) && (_v[1].is('Bool'))) {
    const { _0: json } = _v[0].value;
    return json.match({
      Bool: (v) => {
        const b = v._0;
        return Result.Ok(new Value('Bool', { _0: b }));
      },
      Null: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Number: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      String: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Array: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
      Object: () => Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType })),
    });
  } else {
    return Result.Err(new CastError('IncompatibleTypes', { from: sourceType, to: targetType }));
  }
}

export function Value_tryCastTo(self: Value, targetType: ValueType): Value | null {
  return Value_castTo(self, targetType).ok();
}

