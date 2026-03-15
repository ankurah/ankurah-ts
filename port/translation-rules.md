# Port Rules: ankurah (Rust) -> ankurah-ts (TypeScript)

**Last updated**: 2026-02-10
**Authoritative for**: All file-level, naming, structural, and annotation conventions in the TS port.
**Mandate**: Zero freestyling. The TS port must mirror the Rust structure 1:1. Every exception requires an explicit rule citation.

---

## A. General Rules

These rules apply universally to every file in ankurah-ts.

### A1. File Naming

- Rust `foo_bar.rs` maps to TS `foo_bar.ts` (preserve snake_case filenames for 1:1 mapping).
- Rust `mod.rs` maps to TS `index.ts` (see Exception E2).
- Rust `lib.rs` maps to TS `index.ts` (package entry point; see Exception E2).
- Rust `foo_bar.pest` (PEG grammar) is used as a reference for the hand-written recursive descent parser in TS (see Exception E6).

### A2. Identifier Naming

- Functions and variables: Rust `snake_case` maps to TS `camelCase`.
- Types, classes, interfaces, enums: `PascalCase` in both languages (no change).
- Constants: `UPPER_SNAKE_CASE` in both languages (no change).
- Module names (directory names): preserve `snake_case` to match Rust.

### A3. Line 1 Annotation (Mandatory)

Every `.ts` source file MUST have one of the following on line 1:

```typescript
// MIRRORS: ankurah/<crate>/src/<path>.rs
```
For files that have a 1:1 Rust counterpart.

```typescript
// TS-ONLY: <reason>
```
For files that have NO Rust counterpart (e.g., React Native hooks, polyfill boot, TS-specific utilities).

### A4. Module Re-exports

- Rust `pub mod foo;` in `mod.rs` or `lib.rs` maps to TS `export * from './foo';` or `export { Specific } from './foo';` in `index.ts`.
- Rust `mod foo;` (private module) maps to TS internal import only (not re-exported from `index.ts`).
- Rust `pub use foo::*;` maps to TS `export * from './foo';`.
- Rust `pub use foo::Bar;` maps to TS `export { Bar } from './foo';`.

### A5. Import Path Mapping

- Rust `use crate::` maps to TS relative imports within the same package.
  - Example: `use crate::entity::Entity;` in `core/src/node.rs` becomes `import { Entity } from './entity';` in `packages/core/src/node.ts`.
- Cross-crate imports map to TS package imports:
  - `use ankurah_proto::` maps to `import from '@ankurah/proto'`
  - `use ankurah_core::` maps to `import from '@ankurah/core'`
  - `use ankurah_signals::` maps to `import from '@ankurah/signals'`
  - `use ankql::` maps to `import from '@ankurah/ankql'`
  - `use ankurah_storage_common::` maps to `import from '@ankurah/storage-common'`
- See Section F for the complete import mapping table.

