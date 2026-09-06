// MIRRORS: ankurah/storage/indexeddb-wasm/src/planner_integration.rs
import { Result, AnyhowError, floatMax, tracing, checkedAdd, saturatingAdd, iterLast, range } from '@ankurah/base';
import { Value } from '@ankurah/core';
import { CanonicalRange, KeyBounds, ScanDirection } from '@ankurah/storage-common';
import { IdbValue } from './idb_value';

function nextUpperBound(value: Value): [Value, boolean] | null {
  return value.match({
    Bool: (v) => {
      if (v._0 === false) {
        return [new Value('Bool', { _0: true }), true];
      } else {
        return [new Value('I32', { _0: 2 }), true];
      }
    },
    I16: (_v) => {
      const v = _v._0;
      return [new Value('I16', { _0: saturatingAdd(v, 1, 'i16') }), true] as any;
    },
    I32: (_v) => {
      const v = _v._0;
      return [new Value('I32', { _0: saturatingAdd(v, 1, 'i32') }), true] as any;
    },
    I64: (_v) => {
      const v = _v._0;
      return [new Value('I64', { _0: saturatingAdd(v, 1n, 'i64') }), true] as any;
    },
    F64: (_v) => {
      const v = _v._0;
      if (Number.isNaN(v) || (!Number.isFinite(v) && !Number.isNaN(v))) {
        return null;
      } else {
        const epsilon = floatMax(Number.EPSILON, Math.abs(v) * Number.EPSILON);
        return [new Value('F64', { _0: v + epsilon }), true];
      }
    },
    String: (v) => {
      const s = v._0;
      let bumped = s;
      bumped += '\u{0}';
      return [new Value('String', { _0: bumped }), true];
    },
    EntityId: (v) => {
      const entityId = v._0;
      let bumped = entityId.toBase64();
      bumped += '\u{0}';
      return [new Value('String', { _0: bumped }), true];
    },
    Object: (v) => null,
    Binary: (v) => null,
    Json: (v) => null,
  });
}

export function normalize(bounds: KeyBounds): [CanonicalRange, number, Value[]] {
  let lowerTuple = [];
  let upperTuple = [];
  let lowerOpen = false;
  let upperOpen = false;
  let eqPrefixLen = 0;
  let eqPrefixValues = [];
  for (const bound of bounds.keyparts) {
    {
      const _v1 = [bound.low, bound.high];
      if ((_v1[0].is('Value')) && (_v1[1].is('Value'))) {
        const { datum: lowDatum, inclusive: lowIncl } = _v1[0].value;
        const { datum: highDatum, inclusive: highIncl } = _v1[1].value;
        {
          const _v = [lowDatum, highDatum];
          if ((_v[0].is('Val')) && (_v[1].is('Val'))) {
            const { _0: lowVal } = _v[0].value;
            const { _0: highVal } = _v[1].value;
            if (lowVal.equals(highVal) && lowIncl && highIncl) {
              lowerTuple.push(lowVal.clone());
              upperTuple.push(highVal.clone());
              eqPrefixLen = checkedAdd(eqPrefixLen, 1, 'i32');
              eqPrefixValues.push(lowVal.clone());
              continue;
            }
          }
        }
      }
    }
    if (bound.low.is('UnboundedLow')) {

    } else if (bound.low.is('Value') && (bound.low.value.datum.is('Val'))) {
      const { inclusive } = bound.low.value;
      const { _0: val } = bound.low.value.datum.value;
      lowerTuple.push(val.clone());
      lowerOpen = !inclusive;
    } else {
      break
    }
    const _m0 = bound.high.match<any>({
      UnboundedHigh: (v) => {
        return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
      },
      Value: (v) => {
        if (v.datum.is('Val')) {
          const { _0: val } = v.datum.value;
          const inclusive = v.inclusive;
          upperTuple.push(val.clone());
          upperOpen = !inclusive;
        } else {
          return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
        }
      },
      UnboundedLow: () => {
        return { $jump: 'return', $value: [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues] };
      },
    });
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
    break;
  }
  if (eqPrefixLen === bounds.keyparts.length && eqPrefixLen > 0) {
    {
      const _v4 = lowerTuple.last();
      if (_v4 != null) {
        const lastValue = _v4;
        {
          const _v3 = nextUpperBound(lastValue);
          if (_v3 != null) {
            const [nextValue, upperOpen] = _v3;
            let _moved1 = false;
            try {
              let upperWithBump = lowerTuple.clone();
              {
                const _v2 = upperWithBump.lastMut();
                if (_v2 != null) {
                  const slot = _v2;
                  _moved1 = true;
                  slot.value = nextValue;
                  return [new CanonicalRange([lowerTuple, lowerOpen], [upperWithBump, upperOpen]), eqPrefixLen, eqPrefixValues];
                }
              }
            } finally {
              if (!_moved1) nextValue.drop();
            }
          }
        }
      }
    }
    return [new CanonicalRange([lowerTuple, lowerOpen], null), eqPrefixLen, eqPrefixValues];
  }
  let _moved2 = false;
  const canonicalRange = new CanonicalRange((lowerTuple.length === 0 ? null : [lowerTuple, lowerOpen]), (upperTuple.length === 0 ? null : [upperTuple, upperOpen]));
  try {
    _moved2 = true;
    return [canonicalRange, eqPrefixLen, eqPrefixValues];
  } finally {
    if (!_moved2) canonicalRange.drop();
  }
}

