# ankurah-ts Parity Checker / Transpiler — Specification

## Overview

A Rust binary that verifies structural and semantic parity between the Rust source (`ankurah-ts-support/`) and the TypeScript port (`ankurah-ts/packages/`). Uses proper AST parsing for both languages — `syn` for Rust, `deno_ast` (SWC) for TypeScript.

Designed to evolve from a checker into a transpiler in phases.

## Guiding Principle

**The existing port is the test suite for the transpiler.** Every file we've already ported is expected output. The checker/transpiler is validated against it — discrepancies are either a checker bug or a porting bug. Both are valuable findings.

---

## Phase 1: Parity Checker

Verify that every Rust item has a TS counterpart, with correct name mapping and commit-hash attestations.

### 1.1 Rust Extraction (via `syn`)

Parse each `.rs` file with `syn::parse_file()`. Extract:

| Rust item | `syn` type | Data to extract |
|-----------|-----------|-----------------|
| `fn` | `ItemFn` / `ImplItemFn` / `TraitItemFn` | name, visibility, is_async, params, return type, body span, line number |
| `struct` | `ItemStruct` | name, visibility, fields, generics, line number |
| `enum` | `ItemEnum` | name, visibility, variants (name + fields each), line number |
| `trait` | `ItemTrait` | name, visibility, methods (name + signature each), line number |
| `impl` (inherent) | `ItemImpl` | target type, methods, line number |
| `impl Trait for Type` | `ItemImpl` with `trait_` | trait name, target type, methods, line number |
| `type` alias | `ItemType` | name, line number |
| `const` / `static` | `ItemConst` / `ItemStatic` | name, type, line number |
| `mod` | `ItemMod` | name, visibility, inline vs file |
| `use` | `ItemUse` | path, visibility |

**Attribute detection:**
- `#[cfg(test)]` — mark item as test-only
- `#[cfg(feature = "wasm")]` — mark item as wasm-only (excluded from TS)
- `#[test]` / `#[tokio::test]` — mark function as test
- `#[derive(...)]` — record derived traits

**Context tracking:**
- Which `impl` block a method belongs to (for matching to TS class methods)
- Which `mod tests` block a test function belongs to
- Nesting depth for items inside `cfg` blocks

### 1.2 TypeScript Extraction (via `deno_ast` / SWC)

Parse each `.ts` file with `deno_ast::parse_module()`. Extract:

| TS item | SWC type | Data to extract |
|---------|---------|-----------------|
| `class` | `ClassDecl` (+ `ExportDecl` wrapper) | name, methods (name, is_static, is_async, params), properties, `extends`, `implements`, line number |
| `interface` | `TsInterfaceDecl` | name, methods, properties, extends, line number |
| `type` alias | `TsTypeAliasDecl` | name, line number |
| `function` | `FnDecl` (+ `ExportDecl` wrapper) | name, is_async, params, return type, body span, line number |
| `const fn` | `VarDecl` where init is `ArrowExpr` or `FnExpr` | name, is_async, params, body span, line number |
| `test()` call | `CallExpr` where callee is `test`/`it`/`describe` | test name (first string arg), body span, line number |
| `test.skip()` | `CallExpr` with member expr `test.skip` | test name, line number |
| Class methods | `ClassMethod` inside `ClassDecl` | name, is_static, is_async, body span, parent class |
| Class constructor | `Constructor` inside `ClassDecl` | params, body span, parent class |
| `export` | `ExportDecl` / `ExportNamed` | which items are exported |

**Comment/attestation extraction:**
- For each item, check the preceding line(s) for `// @<hex-hash>` pattern
- Record the hash value and which item it's attached to

### 1.3 Name Mapping

All mapping is deterministic. No ambiguity.

**Functions:** `snake_case` → `camelCase` via mechanical conversion, plus a static exception table:

