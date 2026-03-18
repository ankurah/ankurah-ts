# ankurah-ts Transpiler — Specification

## Executive Summary

A Rust binary that transpiles Rust source code into TypeScript. It parses Rust with `syn`, generates TypeScript output, and diffs that output against the existing hand-ported TS files. **The existing port is the test suite for the transpiler** — discrepancies are either a transpiler bug or a porting bug, both valuable findings.

There is **no TS parser dependency**. The transpiler only needs to read Rust (via `syn`) and generate TS text. Validation is done by diffing generated output against existing files. The tool does not parse TypeScript ASTs.

**Phases:**
1. **Skeleton transpiler** — generate TS declarations (classes, interfaces, functions, imports, exports) from Rust. Diff against existing TS. No function bodies.
2. **Body transpiler** — translate function bodies using mechanical pattern matching. Diff against existing.
3. **Production transpiler** — managed file whitelist, auto-apply clean diffs, override annotations for manual exceptions.

**Dependencies:** `syn` (Rust parsing), `clap` (CLI), `walkdir` (file discovery), `anyhow` (errors), `colored` (terminal output). That's it.

---

## Phase 1: Skeleton Transpiler

Parse Rust, generate TS declaration structure, diff against existing.

### 1.1 Rust Extraction (via `syn`)

Parse each `.rs` file with `syn::parse_file()`. Extract:

| Rust item | `syn` type | Data to extract |
|-----------|-----------|-----------------|
| `fn` | `ItemFn` / `ImplItemFn` / `TraitItemFn` | name, visibility, is_async, params, return type, body span/line count |
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
- `#[cfg(feature = "wasm")]` / `#[cfg(feature = "uniffi")]` — skip item entirely (not ported to TS)
- `#[cfg(test)]` — mark as test-only (generates into `.test.ts` instead of `.ts`)
- `#[test]` / `#[tokio::test]` — mark function as test
- `#[derive(...)]` — record derived traits (informs what methods to generate: Clone→clone, Debug→toString, PartialEq→equals, Serialize/Deserialize→encode/decode)

**Context tracking:**
- Which `impl` block a method belongs to → methods are generated inside the corresponding TS class
- Which `mod tests` block a function belongs to → generates into the `.test.ts` file

### 1.2 Name Mapping

All mapping is deterministic.

**Functions:** `snake_case` → `camelCase` via mechanical conversion, plus static exceptions:

```rust
const STATIC_MAP: &[(&str, &str)] = &[
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
    ("new", "new"),
    ("next", "next"),
    ("deref", "deref"),
];
```

**Types:** `PascalCase` stays unchanged.

**Test functions:** Preserved as-is. `fn test_foo_bar` → `test('test_foo_bar', ...)`.

**Modules:** `mod foo` → `foo.ts` or `foo/index.ts`. `mod.rs` / `lib.rs` → `index.ts`.

### 1.3 TS Code Generation

For each Rust file, generate a complete TS file:

**Header:**
```typescript
// MIRRORS: ankurah/<crate>/src/<path>.rs
```

**Imports:** Generated from `use` statements using the crate→package mapping table.

**Structs → Classes:**
```rust
pub struct Node {
    pub id: EntityId,
    durable: bool,
}
```
→
```typescript
export class Node extends Struct {
  readonly id: EntityId;
  private durable: boolean;
}
```

**Enums → Enum classes:**
```rust
pub enum DeltaContent {
    StateSnapshot { state: StateFragment },
    EventBridge { events: Vec<EventFragment> },
}
```
→
```typescript
type DeltaContentV = {
  StateSnapshot: { state: StateFragment };
  EventBridge: { events: EventFragment[] };
};
export class DeltaContent extends Enum<DeltaContentV> {
}
```

**Traits → Interfaces:**
```rust
pub trait StorageEngine {
    async fn collection(&self, id: &CollectionId) -> Result<StorageCollection>;
}
```
→
```typescript
export interface StorageEngine {
  collection(id: CollectionId): Promise<StorageCollection>;
}
```

**Functions → Functions/Methods:**
```rust
pub fn next_entity_id(&self) -> EntityId { ... }
```
→
```typescript
nextEntityId(): EntityId {
  // TODO: implement
}
```

**impl blocks → merged into classes:**
All methods from `impl Node { ... }` and `impl Display for Node { ... }` are generated inside `class Node { ... }`.

**Visibility:** `pub` → `export` (top-level) or omit access modifier (class members default to public in TS). `pub(crate)` → no export, no access modifier. Private → `private`.

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

### 1.5 Validation (diff against existing)

The transpiler generates a TS file, then diffs it against the existing hand-ported TS file:

