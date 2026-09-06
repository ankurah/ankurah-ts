// MIRRORS: ankurah/core/src/indexing/encoding.rs
import { Enum, Result, wrappingSub } from '@ankurah/base';
import { Value_castTo } from '../value/cast';
import { Value_toBytes } from '../value/collatable';
import { Value, ValueType } from '../value/index';
import { KeySpec } from './key_spec';

export type IndexErrorV = {
  TypeMismatch: { _0: ValueType; _1: ValueType };
};

export class IndexError extends Enum<IndexErrorV> {

  debug(): string {
    return this.match({
      TypeMismatch: (v) => `TypeMismatch(${v._0.debug()}, ${v._1.debug()})`,
    });
  }

  override toString(): string {
    return this.match({
      TypeMismatch: (v) => `Type mismatch: expected ${v._0.debug()}, got ${v._1.debug()}`,
    });
  }
}

export function encodeComponentTyped(value: Value, expectedType: ValueType, descending: boolean): Result<Uint8Array, IndexError> {
  const _r0 = Value_castTo(value, expectedType).mapErr((_) => new IndexError('TypeMismatch', { _0: expectedType, _1: ValueType.of(value) }));
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const value_1 = _r0.unwrap();
  try {
    return encodeValueComponent(value_1, expectedType, descending);
  } finally {
    value_1.drop();
  }
}

function encodeValueComponent(value: Value, expectedType: ValueType, descending: boolean): Result<Uint8Array, IndexError> {
  const _v = [value, expectedType];
  if ((_v[0].is('String')) && (_v[1].is('String'))) {
    const { _0: s } = _v[0].value;
    if (!descending) {
      let out = [];
      for (const b of s.asBytes()) {
        if (b === 0) {
          out.push(0);
          out.push(255);
        } else {
          out.push(b);
        }
      }
      out.push(0);
      return Result.Ok(out);
    } else {
      let out = [];
      for (const b of s.asBytes()) {
        const inv = wrappingSub((255), b, 'u8');
        if (inv === 255) {
          out.push(255);
          out.push(0);
        } else {
          out.push(inv);
        }
      }
      out.push(255);
      out.push(255);
      return Result.Ok(out);
    }
  } else if (((_v[0].is('I16')) || (_v[0].is('I32')) || (_v[0].is('I64'))) && ((_v[1].is('I16')) || (_v[1].is('I32')) || (_v[1].is('I64')))) {
    {
      const bytes = Value_toBytes(value);
      if (!descending) {
        return Result.Ok(bytes);
      } else {
        return Result.Ok(Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8'))));
      }
    }
  } else if ((_v[0].is('F64')) && (_v[1].is('F64'))) {
    {
      const bytes = Value_toBytes(value);
      if (!descending) {
        return Result.Ok(bytes);
      } else {
        return Result.Ok(Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8'))));
      }
    }
  } else if ((_v[0].is('Bool')) && (_v[1].is('Bool'))) {
    {
      const b = Value_toBytes(value)[0];
      return Result.Ok(new Uint8Array([(!descending ? b : wrappingSub((255), b, 'u8'))]));
    }
  } else if ((_v[0].is('EntityId')) && (_v[1].is('EntityId'))) {
    const { _0: entityId } = _v[0].value;
    {
      const bytes = entityId.toBytes();
      if (!descending) {
        return Result.Ok(bytes.slice());
      } else {
        return Result.Ok(Uint8Array.from([...bytes].map((b) => wrappingSub((255), b, 'u8'))));
      }
    }
  } else if (((_v[0].is('Object')) || (_v[0].is('Binary'))) && ((_v[1].is('Binary')) || (_v[1].is('Object')))) {
    const { _0: bytes } = _v[0].value;
    if (!descending) {
      let out = [];
      for (const b of [...bytes]) {
        if (b === 0) {
          out.push(0);
          out.push(255);
        } else {
          out.push(b);
        }
      }
      out.push(0);
      return Result.Ok(out);
    } else {
      let out = [];
      for (const b of [...bytes]) {
        const inv = wrappingSub((255), b, 'u8');
        if (inv === 255) {
          out.push(255);
          out.push(0);
        } else {
          out.push(inv);
        }
      }
      out.push(255);
      out.push(255);
      return Result.Ok(out);
    }
  } else if ((_v[0].is('Json')) && (_v[1].is('Json'))) {
    const { _0: json } = _v[0].value;
    return Result.Ok(encodeJsonValue(json, descending));
  } else {
    return Result.Err(new IndexError('TypeMismatch', { _0: expectedType, _1: ValueType.of(value) }));
  }
}

function encodeJsonValue(json: unknown, descending: boolean): Uint8Array {
  const [tag, payload] = json.match({
    Null: () => [JSON_TAG_NULL, []] as any,
    Bool: (v) => {
      const b = v._0;
      return [JSON_TAG_BOOL, [(b ? 1 : 0)]] as any;
    },
    Number: (v) => {
      const n = v._0;
      {
        const _v1 = n.asI64();
        if (_v1 != null) {
          const i = _v1;
          const _t0 = new Value('I64', { _0: i });
          try {
            return [JSON_TAG_INT, Value_toBytes(_t0)];
          } finally {
            _t0.drop();
          }
        } else {
        const _v = n.asF64();
        if (_v != null) {
          const f = _v;
          const _t1 = new Value('F64', { _0: f });
          try {
            return [JSON_TAG_FLOAT, Value_toBytes(_t1)];
          } finally {
            _t1.drop();
          }
        } else {
        return [JSON_TAG_NULL, []];
      }
      }
      }
    },
    String: (v) => {
      const s = v._0;
      let payload = [];
      for (const b of s.asBytes()) {
        if (b === 0) {
          payload.push(0);
          payload.push(255);
        } else {
          payload.push(b);
        }
      }
      payload.push(0);
      return [JSON_TAG_STRING, payload];
    },
    Object: (v) => [JSON_TAG_NULL, []] as any,
    Array: (v) => [JSON_TAG_NULL, []] as any,
  });
  if (!descending) {
    let out = [];
    out.push(tag);
    out.extend(payload);
    return out;
  } else {
    let out = [];
    out.push(wrappingSub((255), tag, 'u8'));
    out.extend([...payload].map((b) => wrappingSub((255), b, 'u8')));
    return out;
  }
}

export function encodeTupleValuesWithKeySpec(values: Value[], keySpec: KeySpec): Result<Uint8Array, IndexError> {
  let out = [];
  for (const [i, v] of [...values].entries()) {
    if (i >= keySpec.keyparts.length) {
      break;
    }
    const keypart = keySpec.keyparts[i];
    try {
      const _r0 = encodeComponentTyped(v, keypart.valueType, keypart.direction.isDesc());
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const bytes = _r0.unwrap();
      out.extendFromSlice(bytes);
    } finally {
      keypart.drop();
    }
  }
  return Result.Ok(out);
}

const JSON_TAG_NULL: number = 0;

const JSON_TAG_BOOL: number = 16;

const JSON_TAG_INT: number = 32;

const JSON_TAG_FLOAT: number = 48;

const JSON_TAG_STRING: number = 64;