### A6. Type System Mapping

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `struct Foo { ... }` | `class Foo extends Struct { ... }` | All structs extend `Struct` from `@ankurah/base` |
| `enum Foo { A, B(T) }` | `class Foo extends Enum<FooV> { ... }` | See Enum pattern below |
| `impl Drop for T` | `class T extends Drop` | Override `drop()` for custom cleanup. Call `super.drop()`. |
| `trait Foo { ... }` | `interface Foo { ... }` | Unless it has default impls (see E7) |
| `trait Foo` with default impls | `abstract class Foo` or mixin | See Exception E7 |
| `impl Trait for Struct` | `class Struct implements Interface` | See Exception E4 |
| `impl Struct { ... }` | Methods inside `class Struct { ... }` | See Exception E4 |
| Generics `<T: Trait>` | Generics `<T extends Interface>` | Direct mapping |
| `Result<T, E>` | Throw with typed `Error` subclass | Per decisions.md |
| `Option<T>` | `T \| null` | |
| `Arc<T>` | `Arc<T>` | From `@ankurah/base`. Refcounted shared ownership. |
| `Rc<T>` | `Arc<T>` | Same as Arc (no threading distinction in JS) |
| `Weak<T>` | `Weak<T>` | From `@ankurah/base`. `upgrade()` returns `Arc<T> \| null`. |
| `&T` (in struct fields) | `Borrow<T>` | From `@ankurah/base`. Non-owning, no-op dispose. |
| `&mut T` (in struct fields) | `BorrowMut<T>` | From `@ankurah/base`. Non-owning mutable, no-op dispose. |
| `Mutex<T>` | `Mutex<T>` | From `@ankurah/base`. `using guard = mutex.lock()`. |
| `RwLock<T>` | `Mutex<T>` | Same as Mutex (no reader/writer distinction in JS) |
| `tokio::sync::Mutex` | `AsyncMutex` | From `@ankurah/base`. Async serialization. |
| `RefCell<T>` | `RefCell<T>` | From `@ankurah/base`. `borrow()` / `borrow_mut()`. |
| `Box<dyn Trait>` | Interface type directly | No boxing needed in TS |
| `AtomicBool` | `boolean` | Single-threaded JS |
| `AtomicU32` / `AtomicUsize` | `number` | Single-threaded JS |
| `Vec<T>` | `T[]` or `Array<T>` | |
| `HashMap<K,V>` / `BTreeMap<K,V>` | `Map<K,V>` | |
| `HashSet<T>` / `BTreeSet<T>` | `Set<T>` | |
| `String` / `&str` | `string` | |
| `Vec<u8>` / `&[u8]` | `Uint8Array` | |
| `bool` | `boolean` | |
| `i16` / `i32` | `number` | |
| `i64` / `u64` | `bigint` or `number` | Context-dependent; see decisions.md |
| `f64` | `number` | |
| `usize` | `number` | |

### A6a. Enum Pattern

Rust enums map to `class Foo extends Enum<V>` where `V` is a variant type map. Unit variants use `{}`. Static methods construct each variant.

```rust
// Rust
enum DeltaContent {
    StateSnapshot { state: StateFragment },
    EventBridge { events: Vec<EventFragment> },
}
```

```typescript
// TS
type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: EventFragment[] };
};
class DeltaContent extends Enum<DeltaContentV> {
  // impl methods go here
}

// Construction — mirrors Rust DeltaContent::StateSnapshot { state }
new DeltaContent('StateSnapshot', { state })
new DeltaContent('EventBridge', { events })
```

`match` statements become `.match({})`:
```rust
match content {
    DeltaContent::StateSnapshot { state } => handle_state(state),
    DeltaContent::EventBridge { events } => handle_events(events),
}
```
```typescript
content.match({
  StateSnapshot: (v) => handleState(v.state),
  EventBridge: (v) => handleEvents(v.events),
});
```

`if let` becomes `.is()`:
```rust
if let DeltaContent::StateSnapshot { state } = content { ... }
```
```typescript
if (content.is('StateSnapshot')) { const state = content.value.state; ... }
```

### A7. Derive Macro Mapping

| Rust Derive | TypeScript Equivalent | Notes |
|------------|----------------------|-------|
| `#[derive(Clone)]` | Implement `clone()` method | For `Arc<T>`, use `arc.clone()`. For structs, implement field-by-field clone if needed. |
| `#[derive(Debug)]` | `toString()` method | Add if useful for debugging |
| `#[derive(PartialEq, Eq)]` | Custom `equals()` method | If equality comparison is needed |
| `#[derive(Serialize, Deserialize)]` | Bincode codec functions | Per wire-format-interop.md |
| `#[derive(Model)]` | Hand-written wrappers (Phase 1) | See Exception E1 |
| `#[derive(Hash)]` | No direct equivalent | Use string keys for maps if needed |

### A8. Error Handling

- Rust `Result<T, E>` maps to TS functions that throw typed `Error` subclasses.
- Mirror Rust error types 1:1 as TS Error subclasses: `MutationError`, `RetrievalError`, `StateError`, `PropertyError`, `DecodeError`, `SubscriptionError`.
- Rust `anyhow::Error` maps to plain `Error` or a generic `AnkurahError`.
- Rust `.unwrap()` / `.expect()` maps to TS assertions or direct access (errors propagate via throw).
- Rust `?` operator maps to normal TS try/catch flow (errors propagate via throw).

