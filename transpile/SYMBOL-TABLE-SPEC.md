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

### 4.4a A projection that no impl settles is still a type

For: a generic body reaches values whose type is written as a projection and
nothing in that body says what the projection IS. `Predicate::populate<I: IntoIterator>`
calls `values.into_iter()`, whose return type Rust spells
`<I as IntoIterator>::IntoIter`, and then calls `.next()` on it. There is no
instantiation in sight and no impl to select, so 4.4's rule — "a projection that
does not normalize is a diagnostic" — reported the receiver and stopped. But Rust
does not need the impl here either: `IntoIterator` DECLARES what its `IntoIter`
is good for, `type IntoIter: Iterator<Item = Self::Item>`, and that declaration
is what makes `.next()` legal in a body that never learns which iterator it is.

So a projection is a type in its own right, and it carries the bounds its
declaration gives it.

- **What it is.** `Assoc{base, trait_, name}` normalizes through the impl table
  where the table can answer (4.4). Where it cannot, the projection STANDS —
  which is the truth about it — and its bounds are the bounds the declaring
  trait wrote on that associated type, instantiated for this projection:
  `Self` is the projection's own base, and the trait's parameters are the
  arguments the bound at the use site was written with. `<I as IntoIterator>::IntoIter`
  where `I: IntoIterator<Item = V>` therefore carries `Iterator<Item = V>`.
- **What can be done with it.** Exactly what its bounds declare, which is the
  same rule a bounded type parameter already follows (4.2): a method call on it
  resolves through the trait method the bound names, and its return type is that
  signature's, substituted. Nothing else: a projection with no bounds answers no
  method, and the site says so as it does today.
- **Where it resolves.** At an instantiation site the engine sees the concrete
  type, the impl table answers, and the projection is replaced before any of
  this is reached. This section is about the body that is compiled ONCE for
  every instantiation, which is the only body the port emits.
- **What it does not do.** A bound written with a `where` clause on the impl
  rather than on the associated type's declaration is not read here, and neither
  is a bound a supertrait puts on the same name; both are recorded in 7a rather
  than guessed at. An associated CONST is not a type and is not covered.

The declared std surface is where this is written down for std: the stubs
already say `type IntoIter: Iterator<Item = Self::Item>;`, and the extractor now
keeps that bound instead of keeping only the name.

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
- A name a pattern binds is typed only INSIDE that pattern's scope. A question
  about what an arm takes — `taking::taken`, and everything that reads it — has
  to be asked there; asked outside it, a binding the engine could type reads as
  one it could not, which is the answer that refuses (R5, step 9a slice 8).

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

