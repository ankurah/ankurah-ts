# Yrs/Yjs Interoperability Validation

## The Question

ankurah uses **Yrs** (Rust) for collaborative text fields (`YrsString`). The TypeScript port would use **Yjs** (JavaScript). These are supposed to be compatible implementations of the same CRDT algorithm - Yrs is literally a port of Yjs to Rust. But "supposed to be" requires validation.

## What Needs To Be Compatible

### 1. Document State Encoding

Yrs and Yjs both support two encoding formats:
- **V1 encoding**: Original format, wider compatibility
- **V2 encoding**: Newer, more compact

ankurah's `YrsBackend::to_state_buffer()` calls `yrs::Doc::encode_state_as_update_v1()` (or v2). The TS port must be able to decode this with `Y.applyUpdate(doc, buffer)`.

**Validation test**: Create a Yrs document in Rust, encode its state, decode it in Yjs, and verify the content matches.

### 2. Update Encoding (Operations)

When ankurah generates operations from a transaction (`YrsBackend::to_operations()`), these are Yrs update bytes. The TS port must be able to apply these updates to a Yjs document.

**Validation test**: Generate operations in Rust, apply them in Yjs, verify the result.

### 3. State Vector Exchange

For sync, nodes exchange state vectors to determine what updates to send. The state vector encoding must be compatible.

**Validation test**: Exchange state vectors between Yrs and Yjs, verify correct delta computation.

### 4. Merge Behavior

When concurrent edits occur, both implementations must resolve conflicts identically.

**Validation test**: Make concurrent edits in Yrs and Yjs, merge, verify identical results on both sides.

## Known Compatibility Status

As of Yrs 0.24.x and Yjs 13.x:
- **V1 encoding**: Compatible (this is the primary interop format)
- **V2 encoding**: Compatible (but verify version-specific behavior)
- **State vectors**: Compatible
- **Client IDs**: Must be unique across Yrs and Yjs nodes (both use random u64)

## Potential Gotchas

### 1. Version Mismatch

Yrs and Yjs evolve independently. A Yrs release may support encoding features that a given Yjs version doesn't understand, or vice versa.

**Mitigation**: Pin specific compatible versions and test cross-version compatibility in CI.

### 2. Client ID Collision

Both Yrs and Yjs generate random client IDs. In theory, collision probability is negligible (2^64 space), but if ankurah uses a deterministic client ID scheme, we need to ensure it's compatible.

**Check**: How does ankurah assign Yrs client IDs? Is it the node's EntityId? If so, the TS port must use the same scheme.

### 3. Document Structure

ankurah uses Yrs with a specific document structure (one Text per property). The TS port must create the same structure:

```rust
// Rust (ankurah YrsBackend)
let doc = Doc::new();
let text = doc.get_or_insert_text("property_name");
text.insert(&mut txn, 0, "value");
```

```typescript
// TypeScript (must match)
const doc = new Y.Doc();
const text = doc.getText("property_name");
text.insert(0, "value");
```

The key names ("property_name") must match exactly.

### 4. Transaction Semantics

Yrs uses explicit transactions (`doc.transact()`). Yjs auto-creates transactions or uses `doc.transact()`. The operation encoding should be identical regardless of transaction API.

### 5. Binary Encoding Details

The actual byte layout of updates:
- Yrs encodes updates as `Vec<u8>` which is then stored in ankurah's operation data
- Yjs uses `Uint8Array` for the same purpose
- These must be byte-identical for the same logical operation

## Validation Test Suite

### Phase 1: Basic Interop (Must Pass Before Proceeding)

```typescript
// test/yrs-yjs-interop.test.ts

describe('Yrs/Yjs interop', () => {
    test('Yrs state can be loaded by Yjs', async () => {
        // 1. Use Rust test to create a Yrs document with known content
        //    and save its state buffer to a fixture file
        const yrsState = readFixture('yrs_text_hello.bin');

        // 2. Load into Yjs
        const doc = new Y.Doc();
        Y.applyUpdate(doc, yrsState);

        // 3. Verify content
        const text = doc.getText('message');
        expect(text.toString()).toBe('hello world');
    });

    test('Yjs state can be loaded by Yrs', async () => {
        // 1. Create Yjs document
        const doc = new Y.Doc();
        const text = doc.getText('message');
        text.insert(0, 'hello world');
        const update = Y.encodeStateAsUpdate(doc);

        // 2. Save update and have Rust test verify it loads correctly
        writeFixture('yjs_text_hello.bin', update);

        // 3. Rust-side test reads this file and verifies
    });

    test('concurrent edits merge identically', async () => {
        // 1. Create base document in Yrs, get state
        const baseState = readFixture('yrs_base.bin');

        // 2. Fork: apply edit A in Yrs, edit B in Yjs
        const docA_state = readFixture('yrs_edit_a.bin');

        const docB = new Y.Doc();
        Y.applyUpdate(docB, baseState);
        docB.getText('message').insert(5, ' beautiful');
        const docB_update = Y.encodeStateAsUpdate(docB);

        // 3. Merge A+B in Yjs
        const merged = new Y.Doc();
        Y.applyUpdate(merged, docA_state);
        Y.applyUpdate(merged, docB_update);

        // 4. Compare with Rust-side merge result
        const rustMerged = readFixture('yrs_merged.bin');
        const rustDoc = new Y.Doc();
        Y.applyUpdate(rustDoc, rustMerged);

        expect(merged.getText('message').toString())
            .toBe(rustDoc.getText('message').toString());
    });

    test('incremental operations round-trip', async () => {
        // 1. Start with empty doc
        // 2. Apply a series of Yrs-generated operations
        // 3. Apply a series of Yjs-generated operations
        // 4. Verify final state matches in both implementations
    });
});
```

