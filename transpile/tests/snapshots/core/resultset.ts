// MIRRORS: ankurah/core/src/resultset.rs
import { Struct, Enum, Drop, Arc, Mutex, MutexGuard, OwnedClosure, dropOwned, checkedAdd, HashMap } from '@ankurah/base';
import { Broadcast, BroadcastId, CurrentObserver, Get, IntoSubscribeListener, Listener, ListenerGuard, Peek, Signal, Subscribe, SubscriptionGuard } from '@ankurah/signals';
import { Entity } from './entity';
import { View } from './indexel';
import { encodeTupleValuesWithKeySpec } from './indexing/encoding';
import { KeySpec } from './indexing/key_spec';
import { AbstractEntity } from './reactor';
import { EntityId } from '@ankurah/proto';
import { Broadcast, BroadcastId, CurrentObserver, Get, ListenerGuard, Peek, Signal, Subscribe, SubscriptionGuard } from '@ankurah/signals';

export class EntityResultSet<E extends AbstractEntity = Entity> extends Struct implements Signal {
  _0: Arc<Inner<E>>;

  constructor(_0: Arc<Inner<E>>) {
    super();
    this._0 = _0;
  }

  static fromVec<E>(entities: E[], loaded: boolean): EntityResultSet<E> {
    let index = new HashMap();
    let order = [];
    for (const [i, entity] of [...entities].entries()) {
      index.insert(entity.id(), i);
      order.push(new EntityEntry(entity, null, false));
    }
    const state = new State(order, index, null, null, false);
    return new EntityResultSet(Arc.new(new Inner(new Mutex(state), AtomicBool.new(loaded), Broadcast.new())));
  }

  static empty<E>(): EntityResultSet<E> {
    const state = new State([], new HashMap(), null, null, false);
    return new EntityResultSet(Arc.new(new Inner(new Mutex(state), AtomicBool.new(false), Broadcast.new())));
  }

  static single<E>(entity: E): EntityResultSet<E> {
    const entry = new EntityEntry(entity.clone(), null, false);
    let state = new State([entry], new HashMap(), null, null, false);
    state.index.insert(entity.id(), 0);
    return new EntityResultSet(Arc.new(new Inner(new Mutex(state), AtomicBool.new(false), Broadcast.new())));
  }

  write(): ResultSetWrite<E> {
    const guard = this._0.value.state.lock();
    return new ResultSetWrite(this, false, guard);
  }

  read(): ResultSetRead<E> {
    const guard = this._0.value.state.lock();
    return new ResultSetRead(guard);
  }

  setLoaded(loaded: boolean): void {
    this._0.value.loaded = loaded;
    this._0.value.broadcast.send([]);
  }

  isLoaded(): boolean {
    CurrentObserver.track(this);
    return this._0.value.loaded;
  }

  clear(): void {
    let st = this._0.value.state.lock();
    st.value.order.length = 0;
    st.value.index.clear();
    st.drop();
    this._0.value.broadcast.send([]);
  }

  keys(): EntityResultSetKeyIterator {
    CurrentObserver.track(this);
    const st = this._0.value.state.lock();
    try {
      const keys = [...st.value.order].map((e) => e.entity.id());
      return EntityResultSetKeyIterator.new(keys);
    } finally {
      st.drop();
    }
  }

  containsKey(id: EntityId): boolean {
    CurrentObserver.track(this);
    const st = this._0.value.state.lock();
    try {
      return st.value.index.has(id);
    } finally {
      st.drop();
    }
  }

  byId(id: EntityId): E | null {
    CurrentObserver.track(this);
    const st = this._0.value.state.lock();
    try {
      return st.value.index.get(id) != null ? ((i) => st.value.order[i].entity.clone())(st.value.index.get(id)!) : null;
    } finally {
      st.drop();
    }
  }

  len(): number {
    CurrentObserver.track(this);
    const st = this._0.value.state.lock();
    try {
      return st.value.order.length;
    } finally {
      st.drop();
    }
  }

  isGapDirty(): boolean {
    const st = this._0.value.state.lock();
    try {
      return st.value.gapDirty;
    } finally {
      st.drop();
    }
  }

