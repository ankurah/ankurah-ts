// MIRRORS: ankurah/core/src/property/backend/lww.rs
import { Struct, Result, Arc, Mutex, RwLock, AnyhowError, JsonError, dropOwned, OwnershipFatal, UnsupportedShape, unsupported, HashMap } from '@ankurah/base';
import { Operation } from '@ankurah/proto';
import { Listener, Broadcast, BroadcastId, ListenerGuard } from '@ankurah/signals';
import { BincodeReader, BincodeWriter } from './codec';
import { MutationError, RetrievalError, StateError } from '../../error';
import { Value } from '../../value/index';
import { LWW } from '../value/lww';
import { PropertyBackend } from './index';

class ValueEntry extends Struct {
  value: Value | null;
  committed: boolean;

  constructor(value: Value | null, committed: boolean) {
    super();
    this.value = value;
    this.committed = committed;
  }

  clone(): ValueEntry {
    return new ValueEntry(this.value?.clone() ?? null, this.committed);
  }

  debug(): string {
    return `ValueEntry { value: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(this.value)}, committed: ${String(this.committed)} }`;
  }
}

export class LWWBackend extends Struct implements PropertyBackend {
  values: RwLock<HashMap<PropertyName, ValueEntry>>;
  fieldBroadcasts: Mutex<HashMap<PropertyName, Broadcast>>;

  constructor(values: RwLock<HashMap<PropertyName, ValueEntry>>, fieldBroadcasts: Mutex<HashMap<PropertyName, Broadcast>>) {
    super();
    this.values = values;
    this.fieldBroadcasts = fieldBroadcasts;
  }

  static new(): LWWBackend {
    return new LWWBackend(new RwLock(new HashMap<string, ValueEntry>()), new Mutex(new HashMap<string, Broadcast<void>>()));
  }

  set(propertyName: PropertyName, value: Value | null): void {
    let values = this.values.write();
    try {
      values.value.set(propertyName, new ValueEntry(value, false));
    } finally {
      values.drop();
    }
  }

  get(propertyName: PropertyName): Value | null {
    const values = this.values.read();
    try {
      const _m0 = values.value.get(propertyName);
      return (_m0 != null ? ((entry) => entry.value.clone())(_m0!) : null);
    } finally {
      values.drop();
    }
  }

  fieldBroadcastId(fieldName: PropertyName): BroadcastId {
    let fieldBroadcasts = this.fieldBroadcasts.lock();
    try {
      const broadcast = fieldBroadcasts.value.entry(fieldName).orDefault(() => Broadcast.default());
      return broadcast.value.id();
    } finally {
      fieldBroadcasts.drop();
    }
  }

  static default(): LWWBackend {
    return LWWBackend.new();
  }

  asArcDynAny(): Arc<Any> {
    return this;
  }

  asDebug(): Debug {
    return this;
  }

  fork(): Arc<PropertyBackend> {
    const values = this.values.read();
    const cloned = (values.value).clone();
    values.drop();
    return Arc.new(new LWWBackend(new RwLock(cloned), new Mutex(new HashMap<string, Broadcast<void>>())));
  }

  properties(): PropertyName[] {
    const values = this.values.read();
    try {
      return [...values.value.keys()];
    } finally {
      values.drop();
    }
  }

  propertyValue(propertyName: PropertyName): Value | null {
    return this.get(propertyName);
  }

  propertyValues(): HashMap<PropertyName, Value | null> {
    const values = this.values.read();
    try {
      return HashMap.from([...values.value].map(([k, v]) => [k.clone(), v.value.clone()]));
    } finally {
      values.drop();
    }
  }

  static propertyBackendName(): string {
    return 'lww';
  }

  toStateBuffer(): Result<Uint8Array, StateError> {
    const propertyValues = this.propertyValues();
    try {
      const _r0 = (() => { const _w = new BincodeWriter(); propertyValues.encode(_w); return _w.finish(); })();
      if (_r0.isErr()) return Result.Err(StateError.fromError(_r0.unwrapErr()));
      const stateBuffer = _r0.unwrap();
      return Result.Ok(stateBuffer);
    } finally {
      dropOwned(propertyValues);
    }
  }

