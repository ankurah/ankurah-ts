# Symbol Table & Type Resolution Spec

## Problem

The transpiler translates Rust function bodies without type context. `BodyTranslator` has only `self_type: &str`. This causes incorrect output when translation depends on the type of an expression:

- `self.inner.listeners` where `inner: Arc<Inner<T>>` — needs `.value` insertion between Arc and field access
- `x.iter()` — Map needs `Array.from(x)`, Vec/Array needs `[...x]`
- `Foo::Bar(val)` — is `Foo` an enum (variant constructor) or a struct (static method)?
- `let guard = self.data.write()` — guard is `RwLockWriteGuard<T>`, field access through it needs `.value`
- `static new(): Broadcast<T>` — static methods can't reference enclosing class type params in TS

These are not edge cases. They block transpilation of the signals crate (~400 tsc errors) and will block every subsequent crate.

## Architecture: Three-Phase Pipeline

Currently the transpiler does: parse file → extract items + translate bodies → emit TS. Body translation happens eagerly during extraction, before other files in the crate are even parsed.

The new pipeline separates extraction from body translation:

```
Phase 1: PARSE
  For each .rs file in the crate:
    syn::parse_file → syn::File
    Extract signatures → StructInfo, EnumInfo, ImplInfo, FnInfo, etc.
    Store raw syn::Block in FnInfo (clone, do NOT translate bodies yet)

Phase 2: REGISTER
  Build TypeRegistry from all extracted signatures:
    - All struct fields (name → ResolvedType)
    - All enum variants (name → VariantDef)
    - All impl method signatures (params, return types)
    - Provided types from config (Arc, RwLock, Result, etc.)
    - Cross-crate type mappings from config

Phase 3: TRANSLATE
  For each file, for each function body:
    Create ScopeStack seeded with [CrateScope, ModuleScope]
    Push ImplScope (if in an impl block) with self_type resolved to field types
    Push FnScope with param types
    Translate body with scope context available
    Push/pop BlockScopes for nested blocks during translation

Phase 4: EMIT (existing codegen, unchanged)
  For each file:
    Generate TS with resolved imports from FnInfo.body_ts (now populated)
```

### Where Phase 3 is triggered

A new function in `main.rs` iterates all `RustFile.impls[].methods[].body_ast` and `RustFile.functions[].body_ast`, translates them with the registry + scope, and populates `body_ts`. This happens between Phase 2 (register) and Phase 4 (emit/codegen). The codegen/emit layer is unchanged — it reads `body_ts` as before.

## Data Structures

All resolution-layer types live in a new `resolve.rs` module, separate from `types.rs` (which is the extraction layer).

### ResolvedType

Types are represented structurally, not as strings. This is the internal representation used for resolution during body translation. String representations continue to be used for TS emission.

```rust
#[derive(Debug, Clone, PartialEq)]
enum ResolvedType {
    /// string, number, boolean, void, never, Uint8Array
    Primitive(String),

    /// User-defined, system, or external named type: EntityId, Arc<T>, Inner<T>
    /// System types (Arc, RwLock, etc.) are Named — their behavior comes from
    /// TypeDef metadata (deref_field, methods), not from the ResolvedType variant.
    Named { name: String, args: Vec<ResolvedType> },

    /// Generic type parameter (unresolved T, K, V)
    Param(String),

    /// T[] (Vec<T> maps here, since TS syntax is T[] not Vec<T>)
    Array(Box<ResolvedType>),

    /// [A, B]
    Tuple(Vec<ResolvedType>),

    /// (params) => ret
    Fn { params: Vec<ResolvedType>, ret: Box<ResolvedType> },

    /// T | null (Option<T> maps here at resolution time)
    Nullable(Box<ResolvedType>),

    /// Could not resolve
    Unknown,
}
```

8 variants. `Map<K,V>` and `Set<T>` are `Named` types with TypeDefs, not dedicated variants — having both `Named("Map", [K,V])` and a separate `Map(K,V)` variant would create a split where body translation has to check both paths.

`Option<T>` resolves to `Nullable(T)` at parse time (matching the existing TS mapping where `Option<T>` → `T | null`). Option-specific methods (`.unwrap()`, `.is_some()`) are handled as special cases in `resolve_method`, not through the TypeRegistry, since there's no `Nullable` entry in the registry.

`Result<T, E>` stays as `Named("Result", [T, E])` since it has a real TS class in `@ankurah/base`.

