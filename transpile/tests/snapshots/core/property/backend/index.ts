// MIRRORS: ankurah/core/src/property/backend/mod.rs
import { Result, Arc, dropOwned, HashMap } from '@ankurah/base';
import { Operation } from '@ankurah/proto';
import { RetrievalError } from '../../error';
import { LWWBackend } from './lww';
import { YrsBackend } from './yrs';
export * from './lww';
export * from './yrs';

export abstract class PropertyBackend {
  abstract asArcDynAny(): Arc<Any>;
  abstract asDebug(): Debug;
  abstract fork(): Arc<PropertyBackend>;
  abstract properties(): PropertyName[];
  propertyValue(propertyName: PropertyName): Value | null {
    let map = this.propertyValues();
    try {
      return map.remove(propertyName).flatten();
    } finally {
      dropOwned(map);
    }
  }
  abstract propertyValues(): HashMap<PropertyName, Value | null>;
  abstract propertyBackendName(): string;
  abstract toStateBuffer(): Result<Uint8Array, StateError>;
  abstract fromStateBuffer(stateBuffer: Uint8Array): Result<Self, RetrievalError>;
  abstract toOperations(): Result<Operation[] | null, MutationError>;
  abstract applyOperations(operations: Operation[]): Result<void, MutationError>;
  abstract listenField(fieldName: PropertyName, listener: Listener): ListenerGuard;
}

export function backendFromString(name: string, buffer: Uint8Array | null): Result<Arc<PropertyBackend>, RetrievalError> {
  if (name === 'yrs') {
    const _r0 = YrsBackend.fromStateBuffer(buffer);
    if (_r0.isErr()) return { $jump: 'return', $value: Result.Err(_r0.unwrapErr()) };
    const _m1 = (() => {
      const _v = buffer;
      if (_v != null) {
        const buffer = _v;
        return _r0.unwrap();
      } else {
        return YrsBackend.new();
      }
    })();
    if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
    let _moved2 = false;
    const backend = (_m1 as any);
    try {
      _moved2 = true;
      return Result.Ok(Arc.new(backend));
    } finally {
      if (!_moved2) backend.drop();
    }
  } else if (name === 'lww') {
    const _r3 = LWWBackend.fromStateBuffer(buffer);
    if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
    const _m4 = (() => {
      const _v1 = buffer;
      if (_v1 != null) {
        const buffer = _v1;
        return _r3.unwrap();
      } else {
        return LWWBackend.new();
      }
    })();
    if ((_m4 as any)?.$jump === 'return') return (_m4 as any).$value;
    let _moved5 = false;
    const backend = (_m4 as any);
    try {
      _moved5 = true;
      return Result.Ok(Arc.new(backend));
    } finally {
      if (!_moved5) backend.drop();
    }
  } else {
    throw new Error(`unknown backend: ${JSON.stringify(name)}`);
  }
}

