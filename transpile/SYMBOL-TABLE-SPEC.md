# Type Resolution Spec

Rewritten 2026-09-02 against transpiler commit `f602831` and the corpus measurements
taken that day. Supersedes the March 2026 version of this file, whose still-valid
parts are folded into section 2.

## 1. What this is for

The transpiler emits TypeScript by walking the syn AST of ankurah's Rust source.
Almost every translation decision depends on the Rust type of the expression at
hand, and Rust does not write that type down at the use site:

- `self.inner.listeners` where `inner: Arc<Inner<T>>` needs a `.value` between
  `inner` and `listeners`, one per non-transparent deref step.
- `x.iter()` translates one way for a `Vec`, another for a `HashMap`, a third for
  a `RwLockReadGuard<HashMap<..>>`.
- `listener.into_broadcast_listener()` on a closure resolves through a blanket
  `impl<F: Fn(T)> IntoBroadcastListener<T> for F`; which impl is hit decides what
  TypeScript is emitted.
- `let v = x.values().cloned().collect::<Vec<_>>()` needs `Iterator::Item` at each
  step to type `v`, and `v`'s type decides every later use of it.
- `f()?` where `f` returns `Result<T, DecodeError>` inside a function returning
  `Result<T, anyhow::Error>` goes through `From`; the emitted code has to call it.
- Drop placement (see the ownership memo) needs to know which locals and
  temporaries are guards or otherwise droppable, which is a type question.

This engine exists to answer those questions for ankurah's code, and to refuse
to answer rather than guess. It is deliberately not a general Rust type checker.
It covers the constructs ankurah uses, measured in section 3, and any construct
outside that set stops the run with a diagnostic naming the Rust location.

Rulings this spec is bound by (Daniel, 2026-03 and 2026-09-02):

- Built in-house, on stable Rust, on syn. No rustc, no rust-analyzer, no MIR in
  the product. rust-analyzer's answers are used once as a test oracle (section 6)
  and nowhere else.
- No macro expansion. The engine supplies the types that the transpiler's
  targeted macro handling needs (section 4.10); it never reads expanded code.
- Fail loudly. A site the engine cannot type is an error, never a heuristic.
- The registry is not flat: same-leaf-name types in different modules resolve
  separately; a crate type can never evict a system type.
- Types are structural values, never TypeScript strings, until emission.
- Ownership of this spec is delegated to the supervising agent. Daniel's
  acceptance criterion, in full: "I don't reeeeally care what goes into the
  sausage, as long as it's tasty - that means typescript code works, implements
  ankurah protocols and behaviors faithfully and correctly, and mostly
  automatically as the upstream changes, with hopefully only occasional
  intervention necessary. ... if the CI process doing the port occasionally
  breaks because the upstream introduced some incompatible change, no big deal.
  If it's suprise mystery slop, that's a very big deal."
- `.provided.ts` is not reserved for macros and out-of-family code: "it's not
  just macros and out-of-family code - there might also be files that are just
  awkward or too hard to get working correctly with the transpiler. they should
  be infrequent, but available as an option."
- The runtime polyfills in `packages/base` are held to their Rust counterparts'
  semantics: "The point of the polyfills is to act like their rust
  counterparts... they must behave identically, or be replaced with some
  equivalent that does". So `Mutex<T>`, `RwLock<T>` and `RefCell<T>` drop their
  contents when they are dropped, and dropping one while a guard is outstanding
  is a fatal error: that is impossible in Rust, so in the port it can only mean
  the transpiler emitted the wrong scope.

## 1a. Crate scope and target environments

For: the engine is bounded to what ankurah's code does, and ankurah has crates
whose backends do not exist in the environments the port targets. Daniel: "It
makes no sense to port the postgres implementation to typescript for browser or
RN/expo. maybe for node, but node isn't a primary target. bottom line,
transpile things that are environment appropriate." The primary targets are the
browser and React Native/Expo; node is not a primary target.

| crate | disposition |
|---|---|
| `proto`, `ankql`, `signals`, `core` | transpile; this is the corpus measured in section 3 |
| `storage-common` | transpile: planner, predicates, bounds, sorting, no platform dependencies. It joins the corpus and the oracle |
| `storage-sqlite` | transpile the SQL builder, value, error, and the engine module. The rusqlite binding in `connection.rs` becomes a provided file exposing a small driver interface, and rusqlite's types get stub declarations the way std does (4.4) |
| `storage-indexeddb-wasm` | transpile, for the browser. The web-sys glue resolves through stub declarations (4.4) mapped to the IndexedDB API |
| `websocket-client-wasm` | transpile. The web-sys `WebSocket` client is the browser and React Native client |
| `connector-local-process`, `ankurah` (the facade crate) | transpile |
| `storage-postgres`, `websocket-server` (axum), `websocket-client` (tokio-tungstenite), `sled`, `derive`, `tests-wasm`, `examples` | out of scope |

The runbook's exclusion table has the websocket client backwards: it excludes
`connectors/websocket-client-wasm` and keeps the tokio-tungstenite
`websocket-client`, and `transpile.toml [crates]` maps the tokio crate for the
same reason. The doc retraction step corrects both.

cfg configuration: the transpiler evaluates `cfg` as ankurah's wasm32
configuration, `target_arch = "wasm32"` plus the `singlethread` feature, so the
browser code paths ankurah already maintains and tests are the source of truth
for the port. Today `transpile.toml` declares only the feature, and `cfg.rs`
evaluates every non-feature predicate to false, which keeps each
`not(target_arch = "wasm32")` branch and drops the wasm32 one.

Target-environment breakout, Daniel's proposal: where a crate's backend differs
by environment, one transpiled package holds all of the ankurah logic and one
thin hand-written package per environment provides only the driver, named crate
first and environment second: `storage-sqlite-expo` over expo-sqlite and
`storage-sqlite-node` over better-sqlite3, "which I don't really care about, but
those are both different sqlite backends". They are separate packages rather
than provided files inside one package so that an application pulls in exactly
one driver dependency. Both drivers have synchronous APIs, so the transpiled
engine stays synchronous. sqlite is the only crate differentiated this way
today; the existing `storage-expo-sqlite` and `storage-better-sqlite3` packages
are renamed at the dispositions step, and node stays outside the primary CI
gate.

A crate in scope can therefore have modules that are not transpiled at all:
replaced by a provided file, as `connection.rs` is, or supplied by an
environment package. The transpiler needs a per-crate notion of an externalized
module, so that the `mod x;` declaration and any `pub use x::` re-export naming
it are handled deliberately, the module not emitted and its name resolved to the
replacement, rather than erroring on a module whose source it never read.

## 2. What exists today (commit f602831)

Measured by reading `transpile/src/resolve.rs` (1,094 lines), `type_context.rs`
(284), `types.rs` (138), `name_map.rs`, `config.rs`, `transpile.toml`.