  clearGapDirty(): void {
    let st = this._0.value.state.lock();
    try {
      st.value.gapDirty = false;
    } finally {
      st.drop();
    }
  }

  getLimit(): number | null {
    const st = this._0.value.state.lock();
    try {
      return st.value.limit;
    } finally {
      st.drop();
    }
  }

  lastEntity(): E | null {
    const st = this._0.value.state.lock();
    try {
      return st.value.order.at(-1) != null ? ((entry) => entry.entity.clone())(st.value.order.at(-1)!) : null;
    } finally {
      st.drop();
    }
  }

  orderBy(keySpec: KeySpec | null): void {
    try {
      let _moved0 = false;
      let st = this._0.value.state.lock();
      try {
        if (st.value.keySpec === keySpec) {
          return;
        }
        const _a1 = keySpec.clone();
        dropOwned(st.value.keySpec);
        st.value.keySpec = _a1;
        const _seq3 = st.value.order;
        let _at4 = 0;
        try {
          while (_at4 < _seq3.length) {
            const entry = _seq3[_at4++];
            try {
              const _a2 = (() => {
                {
                  const _v = keySpec;
                  if (_v != null) {
                    const ks = _v;
                    return ResultSetWrite.computeSortKey(entry.entity, ks);
                  } else {
                  return null;
                }
                }
              })();
              dropOwned(entry.sortKey);
              entry.sortKey = _a2;
            } finally {
              entry.drop();
            }
          }
        } finally {
          dropOwned(_seq3.slice(_at4));
        }
        st.value.order.sort((a, b) => {
          const _v1 = [a.sortKey, b.sortKey];
          if ((_v1[0] != null) && (_v1[1] != null)) {
            const keyA = _v1[0];
            const keyB = _v1[1];
            const _v2 = keyA.compareTo(keyB);
            if (_v2 === 0) {
              return a.entity.id().compareTo(b.entity.id());
            } else {
              const other = _v2;
              return other;
            }
          } else if ((_v1[0] != null) && (_v1[1] == null)) {
            return 1;
          } else if ((_v1[0] == null) && (_v1[1] != null)) {
            return -1;
          } else {
            return a.entity.id().compareTo(b.entity.id());
          }
        });
        st.value.index.clear();
        const indexUpdates = [...st.value.order].entries().map(([i, entry]) => [entry.entity.id(), i]);
        for (const [id, i] of indexUpdates) {
          st.value.index.set(id, i);
        }
        _moved0 = true;
        st.drop();
        this._0.value.broadcast.send([]);
      } finally {
        if (!_moved0) st.drop();
      }
    } finally {
      dropOwned(keySpec);
    }
  }

  limit(limit: number | null): void {
    let _moved0 = false;
    let st = this._0.value.state.lock();
    try {
      if (st.value.limit === limit) {
        return;
      }
      st.value.limit = limit;
      let entitiesRemoved = false;
      {
        const _v = limit;
        if (_v != null) {
          const limit = _v;
          if (st.value.order.length > limit) {
            st.value.order.length = limit;
            entitiesRemoved = true;
            st.value.index.clear();
            const indexUpdates = [...st.value.order].entries().map(([i, entry]) => [entry.entity.id(), i]);
            for (const [id, i] of indexUpdates) {
              st.value.index.set(id, i);
            }
          }
        }
      }
      _moved0 = true;
      st.drop();
      if (entitiesRemoved) {
        this._0.value.broadcast.send([]);
      }
    } finally {
      if (!_moved0) st.drop();
    }
  }

  wrap<R extends View>(): ResultSet<R> {
    return new ResultSet(this.clone(), undefined /* PhantomData */);
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this._0.value.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(listener));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this._0.value.broadcast.id();
  }

  clone(): EntityResultSet<E> {
    return new EntityResultSet(this._0.clone());
  }

  debug(): string {
    return `EntityResultSet(${this._0.value.debug()})`;
  }
}

export class ResultSet<R extends View> extends Struct implements Signal, Get<E[]>, Peek<E[]>, Subscribe<E[]> {
  _0: EntityResultSet<Entity>;

  constructor(_0: EntityResultSet<Entity>) {
    super();
    this._0 = _0;
  }

  byId(id: EntityId): R | null {
    return this._0.byId(id) != null ? ((e) => R.fromEntity(e))(this._0.byId(id)!) : null;
  }