### A9. Async Mapping

- Rust `async fn foo() -> Result<T, E>` maps to TS `async foo(): Promise<T>` (throws on error).
- Rust `tokio::spawn()` maps to TS `Promise` / microtask / `setTimeout` as appropriate.
- Rust channels (`mpsc`, `oneshot`) map to TS event emitters, callbacks, or custom async patterns.

### A10. Test File Mapping

- Rust `#[cfg(test)] mod tests { ... }` inline in `foo.rs` maps to TS `foo.test.ts` adjacent to `foo.ts` (see Exception E3).
- Rust integration tests in `tests/` directory map to TS `__tests__/` directory or adjacent `.test.ts` files in the package.
- Rust `#[test]` / `#[tokio::test]` maps to TS `test('...', async () => { ... })`.
- Rust `assert_eq!(a, b)` maps to TS `expect(a).toBe(b)` or `expect(a).toEqual(b)`.

### A11. Visibility Mapping

- Rust `pub` maps to TS `export`.
- Rust `pub(crate)` maps to TS non-exported (internal to package).
- Rust private (no modifier) maps to TS `private` class member or non-exported module-level.

### A12. Feature Flag Mapping

- Rust `#[cfg(feature = "wasm")]` code is OUT OF SCOPE for the TS port (TS IS the non-WASM target).
- Rust `#[cfg(feature = "uniffi")]` code is OUT OF SCOPE (UniFFI is for Rust FFI, not needed in pure TS).
- Rust `#[cfg(test)]` maps to separate `.test.ts` files (see Exception E3).
- Rust `#[cfg(feature = "postgres")]` maps to OUT OF SCOPE for Phase 1.

---

## B. Crate-to-Package Mapping

### B1. Complete Mapping Table

| Rust Crate (Cargo.toml `name`) | Rust Path | TS Package | TS Path | Status |
|-------------------------------|-----------|------------|---------|--------|
| `ankurah-proto` | `proto/` | `@ankurah/proto` | `packages/proto/` | IN SCOPE Phase 1 |
| `ankurah-core` | `core/` | `@ankurah/core` | `packages/core/` | IN SCOPE Phase 1 |
| `ankurah-signals` | `signals/` | `@ankurah/signals` | `packages/signals/` | IN SCOPE Phase 1 |
| `ankql` | `ankql/` | `@ankurah/ankql` | `packages/ankql/` | IN SCOPE Phase 1 |
| `ankurah-storage-common` | `storage/common/` | `@ankurah/storage-common` | `packages/storage-common/` | IN SCOPE Phase 1 |
| `ankurah-storage-sqlite` | `storage/sqlite/` | `@ankurah/storage-sqlite` | `packages/storage-sqlite/` | IN SCOPE Phase 1 |
| `ankurah-websocket-client` | `connectors/websocket-client/` | `@ankurah/connector-websocket` | `packages/connector-websocket/` | IN SCOPE Phase 1 |
| `ankurah-connector-local-process` | `connectors/local-process/` | `@ankurah/connector-local` | `packages/connector-local/` | IN SCOPE Phase 1 |
| `ankurah-derive` | `derive/` | NO TS EQUIVALENT | N/A | Exception E1: TS has no proc macros |
| `ankurah` | `ankurah/` | NO DIRECT EQUIVALENT | N/A | Facade crate; re-exports handled by each package |
| `ankurah-storage-postgres` | `storage/postgres/` | OUT OF SCOPE | N/A | Phase 1 exclusion |
| `ankurah-storage-sled` | `storage/sled/` | OUT OF SCOPE | N/A | Phase 1 exclusion |
| `ankurah-storage-indexeddb-wasm` | `storage/indexeddb-wasm/` | OUT OF SCOPE | N/A | Phase 1 exclusion (WASM-specific) |
| `ankurah-websocket-server` | `connectors/websocket-server/` | OUT OF SCOPE | N/A | Phase 1 exclusion |
| `ankurah-websocket-client-wasm` | `connectors/websocket-client-wasm/` | OUT OF SCOPE | N/A | Phase 1 exclusion (WASM-specific) |

