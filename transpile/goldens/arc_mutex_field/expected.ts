// MIRRORS: ankurah/arc_mutex_field/src/input.rs
import { Struct, Arc, Mutex } from '@ankurah/base';

export class Counter extends Struct {
  _0: Arc<Inner>;

  constructor(_0: Arc<Inner>) {
    super();
    this._0 = _0;
  }

  static new(label: string): Counter {
    return new Counter(Arc.new(new Inner(new Mutex(label))));
  }

  labelLen(): number {
    const _t0 = this._0.value.label.lock();
    try {
      return _t0.value.length;
    } finally {
      _t0.drop();
    }
  }

  setLabel(label: string): void {
    this._0.value.label.lock().value = label;
  }
}

class Inner extends Struct {
  label: Mutex<string>;

  constructor(label: Mutex<string>) {
    super();
    this.label = label;
  }
}