### Phase 2: ankurah-Specific Patterns

```typescript
describe('ankurah YrsBackend interop', () => {
    test('YrsBackend state buffer round-trip', async () => {
        // Load an actual ankurah entity's Yrs state buffer
        // This tests the specific way ankurah structures Yrs documents
        const stateBuffer = readFixture('ankurah_yrs_backend_state.bin');
        const doc = new Y.Doc();
        Y.applyUpdate(doc, stateBuffer);

        // Verify all properties are accessible
        expect(doc.getText('title').toString()).toBe('Test Album');
        expect(doc.getText('description').toString()).toBe('');
    });

    test('YrsBackend operations apply correctly', async () => {
        // Load ankurah-generated operations (from a commit event)
        const ops = readFixture('ankurah_yrs_operations.bin');

        const doc = new Y.Doc();
        Y.applyUpdate(doc, ops);

        // Verify the operation result
        expect(doc.getText('title').toString()).toBe('Test Album');
    });
});
```

## Rust-Side Fixtures

Add to ankurah's test suite:

```rust
// ankurah/core/tests/yrs_fixtures.rs

#[test]
fn generate_yrs_interop_fixtures() {
    let fixtures_dir = Path::new("tests/fixtures/yrs");
    fs::create_dir_all(fixtures_dir).unwrap();

    // Basic text
    let doc = Doc::new();
    let text = doc.get_or_insert_text("message");
    let mut txn = doc.transact_mut();
    text.insert(&mut txn, 0, "hello world");
    let state = txn.encode_update_v1();
    drop(txn);
    fs::write(fixtures_dir.join("yrs_text_hello.bin"), &state).unwrap();

    // Empty string (the bug from #175)
    let doc2 = Doc::new();
    let text2 = doc2.get_or_insert_text("message");
    let mut txn2 = doc2.transact_mut();
    text2.insert(&mut txn2, 0, "");
    let state2 = txn2.encode_update_v1();
    drop(txn2);
    fs::write(fixtures_dir.join("yrs_text_empty.bin"), &state2).unwrap();

    // Multiple properties (ankurah pattern)
    let doc3 = Doc::new();
    let title = doc3.get_or_insert_text("title");
    let desc = doc3.get_or_insert_text("description");
    let mut txn3 = doc3.transact_mut();
    title.insert(&mut txn3, 0, "Test Album");
    desc.insert(&mut txn3, 0, "A great album");
    let state3 = txn3.encode_update_v1();
    drop(txn3);
    fs::write(fixtures_dir.join("yrs_multi_property.bin"), &state3).unwrap();

    // Concurrent edit base + fork
    // ... generate fixtures for merge testing
}
```

## Encoding Format Notes

ankurah's `YrsBackend` stores state as:

```rust
// YrsBackend::to_state_buffer()
fn to_state_buffer(&self) -> Result<Vec<u8>> {
    let doc = self.doc.lock().unwrap();
    let txn = doc.transact();
    Ok(txn.encode_state_as_update_v1(&StateVector::default()))
}

// YrsBackend::from_state_buffer()
fn from_state_buffer(buffer: &Vec<u8>) -> Result<Self> {
    let doc = Doc::new();
    let mut txn = doc.transact_mut();
    txn.apply_update(Update::decode_v1(buffer)?);
    drop(txn);
    Ok(Self { doc: Mutex::new(doc) })
}
```

The TS equivalent:

```typescript
// YjsBackend.toStateBuffer()
toStateBuffer(): Uint8Array {
    return Y.encodeStateAsUpdate(this.doc);  // V1 by default
}

// YjsBackend.fromStateBuffer()
static fromStateBuffer(buffer: Uint8Array): YjsBackend {
    const doc = new Y.Doc();
    Y.applyUpdate(doc, buffer);
    return new YjsBackend(doc);
}
```

## Decision: V1 vs V2 Encoding

ankurah currently uses **V1 encoding** (`encode_state_as_update_v1`). The TS port should also use V1 for maximum compatibility. V2 can be adopted later if both Yrs and Yjs versions support it.

## Action Items

1. **Add Yrs interop fixture generation** to ankurah/core tests
2. **Pin Yrs version** in ankurah Cargo.toml (currently 0.24.0)
3. **Pin Yjs version** in ankurah-ts package.json (latest 13.x)
4. **Write cross-implementation test suite** as described above
5. **Verify client ID handling** in ankurah's YrsBackend
6. **Document the encoding version** used (V1) as a project invariant
