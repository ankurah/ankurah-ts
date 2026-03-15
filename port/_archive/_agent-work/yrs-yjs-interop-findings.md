# Yrs/Yjs Interoperability Research Findings

> Research date: 2026-02-10
> Researcher: Claude agent
> Context: ankurah-ts TypeScript port of ankurah (Rust)

---

## 1. ankurah's Yrs Usage (Actual Code Analysis)

### Version
- **Yrs crate version**: `0.24.0` (pinned in `core/Cargo.toml` line 46: `yrs = "0.24.0"`)
- **Cargo.lock confirms**: `yrs 0.24.0` from crates.io registry
- Source: `/Users/daniel/ak/ankurah/core/Cargo.toml`, `/Users/daniel/ak/ankurah/Cargo.lock`

### How Yrs Doc Is Created

ankurah creates Yrs documents with **`yrs::Doc::new()`** -- the default constructor with no explicit options:

```rust
// ankurah/core/src/property/backend/yrs.rs lines 32-36
pub fn new() -> Self {
    let doc = yrs::Doc::new();
    let starting_state = doc.transact().state_vector();
    Self { doc, previous_state: Mutex::new(starting_state), field_broadcasts: Mutex::new(BTreeMap::new()) }
}
```

And when reconstructing from state:

```rust
// ankurah/core/src/property/backend/yrs.rs lines 143-152
fn from_state_buffer(state_buffer: &Vec<u8>) -> std::result::Result<Self, crate::error::RetrievalError> {
    let doc = yrs::Doc::new();
    let mut txn = doc.transact_mut();
    let update = yrs::Update::decode_v2(state_buffer).map_err(|e| ...)?;
    txn.apply_update(update).map_err(|e| ...)?;
    txn.commit();
    drop(txn);
    let starting_state = doc.transact().state_vector();
    Ok(Self { doc, previous_state: Mutex::new(starting_state), field_broadcasts: Mutex::new(BTreeMap::new()) })
}
```

### Client ID Assignment

**`yrs::Doc::new()` generates a random client_id** using `fastrand` (a dependency of yrs 0.24.0 visible in Cargo.lock). The Yrs `Doc::new()` internally creates an `Options::default()` which assigns a random `u64` as the client ID.

**ankurah does NOT set an explicit client_id.** There is no usage of `Doc::with_client_id`, `Options`, or any client_id configuration anywhere in the ankurah codebase. Each `YrsBackend::new()` call gets a fresh random client_id.

**Implication for ankurah-ts**: The TypeScript port can also use `new Y.Doc()` with default random `clientID`. Since each Doc gets a random ID, there is no coordination scheme to replicate. The existing spec's question about whether ankurah uses deterministic client IDs is answered: **it does not**.

### CRITICAL FINDING: ankurah Uses V2 Encoding (Not V1)

The existing spec document (`yrs-yjs-interop-validation.md`) states ankurah uses V1 encoding. **This is incorrect.** The actual code uses **V2 encoding exclusively**:

| Operation | Method | Encoding |
|-----------|--------|----------|
| State serialization | `txn.encode_state_as_update_v2(&StateVector::default())` | **V2** |
| State deserialization | `Update::decode_v2(state_buffer)` | **V2** |
| Diff/operations encoding | `txn.encode_diff_v2(&previous_state)` | **V2** |
| Operation application | `Update::decode_v2(update)` | **V2** |
| Empty update check | `Update::EMPTY_V2` | **V2** |

Source code evidence (all from `/Users/daniel/ak/ankurah/core/src/property/backend/yrs.rs`):

```rust
// Line 139: State serialization
let state_buffer = txn.encode_state_as_update_v2(&yrs::StateVector::default());

// Line 146: State deserialization
let update = yrs::Update::decode_v2(state_buffer).map_err(|e| ...)?;

// Line 159: Diff encoding for operations
let diff = txn.encode_diff_v2(&previous_state);

// Line 77: Applying remote updates
let update = Update::decode_v2(update).map_err(|e| ...)?;

// Line 163: Empty update detection
if diff == Update::EMPTY_V2 {
```

### Document Structure

ankurah uses one Yrs `Text` shared type per property, stored at the root level with the property name as the key:

```rust
// Reading a property
let text = txn.get_text(property_name.as_ref());

// Writing a property
let text = self.doc.get_or_insert_text(property_name.as_ref());
text.insert(&mut ytx, index, value);
```

