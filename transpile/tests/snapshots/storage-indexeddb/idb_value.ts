// MIRRORS: ankurah/storage/indexeddb-wasm/src/idb_value.rs
import { Struct, Result, tracing, unsupported, range } from '@ankurah/base';
import { Value, Json } from '@ankurah/core';
import { Object } from './util/object';
import { EntityId } from '@ankurah/proto';

export class IdbValue extends Struct {
  _0: Value;

  constructor(_0: Value) {
    super();
    this._0 = _0;
  }

  intoValue(): Value {
    try {
      return this.takeField('_0');
    } finally {
      this.drop();
    }
  }

  static fromValue(value: Value): IdbValue {
    return new IdbValue(value);
  }

  static fromRefValue(value: Value): IdbValue {
    return new IdbValue(value.clone());
  }

  static tryFromJsValue(jsValue: unknown): Result<IdbValue, unknown> {
    {
      const _v = Value.tryFromJsValue(jsValue);
      if (_v.isOk()) {
        const value = _v.unwrap();
        return Result.Ok(new IdbValue(value));
      } else {
      _v.drop();
    }
    }
    if ((jsValue !== null && typeof jsValue === 'object')) {
      {
        const _v1 = serdeWasmBindgen.fromValue(jsValue);
        if (_v1.isOk()) {
          const json = _v1.unwrap();
          return Result.Ok(new IdbValue(new Value('Json', { _0: json })));
        }
      }
    }
    return Result.Err(jsValue);
  }
}

function convertJsonBoolsToNumbers(json: unknown): unknown {
  return json.match({
    Bool: (v) => {
      const b = v._0;
      return serdeJson.Value.Number((b ? (1) : (0)));
    },
    Array: (v) => {
      const arr = v._0;
      return serdeJson.Value.Array([...arr].map(convertJsonBoolsToNumbers));
    },
    Object: (v) => {
      const obj = v._0;
      return serdeJson.Value.Object(unsupported('`collect` into `Map<string, unknown>` is a `FromIterator` the port has no construction for'));
    },
    Null: () => {
      const other = json;
      return structuredClone(other);
    },
    Number: () => {
      const other = json;
      return structuredClone(other);
    },
    String: () => {
      const other = json;
      return structuredClone(other);
    },
  });
}

export const MAX_SAFE_INTEGER: bigint = 9007199254740991n;

export const MIN_SAFE_INTEGER: bigint = -9007199254740991n;

export function JsValue_fromIdbValue(value: IdbValue): unknown {
  try {
    return value._0.match({
      I16: (v) => {
        const x = v._0;
        return JsValue.fromF64(x);
      },
      I32: (v) => {
        const x = v._0;
        return JsValue.fromF64(x);
      },
      I64: (v) => {
        const x = v._0;
        if (x < 0n) {
          if (x < MIN_SAFE_INTEGER) {
            tracing.warn(`Negative i64 ${x} exceeds safe integer range (${MIN_SAFE_INTEGER}), precision loss will occur`);
          }
          return JsValue.fromF64(Number(x));
        } else if (x <= MAX_SAFE_INTEGER) {
          return JsValue.fromF64(Number(x));
        } else {
          return JsValue.fromStr(`${x}`);
        }
      },
      F64: (v) => {
        const x = v._0;
        return JsValue.fromF64(x);
      },
      Bool: (v) => {
        const b = v._0;
        return JsValue.fromF64((b ? 1.0 : 0.0));
      },
      String: (v) => {
        const s = v._0;
        return JsValue.fromStr(s);
      },
      EntityId: (v) => {
        const entityId = v._0;
        return JsValue.fromStr(entityId.toBase64());
      },
      Binary: (v) => {
        const bytes = v._0;
        return jsSys.Uint8Array.from(bytes);
      },
      Object: (v) => {
        const bytes = v._0;
        return jsSys.Uint8Array.from(bytes);
      },
      Json: (v) => {
        const json = v._0;
        const converted = convertJsonBoolsToNumbers(json);
        const serializer = serdeWasmBindgen.Serializer.jsonCompatible();
        return converted.encode(serializer).unwrapOr(JsValue.NULL);
      },
    });
  } finally {
    value.drop();
  }
}