`Box<T>` resolves to `T` directly (transparent — Box has no TS runtime representation).

### Generic Parameter Substitution

`ResolvedType` has a `substitute` method for replacing type parameters with concrete types. This is critical for method chain resolution — without it, `Mutex<Inner<T>>.lock()` would return `MutexGuard<T>` (unbound param) instead of `MutexGuard<Inner<T>>`.

```rust
impl ResolvedType {
    /// Replace type parameters according to a substitution map.
    /// Used when resolving method return types on generic types.
    fn substitute(&self, subst: &HashMap<&str, &ResolvedType>) -> ResolvedType {
        match self {
            ResolvedType::Param(name) =>
                subst.get(name.as_str()).map(|t| (*t).clone()).unwrap_or_else(|| self.clone()),
            ResolvedType::Named { name, args } => ResolvedType::Named {
                name: name.clone(),
                args: args.iter().map(|a| a.substitute(subst)).collect(),
            },
            ResolvedType::Array(inner) =>
                ResolvedType::Array(Box::new(inner.substitute(subst))),
            ResolvedType::Nullable(inner) =>
                ResolvedType::Nullable(Box::new(inner.substitute(subst))),
            ResolvedType::Tuple(elems) =>
                ResolvedType::Tuple(elems.iter().map(|e| e.substitute(subst)).collect()),
            ResolvedType::Fn { params, ret } => ResolvedType::Fn {
                params: params.iter().map(|p| p.substitute(subst)).collect(),
                ret: Box::new(ret.substitute(subst)),
            },
            _ => self.clone(),
        }
    }
}
```

### TypeDef

Every type — user-defined structs, user-defined enums, AND provided system types — gets a TypeDef in the registry:

```rust
struct TypeDef {
    name: String,
    kind: TypeKind,
    /// Fields accessible on instances of this type
    fields: Vec<(String, ResolvedType)>,
    /// Methods with return types (for chained type inference)
    methods: HashMap<String, MethodSig>,
    /// If accessing through this type requires an indirection in TS.
    ///   None         → not a deref type, look up fields directly
    ///   Some("")     → transparent deref (Box), unwrap to inner type, emit nothing
    ///   Some("value") → deref wrapper (Arc), emit .value, then access inner type's fields
    deref_field: Option<String>,
    /// Generic type parameter names (e.g., ["T"] for Arc<T>, ["K", "V"] for HashMap<K,V>)
    type_params: Vec<String>,
}

enum TypeKind {
    Struct,
    Enum { variants: Vec<VariantDef> },
    Trait,
}

struct VariantDef {
    name: String,
    fields: Vec<(String, ResolvedType)>,
}

struct MethodSig {
    params: Vec<(String, ResolvedType)>,
    ret: ResolvedType,
    is_static: bool,
}
```

Method keys in TypeDef use **Rust names** (snake_case). The `name_map` module handles Rust→TS name conversion during emission, separately from type resolution.

### TypeRegistry

The crate-wide type registry. Populated during Phase 2 from both parsed Rust sources AND config-declared provided types.

```rust
struct TypeRegistry {
    /// All known types: user-defined + provided + cross-crate
    /// Keyed by Rust type name.
    types: HashMap<String, TypeDef>,
}

impl TypeRegistry {
    /// Look up a type definition by name
    fn get(&self, name: &str) -> Option<&TypeDef>;

    /// Is this name an enum?
    fn is_enum(&self, name: &str) -> bool;

    /// Is this a valid variant of the given enum?
    fn is_variant(&self, type_name: &str, variant_name: &str) -> bool;

    /// Resolve a field access on a typed expression.
    /// Returns the field's type and the deref accessor to insert (if any).
    ///
    /// Algorithm:
    ///   1. Look up type's TypeDef
    ///   2. If field exists directly → return (field_type, None)
    ///   3. If TypeDef has deref_field → unwrap inner type (with generic substitution),
    ///      recurse from step 1, return (field_type, Some(accessor))
    ///   4. If no deref_field → return None
    ///
    /// Deref does NOT trigger for fields/methods defined on the wrapper itself.
    /// arc.clone() → clone is on Arc → no deref. arc.some_inner_field → deref.
    fn resolve_field(&self, ty: &ResolvedType, field: &str) -> Option<(ResolvedType, Option<String>)>;

    /// Resolve a method call on a typed expression.
    /// Returns the method's return type (with generic params substituted).
    ///
    /// Algorithm:
    ///   1. Look up type's TypeDef
    ///   2. If method exists on TypeDef → return return_type (with generic substitution)
    ///   3. If TypeDef has deref_field → unwrap inner type, recurse from step 1
    ///   4. If no deref_field → return None
    ///
    /// Generic substitution: when RwLock declares type_params=["T"] and method
    /// write returns "RwLockWriteGuard<T>", calling .write() on RwLock<Map<K,V>>
    /// substitutes T→Map<K,V>, returning RwLockWriteGuard<Map<K,V>>.
    fn resolve_method(&self, ty: &ResolvedType, method: &str) -> Option<ResolvedType>;
}
```

