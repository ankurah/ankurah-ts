# Spec Cross-Check Analysis

**Generated**: 2026-02-10
**Scope**: All 10 spec files in `/Users/daniel/ak/ankurah-ts/specs/`
**Method**: Line-by-line reading of each spec, cross-referenced against all other specs, then verified against the actual Rust codebase at `/Users/daniel/ak/ankurah/`

---

## 1. CONTRADICTIONS

### 1.1 CRITICAL: V1 vs V2 Yrs Encoding -- Specs Say V1, Rust Uses V2

Multiple specs explicitly state that ankurah uses V1 encoding:

**yrs-yjs-interop-validation.md, line 275:**
> "ankurah currently uses **V1 encoding** (`encode_state_as_update_v1`). The TS port should also use V1 for maximum compatibility."

**yrs-yjs-interop-validation.md, lines 243-244:**
```rust
Ok(txn.encode_state_as_update_v1(&StateVector::default()))
```

**architectural-decisions.md, line 30:**
> "Use V1 encoding (not V2) for maximum cross-implementation compatibility"

**Actual Rust code** at `/Users/daniel/ak/ankurah/core/src/property/backend/yrs.rs`, line 139:
```rust
let state_buffer = txn.encode_state_as_update_v2(&yrs::StateVector::default());
```

And at line 77:
```rust
let update = Update::decode_v2(update).map_err(|e| ...)?;
```

And at line 159:
```rust
let diff = txn.encode_diff_v2(&previous_state);
```

**Verdict**: The Rust codebase exclusively uses **V2 encoding** for all Yrs operations (state buffers, updates, diffs). Every spec that mentions V1 is wrong. This is the single most dangerous error in the specs because it would cause silent data corruption -- a TS client using V1 encoding would produce updates that the Rust server cannot decode, and vice versa.

### 1.2 Wire Protocol Structure: Specs Describe Nonexistent Types

**ecosystem-research.md, lines 136-139:**
> ```
> NodeMessage -> NodeRequest / NodeResponse / NodeUpdate / NodeUpdateAck
> NodeRequestBody: CommitTransaction, Get, GetEvents, Fetch, SubscribeQuery
> ```

**architecture.md, lines 113-114:**
> ```
> NodeMessage | NodeMessage |
> NodeMessageBody | NodeMessageBody (discriminated union) |
> ```

**Actual Rust code** at `/Users/daniel/ak/ankurah/proto/src/message.rs`:
```rust
pub enum Message {
    Presence(Presence),
    PeerMessage(NodeMessage),
}

pub enum NodeMessage {
    Request { auth: Vec<AuthData>, request: NodeRequest },
    Response(NodeResponse),
    Update(NodeUpdate),
    UpdateAck(NodeUpdateAck),
    UnsubscribeQuery { from: EntityId, query_id: QueryId },
}
```

The specs describe `NodeMessage` as a flat struct with fields `{id, from, to, body}` and `NodeMessageBody` as a body enum. In reality:
- The wire-level type is `Message` (not `NodeMessage`), which wraps `Presence` or `PeerMessage(NodeMessage)`.
- `NodeMessage` is itself an enum, not a struct-with-body.
- `NodeRequest` is a separate struct with `{id, to, from, body: NodeRequestBody}`.
- There is no type called `NodeMessageBody` anywhere in the codebase.
- The `UnsubscribeQuery` variant is missing from all spec descriptions.
- `NodeMessage::Request` carries an `auth: Vec<AuthData>` field that no spec mentions.

### 1.3 Operation Structure: Specs Describe Wrong Shape

**wire-format-interop.md, lines 329-338:**
```rust
pub struct Operation {
    pub backend: String,      // "lww" or "yrs"
    pub data: Vec<u8>,        // Backend-specific operation bytes
}
```

**Actual Rust code** at `/Users/daniel/ak/ankurah/proto/src/data.rs`, lines 178-181:
```rust
pub struct Operation {
    pub diff: Vec<u8>,
}
```

And operations are organized by backend via `OperationSet` (line 157):
```rust
pub struct OperationSet(pub BTreeMap<String, Vec<Operation>>);
```

