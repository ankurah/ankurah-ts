// MIRRORS: ankurah/consuming_if_let/src/input.rs
import { Struct, Enum, dropOwned, dropUnbound, checkedAdd } from '@ankurah/base';

export class Op extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Leaf extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }
}

export class Inner extends Struct {
  readonly n: number;

  constructor(n: number) {
    super();
    this.n = n;
  }

  clone(): Inner {
    return new Inner(this.n);
  }
}

export class Holder extends Struct {
  readonly inner: Inner;
  readonly tag: number;

  constructor(inner: Inner, tag: number) {
    super();
    this.inner = inner;
    this.tag = tag;
  }

  clone(): Holder {
    return new Holder(this.inner.clone(), this.tag);
  }
}

export type NodeV = {
  Pair: { left: Node; op: Op; right: Node };
  End: { _0: Leaf };
};

export class Node extends Enum<NodeV> {

  static leaf(n: number): Node {
    return new Node('End', { _0: new Leaf(n) });
  }
}

export function rightLeaf(node: Node): number {
  const _m1 = node.intoMatch<any>({
    Pair: (v) => {
      const val = v.right;
      try {
        let _moved0 = false;
        try {
          _moved0 = true;
          return { $jump: 'return', $value: depth(val) };
        } finally {
          if (!_moved0) dropOwned(val);
        }
      } finally {
        dropUnbound(v, ['right']);
      }
    },
    End: (v) => {
      try {
      } finally {
        dropUnbound(v, []);
      }
    },
  });
  if ((_m1 as any)?.$jump === 'return') return (_m1 as any).$value;
  return 0;
}

export function depth(node: Node): number {
  return node.intoMatch({
    Pair: (v) => {
      const left = v.left;
      const op = v.op;
      const right = v.right;
      try {
        return checkedAdd(checkedAdd(op.n, depth(left), 'u32'), depth(right), 'u32');
      } finally {
        op.drop();
      }
    },
    End: (v) => {
      const leaf = v._0;
      try {
        return leaf.n;
      } finally {
        leaf.drop();
      }
    },
  });
}

export function onlyPairs(node: Node): number {
  const _m0 = node.intoMatch<any>({
    Pair: (v) => {
      const op = v.op;
      try {
        try {
          return { $jump: 'return', $value: op.n };
        } finally {
          op.drop();
        }
      } finally {
        dropUnbound(v, ['op']);
      }
    },
    End: (v) => {
      try {
      } finally {
        dropUnbound(v, []);
      }
    },
  });
  if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
  return 7;
}

export function eat(i: Inner): number {
  try {
    return i.n;
  } finally {
    i.drop();
  }
}

export function fromAClone(h: Holder): number {
  try {
    const _t0 = h.clone();
    try {
      const taken = _t0.takeField('inner');
      return checkedAdd(eat(taken), h.tag, 'u32');
    } finally {
      _t0.drop();
    }
  } finally {
    h.drop();
  }
}