### ScopeStack

Variable bindings are tracked in a stack of scopes. Scopes are pushed on entry to impl/fn/block and popped on exit.

```rust
struct ScopeStack {
    scopes: Vec<Scope>,
}

struct Scope {
    kind: ScopeKind,
    bindings: HashMap<String, ResolvedType>,
}

enum ScopeKind {
    /// Crate-level: all types visible
    Crate,
    /// Per-file: use imports resolved
    Module { use_imports: HashMap<String, String> },
    /// Per impl block: self_type bound
    Impl { self_type: ResolvedType },
    /// Per function: params bound
    Fn,
    /// Per { } block: let-bindings
    Block,
    /// Closure: captures from enclosing scope
    Closure,
}

impl ScopeStack {
    /// Push a new scope
    fn push(&mut self, scope: Scope);

    /// Pop the innermost scope
    fn pop(&mut self) -> Option<Scope>;

    /// Resolve a variable name, walking from innermost to outermost scope.
    /// Returns the first match (innermost scope wins — handles shadowing correctly,
    /// including same-block shadowing via HashMap::insert overwrite).
    fn resolve(&self, name: &str) -> Option<&ResolvedType>;

    /// Find the nearest Impl scope's self_type
    fn self_type(&self) -> Option<&ResolvedType>;

    /// Bind a variable in the current (innermost) scope.
    /// If the name already exists in this scope, it is overwritten (Rust shadowing).
    fn bind(&mut self, name: String, ty: ResolvedType);

    /// Bind a destructured pattern: let (a, b) = tuple_expr, let Foo { x, y } = foo_expr.
    /// Recursively binds each sub-pattern to the corresponding field/element type.
    fn bind_pattern(&mut self, pat: &syn::Pat, ty: &ResolvedType, registry: &TypeRegistry);

    /// Resolve a type name through use-imports (for aliased type lookups).
    /// Checks Module scope use_imports, returns canonical name.
    fn resolve_type_name(&self, name: &str) -> Option<&str>;
}
```

### BodyTranslator (extended)

```rust
pub struct BodyTranslator<'a> {
    pub registry: &'a TypeRegistry,
    pub scopes: ScopeStack,
}
```

`self_type: &str` is removed. It becomes a binding in the ImplScope (`this → ResolvedType`). The old free functions (`translate_expr`, `translate_block`, `translate_pat`) are kept as compatibility shims that create a BodyTranslator with an empty registry and scopestack, ensuring no regression for code paths that haven't been updated yet (match_expr, control_flow, macros). These shims are removed once all callers are updated to thread `&BodyTranslator`.

## System Types in Config

System types are foundational runtime types (Arc, RwLock, Vec, etc.) whose shapes are
declared in config so the transpiler can resolve through them. All have TS implementations
in `@ankurah/base/std/`. These are distinct from `[provided_impls]`, which are subject-code
types whose implementations are hand-ported in `*.provided.ts` files.

---

System types (Arc, RwLock, Mutex, Result, Option, Box, etc.) are declared in `transpile.toml` and loaded into the TypeRegistry alongside parsed Rust types. They are not special-cased in body translation code.

Method return types in the config are strings that are parsed into `ResolvedType` at config load time by a `parse_type_string` function. This parsing is syntactic (builds a tree from angle-bracket syntax) and does not require the TypeRegistry to exist yet.

```
parse_type_string grammar:
  type := name ("<" type ("," type)* ">")?
        | type "| null"
        | type "[]"
        | name

  Bare single uppercase letter (T, K, V) → Param
  Primitives (string, number, boolean, void, never, Uint8Array) → Primitive
  Everything else → Named
```