### 2.1 Pipeline

The four-phase pipeline from the March spec is implemented in `main.rs`:
parse and extract signatures (bodies stored as `syn::Block` in
`FnInfo.body_ast`), build the registry, translate bodies with registry and scope,
emit. `codegen.rs` and `emit.rs` consume `body_ts` strings.

### 2.2 Data structures that stay

- `ResolvedType` (`resolve.rs:16`): `Primitive`, `Named{name,args}`, `Param`,
  `Array`, `Tuple`, `Fn{params,ret}`, `Nullable`, `Unknown`, with `substitute`
  (`:51`) for type-parameter substitution. Keep, and extend (section 4.1).
- `TypeDef` (`:91`) with `TypeKind::{Struct, Enum{variants}, Trait}`,
  `VariantDef`, `MethodSig{params, ret, is_static}`, `deref_field`, `type_params`.
  Keep as the declared-shape record; the impl table (4.2) is added beside it.
- `ScopeStack` (`:395`) with `Crate/Module/Impl/Fn/Block/Closure` scopes,
  innermost-wins resolution and same-scope shadowing (`:465`, `:486`). Keep.
- `TypeContext` (`type_context.rs:14`): registry plus scopes plus module name;
  `resolve_expr` (`:80`) handles `self`, single-segment paths (camelCase and
  Rust-name lookups), field access with one deref hop, method calls with one
  deref hop, `&e`, `*e`, parens, and block tail. Keep the shape; replace the
  internals per section 4.
- Enum and variant detection through the registry (`resolve.rs:206-240`), used
  by `body.rs` to tell `Foo::Bar(v)` from a static call. Keep.
- Module-qualified registration: types are keyed both as `module::Name` and bare
  `Name`, with an `ambiguous_bare` set (`resolve.rs:138-186`). Keep the idea; fix
  the eviction rule (2.3, item 1).
- System types declared in `transpile.toml [system_types]` with `deref_field`,
  `type_params`, and method return types, parsed by `parse_type_string`
  (`resolve.rs:519`). Kept for now; section 4.4 moves the std surface into declared Rust stubs.

### 2.3 Defects and gaps in what exists

1. **System types are evicted by same-named crate types.** `register_in_module`
   (`resolve.rs:162-186`) treats a bare-name collision as ambiguity and deletes
   the bare entry. `signals/src/broadcast.rs:50` defines `pub struct Ref<'a, T>`,
   which deletes the system `Ref`; `deref_field` and `is_own_method` then do bare
   lookups only (`:346`, `:362`) and never find it. Core will collide on `Context`,
   `Node`, `Value`, `Error`. Rule violated: a crate type can never evict a system
   type.
2. **One deref hop, always into the first generic argument.** `resolve_field_impl`
   (`:251-301`) and `resolve_method_impl` (`:313-343`) unwrap `args[0]` once; the
   comment at `:272` says nested chains "lose the inner accessor". `Vec<T>` derefs
   to `[T]`, `String` to `str`, neither of which is a generic argument of anything.
3. **No impl table, no trait identity.** `build_registry` (`:601-682`) merges every
   impl's methods into the target type's flat `methods: HashMap<String, MethodSig>`
   and drops `trait_name`, `trait_type_args`, and `generic_bounds` (which
   `ImplInfo` at `types.rs:108-118` already carries). Two traits with the same
   method name overwrite each other; blanket impls and impls on foreign types are
   invisible; `dyn Trait` receivers resolve to nothing.
4. **Types round-trip through TypeScript strings.** `FieldInfo.ty` and
   `ParamInfo.ty` are TS strings produced by `name_map::map_type`
   (`name_map.rs:68`); `resolve_local_type` (`type_context.rs:183`) maps a
   `syn::Type` to a TS string and re-parses it with `parse_type_string`
   (`resolve.rs:519`), a bracket-splitting parser. `Vec<u8>` has already become
   `Uint8Array`, `Option<T>` has become `T | null`, `HashMap` has become `Map`,
   `u64` has become `bigint | number`, and `Box<T>` has become `T` before the
   engine sees them. Every fact pays this tax; several cannot survive it (a
   `Box<dyn Trait>` field is indistinguishable from a `Trait` field).
5. **No associated types**, so no iterator adaptors: `ResolvedType` cannot
   express `<I as Iterator>::Item`.
6. **Closures**: `resolve_closure_param_types` (`type_context.rs:261-283`) handles
   exactly `ThreadLocal<T>::with`; its comment says the general case is a TODO.
   Closure return types are never inferred.
7. **No `From`/`Into`/`TryFrom` table**, so `?` on a differing error type and
   every `.into()` are untyped.
8. **No operator overloading**: `PartialEq`, `PartialOrd`, `Add`, `Index`, `Neg`,
   `Not` calls are not resolved.
9. **`Unknown` is a silent fallback.** The March spec's "Tier 3: fall back to
   current heuristic behaviour" is live in `body.rs` (PascalCase enum guess at
   `body.rs:841`, `starts_with` checks on names, textual identifier substitution
   in `match_expr.rs:310-334`). Under the fail-loud ruling every one of these is
   a defect.
10. **Tests**: `resolve.rs` has 14 unit tests on the registry, scope stack, and
    `parse_type_string`; `type_context.rs` has none; nothing exercises the engine
    against real corpus files.

## 3. What the engine must cover, measured on the corpus

Counts from the 2026-09-02 study, taken with rust-analyzer's inference as ground
truth over `ankurah-ts-support @ e0bc2b76`, crates `proto`, `ankql`, `signals`,
`core` (107 files, 29,078 expressions). The numbers size the work; the engine is
bounded to these constructs and the idioms behind them.

| construct | proto | ankql | signals | core | total |
|---|---:|---:|---:|---:|---:|
| method calls | 225 | 367 | 429 | 3,379 | 4,400 |
| through an inherent impl | 134 | 260 | 223 | 1,861 | 2,478 |
| through a concrete trait impl | 48 | 72 | 51 | 574 | 745 |
| through a generic or blanket impl | 36 | 35 | 61 | 482 | 614 |
| receiver not syntactically typed | 77 | 131 | 138 | 1,195 | 1,541 |
| needing auto-deref | 63 | 177 | 116 | 927 | 1,283 |
| needing a `Deref` impl (not `&`) | 11 | 14 | 57 | 257 | 339 |
| needing two or more deref steps | 0 | 15 | 7 | 51 | 73 |
| needing an unsize coercion (`Vec<T>` to `[T]`) | 0 | 1 | 0 | 53 | 54 |
| on a `dyn Trait` receiver | 0 | 0 | 2 | 9 | 11 |
| `?` expressions | 48 | 88 | 6 | 276 | 418 |
| `?` changing the error type (lower bound) | 31 | 26 | 0 | 18 | 75 |
| closures | 32 | 20 | 79 | 219 | 350 |
| closures with a non-unit inferred return | 30 | 20 | 34 | 172 | 256 |
| `.into()` / `.try_into()` | 24 | 18 | 3 | 71 | 116 |
| overloaded binary operators | 14 | 28 | 14 | 258 | 314 |
| overloaded index | 7 | 2 | 0 | 72 | 81 |
| overloaded prefix operators | 3 | 34 | 21 | 293 | 351 |
| `.await` | 0 | 0 | 2 | 201 | 203 |
| `impl Trait` in argument position | 3 | 1 | 11 | 27 | 42 |