### B2. Additional TS-Only Packages

| TS Package | TS Path | Purpose | Annotation |
|-----------|---------|---------|------------|
| `@ankurah/react-native` | `packages/react-native/` | React Native hooks and bindings | TS-ONLY: React Native integration |
| `@ankurah/storage-memory` | `packages/storage-memory/` | In-memory storage for testing | TS-ONLY: test utility |

---

## C. File-Level Mapping

### C1. Mapping Principle

The mapping from Rust source files to TS source files is mechanical. Apply these rules in order:

1. **Crate -> Package**: Use the crate-to-package mapping from Section B1 to determine the target package.
2. **Path preservation**: `ankurah/<crate>/src/<path>.rs` maps to `packages/<pkg>/src/<path>.ts`. The directory structure inside `src/` is preserved 1:1.
3. **Entry points**: `lib.rs` and `mod.rs` become `index.ts` (Exception E2).
4. **File-with-submodules**: When Rust has both `foo.rs` and a `foo/` directory, TS uses `foo/index.ts` for the primary code (Exception E12).
5. **Skipped files**: WASM-only modules (E9), feature-gated modules (E10), and Rust-only integrations (E14) produce no TS file.
6. **Filename exceptions**: `yrs.rs` becomes `yjs.ts` (Exception E5).

### C2. Illustrative Examples

**Example: `ankurah-proto` -> `@ankurah/proto`** (flat crate, no subdirectories)

| Rust Source | TS Source |
|---|---|
| `ankurah/proto/src/lib.rs` | `packages/proto/src/index.ts` [E2] |
| `ankurah/proto/src/id.rs` | `packages/proto/src/id.ts` |
| `ankurah/proto/src/message.rs` | `packages/proto/src/message.ts` |
| `ankurah/proto/src/clock.rs` | `packages/proto/src/clock.ts` |
| `ankurah/proto/src/wasm.rs` | NO EQUIVALENT [E9: WASM-only] |

Every other `.rs` file in `proto/src/` follows the same `<name>.rs` -> `<name>.ts` pattern.

**Example: `ankurah-core` nested directory** (`core/src/property/backend/`)

| Rust Source | TS Source |
|---|---|
| `ankurah/core/src/property/backend/mod.rs` | `packages/core/src/property/backend/index.ts` [E2] |
| `ankurah/core/src/property/backend/lww.rs` | `packages/core/src/property/backend/lww.ts` |
| `ankurah/core/src/property/backend/yrs.rs` | `packages/core/src/property/backend/yjs.ts` [E5] |

This pattern applies at any nesting depth: `reactor/`, `selection/`, `util/`, `value/`, etc.

### C3. File Count Summary

| Crate | Rust Files (src/ + tests/) | TS Files (in scope) | Skipped (OOS/feature-gated) |
|-------|---------------------------|--------------------|-----------------------------|
| ankurah-proto | 17 | 15 | 2 (wasm.rs, postgres.rs) |
| ankurah-core | 67 | 63 | 4 (tsify.rs, pn_counter x2, wasm.rs) |
| ankurah-signals | 23 (19 src + 4 tests) | 19 | 4 (reactive_graph, react, react_native, jsvalue) |
| ankql | 9 (8 .rs + 1 .pest) | 9 | 0 |
| ankurah-storage-common | 8 | 8 | 0 |
| ankurah-storage-sqlite | 6 | 6 | 0 |
| ankurah-websocket-client | 3 | 3 | 0 |
| ankurah-connector-local-process | 1 | 1 | 0 |
| **TOTAL** | **134** | **124** | **10** |

Plus TS-only files (React Native hooks, polyfills, in-memory storage, test utilities).

---

## D. Exception Rules

Every case where the TS port diverges from a literal 1:1 file/structure mapping MUST cite one of the following exception rules.

### E1: Derive Macro Crate Has No TS Equivalent

- **Rust pattern**: `ankurah-derive` proc macro crate (23 files) generates View/Mutable/Model implementations at compile time via `#[derive(Model)]`.
- **TS equivalent**: Hand-written model wrappers in Phase 1. Codegen CLI in future phases.
- **Justification**: TypeScript has no compile-time proc macro system. Macro-generated code is written by hand in TS, matching the OUTPUT of the Rust macros, not the macro implementation itself.

