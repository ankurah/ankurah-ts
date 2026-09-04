# Port Rules: ankurah (Rust) -> ankurah-ts (TypeScript)

**Written**: 2026-02-10. **Corrected**: 2026-09-02, where a later ruling repealed
the premise a rule rested on; each correction says what changed and is dated
inline, and [retractions-2026-09-02.md](retractions-2026-09-02.md) lists them in
one table.
**Authoritative for**: All file-level, naming, structural, and annotation conventions in the TS port.
**Mandate**: Zero freestyling. The TS port must mirror the Rust structure 1:1. Every exception requires an explicit rule citation.
**Who applies these rules**: the transpiler in `transpile/`. They are written for a reader, but they are implemented in code, and a rule that the transpiler does not implement is a wish, not a rule.

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
| `impl Drop for T` | `class T extends Drop` | Override `onDrop()`, never `drop()`. Corrected 2026-09-02: `drop()` is `AkObject`'s template and overriding it runs the cleanup after the fields are already gone. |
| `trait Foo { ... }` | `interface Foo { ... }` | Unless it has default impls (see E7) |
| `trait Foo` with default impls | `abstract class Foo` or mixin | See Exception E7 |
| `impl Trait for Struct` | `class Struct implements Interface` | See Exception E4 |
| `impl Struct { ... }` | Methods inside `class Struct { ... }` | See Exception E4 |
| Generics `<T: Trait>` | Generics `<T extends Interface>` | Direct mapping |
| `Result<T, E>` | `Result<T, E>` | From `@ankurah/base`. A returned value, not a throw. Corrected 2026-09-02; see A8. |
| `Option<T>` | `T \| null` | |
| `Arc<T>` | `Arc<T>` | From `@ankurah/base`. Refcounted shared ownership. |
| `Rc<T>` | `Arc<T>` | Same as Arc (no threading distinction in JS) |
| `Weak<T>` | `Weak<T>` | From `@ankurah/base`. `upgrade()` returns `Arc<T> \| null`. |
| `&T` (in struct fields) | `Borrow<T>` | From `@ankurah/base`. Non-owning: no `drop()` at all, and the cascade steps over it. |
| `&mut T` (in struct fields) | `BorrowMut<T>` | From `@ankurah/base`. Non-owning mutable, same treatment. |
| `Mutex<T>` | `Mutex<T>` | From `@ankurah/base`. `const guard = mutex.lock()`, dropped in a `finally`. Corrected 2026-09-02: was `using guard = ...`. |
| `RwLock<T>` | `RwLock<T>` | Corrected 2026-09-02: its own type, not an alias for `Mutex`. `read()` and `write()` return distinct guards. |
| `tokio::sync::Mutex<T>` | `AsyncMutex<T>` | From `@ankurah/base`. `acquire()` returns a guard; the lock survives an `await`. |
| `RefCell<T>` | `RefCell<T>` | From `@ankurah/base`. `borrow()` / `borrowMut()`. Corrected 2026-09-02: the method is `borrowMut()`, per the `snake_case` → `camelCase` rule in A2. |
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
| `#[derive(Serialize, Deserialize)]` | Bincode codec methods generated from the field list | See "Derives are read, never expanded" below |
| `#[derive(Model)]` | `defineModel()` | See Exception E1 |
| `#[derive(Hash)]` | No direct equivalent | Use string keys for maps if needed |

**Derives are read, never expanded.** The transpiler reads the derive attribute
and the field list and generates the TypeScript from those. It never expands the
macro and translates the expansion, and the reason is not just that expanded
code drags Rust idioms along with it: **a serde derive expansion contains no
wire-format information at all.** serde's generated code is format-agnostic — it
calls into a `Serializer`, and which byte a variant index becomes lives in the
bincode crate, never in the expansion. Twelve thousand lines of expanded proto
code cannot tell you the wire format; the transpiler's bincode module reads it
off the field list. The expansion would also carry `_serde::__private228`, which
pins a serde patch version into the port, and `format_args!` survives expansion
unexpanded anyway.

### A8. Error Handling

**Rewritten 2026-09-02.** The rule used to be "`Result` maps to throw". It does
not: a Rust function that returns `Result<T, E>` returns a `Result<T, E>` value,
and a throw means a panic.