The spec shows `Operation` having a `backend` field. In reality, the backend name is the *key* in the `OperationSet` BTreeMap, and `Operation` only has a `diff` field (not `data`). This affects every bincode encoding calculation.

### 1.4 CLI Codegen: In-Scope vs De-Scoped Contradiction

**design-resume.md, line 25:**
> "**No code generation initially** - Hand-write TypeScript model wrappers to match Rust structs. CLI codegen is a later optimization."

**architectural-decisions.md, lines 23-25:**
> "**Phase 1**: Hand-write TypeScript model wrappers"
> "**Later phase**: Replace hand-written wrappers with a CLI codegen tool"

**architecture.md, lines 223-224:**
> "In Scope: ... CLI code generator for typed wrappers"

**initial-porting-workflow.md, Phase 11 (lines 275-304):**
> Phase 11 explicitly includes building the CLI codegen as part of the initial port.

The architecture.md lists CLI codegen as "In Scope" for Phase 1, and the porting workflow includes it as Phase 11. But design-resume.md and architectural-decisions.md both say "no code generation initially." Either the workflow has 13 phases that go beyond Phase 1, or Phase 1 scope is inconsistent.

### 1.5 Wire Format Decision: Bincode-Only vs JSON-Allowed

**design-resume.md, line 22:**
> "**Bincode wire format required** - No JSON wire format alternative."

**architectural-decisions.md, line 4:**
> "Bincode only. No JSON wire format alternative."

**wire-format-interop.md, lines 308-323:**
> "**Alternative: JSON Wire Format** ... **Recommendation**: Start with JSON wire format for rapid development, add bincode for production optimization."

**architecture.md, lines 131-132:**
> "**Alternative**: Use JSON as the wire format between TS and Rust nodes (simpler, but larger payloads)."

The design-resume and architectural-decisions explicitly reject JSON wire format. But wire-format-interop.md actively recommends starting with JSON, and architecture.md presents it as a viable alternative. These documents fundamentally disagree on a core architectural decision.

---

## 2. UNSTATED ASSUMPTIONS

### 2.1 Bincode Configuration Assumed but Unverified

**ecosystem-research.md, lines 106-112** states the bincode encoding rules:
> - Integers: Little-endian, fixed-size
> - String/Vec lengths: u64 (8 bytes)
> - Enum variant index: u32 (4 bytes)

**design-resume.md, line 119:**
> "What bincode configuration does ankurah use? (default config? varint lengths? fixed-size integers?) Need to examine `bincode::serialize` calls."

This is still listed as an open question, yet ecosystem-research.md asserts specific answers (u64 lengths, u32 variants). The Rust code uses `bincode::serialize()` which uses default bincode v1 configuration (fixed-size integers, u64 lengths). However, bincode v2 uses different defaults (varint lengths). The specs assume v1 defaults but do not verify which version of the `bincode` crate is in use.

**Verification**: The Rust code at `proto/src/data.rs` line 20 uses `bincode::serialize(&entity_id)` and `bincode::serialize(&operations)`. The tests at `proto/src/id.rs` line 177 show `bincode::serialize` producing raw 16-byte output for EntityId (no length prefix), confirming fixed-size array encoding. But the exact crate version and feature flags would need checking in `Cargo.toml`.

### 2.2 EntityId Bincode Encoding: Custom Serialization Not Mentioned

The specs treat EntityId as a simple ULID. But `/Users/daniel/ak/ankurah/proto/src/id.rs` lines 134-160 show a **custom** `Serialize`/`Deserialize` implementation that uses `is_human_readable()` to switch between base64 (JSON) and raw bytes (bincode). The same pattern exists for `EventId` at `/Users/daniel/ak/ankurah/proto/src/data.rs` lines 74-100.

No spec mentions this dual-encoding behavior. The TS bincode implementation must replicate this exact behavior -- raw 16-byte array for EntityId in bincode, but base64 string in JSON.

### 2.3 Yjs `encodeStateAsUpdate` Default Version Assumption

