# ankurah-ts Transpiler — Specification

## Executive Summary

A Rust binary that transpiles Rust source code into TypeScript. The pipeline:

```
Rust source → syn parse → classify items → route to transform modules → TS text → write to file
```

The **transform layer** is where all translation rules are codified. `syn` handles Rust parsing. TS output is currently string-based (validated via spike); OXC AST generation is a future upgrade path.

**The existing port is the test suite for the transpiler.** Every file we've already ported is expected output. Run the transpiler, write the output, `git diff` to see discrepancies. Discrepancies are either a transpiler bug or a porting bug — both valuable.

**The transpiler does no diffing itself.** It writes files. `git diff` is the validation tool.

**Phases:**
1. **Skeleton transpiler** — generate TS declarations (classes, interfaces, functions, imports, exports) from Rust. No function bodies. *(In progress — spike validated.)*
2. **Body transpiler** — translate function bodies via AST-level pattern matching.
3. **Production transpiler** — managed file whitelist, whole-crate batch processing.

**Dependencies:** `syn` (Rust parsing), `quote` (token stream), `proc-macro2` (span locations), `clap` (CLI), `walkdir` (file discovery), `anyhow` (errors), `toml` (config).

## Required Context

The transpiler is NOT a generic Rust→TS tool. It targets the specific ankurah-ts architecture. The transform layer must be aware of and implement:

| Document | What it governs |
|----------|----------------|
| `packages/base/src/` | The ownership type hierarchy: `AkObject`, `Struct`, `Enum<V>`, `Drop`, `Arc<T>`, `Weak<T>`, `Borrow<T>`, `BorrowMut<T>`, `Mutex<T>`, `RefCell<T>`, `AsyncMutex`. These are the types the transpiler emits — not generic TS classes. |
| `port/decisions.md` | Architectural decisions: bincode-only wire format, Yjs (not Yrs), error handling as throw, `defineModel()` for derive macros, bun workspaces, etc. |
| `port/ownership.md` | How Rust ownership (Drop, lifetimes, borrows, Arc, Mutex) maps to the TS types in `@ankurah/base`. |
| `port/ownership/provided-types.md` | API reference for the provided ownership types. |
| `port/translation-rules.md` | The full mechanical translation rule set: file naming, identifier naming, type mapping, enum patterns, error handling, async mapping, visibility, feature flags, exceptions (E1-E18). |

---

## Architecture

### Pipeline

```
                    ┌─────────────┐
  Rust .rs file ──▶ │  syn parse  │──▶ syn::File (Rust AST)
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  CLASSIFY   │  ◀── Detect derives, custom impls, crate imports
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   ROUTE     │  ◀── Module dispatch based on classification + config
                    └─────────────┘
                     │    │    │
            ┌────────┘    │    └────────┐
            ▼             ▼             ▼
       ┌─────────┐  ┌──────────┐  ┌──────────┐
       │ default  │  │ bincode  │  │ provided │
       │transform │  │ rewrite  │  │   impl   │
       └─────────┘  └──────────┘  └──────────┘
            │             │             │
            └─────────────┴─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  TS output  │──▶ Write to disk ──▶ git diff to validate
                    └─────────────┘
```

### Transform Module System

The transpiler routes each item through one or more transform modules based on signals detected in the syn AST and configuration in `transpile.toml`. Modules compose — a single type can be processed by the default transform (class skeleton) AND a rewrite module (bincode encode/decode).

**Three module patterns:**

| Pattern | Reads syn AST? | Generates bodies? | When to use |
|---------|---------------|-------------------|-------------|
| **Default transform** | Yes | No (stubs) | Most types — 1:1 syntactic translation |
| **Rewrite module** | Yes (fields, types, variants) | Yes (full method bodies) | Code generated from derive macros (bincode, defineModel) |
| **Provided impl + redirect** | Minimally (just imports) | No (hand-written TS preserved) | Different library with compatible semantics (yrs → yjs compat), custom serde impls |

**Module routing signals (from syn AST):**

