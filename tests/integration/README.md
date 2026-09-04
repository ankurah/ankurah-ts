# Integration tests

These tests run the TypeScript side against a real Rust ankurah node. Everything else in
this repository tests TypeScript against fixtures — bytes captured from Rust once and read
back later. Here the Rust is running while the test runs, so the two sides have to agree
about the wire in both directions, at the same moment, or the test fails.

## Running them

```bash
bun test tests/integration                      # everything here
bun test tests/integration/handshake.test.ts    # just the handshake
```

These are run explicitly, by naming the path as above. A plain root `bun test` does not pick
them up: the root `bunfig.toml` points `bun test` at `packages`, and these live outside it
on purpose, so the unit tests never build or start a Rust node to run.

The first run compiles the Rust node — around 200 crates from nothing, which took 17
seconds on the machine this was written on. Later runs cost almost nothing: Cargo finds its
artifacts fresh in a tenth of a second, and starting a node takes a fraction of a second
more.

## What is here

| Path | What it is |
| --- | --- |
| `durable-node/` | A small Rust binary: a durable ankurah node with sled storage and the websocket server, taking `--bind` and `--storage-dir`, printing `READY <addr:port>` once it is listening, and exiting on SIGTERM. |
| `support/durable-node.ts` | `startDurableNode()` — builds the binary if Cargo says it needs building, starts it on an operating-system-assigned port with a fresh temporary database, waits for `READY`, and hands back `{ url, wsUrl, port, storageDir, stderr(), stop() }`. `stop()` kills the node and deletes its database. |
| `handshake.test.ts` | The handshake, byte for byte, against that node. |
| `node.test.ts` | The milestone this harness exists for, written as tests that fail today. |

## What the handshake test proves today

A Rust node joins another node by opening a websocket to `/ws` and exchanging one Presence
message in each direction. `handshake.test.ts` performs that exchange from TypeScript:

1. **The server speaks first.** The Rust server sends its Presence the moment the socket
   upgrades, without being asked. The test decodes it with `@ankurah/proto` and checks the
   three things a client depends on: the node says it is durable, it carries the system root
   an ephemeral peer needs in order to join, and its node id is a well-formed 16-byte id.
2. **The decode is complete, not merely successful.** The test insists the decoder consumed
   the whole frame, and that re-encoding what it decoded reproduces the server's bytes
   exactly. A field read at the wrong width, or one skipped entirely, fails here rather than
   somewhere downstream.
3. **The server can read what TypeScript writes.** The test sends its own Presence and then a
   peer request. The Rust server refuses peer messages until it has read a Presence, so
   getting a response back at all is the proof that the bytes TypeScript produced were the
   bytes Rust expected. The response is decoded and checked against the request that
   provoked it: same request id, from the server, addressed to us.
4. **That proof is itself checked.** A second test sends the same peer request on a
   connection that never sent a Presence, and waits for the answer that never comes. Without
   it, step 3's response could have meant nothing.

Every byte in either direction is produced or read by the transpiled `@ankurah/proto`
package. Nothing in the test is a hand-written byte string.

What it does not prove: nothing here creates an entity, runs a query, or holds a
subscription. That is `node.test.ts`, below.

## What it will prove

`node.test.ts` names the milestone: a pure-TypeScript ephemeral node on Bun, with memory
storage and the websocket connector, exchanging entities and subscription updates with a
Rust durable node. Four tests, one per direction of each half:

- an entity created in TypeScript, read back from a second Rust connection
- an entity created in Rust, read from the TypeScript node
- a TypeScript subscription receiving an update made on the Rust node
- a Rust subscription receiving an update made on the TypeScript node

All four fail today, and they fail rather than skip on purpose: a skipped test disappears
into a summary line, while a failing one with a reason keeps saying what is missing. The
reason today is that `@ankurah/core` is not ported, so there is no TypeScript node to create
entities with. The Rust half of each of these is already written, in
`ankurah-ts-support/tests/tests/websocket.rs`; when core lands, each test body becomes the
TypeScript side of the flow described in the comment above it.

## Toolchain

The durable node is a Rust crate, so running these tests needs Cargo and the nightly
toolchain pinned in `durable-node/rust-toolchain.toml` — the same pin the ankurah support
checkout uses. `rustup` installs it on demand.

It also needs the support checkout itself, beside this repository at
`../ankurah-ts-support` (see `port/port-runbook.md`). The crate's dependencies are path
dependencies into that checkout.

By default Cargo builds into the support checkout's `target/` directory, which already holds
most of what this binary links against. Building anywhere else means a second copy of the
same gigabyte of artifacts. Set `ANKURAH_DURABLE_NODE_TARGET_DIR` to do that anyway.

## The support checkout is never modified

`ankurah-ts-support` is the Rust baseline this port is written against. It is read-only:
nothing in this harness adds to it, edits it, or fixes it, and a test that seems to need a
change there needs a different test instead. Cargo writing compiled artifacts under its
`target/` directory is the one exception.

That is why the durable node lives here rather than there, even though
`ankurah-ts-support/examples/server` is nearly the same program. The differences this
harness needs — a port and a storage directory from the command line, a readiness line, no
background task inventing log entries — are changes to that example, so it was rewritten
here instead of edited there.
