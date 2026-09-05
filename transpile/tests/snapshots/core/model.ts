// MIRRORS: ankurah/core/src/model.rs
import { Struct, Result, Arc, OwnedClosure } from '@ankurah/base';
import { CollectionId, EntityId, State } from '@ankurah/proto';
import { Entity } from './entity';
import { SubscriptionGuard } from '@ankurah/signals';

export class MutableBorrow<T extends Mutable> extends Struct {
  mutable: T;
  _entityRef: Entity;

  constructor(mutable: T, _entityRef: Entity) {
    super();
    this.mutable = mutable;
    this._entityRef = _entityRef;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [this.mutable];
  }

  static new<T>(entityRef: Entity): MutableBorrow<T> {
    return new MutableBorrow(T.new(entityRef.clone()), entityRef);
  }

  intoCore(): T {
    try {
      return this.mutable;
    } finally {
      this.drop();
    }
  }

  deref(): T {
    return this.mutable;
  }

  derefMut(): T {
    return this.mutable;
  }

  debug(): string {
    return `MutableBorrow { mutable: ${this.mutable}, _entityRef: ${this._entityRef.debug()} }`;
  }
}

export interface Model {
  collection(): CollectionId;
  initializeNewEntity(entity: Entity): void;
}

export abstract class View {
  id(): EntityId {
    return this.entity().id();
  }
  collection(): CollectionId {
    return Model.collection();
  }
  abstract entity(): Entity;
  abstract fromEntity(inner: Entity): Self;
  abstract toModel(): Result<Model, PropertyError>;
}

export abstract class Mutable {
  id(): EntityId {
    return this.entity().id();
  }
  collection(): CollectionId {
    return Model.collection();
  }
  abstract entity(): Entity;
  abstract new(entity: Entity): Self;
  state(): Result<State, StateError> {
    return this.entity().toState();
  }
  read(): View {
    const inner = this.entity();
    const newInner = (() => {
      return inner.deref().kind.match({
        Transacted: (v) => {
          const upstream = v.upstream;
          return upstream.clone();
        },
        Primary: () => inner.clone(),
      });
    })();
    return Mutable.fromEntity(newInner);
  }
}

export function viewSubscribe<V, F>(view: V, listener: F): SubscriptionGuard {
  const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
  const viewClone = view.clone();
  const subscription = view.listen(Arc.new(new OwnedClosure([listener_1], (_) => {
    listener_1(viewClone.clone());
  })));
  return SubscriptionGuard.new(subscription);
}

export function viewSubscribeNoClone<V, F>(view: V, listener: F): SubscriptionGuard {
  const listener_1 = IntoSubscribeListener_dispatch_intoSubscribeListener(listener);
  const subscription = view.listen(Arc.new(new OwnedClosure([listener_1], (_) => {
    listener_1([]);
  })));
  return SubscriptionGuard.new(subscription);
}