```toml
[system_types]

[system_types.Arc]
deref_field = "value"
type_params = ["T"]
methods = { clone = "Arc<T>", downgrade = "Weak<T>" }

[system_types.Weak]
type_params = ["T"]
methods = { upgrade = "Arc<T> | null", clone = "Weak<T>" }

[system_types.Mutex]
type_params = ["T"]
methods = { lock = "MutexGuard<T>" }

[system_types.MutexGuard]
deref_field = "value"
type_params = ["T"]

[system_types.RwLock]
type_params = ["T"]
methods = { read = "RwLockReadGuard<T>", write = "RwLockWriteGuard<T>" }

[system_types.RwLockReadGuard]
deref_field = "value"
type_params = ["T"]

[system_types.RwLockWriteGuard]
deref_field = "value"
type_params = ["T"]

[system_types.RefCell]
type_params = ["T"]
methods = { borrow = "Ref<T>", borrow_mut = "RefMut<T>" }

[system_types.Ref]
deref_field = "value"
type_params = ["T"]

[system_types.RefMut]
deref_field = "value"
type_params = ["T"]

[system_types.Box]
deref_field = ""  # transparent — unwrap to inner type, emit nothing
type_params = ["T"]

[system_types.Option]
type_params = ["T"]
# Option methods handled as special cases on Nullable, not through TypeDef
# (Option<T> resolves to Nullable(T), which has no TypeDef entry)

[system_types.Result]
type_params = ["T", "E"]
methods = { unwrap = "T", expect = "T", is_ok = "boolean", is_err = "boolean", map = "Result<U, E>", map_err = "Result<T, F>" }

[system_types.Vec]
type_params = ["T"]
methods = { len = "number", push = "void", pop = "T | null", iter = "T[]", clone = "T[]" }

[system_types.HashMap]
type_params = ["K", "V"]
methods = { get = "V | null", insert = "void", len = "number", iter = "Array<[K, V]>", contains_key = "boolean" }

[system_types.BTreeMap]
type_params = ["K", "V"]
methods = { get = "V | null", insert = "void", len = "number", iter = "Array<[K, V]>", contains_key = "boolean" }

[system_types.HashSet]
type_params = ["T"]
methods = { insert = "void", contains = "boolean", len = "number", iter = "T[]" }
```

Note: Method return type strings represent TS-side semantics (what the TS expression produces), not Rust-side types. E.g., `Vec.iter = "T[]"` because the TS translation of `.iter()` produces `T[]` (via spread), even though Rust's `.iter()` returns an iterator.

Note: Primitive type method dispatch (`.len()` on strings, etc.) stays in `body.rs`'s existing `translate_method_call` heuristics. The registry handles types that need structural resolution (deref, enum detection, field lookup, method chaining).

## Type Resolution During Body Translation

### Field access: `self.inner.listeners`

