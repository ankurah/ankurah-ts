// TS-ONLY: Validate @ankurah/base types against real ankurah patterns
import { describe, test, expect } from 'bun:test';
import { Struct, Enum, Drop, Arc, Weak, Borrow, BorrowMut, Mutex, disposeSymbol } from '../src/index.ts';

// ══════════════════════════════════════════════════════════════════════
// Mock types standing in for real ankurah types
// ══════════════════════════════════════════════════════════════════════

class Broadcast extends Struct {
  sent = false;
  send(): void { this.sent = true; }
}

// ══════════════════════════════════════════════════════════════════════
// Struct 1: ReactorSubscription / ReactorSubInner
//
// Rust:
//   struct ReactorSubInner { subscription_id, reactor, broadcast }
//   impl Drop for ReactorSubInner { fn drop(&mut self) { reactor.unsubscribe(id) } }
//   struct ReactorSubscription(Arc<ReactorSubInner>)
//   impl Clone for ReactorSubscription { fn clone(&self) { Self(self.0.clone()) } }
// ══════════════════════════════════════════════════════════════════════

let unsubscribedIds: string[] = [];

class ReactorSubInner extends Drop {
  subscriptionId: string;
  broadcast: Broadcast;

  constructor(id: string) {
    super();
    this.subscriptionId = id;
    this.broadcast = new Broadcast();
  }

  drop(): void {
    // impl Drop — unsubscribe from reactor
    unsubscribedIds.push(this.subscriptionId);
  }
}

class ReactorSubscription extends Struct {
  inner: Arc<ReactorSubInner>;

  constructor(inner: Arc<ReactorSubInner>) {
    super();
    this.inner = inner;
  }

  static create(id: string): ReactorSubscription {
    return new ReactorSubscription(Arc.new(new ReactorSubInner(id)));
  }

  clone(): ReactorSubscription {
    return new ReactorSubscription(this.inner.clone());
  }

  id(): string { return this.inner.value.subscriptionId; }
}

// ══════════════════════════════════════════════════════════════════════
// Struct 2: Entity / EntityInner
//
// Rust:
//   struct EntityInner { id, collection, state: RwLock<EntityInnerState>, kind, broadcast }
//   struct Entity(Arc<EntityInner>)
//   // EntityInner does NOT have impl Drop
// ══════════════════════════════════════════════════════════════════════

class EntityInnerState extends Struct {
  head: number = 0;
  backends: Map<string, string> = new Map();
}

class EntityInner extends Struct {
  id: string;
  collection: string;
  state: Mutex<EntityInnerState>;
  broadcast: Broadcast;

  constructor(id: string, collection: string) {
    super();
    this.id = id;
    this.collection = collection;
    this.state = new Mutex(new EntityInnerState());
    this.broadcast = new Broadcast();
  }
}

class Entity extends Struct {
  inner: Arc<EntityInner>;

  constructor(inner: Arc<EntityInner>) {
    super();
    this.inner = inner;
  }

  static create(id: string, collection: string): Entity {
    return new Entity(Arc.new(new EntityInner(id, collection)));
  }

  clone(): Entity {
    return new Entity(this.inner.clone());
  }

  id(): string { return this.inner.value.id; }
}

// ══════════════════════════════════════════════════════════════════════
// Enum 1: EntityKind
//
// Rust:
//   enum EntityKind {
//     Primary,
//     Transacted { trx_alive: Arc<AtomicBool>, upstream: Entity },
//   }
// ══════════════════════════════════════════════════════════════════════

type EntityKindV = {
  Primary: {};
  Transacted: { trxAlive: { value: boolean }; upstream: Entity };
};

class EntityKind extends Enum<EntityKindV> {
  static Primary = () => new EntityKind('Primary', {});
  static Transacted = (trxAlive: { value: boolean }, upstream: Entity) =>
    new EntityKind('Transacted', { trxAlive, upstream });
}

// ══════════════════════════════════════════════════════════════════════
// Enum 2: DeltaContent
//
// Rust:
//   enum DeltaContent {
//     StateSnapshot { state: StateFragment },
//     EventBridge { events: Vec<EventFragment> },
//     StateAndRelation { state: StateFragment, relation: CausalAssertionFragment },
//   }
// ══════════════════════════════════════════════════════════════════════