### E2: `mod.rs` / `lib.rs` -> `index.ts`

- **Rust pattern**: `mod.rs` is the entry point for directory modules; `lib.rs` is the crate entry point.
- **TS equivalent**: `index.ts` serves both roles in TypeScript.
- **Justification**: Language convention. TypeScript/Node.js uses `index.ts` as the default module resolution target for directories and packages.

### E3: Inline Tests -> Separate `.test.ts` Files

- **Rust pattern**: `#[cfg(test)] mod tests { ... }` inside source files; integration tests in `tests/` directory.
- **TS equivalent**: `foo.test.ts` adjacent to `foo.ts`; integration tests in `__tests__/` directories.
- **Justification**: Language convention. TS test runners (Jest, Vitest) expect separate test files. TS has no conditional compilation to exclude test code from production builds.

### E4: `impl` Blocks -> Class Methods

- **Rust pattern**: `impl Struct { ... }` and `impl Trait for Struct { ... }` as separate blocks outside the struct definition.
- **TS equivalent**: Methods defined inside `class Struct { ... }` body. Interface implementation via `class Struct implements Interface { ... }`.
- **Justification**: Language difference. TypeScript uses classes with inline method definitions, not separate `impl` blocks.

### E5: `yrs` -> `yjs` (Library Rename)

- **Rust pattern**: Files named `yrs.rs` reference the Yrs (Rust) CRDT library.
- **TS equivalent**: Files named `yjs.ts` reference the Yjs (JavaScript) CRDT library.
- **Justification**: The TS port uses Yjs (the original JS library) instead of Yrs (the Rust port of Yjs). The filename reflects the actual library used. Both are wire-compatible via V2 encoding.

### E6: Pest PEG Grammar -> Hand-Written Recursive Descent Parser in TS

- **Rust pattern**: `ankql.pest` PEG grammar parsed by the `pest` crate; `grammar.rs` integrates pest-generated parser.
- **TS equivalent**: `parser.ts` contains a hand-written recursive descent parser. `grammar.ts` contains token/rule definitions used as reference documentation derived from the Pest grammar. There is no `.peggy` file or Peggy dependency.
- **Justification**: Pest is a Rust-specific PEG parser generator. Rather than adopting another parser generator (e.g., Peggy), the TS port uses a hand-written recursive descent parser for full control, easier debugging, and zero external parser dependencies. The Rust `ankql.pest` grammar serves as the authoritative REFERENCE for what the hand-written parser must accept.

### E7: Traits with Default Implementations -> Abstract Classes or Mixins

- **Rust pattern**: `trait Foo { fn bar(&self) { default_impl() } }` provides default method implementations.
- **TS equivalent**: `abstract class Foo { bar(): void { defaultImpl() } }` or mixin pattern.
- **Justification**: TypeScript interfaces cannot have method implementations. When a Rust trait provides default implementations, the TS equivalent must be an abstract class or use a mixin pattern.

### E8: Concurrency Primitives Mapped to @ankurah/base

- **Rust pattern**: `Arc<T>`, `Rc<T>`, `Mutex<T>`, `RwLock<T>`, `RefCell<T>`, `Weak<T>`, `AtomicBool`, `Send + Sync` bounds.
- **TS equivalent**: Provided types from `@ankurah/base` that mirror the Rust API shape. `Arc<T>` provides refcounted shared ownership. `Mutex<T>` provides guard-based access. `AtomicBool`/`AtomicU32` become plain `boolean`/`number`. `Send + Sync` bounds are removed.
- **Justification**: JS is single-threaded, but the ownership semantics (refcounting, Drop cascade, borrow checking) must be preserved for correctness. The provided types absorb the JS complexity so translated code reads like the Rust source.

### E9: WASM-Only Modules Skipped