1. `self` → ScopeStack resolves to ImplScope self_type → `Named("Broadcast", [Param("T")])`
2. Look up `Broadcast` in TypeRegistry → fields include `inner: Named("Arc", [Named("Inner", [Param("T")])])`
3. `inner` is not a field on `Arc`'s TypeDef → check `deref_field` → `Some("value")`
4. Emit `.value`, unwrap to `Named("Inner", [Param("T")])` (with generic substitution from Arc's type_params)
5. Look up `Inner` in TypeRegistry → fields include `listeners: Named("RwLock", [Named("Map", [Primitive("number"), Named("BroadcastListener", [Param("T")])])])`
6. Final TS: `this.inner.value.listeners`

### Method call: `self.data.write().splice(...)`

1. `self.data` → resolves to `Named("RwLock", [Named("Map", [...])])`
2. `.write()` → look up `RwLock`'s TypeDef → method `write` returns `RwLockWriteGuard<T>`
3. Generic substitution: RwLock's `T` = `Named("Map", [...])` → return `Named("RwLockWriteGuard", [Named("Map", [...])])`
4. `.splice()` → look up `RwLockWriteGuard`'s TypeDef → `splice` not found → check `deref_field` → `Some("value")` → unwrap to `Named("Map", [...])` → emit `.value`
5. Final TS: `this.data.write().value.splice(...)`

### Method on wrapper: `self.inner.clone()`

1. `self.inner` → `Named("Arc", [Named("Inner", [Param("T")])])`
2. `.clone()` → look up `Arc`'s TypeDef → method `clone` exists on Arc → returns `Arc<T>` → no deref
3. Final TS: `this.inner.clone()` (no `.value` inserted)

### Enum detection: `Signal::Constant(v)`

1. `Signal` → `registry.is_enum("Signal")` → true
2. `Constant` → `registry.is_variant("Signal", "Constant")` → true
3. Emit: `new Signal('Constant', { _0: v })` (variant constructor pattern)

If `Signal` were a struct, `registry.is_enum` returns false → emit as static method call instead.

### Let-binding inference: `let guard = self.data.write()`

1. Translate RHS → resolve type as above → `Named("RwLockWriteGuard", [Named("Map", [...])])`
2. `scopes.bind("guard", resolved_type)` in current BlockScope
3. Subsequent `guard.field` access resolves through the binding

### Nullable (Option) method resolution

`Option<T>` resolves to `Nullable(inner_T)` at parse time. Method calls on nullable types are handled as special cases, not through the TypeRegistry:

- `.unwrap()` / `.expect()` → strips Nullable, returns inner type
- `.is_some()` / `.is_none()` → `Primitive("boolean")`
- `.map(f)` → `Nullable(f's return type)` (if resolvable, else Unknown)

### Tier 2 chains resolve incrementally

Tier 2 resolution is per-expression-node. Chains of arbitrary length resolve as the expression tree is walked — each `Expr::MethodCall` resolves its receiver by recursing into `expr()`, which resolves the receiver's receiver, etc. So `self.data.write().unwrap().listeners` resolves step by step through the recursive call.

## Type Inference Depth

### Tier 1 — Direct (always infer):

| Pattern | Inferred type |
|---|---|
| `let x: T = ...` | Annotated T |
| `let x = Type::new(...)` | Named("Type", [...]) |
| `let x = Type { ... }` | Named("Type", [...]) (struct construction) |
| `let x = Enum::Variant(...)` | Named("Enum", [...]) |
| `let x = literal` | Primitive |
| `let x = "foo".to_string()` | Primitive("string") |
| `let x = vec![...]` | Array(element_type) |
| `let x = expr.clone()` | Same type as expr |
| `let x = some_fn(args)` | Return type from FnInfo (registry lookup) |

### Tier 2 — Method return type lookup (infer via registry):

| Pattern | Resolution |
|---|---|
| `let x = self.method()` | Look up method in ImplScope → return type |
| `let x = obj.method()` | Resolve obj type, look up method in registry (with generic substitution) |
| `let x = self.field` | Look up field type in registry |
| `let x = self.data.write()` | Chain: resolve self.data → RwLock<T>, resolve .write() → RwLockWriteGuard<T> |

### Tier 3 — Skip (fall back to Unknown):

| Pattern | Why |
|---|---|
| Closure parameter types | Requires Hindley-Milner |
| Generic method instantiation | Requires unification |
| Chained iterator combinators | Requires generic propagation |

When type is Unknown, the body translator falls back to current heuristic behavior (no regression).

## Static Method Generic Handling

When a static method in `impl<T> Foo<T>` references the impl's type param `T`, the emitted TS method needs its own generic param (since TS static methods can't access instance generics):

```rust
// Rust
impl<T> Foo<T> {
    fn new(val: T) -> Self { ... }
}

// TS (correct)
static new<T>(val: T): Foo<T> { ... }
```

During body translation: when creating a FnScope for a static method, the impl's type params are added as the method's own generic params in the emitted signature. The scope still has access to the ImplScope for type lookups, but the emission layer adds `<T>` to the method signature.

## Impl Trait Method Placement