  iter(): ResultSetIter<E> {
    return ResultSetIter.new(this.clone());
  }

  deref(): EntityResultSet<Entity> {
    return this._0;
  }

  clone<E>(): ResultSet<E> {
    return new ResultSet(this._0.clone(), undefined /* PhantomData */);
  }

  static default<R, E>(): ResultSet<E> {
    const entityResultset = EntityResultSet.empty();
    return new ResultSet(entityResultset, undefined /* PhantomData */);
  }

  listen(listener: Listener): ListenerGuard {
    const _t0 = this._0._0.value.broadcast.reference();
    try {
      return ListenerGuard.new(_t0.listen(listener));
    } finally {
      _t0.drop();
    }
  }

  broadcastId(): BroadcastId {
    return this._0._0.value.broadcast.id();
  }

  get<E>(): E[] {
    CurrentObserver.track(this);
    const _t0 = this._0._0.value.state.lock();
    try {
      return [..._t0.value.order].map((e) => E.fromEntity(e.entity.clone()));
    } finally {
      _t0.drop();
    }
  }

  peek<E>(): E[] {
    const _t0 = this._0._0.value.state.lock();
    try {
      return [..._t0.value.order].map((e) => E.fromEntity(e.entity.clone()));
    } finally {
      _t0.drop();
    }
  }

  subscribe<F>(listener: F): SubscriptionGuard {
    const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
    const me = this.clone();
    const _t0 = this._0._0.value.broadcast.reference();
    try {
      const guard = _t0.listen(new OwnedClosure([me, listener_1], (_) => {
        const _t1 = me._0._0.value.state.lock();
        try {
          const entities = [..._t1.value.order].map((e) => E.fromEntity(e.entity.clone()));
          _t1.drop();
          listener_1(entities);
        } finally {
          _t1.drop();
        }
      }));
      return SubscriptionGuard.new(ListenerGuard.new(guard));
    } finally {
      _t0.drop();
    }
  }

  debug(): string {
    return `ResultSet(${this._0.debug()}, ${this._1})`;
  }
}

class Inner<E extends AbstractEntity> extends Struct {
  state: Mutex<State<E>>;
  loaded: boolean;
  broadcast: Broadcast<void>;

  constructor(state: Mutex<State<E>>, loaded: boolean, broadcast: Broadcast<void>) {
    super();
    this.state = state;
    this.loaded = loaded;
    this.broadcast = broadcast;
  }

  debug(): string {
    return `Inner { state: ${this.state}, loaded: ${this.loaded}, broadcast: ${this.broadcast.debug()} }`;
  }
}

class State<E extends AbstractEntity> extends Struct {
  order: EntityEntry<E>[];
  index: HashMap<EntityId, number>;
  keySpec: KeySpec | null;
  limit: number | null;
  gapDirty: boolean;

  constructor(order: EntityEntry<E>[], index: HashMap<EntityId, number>, keySpec: KeySpec | null, limit: number | null, gapDirty: boolean) {
    super();
    this.order = order;
    this.index = index;
    this.keySpec = keySpec;
    this.limit = limit;
    this.gapDirty = gapDirty;
  }

  debug(): string {
    return `State { order: ${`[${Array.from(this.order).map((e) => e.debug()).join(', ')}]`}, index: ${this.index}, keySpec: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(this.keySpec)}, limit: ${(($v) => $v === null ? 'None' : `Some(${String($v)})`)(this.limit)}, gapDirty: ${String(this.gapDirty)} }`;
  }
}

class EntityEntry<E extends AbstractEntity> extends Struct {
  entity: E;
  sortKey: IVec | null;
  dirty: boolean;

  constructor(entity: E, sortKey: IVec | null, dirty: boolean) {
    super();
    this.entity = entity;
    this.sortKey = sortKey;
    this.dirty = dirty;
  }

  clone(): EntityEntry<E> {
    return new EntityEntry(this.entity.clone(), this.sortKey?.clone() ?? null, this.dirty);
  }

  debug(): string {
    return `EntityEntry { entity: ${this.entity}, sortKey: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(this.sortKey)}, dirty: ${String(this.dirty)} }`;
  }
}