A single `YrsBackend` can hold multiple text properties in one `yrs::Doc`. For example, an entity with fields `title` and `description` would have two root-level Text types: `doc.getText("title")` and `doc.getText("description")`.

### Operation Wire Format

Operations are stored as `ankurah_proto::Operation { diff: Vec<u8> }` where `diff` contains V2-encoded Yrs update bytes. These are grouped by backend name (e.g. `"yrs"`) in an `OperationSet` (`BTreeMap<String, Vec<Operation>>`).

---

## 2. Yjs Compatibility

### Current Yjs Version
- **Latest stable**: 13.6.x series (as of February 2026)
- **Beta/pre-release**: 14.0.0 beta channel
- **Recommendation**: Use latest `13.6.x` (latest stable)
- Source: [yjs on npm](https://www.npmjs.com/package/yjs), [yjs GitHub](https://github.com/yjs/yjs)

### V2 Encoding Support in Yjs

Yjs fully supports V2 encoding with dedicated API functions (all suffixed with `V2`):

| Yjs V1 Function | Yjs V2 Function |
|-----------------|----------------|
| `Y.applyUpdate(doc, update)` | `Y.applyUpdateV2(doc, update)` |
| `Y.encodeStateAsUpdate(doc)` | `Y.encodeStateAsUpdateV2(doc)` |
| `Y.diffUpdate(update, sv)` | `Y.diffUpdateV2(update, sv)` |
| `Y.mergeUpdates(updates)` | `Y.mergeUpdatesV2(updates)` |
| `doc.on('update', ...)` | `doc.on('updateV2', ...)` |

Conversion functions are also available:
- `Y.convertUpdateFormatV1ToV2(update)`
- `Y.convertUpdateFormatV2ToV1(update)`

Source: [Yjs Document Updates API](https://docs.yjs.dev/api/document-updates)

### Binary Format Compatibility

**Yrs and Yjs use identical binary encoding formats.** Both use lib0 encoding (a custom variable-length encoding library). The Yrs crate explicitly ships with lib0 encoding that is "compatible with Yjs" -- this is the core design goal of the y-crdt project.

Both V1 and V2 are compatible across Yrs and Yjs:
- **V1**: Variable-length integer encoding, field inference from block relationships
- **V2**: Extends V1 with run-length encoding (inspired by Martin Kleppmann's Automerge research)

V2 provides dramatically better compression for full state snapshots (benchmarks show roughly 5% of V1 size for large documents), but may be slightly less efficient for small incremental updates.

Source: [Deep dive into Yrs architecture](https://www.bartoszsypytkowski.com/yrs-architecture/), [y-crdt README](https://github.com/y-crdt/y-crdt/blob/main/README.md)

### State Vector Encoding

State vectors use the same binary format in both implementations:
- `Y.encodeStateVector(doc)` in Yjs
- `txn.state_vector()` in Yrs (which can be encoded via lib0)

The state vector is a map of `{ clientID: clock }` pairs, encoded with lib0 variable-length encoding. This format is identical in both implementations.

### Known Gotchas and Incompatibilities

1. **No known binary format incompatibilities** between Yrs 0.24.x and Yjs 13.6.x for V1 or V2 encoding.

2. **Client ID must be unique across peers.** Both libraries generate random client IDs (53-bit in Yjs, because JavaScript `Number.MAX_SAFE_INTEGER`; 64-bit in Yrs). Since ankurah uses random IDs with no coordination, collision probability is negligible.

3. **Multiple Yjs versions in bundle**: If the bundler accidentally includes two copies of Yjs (e.g., one from a dependency and one direct), constructor checks break and CRDT corruption can occur. Use package manager deduplication.

4. **Feature parity gap**: Yrs is "in an ongoing process of reaching feature compatibility with Yjs." However, for the features ankurah uses (Text type, basic insert/delete/get, update encoding/decoding), both implementations are fully compatible.

5. **V2 event listener**: When using V2 encoding, you must listen for `'updateV2'` events (not `'update'`), otherwise you get V1-encoded updates from the event listener.

---

## 3. y-protocols Research

### Version
- **Latest stable**: `1.0.6`
- Source: [y-protocols on npm](https://www.npmjs.com/package/y-protocols), [y-protocols GitHub](https://github.com/yjs/y-protocols)

### Sync Protocol Overview

y-protocols defines a binary sync protocol with three message types:

| Message Type | Constant | Purpose |
|-------------|----------|---------|
| SyncStep1 | `0` | Contains state vector; requests missing updates |
| SyncStep2 | `1` | Contains missing updates (reply to SyncStep1) |
| Update | `2` | Incremental update during live editing |

Source: [y-protocols PROTOCOL.md](https://github.com/yjs/y-protocols/blob/master/PROTOCOL.md)

### Sync Protocol Flow

```
Client A                              Client B
   |                                      |
   |--- SyncStep1 (stateVectorA) ------->|
   |<-- SyncStep1 (stateVectorB) --------|
   |                                      |
   |<-- SyncStep2 (missingUpdatesForA) ---|
   |--- SyncStep2 (missingUpdatesForB) -->|
   |                                      |
   | (both now synced, exchange Updates)  |
   |--- Update (incrementalChange) ------>|
   |<-- Update (incrementalChange) -------|
```

### Key API Functions

```typescript
import * as Y from 'yjs'
import * as syncProtocol from 'y-protocols/sync'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'

// Step 1: Send our state vector
const encoder = encoding.createEncoder()
syncProtocol.writeSyncStep1(encoder, doc)
const message = encoding.toUint8Array(encoder)
// send message to peer

// Step 2: On receiving a message, process it
const decoder = decoding.createDecoder(receivedMessage)
const encoder2 = encoding.createEncoder()
const messageType = syncProtocol.readSyncMessage(decoder, encoder2, doc, 'origin')
// if encoder2 has content, send it back

// Sending incremental updates
const updateEncoder = encoding.createEncoder()
syncProtocol.writeUpdate(updateEncoder, update)
const updateMessage = encoding.toUint8Array(updateEncoder)
```

### Relevance to ankurah-ts

ankurah does NOT use y-protocols for its sync mechanism. ankurah has its own event-sourced sync protocol (`NodeMessage` with operations/events). However, y-protocols could be useful as a reference implementation for the state-vector exchange pattern.

For ankurah-ts, the sync protocol is ankurah's own `NodeMessage`-based protocol (via WebSocket), not y-protocols. The Yrs/Yjs update bytes travel inside ankurah's `Operation.diff` field. **y-protocols is NOT a dependency for ankurah-ts.**

---

## 4. Encoding Format Details

### V1 Encoding

- Uses lib0 variable-length integer encoding
- Relies on block store layout to omit inferrable fields
- Each update contains: struct info, client ID, clock, length, parent info, content
- Default format for `Y.encodeStateAsUpdate()` and `doc.on('update', ...)`
- Good for small incremental updates (individual keystrokes)

### V2 Encoding

- Extends V1 with run-length encoding
- Groups similar fields together for better compression
- Significantly smaller for full document state snapshots (roughly 5% of V1 for large docs)
- Available via `Y.encodeStateAsUpdateV2()` and `doc.on('updateV2', ...)`
- **This is what ankurah uses** (confirmed by code analysis)

### State Vector Encoding

A state vector is a `Map<ClientID, Clock>` encoded as:
- Number of entries (variable-length uint)
- For each entry: ClientID (variable-length uint), Clock (variable-length uint)

The encoding is identical in Yrs and Yjs (both use lib0 format). Importantly, the state vector encoding does NOT have separate V1/V2 variants -- there is only one state vector encoding, shared by both V1 and V2 update paths.

### Binary Format Identity

For the same logical document state:
- `yrs::TransactionMut::encode_state_as_update_v2()` produces bytes that `Y.applyUpdateV2()` can decode
- `yrs::Update::decode_v2(bytes)` can decode bytes from `Y.encodeStateAsUpdateV2()`
- `Y.applyUpdateV2(doc, bytes)` accepts bytes from Yrs `encode_state_as_update_v2()`

This is the core promise of the y-crdt project: binary protocol compatibility with Yjs.

---

## 5. React Native / Expo Go Compatibility

### Yjs is Pure JavaScript -- With a Caveat

Yjs itself is pure JavaScript with no native module dependencies. However, it relies on `crypto.getRandomValues()` for generating client IDs, which is **not available in all React Native environments**.

### Known Issue: `crypto` Not Available

The Hermes JavaScript engine (used by React Native / Expo) does not natively provide `crypto.getRandomValues()`. This causes Yjs to throw an error on initialization:

```
Property 'crypto' doesn't exist
```

Source: [Yjs community discussion](https://discuss.yjs.dev/t/has-anyone-installed-yjs-for-react-native/2137)

### Solution: Polyfill

**Option A (Expo Go compatible):** Use `expo-crypto` which is available in Expo Go without a dev build:

```typescript
// Must run BEFORE importing Yjs
import { getRandomValues } from 'expo-crypto';
if (!globalThis.crypto) {
    globalThis.crypto = { getRandomValues } as any;
}

// Now safe to import Yjs
import * as Y from 'yjs';
```

**Option B (requires dev build):** Install `react-native-get-random-values` and import it before Yjs:

```typescript
// Must be the FIRST import in your entry point
import 'react-native-get-random-values';
import * as Y from 'yjs';
```

Note: `react-native-get-random-values` is a native module, so it requires a development build in Expo (not compatible with Expo Go). Option A using `expo-crypto` is preferred for Expo Go compatibility.

### Confirmed Working

Users have reported Yjs working in Expo with:
- Expo SDK 48+ / React Native 0.71+ with the crypto polyfill
- y-websocket for network sync
- y-expo-sqlite for local persistence

### y-expo-sqlite

There is an existing community package [y-expo-sqlite](https://github.com/brentvatne/y-expo-sqlite) (by Brent Vatne / Expo team) that provides a Yjs persistence provider backed by expo-sqlite. This validates that Yjs works well in the Expo ecosystem.

---

## 6. Practical Validation Plan

### Overview

The validation should confirm that Yrs (Rust, v0.24.0) and Yjs (JavaScript, v13.6.x) can exchange **V2-encoded** updates bidirectionally.

### Test 1: Yrs State -> Yjs (Rust generates, JS reads)

**Rust side** (add to ankurah test suite or standalone binary):

```rust
use yrs::{Doc, Text, Transact, ReadTxn, GetString, StateVector};

fn generate_fixture() -> Vec<u8> {
    let doc = Doc::new();
    let text = doc.get_or_insert_text("title");
    let mut txn = doc.transact_mut();
    text.insert(&mut txn, 0, "Hello World");
    // Use V2 encoding (matching ankurah's actual usage)
    let state = txn.encode_state_as_update_v2(&StateVector::default());
    state
}
```

**JavaScript side**:

```typescript
import * as Y from 'yjs';

test('Yrs V2 state loads in Yjs', () => {
    // Load fixture bytes generated by Rust
    const yrsState = readFixtureFile('yrs_hello_v2.bin');

    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, yrsState);  // Note: applyUpdateV2, not applyUpdate!

    const text = doc.getText('title');
    expect(text.toString()).toBe('Hello World');
});
```

### Test 2: Yjs State -> Yrs (JS generates, Rust reads)

**JavaScript side**:

```typescript
const doc = new Y.Doc();
const text = doc.getText('message');
text.insert(0, 'From JavaScript');
const update = Y.encodeStateAsUpdateV2(doc);
writeFixtureFile('yjs_message_v2.bin', update);
```

**Rust side**:

```rust
use yrs::{Doc, Text, Transact, ReadTxn, GetString, Update};
use yrs::updates::decoder::Decode;

fn verify_yjs_fixture(bytes: &[u8]) {
    let doc = Doc::new();
    let mut txn = doc.transact_mut();
    let update = Update::decode_v2(bytes).unwrap();
    txn.apply_update(update).unwrap();
    txn.commit();
    drop(txn);

    let txn = doc.transact();
    let text = txn.get_text("message").unwrap();
    assert_eq!(text.get_string(&txn), "From JavaScript");
}
```

### Test 3: Bidirectional Incremental Operations

This mimics the actual ankurah sync flow:

```typescript
test('incremental V2 operations round-trip', () => {
    // 1. Start with Yrs-generated base state
    const baseState = readFixture('yrs_base_v2.bin');
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, baseState);
    expect(doc.getText('title').toString()).toBe('Hello');

    // 2. Apply a Yrs-generated incremental operation
    const yrsOp = readFixture('yrs_append_world_v2.bin');
    Y.applyUpdateV2(doc, yrsOp);
    expect(doc.getText('title').toString()).toBe('Hello World');

    // 3. Make a Yjs edit and export as V2 update
    const stateVectorBefore = Y.encodeStateVector(doc);
    doc.getText('title').insert(11, '!');

    // Get the diff since the last state
    const yjsUpdate = Y.encodeStateAsUpdateV2(doc, stateVectorBefore);

    // 4. This yjsUpdate can be sent back to Rust and applied:
    //    let update = Update::decode_v2(&yjs_update_bytes).unwrap();
    //    txn.apply_update(update).unwrap();
    //    assert_eq!(text.get_string(&txn), "Hello World!");
});
```

### Test 4: Concurrent Edit Merge

```typescript
test('concurrent edits merge identically in Yrs and Yjs', () => {
    // 1. Common base state (generated by Yrs)
    const baseState = readFixture('yrs_concurrent_base_v2.bin');

    // 2. Fork A: Yrs makes an edit (fixture)
    const forkA_update = readFixture('yrs_concurrent_editA_v2.bin');

    // 3. Fork B: Yjs makes an edit
    const docB = new Y.Doc();
    Y.applyUpdateV2(docB, baseState);
    docB.getText('content').insert(5, ' beautiful');
    const forkB_update = Y.encodeStateAsUpdateV2(docB);

    // 4. Merge both in Yjs
    const merged = new Y.Doc();
    Y.applyUpdateV2(merged, baseState);
    Y.applyUpdateV2(merged, forkA_update);
    Y.applyUpdateV2(merged, forkB_update);

    // 5. Compare with Yrs merge result (fixture)
    const yrsMergedState = readFixture('yrs_concurrent_merged_v2.bin');
    const yrsDoc = new Y.Doc();
    Y.applyUpdateV2(yrsDoc, yrsMergedState);

    // Both must produce identical text
    expect(merged.getText('content').toString())
        .toBe(yrsDoc.getText('content').toString());
});
```

### Test 5: ankurah-Specific Multi-Property Pattern

```typescript
test('ankurah YrsBackend multi-property pattern', () => {
    // ankurah stores multiple text properties in a single Yrs Doc
    const state = readFixture('ankurah_multi_property_v2.bin');

    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, state);

    // Each property is a separate root-level Text type
    expect(doc.getText('title').toString()).toBe('Test Album');
    expect(doc.getText('artist').toString()).toBe('Test Artist');
    expect(doc.getText('description').toString()).toBe('A great album');
});
```

### Rust Fixture Generator

Add this to ankurah's test suite to generate all test fixtures:

```rust
// ankurah/core/tests/yrs_interop_fixtures.rs

use std::{fs, path::Path};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};
use yrs::updates::decoder::Decode;

#[test]
fn generate_yrs_interop_fixtures_v2() {
    let dir = Path::new("tests/fixtures/yrs-interop");
    fs::create_dir_all(dir).unwrap();

    // Fixture 1: Simple text
    {
        let doc = Doc::new();
        let text = doc.get_or_insert_text("title");
        let mut txn = doc.transact_mut();
        text.insert(&mut txn, 0, "Hello World");
        let state = txn.encode_state_as_update_v2(&StateVector::default());
        fs::write(dir.join("yrs_hello_v2.bin"), &state).unwrap();
    }

    // Fixture 2: Multi-property (ankurah pattern)
    {
        let doc = Doc::new();
        let title = doc.get_or_insert_text("title");
        let artist = doc.get_or_insert_text("artist");
        let desc = doc.get_or_insert_text("description");
        let mut txn = doc.transact_mut();
        title.insert(&mut txn, 0, "Test Album");
        artist.insert(&mut txn, 0, "Test Artist");
        desc.insert(&mut txn, 0, "A great album");
        let state = txn.encode_state_as_update_v2(&StateVector::default());
        fs::write(dir.join("ankurah_multi_property_v2.bin"), &state).unwrap();
    }

    // Fixture 3: Base state for incremental test
    {
        let doc = Doc::new();
        let text = doc.get_or_insert_text("title");
        let mut txn = doc.transact_mut();
        text.insert(&mut txn, 0, "Hello");
        let base_state = txn.encode_state_as_update_v2(&StateVector::default());
        let sv_after = txn.state_vector();
        fs::write(dir.join("yrs_base_v2.bin"), &base_state).unwrap();
        drop(txn);

        // Incremental edit
        let mut txn2 = doc.transact_mut();
        let text = txn2.get_text("title").unwrap();
        text.insert(&mut txn2, 5, " World");
        let diff = txn2.encode_diff_v2(&sv_after);
        fs::write(dir.join("yrs_append_world_v2.bin"), &diff).unwrap();
    }

    // Fixture 4: Concurrent edit base + fork A
    {
        let doc = Doc::new();
        let text = doc.get_or_insert_text("content");
        let mut txn = doc.transact_mut();
        text.insert(&mut txn, 0, "Hello world");
        let base = txn.encode_state_as_update_v2(&StateVector::default());
        fs::write(dir.join("yrs_concurrent_base_v2.bin"), &base).unwrap();
        drop(txn);

        // Fork A: Yrs edit (insert " amazing" after "Hello")
        let doc_a = Doc::new();
        {
            let mut txn = doc_a.transact_mut();
            let update = Update::decode_v2(&base).unwrap();
            txn.apply_update(update).unwrap();
        }
        let sv_a = doc_a.transact().state_vector();
        {
            let text = doc_a.get_or_insert_text("content");
            let mut txn = doc_a.transact_mut();
            text.insert(&mut txn, 5, " amazing");
        }
        let diff_a = doc_a.transact_mut().encode_diff_v2(&sv_a);
        fs::write(dir.join("yrs_concurrent_editA_v2.bin"), &diff_a).unwrap();

        // Full merged state from Yrs side (base + edit A)
        let merged_state = doc_a.transact().encode_state_as_update_v2(&StateVector::default());
        fs::write(dir.join("yrs_concurrent_merged_v2.bin"), &merged_state).unwrap();
    }
}
```

---

## 7. Summary of Key Findings

### Corrections to Existing Spec

The existing spec (`yrs-yjs-interop-validation.md`) contains an **incorrect claim** about encoding format:

| Spec Claim | Actual Code |
|-----------|-------------|
| "ankurah uses V1 encoding (`encode_state_as_update_v1`)" | ankurah uses **V2** encoding (`encode_state_as_update_v2`, `decode_v2`, `encode_diff_v2`) |
| "V1 encoding: Compatible (this is the primary interop format)" | V2 is the actual interop format; V2 is also fully compatible |
| Sample code shows `Update::decode_v1` | Actual code uses `Update::decode_v2` |
| "Decision: V1 vs V2 Encoding ... ankurah currently uses V1" | **Wrong**. ankurah uses V2 exclusively. |

### Version Matrix

| Component | Version | Notes |
|-----------|---------|-------|
| Yrs (Rust) | 0.24.0 | Used by ankurah, pinned in Cargo.toml |
| Yjs (JS) | 13.6.x | Latest stable, recommended for ankurah-ts |
| y-protocols | 1.0.6 | Latest stable; NOT needed for ankurah sync (ankurah has its own protocol) |
| lib0 | (transitive) | Encoding library used by both Yrs and Yjs; pulled in as Yjs dependency |

### TypeScript Port: YjsBackend Implementation Guide

For the `YjsBackend` class in ankurah-ts:

```typescript
import * as Y from 'yjs';

interface Operation {
    diff: Uint8Array;
}

class YjsBackend implements PropertyBackend {
    private doc: Y.Doc;
    private previousStateVector: Uint8Array;

    constructor() {
        this.doc = new Y.Doc();
        this.previousStateVector = Y.encodeStateVector(this.doc);
    }

    // Matches: YrsBackend::to_state_buffer()
    toStateBuffer(): Uint8Array {
        return Y.encodeStateAsUpdateV2(this.doc);  // V2!
    }

    // Matches: YrsBackend::from_state_buffer()
    static fromStateBuffer(buffer: Uint8Array): YjsBackend {
        const backend = new YjsBackend();
        Y.applyUpdateV2(backend.doc, buffer);  // V2!
        backend.previousStateVector = Y.encodeStateVector(backend.doc);
        return backend;
    }

    // Matches: YrsBackend::to_operations()
    toOperations(): Operation[] | null {
        const diff = Y.encodeStateAsUpdateV2(
            this.doc,
            this.previousStateVector
        );
        this.previousStateVector = Y.encodeStateVector(this.doc);

        // Check for empty V2 update
        // Yrs uses Update::EMPTY_V2 constant; in Yjs we need to check
        // if the diff is trivially empty (only header bytes, no content)
        if (this.isEmptyV2Update(diff)) return null;
        return [{ diff }];
    }

    // Matches: YrsBackend::apply_operations()
    applyOperations(operations: Operation[]): void {
        for (const op of operations) {
            Y.applyUpdateV2(this.doc, op.diff);  // V2!
        }
    }

    // Matches: YrsBackend::get_string()
    getString(propertyName: string): string | undefined {
        const text = this.doc.getText(propertyName);
        const value = text.toString();
        return value.length > 0 ? value : undefined;
    }

    // Matches: YrsBackend::insert()
    insert(propertyName: string, index: number, value: string): void {
        this.doc.getText(propertyName).insert(index, value);
    }

    // Matches: YrsBackend::delete()
    delete(propertyName: string, index: number, length: number): void {
        this.doc.getText(propertyName).delete(index, length);
    }

    private isEmptyV2Update(update: Uint8Array): boolean {
        // The Yrs constant Update::EMPTY_V2 is [0, 0, 0, 0]
        // Verify this matches by comparing length and content
        return update.length === 4
            && update[0] === 0
            && update[1] === 0
            && update[2] === 0
            && update[3] === 0;
    }
}
```

### Expo Go Compatibility Checklist

1. Yjs is pure JS -- works in Expo Go with a polyfill
2. **Must polyfill `crypto.getRandomValues()`** before importing Yjs
3. Use `expo-crypto` for Expo Go compatibility (no dev build needed):
   ```typescript
   import { getRandomValues } from 'expo-crypto';
   if (!globalThis.crypto) {
       globalThis.crypto = { getRandomValues } as any;
   }
   ```
4. Alternatively, `react-native-get-random-values` works but requires a dev build
5. Community package `y-expo-sqlite` validates the Yjs + Expo ecosystem

### Open Questions

1. **V2 empty update detection in Yjs**: ankurah Rust checks `diff == Update::EMPTY_V2`. The Yrs source defines `EMPTY_V2` as `[0, 0, 0, 0]` (4 zero bytes). Need to verify this is the same output Yjs produces for an empty V2 diff, or if Yjs uses a different sentinel. The `isEmptyV2Update` method above is a hypothesis that needs validation.

2. **State vector encoding for diff computation**: ankurah stores a `previous_state: StateVector` as a Yrs in-memory struct and uses `encode_diff_v2(&previous_state)`. In Yjs, the nearest equivalent is `Y.encodeStateAsUpdateV2(doc, previousStateVectorBytes)` where `previousStateVectorBytes` is the `Uint8Array` from `Y.encodeStateVector(doc)` captured at the previous point in time. The Yjs API takes the _encoded_ state vector, while Yrs takes the _decoded_ struct. Functionally equivalent but the API surface differs.

3. **y-protocols necessity**: ankurah's own sync protocol handles sync. y-protocols is NOT needed unless we want to add Yjs-native sync as an alternative transport. For Phase 1, skip y-protocols entirely.

4. **Yjs getText on nonexistent key**: In Yrs, `txn.get_text("nonexistent")` returns `None`. In Yjs, `doc.getText("nonexistent")` always returns a `Y.Text` instance (it auto-creates). The Rust code distinguishes between "text exists" and "text doesn't exist" in `get_property_string()`. The TS port needs to handle this differently -- likely by checking `text.toString().length === 0` or tracking which properties have been initialized.

---

## References

- [yjs on npm](https://www.npmjs.com/package/yjs) - Latest version
- [Yjs Document Updates API](https://docs.yjs.dev/api/document-updates) - V1/V2 encoding API
- [Yjs GitHub](https://github.com/yjs/yjs) - Source code
- [y-crdt/y-crdt GitHub](https://github.com/y-crdt/y-crdt) - Yrs Rust implementation
- [yrs on crates.io](https://crates.io/crates/yrs) - Yrs 0.24.0
- [yrs docs.rs](https://docs.rs/yrs/latest/yrs/) - Yrs API documentation
- [Deep dive into Yrs architecture](https://www.bartoszsypytkowski.com/yrs-architecture/) - V1/V2 encoding details
- [y-protocols GitHub](https://github.com/yjs/y-protocols) - Sync protocol specification
- [y-protocols PROTOCOL.md](https://github.com/yjs/y-protocols/blob/master/PROTOCOL.md) - Binary sync protocol spec
- [y-expo-sqlite](https://github.com/brentvatne/y-expo-sqlite) - Yjs + Expo SQLite persistence
- [Yjs RN discussion](https://discuss.yjs.dev/t/has-anyone-installed-yjs-for-react-native/2137) - React Native compatibility
- [y-crdt/y-sync](https://github.com/y-crdt/y-sync) - Yrs sync protocol implementation (Rust)
