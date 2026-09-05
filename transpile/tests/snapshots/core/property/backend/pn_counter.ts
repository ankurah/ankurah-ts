// MIRRORS: ankurah/core/src/property/backend/pn_counter.rs
import { Struct, Result, Arc, RwLock, HashMap } from '@ankurah/base';
import { Clock, Operation } from '@ankurah/proto';
import { MutationError, RetrievalError, StateError } from '../../error';
import { Value } from '../../value/index';
import { PropertyBackend } from './index';

export class PNBackend extends Struct implements PropertyBackend {
  values: Arc<RwLock<HashMap<PropertyName, PNValue>>>;

  constructor(values: Arc<RwLock<HashMap<PropertyName, PNValue>>>) {
    super();
    this.values = values;
  }

  static new(): PNBackend {
    return new PNBackend(Arc.new(new RwLock(new HashMap<string, PNValue>())));
  }

  get(propertyName: PropertyName): PNValue | null {
    const values = this.values.value.read();
    try {
      return values.value.get(propertyName);
    } finally {
      values.drop();
    }
  }

  add(propertyName: PropertyName, amount: PNValue): void {
    const values = this.values.value.write();
    PNBackend.addRaw(values, propertyName, amount);
  }

  static addRaw(values: DerefMut, propertyName: PropertyName, amount: PNValue): void {
    const value = values.derefMut().entry(propertyName).orDefault();
    value.value += amount;
  }

  static default(): PNBackend {
    return PNBackend.new();
  }

  asArcDynAny(): Arc<Any> {
    return this;
  }

  asDebug(): Debug {
    return this;
  }

  fork(): PropertyBackend {
    const values = this.values.value.read();
    try {
      const snapshotted = [...values.value].map(([key, value]) => [key.clone(), value.snapshot()]);
      return new PNBackend(Arc.new(new RwLock(snapshotted)));
    } finally {
      values.drop();
    }
  }

  properties(): string[] {
    const values = this.values.value.read();
    try {
      return [...values.value.keys()];
    } finally {
      values.drop();
    }
  }

  propertyValues(): HashMap<PropertyName, Value> {
    const values = this.values.value.read();
    try {
      let map = new HashMap();
      for (const [property, data] of [...values.value]) {
        map.insert(property, Value.Number(data.value));
      }
      return map;
    } finally {
      values.drop();
    }
  }

  static propertyBackendName(): string {
    return 'pn';
  }

  toStateBuffer(): Result<Uint8Array, StateError> {
    const values = this.values.value.read();
    try {
      const serializable = [...values.value].map(([key, value]) => [key, value.value]);
      const _r0 = (() => { const _w = new BincodeWriter(); serializable.encode(_w); return _w.finish(); })();
      if (_r0.isErr()) return Result.Err(StateError.fromError(_r0.unwrapErr()));
      const serialized = _r0.unwrap();
      return Result.Ok(serialized);
    } finally {
      values.drop();
    }
  }

  static fromStateBuffer(stateBuffer: Uint8Array): Result<PNBackend, RetrievalError> {
    const _r0 = (() => { const _r = new BincodeReader(stateBuffer); return (() => { const _m = new HashMap<PropertyName, PNValue>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(PropertyName.decode(_r), PNValue.decode(_r)); } return _m; })(); })();
    if (_r0.isErr()) return Result.Err(RetrievalError.fromBincodeError(_r0.unwrapErr()));
    const values = _r0.unwrap();
    return Result.Ok(new PNBackend(Arc.new(new RwLock(values))));
  }

  toOperations(): Result<Operation[], MutationError> {
    const values = this.values.value.read();
    try {
      const diffs = [...values.value].map(([key, value]) => [key, value.diff()]);
      const _r0 = (() => { const _w = new BincodeWriter(); diffs.encode(_w); return _w.finish(); })();
      if (_r0.isErr()) return Result.Err(MutationError.fromBincodeError(_r0.unwrapErr()));
      const serializedDiffs = _r0.unwrap();
      return Result.Ok([new Operation(serializedDiffs)]);
    } finally {
      values.drop();
    }
  }

  applyOperations(operations: Operation[], _currentHead: Clock, _eventHead: Clock): Result<void, MutationError> {
    for (const operation of operations) {
      const _r0 = (() => { const _r = new BincodeReader(operation.diff); return (() => { const _m = new HashMap<PropertyName, bigint>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(PropertyName.decode(_r), _r.readU64()); } return _m; })(); })();
      if (_r0.isErr()) return Result.Err(MutationError.fromBincodeError(_r0.unwrapErr()));
      const diffs = _r0.unwrap();
      let values = this.values.value.write();
      try {
        for (const [property, diff] of diffs) {
          PNBackend.addRaw(values.value, property, diff);
        }
      } finally {
        values.drop();
      }
    }
    return Result.Ok([]);
  }

  debug(): string {
    return `PNBackend { values: ${this.values} }`;
  }
}