**yrs-yjs-interop-validation.md, line 261:**
```typescript
return Y.encodeStateAsUpdate(this.doc);  // V1 by default
```

This assumes Yjs's `encodeStateAsUpdate` uses V1 by default. This is actually correct for current Yjs (the function is V1), but the Rust code uses V2. So even if the assumption about Yjs defaults is correct, it is the *wrong encoding version* to use. The correct Yjs calls would be `Y.encodeStateAsUpdateV2()` and `Y.applyUpdateV2()`.

### 2.4 `crypto.getRandomValues` Polyfill

**ecosystem-research.md, line 33:**
> "works in Expo Go with the Hermes engine (may need `crypto.getRandomValues` polyfill)"

This is flagged as a "may need" but Yjs relies on `crypto.getRandomValues()` for client ID generation. Hermes does not provide `crypto.getRandomValues` natively. If this polyfill is missing, Yjs will fail at runtime. The `expo-crypto` module provides `getRandomValues` but must be explicitly set up.

### 2.5 Storage Common Is Not "Just Trait Definitions"

**initial-porting-workflow.md, lines 150-159:**
> "Phase 4: Storage Common ... This is a small package - just trait definitions."

**Actual Rust code** at `/Users/daniel/ak/ankurah/storage/common/src/lib.rs`:
```
pub mod bounds;
pub mod filtering;
pub mod planner;
pub mod predicate;
pub mod sorting;
pub mod traits;
pub mod types;
```

This is 7 modules including a query planner, filtering engine, sorting engine, predicate evaluation, bounds checking, and type definitions. This is substantially more than "just trait definitions." The TS port estimate of "~1 file, Low complexity" is a significant underestimate.

### 2.6 Signals Library Structure Mismatch

**initial-porting-workflow.md, Phase 2 (lines 97-119)** lists files to port:
> `broadcast.rs`, `signal.rs`, `signal/mutable.rs`, `signal/read.rs`, `signal/calculated.rs`, `signal/memo.rs`, `observer.rs`

**Actual signals directory** at `/Users/daniel/ak/ankurah/signals/src/`:
```
broadcast.rs, context.rs, jsvalue.rs, lib.rs, observer.rs,
observer/callback_observer.rs, porcelain.rs, porcelain/subscribe.rs,
porcelain/wait.rs, react.rs, react_native.rs, reactive_graph.rs,
signal.rs, signal/calculated.rs, signal/map.rs, signal/memo.rs,
signal/mutable.rs, signal/read.rs, value.rs
```

Missing from the spec's port list:
- `context.rs` -- signal context management
- `reactive_graph.rs` -- reactive dependency graph
- `porcelain.rs`, `porcelain/subscribe.rs`, `porcelain/wait.rs` -- ergonomic wrappers
- `signal/map.rs` -- mapped/derived signals
- `value.rs` -- signal value types
- `react.rs`, `react_native.rs` -- framework integration (possibly relevant for `@ankurah/react-native`)

The spec lists 7 files; the actual codebase has 16+. This is more than double the estimated scope.

---

## 3. GAPS

### 3.1 No Proto Module Inventory

The specs reference `proto/src/{data,message,request,update,clock,id,sys}.rs` (design-resume.md, line 58).

**Actual proto modules** at `/Users/daniel/ak/ankurah/proto/src/lib.rs`:
```
auth, clock, collection, data, error, human_id, id, message, peering, request, subscription, sys, transaction, update
```

Missing from spec awareness:
- `auth.rs` -- `AuthData`, `Attested<T>`, `AttestationSet` (critical for wire protocol)
- `peering.rs` -- `Presence` type (used in `Message` enum)
- `subscription.rs` -- `QueryId` type (used in NodeMessage)
- `transaction.rs` -- `TransactionId` type
- `collection.rs` -- `CollectionId` type definition
- `human_id.rs` -- human-readable ID generation
- `error.rs` -- `DecodeError`, `IdParseError`

