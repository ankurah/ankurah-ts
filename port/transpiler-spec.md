# ankurah-ts Transpiler — Specification

**Written 2026-02/03. Corrected 2026-09-02** where a later ruling repealed the
premise a section rested on; each correction says what changed and is dated
inline, and [retractions-2026-09-02.md](retractions-2026-09-02.md) lists them in
one table. **How the transpiler decides what Rust type an expression has — the
question most of the translation turns on — is specified separately and in
detail in [../transpile/SYMBOL-TABLE-SPEC.md](../transpile/SYMBOL-TABLE-SPEC.md),
which is newer than this file and wins where the two disagree.**

## Executive Summary

A Rust binary that transpiles Rust source code into TypeScript. The pipeline:

```
Rust source → syn parse → classify items → route to transform modules → TS text → write to file
```

The **transform layer** is where all translation rules are codified. `syn` handles Rust parsing. TS output is currently string-based (validated via spike); OXC AST generation is a future upgrade path.

**The existing port is the test suite for the transpiler.** Every file we've already ported is expected output. Run the transpiler, write the output, `git diff` to see discrepancies. Discrepancies are either a transpiler bug or a porting bug — both valuable.

**The transpiler does no diffing itself.** It writes files. `git diff` is the validation tool.

**Phases** (written when all three were ahead of us; as of 2026-09-02 the first two are built and the third is not):
1. **Skeleton transpiler** — generate TS declarations (classes, interfaces, functions, imports, exports) from Rust. No function bodies. *(Built.)*
2. **Body transpiler** — translate function bodies via AST-level pattern matching. *(Built, and the work in flight is the type resolution it depends on.)*
3. **Production transpiler** — managed file whitelist, whole-crate batch processing. *(Whole-crate batch processing is built; the managed-file whitelist is not.)*

**Dependencies:** `syn` (Rust parsing), `quote` (token stream), `proc-macro2` (span locations), `clap` (CLI), `walkdir` (file discovery), `anyhow` (errors), `toml` (config).

## Required Context

The transpiler is NOT a generic Rust→TS tool. It targets the specific ankurah-ts architecture. The transform layer must be aware of and implement:

**Why no general tool exists to use instead (recorded 2026-09-02).** A survey of
the prior art found nothing to adopt, and the reasons are structural rather than
accidental: WebAssembly absorbed the demand for running Rust in a browser, so
nobody needed a source translator; translation research flows the other way,
into Rust rather than out of it; and the four problems that make the job hard —
trait resolution, monomorphization, macro expansion, and drop timing — push any
general tool into rustc's internals. Translating one known codebase is exactly
what lets this transpiler decline all four: it can be told the traits ankurah
uses, refuse to expand macros, and read drop timing off ankurah's own types.

| Document | What it governs |
|----------|----------------|
| `packages/base/src/` | The ownership type hierarchy: `AkObject`, `Struct`, `Enum<V>`, `Drop`, `Arc<T>`, `Weak<T>`, `Borrow<T>`, `BorrowMut<T>`, `Mutex<T>`, `RefCell<T>`, `AsyncMutex`. These are the types the transpiler emits — not generic TS classes. |
| `port/decisions.md` | Architectural decisions: bincode-only wire format, Yjs (not Yrs), `Result` as a returned value, `defineModel()` for derive macros, bun workspaces, etc. |
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
| `#[cfg(feature = "wasm")]` | Attribute | Skip entirely — but see the note below on `cfg` |
| Everything else | — | Default transform |

**On `cfg` (2026-09-02).** "Skip anything wasm-gated" is too blunt now that the
browser is a primary target. The transpiler is moving to evaluate `cfg` as
ankurah's wasm32 configuration — `target_arch = "wasm32"` plus the `singlethread`
feature — so that the browser branch ankurah already maintains and tests is the
branch the port follows. Today `cfg.rs` evaluates every non-feature predicate to
false, which keeps each `not(target_arch = "wasm32")` branch and drops the
wasm32 one: the opposite of the intent. What stays skipped either way are the
`wasm-bindgen` and `JsValue` bridge modules, which convert Rust values into
JavaScript ones and have nothing to say to a TypeScript port.

### Drop Analysis

The transpiler includes a **transitive Drop ownership analyzer** (`drop-analysis` command). It walks all `.rs` files, finds types with `impl Drop`, then computes the transitive closure — which types contain Drop types through their fields.

**Validated results (full ankurah codebase):**
- 14 types with direct `impl Drop`
- 105 types that transitively contain Drop types
- 332 pure value types (no transitive Drop)

**Current approach (corrected 2026-09-02):** every type that Rust would run drop
glue for extends `AkObject` and is leak-tracked. The exception is not an
optimization but a rule: a Rust `Copy` type cannot implement `Drop`, so it
carries no drop glue at all — the emitter gives it no `drop()` method and no
registry entry, whatever class shape it picks. The runtime tests for the absence
of drop glue rather than for "is it a primitive", which is what makes re-storing
a `Copy` value legal while re-storing an owned object the container already
holds is fatal.

The retracted sentence said the analysis could let the transpiler "skip `using`
declarations" for the 332 value types. There are no `using` declarations to
skip — Hermes refuses them — and skipping drop tracking for a type merely
because nothing it contains implements `Drop` would lose the leak detection that
makes a forgotten value visible.

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

