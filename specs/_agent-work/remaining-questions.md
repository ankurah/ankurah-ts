# Remaining Open Questions for ankurah-ts

Compiled from all agent findings. Items already decided are excluded.

---

## 1. Bincode Implementation

### 1.1 ULID serialization for TransactionId/RequestId/QueryId/UpdateId
These types wrap `Ulid` using derived (not custom) serde. Ulid's default serde serializes as a **26-character string** (not raw 16 bytes like EntityId). This means each of these IDs is 34 bytes on the wire (u64 length prefix + 26 ASCII bytes). Do we match this exactly, or is there any plan to change the Rust side to use raw bytes like EntityId?

### 1.2 BTreeMap key sort order: byte-order vs locale-order
Bincode serializes `BTreeMap<String, V>` in Rust's `Ord` order for `String`, which is byte-lexicographic (UTF-8 byte order). The TS writer must use the same ordering. Should we use `Array.sort()` (which uses UTF-16 code unit order) or explicitly sort by UTF-8 byte comparison? For ASCII-only keys these are identical, but for non-ASCII keys they could diverge. Are non-ASCII backend names or property names possible in practice?

### 1.3 Bincode u64 length fields: BigInt vs Number in the codec API
The domcorder analysis recommends using `Number` for length fields and `BigInt` only for actual i64/u64 data values. Should the public codec API expose length fields as `number` (with a bounds check) and data fields as `bigint`? Or should we use `bigint` uniformly for simplicity?

### 1.4 `Literal::EntityId(Ulid)` vs `Value::EntityId(proto::EntityId)` -- same TS type?
These two serialize completely differently on the wire (26-char string vs 16 raw bytes). Should they share a TS `EntityId` type with different codec paths, or should they be distinct types (e.g. `AstEntityId` vs `EntityId`)?

---

## 2. Fixture Generation and Validation

### 2.1 Who generates the Rust-side bincode fixtures?
The test fixtures at `proto/test/fixtures/` do not exist yet. Is there an expected timeline for generating them from the Rust side? Does the TS port block on these, or should we write a Rust test to generate them as part of this project?

### 2.2 Yrs interop fixtures
Same question for Yrs/Yjs V2 interop fixtures. The validation plan requires Rust-generated `.bin` files with V2-encoded Yrs state. Should this be added to the ankurah Rust test suite, or maintained separately?

---

## 3. AnkQL Parser

### 3.1 Parser implementation strategy
The Rust AnkQL parser uses a Pest PEG grammar (`ankql.pest`). TS options include:
- Hand-written recursive descent parser
- PEG.js / Peggy (PEG parser generator for JS)
- Chevrotain (parser toolkit)
- Ohm (PEG-based)

Which approach? Hand-written is most portable and avoids runtime dependencies. Peggy generates compact parsers. This affects bundle size and maintainability.

### 3.2 Parser scope: parse-only or parse+evaluate?
The Rust ankql crate includes not just parsing but also `selection.rs`, `selection/sql.rs`, predicate evaluation, and SQL generation. How much of that is needed in the TS client? If the TS client only sends parsed ASTs to the server, it needs parse only. If it evaluates predicates locally (for optimistic filtering), it needs evaluation too.

---

## 4. Entity/Transaction Lifecycle in TS

### 4.1 Entity identity: pointer equality or value equality?
Rust uses `Arc::ptr_eq` for Entity equality. TS has no pointer identity for plain objects. Options:
- Use a `Map<EntityId, Entity>` as an identity cache (similar to WeakEntitySet)
- Use branded types with an internal `Symbol` for identity
- Just use `EntityId` equality everywhere

Which approach best matches the Rust semantics without overengineering?

### 4.2 Transaction `alive` flag and Drop semantics
Rust `Transaction` uses `Drop` to mark `alive = false` (auto-rollback). TS has no `Drop`. Options:
- `using` keyword with `Symbol.dispose` (TC39 stage 3, not available everywhere)
- Explicit `try/finally` pattern in user code
- `transaction.dispose()` method with a linter rule
- `AbortController`-style pattern

The specs mention `Symbol.dispose` but Hermes support is unverified. What is the fallback if `Symbol.dispose` is not available?

### 4.3 AppendOnlyVec equivalent
Rust Transaction uses `AppendOnlyVec` (lock-free append-only collection). TS is single-threaded, so a plain array suffices functionally. But should the TS `Transaction` API expose this as an immutable view to prevent accidental mutation?

### 4.4 WeakRef for WeakEntitySet
The Rust `WeakEntitySet` uses `Weak<EntityInner>`. TS `WeakRef` is available in modern engines but the spec-cross-check flags it as unverified on Hermes. Has `WeakRef` availability on Hermes been confirmed? If not, what is the fallback (manual cleanup? `FinalizationRegistry`?)?

---

## 5. Signals System

### 5.1 Which signal types are needed for Phase 1?
The Rust signals crate has 16+ files including `Mut`, `Read`, `Memo`, `Map`, `Calculated`, `context.rs`, `reactive_graph.rs`, etc. The spec lists 7. Which signal types are actually needed by the core/property layer for Phase 1? Can we start with just `Broadcast`, `Signal` trait, `Subscribe`, `ListenerGuard` and defer `Memo`, `Map`, `Calculated`, `reactive_graph`?

### 5.2 Signal context and automatic dependency tracking
The Rust code has `context.rs` and `reactive_graph.rs` for automatic dependency tracking (reading a signal inside a computed context auto-subscribes). Is this needed for the TS port, or can we rely on explicit subscriptions and React's own reactivity model?