Representative sites, each of which the engine must type:

- Guard deref then map method: `signals/src/observer/callback_observer.rs:51`
  `self.0.entries.write().expect(..)`, `RwLockWriteGuard<HashMap<..>>` to
  `HashMap` to `&mut HashMap`.
- Unsize deref: `proto/src/clock.rs:14` `self.0.iter()`, `Vec<EventId>` to
  `[EventId]` to `&[EventId]`; `ankql/src/selection/sql.rs:43` `s.chars()`,
  `&String` to `String` to `str` to `&str` (three steps, two mechanisms).
- Iterator chain with turbofish: `signals/src/broadcast.rs:100`
  `listeners.values().cloned().collect::<Vec<_>>()`, guard deref, `HashMap::values`,
  `Iterator::cloned`, `Iterator::collect` driven by the turbofish.
- Blanket impl on closures: `signals/src/broadcast.rs:149-153`
  `impl<F: Fn(T)> IntoBroadcastListener<T> for F`, called at `broadcast.rs:106`.
- Trait impl on `Arc`: `signals/src/signal/calculated.rs:137-167`
  `impl Observer for Arc<Inner<T>>`.
- `dyn` receiver: `core/src/context.rs:83` `self.0.node_id()` on
  `&(dyn TContext + Send + Sync)`.
- `?` through `From`: `proto/src/data.rs:69`, `DecodeError` into the enclosing
  error type; `proto/src/id.rs:152` through serde's `Deserializer::Error`.
- Closure return: `signals/src/reactive_graph.rs:83` `|| BridgeSource::new(id, signal)`
  returning `Arc<BridgeSource>`.
- Method on a primitive through a blanket impl: `ankql/src/selection/sql.rs:902`
  `.to_string()` on `&i16` via `impl<T: Display> ToString for T`.

## 4. Design

Each capability states what it is for, the mechanism, and what happens when it
cannot answer. Names are proposals; module boundaries follow section 5.

### 4.1 Structural types end to end

For: every later capability needs the real Rust type, and the string round trip
(2.3 item 4) destroys it.

- `resolve_type(&syn::Type, &TypeEnv) -> Result<Ty, Diag>` replaces
  `map_type` followed by `parse_type_string` inside the engine. `map_type` stays
  for emission only, and is fed `Ty`, not `syn::Type`.
- `Ty` extends `ResolvedType` with: `Ref{mutable, inner}` (kept in the engine,
  erased at emission), `Slice(inner)`, `Str`, `Unit`, `Never`,
  `Dyn{traits}`, `Assoc{base, trait_, name}` (a projection such as
  `<I as Iterator>::Item`, normalized by 4.4), `Infer` (a `_` in a turbofish or
  annotation, resolved by 4.7). `Named` keeps `name` as a module-qualified path
  plus a `TypeId`, never a bare string. `Option<T>` stays `Named` in the engine;
  the `Nullable` mapping happens at emission.
- `FieldInfo`, `ParamInfo`, `FnInfo.return_type`, `TypeAliasInfo`, `ConstInfo`
  carry `syn::Type` (or the resolved `Ty`) in addition to the TS string, so the
  registry is built from Rust types. `rust_ty: String` on `FieldInfo` goes away.
- The registry is keyed by `TypeId` with a name index of module-qualified paths.
  Bare-name lookup exists only for `use`-resolved and prelude names inside a
  module scope; system types live in a reserved module that a crate type cannot
  shadow (fixes 2.3 item 1).

Cannot answer: a `syn::Type` the resolver does not model (a raw pointer, a
function pointer with ABI, a const generic other than an array length) is a
diagnostic at the type's span.

### 4.2 Impl table and method resolution

For: 1,359 of 4,400 method calls dispatch through a trait, 1,283 need
auto-deref, and the right impl decides the emitted TypeScript.

- `ImplDef { id, self_ty: Ty (with params), trait_: Option<TraitRef>,
  generics: Vec<Param{name, bounds}>, where_: Vec<Bound>, assoc_types:
  HashMap<String, Ty>, methods: HashMap<String, MethodSig> }` built from every
  `impl` in the corpus (`ImplInfo` already carries `trait_name`,
  `trait_type_args`, `generic_bounds`; extraction keeps `where` clauses and the
  `syn::Type` of the target), plus the declared std surface (4.4).
- `TraitDef { id, path, supertraits, assoc_types, methods with self-relative
  signatures, blanket_impls }`.
- Method lookup follows Rust's algorithm restricted to what the corpus needs:
  1. Build the candidate receiver list by auto-deref: the receiver type, then
     repeatedly its deref target (builtin for `Ref`, `Deref::Target` from the
     impl table for everything else, unsize `Vec<T>` to `[T]` and `String` to
     `str` at the end of the chain). Record each step as
     `DerefStep { from, to, kind: Builtin | Overloaded(impl id) | Unsize }`.
  2. At each candidate, try by-value, then `&`, then `&mut` auto-ref.
  3. At each (candidate, autoref): inherent impls whose `self_ty` unifies (4.8),
     then trait impls whose `self_ty` unifies and whose trait is in scope (a
     `use`, the prelude, or a bound on a generic in scope), then blanket impls
     whose bounds hold given known impls.
  4. First step with exactly one match wins. Zero matches after the chain is
     exhausted, or more than one match at a step, is a diagnostic. There is no
     "first match" tie-break.
- `dyn Trait` receivers resolve to the trait's method; the emitted call is a
  normal TypeScript method call, and the emission layer needs only the trait
  method signature.

Resolution order as implemented (step 2). Rust has two tiers and so does this:

  1. Build the receiver list by auto-deref, then append the unsized form of the
     last entry (`[T; N]` to `[T]`). `Vec<T>` to `[T]`, `String` to `str` and
     `Box<T>` to `T` are `Deref` impls, which is the mechanism Rust uses for
     them; only the array is an unsizing.
  2. At each receiver, take by value, then `&`, then `&mut`.
  3. At each (receiver, borrow), match the *method's declared receiver type*
     against the borrowed receiver — not the borrow kind against the borrow
     being tried. `impl Direct for Conc` declaring `fn tag(&self)` accepts a
     `&Conc` with no borrow added, so it competes at the first step; comparing
     borrow kinds let a blanket impl win a step earlier with a different answer.
  4. Inherent impls first. Then one extension tier holding every trait impl
     written for a definite type, every impl written for one of its own
     parameters, and the declaration a `dyn Trait` or a bounded parameter
     dispatches through. Coherence forbids two impls of one trait for one type,
     so splitting the extension tier further could only ever hide a clash
     between two different traits — which is the clash Rust reports (E0034,
     checked against rustc).
  5. Exactly one match at a step wins. Two is a diagnostic naming both. None
     after the chain is exhausted is a diagnostic naming every receiver tried.

