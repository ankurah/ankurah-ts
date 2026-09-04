// Proves that TypeScript and Rust agree on the bytes of the websocket handshake.
//
// A Rust node joins another node by opening a websocket, exchanging a Presence message in
// each direction, and then sending peer messages. This test performs that exchange from
// TypeScript against a real Rust durable node, and every byte in either direction is
// produced or read by the transpiled @ankurah/proto package — nothing here is hand-rolled.
//
// The sequence mirrors connectors/websocket-client/src/client.rs and
// connectors/websocket-server/src/server.rs in the support checkout:
//
//   1. The client opens ws://host:port/ws. `WebsocketClient::normalize_url` appends `/ws`;
//      the server routes only that path.
//   2. The server sends its own Presence the moment the socket upgrades, without waiting
//      to be asked. A durable node's Presence carries its system root.
//   3. The client sends its Presence. Until the server has read it, the connection is in
//      `Connection::Initial` and every peer message is refused.
//   4. Peer messages flow both ways. Getting a Response back is the only wire-visible
//      proof that the server read our Presence and registered us as a peer.
//
// The proto package is imported by path rather than as @ankurah/proto because Bun's
// isolated install only exposes workspace packages to each other, not to files at the
// repository root.

import { describe, expect, test } from 'bun:test';

import {
  AuthData,
  BincodeReader,
  CollectionId,
  EntityId,
  Message,
  NodeMessage,
  NodeRequest,
  NodeRequestBody,
  Presence,
  RequestId,
  serialize,
} from '../../packages/proto/src/index.ts';

import { startDurableNode } from './support/durable-node.ts';

/** How long to wait for any single frame from the server. */
const FRAME_TIMEOUT_MS = 10_000;

/** How long to wait before concluding that a frame is never coming. */
const UNANSWERED_WAIT_MS = 1_500;