| Signal | How detected | Module routed to |
|--------|-------------|-----------------|
| `#[derive(Serialize, Deserialize)]` | Attribute on struct/enum | `bincode_rewrite` — generates encode/decode from field layout |
| `impl Serialize for T` (custom) | impl block with trait `Serialize` | Lookup `provided_impls` config — preserve hand-written TS |
| `#[derive(Model)]` | Attribute on struct | `model_rewrite` — generates defineModel() call *(future)* |
| `use yrs::*` | Use statement with configured crate | `provided_redirect` — rewrite imports to compat wrapper |
| `impl Drop for T` | impl block with trait `Drop` | Default transform with `extends Drop` base class |
| `#[cfg(feature = "wasm")]` | Attribute | Skip entirely |
| Everything else | — | Default transform |

### Drop Analysis

The transpiler includes a **transitive Drop ownership analyzer** (`drop-analysis` command). It walks all `.rs` files, finds types with `impl Drop`, then computes the transitive closure — which types contain Drop types through their fields.

**Validated results (full ankurah codebase):**
- 14 types with direct `impl Drop`
- 105 types that transitively contain Drop types
- 332 pure value types (no transitive Drop)

**Current approach:** All types extend AkObject (every Rust type gets disposal cascade and leak detection). The drop analysis is available as a **future optimization** — the transpiler could skip `using` declarations for the 332 value types that provably have no cleanup.

---

## Phase 1: Skeleton Transpiler *(In Progress)*

Parse Rust, generate TS declarations with stubbed function bodies.

### 1.1 Rust Extraction (via `syn`)

Parse each `.rs` file with `syn::parse_file()`. Extract:

| Rust item | `syn` type | Data to extract |
|-----------|-----------|-----------------|
| `fn` | `ItemFn` / `ImplItemFn` / `TraitItemFn` | name, visibility, is_async, params, return type |
| `struct` | `ItemStruct` | name, visibility, fields (name + type each), generics, derives |
| `enum` | `ItemEnum` | name, visibility, variants (name + fields each), generics, derives |
| `trait` | `ItemTrait` | name, visibility, method signatures, has_default_impls |
| `impl` (inherent) | `ItemImpl` | target type, methods |
| `impl Trait for Type` | `ItemImpl` with `trait_` | trait name, target type, methods |
| `type` alias | `ItemType` | name, underlying type |
| `const` / `static` | `ItemConst` / `ItemStatic` | name, type |
| `mod` | `ItemMod` | name, visibility, inline vs file |
| `use` | `ItemUse` | path, visibility |

**Attribute handling:**
- `#[cfg(feature = "wasm")]` / `#[cfg(feature = "uniffi")]` — skip item entirely
- `#[cfg(test)]` — mark as test-only (generates into `.test.ts`)
- `#[test]` / `#[tokio::test]` — mark function as test
- `#[derive(...)]` — record derived traits; route to appropriate module

**Context tracking:**
- Which `impl` block a method belongs to → merge into corresponding TS class
- `impl Trait for Type` → detect Serialize/Deserialize/Drop for module routing
- Which `mod tests` block a function belongs to → generate into `.test.ts`

### 1.2 Transform: syn Items → TS Output

**Structs → Classes:**
```
syn::ItemStruct { ident: "Event", fields: [collection: CollectionId, entity_id: EntityId, ...] }
    ↓ default transform
class Event extends Struct {
  readonly collection: CollectionId;
  readonly entityId: EntityId;
  ...
  constructor(...) { super(); throw new Error('TODO'); }
}
    ↓ bincode_rewrite (if derive(Serialize, Deserialize))
  encode(writer: BincodeWriter): void {
    this.collection.encode(writer);
    this.entityId.encode(writer);
    ...
  }
  static decode(reader: BincodeReader): Event { ... }
```

**Enums → Classes extending Enum\<V\>:**
```
syn::ItemEnum { ident: "DeltaContent", variants: [StateSnapshot{state}, EventBridge{events}] }
    ↓
type DeltaContentV = { StateSnapshot: { state: StateFragment }; EventBridge: { events: EventFragment[] }; };
class DeltaContent extends Enum<DeltaContentV> { ... }
    ↓ bincode_rewrite (if derive(Serialize, Deserialize))
  encode(writer: BincodeWriter): void {
    this.match({
      StateSnapshot: (v) => { writer.writeVariant(0); v.state.encode(writer); },
      EventBridge: (v) => { writer.writeVariant(1); writer.writeVec(v.events, (w, item) => item.encode(w)); },
    });
  }
```