### 5.3 `BroadcastId` derivation
Rust derives `BroadcastId` from `Arc::as_ptr()` cast to `usize`. TS has no stable pointer identity. Options:
- Auto-incrementing integer counter
- `Symbol()` for uniqueness
- Skip BroadcastId entirely if its only use is deduplication in the reactive graph

What is BroadcastId actually used for, and what is the simplest TS equivalent?

---

## 6. Wire Protocol Types

### 6.1 Stub types for de-scoped features
Several wire protocol types must exist for bincode compat even though their features are de-scoped:
- `Attested<T>`, `AttestationSet`, `AuthData` (attestation de-scoped)
- `CausalRelation`, `CausalAssertion`, `CausalAssertionFragment` (lineage de-scoped)
- `DeltaContent::StateAndRelation` variant (lineage de-scoped)

Should these be full types with empty/default attestation values, or minimal stubs that only handle serialization? For `Attested<T>`, should `commit` always wrap events with an empty `AttestationSet`?

### 6.2 `sys::Item` and `#[serde(other)]`
The `sys::Item::Other` variant uses `#[serde(other)]` which catches unknown variant indices during deserialization. Should the TS decoder replicate this (return `{ type: 'Other' }` for unknown variants) or throw on unknown variants?

### 6.3 `Presence` message and system_root
The `Presence` type includes `system_root: Option<Attested<EntityState>>`. For ephemeral TS nodes, is `system_root` always `None`? If so, can we hardcode this for Phase 1?

---

## 7. Storage Layer

### 7.1 Storage interface: sync or async?
The specs show both patterns. `better-sqlite3` is sync, `expo-sqlite` offers both. Should the `StorageEngine` interface be:
- Fully async (simpler conceptually, works everywhere)
- Sync-first with async wrappers (matches Rust more closely, better perf on mobile)
- Dual interface (more code, more flexibility)

### 7.2 Memory storage engine for testing
Is an in-memory `StorageEngine` (no SQLite dependency) needed for unit tests? If so, should it be a separate package or built into `@ankurah/core`?

### 7.3 Storage common scope
The Rust `storage/common` crate has 7 modules (query planner, filtering, sorting, predicate evaluation, bounds, traits, types). How much of this is needed for the SQLite storage engine vs the server? If the TS client delegates query planning to the server, can we skip the planner and just implement traits + types?

---

## 8. Reactor Complexity

### 8.1 Reactor scope for TS client
The Rust reactor is 8+ files with index watchers, wildcard watchers, comparison indexes, subscription state management, etc. How much of this is needed on the TS client side vs being server-only? If the TS client only processes subscription updates from the server (not local query evaluation), can the reactor be significantly simplified?

### 8.2 WatcherSet index types
The reactor supports index watchers (field-value specific), wildcard watchers (collection-wide), and entity watchers (by ID). Are all three needed in the TS client, or can we start with entity watchers only?

---

## 9. React Native Integration

### 9.1 React hook API design
The Rust crate has `react.rs` and `react_native.rs` in the signals module. What should the React hook API look like? Options:
- `useQuery<V>(predicate)` returning `{ items: V[], loading, error }`
- `useEntity<V>(id)` returning `V | null`
- `useMutation()` returning a transaction factory
- Signal-based: `useSignal(signal)` that re-renders on signal changes

Should hooks subscribe at the field level (fine-grained re-renders) or entity level?

### 9.2 WebSocket reconnection strategy
The connector needs auto-reconnection with exponential backoff. Should this be built into `@ankurah/connector-websocket` or left to the app developer? What about offline queue (operations made while disconnected)?

---

## 10. Yjs Implementation Details

### 10.1 Empty V2 update detection
Rust uses `diff == Update::EMPTY_V2` (constant `[0, 0, 0, 0]`). Does Yjs produce the same 4-byte sentinel for empty V2 diffs? This needs prototyping/validation before committing to the `isEmptyV2Update` check.

### 10.2 `getText()` on nonexistent key: Yrs vs Yjs behavior
Yrs `txn.get_text("nonexistent")` returns `None`. Yjs `doc.getText("nonexistent")` auto-creates and returns a `Y.Text`. The Rust code distinguishes these cases in `get_property_string()`. How should the TS YjsBackend handle this? Track initialized properties explicitly, or check `text.toString().length === 0`?

---

## 11. Project Scaffolding

### 11.1 Monorepo manager: pnpm vs bun
The decision to explore bun is noted as "research pending." Does this block scaffolding, or should we start with pnpm + turborepo (the well-trodden path) and migrate later if bun proves viable?

### 11.2 TypeScript build: tsc, tsup, or unbuild?
Each package needs a build step. Options:
- `tsc` only (simplest, no bundling)
- `tsup` (esbuild-based, bundles for multiple formats)
- `unbuild` (rollup-based, used by nuxt ecosystem)

React Native typically consumes TS source directly via Metro. Node.js consumers need compiled JS. What is the build target?

### 11.3 Test framework
Vitest is the modern default. Jest is the React Native default. Bun has its own test runner. Which test framework should the monorepo use?

---

## 12. Spec Cleanup

### 12.1 Which existing spec files should be deleted vs corrected?
The spec-cross-check found major errors in several spec files (wrong V1/V2, wrong wire types, wrong Operation shape). Since proto structs are authoritative, should the incorrect specs be deleted entirely, or corrected in place with a note? Specifically:
- `yrs-yjs-interop-validation.md` (V1 claims are wrong)
- `wire-format-interop.md` (Operation struct shape is wrong)
- `architecture.md` (NodeMessageBody type doesn't exist)
- `continue-implementation.md` (open questions that are now answered)

### 12.2 Should stale spec content be archived or removed?
Some specs contain useful context alongside incorrect details. Should incorrect sections be struck through, or should we maintain a clean set of specs with only correct information?