When building the TypeRegistry from `ImplInfo`:
- `impl MyTrait for MyType` where `MyType` is defined in this crate → merge methods into `MyType`'s TypeDef
- `impl ForeignTrait for MyType` → merge methods into `MyType`'s TypeDef
- `impl MyTrait for ForeignType` → do NOT merge (can't add methods to String, Vec, etc.). These become standalone functions or static factory methods.

## Known Limitations

These are acknowledged gaps that are either low-priority or require significantly more infrastructure:

1. **Trait method resolution order** — when two traits provide the same method name and both are in scope, Rust uses trait bounds to disambiguate. The transpiler picks the first match. Mitigated by the existing `name_map` handling of common traits (Display→toString, Iterator methods, etc.).

2. **`impl Trait` return types** — `fn foo() -> impl Iterator<Item=T>` doesn't expose the concrete type. Falls back to Unknown.

3. **Closure parameter types** — `|x| x.method()` — type of `x` requires Hindley-Milner inference. Falls back to Unknown.

4. **Match arm reference stripping** — `match &enum_val { Variant(x) => ... }` — `x` is `&T` in Rust but the pattern translator strips the reference. If x's type is inferred for downstream use, it should be T not &T. Low priority since TS has no references.

5. **Auto-ref/auto-deref in method dispatch** — `x.method()` in Rust might resolve to `(&x).method()` via a trait impl on `&T`. Not modeled.

## Changes to Existing Code

### NEW: resolve.rs

New module containing: `ResolvedType`, `TypeDef`, `TypeKind`, `VariantDef`, `MethodSig`, `TypeRegistry`, `ScopeStack`, `Scope`, `ScopeKind`, `parse_type_string`.

### extract.rs

- `extract_fn_with_body` stops calling `body::translate_block`. Stores `body_ast: Option<syn::Block>` (cloned/owned) instead of populating `body_ts`.
- `extract_fn_with_body_and_self` is removed (self_type moves to scope).

### types.rs

- `FnInfo` gains `body_ast: Option<syn::Block>` alongside existing `body_ts: Option<String>`.
- `body_ast` is populated during Phase 1 (extraction). `body_ts` is populated during Phase 3 (translation).
- After Phase 3, `body_ast` can be dropped (set to None) to free memory.

### body.rs

- `BodyTranslator` gains `registry: &TypeRegistry` and `scopes: ScopeStack`.
- `self_type: &str` removed.
- Field access (`Expr::Field`) resolves receiver type, checks `deref_field`, inserts accessor if needed.
- Method calls (`Expr::MethodCall`) resolve receiver type for dispatch and deref suppression.
- Let bindings (`Stmt::Local`) infer type from RHS and bind in current scope via `scopes.bind()` or `scopes.bind_pattern()`.
- Enum variant detection uses `registry.is_enum()` + `registry.is_variant()` instead of PascalCase heuristic.
- Free functions `translate_expr`, `translate_block`, `translate_pat` are kept as compatibility shims (create BodyTranslator with empty registry + scopestack). Removed once all callers are updated.

### main.rs `batch_generate`

- Phase 1: parse + extract signatures (no body translation). Clone `syn::Block` into FnInfo.body_ast.
- Phase 2 (new): build TypeRegistry from all StructInfo/EnumInfo/ImplInfo + config system_types.
- Phase 3 (new): translate all bodies with registry + scope. Populate FnInfo.body_ts.
- Phase 4 (existing codegen): generate TS with resolved imports from FnInfo.body_ts.

### codegen.rs, emit.rs

Unchanged. These consume `FnInfo.body_ts` which is still a String.

### control_flow.rs, match_expr.rs

These currently call `translate_expr` (standalone free function). They need to be updated to accept `&BodyTranslator` to access scope context. Can be done incrementally — the free function shims provide backward compatibility during the transition.

Long-term: `match_expr.rs`'s string-level identifier replacement (`replace_identifier`) should be replaced with scope-based binding (push BlockScope per match arm, bind destructured variables).

### name_map.rs

- New function: `resolve_type(syn::Type, registry: &TypeRegistry) -> ResolvedType` — parallel to existing `map_type(syn::Type) -> String`. Note: takes registry (for named type lookup), not scope (scope is for variable names, not type names).
- `map_type` continues to exist for emission.

### config.rs

- Parse `[system_types]` section from transpile.toml.
- Use `parse_type_string` to convert method return type strings to `ResolvedType`.
- Produce `Vec<TypeDef>` from config for seeding the TypeRegistry.

## Validation Strategy

### Before starting implementation

Capture current transpiler output for proto and ankql as golden files. This is the regression baseline.

### After implementation

1. **Regression**: diff proto and ankql output against golden files. Any difference is a bug (the new pipeline should produce identical output for these crates, since the type-aware resolution falls back to Unknown → heuristic behavior for patterns that don't need it).

2. **Signals**: run transpiler against signals crate, count tsc errors. The deref insertion + enum detection + method chain resolution should eliminate a specific, countable class of errors. Track error count reduction.

3. **Enum detection**: verify no false positives from the heuristic/registry interaction. The registry check is gating: if in registry and is enum → variant constructor; if in registry and is struct → static method; if not in registry → fall back to current PascalCase heuristic.
