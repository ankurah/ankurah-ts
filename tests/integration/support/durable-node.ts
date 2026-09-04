// Starts and stops the Rust durable node that the integration tests talk to.
//
// A test that needs a real peer calls `startDurableNode()` and gets back a running node
// with its own empty database, plus a `stop()` that takes the node down and deletes that
// database. Nothing is shared between tests: each node gets its own port and its own
// temporary directory, so tests can run in any order and leave no residue.
//
// The port comes from the operating system, not from this file. Asking here for a port that
// looks free and hoping it is still free when the node binds it is a race that shows up as a
// mystery failure once in a while; instead the node binds port 0, the kernel hands it a port
// nobody else has, and the node reports which one in its READY line.
//
// The Rust source lives at tests/integration/durable-node. Building it is Cargo's job;
// this file only decides when to ask. The first `startDurableNode()` in a Bun process runs
// `cargo build` once and every later call reuses that result, so a test file pays the build
// cost at most once and usually pays nothing, because Cargo finds its own artifacts fresh.

import { spawn as spawnProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

const CRATE_DIR = path.resolve(import.meta.dir, '../durable-node');
const BINARY_NAME = 'ankurah-ts-durable-node';

// Cargo writes into the support checkout's target directory by default. That checkout has
// already compiled most of what this binary links against, and building somewhere else
// means a second copy of the same gigabyte of artifacts. Writing under the support
// checkout's `target/` is the one thing this harness is allowed to change there. Set
// ANKURAH_DURABLE_NODE_TARGET_DIR to build somewhere else.
const TARGET_DIR =
  process.env.ANKURAH_DURABLE_NODE_TARGET_DIR ??
  path.resolve(import.meta.dir, '../../../../ankurah-ts-support/target');

const DEFAULT_READY_TIMEOUT_MS = 60_000;
const STOP_TIMEOUT_MS = 5_000;

/** A running durable node. */
export interface DurableNode {
  /**
   * The server URL, in the form a websocket client is handed: `ws://127.0.0.1:<port>`.
   * This is the same string Rust's `start_test_server` returns and passes to
   * `WebsocketClient::new`, which appends `/ws` itself.
   */
  readonly url: string;
  /**
   * The websocket endpoint itself, `ws://127.0.0.1:<port>/ws` — what
   * `WebsocketClient::normalize_url` produces and what `new WebSocket(...)` needs.
   */
  readonly wsUrl: string;
  readonly port: number;
  /** The temporary sled directory this node stores into. Deleted by `stop()`. */
  readonly storageDir: string;
  /** Everything the node has written to stderr so far — its tracing output. */
  stderr(): string;
  /** Stop the node and delete its storage directory. Safe to call more than once. */
  stop(): Promise<void>;
}

export interface StartOptions {
  /** How long to wait for the node's READY line. Defaults to 60s, which covers a cold build. */
  readyTimeoutMs?: number;
}

// ── Building ────────────────────────────────────────────────────────────────

let buildOnce: Promise<string> | null = null;

/** Build the durable node if Cargo says it needs building, and return the binary path. */
export function buildDurableNode(): Promise<string> {
  buildOnce ??= runCargoBuild();
  return buildOnce;
}

async function runCargoBuild(): Promise<string> {
  const result = await run('cargo', ['build', '--bin', BINARY_NAME], {
    cwd: CRATE_DIR,
    env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
  });

  if (result.code !== 0) {
    throw new Error(
      `Building the durable node failed (cargo exited ${result.code}).\n` +
        `  crate:      ${CRATE_DIR}\n` +
        `  target dir: ${TARGET_DIR}\n` +
        `The crate needs the nightly toolchain pinned in its rust-toolchain.toml and the\n` +
        `ankurah support checkout beside this repository at ../ankurah-ts-support.\n\n` +
        `cargo stderr:\n${result.stderr}`,
    );
  }

  return path.join(TARGET_DIR, 'debug', BINARY_NAME);
}

// ── Starting ────────────────────────────────────────────────────────────────

export async function startDurableNode(options: StartOptions = {}): Promise<DurableNode> {
  const binary = await buildDurableNode();
  const readyTimeoutMs = options.readyTimeoutMs ?? DEFAULT_READY_TIMEOUT_MS;
  const storageDir = await mkdtemp(path.join(tmpdir(), 'ankurah-durable-node-'));
  try {
    return await launch(binary, storageDir, readyTimeoutMs);
  } catch (error) {
    await rm(storageDir, { recursive: true, force: true });
    throw error;
  }
}

async function launch(binary: string, storageDir: string, readyTimeoutMs: number): Promise<DurableNode> {
  const bind = '127.0.0.1:0';
  const child = spawnProcess(binary, ['--bind', bind, '--storage-dir', storageDir], {
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  let stdoutText = '';
  let stderrText = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk: string) => {
    stderrText += chunk;
  });

  const readyAddress = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      reject(
        new Error(
          `The durable node did not print READY within ${readyTimeoutMs}ms.\n` +
            `  binary: ${binary}\n  bind:   ${bind}\n\nnode stderr:\n${stderrText}`,
        ),
      );
    }, readyTimeoutMs);

    const settle = (fn: () => void) => {
      clearTimeout(timer);
      fn();
    };

    child.stdout.on('data', (chunk: string) => {
      stdoutText += chunk;
      const match = /^READY (.+)$/m.exec(stdoutText);
      if (match) settle(() => resolve(match[1]!.trim()));
    });

    child.on('error', (error) => {
      settle(() => reject(new Error(`Could not run the durable node binary at ${binary}: ${error.message}`)));
    });

    child.on('exit', (code, signal) => {
      settle(() =>
        reject(
          new Error(
            `The durable node exited before it was ready (code ${code}, signal ${signal}).\n` +
              `  binary: ${binary}\n  bind:   ${bind}\n\nnode stderr:\n${stderrText}`,
          ),
        ),
      );
    });
  });

  const port = Number(readyAddress.slice(readyAddress.lastIndexOf(':') + 1));
  if (!Number.isInteger(port) || port <= 0) {
    child.kill('SIGKILL');
    throw new Error(`The durable node reported an address this harness cannot read a port from: READY ${readyAddress}`);
  }

  const exited = new Promise<void>((resolve) => child.on('exit', () => resolve()));
  let stopped: Promise<void> | null = null;

  return {
    url: `ws://${readyAddress}`,
    wsUrl: `ws://${readyAddress}/ws`,
    port,
    storageDir,
    stderr: () => stderrText,
    stop: () => (stopped ??= stopNode(child, exited, storageDir)),
  };
}