- **Rust pattern**: Modules gated with `#[cfg(feature = "wasm")]` such as `proto/src/wasm.rs`, `core/src/model/tsify.rs`, `core/src/value/wasm.rs`, `signals/src/react.rs`, `signals/src/jsvalue.rs`.
- **TS equivalent**: No TS file created.
- **Justification**: These modules provide Rust-to-WASM bridging (wasm-bindgen, JsValue conversions). The TS port IS the native JS target, so these bridges are unnecessary.

### E10: Feature-Gated Database Modules Skipped

- **Rust pattern**: `#[cfg(feature = "postgres")]` in `proto/src/postgres.rs`.
- **TS equivalent**: No TS file created.
- **Justification**: PostgreSQL support is out of scope for Phase 1. The feature-gated module provides Postgres-specific type conversions not needed in the TS port.

### E11: Drop Semantics -> @ankurah/base Drop + AkObject

- **Rust pattern**: `impl Drop for T { fn drop(&mut self) { ... } }` for custom cleanup. Automatic drop cascade for all owned fields.
- **TS equivalent**: `class T extends Drop` with `drop()` override for custom cleanup. All structs extend `Struct` (via `AkObject`) which provides automatic drop cascade via `[Symbol.dispose]()`. `using` declarations trigger cleanup at block exit.
- **Justification**: JS has no deterministic destructors. `@ankurah/base` provides `AkObject` (auto-cascade), `Drop` (custom cleanup), `Arc` (refcounted ownership), `Borrow`/`BorrowMut` (non-owning, no cascade), matching Rust's ownership model as closely as possible.

### E12: File-With-Submodules -> Directory with index.ts

- **Rust pattern**: A file like `reactor.rs` that also has a directory `reactor/` containing sub-modules. Rust allows this (the file IS the module, sub-files are sub-modules).
- **TS equivalent**: A directory `reactor/` with `index.ts` containing the primary code and sibling files for sub-modules.
- **Justification**: TypeScript cannot have both a file `reactor.ts` and a directory `reactor/`. The file must become `reactor/index.ts`.

### E13: Rust Macros -> TS Functions or Logger Calls

- **Rust pattern**: `macro_rules! action_info { ... }` and similar logging/formatting macros in `core/src/util/mod.rs`.
- **TS equivalent**: Regular functions or direct logger calls (e.g., `console.log`, structured logging library).
- **Justification**: TypeScript has no compile-time macro system. Rust macros that generate code at compile time must be replaced with regular functions or inlined.

### E14: Rust-Only External Crate Integration Skipped

- **Rust pattern**: `signals/src/reactive_graph.rs` integrates with the `reactive_graph` Rust crate.
- **TS equivalent**: No TS file created.
- **Justification**: The `reactive_graph` crate is a Rust-specific reactive framework. The TS signals implementation is self-contained and does not need this integration.

### E15: Rust FFI Bridge Replaced by Native TS Package

- **Rust pattern**: `signals/src/react_native.rs` provides UniFFI bindings for React Native to call Rust signals.
- **TS equivalent**: `@ankurah/react-native` package provides native TS React hooks. Annotated as `// TS-ONLY: React Native hooks (replaces Rust UniFFI bridge)`.
- **Justification**: In the pure TS port, React Native can use TS signals directly. The Rust-to-RN bridge layer is eliminated entirely.

### E16: SQLite API Adaptation

- **Rust pattern**: `storage/sqlite/src/connection.rs` and `engine.rs` use `rusqlite` with `bb8` connection pooling.
- **TS equivalent**: Same file names, but internals use `expo-sqlite` (mobile) or `better-sqlite3` (Node testing).
- **Justification**: Different SQLite binding libraries per platform. The storage interface (from storage-common) remains identical; only the internal implementation of the engine differs.

### E17: WebSocket API Adaptation

- **Rust pattern**: `connectors/websocket-client/src/client.rs` and `sender.rs` use `tokio-tungstenite`.
- **TS equivalent**: Same file names, but internals use the standard WebSocket API (available in React Native and browsers).
- **Justification**: Different WebSocket libraries. The connector interface (PeerSender) remains identical; only the underlying transport implementation differs.

### E18: Tokio Channels -> TS Async Patterns