export function toIdbKeyrange(canonicalRange: CanonicalRange): Result<[IdbKeyRange, boolean], Error> {
  const _v = [canonicalRange.lower, canonicalRange.upper];
  if ((_v[0] != null) && (_v[1] == null)) {
    const [lowerTuple, lowerOpen] = _v[0];
    {
      const _r0 = idbKeyTuple(lowerTuple);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const lowerJs = _r0.unwrap();
      const _r1 = webSys.IdbKeyRange.lowerBoundWithOpen(lowerJs, lowerOpen).mapErr((e) => AnyhowError.msg(`Failed to create lower bound IdbKeyRange: ${e}`));
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      const range = _r1.unwrap();
      return Result.Ok([range, true]);
    }
  } else if ((_v[0] != null) && (_v[1] != null)) {
    const [lowerTuple, lowerOpen] = _v[0];
    const [upperTuple, upperOpen] = _v[1];
    {
      const _r2 = idbKeyTuple(lowerTuple);
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      const lowerJs = _r2.unwrap();
      const _r3 = idbKeyTuple(upperTuple);
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      const upperJs = _r3.unwrap();
      const _r4 = webSys.IdbKeyRange.boundWithLowerOpenAndUpperOpen(lowerJs, upperJs, lowerOpen, upperOpen).mapErr((e) => AnyhowError.msg(`Failed to create bound IdbKeyRange: ${e}`));
      if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
      const range = _r4.unwrap();
      return Result.Ok([range, false]);
    }
  } else if ((_v[0] == null) && (_v[1] != null)) {
    const [upperTuple, upperOpen] = _v[1];
    {
      const _r5 = idbKeyTuple(upperTuple);
      if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
      const upperJs = _r5.unwrap();
      const _r6 = webSys.IdbKeyRange.upperBoundWithOpen(upperJs, upperOpen).mapErr((e) => AnyhowError.msg(`Failed to create upper bound IdbKeyRange: ${e}`));
      if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
      const range = _r6.unwrap();
      return Result.Ok([range, false]);
    }
  } else {
    return Result.Err(AnyhowError.msg('Cannot create IdbKeyRange for completely unbounded range'));
  }
}

function idbKeyTuple(parts: Value[]): Result<JsValue, Error> {
  const arr = jsSys.Array.new();
  for (const p of parts) {
    const jsVal = IdbValue.fromRefValue(p);
    arr.push(jsVal);
  }
  return Result.Ok(arr);
}

export function planBoundsToIdbRange(bounds: KeyBounds, scanDirection: ScanDirection): Result<[IdbKeyRange, boolean, number, Value[]], Error> {
  const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
  const adjustedRange = (scanDirection.equals(new ScanDirection('Reverse', {})) && (canonicalRange.upper == null) && eqPrefixLen > 0 && (canonicalRange.lower != null) ? (() => {
    {
      const _v2 = iterLast(eqPrefixValues);
      if (_v2 != null) {
        const lastEqValue = _v2;
        {
          const _v1 = nextUpperBound(lastEqValue);
          if (_v1 != null) {
            const [nextValue, isOpen] = _v1;
            let _moved0 = false;
            try {
              let upperTuple = eqPrefixValues.slice(0, eqPrefixLen).map((e) => e.clone());
              {
                const _v = upperTuple.lastMut();
                if (_v != null) {
                  const slot = _v;
                  const _a1 = nextValue;
                  slot.value.drop();
                  _moved0 = true;
                  slot.value = _a1;
                }
              }
              return new CanonicalRange(canonicalRange.lower.clone(), [upperTuple, isOpen]);
            } finally {
              if (!_moved0) nextValue.drop();
            }
          } else {
          return canonicalRange;
        }
        }
      } else {
      return canonicalRange;
    }
    }
  })() : canonicalRange);
  try {
    const _r2 = toIdbKeyrange(adjustedRange);
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    const [idbRange, upperOpenEnded] = _r2.unwrap();
    return Result.Ok([idbRange, upperOpenEnded, eqPrefixLen, eqPrefixValues]);
  } finally {
    adjustedRange.drop();
  }
}

