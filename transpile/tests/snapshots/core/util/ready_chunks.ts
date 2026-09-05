// MIRRORS: ankurah/core/src/util/ready_chunks.rs
import { Struct } from '@ankurah/base';
import { Context } from '../context';
import { Item } from '@ankurah/proto';

export class ReadyChunks<F extends Future> extends Struct {
  inner: FuturesUnordered<Pin<F>>;

  constructor(inner: FuturesUnordered<Pin<F>>) {
    super();
    this.inner = inner;
  }

  static new<F, I>(futures: I): ReadyChunks<F> {
    const inner = futures.intoIter().map((f) => Box.pin(f));
    return new ReadyChunks(inner);
  }

  isEmpty(): boolean {
    return this.inner.isEmpty();
  }

  len(): number {
    return this.inner.len();
  }

  pollNext(cx: Context): Poll<Item | null> {
    let batch = [];
    const _m0 = this.inner.pollNextUnpin(cx).match<any>({
      Ready: (v) => {
        const item = v._0;
        return batch.push(item);
      },
      Pending: () => {
        return { $jump: 'return', $value: Poll.Pending }
      },
    });
    if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
    while (true) {
      const _v = this.inner.pollNextUnpin(cx);
      if (_v.is('Ready') && (_v.value._0 != null)) {
        const item = _v.value._0;
        return batch.push(item);
      } else {
        break;
      }
    }
    return new Poll('Ready', { _0: batch });
  }
}

