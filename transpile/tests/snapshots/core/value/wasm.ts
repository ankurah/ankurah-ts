// MIRRORS: ankurah/core/src/value/wasm.rs
import { Result } from '@ankurah/base';
import { Value } from './index';

export function JsValue_fromValue(value: Value): unknown {
  try {
    return value.match({
      String: (v) => {
        const s = v._0;
        return JsValue.fromStr(s);
      },
      I16: (v) => {
        const i = v._0;
        return JsValue.fromF64(i);
      },
      I32: (v) => {
        const i = v._0;
        return JsValue.fromF64(i);
      },
      I64: (v) => {
        const i = v._0;
        return JsValue.fromF64(Number(i));
      },
      F64: (v) => {
        const f = v._0;
        return JsValue.fromF64(f);
      },
      Bool: (v) => {
        const b = v._0;
        return JsValue.fromBool(b);
      },
      EntityId: (v) => {
        const entityId = v._0;
        return JsValue.fromStr(entityId.toBase64());
      },
      Object: (v) => {
        const bytes = v._0;
        return jsSys.Uint8Array.from(bytes.slice(0));
      },
      Binary: (v) => {
        const bytes = v._0;
        return jsSys.Uint8Array.from(bytes.slice(0));
      },
      Json: (v) => {
        const json = v._0;
        return serdeWasmBindgen.toValue(json).unwrapOr(JsValue.NULL);
      },
    });
  } finally {
    value.drop();
  }
}

export function JsValue_fromRefValue(value: Value): unknown {
  return value.match({
    String: (v) => {
      const s = v._0;
      return JsValue.fromStr(s);
    },
    I16: (v) => {
      const i = v._0;
      return JsValue.fromF64(i);
    },
    I32: (v) => {
      const i = v._0;
      return JsValue.fromF64(i);
    },
    I64: (v) => {
      const i = v._0;
      return JsValue.fromF64(Number(i));
    },
    F64: (v) => {
      const f = v._0;
      return JsValue.fromF64(f);
    },
    Bool: (v) => {
      const b = v._0;
      return JsValue.fromBool(b);
    },
    EntityId: (v) => {
      const entityId = v._0;
      return JsValue.fromStr(entityId.toBase64());
    },
    Object: (v) => {
      const bytes = v._0;
      return jsSys.Uint8Array.from(bytes.slice(0));
    },
    Binary: (v) => {
      const bytes = v._0;
      return jsSys.Uint8Array.from(bytes.slice(0));
    },
    Json: (v) => {
      const json = v._0;
      return serdeWasmBindgen.toValue(json).unwrapOr(JsValue.NULL);
    },
  });
}

export function Value_tryFromJsValue(value: unknown): Result<Value, unknown> {
  if ((value === null) || (value === undefined)) {
    return Result.Err(value);
  }
  {
    const _v = (typeof value === 'string' ? value : null);
    if (_v != null) {
      const s = _v;
      return Result.Ok(new Value('String', { _0: s }));
    }
  }
  {
    const _v1 = (typeof value === 'boolean' ? value : null);
    if (_v1 != null) {
      const b = _v1;
      return Result.Ok(new Value('Bool', { _0: b }));
    }
  }
  {
    const _v2 = (typeof value === 'number' ? value : null);
    if (_v2 != null) {
      const n = _v2;
      if ((n - Math.trunc(n)) === 0.0) {
        const nInt = (($v) => $v < -9223372036854775808n ? -9223372036854775808n : $v > 9223372036854775807n ? 9223372036854775807n : $v)(BigInt(Math.min(Math.max(Math.trunc(n) || 0, -9223372036854775808), 9223372036854775807)));
        if (nInt >= BigInt(-2147483648) && nInt <= BigInt(2147483647)) {
          return Result.Ok(new Value('I32', { _0: Number(BigInt.asIntN(32, nInt)) }));
        } else {
          return Result.Ok(new Value('I64', { _0: nInt }));
        }
      } else {
        return Result.Ok(new Value('F64', { _0: n }));
      }
    }
  }
  if ((typeof value === 'object')) {
    const array = value;
    let bytes = Array(array.length()).fill(0);
    array.copyTo(bytes);
    return Result.Ok(new Value('Binary', { _0: bytes }));
  }
  return Result.Err(value);
}