  static fromStateBuffer(stateBuffer: Uint8Array): Result<LWWBackend, RetrievalError> {
    const _r0 = (() => { const _r = new BincodeReader(stateBuffer); return (() => { const _m = new HashMap<PropertyName, Value | null>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(PropertyName.decode(_r), _r.readOption((r) => Value.decode(r))); } return _m; })(); })();
    if (_r0.isErr()) return Result.Err(RetrievalError.fromBincodeError(_r0.unwrapErr()));
    const rawMap = _r0.unwrap();
    const map = unsupported('`collect` builds whatever its target type names, and the engine could not name the type this one is collected into');
    return Result.Ok(new LWWBackend(new RwLock(map), new Mutex(new HashMap<string, Broadcast<void>>())));
  }

  toOperations(): Result<Operation[] | null, MutationError> {
    let values = this.values.write();
    try {
      let changedValues = new HashMap();
      for (const [name, entry] of values.value.iterMut()) {
        if (!entry.committed) {
          changedValues.insert(name, entry.value.clone());
          entry.committed = true;
        }
      }
      if (changedValues.length === 0) {
        return Result.Ok(null);
      }
      const _r0 = (() => { const _w = new BincodeWriter(); changedValues.encode(_w); return _w.finish(); })();
      if (_r0.isErr()) return Result.Err(MutationError.fromBincodeError(_r0.unwrapErr()));
      const _t1 = new LWWDiff(LWW_DIFF_VERSION, _r0.unwrap());
      try {
        const _r2 = (() => { const _w = new BincodeWriter(); _t1.encode(_w); return _w.finish(); })();
        if (_r2.isErr()) return Result.Err(MutationError.fromBincodeError(_r2.unwrapErr()));
        return Result.Ok([new Operation(_r2.unwrap())]);
      } finally {
        _t1.drop();
      }
    } finally {
      values.drop();
    }
  }

  applyOperations(operations: Operation[]): Result<void, MutationError> {
    let changedFields = [];
    for (const operation of operations) {
      const _r0 = (() => { const _r = new BincodeReader(operation.diff); return _r; })();
      if (_r0.isErr()) return Result.Err(MutationError.fromBincodeError(_r0.unwrapErr()));
      const { version, data } = _r0.unwrap();
      const _v = version;
      if (_v === 1) {
        const _r1 = (() => { const _r = new BincodeReader(data); return (() => { const _m = new HashMap<string, Value | null>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(_r.readString(), _r.readOption((r) => Value.decode(r))); } return _m; })(); })();
        if (_r1.isErr()) return Result.Err(MutationError.fromBincodeError(_r1.unwrapErr()));
        let _moved2 = false;
        const changes = _r1.unwrap();
        try {
          let values = this.values.write();
          try {
            _moved2 = true;
            const _seq3 = changes.intoEntries();
            let _at4 = 0;
            try {
              while (_at4 < _seq3.length) {
                const [propertyName, newValue] = _seq3[_at4++];
                values.value.set(propertyName, new ValueEntry(newValue, true));
                changedFields.push(propertyName);
              }
            } finally {
              dropOwned(_seq3.slice(_at4));
            }
          } finally {
            values.drop();
          }
        } finally {
          if (!_moved2) dropOwned(changes);
        }
      } else {
        const version = _v;
        return Result.Err(new MutationError('UpdateFailed', { _0: AnyhowError.msg(`Unknown LWW operation version: ${version}`) }))
      }
    }
    const fieldBroadcasts = this.fieldBroadcasts.lock();
    try {
      for (const fieldName of changedFields) {
        {
          const _v1 = fieldBroadcasts.value.get(fieldName);
          if (_v1 != null) {
            const broadcast = _v1;
            broadcast.send([]);
          }
        }
      }
      return Result.Ok([]);
    } finally {
      fieldBroadcasts.drop();
    }
  }

  listenField(fieldName: PropertyName, listener: Listener): ListenerGuard {
    let fieldBroadcasts = this.fieldBroadcasts.lock();
    try {
      const broadcast = fieldBroadcasts.value.entry(fieldName).orDefault(() => Broadcast.default());
      const _t0 = broadcast.value.reference();
      try {
        return ListenerGuard.from(_t0.listen(listener));
      } finally {
        _t0.drop();
      }
    } finally {
      fieldBroadcasts.drop();
    }
  }

  debug(): string {
    return `LWWBackend { values: ${this.values}, fieldBroadcasts: ${this.fieldBroadcasts} }`;
  }
}

export class LWWDiff extends Struct {
  version: number;
  data: Uint8Array;

  constructor(version: number, data: Uint8Array) {
    super();
    this.version = version;
    this.data = data;
  }

  encode(writer: BincodeWriter): void {
    writer.writeU8(this.version);
    writer.writeByteVec(this.data);
  }

  static decode(reader: BincodeReader): LWWDiff {
    const version = reader.readU8();
    const data = reader.readByteVec();
    return new LWWDiff(version, data);
  }

  toJSON(): unknown {
    return { 'version': this.version, 'data': Array.from(this.data) };
  }

  static fromJson(value: unknown): Result<LWWDiff, JsonError> {
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `LWWDiff`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('version' in _o)) {
        return Result.Err(JsonError.custom('missing field `version`'));
      }
      const _rversion = ((v: unknown) => (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 255 ? Result.Ok(v as number) : Result.Err(JsonError.custom('expected a u8'))))(_o['version']);
      if (_rversion.isErr()) return Result.Err(_rversion.unwrapErr());
      const version = _rversion.unwrap();
      if (!('data' in _o)) {
        return Result.Err(JsonError.custom('missing field `data`'));
      }
      const _rdata = ((v: unknown) => (Array.isArray(v) && v.every((b) => typeof b === 'number' && Number.isInteger(b) && b >= 0 && b <= 255) ? Result.Ok(new Uint8Array(v as number[])) : Result.Err(JsonError.custom('expected an array of bytes'))))(_o['data']);
      if (_rdata.isErr()) return Result.Err(_rdata.unwrapErr());
      const data = _rdata.unwrap();
      return Result.Ok(new LWWDiff(version, data));
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    }
  }
}

const LWW_DIFF_VERSION: number = 1;