- **Rust pattern**: `tokio::sync::mpsc` channels for inter-task communication in `connectors/local-process/src/lib.rs`.
- **TS equivalent**: Event emitters, async generators, or custom message passing (single-threaded, no real channels needed).
- **Justification**: JavaScript is single-threaded. Tokio channels exist for cross-thread communication. TS equivalent is simpler direct function calls or event emitters.

---

## E. Package Directory Layout Template

### E1. `@ankurah/proto` (Example)

```
packages/proto/
  src/
    index.ts            # MIRRORS: ankurah/proto/src/lib.rs
    auth.ts             # MIRRORS: ankurah/proto/src/auth.rs
    clock.ts            # MIRRORS: ankurah/proto/src/clock.rs
    collection.ts       # MIRRORS: ankurah/proto/src/collection.rs
    data.ts             # MIRRORS: ankurah/proto/src/data.rs
    error.ts            # MIRRORS: ankurah/proto/src/error.rs
    human_id.ts         # MIRRORS: ankurah/proto/src/human_id.rs
    id.ts               # MIRRORS: ankurah/proto/src/id.rs
    message.ts          # MIRRORS: ankurah/proto/src/message.rs
    peering.ts          # MIRRORS: ankurah/proto/src/peering.rs
    request.ts          # MIRRORS: ankurah/proto/src/request.rs
    subscription.ts     # MIRRORS: ankurah/proto/src/subscription.rs
    sys.ts              # MIRRORS: ankurah/proto/src/sys.rs
    transaction.ts      # MIRRORS: ankurah/proto/src/transaction.rs
    update.ts           # MIRRORS: ankurah/proto/src/update.rs
    codec.ts            # TS-ONLY: bincode reader/writer (per domcorder patterns)
  __tests__/
    codec.test.ts       # TS-ONLY: bincode round-trip tests
    fixtures.test.ts    # TS-ONLY: cross-language fixture validation
  package.json
  tsconfig.json
```

**Other packages follow the same pattern.** Derive the layout by applying rules A1, A3, E2, E12 to the corresponding Rust crate structure. Use the crate-to-package mapping in Section B1 to find the target directory, then mirror the `src/` tree 1:1, skipping files covered by E9/E10/E14.

---

## F. Import Mapping Rules

### F1. Cross-Crate Import Table

| Rust `use` Statement | TS `import` Statement |
|---------------------|----------------------|
| `use ankurah_proto::*;` | `import { ... } from '@ankurah/proto';` |
| `use ankurah_proto as proto;` | `import * as proto from '@ankurah/proto';` |
| `use ankurah_core::entity::Entity;` | `import { Entity } from '@ankurah/core';` (if re-exported) or `import { Entity } from '@ankurah/core/entity';` |
| `use ankurah_signals::*;` | `import { ... } from '@ankurah/signals';` |
| `use ankql::ast::Selection;` | `import { Selection } from '@ankurah/ankql';` (if re-exported) |
| `use ankurah_storage_common::*;` | `import { ... } from '@ankurah/storage-common';` |

### F2. Intra-Package Import Rules

| Rust `use` Pattern | TS `import` Pattern | Example |
|-------------------|--------------------| --------|
| `use crate::foo::Bar;` | `import { Bar } from './foo';` | Within same package |
| `use crate::foo::bar::Baz;` | `import { Baz } from './foo/bar';` | Nested module |
| `use super::Bar;` | `import { Bar } from '../bar';` or `import { Bar } from '..';` | Parent module |
| `use self::foo::Bar;` | `import { Bar } from './foo';` | Same-level sub-module |

### F3. Conditional Import Rules

- Rust `#[cfg(feature = "wasm")] use wasm_bindgen::*;` -> No TS equivalent (skip entirely).
- Rust `#[cfg(test)] use ...;` -> Place test-only imports in the `.test.ts` file.

### F4. Re-export Chains

When Rust re-exports a type through multiple levels (e.g., `lib.rs -> pub use model::Model;`), the TS `index.ts` should mirror the same re-export chain:

```typescript
// packages/core/src/index.ts
// MIRRORS: ankurah/core/src/lib.rs
export { Model } from './model';
export { Node } from './node';
export { EntityId } from '@ankurah/proto';
// ... etc
```

---

## G. Annotation Requirements