export class ResultSetWrite<E extends AbstractEntity = Entity> extends Drop {
  resultset: EntityResultSet<E>;
  changed: boolean;
  guard: MutexGuard<State<E>> | null;

  constructor(resultset: EntityResultSet<E>, changed: boolean, guard: MutexGuard<State<E>> | null) {
    super();
    this.resultset = resultset;
    this.changed = changed;
    this.guard = guard;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [this.changed, this.guard];
  }

  add(entity: E): boolean {
    const guard = this.guard.asMut();
    const id = entity.id();
    if (guard.value.index.has(id)) {
      return false;
    }
    const sortKey = guard.value.keySpec.asRef() != null ? ((keySpec) => ResultSetWrite.Self.computeSortKey(entity, keySpec))(guard.value.keySpec.asRef()!) : null;
    const entry = new EntityEntry(entity, sortKey, false);
    const pos = guard.value.order.binarySearchBy((existing) => {
      const _v = [existing.sortKey, entry.sortKey];
      if ((_v[0] != null) && (_v[1] != null)) {
        const existingKey = _v[0];
        const entryKey = _v[1];
        return existingKey.compareTo(entryKey).thenWith(() => existing.entity.id().compareTo(entry.entity.id()));
      } else if ((_v[0] != null) && (_v[1] == null)) {
        return -1;
      } else if ((_v[0] == null) && (_v[1] != null)) {
        return 1;
      } else {
        return existing.entity.id().compareTo(entry.entity.id());
      }
    }).unwrapOrElse((pos) => pos);
    guard.value.order.splice(pos, 0, entry);
    guard.value.index.set(id, pos);
    for (const i of undefined /* range (checkedAdd(pos, 1, 'usize'))..guard.value.order.length */) {
      const entryId = guard.value.order[i].entity.id();
      guard.value.index.set(entryId, i);
    }
    {
      const _v2 = guard.value.limit;
      if (_v2 != null) {
        const limit = _v2;
        if (guard.value.order.length > limit) {
          {
            const _v1 = guard.value.order.pop();
            if (_v1 != null) {
              const removedEntry = _v1;
              try {
                const removedId = removedEntry.entity.id();
                guard.value.index.delete(removedId);
              } finally {
                removedEntry.drop();
              }
            }
          }
        }
      }
    }
    this.changed = true;
    return true;
  }

  remove(id: EntityId): boolean {
    const guard = this.guard.asMut();
    {
      const _v = guard.value.index.remove(id);
      if (_v != null) {
        const idx = _v;
        if (guard.value.limit.isSomeAnd((limit) => guard.value.order.length === limit)) {
          guard.value.gapDirty = true;
        }
        guard.value.order.splice(idx, 1)[0];
        if (idx < guard.value.order.length) {
          fixFrom(guard, idx);
        }
        this.changed = true;
        return true;
      } else {
      return false;
    }
    }
  }

  contains(id: EntityId): boolean {
    return this.guard.asRef().value.index.has(id);
  }

  iterEntities(): [EntityId, E][] {
    const guard = this.guard.asRef();
    return [...guard.value.order].map((entry) => [entry.entity.id(), entry.entity]);
  }