**Traits → Interfaces (or abstract classes if default impls exist).**

**Functions → Function declarations or method definitions.** Phase 1 bodies are `throw new Error('TODO')` stubs.

**`impl Drop for T` → `class T extends Drop`** with `drop()` override stub.

### 1.3 Name Mapping

All deterministic. Implemented in `name_map.rs`.

**Functions:** `snake_case` → `camelCase`, plus static exceptions:

| Rust name | TS name |
|-----------|---------|
| `fmt` | `toString` |
| `serialize` | `encode` |
| `deserialize` | `decode` |
| `eq` | `equals` |
| `ne` | `notEquals` |
| `partial_cmp` | `compareTo` |
| `clone` | `clone` |
| `default` | `default` |
| `drop` | `drop` |
| `from` | `from` |
| `try_from` | `tryFrom` |
| `new` | `new` |
| `next` | `next` |
| `deref` | `deref` |

**Types:** PascalCase stays unchanged.

**Test functions:** Preserved as-is. `fn test_foo_bar` → `test('test_foo_bar', ...)`.

**Modules:** `mod foo` → `foo.ts` or `foo/index.ts`. `mod.rs` / `lib.rs` → `index.ts`.

### 1.4 Type Mapping

Implemented in `name_map::map_type()`.

| Rust type | TS type |
|-----------|---------|
| `String` / `&str` | `string` |
| `bool` | `boolean` |
| `i16` / `i32` / `usize` | `number` |
| `i64` / `u64` | `bigint \| number` |
| `f64` | `number` |
| `Vec<T>` | `T[]` |
| `Vec<u8>` / `&[u8]` | `Uint8Array` |
| `Option<T>` | `T \| null` |
| `Result<T, E>` | `T` (throws on error) |
| `HashMap<K,V>` / `BTreeMap<K,V>` | `Map<K,V>` |
| `HashSet<T>` / `BTreeSet<T>` | `Set<T>` |
| `Arc<T>` | `Arc<T>` |
| `Weak<T>` | `Weak<T>` |
| `Mutex<T>` / `RwLock<T>` | `Mutex<T>` |
| `RefCell<T>` | `RefCell<T>` |
| `Box<dyn Trait>` | `Trait` |
| `&T` (in fields) | `Borrow<T>` |
| `&mut T` (in fields) | `BorrowMut<T>` |
| `AtomicBool` | `boolean` |
| `AtomicU32` / `AtomicUsize` | `number` |

### 1.5 Annotation Generation

Every generated file gets:
- Line 1: `// MIRRORS: ankurah/<crate>/src/<path>.rs`
- `#[cfg(test)] mod tests { ... }` generates a separate `.test.ts` file with its own MIRRORS annotation

### 1.6 File Discovery & Output

- Walk `ankurah-ts-support/<crate>/src/` to find Rust source files
- Use crate→package mapping to determine target TS path
- `mod.rs` / `lib.rs` → `index.ts`
- `#[cfg(test)] mod tests { ... }` → `<name>.test.ts`
- Write generated TS to the target path
- Run `git diff` to see what changed vs existing

---

## Bincode Rewrite Module

For structs/enums with `#[derive(Serialize, Deserialize)]`, generates `encode(writer: BincodeWriter)` and `static decode(reader: BincodeReader)` methods.

**Validated:** The generated output for `proto::data::Event` matches the hand-ported encode/decode exactly.

### Struct Encoding

Field-by-field in declaration order. For each field, dispatch based on TS type:

| TS type | Encode | Decode |
|---------|--------|--------|
| `string` | `writer.writeString(x)` | `reader.readString()` |
| `boolean` | `writer.writeBool(x)` | `reader.readBool()` |
| `number` | `writer.writeU32(x)` | `reader.readU32()` |
| `Uint8Array` | `writer.writeBytes(x)` | `reader.readBytes()` |
| `T[]` | `writer.writeVec(x, (w, item) => item.encode(w))` | `reader.readVec((r) => T.decode(r))` |
| `T \| null` | `writer.writeOption(x, (w, v) => v.encode(w))` | `reader.readOption((r) => T.decode(r))` |
| Custom type | `x.encode(writer)` | `T.decode(reader)` |