```rust
static STATIC_MAP: &[(&str, &str)] = &[
    ("fmt", "toString"),
    ("serialize", "encode"),
    ("deserialize", "decode"),
    ("eq", "equals"),
    ("ne", "notEquals"),
    ("partial_cmp", "compareTo"),
    ("clone", "clone"),
    ("default", "default"),
    ("drop", "drop"),
    ("from", "from"),
    ("try_from", "tryFrom"),
    ("into", "into"),
    ("new", "new"),  // maps to constructor or static new()
    ("next", "next"),
    ("deref", "deref"),
];
```

**Types:** `PascalCase` stays (no conversion needed for struct/enum/trait names).

**Test functions:** The TS test string MUST be the exact Rust function name. `fn test_foo_bar` → `test('test_foo_bar', ...)`. Free-form test names are a porting error.

**Modules:** `mod foo` → check for `foo.ts` or `foo/index.ts`. `mod.rs` / `lib.rs` → `index.ts`.

### 1.4 Matching Algorithm

For each Rust file that has a MIRRORS TS file:

1. Parse both files into item lists
2. Group TS items by their parent class (methods) or top-level (functions, classes, interfaces)
3. For each Rust item:
   - Compute expected TS name via mapping rules
   - Search TS items for a match (by name + kind)
   - For `impl` block methods: search within the corresponding TS class
   - For trait methods: search within the TS class that implements the trait
   - Record: matched, unmatched, or ambiguous
4. For each matched pair, check for `// @<hash>` attestation
5. Compare body sizes (line count) and flag >50% shorter

### 1.5 Commit Hash Attestation

- Get the latest commit hash that modified the Rust file: `git log -1 --format=%h -- <file>`
- For each matched TS item with a `// @<hash>`, compare against the Rust file's hash
- States: **attested** (hash matches), **stale** (hash differs), **unattested** (no hash)

### 1.6 Output

```
=== packages/core/src/node.ts (MIRRORS: core/src/node.rs @abc1234) ===
  ✓ struct Node → class Node @abc1234
  ✓ fn next_entity_id → nextEntityId @abc1234
  ✗ fn fetch_from_peer → fetchFromPeer — NOT FOUND
  ⚠ fn commit_local_trx → commitLocalTrx @abc1234 (TS 15 lines vs Rust 45 lines)
  ↻ fn context → context @old1234 → abc1234 STALE

Summary:
  Files: 102 | Items: 1258 | Matched: 1100 | Missing: 158 | Attested: 900 | Stale: 50
```

### 1.7 File Discovery

- Walk `packages/*/src/**/*.ts` and `packages/*/__tests__/**/*.ts`
- Read line 1 for `// MIRRORS: ankurah/<path>.rs`
- Resolve Rust path relative to `ankurah-ts-support/`
- Group multiple TS files mirroring the same Rust file (e.g., `parser.ts` + `parser.test.ts` both mirror `parser.rs`)

---

## Phase 2: Skeleton Transpiler

Generate TS declaration structure from Rust source. Compare against existing code.

### 2.1 What it generates

For each Rust file, generate:
- `// MIRRORS: ankurah/<path>.rs` header
- `import` statements (from `use` statements, mapped via crate→package table)
- `class` declarations (from `struct` + its `impl` blocks, merged)
- `interface` declarations (from `trait`)
- `type` aliases (from `type`)
- `enum` class declarations (from `enum`, using `Enum<V>` pattern)
- Function signatures (from `fn`, with mapped names and types)
- `export` keywords (from `pub` visibility)
- Method stubs with `// TODO: implement` body

### 2.2 Type mapping

| Rust type | TS type |
|-----------|---------|
| `String` / `&str` | `string` |
| `bool` | `boolean` |
| `i16` / `i32` / `usize` | `number` |
| `i64` / `u64` | `bigint` or `number` (context-dependent) |
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
| `Box<dyn Trait>` | `Trait` (interface directly) |

### 2.3 Validation

Compare generated skeleton against existing TS file:
- Every generated declaration should have a match in the existing file
- Flag extra items in generated output (Rust items we missed porting)
- Flag extra items in existing TS (TS-only additions — should be annotated)

---

## Phase 3: Body Transpiler

Translate function bodies. Start with mechanical patterns.

