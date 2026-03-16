// MIRRORS: ankurah/connectors/websocket-client/src/client.rs

import { Message, Presence, serialize, deserialize } from '@ankurah/proto';
import type { EntityId, NodeMessage } from '@ankurah/proto';
import type { NodeComms } from '@ankurah/core';
import { Mut, Read, waitFor } from '@ankurah/signals';

import { WebsocketPeerSender } from './sender.ts';

// ── ConnectionError ─────────────────────────────────────────────────────────
// Rust: pub enum ConnectionError { General(String) }

export class ConnectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ConnectionError';
  }

  static general(message: string): ConnectionError {
    return new ConnectionError(`General connection error: ${message}`);
  }
}

// ── ConnectionState ─────────────────────────────────────────────────────────
// Rust: pub enum ConnectionState { Disconnected, Connecting { url }, Connected { url, server_presence }, Error(ConnectionError) }
// Divergence: Rust uses enum with data variants; TS uses discriminated union [E8]

export type ConnectionState =
  | { type: 'Disconnected' }
  | { type: 'Connecting'; url: string }
  | { type: 'Connected'; url: string; serverPresence: Presence }
  | { type: 'Error'; error: ConnectionError };

// impl Display
function connectionStateToString(state: ConnectionState): string {
  switch (state.type) {
    case 'Disconnected': return 'Disconnected';
    case 'Connecting': return 'Connecting';
    case 'Connected': return 'Connected';
    case 'Error': return 'Error';
  }
}

// ── Constants ───────────────────────────────────────────────────────────────

const INITIAL_BACKOFF_MS = 1000;  // 1 second
const MAX_BACKOFF_MS = 30000;     // 30 seconds

// ── WebsocketClientBuilder ──────────────────────────────────────────────────
// Rust: pub struct WebsocketClientBuilder<SE, PA>
// Divergence: Rust is generic over StorageEngine/PolicyAgent; TS uses NodeComms interface [A6].
// Divergence: config, disable_nagle, connector fields omitted — standard WebSocket API
// has no equivalent (no TLS connector, no Nagle control, no WebSocketConfig) [E17].

export class WebsocketClientBuilder {
  private readonly node: NodeComms;
  private readonly serverUrl: string;

  constructor(node: NodeComms, serverUrl: string) {
    this.node = node;
    this.serverUrl = serverUrl;
  }

  // Rust: pub fn config, disable_nagle, connector, insecure
  // Divergence: All omitted — standard WebSocket API handles TLS/config internally [E17]

  /// Build and start the WebSocket client
  async build(): Promise<WebsocketClient> {
    return WebsocketClient.create(this.node, this.serverUrl);
  }
}

// ── WebsocketClient ─────────────────────────────────────────────────────────
// Rust: pub struct WebsocketClient<SE, PA>
// Divergence: Rust uses Arc<Inner<SE, PA>> with tokio tasks; TS uses standard WebSocket API
// with event-based handling [E17].
// Divergence: Rust is generic over StorageEngine/PolicyAgent; TS uses NodeComms [A6].

export class WebsocketClient {
  private readonly node: NodeComms;
  private readonly serverUrl: string;
  private readonly connectionState: Mut<ConnectionState>;
  private connected: boolean;
  private shutdownRequested: boolean;
  private ws: WebSocket | null;
  private peerSender: WebsocketPeerSender | null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null;
  private backoffMs: number;

  private constructor(node: NodeComms, serverUrl: string) {
    this.node = node;
    this.serverUrl = serverUrl;
    this.connectionState = new Mut<ConnectionState>({ type: 'Disconnected' });
    this.connected = false;
    this.shutdownRequested = false;
    this.ws = null;
    this.peerSender = null;
    this.reconnectTimer = null;
    this.backoffMs = INITIAL_BACKOFF_MS;
  }

  /// Create a new WebSocket client and start connecting to the server
  // Rust: pub async fn new(node, server_url) -> Result<Self>
  static async new(node: NodeComms, serverUrl: string): Promise<WebsocketClient> {
    return WebsocketClient.create(node, serverUrl);
  }

  /// Create a builder for configuring the WebSocket client
  // Rust: pub fn builder(node, server_url) -> WebsocketClientBuilder
  static builder(node: NodeComms, serverUrl: string): WebsocketClientBuilder {
    return new WebsocketClientBuilder(node, serverUrl);
  }

  // Rust: async fn create(node, server_url, config, disable_nagle, connector) -> Result<Self>
  // Divergence: config/disable_nagle/connector params omitted [E17]
  static async create(node: NodeComms, serverUrl: string): Promise<WebsocketClient> {
    const wsUrl = WebsocketClient.normalizeUrl(serverUrl);
    console.info(`Creating WebSocket client for ${wsUrl}`);

    const client = new WebsocketClient(node, wsUrl);
    // Start connection loop
    client.runConnectionLoop();
    return client;
  }