Trait visibility. Rust admits an extension method only when its trait is in
scope, and the engine applies that as a **filter** on the extension tier: a
candidate whose trait the calling module cannot name is dropped before the step
is counted. Where the filter would leave nothing, the unfiltered list stands and
the sole survivor is taken and reported ("resolved through trait `T`, which is
not in scope here"), so a gap in the `use` map shows up in the diagnostics count
rather than deleting a method silently.

Ruling changed 2026-09-03 (it was a tie-break until then). The declared std
surface has reflexive blanket impls — `impl<T: ?Sized> BorrowMut<T> for T`, and
the same for `Borrow` and `AsRef` — which answer to `borrow_mut` on *every*
receiver at depth 0. As a tie-break they had no competition there and won, so
`guard.borrow_mut()` on a `RwLockReadGuard<RefCell<T>>` resolved to the blanket
instead of `RefCell::borrow_mut` one deref later, and the `.value` accessor the
guard needs was never written. Filtering them out at depth 0 lets the chain
continue to the receiver Rust actually reaches.

Duplicates are not a clash. One function reachable by two routes — a supertrait
and a subtrait both offering it — is deduped by callee identity before the
count, which was reporting `Iterator::find` as ambiguous with itself.

Bounds. An impl whose `where` clause definitely fails is not a candidate. One
the engine cannot decide — `F: Fn(T)` before the closures step, a trait with no
declaration yet — stays a candidate, and the undecided bound travels with the
answer as a deferred obligation and is reported at the call. A bound written on
a parameter in scope is its own proof: inside `impl<SE: Engine> Node<SE>` there
is no impl to go looking for. Bound lookup and projection compare the whole
`TraitRef`, arguments and associated bindings included, so `impl Marker<u16> for
S` does not prove `S: Marker<u8>`.

Emission of a step. A declared system wrapper names the field to write
(`.value`); a transparent one writes nothing; a crate's own `impl Deref` writes
the `deref()` call Rust inserts and the emitted class carries.
- Result: `MethodResolution { steps: Vec<DerefStep>, autoref, callee: Callee::{
  Inherent(impl, method) | TraitImpl(impl, method) | TraitObject(trait, method)
  | Blanket(impl, method) }, subst: Subst, ret: Ty }`. Emission derives the
  `.value` accessors from `steps` (one per `Overloaded` step whose impl is a
  non-transparent system type) and the TS method name from `callee`.

### 4.3 Deref and coercion as chains

For: 73 sites need two or more steps and 54 need the unsize step; one hop into
`args[0]` (2.3 item 2) cannot represent `&String` to `str`.

- `Deref::Target` is an ordinary impl-table fact: corpus `impl Deref for X`
  blocks, plus declared targets for system types (`Arc<T>` to `T`, `Box<T>` to
  `T` transparent, all guards to `T`, `Vec<T>` to `[T]`, `String` to `str`,
  `Ref<T>`/`RefMut<T>` to `T`).
- Field access uses the same chain: `resolve_field` walks candidates until a
  type with that field is found, returning the steps for emission.
- `*e` uses one explicit step; `&e` wraps in `Ref`; `Ref` is stripped by
  auto-deref and by emission.

### 4.4 Traits, associated types, and the declared std surface

For: iterator chains, `collect`, `Deref::Target`, `IntoIterator::Item`, and
every `impl Trait for T` the corpus relies on from std.

- Projections `Assoc{base, trait_, name}` normalize by selecting the impl of
  `trait_` for `base` (4.2 step 3) and reading its `assoc_types`. A projection
  that does not normalize is a diagnostic.
- The std surface ankurah uses is declared as Rust stub declarations shipped with
  the transpiler (`transpile/std_surface/`, one file per std area: `iter`,
  `collections`, `string`, `slice`, `option_result`, `convert`, `ops`, `sync`,
  `cell`, `fmt`): signature-only `impl` and `trait` blocks written in ordinary
  Rust, with real generics, bounds, and associated types, parsed by the same
  syn-based extractor that reads ankurah's source. Std types therefore reach the
  registry by the same path as crate types, are exercised by the same tests, and
  fail the same way. Method bodies in a stub are placeholders (`{ todo!() }`, or
  whatever parses) and are ignored; the signature is the declaration. What a std
  item becomes in TypeScript stays where it is today, in `name_map` and the
  per-type emission modules under `native_types/` (Daniel's March ruling: one
  module per Rust type for both type and method conversion). TOML strings cannot
  express bounds or projections, so the `[system_types]` table in
  `transpile.toml` is retired when the stubs land (step 3).
- The same mechanism carries the non-std surfaces the storage and connector
  crates need (1a): rusqlite's types behind the sqlite driver interface, and the
  web-sys types behind the IndexedDB and WebSocket glue, are stub declarations
  too.
- Coverage is by inventory: the corpus's std method calls are enumerated by the
  test in 6.2, and any std method the surface does not declare is a failing
  test, not a runtime guess. Adding a std method is a declared fact plus a
  translation, both reviewed.

### 4.5 Closures

For: 256 closures with a non-unit return, and closure parameters typed by the
callee's bound.

- When a closure is an argument, the callee's resolved signature (4.2) gives the
  parameter's type as a bound `F: Fn(A) -> R`, `FnMut`, `FnOnce`, or an `impl
  Fn(..)` in argument position (the 42 `impl Trait` params). Unify (4.8) the
  bound's `A` with any annotations on the closure, bind parameters in a
  `Closure` scope, type the body, and unify the body's type with `R`.
- A closure in `let` position with annotated parameters is typed from the body.
  A closure with no annotation and no expected type is a diagnostic.
- Captured variables resolve through the enclosing scopes as today.

As implemented (step 4). The three sources are read in Rust's own order: the
annotation the closure writes for itself, the callable the position expects, and
— for the result only — the body's own tail, typed with the parameters bound.

Where the expected type names no callable, one blanket impl of the bound it does
name is followed: `L: IntoBroadcastListener<T>` says nothing about calling `L`,
and `impl<F: Fn(T)> IntoBroadcastListener<T> for F` says that a closure standing
there is an `Fn(T)`. That is the reverse of the deferred obligation resolution
files (4.2): the obligation asks whether `L` is an `Fn`, and a closure written
at that argument is the answer. One hop, and only where exactly one blanket impl
of the bound carries a callable bound of its own; two would be a choice, and
nothing here makes one.

A result is taken from the expected callable's `Output` only when that `Output`
is settled. `Iterator::map` declares `F: FnMut(Self::Item) -> B` and leaves `B`
to whatever the closure returns, so reading `B` off the bound would answer the
question with the question. The same test applies to the body's own tail: a type
still holding a parameter that belongs to somebody else's signature — the `U` of
`TryInto::try_into` — is refused rather than handed on. A parameter the
enclosing signature declared is a real type and stays.

A parameter nothing typed is bound in the closure's scope without a type, so the
body reads it as a name that exists rather than as a name nobody declared, and
the gap is reported once at the closure instead of at every use inside it.

### 4.6 `?`, conversions, and expected types

For: 75 or more `?` sites change the error type; 116 `.into()`/`.try_into()`
calls have no type without an expected type.

- `?` on `Result<T, E1>` inside a function returning `Result<_, E2>`: `T` if
  `E1 == E2`; otherwise require exactly one `impl From<E1> for E2` in the impl
  table (thiserror `#[from]` and `#[error]` attributes register these impls
  through the derive hook, 4.10; `anyhow::Error` gets a declared blanket `From<E:
  std::error::Error>`). Record the conversion on the expression so emission can
  call the TypeScript `from`. `?` on `Option<T>` inside a function returning
  `Option<_>` yields `T`. Anything else is a diagnostic.