### Enum Encoding

Write variant discriminant (u32 index in declaration order), then variant fields.

### Custom Serde Detection

If a type has `impl Serialize for T` (explicit impl, not derive), the bincode module **does not generate** encode/decode. Instead, it looks up `[provided_impls]` in the config to find the hand-written implementation.

---

## Provided Impl Pattern

For types or libraries where the TS implementation is fundamentally different from a syntactic translation, the transpiler preserves hand-written TS code and/or redirects imports to compatibility wrappers.

### Provided Impl (Custom Serde)

Types with custom `impl Serialize` / `impl Deserialize` that can't be auto-generated:

```toml
[provided_impls]
"ankurah_proto::id::EventId" = { module = "provided", path = "packages/proto/src/id.ts" }
"ankurah_proto::id::CollectionId" = { module = "provided", path = "packages/proto/src/collection.ts" }
```

The transpiler sees `impl Serialize for EventId` (not derive), looks up the config, and preserves the hand-written encode/decode at the specified path.

### Provided Redirect (Library Substitution)

When the Rust code uses a library (e.g., `yrs`) that's replaced by a different JS library (e.g., `yjs`), the transpiler redirects imports to a hand-written compatibility wrapper:

```toml
[provided_redirect.yrs]
source_crate = "yrs"
target_module = "@ankurah/base/yrs-compat"
types = { "Doc" = "YrsDoc", "Text" = "YrsText", "Map" = "YrsMap" }
```

The transpiler sees `use yrs::Text`, emits `import { YrsText } from '@ankurah/base/yrs-compat'`. The compat wrapper has the same API surface as the Rust library, backed by the JS library internally. No function body rewriting needed.

### Hardcode (No Syntactic Correspondence)

Some files have no syntactic relationship to their Rust counterpart. The transpiler does not generate these — it preserves existing TS code:

```toml
[hardcode]
files = [
  "ankql/src/parser.rs",    # E6: hand-written recursive descent parser
  "ankql/src/grammar.rs",   # E6: hand-written grammar definitions
]
reason = "E6: no syntactic correspondence between Pest grammar and recursive descent parser"
```

Hardcoded files still participate in drift detection — the transpiler knows about them and flags when the Rust source changes. But it does not attempt to regenerate the TS.

---

## Phase 2: Body Transpiler

Translate function bodies via AST-level transformation.

### 2.1 Rust Expression → TS Expression

Maps `syn::Expr` variants to TS expression strings (or OXC AST nodes in future):

| `syn::Expr` | TS output |
|-------------|-----------|
| `Expr::Let { pat, init }` | `const/let x = init;` |
| `Expr::Match { expr, arms }` | `expr.match({ arm1: ..., arm2: ... })` |
| `Expr::MethodCall { receiver, method, args }` | `receiver.method(args)` |
| `Expr::If { cond, then, else }` | `if (cond) { then } else { else }` |
| `Expr::Block { stmts }` | `{ stmts }` |
| `Expr::Return { expr }` | `return expr;` |
| `Expr::Await { base }` | `await base` |
| `Expr::Try { expr }` | `expr` (unwrap — throws propagate) |
| `Expr::Closure { params, body }` | `(params) => body` |
| `Expr::ForLoop { pat, expr, body }` | `for (const pat of expr) { body }` |

Macro translations:

| Rust macro | TS output |
|------------|-----------|
| `vec![a, b]` | `[a, b]` |
| `format!("...", args)` | `` `...${args}` `` |
| `println!("...")` | `console.log(...)` |
| `assert_eq!(a, b)` | `expect(a).toEqual(b)` (test context) |
| `panic!("...")` | `throw new Error("...")` |

### 2.2 Ownership in Generated Code

The transpiler generates `using` for block-scoped AkObjects by default. The drop analysis can be used as an optimization to skip `using` for value types that provably have no cleanup (332 types identified). This optimization is deferred — correctness first.

### 2.3 Validation

Same as Phase 1 — write output, `git diff`.

---

## Phase 3: Production Transpiler

### 3.1 File whitelist