  // Rust: fn normalize_url(url: &str) -> String
  static normalizeUrl(url: string): string {
    if (url.startsWith('ws://') || url.startsWith('wss://')) {
      return `${url}/ws`;
    } else if (url.startsWith('http://')) {
      return `ws://${url.slice(7)}/ws`;
    } else if (url.startsWith('https://')) {
      return `wss://${url.slice(8)}/ws`;
    } else {
      return `wss://${url}/ws`;
    }
  }

  /// Get the connection state as a reactive signal
  // Rust: pub fn state(&self) -> Read<ConnectionState>
  state(): Read<ConnectionState> {
    return this.connectionState.read();
  }

  /// Check if currently connected to the server
  // Rust: pub fn is_connected(&self) -> bool
  isConnected(): boolean {
    return this.connected;
  }

  /// Gracefully shutdown the WebSocket connection
  // Rust: pub async fn shutdown(self) -> Result<()>
  // Divergence: Rust consumes self; TS mutates state (no ownership transfer) [E8]
  async shutdown(): Promise<void> {
    console.info('Shutting down WebSocket client');

    this.shutdownRequested = true;

    // Cancel any pending reconnect timer
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Close the WebSocket connection
    if (this.ws !== null) {
      this.ws.close();
      this.ws = null;
    }

    // Cleanup peer
    this.cleanupPeer();

    this.connectionState.set({ type: 'Disconnected' });
    this.connected = false;

    console.info('WebSocket client shutdown completed');
  }

  /// Wait for the client to establish a connection to the server (signal-based)
  // Rust: pub async fn wait_connected(&self) -> Result<(), ConnectionError>
  async waitConnected(): Promise<void> {
    // Wait for either Connected or Error state, returning appropriate Result
    return waitFor(this.state(), (state: ConnectionState) => {
      switch (state.type) {
        case 'Connected': return true;
        case 'Error': return true; // will check and throw after
        default: return false; // Continue waiting for Connecting/Disconnected states
      }
    }).then(() => {
      // Check the final state — if it was Error, throw
      const currentState = this.connectionState.peek();
      if (currentState.type === 'Error') {
        throw currentState.error;
      }
    });
  }

  /// Get the node ID of the connected server (if connected)
  // Rust: pub fn server_node_id(&self) -> Option<EntityId>
  serverNodeId(): EntityId | null {
    const state = this.connectionState.peek();
    if (state.type === 'Connected') {
      return state.serverPresence.nodeId;
    }
    return null;
  }

  // ── Connection loop ─────────────────────────────────────────────────────

  /// Main connection loop with automatic reconnection
  // Rust: async fn run_connection_loop(inner: Arc<Inner<SE, PA>>)
  // Divergence: Rust uses tokio::select! loop; TS uses WebSocket events + setTimeout
  // for reconnection. The event-driven model replaces the async stream loop [E17].
  private runConnectionLoop(): void {
    if (this.shutdownRequested) return;

    console.info(`Starting websocket connection loop to ${this.serverUrl}`);
    this.connectOnce();
  }

  /// Attempt a single connection
  // Rust: async fn connect_once(inner: &Arc<Inner<SE, PA>>) -> Result<()>
  // Divergence: Rust uses tokio-tungstenite async streams; TS uses standard WebSocket API
  // with event callbacks (onopen, onmessage, onclose, onerror) [E17].
  private connectOnce(): void {
    if (this.shutdownRequested) return;

    console.info(`Attempting to connect to ${this.serverUrl}`);
    this.connectionState.set({ type: 'Connecting', url: this.serverUrl });

    try {
      const ws = new WebSocket(this.serverUrl);
      ws.binaryType = 'arraybuffer';
      this.ws = ws;

      ws.onopen = () => {
        console.info(`WebSocket handshake completed with ${this.serverUrl}`);
        console.debug('Starting connection handling');

        // Send our presence immediately
        // Rust: proto::Message::Presence(proto::Presence { node_id, durable, system_root })
        const presence = new Message('Presence', {
          presence: new Presence(
            this.node.id(),
            this.node.durable(),
            this.node.systemRoot(),
          ),
        });

        const data = serialize((w) => presence.encode(w));
        ws.send(data);
        console.debug('Sent client presence');
      };

      ws.onmessage = (event: MessageEvent) => {
        this.handleIncomingMessage(event.data);
      };

      ws.onclose = (event: CloseEvent) => {
        console.info(`WebSocket connection closed (code: ${event.code}, reason: ${event.reason})`);
        this.handleDisconnect();
      };

      ws.onerror = (event: Event) => {
        console.error(`WebSocket error on ${this.serverUrl}`);
        // onerror is always followed by onclose, so reconnect logic happens there
      };
    } catch (e) {
      console.error(`Connection to ${this.serverUrl} failed: ${e}`);
      this.connectionState.set({ type: 'Error', error: ConnectionError.general(String(e)) });
      this.connected = false;
      this.scheduleReconnect();
    }
  }

