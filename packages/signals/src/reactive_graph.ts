// MIRRORS: ankurah/signals/src/reactive_graph.rs
import { Struct, Arc, Mutex } from '@ankurah/base';
import { BroadcastId, ListenerGuard } from './broadcast';
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
    const map = this.bridges.lock();
    const bridge = map.entry(id).orInsertWith(() => new BridgeSource(id, signal)).clone();
    bridge.track();
    bridge.drop();

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
  Guard: ListenerGuard;

  constructor(broadcastId: BroadcastId, trigger: ArcRwSignal<void>, Guard: ListenerGuard) {
    super();
    this.broadcastId = broadcastId;
    this.trigger = trigger;
    this.Guard = Guard;
  }

  static new(broadcastId: BroadcastId, signal: Signal): Arc<BridgeSource> {
    const trigger = new ArcRwSignal([]);
    const triggerClone = trigger.clone();
    const guard = signal.listen(Arc.new((_) => {
      triggerClone.notify();
    }));
    const _ret = Arc.new(new BridgeSource(broadcastId, trigger, guard));
    triggerClone.drop();
    return _ret;
  }

  track(): void {
    if (Owner.current() != null) {
      this.trigger.track();
    }
  }
}