### G1. Line 1 Comment (Mandatory)

Every `.ts` source file in the ankurah-ts monorepo MUST have exactly one of these on line 1:

**For files mirroring a Rust file:**
```typescript
// MIRRORS: ankurah/<crate-path>/src/<file-path>.rs
```
Example:
```typescript
// MIRRORS: ankurah/core/src/entity.rs
```

**For files with no Rust counterpart:**
```typescript
// TS-ONLY: <brief reason>
```
Example:
```typescript
// TS-ONLY: React Native useQuery hook (replaces Rust UniFFI bridge, see E15)
```

### G2. Inline Divergence Comments

When a specific code block within a mirrored file diverges from the Rust equivalent, add an inline comment:

```typescript
// Divergence: Rust uses Arc<RwLock<EntityInnerState>> here; TS uses plain property [E8]
private state: EntityState;
```

### G3. Exception Citations

When an exception rule applies to a file or code block, cite it explicitly:

```typescript
// Exception E5: yrs.rs -> yjs.ts due to library rename (Yrs -> Yjs)
```

```typescript
// Exception E12: reactor.rs is a file-with-submodules in Rust; becomes reactor/index.ts in TS
```

### G4. Rust Reference Comments

For complex logic that is faithfully ported, optionally reference the Rust source location to aid future comparison:

```typescript
// See Rust: ankurah/core/src/reactor.rs:330-370 (notify_change)
async notifyChange<C extends ChangeNotification>(changes: C[]): Promise<void> {
    // ...
}
```

### G5. Test File Annotations

Test files use the MIRRORS annotation pointing to the source of the original tests:

```typescript
// MIRRORS: ankurah/core/src/reactor.rs (tests module)
```

Or for integration tests:

```typescript
// MIRRORS: ankurah/tests/tests/basic.rs
```

### G6. Rust Source Hash Manifest (Drift Detection)

A hash manifest file at `scripts/rust-source-hashes.json` tracks the SHA-256 hash of each Rust source file that has been ported. This enables automated drift detection: when a Rust file changes, the audit script flags the corresponding TS file as potentially needing an update.

**Manifest format** (`scripts/rust-source-hashes.json`):
```json
{
  "core/src/entity.rs": "a1b2c3d4e5f6...",
  "proto/src/id.rs": "f6e5d4c3b2a1..."
}
```

Keys are Rust file paths relative to the Rust repo root (e.g. `proto/src/id.rs`, NOT `ankurah/proto/src/id.rs`). Values are full SHA-256 hex digests of the file contents.

**Workflow:**
1. **Bootstrap**: Run `bun run scripts/audit-port.ts --backpopulate` to scan all existing MIRRORS annotations and compute hashes of the current Rust files. This creates the initial manifest.
2. **Audit**: The regular audit (`bun run scripts/audit-port.ts`) compares current Rust file hashes against the manifest and warns about any drift.
3. **After porting changes**: Run `bun run scripts/audit-port.ts --update-manifest` to record the new Rust file hashes after reviewing/porting the changes.

**Rule**: When porting a Rust file change to TS, always update the manifest afterward so the drift detection stays current. The manifest file should be committed alongside the TS changes.

---

## H. Validation Checklist

Use this checklist to verify port compliance:

- [ ] Every `.ts` file has a line 1 MIRRORS or TS-ONLY comment
- [ ] Every MIRRORS comment points to an existing Rust file
- [ ] Every in-scope Rust file has a corresponding TS file
- [ ] File names match (snake_case preserved, except E5 yrs->yjs)
- [ ] Directory structure mirrors Rust module hierarchy
- [ ] All exception usages cite an E-number
- [ ] No TS file exists without either a MIRRORS or TS-ONLY annotation
- [ ] Cross-crate imports use `@ankurah/<package>` format
- [ ] Intra-package imports use relative paths
- [ ] Test files are adjacent to source (`.test.ts`) or in `__tests__/`
- [ ] Re-export chains in `index.ts` match Rust `lib.rs` / `mod.rs` re-exports
- [ ] Hash manifest (`scripts/rust-source-hashes.json`) is updated after porting Rust changes
