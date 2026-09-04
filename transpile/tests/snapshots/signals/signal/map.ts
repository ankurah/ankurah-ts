// MIRRORS: ankurah/signals/src/signal/map.rs
import { Struct, Arc } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from '../broadcast';
import { CurrentObserver } from '../context';
import { Subscribe, SubscriptionGuard } from '../porcelain/subscribe';
import { Get, Peek, Signal, With } from '../signal';

export class Map<Upstream extends Signal & With<Input> & Clone, Input, Output extends Clone, Transform extends Fn & Clone> extends Struct implements Signal, With<Output>, Get<Output>, Peek<Output>, Subscribe<Output> {
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

  listen(listener: Listener): ListenerGuard {
    return this.source.listen(listener);
  }

  broadcastId(): BroadcastId {
    return this.source.broadcastId();
  }

  with<R>(f: (arg0: Output) => R): R {
    CurrentObserver.track(this.source);
    return this.source.with((input) => {
      const output = (this.transform)(input);
      const _ret = f(output);
      output.drop();
      return _ret;
    });
  }

  get(): Output {
    CurrentObserver.track(this.source);
    return this.source.with((input) => (this.transform)(input));
  }

  peek(): Output {
    return this.source.with((input) => (this.transform)(input));
  }

  subscribe<L>(listener: L): SubscriptionGuard {
    const listener_1 = listener.intoSubscribeListener();
    const source = this.source.clone();
    const transform = this.transform.clone();
    const subscription = this.source.listen(Arc.new((_) => {
      source.with((input) => {
        listener_1(transform(input));
      });
    }));
    const _ret = SubscriptionGuard.new(subscription);
    transform.drop();
    source.drop();
    listener.drop();
    return _ret;
  }
}

