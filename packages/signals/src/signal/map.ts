// MIRRORS: ankurah/signals/src/signal/map.rs
import { Struct, Arc } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from '../broadcast';
import { SubscriptionGuard } from '../porcelain/subscribe';

export class Map<Upstream, Input, Output, Transform> extends Struct implements Signal, With, Get, Peek, Subscribe {
  source: Upstream;
  transform: Transform;

  constructor(source: Upstream, transform: Transform) {
    super();
    this.source = source;
    this.transform = transform;
  }

  static new<Upstream, Input, Output, Transform>(source: Upstream, transform: Transform): Map<Upstream, Input, Output, Transform> {
    return new Map(source, transform, undefined /* PhantomData */);
  }

  clone(): Map<Upstream, Input, Output, Transform> {
    return new Map(this.source.clone(), this.transform.clone(), undefined /* PhantomData */);
  }
}