```toml
[managed_files]
ankql = ["src/ast.rs", "src/error.rs"]
# ... gradually expand as transpiler output matches existing code
```

### 3.2 Workflow

1. Rust source changes (new commit on `ankurah-ts-support`)
2. Run transpiler on managed files
3. `git diff` shows what changed
4. Review and commit

### 3.3 Provided impl preservation

Files/methods listed in `[provided_impls]` or `[hardcode]` are preserved — the transpiler does not overwrite them. Drift detection still flags when the corresponding Rust source changes, but regeneration is manual.

---

## Project Structure

```
transpile/
├── Cargo.toml
├── transpile.toml        # Configuration (paths, crate mapping, modules, provided impls)
├── src/
│   ├── main.rs           # CLI entry point (clap)
│   ├── skeleton.rs       # Phase 1: syn extraction + TS skeleton generation
│   ├── name_map.rs       # Deterministic name mapping (snake→camel, type mapping)
│   ├── drop_analysis.rs  # Transitive Drop ownership analysis
│   ├── bincode_module.rs # Rewrite module: generate encode/decode from field layout
│   ├── config.rs         # Read transpile.toml (TODO)
│   └── attestation.rs    # Commit hash checking (TODO)
```

## Dependencies

```toml
[dependencies]
syn = { version = "2", features = ["full", "parsing", "visit"] }
quote = "1"
proc-macro2 = { version = "1", features = ["span-locations"] }
clap = { version = "4", features = ["derive"] }
walkdir = "2"
anyhow = "1"
toml = "0.8"
```

Note: OXC dependencies (`oxc_ast`, `oxc_codegen`, etc.) are deferred. The spike validated that string-based TS generation works and produces correct output. OXC can be added later for better formatting and AST-level manipulation.

## CLI

```bash
# Analyze transitive Drop ownership for a crate
cargo run -- drop-analysis ../ankurah-ts-support/proto/src

# Generate TS skeleton for a single file (stdout)
cargo run -- skeleton ../ankurah-ts-support/proto/src/data.rs --crate-path proto/src/data.rs

# Transpile a whole crate (TODO)
cargo run -- transpile --crate ankql

# Transpile all crates (TODO)
cargo run -- transpile --all

# Check attestation hashes (TODO)
cargo run -- attest --check [--crate <name>]
```

## Configuration

```toml
# transpile/transpile.toml

[paths]
rust_source = "../ankurah-ts-support"
ts_target = ".."  # ankurah-ts root (packages/ is under this)

[crates]
ankql = "ankql"
"ankurah-proto" = "proto"
"ankurah-signals" = "signals"
"ankurah-core" = "core"
"ankurah-storage-common" = "storage-common"
"ankurah-storage-sqlite" = "storage-sqlite"
"ankurah-storage-postgres" = "storage-postgres"
"ankurah-storage-indexeddb-wasm" = "storage-indexeddb"
"ankurah-websocket-client" = "connector-websocket"
"ankurah-websocket-server" = "connector-websocket-server"
"ankurah-connector-local-process" = "connector-local"
"ankurah" = "ankurah"

[excluded_features]
skip = ["wasm", "uniffi"]

[name_overrides]
fmt = "toString"
serialize = "encode"
deserialize = "decode"
eq = "equals"
default = "default"
drop = "drop"
new = "new"
from = "from"

[provided_impls]
# Types with custom impl Serialize (not derive) — hand-written encode/decode preserved
"ankurah_proto::id::EventId" = { module = "provided", path = "packages/proto/src/id.ts" }
"ankurah_proto::id::CollectionId" = { module = "provided", path = "packages/proto/src/collection.ts" }

[provided_redirect.yrs]
# Rust yrs library → JS yjs library via compatibility wrapper
source_crate = "yrs"
target_module = "@ankurah/base/yrs-compat"
types = { "Doc" = "YrsDoc", "Text" = "YrsText", "Map" = "YrsMap" }

[hardcode]
# Files with no syntactic correspondence — transpiler preserves existing TS
files = [
  "ankql/src/parser.rs",
  "ankql/src/grammar.rs",
]

[managed_files]
# Phase 3: files the transpiler owns (start empty, grow gradually)
```
