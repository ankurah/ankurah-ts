# ankurah-ts Transpiler — Specification

## Executive Summary

A Rust binary that transpiles Rust source code into TypeScript. The pipeline:

```
Rust source → syn parse → Rust AST → TRANSFORM → OXC TS AST → oxc_codegen → TS text → write to file
```

The **transform layer** (Rust AST → OXC AST) is where all translation rules are codified. This is the core of the tool and where most effort goes. `syn` handles Rust parsing, OXC handles TS AST representation and code generation.

**The existing port is the test suite for the transpiler.** Every file we've already ported is expected output. Run the transpiler, write the output, `git diff` to see discrepancies. Discrepancies are either a transpiler bug or a porting bug — both valuable.

**The transpiler does no diffing itself.** It writes files. `git diff` is the validation tool.

**Phases:**
1. **Skeleton transpiler** — generate TS declarations (classes, interfaces, functions, imports, exports) from Rust. No function bodies.
2. **Body transpiler** — translate function bodies via AST-level pattern matching.
3. **Production transpiler** — managed file whitelist, override annotations for manual exceptions.

**Dependencies:** `syn` (Rust parsing), `oxc_ast` + `oxc_codegen` + `oxc_allocator` + `oxc_span` (TS AST + code generation), `clap` (CLI), `walkdir` (file discovery), `anyhow` (errors), `toml` (config).

## Required Context

The transpiler is NOT a generic Rust→TS tool. It targets the specific ankurah-ts architecture. The transform layer must be aware of and implement:

| Document | What it governs |
|----------|----------------|
| `packages/base/src/` | The ownership type hierarchy: `AkObject`, `Struct`, `Enum<V>`, `Drop`, `Arc<T>`, `Weak<T>`, `Borrow<T>`, `BorrowMut<T>`, `Mutex<T>`, `RefCell<T>`, `AsyncMutex`. These are the types the transpiler emits — not generic TS classes. |
| `port/decisions.md` | Architectural decisions: bincode-only wire format, Yjs (not Yrs), error handling as throw, `defineModel()` for derive macros, bun workspaces, etc. |
| `port/ownership.md` | How Rust ownership (Drop, lifetimes, borrows, Arc, Mutex) maps to the TS types in `@ankurah/base`. |
| `port/ownership/provided-types.md` | API reference for the provided ownership types. |
| `port/translation-rules.md` | The full mechanical translation rule set: file naming, identifier naming, type mapping, enum patterns, error handling, async mapping, visibility, feature flags, exceptions (E1-E18). |

**The transpiler must read and implement ALL of these.** For example:
- `struct Foo` → `class Foo extends Struct` (not plain `class Foo`)
- `impl Drop for T` → `class T extends Drop` with `drop()` override
- `Arc<T>` stays as `Arc<T>` (from `@ankurah/base`), not converted to plain reference
- `enum Foo { A, B(T) }` → `class Foo extends Enum<FooV>` with variant type map
- `Result<T, E>` → `T` (throws on error), not `Result<T, E>`
- `#[derive(Serialize, Deserialize)]` → `encode(writer: BincodeWriter)` / `static decode(reader: BincodeReader)`

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
                    │  TRANSFORM  │  ◀── This is where all the rules live
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   OXC AST   │──▶ oxc_ast::ast::Program (TS AST)
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ oxc_codegen │──▶ TypeScript text
                    └─────────────┘
                           │
                           ▼
                    Write to disk ──▶ git diff to validate
```

### Why OXC for output?

- Proper AST means well-formed, correctly formatted TS output
- No hand-written string concatenation
- OXC's codegen handles indentation, semicolons, line breaks
- The AST is the same one used by real TS tooling (linters, formatters, bundlers)
- As the transpiler evolves, having a real AST enables optimization and analysis passes

---

## Phase 1: Skeleton Transpiler

Parse Rust, build OXC TS AST for declarations, generate TS text.

### 1.1 Rust Extraction (via `syn`)

Parse each `.rs` file with `syn::parse_file()`. Extract:

| Rust item | `syn` type | Data to extract |
|-----------|-----------|-----------------|
| `fn` | `ItemFn` / `ImplItemFn` / `TraitItemFn` | name, visibility, is_async, params, return type, body span |
| `struct` | `ItemStruct` | name, visibility, fields (name + type each), generics |
| `enum` | `ItemEnum` | name, visibility, variants (name + fields each) |
| `trait` | `ItemTrait` | name, visibility, method signatures |
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
- `#[derive(...)]` — record derived traits (Clone→clone, Debug→toString, PartialEq→equals, Serialize/Deserialize→encode/decode)

**Context tracking:**
- Which `impl` block a method belongs to → merge into corresponding TS class
- Which `mod tests` block a function belongs to → generate into `.test.ts`

### 1.2 Transform: Rust AST → OXC AST

This is the core of the transpiler. For each `syn` item, construct the equivalent `oxc_ast` node.

**Structs → Classes:**
```
syn::ItemStruct { ident: "Node", fields: [id: EntityId, durable: bool] }
    ↓
oxc_ast::ast::Class { id: "Node", body: [PropertyDefinition("id"), PropertyDefinition("durable")] }
```

Plus: merge all `impl Node { ... }` methods into the class body as `MethodDefinition` nodes.

**Enums → Classes extending Enum<V>:**
```
syn::ItemEnum { ident: "DeltaContent", variants: [StateSnapshot{state}, EventBridge{events}] }
    ↓
oxc_ast::ast::Class { id: "DeltaContent", superClass: "Enum<DeltaContentV>" }
```

Plus: generate the variant type map `type DeltaContentV = { ... }`.