The `Attested<T>` wrapper is pervasive in the wire protocol (events and states are wrapped in it), yet the specs treat attestation as de-scoped. The type itself still needs to be ported even if cryptographic verification is de-scoped, because the wire format includes attestation fields.

### 3.2 Missing Core Modules from Structural Mapping

**structural-mapping-analysis.md** claims ~88% mappability but omits several real Rust files:

Files in `/Users/daniel/ak/ankurah/core/src/` not mentioned in any spec:
- `collation.rs` -- collation/ordering logic
- `collectionset.rs` -- collection set management
- `lineage.rs` -- lineage tracking (even if de-scoped, may be needed structurally)
- `policy.rs` -- policy agent (even if using PermissiveAgent, the trait must exist)
- `storage.rs` -- storage trait definitions in core (separate from storage/common)
- `task.rs` -- async task management
- `query_value.rs` -- query value handling
- `traits.rs` -- core trait definitions
- `peer_subscription/` -- client relay and server subscription management
- `reactor/watcherset.rs`, `reactor/property_path.rs`, `reactor/update.rs`, `reactor/candidate_changes.rs`, `reactor/comparison_index.rs`, `reactor/fetch_gap.rs`, `reactor/subscription.rs`, `reactor/subscription_state.rs` -- the reactor is actually 8+ files, not just `reactor.rs`
- `indexing/` -- key_spec.rs, encoding.rs, mod.rs
- `value/collatable.rs`, `value/cast.rs`, `value/cast_predicate.rs` -- value casting and collation
- `util/` -- safeset, safemap, ivec, ready_chunks, iterable, cast, expand_states

The reactor alone is a complex subsystem with 8+ files. The spec lists it as a single `reactor.rs -> reactor.ts` mapping.

### 3.3 Missing Error Handling Strategy

**structural-mapping-analysis.md, lines 269-273:**
> "Rust uses `Result<T, E>` extensively. TS can use either:
> - Exceptions (simpler, more idiomatic TS)
> - `Result<T, E>` via a library like `neverthrow`
> **Recommendation**: Use exceptions for the public API"

This is a one-paragraph mention. No spec addresses:
- What error types exist in Rust (there are separate error types in `core/src/error.rs`, `proto/src/error.rs`, `storage/sqlite/src/error.rs`, and `ankql/src/error.rs`)
- How `MutationError`, `RetrievalError`, `StateError`, `PropertyError` map to TS
- Whether to use typed error classes or generic Error
- How to handle the `anyhow::Result` pattern used extensively in Rust
- Error propagation through async boundaries
- Transaction error rollback behavior
- How to surface errors in React hooks (error boundaries?)

The Rust backend code uses at least 5 distinct error types:
- `MutationError` (transaction/write errors)
- `RetrievalError` (read/fetch errors)
- `StateError` (serialization errors)
- `PropertyError` (property access errors)
- `DecodeError` (ID/encoding errors)

None of these are discussed in any spec.

### 3.4 AnkQL Parser: Pest Grammar Not Analyzed

**initial-porting-workflow.md, line 129:**
> "`grammar.rs` + `ankql.pest` -> `parser.ts`"

The spec mentions the .pest file but no spec examines its contents or discusses the complexity of porting it. The actual grammar exists at `/Users/daniel/ak/ankurah/ankql/src/ankql.pest` but its rules, complexity, and completeness are never analyzed. Additionally, the Rust codebase has a separate `parser.rs` and `selection.rs` and `selection/sql.rs` in the ankql module that the specs don't account for.

### 3.5 Bincode Fixtures Don't Exist Yet

**wire-format-interop.md** extensively describes a fixture-based testing approach reading from `ankurah/proto/tests/fixtures/bincode/`.

**Verification**: No `fixtures/bincode/` directory exists anywhere under `/Users/daniel/ak/ankurah/`. This is acknowledged as future work, but the initial-porting-workflow.md lists it as a *prerequisite* (line 13):
> "Add bincode reference fixtures - `cargo test -p ankurah-proto generate_reference_fixtures`"

This prerequisite does not exist. Phase 1 (proto types) cannot be validated against fixtures that don't exist.