class StateFragment extends Struct {
  data: Uint8Array;
  constructor(data: Uint8Array) { super(); this.data = data; }
}

class CausalAssertionFragment extends Struct {
  relation: string;
  constructor(relation: string) { super(); this.relation = relation; }
}

type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: string[] }; // simplified — real would be EventFragment[]
  StateAndRelation: { state: StateFragment; relation: CausalAssertionFragment };
};

class DeltaContent extends Enum<DeltaContentV> {
  static StateSnapshot = (state: StateFragment) =>
    new DeltaContent('StateSnapshot', { state });
  static EventBridge = (events: string[]) =>
    new DeltaContent('EventBridge', { events });
  static StateAndRelation = (state: StateFragment, relation: CausalAssertionFragment) =>
    new DeltaContent('StateAndRelation', { state, relation });
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

describe('ReactorSubscription (Struct + Arc + Drop)', () => {
  test('single owner drop triggers unsubscribe', () => {
    unsubscribedIds = [];
    const sub = ReactorSubscription.create('sub-1');
    sub[disposeSymbol]();
    expect(unsubscribedIds).toEqual(['sub-1']);
  });

  test('cloned subscription — inner drops only on last drop', () => {
    unsubscribedIds = [];
    const sub1 = ReactorSubscription.create('sub-2');
    const sub2 = sub1.clone();
    expect(sub1.inner.strongCount).toBe(2);

    sub1[disposeSymbol]();
    expect(unsubscribedIds).toEqual([]); // sub2 still holds

    sub2[disposeSymbol]();
    expect(unsubscribedIds).toEqual(['sub-2']); // last owner dropped
  });

  test('using block drops subscription', () => {
    unsubscribedIds = [];
    {
      using sub = ReactorSubscription.create('sub-3');
      expect(sub.id()).toBe('sub-3');
    }
    expect(unsubscribedIds).toEqual(['sub-3']);
  });

  test('inner broadcast is cascade-dropped', () => {
    const sub = ReactorSubscription.create('sub-4');
    const broadcast = sub.inner.value.broadcast;
    expect(broadcast.isDropped).toBe(false);
    sub[disposeSymbol]();
    expect(broadcast.isDropped).toBe(true);
  });
});

describe('Entity (Struct + Arc + Mutex, no impl Drop)', () => {
  test('entity clone shares inner', () => {
    const e1 = Entity.create('e-1', 'users');
    const e2 = e1.clone();
    expect(e1.inner.strongCount).toBe(2);
    expect(e2.id()).toBe('e-1');
    e1[disposeSymbol]();
    e2[disposeSymbol]();
  });

  test('mutex access through entity', () => {
    const e = Entity.create('e-2', 'posts');
    {
      using guard = e.inner.value.state.lock();
      guard.value.head = 42;
      guard.value.backends.set('lww', 'data');
    }
    {
      using guard = e.inner.value.state.lock();
      expect(guard.value.head).toBe(42);
      expect(guard.value.backends.get('lww')).toBe('data');
    }
    e[disposeSymbol]();
  });

  test('cascade drops inner state and broadcast', () => {
    const e = Entity.create('e-3', 'albums');
    const inner = e.inner.value;
    const broadcast = inner.broadcast;
    e[disposeSymbol]();
    expect(inner.isDropped).toBe(true);
    expect(broadcast.isDropped).toBe(true);
  });
});

describe('EntityKind (Enum with Arc and Entity in variant)', () => {
  test('Primary variant', () => {
    const kind = EntityKind.Primary();
    const result = kind.match({
      Primary: () => 'primary',
      Transacted: () => 'transacted',
    });
    expect(result).toBe('primary');
    kind[disposeSymbol]();
  });

  test('Transacted variant owns upstream entity', () => {
    const upstream = Entity.create('e-up', 'users');
    const alive = { value: true };
    const kind = EntityKind.Transacted(alive, upstream);

    kind.match({
      Primary: () => { throw new Error('wrong variant'); },
      Transacted: (v) => {
        expect(v.trxAlive.value).toBe(true);
        expect(v.upstream.id()).toBe('e-up');
      },
    });

    // Dropping the enum should cascade to the upstream entity
    const innerRef = upstream.inner.value;
    kind[disposeSymbol]();
    expect(innerRef.isDropped).toBe(true);
  });

  test('is() type narrowing', () => {
    const kind = EntityKind.Primary();
    expect(kind.is('Primary')).toBe(true);
    expect(kind.is('Transacted')).toBe(false);
    kind[disposeSymbol]();
  });
});

// ══════════════════════════════════════════════════════════════════════
// Proto Struct: Event
//
// Rust:
//   struct Event {
//     collection: CollectionId,
//     entity_id: EntityId,
//     operations: OperationSet,
//     parent: Clock,
//   }
//   impl Event { fn is_entity_create(&self) -> bool { self.parent.is_empty() } }
//
// Pure data struct. No Drop, no Arc. Tests that Struct works for plain data.
// ══════════════════════════════════════════════════════════════════════

class Clock extends Struct {
  events: string[];
  constructor(events: string[] = []) { super(); this.events = events; }
  isEmpty(): boolean { return this.events.length === 0; }
}

class OperationSet extends Struct {
  ops: Map<string, Uint8Array[]>;
  constructor() { super(); this.ops = new Map(); }
}

class ProtoEvent extends Struct {
  collection: string;
  entityId: string;
  operations: OperationSet;
  parent: Clock;

