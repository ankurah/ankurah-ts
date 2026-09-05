// MIRRORS: ankurah/core/src/property/backend/yrs.rs
import { Struct, Result, Arc, Mutex, OwnedClosure, HashMap, HashSet } from '@ankurah/base';
import { MutationError, RetrievalError, StateError } from '../../error';
import { Transaction } from '../../transaction';
import { Value } from '../../value/index';
import { PropertyBackend } from './index';
import { Operation } from '@ankurah/proto';
import { Broadcast, BroadcastId, ListenerGuard } from '@ankurah/signals';

export class YrsBackend extends Struct implements PropertyBackend {
  doc: Doc;
  previousState: Mutex<StateVector>;
  fieldBroadcasts: Mutex<HashMap<PropertyName, Broadcast>>;

  constructor(doc: Doc, previousState: Mutex<StateVector>, fieldBroadcasts: Mutex<HashMap<PropertyName, Broadcast>>) {
    super();
    this.doc = doc;
    this.previousState = previousState;
    this.fieldBroadcasts = fieldBroadcasts;
  }

  static new(): YrsBackend {
    const doc = yrs.Doc.new();
    const startingState = doc.transact().stateVector();
    return new YrsBackend(doc, new Mutex(startingState), new Mutex(new HashMap<string, Broadcast<void>>()));
  }

  getString(propertyName: string): string | null {
    const txn = this.doc.transact();
    const text = txn.getText(propertyName.asRef());
    return (text != null ? ((t) => t.getString(txn))(text!) : null);
  }

  insert(propertyName: string, index: number, value: string): Result<void, MutationError> {
    const text = this.doc.getOrInsertText(propertyName.asRef());
    let ytx = this.doc.transactMut();
    text.insert(ytx, index, value);
    return Result.Ok([]);
  }

  delete(propertyName: string, index: number, length: number): Result<void, MutationError> {
    const text = this.doc.getOrInsertText(propertyName.asRef());
    let ytx = this.doc.transactMut();
    text.removeRange(ytx, index, length);
    return Result.Ok([]);
  }

  applyUpdate(update: Uint8Array, changedFields: Arc<Mutex<HashSet<string>>>): Result<void, MutationError> {
    let txn = this.doc.transactMut();
    const _t0 = this.fieldBroadcasts.lock();
    try {
      const _subs = _t0.value.keys().map((b) => {
        const changedFields = changedFields.clone();
        const b_1 = b;
        return txn.getOrInsertText(b_1).observe(new OwnedClosure([changedFields], (_, __) => {
          let changedFields = changedFields.value.lock();
          try {
            changedFields.value.add(b_1.clone());
          } finally {
            changedFields.drop();
          }
        }));
      });
      _t0.drop();
      const _r1 = Update.decodeV2(update).mapErr((e) => new StateError('SerializationError', { _0: e }));
      if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
      const update_1 = _r1.unwrap();
      const _r2 = txn.applyUpdate(update_1).mapErr((e) => new MutationError('UpdateFailed', { _0: e }));
      if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
      _r2.drop();
      txn.commit();
      return Result.Ok([]);
    } finally {
      _t0.drop();
    }
  }

  getPropertyString(trx: Transaction, propertyName: PropertyName): Value | null {
    const value = (() => {
      const _v1 = trx.getText(propertyName.clone());
      if (_v1 != null) {
        const textRef = _v1;
        {
          const text = textRef.getString(trx);
          return text;
        }
      } else {
        return null;
      }
    })();
    return value.map(Value.String);
  }

  fieldBroadcastId(fieldName: PropertyName): BroadcastId {
    let fieldBroadcasts = this.fieldBroadcasts.lock();
    try {
      const broadcast = fieldBroadcasts.value.entry(fieldName.clone()).orDefault(() => Broadcast.default()).value;
      return broadcast.id();
    } finally {
      fieldBroadcasts.drop();
    }
  }

  static default(): YrsBackend {
    return YrsBackend.new();
  }

  asArcDynAny(): Arc<Any> {
    return this;
  }

  asDebug(): Debug {
    return this;
  }

  fork(): Arc<PropertyBackend> {
    const stateBuffer = this.toStateBuffer().unwrap();
    const backend = YrsBackend.fromStateBuffer(stateBuffer).unwrap();
    return Arc.new(backend);
  }

  properties(): string[] {
    const trx = Transact.transact(this.doc);
    const rootRefs = trx.rootRefs();
    return rootRefs.map(([name, ]) => name.clone());
  }

  propertyValue(propertyName: PropertyName): Value | null {
    const trx = Transact.transact(this.doc);
    return this.getPropertyString(trx, propertyName);
  }

  propertyValues(): HashMap<PropertyName, Value | null> {
    const properties = this.properties();
    let values = new HashMap();
    const trx = Transact.transact(this.doc);
    for (const propertyName of properties) {
      const value = this.getPropertyString(trx, propertyName);
      values.insert(propertyName, value);
    }
    return values;
  }

  static propertyBackendName(): string {
    return 'yrs';
  }

  toStateBuffer(): Result<Uint8Array, StateError> {
    const txn = this.doc.transact();
    const stateBuffer = txn.encodeStateAsUpdateV2(yrs.StateVector.default());
    return Result.Ok(stateBuffer);
  }

  static fromStateBuffer(stateBuffer: Uint8Array): Result<YrsBackend, RetrievalError> {
    const doc = yrs.Doc.new();
    let txn = doc.transactMut();
    const _r0 = yrs.Update.decodeV2(stateBuffer).mapErr((e) => new RetrievalError('FailedUpdate', { _0: e }));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const update = _r0.unwrap();
    const _r1 = txn.applyUpdate(update).mapErr((e) => new RetrievalError('FailedUpdate', { _0: e }));
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    _r1.drop();
    txn.commit();
    void txn;
    const startingState = doc.transact().stateVector();
    return Result.Ok(new YrsBackend(doc, new Mutex(startingState), new Mutex(new HashMap<string, Broadcast<void>>())));
  }

  toOperations(): Result<Operation[] | null, MutationError> {
    let previousState = this.previousState.lock();
    try {
      const txn = this.doc.transactMut();
      const diff = txn.encodeDiffV2(previousState);
      previousState.value = txn.stateVector();
      if (diff === Update.EMPTY_V2) {
        return Result.Ok(null);
      } else {
        return Result.Ok([new Operation(diff)]);
      }
    } finally {
      previousState.drop();
    }
  }

  applyOperations(operations: Operation[]): Result<void, MutationError> {
    const changedFields = Arc.new(new Mutex(new HashSet()));
    for (const operation of operations) {
      const _r0 = this.applyUpdate(operation.diff, changedFields);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      _r0.drop();
    }
    const fieldBroadcasts = this.fieldBroadcasts.lock();
    try {
      for (const fieldName of [...changedFields.lock()]) {
        {
          const _v = fieldBroadcasts.value.get(fieldName);
          if (_v != null) {
            const broadcast = _v;
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
      const broadcast = fieldBroadcasts.value.entry(fieldName.clone()).orDefault(() => Broadcast.default()).value;
      const _t0 = broadcast.reference();
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
    return `YrsBackend { doc: ${this.doc}, previousState: ${this.previousState}, fieldBroadcasts: ${this.fieldBroadcasts} }`;
  }
}

