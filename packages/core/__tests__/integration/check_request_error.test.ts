// MIRRORS: ankurah/tests/tests/check_request_error.rs
//
// Tests that check_request errors are properly propagated back to the client.
// This test ensures that when the server's PolicyAgent rejects a request via check_request,
// the client receives an error response rather than hanging indefinitely.

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { LocalProcessConnection } from '@ankurah/connector-local';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { ValidationError } from '../../src/error.ts';
import type { PolicyAgent } from '../../src/policy.ts';
import type {
  AuthData,
  Attestation,
  Attested,
  EntityId,
  EntityState,
  Event,
  NodeRequest,
  CausalAssertion,
  CollectionId,
  State,
} from '@ankurah/proto';
import type { Predicate } from '@ankurah/ankql';
import type { Entity } from '../../src/entity.ts';

// ── Models ──
// Mirrors: common.rs `struct Album { name: String, year: String }`
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── RejectingAgent ──
// Mirrors: check_request_error.rs RejectingAgent — rejects all incoming requests at check_request stage.

class RejectingAgent implements PolicyAgent {
  signRequest(_cdata: unknown[], _request: NodeRequest): AuthData[] {
    return [];
  }

  async checkRequest(_auth: AuthData[], _request: NodeRequest): Promise<unknown[]> {
    throw ValidationError.validationFailed('Request rejected by RejectingAgent');
  }

  checkEvent(): Attestation | null {
    return null;
  }

  validateReceivedEvent(_fromPeerId: EntityId, _event: Attested<Event>): void {}

  attestState(_state: EntityState): Attestation | null {
    return null;
  }

  validateReceivedState(_fromPeerId: EntityId, _state: Attested<EntityState>): void {}

  canAccessCollection(): void {}

  filterPredicate(_cdata: unknown[], _collection: CollectionId, predicate: Predicate): Predicate {
    return predicate;
  }

  checkRead(): void {}

  checkReadEvent(): void {}

  checkWrite(): void {}

  validateCausalAssertion(_peerId: EntityId, _headRelation: CausalAssertion): void {}
}

// ── Tests ──

describe('check_request_error', () => {
  // Mirrors: check_request_error.rs check_request_error_returns_to_client
  test('check_request_error_returns_to_client', async () => {
    // Server uses RejectingAgent - will reject all incoming requests
    const server = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new RejectingAgent(),
      durable: true,
    });
    await server.system.create();

    // Client uses PermissiveAgent - allows local operations
    const client = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: false,
    });

    // Connect client to server
    const conn = await LocalProcessConnection.new(server, client);
    await client.system.waitSystemReady();

    const clientCtx = await client.contextAsync();

    // Try to create an entity on the client - this should fail when relaying to server
    // because the server's check_request will reject it
    const trx = clientCtx.begin();
    await trx.create(Album, { name: 'Test Album', year: '2024' });

    // The commit should return an error (not hang!) because the server rejected the request
    let caughtError: Error | null = null;
    try {
      await trx.commit();
    } catch (err) {
      caughtError = err as Error;
    }

    expect(caughtError).not.toBeNull();
    // Verify the error message contains our rejection reason
    const errMsg = caughtError!.message;
    expect(errMsg.includes('rejected') || errMsg.includes('Request rejected')).toBe(true);

    conn.destroy();
  });
});
