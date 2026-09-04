// MIRRORS: ankurah/signals/src/reactive_graph.rs
import { Struct, Arc, Mutex } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from './broadcast';
import { Observer } from './observer';
import { Signal } from './signal';

export class ReactiveGraphObserver extends Struct implements Observer {
  bridges: Mutex<Map<BroadcastId, Arc<BridgeSource>>>;
  observerId: number;

  constructor(bridges: Mutex<Map<BroadcastId, Arc<BridgeSource>>>, observerId: number) {
    super();
    this.bridges = bridges;
    this.observerId = observerId;
  }

  static new(): ReactiveGraphObserver {
    const bridges = new Mutex(new Map());
    const observerId = bridges as unknown as number;
    return new ReactiveGraphObserver(bridges, observerId);
  }

  observe(signal: Signal): void {
    const id = signal.broadcastId();
    {
      const _v = this.bridges.lock();
      if (true) {
        const map = _v;
        try {
          const bridge = map.value.entry(id).orInsertWith(() => BridgeSource.new(id, signal)).clone();
          try {
            bridge.value.track();
          } finally {
            bridge.drop();
          }
        } finally {
          map.drop();
        }
      }
    }
  }

  observerId(): number {
    return this.observerId;
  }

  asAny(): Any {
    return this;
  }
}

class BridgeSource extends Struct {
  broadcastId: BroadcastId;
  trigger: ArcRwSignal<void>;
  _guard: ListenerGuard;

  constructor(broadcastId: BroadcastId, trigger: ArcRwSignal<void>, _guard: ListenerGuard) {
    super();
    this.broadcastId = broadcastId;
    this.trigger = trigger;
    this._guard = _guard;
  }

  static new(broadcastId: BroadcastId, signal: Signal): Arc<BridgeSource> {
    const trigger = ArcRwSignal.new([]);
    const triggerClone = trigger.clone();
    const guard = signal.listen(Arc.new((_) => {
      triggerClone.notify();
    }));
    return Arc.new(new BridgeSource(broadcastId, trigger, guard));
  }

  track(): void {
    if (Owner.current() != null) {
      this.trigger.track();
    }
  }
}