- Rust `Result<T, E>` maps to `Result<T, E>` from `@ankurah/base` — a returned value.
- Mirror Rust error types 1:1 as TS classes: `MutationError`, `RetrievalError`, `StateError`, `PropertyError`, `DecodeError`, `SubscriptionError`. They are what goes in the `Err`.
- Rust `anyhow::Error` maps to a generic error type in the `Err` position.
- Rust `.unwrap()` / `.expect()` map to `Result.unwrap()` / `.expect()`, which consume the receiver and throw on an `Err` — because that is what panics in Rust.
- Rust `?` maps to a check on the `Result`: return the `Err` onward, and drop the `Ok` wrapper you did not return. Discarding the `Result` object instead leaks it.
- A throw in emitted code means the program did something Rust would panic on. A throw from the ownership runtime (`OwnershipFatal`) means the *emitter* did something Rust would not compile; a `catch` that handles a Rust error type must rethrow it unconditionally.

**Current state**: the transpiled packages still throw where Rust returns a
`Result`. That is a known defect awaiting the emission step, not a second
sanctioned style.

### A9. Async Mapping

**Rewritten 2026-09-02.** tokio's shapes have direct TypeScript equivalents, and
the port uses them rather than inventing an async layer.

- Rust `async fn foo() -> Result<T, E>` maps to TS `async foo(): Promise<Result<T, E>>`.
- `tokio::spawn` maps to a `Promise`; a `JoinHandle` is a wrapper around it, and awaiting the handle awaits the promise.
- `select!` maps to `Promise.race`, with the winning branch tagged so the arms stay distinguishable. **The one real difference**: `select!` drops — cancels — the losing futures, while a losing `Promise` keeps running. ankurah has a single `select!` site; check it for side effects in the losing branch rather than designing around the difference.
- `Notify` and `oneshot` map to a promise plus its resolver; `mpsc` maps to an async queue.
- `tokio::sync::Mutex` maps to `AsyncMutex` from `@ankurah/base`.

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

**Corrected 2026-09-02**: scope is decided by target environment, not by phase.
Two rows flipped — the browser WebSocket client is in and the tokio one is out,
and IndexedDB is in. See the crate scope table in
[port-runbook.md](port-runbook.md), which is authoritative.

| Rust Crate (Cargo.toml `name`) | Rust Path | TS Package | TS Path | Status |
|-------------------------------|-----------|------------|---------|--------|
| `ankurah-proto` | `proto/` | `@ankurah/proto` | `packages/proto/` | IN SCOPE |
| `ankurah-core` | `core/` | `@ankurah/core` | `packages/core/` | IN SCOPE |
| `ankurah-signals` | `signals/` | `@ankurah/signals` | `packages/signals/` | IN SCOPE |
| `ankql` | `ankql/` | `@ankurah/ankql` | `packages/ankql/` | IN SCOPE |
| `ankurah-storage-common` | `storage/common/` | `@ankurah/storage-common` | `packages/storage-common/` | IN SCOPE |
| `ankurah-storage-sqlite` | `storage/sqlite/` | `@ankurah/storage-sqlite` | `packages/storage-sqlite/` | IN SCOPE; the rusqlite binding becomes a provided driver interface (E16) |
| `ankurah-storage-indexeddb-wasm` | `storage/indexeddb-wasm/` | `@ankurah/storage-indexeddb` | `packages/storage-indexeddb/` | IN SCOPE — ankurah's browser storage path; `web-sys` resolves to the IndexedDB API |
| `ankurah-websocket-client-wasm` | `connectors/websocket-client-wasm/` | `@ankurah/connector-websocket` | `packages/connector-websocket/` | IN SCOPE — the `web-sys` WebSocket client is the browser and React Native client (E17) |
| `ankurah-connector-local-process` | `connectors/local-process/` | `@ankurah/connector-local` | `packages/connector-local/` | IN SCOPE |
| `ankurah` | `ankurah/` | `@ankurah/ankurah` | `packages/ankurah/` | IN SCOPE — the facade crate |
| `ankurah-derive` | `derive/` | NO TS EQUIVALENT | N/A | Exception E1: TS has no proc macros |
| `ankurah-websocket-client` | `connectors/websocket-client/` | OUT OF SCOPE | N/A | The tokio-tungstenite client; the browser client above replaces it |
| `ankurah-storage-postgres` | `storage/postgres/` | OUT OF SCOPE | N/A | Server-side database; neither primary target can reach it |
| `ankurah-storage-sled` | `storage/sled/` | OUT OF SCOPE | N/A | Rust-specific embedded DB |
| `ankurah-websocket-server` | `connectors/websocket-server/` | OUT OF SCOPE | N/A | The Rust server stays the server |

