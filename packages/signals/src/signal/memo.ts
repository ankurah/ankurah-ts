// MIRRORS: ankurah/signals/src/signal/memo.rs
import { Struct, Arc, RwLock, Ref } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from '../broadcast';
import { SubscriptionGuard } from '../porcelain/subscribe';

export class Memo<Upstream, Input, Output, Transform> extends Struct implements Signal, With, Get, Peek, Subscribe {
  source: Upstream;
  transform: Transform;
  cached: Arc<RwLock<Output | null>>;
  Subscription: ListenerGuard;

  constructor(source: Upstream, transform: Transform, cached: Arc<RwLock<Output | null>>, Subscription: ListenerGuard) {
    super();
    this.source = source;
    this.transform = transform;
    this.cached = cached;
    this.Subscription = Subscription;
  }

  static new<Upstream, Input, Output, Transform>(source: Upstream, transform: Transform): Memo<Upstream, Input, Output, Transform> {
    const cached = Arc.new(new RwLock(null));
    const cachedRef = cached.clone();
    const subscription = source.listen(Arc.new((_) => {
      cachedRef.write().value = null;
    }));
    const _ret = new Memo(source, transform, cached, subscription, undefined /* PhantomData */);
    cachedRef.drop();
    return _ret;
  }

  withCached<R>(f: (arg0: Output) => R): R {
    (() => {
      const guard = this.cached.value.read().value.unwrap();
      if (guard != null) {
        const value = guard;
        return f(value);
      }
    })()
    let guard = this.cached.value.write().value.unwrap();
    if (guard == null) {
      const output = this.source.with((input) => (this.transform)(input));
      guard.value = output;
      output.drop();
    }
    const _ret = f(guard.asRef());
    guard.drop();
    return _ret;
  }

  clone(): Memo<Upstream, Input, Output, Transform> {
    return new Memo(this.source.clone(), this.transform.clone());
  }
}