  constructor(collection: string, entityId: string, parent: Clock) {
    super();
    this.collection = collection;
    this.entityId = entityId;
    this.operations = new OperationSet();
    this.parent = parent;
  }

  isEntityCreate(): boolean { return this.parent.isEmpty(); }
}

// ══════════════════════════════════════════════════════════════════════
// Proto Enum: NodeMessage
//
// Rust:
//   enum NodeMessage {
//     Request { auth: Vec<AuthData>, request: NodeRequest },
//     Response(NodeResponse),
//     Update(NodeUpdate),
//     UpdateAck(NodeUpdateAck),
//     UnsubscribeQuery { from: EntityId, query_id: QueryId },
//   }
//   impl Display for NodeMessage { fn fmt(&self, f) { match self { ... } } }
//
// Data enum with methods. Tests Enum<V> with methods and mixed variant shapes.
// ══════════════════════════════════════════════════════════════════════

class NodeRequest extends Struct {
  id: string;
  to: string;
  from: string;
  constructor(id: string, to: string, from: string) {
    super();
    this.id = id;
    this.to = to;
    this.from = from;
  }
}

class NodeResponse extends Struct {
  requestId: string;
  constructor(requestId: string) { super(); this.requestId = requestId; }
}

type NodeMessageV = {
  Request: { auth: string[]; request: NodeRequest };
  Response: { response: NodeResponse };
  Update: { updateId: string };
  UpdateAck: { ackId: string };
  UnsubscribeQuery: { from: string; queryId: string };
};

class NodeMessage extends Enum<NodeMessageV> {
  static Request = (auth: string[], request: NodeRequest) =>
    new NodeMessage('Request', { auth, request });
  static Response = (response: NodeResponse) =>
    new NodeMessage('Response', { response });
  static Update = (updateId: string) =>
    new NodeMessage('Update', { updateId });
  static UpdateAck = (ackId: string) =>
    new NodeMessage('UpdateAck', { ackId });
  static UnsubscribeQuery = (from: string, queryId: string) =>
    new NodeMessage('UnsubscribeQuery', { from, queryId });