```
cargo run -- transpile --diff core/src/node.rs
```

Output:
```diff
--- generated (from Rust)
+++ existing (hand-ported)
@@ -15,6 +15,7 @@
 export class Node extends Struct {
   readonly id: EntityId;
+  readonly collections: CollectionSet;  // extra field in existing
   private durable: boolean;
```

Discrepancies mean:
- **Extra in generated** → Rust has something the TS port is missing
- **Extra in existing** → TS has something not in Rust (should be annotated as TS-ONLY or Divergence)
- **Different** → either the transpiler's mapping is wrong, or the port is wrong

### 1.6 Commit Hash Attestation

After a human verifies a function is correctly ported, they add `// @<hash>` on the preceding line:

```typescript
// @abc1234
nextEntityId(): EntityId {
```

The transpiler can check these:
- `git log -1 --format=%h -- <rust-file>` gets the current Rust commit
- If `@hash` differs from current → file has changed since verification → flag as stale

### 1.7 File Discovery

- Walk `ankurah-ts-support/<crate>/src/` to find all Rust source files
- Use the crate→package mapping to determine the target TS path
- For files with `mod.rs` / `lib.rs`, map to `index.ts`
- For `#[cfg(test)] mod tests { ... }`, generate into `<name>.test.ts`

---

## Phase 2: Body Transpiler

Translate function bodies using mechanical pattern matching.

### 2.1 Mechanical translations

| Rust pattern | TS output |
|-------------|-----------|
| `let x = expr;` | `const x = expr;` |
| `let mut x = expr;` | `let x = expr;` |
| `x.clone()` | `x.clone()` |
| `match expr { ... }` | `expr.match({ ... })` |
| `if let Some(x) = expr { ... }` | `if (expr !== null) { const x = expr; ... }` |
| `if let Enum::Variant(v) = expr { ... }` | `if (expr.is('Variant')) { const v = expr.value; ... }` |
| `expr?` | `expr` (throws propagate naturally) |
| `expr.unwrap()` | `expr!` or direct access |
| `vec![a, b, c]` | `[a, b, c]` |
| `HashMap::new()` | `new Map()` |
| `format!("...", args)` | `` `...${args}` `` |
| `println!("...")` | `console.log(...)` |
| `async { ... }.await` | `await ...` |
| `.iter().map(\|x\| ...)` | `.map(x => ...)` |
| `.iter().filter(\|x\| ...)` | `.filter(x => ...)` |
| `.collect::<Vec<_>>()` | (remove — arrays are the default) |
| `Ok(x)` | `return x` |
| `Err(e)` | `throw e` |
| `Some(x)` | `x` |
| `None` | `null` |

### 2.2 Validation

Same as Phase 1 — diff generated against existing. But now function bodies are included, so the diff covers logic as well as structure.

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
3. Diff generated TS against current TS
4. Auto-apply clean diffs, flag complex changes for review
5. Run test suite to validate

### 3.3 Override mechanism

For functions where the transpiler can't produce correct output:
```typescript
// @transpiler-override: <reason>
```

The transpiler preserves the existing code for items with this annotation.

---

## Project Structure

```
transpile/
├── Cargo.toml
├── checker.toml          # Configuration (paths, crate mapping, name overrides)
├── src/
│   ├── main.rs           # CLI entry point (clap)
│   ├── config.rs         # Read checker.toml
│   ├── rust_parser.rs    # syn-based Rust extraction
│   ├── name_map.rs       # Deterministic name mapping (snake→camel + static table)
│   ├── codegen.rs        # TS code generation from Rust items
│   ├── types.rs          # Rust→TS type mapping
│   ├── attestation.rs    # Commit hash checking
│   └── diff.rs           # Diff generated vs existing, formatted output
```

## Dependencies

```toml
[dependencies]
syn = { version = "2", features = ["full", "parsing"] }
proc-macro2 = { version = "1", features = ["span-locations"] }
clap = { version = "4", features = ["derive"] }
walkdir = "2"
anyhow = "1"
colored = "2"
toml = "0.8"
similar = "2"       # For text diffing
```

No TS parser dependency. The tool only reads Rust and generates TS text.

## CLI

```bash
# Generate TS skeleton for one file (output to stdout)
cargo run -- transpile core/src/node.rs

# Generate and write to the expected TS path
cargo run -- transpile core/src/node.rs --write

# Diff generated vs existing TS
cargo run -- diff core/src/node.rs

# Diff all files in a package
cargo run -- diff --package ankql

# Diff all files in all packages
cargo run -- diff --all

# Check attestation hashes
cargo run -- attest --check [--package <name>]
```

## Configuration

```toml
# transpile/checker.toml

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
```