- `.into()`, `.try_into()`, `T::from(x)`, `x.parse()` and `collect()` use the
  expected type: a `let` annotation, the parameter type at a call, the return
  type at a tail or `return`, a struct field at a struct literal, or a
  turbofish. Expected types propagate one level (bidirectional but shallow); an
  `.into()` with no expected type is a diagnostic.
- `Infer` from `Vec<_>` in a turbofish resolves by unifying with the iterator's
  `Item` projection.

As implemented (step 4). An expectation is carried to one expression, matched by
its span, and put back afterwards, so a sub-expression translated before its
parent cannot take an answer meant for the parent. The positions that supply
one: an annotated `let`, a function's return at the tail and at a `return`, an
argument of a method call or of an associated function (through the callee's
declared parameter types, with the callee's own parameters closed by what the
position wants — which is how `Box::new(move |x| ..)` types its closure), a
struct-literal field, the other operand of `assert_eq!`/`assert_ne!`, and the
receiver of an `unwrap` or an `expect`, which wants the wrapper around what the
call wants.

What an expectation settles: the width of an integer or float literal written
without a suffix; whether a sequence literal is a sequence of bytes, which
decides `Uint8Array` against a JavaScript array; the `_` holes in a written
type; which type reads itself out of a `serde_json::from_str` or a
`bincode::deserialize` written without a turbofish; and a closure's parameters.
A call whose result holds no open parameter is left alone: the expectation is a
hint about a position, never an override of a type the source settled.

### 4.7 Operators

For: 746 operator uses resolve to trait impls; the emitted TypeScript differs
for `equals` versus `===`, `compareTo` versus `<`, and `Index` on a `Map`.

- Binary, unary, and index expressions resolve through the impl table
  (`PartialEq`, `PartialOrd`, `Add`..`Rem`, `Neg`, `Not`, `Index`, `IndexMut`,
  the `*Assign` family), with builtin rules for primitives. The result type is
  the impl's `Output` projection. A missing impl is a diagnostic.

### 4.8 Generics: substitution and unification

- `substitute` stays. Add `unify(pattern: &Ty, concrete: &Ty, subst: &mut Subst)
  -> Result<(), Mismatch>` used by impl selection, closures, and expected types.
- Bounds on impl generics are checked by looking for impls (recursively, with a
  depth limit that is a diagnostic when hit). No specialization, no negative
  reasoning, no coherence checking: the corpus compiles under rustc, so exactly
  one impl applies and the engine only has to find it.

### 4.9 Scopes and names

- `ScopeStack` stays. Module scope gains the file's `use` map resolved to
  `TypeId`s and trait ids (so trait-method visibility in 4.2 step 3 is real),
  the prelude, and `Self` in impl scope bound to the impl's `self_ty` with its
  generics.
- Constants and statics are bound at module scope with resolved types.
- Match arms and `if let` bind pattern variables through `bind_pattern`, with
  Rust's default binding modes (RFC 2005): a non-reference pattern matched
  against a reference peels one layer and binds everything under it by
  reference, `&pat` consumes exactly one layer, and `ref`/`ref mut` say the
  borrow outright. The textual `replace_identifier` in `match_expr.rs` is gone;
  an arm emits its payload as real declarations.
- A Rust shadow is a new variable. JavaScript cannot declare a name twice in one
  scope, so the shadow is emitted under a fresh identifier and every later use
  of the name follows it; assigning to the old one instead changed a value that
  a closure capturing it, or a caller owning it, could still see.

### 4.10 Types for macros and derives

For: the transpiler does not expand macros, so the engine must state what each
supported macro produces and what each supported derive implements.