### B2. Additional TS-Only Packages

| TS Package | TS Path | Purpose | Annotation |
|-----------|---------|---------|------------|
| `@ankurah/base` | `packages/base/` | The hand-written ownership runtime every other package builds on | TS-ONLY: Rust ownership primitives (E11) |
| `@ankurah/react` | `packages/react/` | React hooks and bindings (the package is named `react`, not `react-native`) | TS-ONLY: React integration (E15) |
| `@ankurah/storage-memory` | `packages/storage-memory/` | In-memory storage for testing | TS-ONLY: test utility |
| `@ankurah/eslint-plugin` | `packages/eslint-plugin-ankurah/` | Lint rules standing in for Rust's compile-time ownership checks | TS-ONLY |

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

**Counted 2026-02-10 and not maintained since.** The Rust source has moved on and
the crate scope changed on 2026-09-02, so read this as the shape of the mapping,
not as today's numbers. The transpiler's own inventory (`transpile/tests/`) is
what counts files now.

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
- **TS equivalent**: the `defineModel()` runtime, which produces the same View/Mutable/Input/Ops shapes from a schema written in TypeScript.
- **Justification**: TypeScript has no compile-time proc macro system. The TS code matches the OUTPUT of the Rust macro, not the macro implementation itself.
- **Citation note (2026-09-02)**: this is the rule to cite for the derive crate. Earlier text in the runbook cited E12, which is the file-with-submodules rule and has nothing to do with derives.

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
- **Not to be confused with (2026-09-02)**: a *crate* whose name ends in `-wasm`. `storage/indexeddb-wasm` and `connectors/websocket-client-wasm` are ankurah's browser implementations, and they are in scope — their `web-sys` calls are IndexedDB and WebSocket, which TypeScript has natively. This rule skips the modules that convert Rust values to and from `JsValue`, nothing more. The transpiler's `cfg` evaluation is being moved to ankurah's wasm32 configuration for the same reason: the browser branch ankurah already maintains and tests is the branch the port should follow.

### E10: Feature-Gated Database Modules Skipped

- **Rust pattern**: `#[cfg(feature = "postgres")]` in `proto/src/postgres.rs`.
- **TS equivalent**: No TS file created.
- **Justification**: PostgreSQL support is out of scope for Phase 1. The feature-gated module provides Postgres-specific type conversions not needed in the TS port.

### E11: Drop Semantics -> @ankurah/base Drop + AkObject

**Rewritten 2026-09-02.** The rule used to say the cascade runs through
`[Symbol.dispose]()` and that `using` declarations trigger cleanup at block
exit. Both are retired: Hermes refuses to run `using` declarations at all, so
the mechanism is an explicit `.drop()` call that the transpiler emits.

- **Rust pattern**: `impl Drop for T { fn drop(&mut self) { ... } }` for custom cleanup. Automatic drop cascade for all owned fields.
- **TS equivalent**: `class T extends Drop` with a `protected override onDrop()` for the custom cleanup. `AkObject.drop()` is the whole template and nothing overrides it: it refuses a second drop, leaves the leak registry, runs `onDrop()` while the fields are still alive, then drops what the value owns. A value the block owns is dropped in a `finally`; a guard temporary is dropped at the end of its statement and again in that `finally`, which is why a guard's second drop is deliberately a no-op.
- **Justification**: JS has no deterministic destructors. `@ankurah/base` provides `AkObject` (the template and the cascade), `Drop` (the cleanup hook), `Arc`/`Weak` (refcounted ownership), the containers (`Mutex`, `RwLock`, `RefCell`, `AsyncMutex`), and `Borrow`/`BorrowMut` (non-owning, stepped over by the cascade), matching Rust's ownership model as closely as a running program can.
- **Full contract**: [ownership.md](ownership.md).

### E12: File-With-Submodules -> Directory with index.ts