export function planBoundsToIdbRangeSyntax(bounds: KeyBounds): Result<string, Error> {
  const [canonicalRange, eqPrefixLen, eqPrefixValues] = normalize(bounds);
  tracing.info(`plan_bounds_to_idb_range_syntax input: ${bounds.debug()}`);
  tracing.info(`Normalized canonical_range: ${canonicalRange.debug()}`);
  tracing.info(`eq_prefix_len: ${eqPrefixLen}, eq_prefix_values: ${`[${Array.from(eqPrefixValues).map((e) => e.debug()).join(', ')}]`}`);
  let jsCode = '';
  const _v = [canonicalRange.lower, canonicalRange.upper];
  if ((_v[0] != null) && (_v[1] != null)) {
    const [lowerTuple, lowerOpen] = _v[0];
    const [upperTuple, upperOpen] = _v[1];
    _result += 'IDBKeyRange.bound(';
    const _r0 = valuesToJsArray(lowerTuple);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _result += `${_r0.unwrap()}`;
    _result += ', ';
    const _r1 = valuesToJsArray(upperTuple);
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _result += `${_r1.unwrap()}`;
    _result += `, ${lowerOpen}, ${upperOpen}`;
    _result += ')';
  } else if ((_v[0] != null) && (_v[1] == null)) {
    const [lowerTuple, lowerOpen] = _v[0];
    _result += 'IDBKeyRange.lowerBound(';
    const _r2 = valuesToJsArray(lowerTuple);
    if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
    _result += `${_r2.unwrap()}`;
    _result += `, ${lowerOpen})`;
  } else if ((_v[0] == null) && (_v[1] != null)) {
    const [upperTuple, upperOpen] = _v[1];
    _result += 'IDBKeyRange.upperBound(';
    const _r3 = valuesToJsArray(upperTuple);
    if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
    _result += `${_r3.unwrap()}`;
    _result += `, ${upperOpen})`;
  } else {
    return Result.Err(AnyhowError.msg('Cannot generate syntax for completely unbounded range'));
  }
  tracing.info(`Generated IDBKeyRange syntax: ${jsCode}`);
  return Result.Ok(jsCode);
}

function valuesToJsArray(values: Value[]): Result<string, Error> {
  let result = '[';
  for (const [i, value] of [...values].entries()) {
    if (i > 0) {
      result.pushStr(', ');
    }
    const _m0 = value.match<any>({
      String: (v) => {
        const s = v._0;
        result.push('"');
        result.pushStr(s.replace('\\', '\\\\').replace('"', '\\"'));
        result.push('"');
      },
      I64: (v) => {
        const x = v._0;
        result.pushStr(x.toString());
      },
      I32: (v) => {
        const x = v._0;
        result.pushStr(x.toString());
      },
      I16: (v) => {
        const x = v._0;
        result.pushStr(x.toString());
      },
      F64: (v) => {
        const x = v._0;
        if ((x - Math.trunc(x)) === 0.0) {
          result.pushStr(`${x}`);
        } else {
          result.pushStr(x.toString());
        }
      },
      Bool: (v) => {
        const b = v._0;
        result.pushStr((b ? '1' : '0'));
      },
      EntityId: (v) => {
        const entityId = v._0;
        result.push('"');
        result.pushStr(entityId.toBase64());
        result.push('"');
      },
      Object: (v) => {
        return { $jump: 'return', $value: Result.Err(AnyhowError.msg(`Object, Binary and Json values not supported in key syntax generation: ${value.debug()}`)) };
      },
      Binary: (v) => {
        return { $jump: 'return', $value: Result.Err(AnyhowError.msg(`Object, Binary and Json values not supported in key syntax generation: ${value.debug()}`)) };
      },
      Json: (v) => {
        return { $jump: 'return', $value: Result.Err(AnyhowError.msg(`Object, Binary and Json values not supported in key syntax generation: ${value.debug()}`)) };
      },
    });
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
  }
  result.push(']');
  return Result.Ok(result);
}

export function scanDirectionToCursorDirection(scanDirection: ScanDirection): IdbCursorDirection {
  return scanDirection.match({
    Forward: () => webSys.IdbCursorDirection.Next,
    Reverse: () => webSys.IdbCursorDirection.Prev,
  });
}

