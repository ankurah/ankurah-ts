// MIRRORS: ankurah/core/src/util/ready_chunks.rs
import { Struct, unsupported } from '@ankurah/base';
import { Context } from '../context';
import { Item } from '@ankurah/proto';

export class ReadyChunks<F extends Future> extends Struct {
  inner: FuturesUnordered<Pin<F>>;

  constructor(inner: FuturesUnordered<Pin<F>>) {
    super();
    this.inner = inner;
  }

  static new<F, I>(futures: I): ReadyChunks<F> {
    const inner = unsupported('`collect` into `FuturesUnordered<unknown>` is a `FromIterator` the port has no construction for');
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
        if (v._0 != null) {
          const item = v._0;
          return batch.push(item);
        } else {
          return { $jump: 'return', $value: new Poll('Ready', { _0: null }) }
        }
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
        batch.push(item)
      } else {
        break
      }
    }
    return new Poll('Ready', { _0: batch });
  }
}