**Traits → Interfaces:**
```
syn::ItemTrait { ident: "StorageEngine", items: [fn collection(...)] }
    ↓
oxc_ast::ast::TSInterfaceDeclaration { id: "StorageEngine", body: [TSMethodSignature("collection")] }
```

**Functions → Function declarations or method definitions:**
```
syn::ItemFn { ident: "next_entity_id", sig: { asyncness: None, inputs: [&self], output: EntityId } }
    ↓
oxc_ast::ast::MethodDefinition { key: "nextEntityId", value: Function { async: false, params: [], returnType: "EntityId" } }
```

Phase 1 bodies are just `throw new Error('TODO')` stubs.

**use → import:**
```
syn::ItemUse { path: "ankurah_proto::EntityId" }
    ↓
oxc_ast::ast::ImportDeclaration { source: "@ankurah/proto", specifiers: [ImportSpecifier("EntityId")] }
```

Using the crate→package mapping table.

### 1.3 Name Mapping

All deterministic.

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

| Rust type | TS type |
|-----------|---------|
| `String` / `&str` | `string` |
| `bool` | `boolean` |
| `i16` / `i32` / `usize` | `number` |
| `i64` / `u64` | `bigint \| number` |
| `f64` | `number` |
| `Vec<T>` | `T[]` |
| `Option<T>` | `T \| null` |
| `Result<T, E>` | `T` (throws on error) |
| `HashMap<K,V>` / `BTreeMap<K,V>` | `Map<K,V>` |
| `HashSet<T>` / `BTreeSet<T>` | `Set<T>` |
| `Vec<u8>` / `&[u8]` | `Uint8Array` |
| `Arc<T>` | `Arc<T>` |
| `Weak<T>` | `Weak<T>` |
| `Mutex<T>` / `RwLock<T>` | `Mutex<T>` |
| `RefCell<T>` | `RefCell<T>` |
| `Box<dyn Trait>` | `Trait` |
| `&T` (in fields) | `Borrow<T>` |
| `&mut T` (in fields) | `BorrowMut<T>` |
| `AtomicBool` | `boolean` |
| `AtomicU32` / `AtomicUsize` | `number` |

### 1.5 Commit Hash Attestation

After a human verifies a function is correctly ported, they add `// @<hash>` on the preceding line:

```typescript
// @abc1234
nextEntityId(): EntityId {
```

The transpiler can optionally check these:
- `git log -1 --format=%h -- <rust-file>` gets the current Rust commit
- If `@hash` differs → flag as stale

### 1.6 File Discovery & Output

- Walk `ankurah-ts-support/<crate>/src/` to find Rust source files
- Use crate→package mapping to determine target TS path
- `mod.rs` / `lib.rs` → `index.ts`
- `#[cfg(test)] mod tests { ... }` → `<name>.test.ts`
- Write generated TS to the target path
- Run `git diff` to see what changed vs existing

---

## Phase 2: Body Transpiler

Translate function bodies via AST-level transformation.

### 2.1 Rust Expression → OXC Expression

Instead of string-based pattern matching, this maps `syn::Expr` variants to `oxc_ast::ast::Expression` variants:

| `syn::Expr` | `oxc_ast::ast::Expression` |
|-------------|---------------------------|
| `Expr::Let { pat, init }` | `VariableDeclaration { kind: const/let, init }` |
| `Expr::Match { expr, arms }` | `expr.match({ arm1: ..., arm2: ... })` call expression |
| `Expr::MethodCall { receiver, method, args }` | `MemberExpression + CallExpression` |
| `Expr::If { cond, then, else }` | `IfStatement` |
| `Expr::Block { stmts }` | `BlockStatement` |
| `Expr::Return { expr }` | `ReturnStatement` |
| `Expr::Await { base }` | `AwaitExpression` |
| `Expr::Try { expr }` | (unwrap — throws propagate) |
| `Expr::Closure { params, body }` | `ArrowFunctionExpression` |
| `Expr::ForLoop { pat, expr, body }` | `ForOfStatement` |

Macro translations:
| Rust macro | OXC node |
|------------|----------|
| `vec![a, b]` | `ArrayExpression([a, b])` |
| `format!("...", args)` | `TemplateLiteral` |
| `println!("...")` | `console.log(...)` CallExpression |

### 2.2 Validation

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

### 3.3 Override mechanism

```typescript
// @transpiler-override: <reason>
```

The transpiler preserves existing code for items with this annotation.

---

## Project Structure

```
transpile/
├── Cargo.toml
├── transpile.toml        # Configuration (paths, crate mapping, name overrides)
├── src/
│   ├── main.rs           # CLI entry point (clap)
│   ├── config.rs         # Read transpile.toml
│   ├── rust_parser.rs    # syn-based Rust extraction
│   ├── name_map.rs       # Deterministic name mapping
│   ├── transform.rs      # Rust AST → OXC AST (THE CORE)
│   ├── types.rs          # Rust→TS type mapping
│   └── attestation.rs    # Commit hash checking (optional)
```

## Dependencies

```toml
[dependencies]
syn = { version = "2", features = ["full", "parsing"] }
proc-macro2 = { version = "1", features = ["span-locations"] }
oxc_ast = "0.120"
oxc_codegen = "0.120"
oxc_allocator = "0.120"
oxc_span = "0.120"
clap = { version = "4", features = ["derive"] }
walkdir = "2"
anyhow = "1"
toml = "0.8"
```

## CLI

```bash
# Transpile one Rust file, write TS to expected path
cargo run -- transpile core/src/node.rs

# Transpile all files in a crate
cargo run -- transpile --crate ankql

# Transpile all crates
cargo run -- transpile --all

# Dry run (print to stdout, don't write)
cargo run -- transpile core/src/node.rs --dry-run

# Check attestation hashes
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

[managed_files]
# Phase 3: files the transpiler owns (start empty, grow gradually)
```