- Invocations: `format!`, `write!`, `writeln!` yield `String` / `fmt::Result`;
  `vec![..]` yields `Vec<T>` from its elements; `assert*!`, `panic!`, `todo!`,
  `unreachable!`, `unimplemented!` yield `!`; `matches!` yields `bool`;
  `thread_local!` binds a static of the declared type; tracing macros yield
  unit. Any other macro is a diagnostic at the invocation (this is already the
  no-expansion rule's failure mode).
- Derives register impls: `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
  `Hash`, `Default`, `Debug` (as `Display`-shaped `toString` at emission),
  `Serialize`/`Deserialize` (an `encode`/`decode` pair, typed for the bincode
  module), `thiserror::Error` with `#[error]` (a `Display` impl) and `#[from]`
  (a `From` impl). `#[async_trait]` is an attribute the engine ignores: the
  method is `async` in source and its return type is the written one.

### 4.11 Failure policy

- The engine returns `Result<Ty, Diag>` everywhere; `Unknown` is removed as a
  value.
- Every site in `body.rs`, `control_flow.rs`, `match_expr.rs`, `macros.rs`, and
  `bincode_module.rs` that today falls back to a heuristic on `None` instead
  reports the diagnostic and the run fails without writing output (the
  diagnostics design from the item 1 proposal, kept as a patch beside the
  handoff, describes the sink and the no-write rule).
- The PascalCase enum guess, `starts_with` name checks, the std path-segment
  filter in `body.rs`, and `replace_identifier` in `match_expr.rs` are deleted
  as each capability above makes them unnecessary; none survives to the end.
- Transitional policy, until the fail-loud step (section 7, step 9). The engine
  never returns an unknown type; what stays transitional is the translator. A
  site that still falls back keeps its fallback and records a `Diag` in a per-run
  sink wherever it fires, so that each step's output stays comparable with the
  step before it and a run does not fail on work that has not been done yet.
  Every run prints the diagnostics count and the list. Step 9 makes the sink
  fatal and deletes every fallback. Until it does, the diagnostics count per
  crate is the coverage metric: the amount of that crate the engine cannot yet
  type.

## 5. Module layout (proposal)

```
transpile/src/
  ty/            Ty, Subst, unify, substitute, display          (from resolve.rs)
  registry/      TypeDef, TraitDef, ImplDef, TypeRegistry, name index
  infer/         TypeContext, resolve_expr, method lookup, closures, expected types,
                 ?/From, operators, macro/derive hooks
  diag.rs        diagnostics sink (from the item 1 proposal)
transpile/
  std_surface/   Rust stub declarations, one file per std area (replaces [system_types])
```

`resolve.rs` and `type_context.rs` are split into these; nothing in `emit.rs`
or `codegen.rs` changes shape except that they receive `Ty` where they
received strings.

## 6. Tests and the oracle

### 6.1 Unit tests on minimal Rust inputs

Each capability in section 4 gets tests that parse a few lines of Rust with syn,
build a registry, and assert the resolved `Ty`, the `MethodResolution` steps, or
the diagnostic. These are the specification's executable form; no capability is
done without them.

### 6.2 Corpus inventory tests

A test walks the four corpus crates on the support branch and asserts that every
method call, field access, `?`, closure, operator, and `.into()` resolves or
produces a diagnostic the test expects. The list of expected diagnostics is
checked in and must shrink, never grow, as capabilities land. This is also the
inventory that keeps the std surface (4.4) honest.

### 6.3 rust-analyzer as a one-time oracle

The 2026-09-02 study produced, for every method call, deref chain, closure, and
`?` in the corpus, rust-analyzer's answer keyed by file and byte range
(`ra-spike` under the session scratchpad; to be checked in under
`transpile/tests/oracle/` as JSON). A test compares the engine's answers against
it on the sites the oracle covers and fails on any mismatch. The oracle is data
in the repo: nothing from rust-analyzer is a dependency, and the file is
regenerated only deliberately with the out-of-tree nightly spike. Known gap: the
spike typed 88% of expressions and left items under `#[async_trait]`,
`wasm_bindgen`, and `uniffi::export` untyped; those sites are simply not in the
oracle and are covered by 6.2 instead.

### 6.4 Goldens and snapshots

Two artifacts, kept apart because they answer to different readers:

- Idiom goldens, a small hand-vetted set under `transpile/goldens/`, one per
  construct: a guard deref, an iterator chain, a `?` through `From`, a match that
  binds an enum payload, each a few lines of Rust beside the TypeScript it should
  produce. This is Daniel's review surface for emission decisions, read as
  documentation, and it changes only by a deliberate reviewed edit, never by a
  capture command.
- Corpus snapshots, auto-captured transpiler output for the in-scope crates under
  `transpile/tests/snapshots/`, updated only by an explicit command, with the
  diff read at each transpiler change. A difference is either an expected fix or
  a regression, and the review says which.

The March `transpile/golden/` directory is captured output that nothing reads;
it is retired when the goldens land.

Neither artifact checks behavior. The transpiled Rust unit tests and the support
branch's bincode fixtures do that.

## 7. Work remaining, in order

Each step is discussed before it starts and reviewed as its own diff. Steps 1 and
2 are the substance; the rest are small once they exist. The test harness
(section 6) is built alongside step 1 by a separate agent, so that step 2 onward
has the goldens, the snapshots, and the oracle to land against.

1. **Structural types and the registry** (4.1, 4.9): `Ty`, `resolve_type` from
   `syn::Type`, registry keyed by id with module-qualified names, reserved
   system module, `use`-map in module scope. Deletes the string round trip and
   fixes the `Ref` eviction. Unit tests.
2. **Impl table and method resolution** (4.2, 4.3, 4.8): `ImplDef`, `TraitDef`,
   extraction of trait identity, bounds and `where` clauses, `unify`, the
   auto-deref/auto-ref chain, impl selection with blanket impls, `dyn`
   receivers, `MethodResolution` with steps for emission. Unit tests plus the
   corpus inventory test from 6.2 for method calls.
3. **Std surface** (4.4): the Rust stub declarations for the traits and impls the
   corpus uses, with associated types; iterator adaptors and `collect`; retire
   `[system_types]`.
4. **Closures and expected types** (4.5, 4.6): callee-bound closure typing,
   shallow expected-type propagation, `Infer`.
5. **`?` and conversions** (4.6): `From` lookup, `Option` `?`, `.into()` family,
   recorded conversions for emission.
6. **Operators** (4.7).
7. **Macro and derive hooks** (4.10), including thiserror-derived `Display` and
   `From` impls.
8. **Crate scope and configuration** (1a): trim `[crates]` to the scope table,
   set the wasm32 cfg configuration, teach the transpiler externalized modules,
   and land the sqlite driver interface as a provided file. It comes before the
   fail-loud step because widening the scope adds diagnostics, and those have to
   be driven to zero before the sink can be made fatal.
9. **Fail-loud wiring** (4.11): the diagnostics sink made fatal, removal of every
   heuristic fallback, no-write on error.
10. **Oracle and snapshot tests** (6.3, 6.4) checked in and green.

Acceptance for the engine as a whole: every expression the transpiler translates
in proto, ankql, and signals resolves or produces an expected diagnostic; the
oracle comparison has zero mismatches on covered sites; proto's output is
unchanged where it was already correct.

## 7a. Known gaps, recorded rather than fixed

Each of these is understood, has a place in the plan, and is deliberately not
addressed by the step that found it.

- **A crate `Deref` body needs a coercion the engine does not yet insert.**
  `impl Deref for Entity { type Target = EntityInner; fn deref(&self) -> &Self::Target { &self.0 } }`
  where `self.0: Arc<EntityInner>` relies on Rust's deref coercion at the return
  position. The call site now writes `.deref()` and the signature now reads
  `deref(): EntityInner`, so the gap is a TypeScript type error in one method
  body instead of a silent wrong read at every use. It closes with expected
  types (4.6, step 4).
- **`self: Arc<Self>` and `self: Pin<&mut Self>` are reported and left out of
  the method table.** Five methods in core (`as_arc_dyn_any` ×4, `poll_next`),
  none of them called by method syntax. The written receiver type is kept on the
  extracted function and on `MethodSig.receiver`, so supporting them is a
  matching change, not a modelling one.
- **A growable `Vec<u8>` has no runtime type.** `Uint8Array` is fixed-length, so
  the read-only half of `Vec<u8>` translates and every call that would grow or
  shrink one is reported. Choosing a byte-buffer type is a runtime decision
  (`packages/base`), not a transpiler one.
- **`core` and `alloc` are modelled as the modules they really have, not as
  aliases of the whole of `std`.** Every declaration is written under
  `std_surface/std/`, because that is where the stubs live; the other two roots
  are built from a table of the modules each crate genuinely holds, with
  `core::sync` and `alloc::sync` assembled item by item because their contents
  differ from `std::sync`'s. A `core::` or `alloc::` module the table does not
  list resolves to nothing, which is what Rust does — adding one is a line in
  `ROOT_REEXPORTS`.
- **A method's `where` clause is checked only once its parts are closed.**
  `fn collect<B: FromIterator<Self::Item>>` written with no turbofish leaves `B`
  open, and `fn get<Q>(&self, k: &Q) where K: Borrow<Q>` leaves the `Q` inside
  the bound open; rustc decides both from the argument and the expected type,
  which is step 4. Until then a bound still naming an unfilled parameter is
  skipped rather than failed — failing it deleted `HashMap::get` and
  `Iterator::collect` outright — and a bound the substitution closed is decided
  normally.
- **A derive proves what rustc's derive proves and no more.**
  `#[derive(Clone)] struct W<T>` registers `impl<T: Clone> Clone for W<T>`, so a
  `W<NotClone>` is not `Clone`. rustc's rule is per type *parameter*, not per
  field, and it is deliberately stricter than a field-wise analysis would be:
  the engine follows rustc rather than being cleverer than it.
- **A type's arguments are compared all the way down against the oracle, but
  each name only by its leaf.** rust-analyzer and the declared surface render
  module paths differently and there is no mapping between the two spellings, so
  `Vec<u8>` and `alloc::vec::Vec<u8>` compare equal while `Vec<u8>` and
  `Vec<String>` do not. rust-analyzer's allocator parameter (`Global`) and its
  lifetimes are dropped from the comparison: the surface models neither, and the
  README says so.
- **Raw pointers are not modelled.** `*const T` and `*mut T` stop
  `resolve_type`, so a signature that names one is left out of the method table
  — `Weak::as_ptr` and `Arc::as_ptr` are the corpus's only uses, both immediately
  cast to `usize` for an identity. One of the two oracle sites the engine does
  not cover is `Weak::as_ptr` for this reason.
- **A block's own `let`s are not in scope when the block is typed as an
  expression.** `resolve_expr` on a `Expr::Block` reads its tail expression, and
  the tail may name a local the same block introduced, which nothing has bound;
  binding them needs `&mut self` where `resolve_expr` takes `&self`. This is why
  `let subscribers = { let listeners = ..; listeners.values()..collect() };`
  leaves `subscribers` untyped, and it is the second uncovered oracle site
  (`<[T]>::split_last`).
- **Ownership emission: what the model deliberately does not cover.** The
  releases the emitter writes are described in `port/ownership.md`; these are the
  places it knows it is not faithful, each reported at the site.
  - **A `let` that takes a droppable value apart** — `let (a, b) = pair()` —
    releases neither part. Rust drops the parts separately, which needs a
    per-field release the emitter does not write. A `let x = s.field` on a
    struct is not this: that is `takeField`, and it is emitted.
  - **A local whose type the engine could not name is not released.** The
    release is only as good as the type, and one written against a guess would
    release a value somebody else owns. The same gap reaches a `move` closure:
    a capture the engine could not type is left out of what the closure owns,
    and the closure is reported rather than given an incomplete list silently.
  - **A closure the emitter cannot see the call site of.** A `move` closure over
    droppable values is an `OwnedClosure`, which is invoked as `.call(...)`; the
    emitter rewrites the call sites it can see — a closure bound to a local —
    and reports the ones it cannot, because the callee that receives the closure
    still writes `f(x)`.
  - **`?` across two error types calls the `From` impl the engine resolved**
    (step 5). What is left is named at each site: a conversion whose source or
    target is still a type parameter, which Rust decides per instantiation and
    one emitted body cannot; a conversion the declared surface performs, where
    the port has no runtime type to call it on — every `?` into an
    `anyhow::Error` is one of those, because `@ankurah/base` has no anyhow
    stand-in; and a target two `From` impls emit under one name.
  - **`mem::forget` cannot be said at all**: the emitted value has no way to
    cancel its own drop glue, so it is left for the leak registry.
  - **A discarded value whose runtime call is not the Rust call is not
    released.** `map.insert(k, v)` returns the displaced value in Rust and the
    `Map` itself in TypeScript, so releasing the statement's value would release
    the map. Every other discarded value is released at the end of its
    statement.
  - **An awaited value whose `Future::Output` the engine could not project is
    not released.** The engine hands back the future's own type there, and
    releasing that would drop a value the await already moved.
  - **A loop over an owned sequence the runtime does not write as an array does
    not release what an early exit left behind.** Rust's `IntoIter` drops the
    elements it never handed out; a `Vec` is an array and the emitter can name
    them, and a `HashMap` or an adaptor is not and it cannot.
  - **A macro the emitter does not expand releases nothing it was handed.** The
    macro becomes `undefined` with the source in a comment beside it, so a value
    passed into one goes with it. It is `undefined` rather than the comment
    alone because a comment is not an expression: one written where an argument
    stood did not parse, and a file that does not parse stops the TypeScript
    compiler from checking anything else in the project.
  - **A closure whose body hands a capture away owns nothing of it.** Rust moves
    the capture into the body when an `FnOnce` runs, so the closure has nothing
    left to drop; the runtime's `OwnedClosure` keeps its captures private and
    offers no call that transfers them, so listing one drops it a second time
    when the closure is dropped — a fatal — and leaving it out leaks it only
    where the closure is never called. The capture is left out and the site says
    so. The runtime change that would close it is a `callOnce` that marks the
    closure moved before running the body.
  - **An `if let` that takes a payload out of a wrapper leaves the wrapper.**
    The path where the pattern did not match releases the value it tested, as a
    `while let` does; the path that matched has taken the payload, and marking
    the wrapper moved is what `intoMatch` does and an `if` cannot. An
    `Option<T>` has no wrapper — it is `T | null` — so only an enum reaches
    this.
  - **Two `From` impls for one target that differ only in a reference are one
    function here.** `impl From<EntityId> for Ref<T>` and `impl From<&EntityId>
    for Ref<T>` take the same emitted name, because emission erases the
    reference; the second is dropped and the site says so. Six in the corpus.
  - **A `?` inside a match arm, where the match is a statement.** An arm is an
    arrow function, so the early exit leaves the arm rather than the function.
    Where the match is the enclosing function's value the arm's `Result` is what
    the function returns, and that case is right.
  - **A `select!` arm whose pattern can fail is taken anyway.** tokio disables
    the branch and keeps waiting; this lowering has no form for that.
  - **A field name that is also an `AkObject` member** — `label`, `drop`,
    `takeField` — shadows the runtime's own member. Renaming the field would
    change the wire protocol, so the collision is reported instead.
- **A call that dispatches through a bound the engine cannot close goes through
  a generated run-time dispatcher.** `Ref::listen<L: IntoBroadcastListener<T>>`
  calls `listener.into_broadcast_listener()`, and `L` is open: at run time the
  listener may be a closure, an `Arc` holding one, a `BroadcastListener`, or a
  `Sender`, and Rust picks the impl per instantiation where a single emitted
  body cannot. The port writes one function per trait method that chooses by the
  receiver's shape — `instanceof` for a class the port emits and for the
  runtime's own wrappers, `typeof === 'function' || instanceof OwnedClosure` for
  a callable blanket, and the declared argument count of the function an `Arc`
  holds where two impls differ only in that. A receiver matching none is fatal,
  which rustc's having compiled the crate makes unreachable. Two limits stay.
  An impl written for a bare parameter with no bound the run time can see —
  `impl<T> Iterable<T> for T` — is the last branch rather than a test, so a
  receiver that is a `Vec` reaches the `Vec` impl even where Rust's *item* type
  would have chosen the blanket; the item type is erased and no test can see it.
  And two impls the run time cannot tell apart mean no dispatcher is written at
  all, and the site says which they are.
- **Expectations propagate one level and stop.** A `let` annotation, a return
  position, a call argument, a struct-literal field, the other operand of an
  equality assertion and an `unwrap` receiver each hand the expression under
  them a type to be. They do not reach further: `strings.into_iter().map(|s|
  s.try_into()).collect::<Result<Vec<_>, _>>()` settles the closure's `U` from a
  turbofish two calls downstream, which is inference across a chain rather than
  one position, so the engine says it cannot tell rather than answering with
  somebody else's open parameter. The positions that supply one now also include
  a free function's parameter (read from the signature the registry keeps for
  it), an enum variant's payload — `Err(e.into())` inside a function returning
  `Result<_, MutationError>` — the place an assignment stores into, and the
  operand opposite a binary operator.