describe('websocket handshake with a Rust durable node', () => {
  test('exchanges Presence and completes a peer request round-trip', async () => {
    const node = await startDurableNode();
    // Every frame in either direction, so a failure can say exactly what crossed the wire.
    const wire: string[] = [];

    try {
      const socket = new WebSocket(node.wsUrl);
      socket.binaryType = 'arraybuffer';
      const frames = new FrameReader(socket);
      await frames.opened(FRAME_TIMEOUT_MS);

      // ── 1. The server greets us with its Presence ──────────────────────────

      const serverPresenceBytes = await frames.next('the server presence', FRAME_TIMEOUT_MS);
      wire.push(`<- ${describeFrame(serverPresenceBytes)}`);

      using serverMessage = decodeMessage(serverPresenceBytes, 'the server presence');
      expect(serverMessage.type).toBe('Presence');
      if (!serverMessage.is('Presence')) throw new Error('unreachable');
      const serverPresence = serverMessage.value._0;

      // A durable node reports itself durable and carries the system root it created at
      // startup. An ephemeral peer needs that root to join the system, so its absence
      // would mean the harness is talking to the wrong kind of node.
      expect(serverPresence.durable).toBe(true);
      expect(serverPresence.systemRoot).not.toBeNull();
      expect(serverPresence.nodeId.toBytes().length).toBe(16);

      // Re-encoding what we decoded has to reproduce the server's bytes exactly. This is
      // the check that would catch a field we skipped or a length we read at the wrong width.
      const reEncoded = serialize((writer) => serverMessage.encode(writer));
      expect(toHex(reEncoded)).toBe(toHex(serverPresenceBytes));

      // ── 2. We answer with our own Presence ────────────────────────────────

      // An ephemeral TypeScript node: not durable, and no system root of its own, exactly
      // as `WebsocketClient::connect_once` sends for a node that has not joined yet.
      using clientNodeId = EntityId.new();
      using clientPresence = new Message('Presence', { _0: new Presence(clientNodeId.clone(), false, null) });
      const clientPresenceBytes = serialize((writer) => clientPresence.encode(writer));
      wire.push(`-> ${describeFrame(clientPresenceBytes)}`);
      socket.send(clientPresenceBytes);

      // ── 3. A peer request proves the server accepted us as a peer ─────────

      // The server only answers peer messages once our Presence has moved its connection
      // to `Connection::Established`, so a Response here is proof that the Rust side
      // decoded the bytes we just sent.
      //
      // `Get` with no ids is the cheapest request that still exercises a full round trip:
      // the node opens the collection, finds nothing to return, and answers `Get([])`.
      // `logentry` is one of the collections the durable node's model crate declares.
      using requestId = RequestId.new();
      using request = new Message('PeerMessage', {
        _0: new NodeMessage('Request', {
          // `PermissiveAgent::sign_request` produces one empty AuthData per context, and
          // `check_request` turns each one back into a context — an empty list would leave
          // the server with no context to read with.
          auth: [new AuthData(new Uint8Array(0))],
          request: new NodeRequest(
            requestId.clone(),
            serverPresence.nodeId.clone(),
            clientNodeId.clone(),
            new NodeRequestBody('Get', { collection: CollectionId.from('logentry'), ids: [] }),
          ),
        }),
      });
      const requestBytes = serialize((writer) => request.encode(writer));
      wire.push(`-> ${describeFrame(requestBytes)}`);
      socket.send(requestBytes);

      const responseBytes = await frames.next('the peer response', FRAME_TIMEOUT_MS);
      wire.push(`<- ${describeFrame(responseBytes)}`);

      using responseMessage = decodeMessage(responseBytes, 'the peer response');
      expect(responseMessage.type).toBe('PeerMessage');
      if (!responseMessage.is('PeerMessage')) throw new Error('unreachable');
      const peerMessage = responseMessage.value._0;
      expect(peerMessage.type).toBe('Response');
      if (!peerMessage.is('Response')) throw new Error('unreachable');

      const response = peerMessage.value._0;
      expect(response.requestId.toUlidString()).toBe(requestId.toUlidString());
      expect(response.from.toBase64()).toBe(serverPresence.nodeId.toBase64());
      expect(response.to.toBase64()).toBe(clientNodeId.toBase64());
      expect(response.body.type).toBe('Get');
      if (!response.body.is('Get')) throw new Error('unreachable');
      expect(response.body.value._0.length).toBe(0);

      socket.close();
    } catch (error) {
      throw annotate(error, wire, node.stderr());
    } finally {
      await node.stop();
    }
  }, 120_000);

  // The test above treats the response as proof that the server read our Presence. That is
  // only proof if a request without a Presence goes unanswered, so check that too — without
  // it, the response could have meant nothing.
  test('refuses a peer request from a connection that has not sent its Presence', async () => {
    const node = await startDurableNode();
    const wire: string[] = [];

    try {
      const socket = new WebSocket(node.wsUrl);
      socket.binaryType = 'arraybuffer';
      const frames = new FrameReader(socket);
      await frames.opened(FRAME_TIMEOUT_MS);

      const serverPresenceBytes = await frames.next('the server presence', FRAME_TIMEOUT_MS);
      wire.push(`<- ${describeFrame(serverPresenceBytes)}`);
      using serverMessage = decodeMessage(serverPresenceBytes, 'the server presence');
      if (!serverMessage.is('Presence')) throw new Error('the server did not open with a Presence');

      // Skip our Presence entirely and go straight to a peer request — the same request the
      // test above sends and gets an answer to.
      using requestId = RequestId.new();
      using clientNodeId = EntityId.new();
      using request = new Message('PeerMessage', {
        _0: new NodeMessage('Request', {
          auth: [new AuthData(new Uint8Array(0))],
          request: new NodeRequest(
            requestId.clone(),
            serverMessage.value._0.nodeId.clone(),
            clientNodeId.clone(),
            new NodeRequestBody('Get', { collection: CollectionId.from('logentry'), ids: [] }),
          ),
        }),
      });
      const requestBytes = serialize((writer) => request.encode(writer));
      wire.push(`-> ${describeFrame(requestBytes)}`);
      socket.send(requestBytes);

      await expect(frames.next('a response that should never come', UNANSWERED_WAIT_MS)).rejects.toThrow(
        `timed out after ${UNANSWERED_WAIT_MS}ms`,
      );

      socket.close();
    } catch (error) {
      throw annotate(error, wire, node.stderr());
    } finally {
      await node.stop();
    }
  }, 120_000);
});

// ── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Decode one websocket frame as a proto Message, and insist the whole frame was consumed.
 * Leftover bytes mean the TypeScript decoder read a shorter message than Rust wrote, which
 * would otherwise pass silently.
 */
function decodeMessage(bytes: Uint8Array, what: string): Message {
  const reader = new BincodeReader(bytes);
  let message: Message;
  try {
    message = Message.decode(reader);
  } catch (error) {
    throw new Error(`@ankurah/proto could not decode ${what}: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (reader.remaining !== 0) {
    throw new Error(`@ankurah/proto decoded ${what} but left ${reader.remaining} of ${bytes.length} bytes unread`);
  }
  return message;
}

/** Add the wire log and the node's own stderr to a failure, so it can be read without a rerun. */
function annotate(error: unknown, wire: string[], stderr: string): Error {
  const original = error instanceof Error ? error : new Error(String(error));
  original.message =
    `${original.message}\n\n` +
    `frames on the wire (-> sent, <- received):\n${wire.length > 0 ? wire.join('\n') : '  (none)'}\n\n` +
    `durable node stderr:\n${stderr}`;
  return original;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function describeFrame(bytes: Uint8Array): string {
  return `${bytes.length} bytes: ${toHex(bytes)}`;
}

/**
 * Hands out the binary frames a socket receives, one awaited call at a time, and turns a
 * close or an error into a failure on whatever the test was waiting for.
 */
class FrameReader {
  private readonly pending: Uint8Array[] = [];
  private readonly waiters: Array<() => void> = [];
  private open = false;
  private ended: string | null = null;

  constructor(socket: WebSocket) {
    socket.addEventListener('open', () => {
      this.open = true;
      this.notify();
    });
    socket.addEventListener('message', (event: MessageEvent) => {
      const data: unknown = event.data;
      if (typeof data === 'string') {
        this.ended ??= `the server sent a text frame where the protocol only uses binary: ${data}`;
      } else if (data instanceof ArrayBuffer) {
        this.pending.push(new Uint8Array(data));
      } else if (ArrayBuffer.isView(data)) {
        this.pending.push(new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
      } else {
        this.ended ??= `the server sent a frame of an unexpected type: ${Object.prototype.toString.call(data)}`;
      }
      this.notify();
    });
    socket.addEventListener('close', (event: CloseEvent) => {
      this.ended ??= `the server closed the connection (code ${event.code}${event.reason ? `, reason "${event.reason}"` : ''})`;
      this.notify();
    });
    socket.addEventListener('error', () => {
      this.ended ??= 'the websocket connection failed (the upgrade to /ws may have been rejected)';
      this.notify();
    });
  }

  /** Wait for the upgrade to complete. */
  async opened(timeoutMs: number): Promise<void> {
    await this.until(() => this.open, 'the websocket to open', timeoutMs);
  }

  /** Wait for the next binary frame. */
  async next(what: string, timeoutMs: number): Promise<Uint8Array> {
    await this.until(() => this.pending.length > 0, what, timeoutMs);
    return this.pending.shift()!;
  }

  private async until(satisfied: () => boolean, what: string, timeoutMs: number): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    while (!satisfied()) {
      if (this.ended !== null) throw new Error(`while waiting for ${what}: ${this.ended}`);
      const remaining = deadline - Date.now();
      if (remaining <= 0) throw new Error(`timed out after ${timeoutMs}ms waiting for ${what}`);
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, remaining);
        this.waiters.push(() => {
          clearTimeout(timer);
          resolve();
        });
      });
    }
  }

  private notify(): void {
    const waiting = this.waiters.splice(0);
    for (const wake of waiting) wake();
  }
}