  // ── Message handlers ──────────────────────────────────────────────────────

  // Rust: async fn handle_incoming_message(inner, msg, peer_sender, outgoing_rx, sink) -> Result<MessageResult>
  // Divergence: Rust pattern-matches on tungstenite Message variants; TS handles
  // the raw ArrayBuffer from WebSocket onmessage [E17].
  private handleIncomingMessage(data: unknown): void {
    if (!(data instanceof ArrayBuffer)) {
      console.debug('Received non-binary message, ignoring');
      return;
    }

    const bytes = new Uint8Array(data);

    let message: Message;
    try {
      message = deserialize(bytes, (r) => Message.decode(r));
    } catch (e) {
      console.warn(`Failed to deserialize message: ${e}`);
      return;
    }

    message.match({
      Presence: (v) => {
        this.handleServerPresence(v.presence);
      },
      PeerMessage: (v) => {
        this.handlePeerMessage(v.nodeMessage);
      },
    });
  }

  // Rust: async fn handle_server_presence(inner, server_presence, peer_sender, outgoing_rx)
  private handleServerPresence(serverPresence: Presence): void {
    console.info(`Received server presence: ${serverPresence.nodeId}`);

    // Clean up any existing peer sender
    if (this.peerSender !== null) {
      this.peerSender.close();
    }

    const { sender } = WebsocketPeerSender.new(serverPresence.nodeId);

    // Set up outgoing message handler — sends messages over the WebSocket
    sender.setOutgoingHandler((nodeMessage: NodeMessage) => {
      this.handleOutgoingMessage(nodeMessage);
    });

    this.node.registerPeer(serverPresence, sender);
    this.peerSender = sender;

    this.connectionState.set({
      type: 'Connected',
      url: this.serverUrl,
      serverPresence,
    });
    this.connected = true;
    this.backoffMs = INITIAL_BACKOFF_MS; // Reset backoff on successful connection
    console.info(`Successfully connected to server ${this.serverUrl}`);
  }

  // Rust: async fn handle_peer_message(inner, node_msg)
  private handlePeerMessage(nodeMsg: NodeMessage): void {
    console.debug('Received peer message');
    // Divergence: Rust spawns a tokio task; TS schedules as microtask (single-threaded) [E8]
    this.node.handleMessage(nodeMsg).catch((e) => {
      console.warn(`Error handling peer message: ${e}`);
    });
  }

  // Rust: async fn handle_outgoing_message(sink, msg) -> Result<()>
  private handleOutgoingMessage(nodeMessage: NodeMessage): void {
    if (this.ws === null || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('Cannot send message — WebSocket not open');
      return;
    }

    const protoMessage = new Message('PeerMessage', { nodeMessage });
    try {
      const data = serialize((w) => protoMessage.encode(w));
      this.ws.send(data);
    } catch (e) {
      console.error(`Failed to serialize outgoing message: ${e}`);
    }
  }

  // ── Reconnection ──────────────────────────────────────────────────────────

  private handleDisconnect(): void {
    this.connected = false;
    this.cleanupPeer();

    if (this.shutdownRequested) {
      console.info('Shutdown requested, stopping reconnection attempts');
      this.connectionState.set({ type: 'Disconnected' });
      return;
    }

    console.info(`Retrying connection in ${this.backoffMs}ms`);
    this.connectionState.set({
      type: 'Error',
      error: ConnectionError.general('Connection lost'),
    });

    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    if (this.shutdownRequested) return;

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connectOnce();
    }, this.backoffMs);

    // Exponential backoff
    this.backoffMs = Math.min(this.backoffMs * 2, MAX_BACKOFF_MS);
  }

  // ── Cleanup ───────────────────────────────────────────────────────────────

  private cleanupPeer(): void {
    if (this.peerSender !== null) {
      const peerId = this.peerSender.recipientNodeId();
      this.peerSender.close();
      this.node.deregisterPeer(peerId);
      console.debug(`Deregistered peer ${peerId}`);
      this.peerSender = null;
    }
  }
}

// Divergence: Rust impl Drop for WebsocketClient — aborts the tokio task.
// TS has no destructor. The caller must call shutdown() explicitly.
// Alternatively, could use FinalizationRegistry, but that's unreliable. [E8]