- **An impl written for a reference to its own parameter, whose methods really
  forward, is not emitted.** `impl<T: Signal> Signal for &T` exists because
  `&T` is a distinct type in Rust, and each of its methods forwards to the same
  method on the `T` inside. Forwarding is checked against the body — one call to
  the method's own name, written either way `Signal::listen(*self, l)` and
  `self.0.listen(l)` are — because an impl for `&T` that does something of its
  own is a real impl, and skipping it left every call to it naming nothing.
  Emission erases the reference, so the value already carries the method;
  emitting the impl would write a function whose body calls itself.
- **A trait's method reached through a bounded parameter is emitted as a call on
  the value.** `T: Clone` and `T: Signal` dispatch through the trait's own
  declaration, and the emitted class implements the interface, so the runtime
  object has the method. Only where the impl the engine picked has no class of
  its own does the call become a module-level function.
- **An or-pattern whose alternatives read their names from different places has
  no test the translator can write.** `if let (Expr::Path(p), Expr::Literal(l)) |
  (Expr::Literal(l), Expr::Path(p)) = ..` binds the same two names from opposite
  positions, which needs a per-name conditional extraction. One site, in
  `core/src/reactor/watcherset.rs`; the alternatives that agree — two variants of
  one enum — are lowered.

- **Integer overflow is not wrapped, and division by zero does not panic.** Rust
  wraps in release and panics in debug; JavaScript's numbers grow without bound
  and its integer division by zero is `Infinity` (or `NaN`), where Rust's
  panics. The `as` casts and the bit operators DO wrap — that is the type's
  range, which the cast is about — but `a + b` on a `u8` does not, and `a / b`
  is written without a zero check. Step 7 decided to leave both rather than
  wrap every arithmetic operator: wrapping would put a mask on the hot path of
  arithmetic that never overflows in this corpus, and a zero check would put a
  branch on every division. Whichever way step 9 goes, it goes for both.