- **Rust pattern**: A file like `reactor.rs` that also has a directory `reactor/` containing sub-modules. Rust allows this (the file IS the module, sub-files are sub-modules).
- **TS equivalent**: A directory `reactor/` with `index.ts` containing the primary code and sibling files for sub-modules.
- **Justification**: TypeScript cannot have both a file `reactor.ts` and a directory `reactor/`. The file must become `reactor/index.ts`.

### E13: Rust Macros -> TS Functions or Logger Calls

- **Rust pattern**: `macro_rules! action_info { ... }` and similar logging and formatting macros in `core/src/util/mod.rs`; `#[error]` from thiserror; the `tracing` macros.
- **TS equivalent**: a targeted translation per macro family — a provided logger module for the logging macros, and generated `toString()` bodies for thiserror's `#[error]` format strings. No macro is ever expanded and translated.
- **Justification**: TypeScript has no compile-time macro system, and expanding the Rust macro would translate serde's or tracing's implementation instead of ankurah's code. See "Derives are read, never expanded" in A7 for why an expansion is also less informative than the source.
- **Current state (2026-09-02)**: the transpiler emits `/* name!(...) */` comments for these — 14 error enums with 89 `#[error]` attributes, 83 tracing sites, 15 `action_*!`/`notice_info!` calls. Every `Display` impl and all logging are therefore silently missing from the transpiled output today. Closing this is the first macro work, not a licence to hand-write the output.

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

### G4. Automated Parity Checking + Commit-Hash Attestation

**Parity checking is fully automated.** The script `port/check-attestations.ts` extracts every item from Rust (fn, struct, enum, trait, impl) and maps names to TS automatically:
- `snake_case` → `camelCase` for functions (e.g., `fetch_from_peer` → `fetchFromPeer`)
- `PascalCase` stays for types (e.g., `struct Node` → `class Node`)
- Static mapping for special cases: `fmt` → `toString`, `serialize` → `encode`, `deserialize` → `decode`, `eq` → `equals`, etc.

No manual annotation is needed for mapping. The script finds TS counterparts automatically and flags anything missing.

**Commit-hash attestation** is the human sign-off. After verifying a function/type matches Rust, add `// @<hash>` on the line immediately before it, where `<hash>` is the short commit hash of the latest commit that modified the Rust source file:

```typescript
// @abc1234
async fetchFromPeer(peerId: EntityId, collection: CollectionId, args: MatchArgs): Promise<Entity[]> {

// @abc1234
class Node {
```

The script checks:
1. Every Rust item has a TS counterpart (automated name matching)
2. Every matched item has `// @<hash>` on the preceding line
3. The hash matches the latest commit on the Rust file (flags stale attestations when Rust changes)

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

A hash manifest file at `port/.rust-source-hashes.json` tracks the SHA-256 hash of each Rust source file that has been ported. This enables automated drift detection: when a Rust file changes, the audit script flags the corresponding TS file as potentially needing an update.

**Manifest format** (`port/.rust-source-hashes.json`):
```json
{
  "core/src/entity.rs": "a1b2c3d4e5f6...",
  "proto/src/id.rs": "f6e5d4c3b2a1..."
}
```

Keys are Rust file paths relative to the Rust repo root (e.g. `proto/src/id.rs`, NOT `ankurah/proto/src/id.rs`). Values are full SHA-256 hex digests of the file contents.

**Workflow:**
1. **Bootstrap**: Run `bun run port/audit-port.ts --backpopulate` to scan all existing MIRRORS annotations and compute hashes of the current Rust files. This creates the initial manifest.
2. **Audit**: The regular audit (`bun run port/audit-port.ts`) compares current Rust file hashes against the manifest and warns about any drift.
3. **After porting changes**: Run `bun run port/audit-port.ts --update-manifest` to record the new Rust file hashes after reviewing/porting the changes.

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
- [ ] Hash manifest (`port/.rust-source-hashes.json`) is updated after porting Rust changes
- [ ] Every Rust `pub fn` has a `// Rust: fn <name>` attestation in the TS file (G4)
- [ ] No `as unknown as` without a `// Divergence:` justification on the same or preceding line
- [ ] TS function body sizes are within reasonable range of Rust counterparts (audit script flags >50% shorter)
