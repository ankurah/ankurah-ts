// MIRRORS: ankurah/storage/indexeddb-wasm/src/scanner.rs
import { Struct, Enum, Result, dropOwned } from '@ankurah/base';
import { RetrievalError } from '@ankurah/core';
import { IdbValue } from './idb_value';
import { CBStream, cbStream } from './util/cb_stream';
import { Object } from './util/object';
import { RetrievalError, Value } from '@ankurah/core';

export class IdbIndexScanner extends Struct {
  index: IdbIndex;
  keyRange: IdbKeyRange | null;
  cursorDirection: IdbCursorDirection;
  eqPrefixLen: number;
  eqPrefixJs: unknown[];

  constructor(index: IdbIndex, keyRange: IdbKeyRange | null, cursorDirection: IdbCursorDirection, eqPrefixLen: number, eqPrefixJs: unknown[]) {
    super();
    this.index = index;
    this.keyRange = keyRange;
    this.cursorDirection = cursorDirection;
    this.eqPrefixLen = eqPrefixLen;
    this.eqPrefixJs = eqPrefixJs;
  }

  static new(index: IdbIndex, keyRange: IdbKeyRange | null, cursorDirection: IdbCursorDirection, eqPrefixLen: number, eqPrefixValues: Value[]): IdbIndexScanner {
    try {
      const eqPrefixJs = [...eqPrefixValues].map((v) => {
        const jsVal = IdbValue.fromRefValue(v);
        return jsVal;
      });
      return new IdbIndexScanner(index, keyRange, cursorDirection, eqPrefixLen, eqPrefixJs);
    } finally {
      dropOwned(eqPrefixValues);
    }
  }

  scan(): Stream {
    return futures.stream.unfold(new ScanState('Initial', { _0: this }), (state) => (async () => {
      const _r0 = state;
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const state_1 = _r0.unwrap();
      return await (state_1.match({
        Initial: async (v) => {
          const scanner = v._0;
          const cursorRequest = (() => {
            {
              const _v = scanner.keyRange;
              if (_v != null) {
                const range = _v;
                return scanner.index.openCursorWithRangeAndDirection(range.asRef(), scanner.cursorDirection);
              } else {
              return scanner.index.openCursorWithRangeAndDirection(JsValue.NULL, scanner.cursorDirection);
            }
            }
          })();
          const _m1 = (() => {
            if (cursorRequest.isOk()) {
              const req = cursorRequest.unwrap();
              return req;
            } else {
              const e = cursorRequest.unwrapErr();
              return { $jump: 'return', $value: [Result.Err(new RetrievalError('StorageError', { _0: `Failed to open cursor: ${e}` })), null] };
            }
          })();
          if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
          const cursorRequest_1 = (_m1 as any);
          let _moved2 = false;
          const stream = cbStream(cursorRequest_1, 'success', 'error');
          try {
            _moved2 = true;
            let _moved3 = false;
            const state_2 = new ScanState('Scanning', { stream: Box.pin(stream), eqPrefixLen: scanner.eqPrefixLen, eqPrefixJs: scanner.eqPrefixJs });
            try {
              return await Box.pin((async () => {
                _moved3 = true;
                return await getNextRecord(state_2);
              })());
            } finally {
              if (!_moved3) state_2.drop();
            }
          } finally {
            if (!_moved2) stream.drop();
          }
        },
        Scanning: async () => await getNextRecord(state_1),
      }));
    })());
  }
}

type ScanStateV = {
  Initial: { _0: IdbIndexScanner };
  Scanning: { stream: Pin<CBStream>; eqPrefixLen: number; eqPrefixJs: unknown[] };
};

class ScanState extends Enum<ScanStateV> {
}

async function getNextRecord(state: ScanState): Promise<[Result<Object, RetrievalError>, ScanState | null] | null> {
  const _v = state;
  if (!(_v.is('Scanning'))) {
    return null;
  }
  const { stream, eqPrefixLen, eqPrefixJs } = _v.value;
  const _r0 = await stream.next();
  if (_r0 == null) return null;
  const result = _r0;
  const _m1 = (() => {
    if (result.isOk()) {
      const val = result.unwrap();
      return val;
    } else {
      const e = result.unwrapErr();
      return { $jump: 'return', $value: [Result.Err(new RetrievalError('StorageError', { _0: `Cursor error: ${e}` })), null] };
    }
  })();
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  const cursorResult = (_m1 as any);
  if (cursorResult.isNull() || cursorResult.isUndefined()) {
    return null;
  }
  const _m2 = (() => {
    const _v1 = cursorResult.dynInto();
    if (_v1.isOk()) {
      const c = _v1.unwrap();
      return c;
    } else {
      const e = _v1.unwrapErr();
      return { $jump: 'return', $value: [Result.Err(new RetrievalError('StorageError', { _0: `Failed to cast cursor: ${e}` })), null] };
    }
  })();
  if ((_m2 as any)?.$jump === 'return') return (_m2 as any).$value;
  const cursor = (_m2 as any);
  if (eqPrefixLen > 0) {
    {
      const _v2 = cursor.key();
      if (_v2.isOk()) {
        const keyJs = _v2.unwrap();
        if (!((keyJs === undefined)) && !((keyJs === null))) {
          const keyArr = jsSys.Array.from(keyJs);
          for (const i of undefined /* range 0..(eqPrefixLen) */) {
            const lhs = keyArr.get(i);
            const rhs = eqPrefixJs[i];
            if (!jsSys.Object.is(lhs, rhs)) {
              return null;
            }
          }
        }
      }
    }
  }
  const _m3 = (() => {
    const _v3 = cursor.value();
    if (_v3.isOk()) {
      const v = _v3.unwrap();
      return v;
    } else {
      const e = _v3.unwrapErr();
      return { $jump: 'return', $value: [Result.Err(new RetrievalError('StorageError', { _0: `Failed to get cursor value: ${e}` })), null] };
    }
  })();
  if ((_m3 as any)?.$jump === 'return') return (_m3 as any).$value;
  const value = (_m3 as any);
  let _moved4 = false;
  const entityObj = Object.new(value);
  try {
    {
      const _v4 = cursor.continue();
      if (_v4.isErr()) {
        const e = _v4.unwrapErr();
        return [Result.Err(new RetrievalError('StorageError', { _0: `Failed to advance cursor: ${e}` })), null];
      }
    }
    _moved4 = true;
    return [Result.Ok(entityObj), new ScanState('Scanning', { stream: stream, eqPrefixLen: eqPrefixLen, eqPrefixJs: eqPrefixJs })];
  } finally {
    if (!_moved4) entityObj.drop();
  }
}