  markAllDirty(): void {
    const guard = this.guard.asMut();
    const _seq0 = guard.value.order;
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const entry = _seq0[_at1++];
        try {
          entry.dirty = true;
        } finally {
          entry.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    this.changed = true;
  }

  retainDirty<F>(shouldRetain: F): EntityId[] {
    const guard = this.guard.asMut();
    let removedIds = [];
    let i = 0;
    const wasAtLimit = guard.value.limit.isSomeAnd((limit) => guard.value.order.length === limit);
    while (i < guard.value.order.length) {
      if (guard.value.order[i].dirty) {
        const shouldKeep = shouldRetain(guard.value.order[i].entity);
        if (shouldKeep) {
          const keySpec = guard.value.keySpec.clone();
          {
            const _v = keySpec;
            if (_v != null) {
              const keySpec = _v;
              try {
                guard.value.order[i].sortKey = ResultSetWrite.Self.computeSortKey(guard.value.order[i].entity, keySpec);
              } finally {
                keySpec.drop();
              }
            } else {
            dropOwned(_v);
          }
          }
          guard.value.order[i].dirty = false;
          i = checkedAdd(i, 1, 'i32');
        } else {
          const removedEntry = guard.value.order.splice(i, 1)[0];
          try {
            const removedId = removedEntry.entity.id();
            guard.value.index.delete(removedId);
            removedIds.push(removedId);
          } finally {
            removedEntry.drop();
          }
        }
      } else {
        i = checkedAdd(i, 1, 'i32');
      }
    }
    guard.value.index.clear();
    const indexUpdates = [...guard.value.order].entries().map(([i, entry]) => [entry.entity.id(), i]);
    for (const [id, i] of indexUpdates) {
      guard.value.index.set(id, i);
    }
    if (!(removedIds.length === 0)) {
      this.changed = true;
      if ((!guard.value.gapDirty) && wasAtLimit && guard.value.limit.isSomeAnd((limit) => guard.value.order.length < limit)) {
        guard.value.gapDirty = true;
      }
    }
    return removedIds;
  }

  replaceAll(entities: E[]): void {
    const guard = this.guard.asMut();
    guard.value.order.length = 0;
    guard.value.index.clear();
    for (const entity of entities) {
      const sortKey = guard.value.keySpec.asRef() != null ? ((keySpec) => ResultSetWrite.Self.computeSortKey(entity, keySpec))(guard.value.keySpec.asRef()!) : null;
      const entry = new EntityEntry(entity, sortKey, false);
      guard.value.order.push(entry);
    }
    if (guard.value.keySpec != null) {
      guard.value.order.sort((a, b) => {
        const _v = [a.sortKey, b.sortKey];
        if ((_v[0] != null) && (_v[1] != null)) {
          const keyA = _v[0];
          const keyB = _v[1];
          return (($c) => $c !== 0 ? $c : (() => a.entity.id().compareTo(b.entity.id()))())(keyA.compareTo(keyB));
        } else if ((_v[0] != null) && (_v[1] == null)) {
          return -1;
        } else if ((_v[0] == null) && (_v[1] != null)) {
          return 1;
        } else {
          return a.entity.id().compareTo(b.entity.id());
        }
      });
    } else {
      guard.value.order.sort((a, b) => a.entity.id().compareTo(b.entity.id()));
    }
    {
      const _v1 = guard.value.limit;
      if (_v1 != null) {
        const limit = _v1;
        if (guard.value.order.length > limit) {
          guard.value.order.length = limit;
        }
      }
    }
    const indexUpdates = [...guard.value.order].entries().map(([i, entry]) => [entry.entity.id(), i]);
    for (const [id, i] of indexUpdates) {
      guard.value.index.set(id, i);
    }
    this.changed = true;
  }

  static computeSortKey<E>(entity: E, keySpec: KeySpec): IVec {
    let values = [];
    const _seq0 = keySpec.keyparts;
    let _at1 = 0;
    try {
      while (_at1 < _seq0.length) {
        const keypart = _seq0[_at1++];
        try {
          const value = AbstractEntity.value(entity, keypart.column);
          {
            const _v = value;
            if (_v != null) {
              const v = _v;
              values.push(v);
            } else {
            return IVec.fromSlice([]);
          }
          }
        } finally {
          keypart.drop();
        }
      }
    } finally {
      dropOwned(_seq0.slice(_at1));
    }
    const encoded = encodeTupleValuesWithKeySpec(values, keySpec).unwrapOrDefault();
    return IVec.from(encoded);
  }

  setLoaded(loaded: boolean): void {
    this.resultset._0.value.loaded = loaded;
    this.changed = true;
  }

  protected override onDrop(): void {
    if (this.changed) {
      dropOwned(this.guard.take());
      this.resultset._0.value.broadcast.send([]);
    }
  }
}

export class ResultSetRead<E extends AbstractEntity = Entity> extends Struct {
  guard: MutexGuard<State<E>>;

  constructor(guard: MutexGuard<State<E>>) {
    super();
    this.guard = guard;
  }

  contains(id: EntityId): boolean {
    return this.guard.value.index.has(id);
  }

  iterEntities(): [EntityId, E][] {
    return [...this.guard.value.order].map((entity) => [entity.entity.id(), entity.entity]);
  }

