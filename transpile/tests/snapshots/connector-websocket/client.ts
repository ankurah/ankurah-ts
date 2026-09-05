// MIRRORS: ankurah/connectors/websocket-client-wasm/src/client.rs
import { Struct, Drop, Result, Arc, RefCell, AnyhowError, tracing, checkedAdd, sleep } from '@ankurah/base';
import { Node, NodeComms, Context } from '@ankurah/core';
import { Connection } from './connection';
import { ConnectionState } from './connection_state';
import { Mut, Read } from '@ankurah/signals';

export class WebsocketClient extends Struct {
  inner: Arc<ClientInner>;

  constructor(inner: Arc<ClientInner>) {
    super();
    this.inner = inner;
  }

  static new<SE, PA>(node: Node<SE, PA>, serverUrl: string): Result<WebsocketClient, Error> {
    undefined /* notice_info!("Created new websocket client") */;
    const inner = Arc.new(new ClientInner(serverUrl, new RefCell(null), Mut.new(new ConnectionState('None', {})), node, new RefCell(0n), new RefCell([])));
    const _r0 = inner.connect();
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    return Result.Ok(new WebsocketClient(inner));
  }

  connectionState(): Read<ConnectionState> {
    return this.inner.value.state.read();
  }

  async ready(): Promise<Result<void, string>> {
    return (await new ReadyFuture(this.inner.clone())).mapErr((_) => 'unreachable');
  }

  jsConnectionState(): ConnectionStateEnumSignal {
    const _t0 = this.inner.value.state.read();
    try {
      const sig = _t0.map((state) => state);
      return new ConnectionStateEnumSignal(sig, []);
    } finally {
      _t0.drop();
    }
  }

  clone(): WebsocketClient {
    return new WebsocketClient(this.inner.clone());
  }
}

class ClientInner extends Drop {
  serverUrl: string;
  connection: RefCell<Connection | null>;
  state: Mut<ConnectionState>;
  node: NodeComms;
  reconnectDelay: RefCell<bigint>;
  pendingReadyWakers: RefCell<Waker[]>;

  constructor(serverUrl: string, connection: RefCell<Connection | null>, state: Mut<ConnectionState>, node: NodeComms, reconnectDelay: RefCell<bigint>, pendingReadyWakers: RefCell<Waker[]>) {
    super();
    this.serverUrl = serverUrl;
    this.connection = connection;
    this.state = state;
    this.node = node;
    this.reconnectDelay = reconnectDelay;
    this.pendingReadyWakers = pendingReadyWakers;
  }

  handleStateChange(connection: Connection, newState: ConnectionState): boolean {
    try {
      const _m0 = (() => {
        {
          const c = this.connection.borrow();
          try {
            const _v = c.value;
            if (!(_v != null)) {
              return { $jump: 'return', $value: false };
            }
            const existingConnection = _v;
            if (!connection.equals(existingConnection)) {
              return { $jump: 'return', $value: false };
            }
          } finally {
            c.drop();
          }
        }
      })();
      if ((_m0 as any)?.$jump === 'return') return (_m0 as any).$value;
      (_m0 as any);
      this.state.set(newState.clone());
      undefined /* action_info!(self , "state changed" , "{}" , & new_state) */;
      newState.match({
        Connected: () => {
          const _t1 = this.reconnectDelay.borrowMut();
          try {
            _t1.value = 0n;
          } finally {
            _t1.drop();
          }
          const _t2 = this.pendingReadyWakers.borrowMut();
          try {
            const wakers = mem.take(_t2.value);
            _t2.drop();
            for (const waker of wakers) {
              waker.wake();
            }
          } finally {
            _t2.drop();
          }
        },
        Connecting: () => {
          [];
        },
        None: () => {},
        Closed: () => {
          (() => {
            const _t3 = this.connection.borrowMut();
            try {
              _t3.value = null;
            } finally {
              _t3.drop();
            }
          })();
          const _t4 = this.reconnectDelay.borrow();
          try {
            const nextDelay = (($a, $b) => $a < $b ? $a : $b)((checkedAdd(_t4.value, 500n, 'u64')), MAX_RECONNECT_DELAY);
            _t4.drop();
            const _t5 = this.reconnectDelay.borrowMut();
            try {
              _t5.value = nextDelay;
            } finally {
              _t5.drop();
            }
            this.reconnect(nextDelay);
          } finally {
            _t4.drop();
          }
        },
        Error: () => {
          (() => {
            const _t6 = this.connection.borrowMut();
            try {
              _t6.value = null;
            } finally {
              _t6.drop();
            }
          })();
          const _t7 = this.reconnectDelay.borrow();
          try {
            const nextDelay = (($a, $b) => $a < $b ? $a : $b)((checkedAdd(_t7.value, 500n, 'u64')), MAX_RECONNECT_DELAY);
            _t7.drop();
            const _t8 = this.reconnectDelay.borrowMut();
            try {
              _t8.value = nextDelay;
            } finally {
              _t8.drop();
            }
            this.reconnect(nextDelay);
          } finally {
            _t7.drop();
          }
        },
      });
      return true;
    } finally {
      newState.drop();
    }
  }

  connect(): Result<void, Error> {
    const _r0 = Connection.new(Node_cloned(this.node), this.serverUrl, this.downgrade()).mapErr((e) => AnyhowError.msg(`${e}`));
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const connection = _r0.unwrap();
    try {
      undefined /* action_info!(self , "connecting to" , "{}" , & self . server_url) */;
      _moved1 = true;
      const _t2 = this.connection.borrowMut();
      try {
        _t2.value = connection;
      } finally {
        _t2.drop();
      }
      this.state.set(new ConnectionState('Connecting', { url: this.serverUrl }));
      return Result.Ok([]);
    } finally {
      if (!_moved1) connection.drop();
    }
  }

  reconnect(delay: bigint): void {
    tracing.info(`reconnect: removing old connection with delay ${delay}ms`);
    const self2 = this.clone();
    spawnLocal((async () => {
      tracing.info(`reconnect: sleeping for ${delay}ms`);
      await sleep(Duration.fromMillis(delay));
      tracing.info('reconnect: reconnecting');
      self2.connect();
    })());
  }

  toString(): string {
    return 'Client';
  }

  protected override onDrop(): void {
    tracing.info(`Websocket client inner dropped for node ${Node_id(this.node)}`);
  }
}

export class ReadyFuture extends Struct {
  client: Arc<ClientInner>;

  constructor(client: Arc<ClientInner>) {
    super();
    this.client = client;
  }

  poll(cx: Context): Poll<Result<void, void>> {
    {
      const _v = this.client.value.state.value();
      if (_v.is('Connected')) {
        return new Poll('Ready', { _0: Result.Ok([]) });
      } else {
      const _t0 = this.client.value.pendingReadyWakers.borrowMut();
      try {
        _t0.value.push(cx.waker().clone());
      } finally {
        _t0.drop();
      }
      return Poll.Pending;
    }
    }
  }
}

const MAX_RECONNECT_DELAY: bigint = 10000n;