**`impl Drop for T` → `class T extends Drop`** with a `protected override onDrop()` stub. Corrected 2026-09-02: never an override of `drop()`, which is `AkObject`'s template — see `port/ownership.md`.

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
| `Result<T, E>` | `Result<T, E>` (a returned value; corrected 2026-09-02) |
| `HashMap<K,V>` / `BTreeMap<K,V>` | `Map<K,V>` |
| `HashSet<T>` / `BTreeSet<T>` | `Set<T>` |
| `Arc<T>` | `Arc<T>` |
| `Weak<T>` | `Weak<T>` |
| `Mutex<T>` | `Mutex<T>` |
| `RwLock<T>` | `RwLock<T>` (its own type; corrected 2026-09-02) |
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

Hardcoded files still participate in drift detection — the transpiler knows about them and flags when the Rust source changes. But it does not attempt to regenerate the TS. It does read them for their declarations, so that the rest of the crate can resolve the types they define; it just does not emit them.

**When a file may be provided instead of transpiled (recorded 2026-09-02).** The
criterion is: **macros, out-of-family code, platform bindings, and files that
are simply too awkward or too hard to get working correctly through the
transpiler.** The last of those is deliberate and Daniel's own wording — an
escape hatch that exists so that one stubborn file cannot hold up a crate — with
the equally deliberate condition that provided files stay infrequent. Every one
of them is a file the port maintains by hand forever, so each needs a reason
written down next to it. A provided file is named `.provided.ts` and still obeys
the ownership contract, because the cascade will walk whatever it hands back.

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
| `Expr::Try { expr }` | check the `Result`: return the `Err` onward, drop the `Ok` wrapper. Corrected 2026-09-02; the old rule discarded the `Result` object, which leaks it |
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

**Rewritten 2026-09-02.** The transpiler used to be specified as generating
`using` declarations for block-scoped values. Hermes refuses to run `using`, so
what it emits instead is explicit:

- a value the block owns is dropped in a `finally`;
- a guard temporary is dropped at the end of the statement that produced it, and
  listed again in the enclosing `finally` — which is why a guard's second drop
  is deliberately a no-op and every other second drop is fatal;
- `impl Drop for T` becomes `protected override onDrop()`, never an override of
  `drop()`;
- a `Copy` type gets no drop glue at all.

`port/ownership.md` is the contract in full, and it is what the emitted code is
checked against at run time.

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

**Corrected 2026-09-02.** The listing here named `skeleton.rs` and
`attestation.rs`, neither of which was ever written. `transpile/src/` is the
authoritative listing; these are the areas it is organised into:

```
transpile/
├── Cargo.toml
├── transpile.toml         # Configuration (paths, crate mapping, system types, provided impls)
├── SYMBOL-TABLE-SPEC.md   # How types are resolved — the newer, deeper spec
├── src/
│   ├── main.rs            # CLI entry point (clap): drop-analysis | skeleton | batch
│   ├── extract.rs         # syn parse → signatures, items, bodies kept as syn::Block
│   ├── registry/          # module tree, name resolution, type and member lookup
│   ├── ty/                # the structural type representation and substitution
│   ├── infer/             # expression typing: scopes and context
│   ├── body.rs            # statement and expression translation
│   ├── control_flow.rs    # if / else / if-let
│   ├── match_expr.rs      # match arms
│   ├── macros.rs          # targeted macro handling (never expansion)
│   ├── name_map/          # deterministic naming and the TS shape of a type
│   ├── native_types/      # per-type method translation (Arc, Vec, HashMap, ...)
│   ├── bincode_module.rs  # encode/decode generated from the field layout
│   ├── ownership.rs       # where drops go
│   ├── drop_analysis.rs   # transitive Drop ownership analysis
│   ├── cfg.rs             # cfg evaluation
│   ├── config.rs          # read transpile.toml
│   ├── diag.rs            # diagnostics — the transpiler refuses rather than guesses
│   └── emit.rs, codegen.rs, imports.rs, types.rs
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

**Corrected 2026-09-02.** The three commands below are the ones that exist. The
`transpile --crate`, `transpile --all` and `attest --check` commands listed here
before were never built; whole-crate work is what `batch` does, and attestation
checking lives in `port/check-attestations.ts` on the TypeScript side.

```bash
# Analyze transitive Drop ownership for a crate
cargo run -- drop-analysis ../ankurah-ts-support/proto/src

# Generate TS for a single file (stdout)
cargo run -- skeleton ../ankurah-ts-support/proto/src/data.rs --crate-path proto/src/data.rs

# Transpile a whole crate into an output directory
cargo run -- batch ../ankurah-ts-support/ankql/src <out-dir> --crate-name ankql
```

`batch` prints a `DIAGNOSTICS crate=... total=... undeclared=...` line. Those
counts are the coverage metric and are pinned by `transpile/tests/diagnostics_budget.toml`.

## Configuration

```toml
# transpile/transpile.toml

[paths]
rust_source = "../ankurah-ts-support"
ts_target = ".."  # ankurah-ts root (packages/ is under this)

[crates]
# Corrected 2026-09-02: this is the crate scope from port-runbook.md. The live
# transpile.toml still maps ankurah-storage-postgres, ankurah-websocket-server
# and the tokio ankurah-websocket-client, which are out of scope, and maps the
# tokio websocket client where the browser one belongs. Bringing the file into
# line with this list is a transpiler change, not a doc change.
ankql = "ankql"
"ankurah-proto" = "proto"
"ankurah-signals" = "signals"
"ankurah-core" = "core"
"ankurah-storage-common" = "storage-common"
"ankurah-storage-sqlite" = "storage-sqlite"
"ankurah-storage-indexeddb-wasm" = "storage-indexeddb"
"ankurah-websocket-client-wasm" = "connector-websocket"
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