  len(): number {
    return this.guard.value.order.length;
  }

  isEmpty(): boolean {
    return this.guard.value.order.length === 0;
  }
}

export class ResultSetIter<E extends View & Clone> extends Struct {
  resultset: ResultSet<E>;
  index: number;

  constructor(resultset: ResultSet<E>, index: number) {
    super();
    this.resultset = resultset;
    this.index = index;
  }

  static new<E>(resultset: ResultSet<E>): ResultSetIter<E> {
    return new ResultSetIter(resultset, 0);
  }

  next(): E | null {
    CurrentObserver.track(this.resultset);
    const state = this.resultset._0._0.value.state.lock();
    try {
      if (this.index < state.value.order.length) {
        const entity = state.value.order[this.index].entity;
        try {
          const view = E.fromEntity(entity.clone());
          this.index = checkedAdd(this.index, 1, 'usize');
          return view;
        } finally {
          entity.drop();
        }
      } else {
        return null;
      }
    } finally {
      state.drop();
    }
  }

  debug(): string {
    return `ResultSetIter { resultset: ${this.resultset.debug()}, index: ${String(this.index)} }`;
  }
}

export class EntityResultSetKeyIterator extends Struct {
  keys: EntityId[];
  index: number;

  constructor(keys: EntityId[], index: number) {
    super();
    this.keys = keys;
    this.index = index;
  }

  static new(keys: EntityId[]): EntityResultSetKeyIterator {
    return new EntityResultSetKeyIterator(keys, 0);
  }

  next(): EntityId | null {
    if (this.index < this.keys.length) {
      const key = this.keys[this.index];
      this.index = checkedAdd(this.index, 1, 'usize');
      return key;
    } else {
      return null;
    }
  }

  debug(): string {
    return `EntityResultSetKeyIterator { keys: ${`[${Array.from(this.keys).map((e) => e.debug()).join(', ')}]`}, index: ${String(this.index)} }`;
  }
}

type IVecV = {
  Small: { _0: Uint8Array };
  Large: { _0: Uint8Array };
};

class IVec extends Enum<IVecV> {

  static fromSlice(bytes: Uint8Array): IVec {
    if (bytes.length <= 16) {
      let data = Array(16).fill(0);
      data.slice(0, bytes.length).copyFromSlice(bytes);
      return IVec.Self.Small(data);
    } else {
      return IVec.Self.Large(bytes.slice());
    }
  }

  static from(vec: Uint8Array): IVec {
    return IVec.Self.fromSlice(vec);
  }

  clone(): IVec {
    return new IVec(this.type, { ...this.value });
  }

  equals(other: IVec): boolean {
    return true;
  }

  compareTo(other: IVec): number {
    const order = ['Small', 'Large'];
    const a = order.indexOf(this.type);
    const b = order.indexOf(other.type);
    if (a !== b) return a < b ? -1 : 1;
    switch (this.type) {
      case 'Small': {
        let c = ((xs, ys) => { const n = Math.min(xs.length, ys.length); for (let i = 0; i < n; i++) { const a = xs[i], b = ys[i]; const d = a < b ? -1 : a > b ? 1 : 0; if (d !== 0) return d; } return Math.sign(xs.length - ys.length); })((this.value as any)._0, (other.value as any)._0);
        if (c !== 0) return c;
        return 0;
      }
      case 'Large': {
        let c = ((xs, ys) => { const n = Math.min(xs.length, ys.length); for (let i = 0; i < n; i++) { const a = xs[i], b = ys[i]; const d = a < b ? -1 : a > b ? 1 : 0; if (d !== 0) return d; } return Math.sign(xs.length - ys.length); })((this.value as any)._0, (other.value as any)._0);
        if (c !== 0) return c;
        return 0;
      }
    }
    return 0;
  }

  debug(): string {
    return this.match({
      Small: (v) => `Small(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
      Large: (v) => `Large(${`[${Array.from(v._0).map((e) => String(e)).join(', ')}]`})`,
    });
  }
}

function fixFrom<E extends AbstractEntity>(st: State<E>, start: number): void {
  for (const i of undefined /* range start..st.order.length */) {
    const id = st.order[i].entity.id();
    st.index.set(id, i);
  }
}

