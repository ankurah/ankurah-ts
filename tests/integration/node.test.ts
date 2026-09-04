// The milestone this harness exists for, written down as tests that fail today.
//
// The milestone is a pure-TypeScript ephemeral ankurah node running on Bun, with memory
// storage and the websocket connector, exchanging entities and subscription updates with a
// Rust durable node. Each test below names one half of that exchange and fails with the one
// thing still missing, so the milestone is something you can run rather than something you
// have to remember.
//
// None of these are skipped on purpose. A skipped test is invisible in a summary line; a
// failing test with a reason is a to-do list that reports itself. When @ankurah/core lands,
// replace each body with the flow described above it — the Rust side of every one of them
// is already written in ankurah-ts-support/tests/tests/websocket.rs.

import { describe, test } from 'bun:test';

/** Fail with the single sentence that says what is missing. */
function blocked(reason: string): never {
  throw new Error(reason);
}

const CORE_NOT_PORTED =
  'Blocked: @ankurah/core is not ported, so there is no TypeScript node to create entities, ' +
  'run queries, or hold subscriptions.';

describe('milestone: a TypeScript node exchanging data with a Rust durable node', () => {
  // Mirrors test_websocket_client_create_propagation in
  // ankurah-ts-support/tests/tests/websocket.rs, with the client side in TypeScript:
  // start the durable node, connect an ephemeral TypeScript node over the websocket
  // connector backed by @ankurah/storage-memory, wait for the system to be ready, create an
  // entity in a transaction and commit it, then fetch it back through a second connection
  // to the same Rust node and find it there.
  test('an entity created in TypeScript is readable from a second Rust connection', () => {
    blocked(CORE_NOT_PORTED);
  });

  // Mirrors test_websocket_client_server_fetch, in the other direction: the Rust durable
  // node creates the entity and the TypeScript node fetches it over the websocket.
  test('an entity created in Rust is readable from the TypeScript node', () => {
    blocked(CORE_NOT_PORTED);
  });

  // Mirrors the client half of test_websocket_subscription_propagation: the TypeScript node
  // subscribes to a query, the Rust node commits a matching entity, and the subscription
  // delivers one Add change for that entity.
  test('a TypeScript subscription receives an update made on the Rust node', () => {
    blocked(CORE_NOT_PORTED);
  });

  // Mirrors the server half of test_websocket_bidirectional_subscription: a query is
  // subscribed on the Rust node, the TypeScript node commits a matching entity, and the
  // Rust subscription sees it.
  test('a Rust subscription receives an update made on the TypeScript node', () => {
    blocked(CORE_NOT_PORTED);
  });
});