### 3.6 No Discussion of BigInt Support in Hermes

The specs note that `i64` maps to `bigint` in TypeScript. However:
- `DataView.getBigInt64()` and `DataView.getBigUint64()` are used in the bincode reader sketch
- Hermes (React Native's JS engine) historically had incomplete BigInt support
- No spec verifies that Hermes supports BigInt, BigInt64Array, or DataView BigInt methods
- If Hermes doesn't support these, the entire bincode reader breaks for any type containing i64/u64

---

## 4. STALE INFORMATION

### 4.1 Referenced File Paths That Don't Match Reality

**design-resume.md, lines 47-63** lists "Key File Locations in ankurah/":

| Spec Claim | Actual Status |
|---|---|
| `derive/src/model/{view,mutable,model,description,backend_registry}.rs` | Exists, but also has `backend.rs` and `mod.rs` not listed |
| `core/src/property/value/{lww,yrs,entity_ref,json}.rs` | Exists. Spec omits `pn_counter.rs` |
| `core/src/property/value/{lww,yrs}.ron` | Exists at correct path |
| `proto/src/{data,message,request,update,clock,id,sys}.rs` | Exists but spec omits 7 additional modules: `auth`, `peering`, `subscription`, `transaction`, `collection`, `human_id`, `error` |
| `signals/src/{signal,broadcast,observer}.rs` | Exists but spec omits 13+ additional files |
| `ankql/src/{grammar,ast,conversion}.rs` | Exists but spec omits `parser.rs`, `selection.rs`, `selection/sql.rs`, `error.rs`, `lib.rs` |
| `storage/common/src/lib.rs` | Exists but spec implies it's just traits; it has 7 modules |

### 4.2 `domcorder` Reference Still Unresolved

**design-resume.md, lines 22-23 and 126-136:**
> "See `~/code/domcorder` for a reference implementation (agent couldn't access it - needs manual investigation)"
> "Priority 1: Investigate domcorder for bincode patterns"

This is flagged as Priority 1 and marked as unresolved. The `domcorder-patterns.md` spec is listed as "TODO" (line 114). Since the entire bincode strategy depends on patterns from domcorder, this is a foundational gap.

### 4.3 PR #236 Status Unknown

**design-resume.md, line 67:**
> "Most implementation work remains (backend refactoring, registration flow, derive macro updates)."

**schema-registry-and-codegen.md, lines 28-33** lists incomplete items. No spec verifies the current status of PR #236. If it has been merged or abandoned since the specs were written, the codegen strategy may need revision.

---

## 5. DEPENDENCY RISKS

### 5.1 Yrs 0.24.0 / Yjs Version Compatibility

**yrs-yjs-interop-validation.md, line 39:**
> "As of Yrs 0.24.x and Yjs 13.x: V1 encoding: Compatible"

The Rust crate uses `yrs = "0.24.0"` (confirmed in Cargo.toml). But the specs assume V1 compatibility when the code uses V2. V2 compatibility between Yrs 0.24.x and Yjs 13.x is a different (and less well-documented) question.

**Specific risk**: Yjs's V2 encoding support (`Y.encodeStateAsUpdateV2`, `Y.applyUpdateV2`) was added later and may have subtle differences from Yrs 0.24.0's V2 implementation. The yjs changelog should be checked for V2-related fixes.

### 5.2 expo-sqlite Version

**ecosystem-research.md, line 5:**
> "expo-sqlite (v16.x) is bundled in Expo Go."

The current Expo SDK version should be verified. Expo SDK versions advance rapidly, and the expo-sqlite API has changed across versions. The sync vs async API availability and the tagged template feature were added in specific versions.

### 5.3 Hermes Engine Capabilities

**ecosystem-research.md, lines 64-68:**
> "No WebAssembly - Hermes engine lacks `WebAssembly` global"

Several unstated Hermes capabilities matter:
- `BigInt` support (needed for i64/u64 bincode encoding)
- `WeakRef` support (needed for signal system, per initial-porting-workflow.md line 115)
- `Symbol.dispose` support (needed for MutableHandle, per structural-mapping-analysis.md line 251)
- `DataView.getBigInt64`/`getBigUint64` support (needed for bincode reader)
- `TextEncoder`/`TextDecoder` support (needed for bincode string encoding)
- `crypto.getRandomValues` (needed for Yjs client ID generation)

None of these are verified against a specific Hermes version.

### 5.4 ULID Library

**architecture.md, line 70:**
> `EntityId (branded string, ULID) | Use ulid package`

The Rust code stores EntityId as `Ulid` (16 bytes). The spec suggests using a "branded string" in TS. But for bincode interop, the TS EntityId must be serializable as exactly 16 bytes (verified by the test at `proto/src/id.rs` line 178: `assert_eq!(bytes, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])`). A "branded string" approach would need a clear path to 16-byte binary representation. The `ulid` npm package produces strings, not byte arrays. This needs a library that can do ULID-to-bytes conversion.

---

## 6. PHASE ORDERING ISSUES

### 6.1 Phase 1 (Proto) Depends on Missing Fixtures

**initial-porting-workflow.md, lines 93-96:**
> "Validation: ... Bincode round-trip tests pass against Rust-generated fixtures"

But the fixtures don't exist (see Section 3.5). Phase 1 validation cannot complete without them. The prerequisite section (lines 9-15) lists fixture generation as a Rust-side prerequisite, but there's no guidance on what to do if Rust-side work isn't done first.

### 6.2 Phase 7 (Transaction/Context/Node) Depends on Later Phases

Phase 7 ports Node, which depends on:
- Storage engines (Phase 9) -- Node needs a StorageEngine
- Connectors (Phase 10) -- Node uses PeerSender
- Reactor (Phase 8) -- Node owns a Reactor

The workflow says Phase 7 before Phases 8, 9, 10. This means Node must be implemented against stubs/mocks. This is valid but the spec doesn't mention this strategy or provide guidance on what to stub.

### 6.3 Phase 8 (Reactor) Is Vastly Underestimated

The spec lists 7 files for Phase 8. The actual reactor implementation spans:
- `reactor.rs` (main module)
- `reactor/watcherset.rs`
- `reactor/property_path.rs`
- `reactor/update.rs`
- `reactor/candidate_changes.rs`
- `reactor/comparison_index.rs`
- `reactor/fetch_gap.rs`
- `reactor/subscription.rs`
- `reactor/subscription_state.rs`

Plus the selection/filter.rs, retrieval.rs, node_applier.rs files that the spec lists. The reactor is the most complex subsystem and the phase estimate does not reflect this.

### 6.4 Phase 11 (CLI Codegen) Depends on PR #236

Phase 11 reads `schema.json` output from the schema registry. But the prerequisite (PR #236 completion + schema export CLI) is listed as a Rust-side task that "remains" to be completed. If PR #236 is not merged, Phase 11 is blocked. No fallback is described.

### 6.5 Correct Dependency: Phase 5 (Core Values) After Phase 2 (Signals)

Phase 5 ports property backends (LWW, Yjs). The Rust code at `/Users/daniel/ak/ankurah/core/src/property/backend/mod.rs` line 53 shows:
```rust
fn listen_field(&self, field_name: &PropertyName, listener: ankurah_signals::signal::Listener) -> ankurah_signals::signal::ListenerGuard;
```

And `/Users/daniel/ak/ankurah/core/src/property/backend/yrs.rs` line 24:
```rust
field_broadcasts: Mutex<BTreeMap<PropertyName, ankurah_signals::broadcast::Broadcast>>,
```

Property backends depend on `ankurah_signals`. Phase 2 (Signals) correctly precedes Phase 5, so this specific ordering is sound.

---

## 7. MISSING ERROR HANDLING PATTERNS

### 7.1 Rust Error Types Not Cataloged

The Rust codebase defines these error types (found via grep):

- `/Users/daniel/ak/ankurah/core/src/error.rs`: `MutationError`, `RetrievalError`, `StateError`, `PropertyError` (and likely more)
- `/Users/daniel/ak/ankurah/proto/src/error.rs`: `DecodeError`, `IdParseError`
- `/Users/daniel/ak/ankurah/ankql/src/error.rs`: Parser error types
- `/Users/daniel/ak/ankurah/storage/sqlite/src/error.rs`: SQLite-specific errors

No spec catalogs these or defines a TS error mapping strategy.

### 7.2 `Result<T, E>` Return Types Silently Dropped

Key Rust methods that return `Result`:
- `PropertyBackend::to_state_buffer() -> Result<Vec<u8>, StateError>`
- `PropertyBackend::from_state_buffer() -> Result<Self, RetrievalError>`
- `PropertyBackend::to_operations() -> Result<Option<Vec<Operation>>, MutationError>`
- `PropertyBackend::apply_operations() -> Result<(), MutationError>`
- `Context::get() -> Result<R>`
- `Transaction::commit() -> Result<()>`
- `YrsString::insert() -> Result<(), MutationError>`

The structural-mapping-analysis.md PropertyBackend TS interface sketch (lines 106-115) silently drops all error returns:
```typescript
toStateBuffer(): Uint8Array;  // Rust returns Result<Vec<u8>, StateError>
toOperations(): Operation[] | undefined;  // Rust returns Result<Option<Vec<Operation>>, MutationError>
applyOperations(ops: Operation[]): void;  // Rust returns Result<(), MutationError>
```

No spec discusses whether these become:
- Thrown exceptions (losing type information)
- neverthrow `Result<T, E>` (preserving error types)
- Callback-style `(error, result)` patterns
- A mix of approaches

### 7.3 `anyhow::Result` vs Typed Errors

The Rust code uses both `anyhow::Result` (erased error types) and specific typed errors. The TS port needs to decide: when Rust uses `anyhow::Result`, do we use generic `Error`, or do we reconstruct specific error types?

### 7.4 Transaction Rollback on Error

No spec describes what happens when:
- A transaction commit fails partway through
- An operation within a transaction throws
- The network disconnects during commit
- Storage write fails after operations are collected

The Rust code likely handles these via `Result` propagation and `Drop` semantics. TS has no `Drop`, so explicit cleanup must be designed.

---

## 8. ADDITIONAL FINDINGS

### 8.1 `OperationSet` vs `Vec<Operation>`

Multiple specs describe operations as `Vec<Operation>` but the actual Rust type is `OperationSet(BTreeMap<String, Vec<Operation>>)` -- a map from backend name to operations list. This is a different shape that affects:
- Bincode encoding (map with string keys, not flat vec)
- Event construction
- Operation routing to backends

### 8.2 `Attested<T>` Cannot Be Fully De-Scoped

Even though cryptographic attestation verification is de-scoped, the `Attested<T>` wrapper type appears in:
- `EventFragment.attestations: AttestationSet`
- `StateFragment.attestations: AttestationSet`
- `NodeResponseBody::Get(Vec<Attested<EntityState>>)`
- `NodeResponseBody::GetEvents(Vec<Attested<Event>>)`
- `NodeRequestBody::CommitTransaction { events: Vec<Attested<Event>> }`

The wire protocol requires `Attested<T>` and `AttestationSet` to be serializable/deserializable even if attestation logic is stubbed. No spec addresses this.

### 8.3 `DeltaContent` and `EntityDelta` Are Missing from All Specs

The wire protocol includes `DeltaContent` (enum with `StateSnapshot`, `EventBridge`, `StateAndRelation` variants) and `EntityDelta` used in `NodeResponseBody::Fetch` and `NodeResponseBody::QuerySubscribed`. These types appear in `/Users/daniel/ak/ankurah/proto/src/request.rs` but are not mentioned in any spec.

### 8.4 `CausalRelation` and Lineage Types Must Be Stub-Ported

Even though lineage is de-scoped, `CausalRelation`, `CausalAssertion`, `CausalAssertionFragment`, and `KnownEntity` are wire protocol types that appear in request/response bodies. They must be at least stub-ported for bincode compatibility.

### 8.5 The Quantitative "88%" Claim Is Inflated

**structural-mapping-analysis.md, line 301:**
> "~88% of files are directly or near-directly mappable"

This count includes only ~80 files. The actual Rust codebase has significantly more files:
- Proto: 17 files (spec counted ~5)
- Core: 65+ files (spec counted ~30)
- Signals: 19 files (spec counted ~6)
- AnkQL: 8 files (spec counted ~5)
- Storage common: 7 files (spec counted ~2)

The actual total is closer to 120+ files. The mapping analysis covered roughly 60-70% of the actual codebase, so the "88% mappable" number applies to a subset, not the whole.

### 8.6 `reactor.rs` Is a Module Root with 8+ Sub-Files

**structural-mapping-analysis.md** lists `core/src/reactor.rs -> packages/core/src/reactor.ts` as "Direct."

In reality, the reactor is both `core/src/reactor.rs` (module root) AND a `core/src/reactor/` directory with 8 sub-files: `watcherset.rs`, `property_path.rs`, `update.rs`, `candidate_changes.rs`, `comparison_index.rs`, `fetch_gap.rs`, `subscription.rs`, `subscription_state.rs`. The TS equivalent would need a `reactor/` directory with multiple files, not a single `reactor.ts`.

### 8.7 `Event.operations` Is `OperationSet`, Not `Vec<Operation>`

**design-resume.md, line 116:**
> `EventId is a SHA256 hash of bincode::serialize(entity_id) || bincode::serialize(operations) || bincode::serialize(parent)`

This is correct but the spec elsewhere describes operations as `Vec<Operation>`. The actual `Event` struct at `/Users/daniel/ak/ankurah/proto/src/data.rs` line 106:
```rust
pub struct Event {
    pub collection: CollectionId,
    pub entity_id: EntityId,
    pub operations: OperationSet,
    pub parent: Clock,
}
```

The `operations` field is `OperationSet`, not `Vec<Operation>`. The EventId hash computation must serialize the `OperationSet` (a BTreeMap wrapper), not a flat Vec.

---

## 9. SUMMARY OF CRITICAL ACTION ITEMS

| Priority | Issue | Impact | Section |
|----------|-------|--------|---------|
| **P0** | Yrs V2, not V1 | Data corruption if wrong encoding used | 1.1 |
| **P0** | Wire protocol types are wrong (Message/NodeMessage structure) | Cannot implement interop | 1.2 |
| **P0** | Operation struct shape is wrong (diff not data, OperationSet not Vec) | Bincode encoding fails | 1.3 |
| **P1** | Wire format decision (bincode vs JSON) contradicted across specs | Blocks Phase 1 | 1.5 |
| **P1** | Bincode fixtures don't exist yet | Blocks validation | 3.5 |
| **P1** | Missing proto module inventory (7 modules unaccounted) | Incomplete port | 3.1 |
| **P1** | Storage common underestimated (7 modules, not 1) | Schedule risk | 2.5 |
| **P1** | Reactor complexity underestimated (8+ files, not 1) | Schedule risk | 6.3 |
| **P1** | Signals scope underestimated (16+ files vs 7 listed) | Schedule risk | 2.6 |
| **P2** | Error handling strategy undefined | API design risk | 7.* |
| **P2** | Hermes BigInt/WeakRef/Symbol.dispose unverified | Runtime risk | 5.3 |
| **P2** | Attested<T> must be ported despite attestation de-scope | Wire compat risk | 8.2 |
| **P2** | DeltaContent/EntityDelta/CausalRelation missing from specs | Wire compat risk | 8.3, 8.4 |
| **P2** | "88%" mappability claim inflated (based on subset) | Planning risk | 8.5 |
| **P2** | EntityId/EventId dual-encoding (JSON vs bincode) not documented | Encoding risk | 2.2 |
| **P3** | domcorder reference still unresolved | Bincode impl risk | 4.2 |
| **P3** | PR #236 status unknown | Codegen blocked | 4.3 |
| **P3** | CLI codegen scope contradicted (in-scope vs later) | Planning ambiguity | 1.4 |