- **Arithmetic on a fixed-width integer costs a call (R7).** The port mirrors
  the `debug_assertions = true` build, and that build PANICS on overflow:
  `u8::MAX + 1` is `attempt to add with overflow`, not `0`. JavaScript wraps
  nothing and saturates nothing — it goes on counting in doubles, silently
  losing precision above 2^53 — so a bare `a + b` was a third answer, neither
  Rust's release wrap nor Rust's debug panic. `+`, `-`, `*`, `/` and `%` on an
  integer therefore go through `@ankurah/base`'s `checkedAdd`, `checkedSub`,
  `checkedMul`, `checkedDiv` and `checkedRem`, each of which takes the width by
  name and raises where Rust panics; division and remainder by zero raise as
  Rust does, and integer division truncates towards zero as Rust does.
  `wrapping_*`, `checked_*`, `saturating_*` and `overflowing_*` map to FREE
  helpers of the same shape — `wrappingAdd(x, 1, 'u8')` — because a JavaScript
  number has no methods of those names. The `checked_*` helpers answer
  `T | null`, which is how the port writes the `Option` Rust answers, and
  `checkedDivOption` and `checkedRemOption` answer `None` on exactly the two
  cases their panicking siblings raise on: a zero divisor, and `MIN` over `-1`,
  whose quotient the type cannot hold. The width comes from the resolved
  receiver type, and a receiver the engine could not type is a HOLE rather than
  a guess, because the answer differs by width. Floats are untouched — Rust's `f64` arithmetic is
  IEEE and so is JavaScript's.

  R13: `usize` and `isize` are 32-bit here, because the port's target is wasm32.
  One table in `@ankurah/base`'s `ops.ts` gives the range and the wrap width, so
  the two cannot disagree; the 8 bytes those types occupy on the bincode wire is
  a separate fact and belongs to the codec.

  An ATOMIC wraps rather than panicking. Rust's `fetch_add` and `fetch_sub` are
  defined to wrap at the width whatever the build's debug assertions say —
  `AtomicU32::MAX.fetch_add(1)` stores `0` — so those two go through
  `wrappingAdd` and `wrappingSub` with the atomic's own width, where a
  `static mut` beside them goes through `checkedAdd`. The two spellings of one
  idea used to disagree: the atomic went on counting in a double. `AtomicU64` is
  the one atomic this cannot be written for, because the port spells it a
  `number` and Rust holds a `u64` in it — a `bigint` operand beside a `number`
  place is something JavaScript refuses to mix — so its update stays an ordinary
  `+=` and the site says so.

  A width the port spells `number` can still be handed an answer past
  `Number.MAX_SAFE_INTEGER`. The helper PANICS there rather than returning a
  rounded double: a rounded answer is a wrong number the program then computes
  with, and Rust has no such case to mirror.

  The cost is one call per operation, and the emitted expression is a call
  rather than an operator wherever the answer is not provably in range. The
  emitter skips the helper only where the ANSWER is provable: two decimal
  literals whose result fits, and two array lengths added in a 64-bit type
  (a JavaScript length is below 2^32). Not where the OPERANDS fit —
  `255 + 1` on a `u8` has two operands that fit and an answer that does not.

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
  (`packages/base`), not a transpiler one. **Building one all at once is not
  growing one** (step 9a slice 5, item 2): `collect` into a `Vec<u8>` target
  writes `Uint8Array.from(<the sequence>)`, the same buffer `vec![..]` has
  always built, because `FromIterator` is decided by the TARGET and a byte
  target is a byte buffer. The arm that answers `collect` used to hand the
  adaptors' own array back for a byte target as well as for every other `Vec`,
  which put a `number[]` behind three `Result<Vec<u8>, IndexError>` returns in
  `core/indexing/encoding.ts`. What is still reported there is the growable
  half in the same file — `Vec::with_capacity` followed by `push`, four
  functions of it — and those returns are still arrays, behind their own
  diagnostics.
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
  - **A move FLAG the body never sets is not written.** (Step 9a slice 5, item
    12, E15.) The disposition analysis reads the SOURCE, and a move it finds may
    be one the lowering did not write — an `if let Some(x) = value` binds a name
    out of the option without the emitted arm assigning anything — which left a
    `let _movedN = false;` nothing assigns beside an `if (!_movedN)` that is
    always false. What the block really did is what the block really wrote, so
    the flag and its declaration go and the release stands unguarded. Three
    live sites: two in `storage-indexeddb/collection.ts` and one in
    `core/value/cast_predicate.ts`.
  - **A statement's move FLAG stands after everything the statement evaluates**,
    for every call shape (step 9a slice 5, item 10, E10). The flag is written
    before the statement, because after a `return` it would be dead code; that
    is right for the move and wrong for an argument that can THROW on the way to
    it, which left the flag set and the moved value released by nobody. The rule
    reached `invoke(..)` alone until now, because it asked whether the CALLEE
    was a path naming a flagged local; what decides it is whether THIS CALL
    hands a flagged local away, which is the same question the flag assignment
    asks. Three conditions keep it sound. §3.9: the arguments go above the flag
    only where the LIST cannot contain the move, all or none, because lifting
    some would also reorder what Rust evaluates. A place, a literal, a closure
    and an emitted text that is a bare NAME stay where they are, because
    evaluating one cannot throw. And the call must stand where the STATEMENT's
    own expression stands: the lift is written above the whole statement, so an
    argument of a call nested inside a branch, a closure or an IIFE the
    statement writes could read a name that block declares — two live sites,
    `storage-indexeddb/collection.ts` and `signals/signal/calculated.ts`, showed
    it.
  - **A method's RECEIVER that is a field of a place** is taken out of the
    struct where the method is declared `self` (step 9a slice 5, item 6) — the
    same `takeField` a `let x = s.field` writes. What is left is a field the
    runtime writes as a plain value, an array or a `Map`: `takeField` is
    `AkObject`'s and those are not objects of its, so the read hands the same
    value to two owners and the site says so. Eleven sites in core.
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
    droppable values is an `OwnedClosure`, which is never callable as a function,
    so every call on one is written through a helper that tells the two shapes
    apart. Three shapes reach a helper: a closure bound to a local (`.call`/
    `.callOnce`), a parameter with a callable bound (R10's `invoke`/`invokeRef`),
    and — since 2026-09-05 — an argument to an `Option` combinator or to
    `retain`, which is named once, reached through the helper its bound calls
    for, and released by the branch that does not call it. A fourth since step
    9a slice 5 (F9): the callback of an iterator terminal, which every helper in
    `std/iter.ts` and `std/iter_owned.ts` reaches through `invokeRef` and
    releases when the call ends — Rust's terminals take their `F` by value.
    (Rewritten 2026-09-05, Z14: this bullet used to say the callee that receives
    a closure "still writes `f(x)`", which the emitter has not done since R10.)
    What is left is a callee whose parameter the engine cannot read AS a
    callable — a type it could not resolve, or one that is not a bound at all —
    where the call is written plainly and reported.
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
  - **A loop over an owned sequence the runtime does not write as an array
    releases the CONTAINER for nobody, and does not release what an early exit
    left behind.** Rust's `IntoIter` takes the sequence by value: it drops the
    elements it never handed out, and it drops the emptied sequence itself when
    the loop ends. The elements the loop DID hand out are released per turn — it
    is the two the loop never touches that are not. A `Vec` is an array and the
    emitter can name both; a `HashMap` or an adaptor is not, and releasing the
    map after the loop would release the keys and values the loop has already
    taken, because the runtime has no way to empty one without them. The
    `borrowed_iteration` golden records the leak by name.
    (Made precise 2026-09-05: the bullet used to say "neither what an early exit
    left behind nor the CONTAINER", which read as though the elements handed out
    were not released either.)
  - **A macro the emitter does not expand releases nothing it was handed.** The
    macro becomes `undefined` with the source in a comment beside it, so a value
    passed into one goes with it. It is `undefined` rather than the comment
    alone because a comment is not an expression: one written where an argument
    stood did not parse, and a file that does not parse stops the TypeScript
    compiler from checking anything else in the project.
  - **A closure that moves ONE capture and reads another drops neither.**
    (Rewritten 2026-09-05: the gap this bullet used to record — a closure whose
    body hands a capture away owning nothing of it — is closed. The runtime has
    `OwnedClosure.callOnce`, which marks the closure moved before running the
    body, and `$consumesCaptures`, which the emitter sets from the same question
    it already asks to choose `call` from `callOnce`; `invoke` calls a reading
    closure and then drops it, so its captures' glue runs once.) What is left is
    that `$consumesCaptures` is ONE boolean for the whole closure. A body that
    hands `a` away and only reads `b` sets it, `callOnce` marks every capture
    transferred, and `b` is released by nobody. Closing it needs a per-capture
    disposition — a flag per capture, or a `callOnce` that releases the ones the
    body did not move.
  - **An `if let` that takes a payload out of a wrapper leaves the wrapper.**
    The path where the pattern did not match releases the value it tested, as a
    `while let` does; the path that matched has taken the payload, and marking
    the wrapper moved is what `intoMatch` does and an `if` cannot. An
    `Option<T>` has no wrapper — it is `T | null` — so only an enum reaches
    this.
  - **Two `From` impls for one target that differ only in a reference are two
    functions, and a call reaches the owned one.** (Rewritten 2026-09-05: the
    gap this bullet used to record — the second impl dropped, the site saying so
    — is closed. R8 made the conversion's identity its RUST source, and a
    reference to a type the port gives a class to marks the name, so
    `impl From<Ref<T>> for EntityId` and `impl From<&Ref<T>> for EntityId` emit
    `EntityId_fromRefT` and `EntityId_fromRefRefT`. Both bodies are kept, which
    matters: four of the corpus's six pairs really do differ — the borrowed form
    clones what the owned one moves, or the owned form releases what the
    borrowed one must not.) What is left is the CHOICE at the call site:
    `(&r.id).into()` is written against the owned static, because the resolution
    peels the reference before it looks the impl up. Six emitted `..Ref..`
    functions therefore have no caller, and where the pair's two Rust bodies are
    identical — `proto/id.rs`'s `From<EntityId> for String` and
    `From<&EntityId> for String`, and the same pair for `ankql::ast::Expr` — the
    two emitted functions are identical too. They stay: making them one function
    would be reading identical TypeScript bodies as one conversion, which is the
    rule R8 retracted.
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
- **Expectations propagate one level, and THROUGH a closed list of transparent
  forms** (step 9a slice 7, item 12; G3). An adaptor whose result type fixes its
  operand's payload says exactly as much about that operand as the position says
  about the whole, so an expectation that stops at it is a fact thrown away:
  `let bytes: [u8; 32] = id_bytes.try_into().map_err(..)?;` is the only thing
  that says which `TryFrom` impl `try_into` picks, and Rust picks it by the
  TARGET type. The list is closed and is this one — `?`, `unwrap`, `expect`,
  `ok`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `map_err`, `ok_or`,
  `ok_or_else`, and anyhow's `context` and `with_context` — and a form not on it
  stops the expectation as every form used to. `into` and `try_into` are not on
  it because they are not adaptors: their TARGET is the expectation.

  Two kinds, and the difference is observable. One OPENS the wrapper, so what
  the position wants of the whole is what it wants of the payload; the other
  keeps a wrapper of the same payload, so the expectation's own payload is what
  passes through. `o.ok_or(e)` is the second read backwards — the whole is a
  `Result<T, E>` and the receiver an `Option<T>` — and taking the expectation
  whole would ask the `Option` for a `Result`. The wrapper the operand is asked
  for is built from the RECEIVER's own type rather than guessed at.

  This is the bounded-inference principle extended, not repealed: the list is
  finite, written down, and each entry is a form whose Rust signature makes the
  propagation exact. Inference across a chain — `strings.into_iter().map(|s|
  s.try_into()).collect::<Result<Vec<_>, _>>()`, where a turbofish two calls
  downstream settles the closure's `U` — is still refused.

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
  operand opposite a binary operator. Four more since (step 9a, third slice): a
  TUPLE STRUCT's field, including through `Self(..)` inside the type's own impl;
  a MATCH ARM and an `if` BRANCH in return position, each re-keyed onto its own
  span because whatever it produces is what the function answers; and a field of
  an enum-VARIANT literal, whose fields live on the variant and not on the enum.
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
  its own does the call become a module-level function — and only where the
  trait is declared in THIS crate, because a trait declared elsewhere carries
  its dispatcher there and this run writes none. Both call sites ask that now
  (step 9a slice 5, item 5): the one that resolved to a blanket impl always did,
  and the one that resolved to the trait's declaration wrote
  `TryInto_dispatch_tryInto(..)` — a name nothing declares, five sites across
  core and storage-indexeddb — until it did too.
- **A bound the caller DECLARED beats a blanket impl that rests on an undecided
  one.** (Step 9a slice 5, item 5, G1.) `impl<I: Iterator> IntoIterator for I`
  matches every receiver and leaves `I: Iterator` open, and the rule that a
  written impl is more precise than a declaration then discarded the caller's
  own `fn f<I: IntoIterator>(values: I)` — which SAYS that `I` implements the
  trait. So `values.into_iter()` resolved through the blanket, deferred an
  obligation, and came out as `values.intoIter()`, a method nothing declares.
  An impl whose own bounds HOLD still wins; one that only applies if something
  nobody can decide holds does not. The change removed 34 deferred obligations
  across seven crates, took signals' count of them to zero, and added six
  reports where a wrong answer used to stand — five dispatchers named above and
  one `next` on `<I as IntoIterator>::IntoIter`, which used to resolve through
  `futures::StreamExt`.
- **A `&` on a RECEIVER is not erased before the method probe.** (Step 9a slice
  5, item 5, E11.) `&x` as an EXPRESSION types as `x` — emission erases borrows,
  and every reader downstream is written against the value — and the same
  erasure ran in front of the probe, so `(&v).into_iter()` started at `Vec<T>`
  and found the by-value impl whose `Item` is owned. The loop then released
  every element the caller still held: a double drop where the block released
  them too. The borrow is put back for the probe alone, and the deref chain
  takes it off again where the method really is the by-value one. A parenthesised
  receiver is read through, because Rust's probe reads the expression and not
  its punctuation.
- **`iter_mut` is the sequence itself, and is refused over a value the port
  copies.** (Step 9a slice 5, item 5, F4/E12.) It had no lowering at all and came
  out as `xs.iterMut()`, a method no array and no map declares — a `TypeError`
  the first time the loop is reached, live at `core/node.ts:838` and
  `core/property/backend/lww.ts:142`. Rust hands out `&mut T`; the port has no
  `&mut`, so a loop writes THROUGH only because the variable and the slot are
  the same object. Over a number, a string, a `bigint` or a `char` the variable
  is a copy and the write is lost, so `iter_mut` and a map's `values_mut` over
  such an element are a hole (R12) rather than a silent no-op. The disposition
  is BORROWED either way: `iter_mut` takes `&mut self` and the elements stay the
  caller's.
- **`IntoIterator::into_iter` on a type PARAMETER is the spread.** (Step 9a
  slice 5, item 5, G1.) The port materialises an iterator as a JavaScript array,
  which is what makes `map`, `filter`, `rev` and `contains` array operations
  here, so `into_iter` is the spread on every receiver — a `Vec`, a map, a set
  and a receiver the engine could not name at all. A bare type parameter fell
  between those arms, because `js_shape` says `Plain` for one. The resolution is
  what says this is `IntoIterator` and not a crate method of the same name, and
  a resolution still carrying obligations is left alone: answering one would be
  guessing AND would silence the report.
- **An or-pattern whose alternatives take their names out of a form the
  translator cannot read back refuses in the BRANCH.**
  (Rewritten 2026-09-05: the gap this bullet used to record — an or-pattern whose
  alternatives read their names from DIFFERENT PLACES having no test the
  translator can write — is closed. Rust requires every alternative to bind the
  same SET of names and says nothing about the order, so each name is looked up
  by name in every alternative and read from whichever one matched;
  `core/src/reactor/watcherset.rs`'s `(Expr::Path(p), Expr::Literal(l)) |
  (Expr::Literal(l), Expr::Path(p))` is written.) What is left is an alternative
  whose binding is not a name or a field list — a tuple or a slice
  destructuring, `Inner::A((a, b)) | Inner::B((b, a))` — which the reader that
  pairs the alternatives up cannot parse. That one refuses: the TEST is still
  written, so a value the pattern does not match reaches the arms below it, and
  the branch's first statement is the refusal. No corpus site reaches it.
  A SECOND case is open and not refused: alternatives that name the same
  VARIANT — `Both::Two(t, _) | Both::Two(_, t)` — claim the binding once per
  alternative, so the arm writes `t.drop()` in two nested `finally`s and drops
  it twice. Pre-existing; no corpus site reaches it either.

- **A dynamic shift past the width is JavaScript's masking, not Rust's panic.**
  `x << n` with `n` a value rather than a literal is `x << (n & 31)` in
  JavaScript, and `attempt to shift left with overflow` in the build this port
  mirrors. The three other overflow cases the same debug build panics on —
  arithmetic, division by zero, remainder by zero — go through R7's helpers and
  do panic; the shift amount is the one left. It belongs with R7 when it lands.
- **A compound assignment into an INDEX evaluates the index twice.**
  (Rewritten 2026-09-05: the bullet this replaces said `/=` evaluates its place
  twice, and that is no longer true of the form the corpus writes. `*place op=
  value` binds the place once — `const _m0 = m.entry(k).orInsert(0); _m0.value =
  checkedAdd(_m0.value, 1, 'i32')` — so an `entry` is made once and its key
  cloned once.) What is left is the INDEX form: `a[i()] /= 2` is written
  `a[i()] = checkedDiv(a[i()], 2, 'i32')`, so an index expression that calls
  something calls it twice. A field or a local index is not this — reading
  `s.f` or `a[n]` again reads the same storage and runs nothing, which is the
  same reason Rust may read it once. No corpus site writes a call inside the
  index of a compound assignment.
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
- **A `use` inside a body is hoisted only where the module does not already
  claim the name.** (Step 9a slice 5, item 11, E8.) Rust scopes such a `use` to
  its block and the engine's binding table is per module, so it is hoisted —
  widening its scope — and that is only safe while nothing else in the module
  claims the name. "Claims" means BINDS or DECLARES: the check read the module's
  other `use` items alone, which is not what either doc comment said, so a
  module declaring `pub struct Kind` whose body wrote `use crate::far::Kind;`
  had the far one in its table. (Lookup already preferred the module's own
  declaration, so no emitted line changed; what changed is that the table and
  the doc now say the same thing, and cannot drift.) A name TWO bodies bring in
  from two different places is claimed by neither and BOTH are reported (§3.6):
  hoisting both left the module's one table holding the first, so the second
  body silently meant the first body's type — `new Wrap(undefined)` with only
  one of the two sites reported. The rationale is written once, in
  `registry/uses.rs`, and `extract/uses.rs` cites it (E18).
- **An arm is cast to `any` where the ARM writes a tuple, and nowhere else.**
  (Step 9a slice 5, item 12, E16.) TypeScript takes a `match`'s result type from
  the first arm it reads, and a tuple written in one arm makes every later arm
  an error against it. Whether an arm wrote one is the arm's own question:
  asking the emitted TEXT whether it starts with a bracket cast
  `[...exprs].every(p)` — a boolean — at `storage-sqlite/sql_builder.ts`, a
  `vec![b as u8]` at `core/value/collatable.ts`, and two `every` calls at
  `core/collation.ts`. `Some((a, b))` still counts, because `Option<T>` is
  `T | null` and the arm really does write the array; every other wrapper is an
  object of its own.
- **`rev` copies the sequence first, unless the port built it on the spot.**
  (Step 9a slice 5, item 12, E17.) `Array.prototype.reverse` mutates, and Rust's
  `rev` leaves its receiver alone — so a copy stands in front of it. An array
  the emitter just wrote (`range(..)`, `rangeIncl(..)`, `stepBy(..)`,
  `iterFilterMap(..)`, a spread) is held by nobody else and has nothing for
  `reverse` to mutate out from under: eleven emitted sites copied a range the
  line above had allocated.
- **A same-leaf type in two modules of one crate is reported, not aliased.**
  The port flattens a crate's modules into one package surface, and a file's
  import list is keyed by the LEAF name — so a file naming `left::Wrap` and
  `right::Wrap` imports one of them and writes the bare name for both, and a
  signature against the other names the wrong class. The crate INDEX already
  tells them apart (`left_Wrap`, `right_Wrap`); a file's own imports would need
  a per-file alias map threaded through `map_ty`. Until then the site says so.
  Ten corpus sites: `State` three times in core, `IVec` twice, `ListenerGuard`
  twice in signals.
- **An import list is decided by the names the emission WRITES.** (Rewritten
  2026-09-06, slice 4 item 1. The bullet this replaces said an import may name
  a type the emitted text never writes — 34 of them — and called it tidiness.
  It was not: the same scan also wrote imports for names it read INSIDE STRING
  LITERALS.) Every list — `@ankurah/base`, the cross-crate one, the intra-crate
  one and a test file's — is built from the identifiers the emitted text writes
  as names, lexed by `codegen/written.rs` so that a string literal, a template
  literal's text and a comment contribute nothing and a name after a `.` is the
  member read it is. Before, the lists split rendered text on non-word
  characters: the `collect` refusal names the iterator types it could not build,
  so `storage-indexeddb/collection.ts` imported `Iter`, `SortedStream` and
  `TopKStream` from `@ankurah/core`, which exports none of the three, and the
  module did not load. A `use` hoisted out of a body (§3.4 of slice 3) imported
  its name whether the lowering reached it or not. Unused imports across the
  emission: 34 → 10. `transpile/tests/import_gate.rs` is what says the answer
  resolves — it lays the emitted crates out as packages, with each package's
  declared hand-written half under the emitted output, and runs `bun build`
  with NO `--external` over every emitted module; `tests/import_gate.toml`
  ledgers what is left, matched exactly in both directions.
- **A pattern that takes a DROPPABLE name out of a payload member is refused,
  and the same rule now answers for every element.** (Added 2026-09-06, slice 4
  item 4: K4, K5, K12, K15, K16.) `Outer::W(Inner::X(t))` binds `t` out of the
  `Inner` the `W` variant holds, and the port cannot release an object minus a
  field — so nothing releases the `Inner`. The `Result` side had always refused
  that shape; the plain enum arm merely left the member out of its `dropUnbound`
  list and carried on, so what the pattern did not take LEAKED with no word
  said. One rule, `match_expr/taking.rs`, now answers per element and through
  the wrappers a pattern may be written behind (`|`, parentheses, `&`): a name
  for the whole element owns it, a pattern that takes nothing or reaches inside
  without taking anything droppable leaves the element for the arm to release,
  and one that takes a droppable name (or binds a name the engine cannot type)
  is an R12 hole with the payload released before it throws. Where the arm is a
  chain LINK the refusal stands in the branch, so a value the pattern does not
  match still reaches the arms below it. `Some(x)` is the exception the rule
  names: `Option<T>` is `T | null`, so `x` IS the member and taking it takes all
  of it. A tuple's ELEMENTS are answered one at a time too — for what the arm
  owns, and for whether each is borrowed, which the tuple itself cannot say
  (`(&*left, &*right)` is not a reference even though both of its elements are).
  No corpus site takes the refusal today; `goldens/payload_taking` is the five
  shapes with a driver.
- **A range is the sequence of its values, and an unbounded one is a hole.**
  The port has no `Range` type. `a..b` is materialised, which is what makes
  `rev`, `map`, `filter` and `step_by` work on it, because those are all array
  operations here; the corpus's ranges are small (`0..16`, `0..MAX_RETRIES`,
  `0..bytes.length`). `..n`, `a..` and a range over a width the port holds in a
  `bigint` have no sequence to build and are refused. A `BTreeMap::range(a..)`
  — an ordered-map range query — is two of those refusals.
  **Which widths are built is a whitelist now** (step 9a slice 5, item 8, F3):
  the discrete integer widths `n++` steps — `u8`, `u16`, `u32`, `usize`, `i8`,
  `i16`, `i32`, `isize`. The check used to name the one width it could NOT
  count and let everything else through, so `('a'..='c')` came out as
  `rangeIncl('a', 'c')`, which is `["a"]` because `'a' + 1` is the string
  `"a1"`, and a float range came out as a one-element array. A `char` range is
  the sequence of its code points and is refused; an endpoint the engine could
  not TYPE is left alone, because that is the engine's own gap and refusing it
  would take out `for attempt in 0..MAX_RETRIES` over a function-local `const`.
  **`contains` is written from the BOUNDS**, not from the sequence: it is a
  comparison against the two ends, it is the one method a range the port cannot
  count still answers — a float range is not an iterator in Rust either — and
  written through the sequence `(0.0..1.0).contains(&0.5)` was
  `range(0, 1).contains(0.5)`, a `TypeError` no diagnostic named. **`step_by`
  is `stepBy(xs, n)`** on the materialised sequence (E7); it had no lowering and
  came out as `xs.stepBy(..)`, a method no array declares.
  **A range is materialised even where only its LENGTH is read** — `(0..1_000_000).len()`
  is `(range(0, 1000000)).length` (E9). Acceptable while the corpus's ranges are
  small; recorded here rather than fixed.
- **A hand-written GENERIC prints its payload from the value's own surface, and
  a `char` instantiation is reported.** (Step 9a slice 5, item 9, F7.) Every
  other Debug rendering is decided from the resolved `Ty`, which is what makes a
  Rust `String` print quoted and a Rust enum print its variant name even though
  both are a JavaScript string. `Attested<T>` has no `Ty` at the payload's
  position — `T` is whatever the instantiation put there and the file is
  hand-written — so the payload goes through `@ankurah/base`'s `debugValue`,
  which reads the value: a string is a Rust `String` and prints QUOTED, a number
  and a bigint print as themselves, `null` is `None`, a sequence prints
  element-wise, a byte buffer as its bytes, an object declaring `debug()`
  through it, and anything else is REFUSED by name rather than printed
  `[object Object]`. The gap the ruling names is a `char`: the port writes one
  as a one-character string, which is what a `String` is too, and Rust prints
  those differently — so a provided generic instantiated with `char` is
  reported at the Debug site instead. A float payload is the same erasure in
  miniature: `1.0f64` prints `1` there where the emitter, which has the type,
  writes `1.0`. No corpus site instantiates one with either.
- **A derived `Debug` writes what rustc writes, with two shapes it did not.**
  (Step 9a slice 5, item 9, F6, E6.) A ONE-tuple keeps the comma that tells it
  from a parenthesised value — `(7u32,)` is `(7,)`. A `char` is quoted AND
  escaped the way `char::escape_debug` escapes it — `'\''`, `'\\'`, `'\n'` —
  through `debugChar`, where the port used to write the quotes and print what
  was inside them raw. What is left is the ORDER a `BTreeMap` and a `BTreeSet`
  render in: Rust's is key order, the port has no ordered container, and the
  runtime's map iterates in insertion order. There is nothing at the rendering
  to sort by — the `Ord` the keys are sorted with is not a value it holds — so
  the gap stays reported where the container is CONSTRUCTED, which is where it
  can be fixed (fixpass1's accepted choice). Live at `proto/data.ts` 326 and
  633. The doc comment that used to claim the container iterated in key order,
  "which is what the ordering note in the container says", said the opposite of
  what that note says and is gone.
- **A reader answering `Option<Element>` is refused where the ELEMENT is itself
  an `Option`.** (Step 9a slice 5, item 8, E13.) `Option<T>` is `T | null`, so
  `first`, `last`, `get`, `pop`, `find`, `reduce` and the `max_by`/`min_by`
  families over a `Vec<Option<T>>` have ONE `null` for two different answers:
  "there is no element" and "the element is `None`". Rust tells them apart and
  every caller is written expecting that, so the call is a hole (R12) rather
  than a flattening. The type spelling `Option<Option<T>>` was already reported;
  the READER was not. No corpus site takes it.
- **A consuming iterator terminal OWNS the elements it walks, and a named
  iterator is refused.** (Step 9a slice 5, item 3, F1.) Rust's
  `into_iter().find(p)` hands back the element it selected and drops every other
  one; `max_by_key` drops both losers; `position` moves each element into the
  closure. The port wrote all of them as the reading helpers of `std/iter.ts`,
  which release nothing, so a consuming chain either released the element it had
  just handed back — where the emitter had also hoisted the sequence and given
  it a `dropOwned` — or leaked everything it had not. Which of the two depended
  on whether Rust's signature says `self` or `&mut self` about the ITERATOR,
  which says nothing about the items. Ownership is now part of the lowering:
  `std/iter_owned.ts` is the same terminals over a sequence the expression owns,
  and the emitter writes those names when the resolution came through
  `Iterator` — `slice::last(&self)` is not `Iterator::last(self)`, and the two
  are one word apart — the elements have drop glue, and the receiver is not a
  place. `first` and `get` have no consuming form at all. The gap that remains
  is the NAMED iterator (`let mut it = v.into_iter(); it.find(..)`), which the
  call consumes only part of: the port writes an iterator as the whole array, so
  afterwards it cannot say which elements are still the caller's, and the call
  is a hole (R12) with the block keeping the receiver (J4). One question answers
  both the move scan and the lowering (`terminal_owns_the_sequence`), so they
  cannot disagree about who releases what. **No corpus site takes either path
  today**; `goldens/owned_terminals` is the nine shapes with a driver.
- **`#[derive(Serialize, Deserialize)]`'s JSON half is refused for a type whose
  provided parts do not declare it.** (Rewritten 2026-09-05: the bullet this
  replaces said the JSON half is not emitted at all. It is: `encode`/`decode`
  and `toJSON`/`static fromJson` are all written, and an emitted `fromJson`
  answers `Result<T, JsonError>`.) What decides WHICH types get the pair is
  fixpass3's §4.2: a `[provided_impls]` entry says whether its hand-written file
  declares `fromJson`, `transpile/tests/declared_members.rs` reads the file and
  fails if the claim is false, and a generated type that reaches a provided type
  which declares neither half has both of its own halves refused — the pair is
  refused as one, because a `toJSON` with no `fromJson` writes text nothing can
  read back. Seven proto types carry neither half for that reason.
  **What "declares" means is the DECLARATION, not the text** (step 9a slice 5,
  item 1): the claim is checked against a member written at the class body's own
  top level, of the KIND the emission calls — a `static fromJson(..)` for
  `Class.fromJson(v)`, an instance `toJSON(..)` for `value.toJSON()`, an
  instance `debug()` taking nothing for `value.debug()`. The check used to ask
  whether the class body CONTAINED the text `debug()`, which any `x.debug()`
  inside any method satisfied and which a `static debug()` satisfied too; both
  leave the call the emitter writes undefined. `tests/common/members.rs` is the
  small member reader, and it does not model a method written as a field holding
  an arrow function — such a file's claim FAILS, which is the safe direction.

- **An iterator is the whole array, and no cursor stands in it** (step 9a slice
  7, item 3). Rust's iterators are lazy and hold a position; the port
  materialises the sequence eagerly, so every adaptor is an array operation over
  the whole of it. That answers most shapes and refuses two.

  What it answers: an adaptor that DISCARDS elements owns what it discards, and
  the port drops it where Rust does — `filterOwned`, `skipOwned`, `takeOwned`
  and `stepByOwned` over a chain whose elements the chain owns, and the plain
  array operations over a borrowed one. `map` and `flat_map` are not among them,
  because they hand each element to a closure BY VALUE and a closure's by-value
  parameters are locals of its body, so the closure is what releases them.

  What it refuses: a PARTIAL walk that leaves a tail behind. `it.find(..)` and
  `(&mut it).find(..)` on a named iterator, and `it.next()`, consume some
  elements and leave the rest in `it` — and after the call the port cannot say
  which of the array's elements are still the caller's, so neither release is
  writable and the call is a hole. `it.by_ref().find(..)` is the same shape
  under another spelling and gets the same refusal. The one partial walk that IS
  written is `next` on a receiver the expression just built: nobody else holds
  that sequence, so the call answers the head and the rest goes with the
  iterator at the end of the statement.

  **The durable answer is a cursor representation** — a sequence paired with an
  index, where an adaptor advances the index and the pair keeps the unwalked
  tail until the chain itself is dropped. That would turn every refusal above
  into a lowering, and it is the direction to take when a corpus site makes one
  of them matter. It is not free: every helper in `std/iter.ts` and
  `std/iter_owned.ts` takes the pair instead of an array, and every emitted call
  that hands a sequence to a hand-written callee has to say which of the two it
  is handing over.

- **The binding table is per MODULE, and Rust's `use` is per block** (step 9a
  slice 7, item 11). A `use` written inside a function body is hoisted to the
  module, because that is the only table there is — but only where the module
  does not already claim the name, since widening a name's scope must not change
  what another body means by it. Three shapes fall out of that, and each is
  LOUD rather than guessed at:

  - a body `use` whose name the module already DECLARES is not hoisted, and the
    declaration wins. That is valid Rust the port does not model — Rust scopes
    the body's `use` to its block and the two names coexist — and the body's own
    uses of the name are reported where they resolve to the declaration
    (O15). No corpus site.
  - two bodies bringing in different types under one name are both reported and
    neither is hoisted, because one table cannot hold both.
  - a GLOB a body wrote is never hoisted, because widening a glob widens every
    name it could ever bring — and it is reported, so the names it would have
    brought do not draw false reports of their own ("no field `v` on `Other`")
    (N20).

  Each of those reports carries the `use`'s own span. Written at
  `Span::call_site()`, a report about a `use` reached the reader with no file
  and no line at all.

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