  // impl Display
  toString(): string {
    return this.match({
      Request: (v) => `Request: ${v.request.id}`,
      Response: (v) => `Response: ${v.response.requestId}`,
      Update: (v) => `Update: ${v.updateId}`,
      UpdateAck: (v) => `UpdateAck: ${v.ackId}`,
      UnsubscribeQuery: (v) => `Unsubscribe: ${v.from} ${v.queryId}`,
    });
  }
}

// ══════════════════════════════════════════════════════════════════════
// Proto tests
// ══════════════════════════════════════════════════════════════════════

describe('ProtoEvent (plain data Struct, no Drop)', () => {
  test('construction and methods', () => {
    const evt = new ProtoEvent('users', 'e-1', new Clock());
    expect(evt.isEntityCreate()).toBe(true);
    expect(evt.collection).toBe('users');

    const evt2 = new ProtoEvent('posts', 'e-2', new Clock(['ev-1']));
    expect(evt2.isEntityCreate()).toBe(false);
  });

  test('cascade drops owned Struct fields', () => {
    const parent = new Clock(['ev-1']);
    const ops = new OperationSet();
    const evt = new ProtoEvent('users', 'e-1', parent);

    evt[disposeSymbol]();
    expect(parent.isDropped).toBe(true);
    expect(evt.operations.isDropped).toBe(true);
  });

  test('using block for scoped lifetime', () => {
    let clockRef: Clock;
    {
      using evt = new ProtoEvent('users', 'e-1', new Clock(['ev-1']));
      clockRef = evt.parent;
      expect(clockRef.isDropped).toBe(false);
    }
    expect(clockRef!.isDropped).toBe(true);
  });
});

describe('NodeMessage (proto Enum with methods)', () => {
  test('Request variant with struct data', () => {
    const req = new NodeRequest('req-1', 'node-a', 'node-b');
    const msg = NodeMessage.Request(['auth-1'], req);

    expect(msg.type).toBe('Request');
    expect(msg.toString()).toBe('Request: req-1');
  });

  test('match over all variants', () => {
    const msg = NodeMessage.UnsubscribeQuery('node-a', 'q-42');
    const result = msg.match({
      Request: () => 'request',
      Response: () => 'response',
      Update: () => 'update',
      UpdateAck: () => 'ack',
      UnsubscribeQuery: (v) => `unsub:${v.queryId}`,
    });
    expect(result).toBe('unsub:q-42');
  });

  test('is() narrows variant', () => {
    const msg = NodeMessage.Response(new NodeResponse('req-99'));
    expect(msg.is('Response')).toBe(true);
    expect(msg.is('Request')).toBe(false);
  });

  test('cascade drops struct variant data', () => {
    const req = new NodeRequest('req-2', 'a', 'b');
    const msg = NodeMessage.Request([], req);

    msg[disposeSymbol]();
    expect(req.isDropped).toBe(true);
  });

  test('primitive variant data is harmless', () => {
    const msg = NodeMessage.Update('upd-1');
    msg[disposeSymbol](); // updateId is a string — no disposeSymbol, no crash
    expect(msg.isDropped).toBe(true);
  });

  test('Display impl via toString', () => {
    expect(NodeMessage.Update('u-1').toString()).toBe('Update: u-1');
    expect(NodeMessage.UpdateAck('a-1').toString()).toBe('UpdateAck: a-1');
  });
});

describe('DeltaContent (Enum with struct data variants)', () => {
  test('StateSnapshot match and cascade', () => {
    const state = new StateFragment(new Uint8Array([1, 2, 3]));
    const dc = DeltaContent.StateSnapshot(state);

    dc.match({
      StateSnapshot: (v) => expect(v.state.data).toEqual(new Uint8Array([1, 2, 3])),
      EventBridge: () => { throw new Error('wrong'); },
      StateAndRelation: () => { throw new Error('wrong'); },
    });

    dc[disposeSymbol]();
    expect(state.isDropped).toBe(true);
  });

  test('EventBridge with primitive array (no cascade needed)', () => {
    const dc = DeltaContent.EventBridge(['evt-1', 'evt-2']);
    const result = dc.match({
      StateSnapshot: () => null,
      EventBridge: (v) => v.events,
      StateAndRelation: () => null,
    });
    expect(result).toEqual(['evt-1', 'evt-2']);
    dc[disposeSymbol](); // should not throw — events are strings
  });

  test('StateAndRelation cascades both fields', () => {
    const state = new StateFragment(new Uint8Array([4, 5]));
    const relation = new CausalAssertionFragment('Equal');
    const dc = DeltaContent.StateAndRelation(state, relation);

    dc[disposeSymbol]();
    expect(state.isDropped).toBe(true);
    expect(relation.isDropped).toBe(true);
  });
});