### 3.1 Mechanical translations

| Rust pattern | TS output |
|-------------|-----------|
| `let x = expr;` | `const x = expr;` |
| `let mut x = expr;` | `let x = expr;` |
| `x.clone()` | `x.clone()` |
| `match expr { ... }` | `expr.match({ ... })` |
| `if let Some(x) = expr { ... }` | `if (expr !== null) { const x = expr; ... }` |
| `expr?` | `expr` (throws propagate) |
| `expr.unwrap()` | `expr!` or assertion |
| `vec![a, b, c]` | `[a, b, c]` |
| `HashMap::new()` | `new Map()` |
| `format!("...", args)` | `` `...${args}` `` |
| `println!("...")` | `console.log(...)` |
| `async { ... }.await` | `await ...` |
| `.iter().map(\|x\| ...)` | `.map(x => ...)` |
| `.iter().filter(\|x\| ...)` | `.filter(x => ...)` |
| `.collect::<Vec<_>>()` | (remove, arrays are default) |

### 3.2 Validation

For each generated function body, compare against existing TS:
- Exact match → transpiler is correct, porting is correct
- Transpiler differs from existing → investigate (transpiler bug or porting bug?)
- Use diff to show discrepancies

---

## Phase 4: Production Transpiler

### 4.1 File whitelist

Maintain a list of files managed by the transpiler:
```toml
[managed]
ankql = ["src/ast.rs", "src/error.rs", "src/selection/sql.rs"]
proto = ["src/auth.rs", "src/clock.rs"]
# ... gradually expand
```

### 4.2 Workflow

1. Rust source changes (new commit on `ankurah-ts-support`)
2. Run transpiler on all managed files
3. Diff generated TS against current TS
4. Auto-apply clean diffs, flag complex changes for review
5. Run test suite to validate

### 4.3 Override mechanism

For files/functions where the transpiler can't produce correct output:
```typescript
// @transpiler-override: <reason>
// The transpiler generates X but we need Y because <reason>
```

The transpiler skips items with this annotation and preserves the manual code.

---

## Project Structure

```
port/checker/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point
│   ├── rust_parser.rs    # syn-based Rust extraction
│   ├── ts_parser.rs      # deno_ast-based TS extraction
│   ├── name_map.rs       # Deterministic name mapping
│   ├── matcher.rs        # Item matching algorithm
│   ├── attestation.rs    # Commit hash checking
│   ├── reporter.rs       # Output formatting
│   └── transpiler/       # Phase 2+ (skeleton and body generation)
│       ├── mod.rs
│       ├── skeleton.rs   # Declaration generation
│       ├── body.rs       # Function body translation
│       └── types.rs      # Rust→TS type mapping
├── tests/
│   └── ...
```

## Dependencies

```toml
[dependencies]
syn = { version = "2", features = ["full", "parsing", "visit"] }
deno_ast = "0.44"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
anyhow = "1"
colored = "2"
```

## CLI

```
# Phase 1: Check parity
cargo run -- check [--package <name>] [--verbose] [--json]

# Phase 1: Check with attestation verification
cargo run -- check --verify-attestations [--package <name>]

# Phase 2+: Generate skeleton
cargo run -- transpile --skeleton <rust-file> [--output <ts-file>]

# Phase 2+: Compare generated vs existing
cargo run -- transpile --diff <rust-file>

# Phase 4: Full transpile of managed files
cargo run -- transpile --managed [--apply]
```

## Configuration

```toml
# port/checker/checker.toml
[paths]
rust_source = "../../ankurah-ts-support"
ts_source = "../../ankurah-ts/packages"

[crates]
# Crate name → TS package name
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
# Items gated with these features are skipped
skip = ["wasm", "uniffi"]

[name_overrides]
# Static name mapping exceptions beyond snake→camel
fmt = "toString"
serialize = "encode"
deserialize = "decode"
eq = "equals"
default = "default"
drop = "drop"
new = "new"
from = "from"

[managed_files]
# Phase 4: files the transpiler owns (start empty, grow gradually)
```