async function stopNode(child: ReturnType<typeof spawnProcess>, exited: Promise<void>, storageDir: string): Promise<void> {
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGTERM');
    // A node that ignores SIGTERM must not hold the test process open, so give it a few
    // seconds and then take it down. The timer is cleared either way: a pending timer would
    // keep Bun's event loop alive after the tests are done.
    let killTimer: ReturnType<typeof setTimeout> | undefined;
    const timedOut = await Promise.race([
      exited.then(() => false),
      new Promise<boolean>((resolve) => {
        killTimer = setTimeout(() => resolve(true), STOP_TIMEOUT_MS);
      }),
    ]);
    clearTimeout(killTimer);
    if (timedOut) {
      child.kill('SIGKILL');
      await exited;
    }
  }
  await rm(storageDir, { recursive: true, force: true });
}

// ── Small helpers ───────────────────────────────────────────────────────────

function run(
  command: string,
  args: string[],
  options: { cwd: string; env: NodeJS.ProcessEnv },
): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawnProcess(command, args, { cwd: options.cwd, env: options.env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk: string) => {
      stderr += chunk;
    });
    child.on('error', (error) => reject(new Error(`could not run \`${command}\`: ${error.message}`)));
    child.on('exit', (code) => resolve({ code: code ?? -1, stdout, stderr }));
  });
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