- **`/=` evaluates its place twice.** `a[i()] /= 2` is written
  `a[i()] = Math.trunc(a[i()] / 2)`, and a place with a side effect performs it
  twice. Rust evaluates the place once.
- **A `String` is ordered by UTF-16 code unit, not by byte.** A derived `Ord`
  compares strings with JavaScript's `<`, which orders by code unit; Rust
  compares a `String` by byte. The two agree below U+10000 and disagree on the
  order of an astral character against one in the surrogate range.
- **Rust's `&` and `|` on booleans evaluate both operands; the port's `&&` and
  `||` do not.** A right operand that is not a place is reported at the site.
- **`Clone`, `Any`, `Listener` and `Fn` appear in emitted signatures with no
  TypeScript spelling.** A trait the declared surface holds has no emitted
  interface, so a bound written in terms of one names something that does not
  exist. Step 7 removed the `implements` half of this (a class says `implements
  X` only for a trait this crate declares); the parameter and return positions
  remain, and are most of signals' unresolved-name errors.
- **A function whose body awaits is not always emitted `async`.** 45 sites in
  core say "'await' expressions are only allowed within async functions"; the
  `async` belongs on whatever function the emitter wrapped the body in.
- **`#[derive(Serialize, Deserialize)]` writes the bincode half and not the
  JSON half.** `encode`/`decode` are emitted; `toJSON`/`static fromJson` are
  not, and what error type an emitted `fromJson` answers with is a contract
  question that has not been put. The provided proto ids carry hand-written
  JSON, which is what `json_serde.test.ts` exercises.

## 8. Non-goals

General Rust inference; lifetimes and borrow checking; coherence and
specialization; const generics beyond array lengths; closures with neither an
annotation, an expected type, nor a callable bound to read one from; trait
objects beyond method dispatch on `dyn Trait`; typing expanded macro output of
any kind.

## 9. Questions put to Daniel, answered 2026-09-02

1. Integer widths. Daniel has no opinion beyond "as long as it works correctly in
   terms of serialization and behavioral parity with ankurah (rust)". Decided:
   `i64` and `u64` emit as `bigint`; the smaller integers and both floats emit as
   `number`; `bigint | number` (`name_map.rs:57` today) is neither and goes away.
   The engine keeps every integer as its own Rust type, and the bincode module
   takes the wire width from that Rust type, never from the TypeScript type. That
   is the width bug noted in the handoff: `bincode_module.rs:159` dispatches on
   the TS string, so every `number` encodes as `writeU32`, `u8` and `i32` and
   `f64` alike.
2. The `[system_types]` TOML table versus code (4.4). Answered: Rust stub
   declarations, parsed by the same extractor that reads ankurah's source.
3. `Debug` derives. Answered: emit a debug string built from the field names. A
   `{:?}` use is ordinary code, not a diagnostic.
