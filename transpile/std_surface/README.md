# The declared std and extern surface

## What this is for

The type-resolution engine has to answer Rust type questions about ankurah's
code — which method a receiver resolves to, what it returns, which deref steps
were inserted along the way — and most of those questions end at a type ankurah
did not write. `self.listeners.read().unwrap().values().cloned().collect()`
touches `RwLock`, `LockResult`, `Result`, `RwLockReadGuard`, `HashMap`,
`Values`, `Cloned`, `Iterator` and `Vec` before it produces a type, and the
engine can only answer if it knows all nine.

These files are that knowledge, written as ordinary Rust. They are parsed by the
same syn-based extractor that reads ankurah's source, so a std type reaches the
registry the same way a crate type does, is exercised by the same tests, and
fails the same way. They replace the `[system_types]` table in
`transpile.toml`, which could describe neither bounds nor associated types and
which — because it was written against the TypeScript polyfills rather than
against Rust — described `Mutex::lock` as returning the guard. It does not.

A stub says what Rust says. What a std item becomes in TypeScript is a separate
question, answered where it already is: in `name_map` and the per-type emission
modules.

## Versions

Every declaration here is against a specific version. `rust-toolchain.toml` in
the Rust checkout pins `channel = "nightly"`, which resolves locally to
**rustc 1.97.0-nightly (ca9a134e0, 2026-04-26)**; the extern crate versions are
the ones `Cargo.lock` locks. When a signature changed across versions, the
locked version decides.

| crate | locked | crate | locked |
| --- | --- | --- | --- |
| rustc | 1.97.0-nightly | `anyhow` | 1.0.100 |
| `tokio` | 1.48.0 | `serde` | 1.0.228 |
| `futures` | 0.3.31 | `serde_json` | 1.0.145 |
| `wasm-bindgen` | 0.2.106 | `bincode` | 1.3.3 |
| `web-sys` | 0.3.83 | `serde-wasm-bindgen` | 0.6.5 |
| `js-sys` | 0.3.83 | `wasm-bindgen-futures` | 0.4.56 |
| `yrs` | 0.24.0 | `ulid` | 1.2.1 |
| `bb8` | 0.9.1 | `base64` | 0.22.1 |
| `rusqlite` | 0.32.1 | `sha2` | 0.10.9 |
| `pest` | 2.8.4 | `gloo-timers` | 0.3.0 |
| `itertools` | 0.14.0 | `append-only-vec` | 0.1.8 |
| `indexmap` | 2.12.1 | `send_wrapper` | 0.6.0 |
| `rand` | 0.8.5 | `async-trait` | 0.1.89 |
| `thiserror` | 1.0.69 (`proto`), 2.0.17 (elsewhere) | | |

Three consequences worth naming:

- The toolchain is nightly, not 1.80. `Option::is_none_or` (1.82),
  `Vec::pop_if` (1.86) and `Result::flatten` (1.89) are all stable here and are
  declared. What is *not* declared is anything still behind a feature gate —
  `Box::into_inner`, `Hasher::write_str`, `ExactSizeIterator::is_empty`,
  `Cow::is_borrowed`/`is_owned` — because the corpus enables no such feature and
  cannot call them.
- `str::Pattern` on this toolchain is the GAT form, `trait Pattern: Sized { type
  Searcher<'a>: Searcher<'a>; }`, with no lifetime parameter of its own. The
  pre-1.80 `Pattern<'a>` shape is gone.
- `yrs::Update::merge` is 0.24's `fn merge(&mut self, other: Update)`, which
  returns nothing. Earlier yrs returned a merged `Update`.

## Target

The transpiler evaluates `cfg` as ankurah's wasm32 configuration
(`target_arch = "wasm32"` plus the `singlethread` feature), so this surface
describes a **32-bit pointer**. `usize::to_be_bytes` is `[u8; 4]`, not
`[u8; 8]`, and `usize::BITS` is 32. The collation code's byte layout depends on
which, so this is a wire-visible fact, not a detail.

## Auto traits

`Send`, `Sync` and `Unpin` are declared `auto` in `std/marker.rs`, as std
declares them (`pub unsafe auto trait Send {}`). Rust decides an auto trait
structurally — a type has it when every field has it — and no impl list can
express that.

**The engine rule: an auto-trait bound is always satisfied.** The corpus
compiles under rustc, so every `T: Send`, `T: Sync` and `T: Unpin` bound in it
already holds; the engine discharges them without searching for an impl. Do not
add per-type `impl Send for Foo {}` declarations — they would be an incomplete
list masquerading as a complete one.

`Sized` gets the same treatment from the engine and is *not* declared `auto`,
because it is a lang item rather than an auto trait and writing `auto` would
fabricate a fact about Rust. It is a plain marker trait with a comment saying
so.

## Format

Signature-only Rust, edition 2021. Method bodies are `{ todo!() }` so syn
accepts them; the body is never read. Trait methods are declared bodiless.
Generic parameters, bounds, `where` clauses, lifetimes and associated types are
written exactly as Rust has them:

```rust
impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    pub fn get<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<&V> where K: Borrow<Q> { todo!() }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}
```

Derives are written out as explicit impls, because the engine has no derive
expansion for std:

```rust
impl<T: Clone> Clone for Vec<T> { fn clone(&self) -> Vec<T> { todo!() } }
```

There are no `use` statements. A stub names types by their leaf name (`Vec`,
`Formatter`, `JsValue`); the loader maps a file's path to its module path and
resolves those names within the surface.

**A leaf name that more than one module in this surface declares is written
qualified at every use outside its own module.** This is not a style
preference: the loader has no `use` map to consult, so a bare ambiguous name
resolves to several candidates and the whole signature carrying it is dropped
from the table. `Ordering` in an `impl Ord` block is `std::cmp::Ordering`;
`Context` in a `poll` signature is `std::task::Context`; `Range` outside
`btree_map` is `std::ops::Range`; `Iter` and `IterMut` name their collection
(`std::collections::hash_map::Iter`) wherever they are used, since thirteen
modules declare an `Iter`. A `Debug` or `Display` signature returns
`std::fmt::Result`, which is what rustc's own source writes and which sidesteps
`Error` — the most overloaded name in the surface, declared by eighteen
modules.

Declaration sites stay bare: `pub struct Iter<'a, T>;`, the self type in
`impl Debug for DecodeError`, and an enum's variant names are where a name is
*introduced*, and qualifying them would say something different.

## Smart pointers: `clone` is a trait impl, the rest are associated functions

`Arc`, `Rc` and their `Weak` companions are declared the way std declares them,
and the split is not arbitrary:

```rust
impl<T: ?Sized> Clone for Arc<T> { fn clone(&self) -> Arc<T> { todo!() } }

impl<T: ?Sized> Arc<T> {
    pub fn downgrade(this: &Arc<T>) -> Weak<T> { todo!() }
    pub fn ptr_eq(this: &Arc<T>, other: &Arc<T>) -> bool { todo!() }
}
```

`arc.clone()` resolves to the `Clone` impl on `Arc<T>` itself. Rust's method
lookup tries the receiver's own type first — by value, then `&Arc<T>`, then
`&mut Arc<T>` — and only takes a deref step to `T` when none of those match, so
a correct resolver finds `Clone for Arc<T>` at the autoref step and never sees
the pointee's `clone`. If our resolver did reach the pointee, that is a bug in
the resolver, not a reason to declare `clone` as something Rust does not have.

`downgrade`, `strong_count`, `weak_count`, `ptr_eq`, `as_ptr`, `get_mut` and
`make_mut` really are associated functions taking `this: &Arc<T>`, and std made
them so for exactly the shadowing reason — an `Arc<Foo>` where `Foo` has its own
`as_ptr` must not silently call `Foo`'s. `Weak`'s equivalents are `&self`
methods, again as std has them, because `Weak<T>` does not deref to `T`.

`impl<T: ?Sized> Clone for &T` in `std/clone.rs` is the related fact that is easy
to omit: a shared reference is `Clone`, so `(&v).clone()` yields `&T`, not `T`.

## Module-path convention

The file path *is* the module path.

| file | module |
| --- | --- |
| `std/vec.rs` | `std::vec` |
| `std/collections/hash_map.rs` | `std::collections::hash_map` |
| `std/sync/mutex.rs` | `std::sync` — see below |
| `extern/tokio/sync.rs` | `tokio::sync` |
| `extern/js_sys.rs` | `js_sys` |

Two deliberate departures from a literal path-to-module reading, both because
std's own module layout does not match its file layout:

- Everything under `std/sync/` belongs to `std::sync` itself, not to a
  submodule — `std::sync::Mutex`, not `std::sync::mutex::Mutex`. The files are
  split for review, not for naming. `std/sync/atomic.rs` is the exception: that
  really is `std::sync::atomic`, and its `Ordering` is not `std::cmp::Ordering`.
- `std/tuple.rs` mirrors `core/src/tuple.rs`: the trait impls on tuples belong
  to no module and are attached to the tuple types themselves.
- `std/primitive.rs` and `std/num.rs` hold inherent impls on primitives
  (`impl u64 { .. }`, `impl str { .. }`, `impl<T> [T] { .. }`). Those blocks are
  not writable outside `core` and belong to no module; the loader attaches them
  to the primitive type itself, not to a module path.

An `extern/<crate>.rs` or `extern/<crate>/<module>.rs` file declares that
crate's surface, rooted at the crate name.

## What the loader must handle

The extractor at commit `f602831` reads an impl's self type only when it is a
`syn::Type::Path` (`extract.rs`, `extract_impl`). These stubs also write:

- `impl<T> [T] { .. }` — `Type::Slice`
- `impl str { .. }`, `impl u64 { .. }`, `impl bool { .. }` — primitive paths
- `impl dyn Any + Send + Sync { .. }` — `Type::TraitObject`
- `impl<'a, T: ?Sized> Deref for &T` and `for &mut T` — `Type::Reference`
- `impl<T, const N: usize> IntoIterator for [T; N]` — `Type::Array`

and nested `pub mod` blocks (`extern/tokio/sync.rs`'s `oneshot` and `mpsc`,
`extern/serde.rs`'s `ser` and `de`, `extern/rusqlite.rs`'s `types`). Extending
the extractor for these is the loader's job; the stubs are written to Rust, not
to today's extractor.

Four further constructs the audit pass introduced, all of which syn parses:

- `pub unsafe auto trait Send {}` — `ItemTrait` with `unsafety` and `auto_token`
- generic associated types — `type Searcher<'a>: Searcher<'a>;` in `Pattern`
- higher-ranked bounds — `for<'a> <P as Pattern>::Searcher<'a>: ReverseSearcher<'a>`
- `type` aliases as items — `pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;`
  in `extern/futures.rs`, which must resolve *through* the alias or every boxed
  stream loses its `Stream` relationship
- `extern "rust-call" fn` in the `Fn` family, and `impl Trait` in return
  position (`AppendOnlyVec::iter`) and argument position (`bb8::Builder::min_idle`)

## Simplifications

Each of these drops something real. They are listed so a later reader can tell a
deliberate simplification from an oversight.

1. **No allocator parameter.** Real std is `Vec<T, A: Allocator>`,
   `Box<T, A>`, `Arc<T, A>`. Written here as `Vec<T>`, `Box<T>`, `Arc<T>`.
   ankurah never names an allocator, and rust-analyzer's rendering
   (`Vec<EventId, Global>`) is exactly the noise the oracle's schema note warns
   about normalising away.
2. **The hasher parameter is kept.** `HashMap<K, V, S = RandomState>` and
   `HashSet<T, S = RandomState>` keep `S`, because `core/src/util/iterable.rs`
   writes `impl<T, S: BuildHasher> Iterable<T> for HashSet<T, S>` — a corpus
   impl that would not unify against a one-parameter `HashSet`.
3. **`Borrow` generality is kept.** `get`, `get_mut`, `contains_key`, `remove`
   and the set equivalents keep `Q: ?Sized` and `K: Borrow<Q>`. The simpler
   `fn get(&self, k: &K)` was available and was not taken: it makes
   `map.get("key")` on a `HashMap<String, _>` unresolvable, and the resulting
   diagnostic would point at ankurah's code rather than at this file.
   `std/borrow.rs` carries the impls that discharge the bound — the reflexive
   blanket, `String: Borrow<str>`, `Vec<T>: Borrow<[T]>`.
4. **Provided versus required trait methods is not recorded.** `Iterator`
   declares ~70 methods and real std provides all but `next`. Every method here
   is bodiless, so an `impl Iterator` in the corpus that supplies only `next`
   looks incomplete. Nothing reads that: the engine resolves calls and never
   checks an impl for completeness. Same for `StreamExt`, `FutureExt` and
   `Itertools`, whose blanket impls are written with empty bodies.
5. **`Try` is not declared.** `?` is handled directly by the engine (spec 4.6),
   `Try`/`FromResidual` are unstable, and a declaration nothing reads is worse
   than none. `Iterator::try_fold` is narrowed to the `Result` case for the same
   reason, and says so at its declaration.
6. **`StreamExt::next` is `async fn`.** Real `futures` returns a named
   `Next<'_, Self>`. `s.next().await` types correctly either way; `let f =
   s.next();` does not. Every corpus site awaits immediately. This is the only
   surviving shortcut in the extension traits; `Notify::notified` had the same
   shape and was corrected, because there the named future is load-bearing.
7. **`Pattern`, `Searcher` and `Step` are declared even though std keeps them
   unstable.** `Pattern` is written in full — the `Searcher<'a>` GAT, the
   `for<'a> P::Searcher<'a>: ReverseSearcher<'a>` bounds on every reverse
   operation, and the six real impls — because without it `split`, `find`,
   `rsplit` and `trim_matches` have neither an argument type nor a way to reject
   a forward-only pattern used in reverse. Without `Step`, `Range<A>: Iterator`
   would be unbounded and `Range<String>` would resolve as an iterator.
8. **`Unsize` is declared, and array-to-slice uses it rather than `Deref`.**
   The oracle keeps `Pointer(Unsize)` and `Deref(Some(OverloadedDeref(..)))`
   apart, so declaring `impl Deref for [T; N]` would teach the engine a relation
   rustc does not have. `Vec<T> -> [T]` and `String -> str`, by contrast, really
   are `Deref`, and are declared as such.
9. **Operator impls on primitives are partial.** `std/ops.rs` writes out the
   (trait, width) pairs the corpus uses plus their obvious neighbours, not all
   8 traits x 12 primitives. A missing pair is a diagnostic, not a wrong answer.
10. **Tuple impls stop at arity 6.** std generates `Clone`, `Copy`,
    `PartialEq`/`Eq`, `PartialOrd`/`Ord`, `Hash`, `Default` and `Debug` for
    arities 1 through 12; `std/tuple.rs` writes 0 through 6. A 7-tuple is a
    diagnostic, not a wrong answer.
11. **`sha2` returns a plain `Output`.** Real `sha2` returns
    `GenericArray<u8, U32>` through `typenum`. Every corpus use immediately
    slices or copies the bytes.
12. **`SliceIndex` is declared `unsafe` but its unsafety is not tracked.** The
    keyword is written because it is part of the declaration; the engine has no
    notion of trait unsafety today.

## Omissions

Declared, deliberate, and each with its reason.

| omitted | why |
| --- | --- |
| `bytes` (`BufMut`, `BytesMut`) | reached only from `proto/src/postgres.rs`, which `transpile.toml [excluded_files]` excludes |
| `fallible-iterator` | same file, same reason |
| `postgres-types`, `postgres-protocol` | postgres is out of scope (spec 1a) |
| `std::fs` | the port has no file system |
| `impl Pattern for [char; N]` | the corpus never writes a char-array pattern, and the impl was the only const generic flowing into an associated type (see "Engine gaps") |
| `tracing`, `log` | macros only (`warn!`, `debug!`), and the engine does not expand macros. The macro handler supplies the call's type |
| `reactive_graph`, `futures-signals` | `signals/src/reactive_graph.rs` is `#[cfg(feature = "reactive-graph")]` and the cfg evaluator drops it under ankurah's wasm32 + `singlethread` configuration |
| `uniffi`, `send_wrapper`'s FFI half | `uniffi::setup_scaffolding!` is a macro in a hardcoded file |
| `chrono` | declared as a dependency, not used in any in-scope module |
| `console_error_panic_hook`, `wasm-logger`, `env_logger` | setup-only, called once, no types flow from them |

## Crates declared beyond the brief

The deliverable named tokio, anyhow, thiserror, serde, bincode, ulid, futures,
async_trait, web_sys, wasm_bindgen, js_sys, rusqlite and pest. Twelve more are
declared, each because an in-scope, non-excluded module makes a method call or a
deref through one of its types and the engine would otherwise stop:

`send_wrapper` (a `Deref` step at 13 sites), `yrs` (the whole property backend —
see "Decided"), `serde_json` (`Value`'s accessors), `base64` (the
`Engine` trait the oracle resolved six calls to), `itertools`
(`exactly_one`, `sorted`), `indexmap` (the planner depends on insertion order),
`append_only_vec` (`push` returns the index, and that index is the transaction's
entity handle), `sha2` (the event-id digest is wire-visible),
`wasm_bindgen_futures` (`spawn_local` and `JsFuture`), `serde_wasm_bindgen`
(the `Serializer` builder chain), `gloo_timers` (the reconnect backoff), and
`rand` (`SliceRandom::choose`), and `bb8` (see the ruling below).

Three std modules are declared for a different reason — not because transpiled
code reaches them, but because another stub's signature *names* them, and an
undeclared name drops the signature that mentions it: `std::path` (`Path` and
`PathBuf`, from `rusqlite::Connection::open` and `Error::InvalidPath`),
`std::ffi` (`NulError`, from `rusqlite::Error::NulError`), and `std::array`
(`IntoIter`, the iterator `[T; N]: IntoIterator` yields, which
`bytes.to_be_bytes()` loops over).

## Adding a method

When a future ankurah version calls something not declared here:

1. Run `python3 transpile/std_surface/coverage.py`. A name that appears under
   "unaccounted" is missing from the surface; a name that does not is either
   declared or defined by ankurah itself.
2. Find the item **at the locked version** — `docs.rs/<crate>/<version>` for an
   extern crate, and for std the pinned toolchain's own source, which is on
   disk at `$(rustc +nightly --print sysroot)/lib/rustlib/src/rust/library`.
   Copy the real signature: generics, bounds, `where` clause, lifetimes and all.
   Do not simplify a bound because the engine currently trips on it; the engine
   is the thing to fix. Do not read `docs.rs/<crate>/latest` — several
   corrections in this directory exist because a signature moved between the
   locked version and the current one.
3. Put it in the file whose module path matches std's. A method on a type goes
   in that type's inherent `impl` block; a trait method goes in the trait.
4. If the item is a new type, declare its `Deref`, `Clone`, `Debug`, `Display`,
   `Default`, `PartialEq`/`Eq`/`Ord`, `Hash`, `IntoIterator` and
   `FromIterator` impls too, as far as they are real. The absence of `impl
   Clone for X` is a statement that `X` is not `Clone`.
5. Check it parses:
   `cargo` is not wired up for this directory; a throwaway crate that runs
   `syn::parse_file` over every `.rs` under `std_surface/` is enough, and is how
   these files were checked in.
6. Give it a TypeScript translation in `name_map` / `native_types/` in the same
   change. A declared fact with no translation moves the failure from resolution
   to emission; it does not remove it.

If a simplification is unavoidable, write the deviation as a comment at the
declaration and add a row to "Simplifications" above. A silent deviation in this
directory is a wrong answer everywhere the engine uses it.

## Coverage

`coverage.py` reports two tiers, and only the first is exact. Tier A checks the
rust-analyzer oracle's resolved callees — (type, method) pairs an outside
authority already answered — against the stubs. Tier B sweeps every method name
called in the in-scope crates and reports the names that neither ankurah nor a
stub declares; it cannot resolve receivers, so it over-reports (a name ankurah
defines masks a std method of the same name) and cannot under-report. The script
exits non-zero on a Tier A miss or on any unaccounted name in `proto`, `ankql`,
`signals` or `core`.

## Corrections from the external audit

A Codex audit on 2026-09-02 checked every declaration against upstream. Its
findings were applied except where the locked versions contradict it; the
report accompanying that pass lists the rejected rows. The structural ones
worth carrying forward as rules:

- **A declared extension trait must carry its combinators' impls.** A
  `StreamExt::map` that returns a bare nominal `Map<St, F>` with no `impl
  Stream` is worse than no declaration: the call resolves and the *next* one
  fails, at a type the reader did not write. Every combinator in
  `extern/futures.rs` now carries its `Stream` or `Future` impl.
- **A type alias must stay an alias.** `BoxStream` and `BoxFuture` were
  structs; as aliases to `Pin<Box<dyn Stream<..>>>` they keep the trait
  relationship the alias exists to carry.
- **A wrapper needs both halves of its cast.** `JsCast` has `Into<JsValue>` as a
  supertrait, so a js/web wrapper without its `From<Wrapper> for JsValue` cannot
  satisfy `JsCast` even with the `JsCast` impl written out. Both are generated
  for all 31 wrappers.
- **Do not invent a trait to make a name resolve.** `thiserror::Error` was
  declared as a trait so `use thiserror::Error` would resolve; it is a derive
  macro, and the fabricated trait let `T: thiserror::Error` be proved against
  nothing. Same for `UnwrapThrowExt`, which was a struct plus an unrelated
  trait.
- **Constructor bounds are not operation bounds.** `BinaryHeap::new` needs no
  `Ord`; `push` does. `IndexSet::len` needs no `Hash`. Copying an operation's
  bound onto a constructor rejects valid generic code.

## Engine gaps the stubs expose

These are declarations the loader currently cannot represent. They are correct
Rust and correct for the locked versions; **do not edit them away to silence a
diagnostic.** Each is a note for whoever extends the engine.

| what | where | count |
| --- | --- | --- |
| `self: Pin<&mut Self>` receivers | every `Future::poll` and `Stream::poll_next` | 50 |
| raw-pointer return types (`*const T`, `*mut T`) | `Arc::as_ptr`, `Vec::as_ptr`, `UnsafeCell::get`, `Box::into_raw` and neighbours | 16 |

An arbitrary-self-type receiver is how `Future` and `Stream` are declared; there
is no version of those traits without it. Raw pointers reach ankurah's code in
exactly one place — `Arc::as_ptr` in `signals`, which the oracle records — and
the value is only ever compared, never dereferenced, so the engine needs the
type to exist rather than to mean anything.

One declaration *was* removed for a loader limitation, and it is listed as an
omission rather than hidden here: `impl<const N: usize> Pattern for [char; N]`
and its `CharArraySearcher`, where a const generic flowed from an impl's
parameter list into an associated type. The corpus never writes a `[char; N]`
pattern, so removing it costs nothing; had the corpus used it, the right answer
would have been to fix the loader.

## Decided

**`bb8` is declared; `engine.rs` transpiles unchanged.** (Ruling, 2026-09-02.)
`storage/sqlite/src/engine.rs` holds a `bb8::Pool<SqliteConnectionManager>`,
builds it with `Pool::builder().max_size(..).build(manager).await` at lines 39
and 47, clones it into each `SqliteBucket`, and checks a connection out with
`pool.get().await` at eight sites. Only `connection.rs` becomes a provided file,
so the pool is in transpiled code. The question was whether the driver interface
should absorb pooling; the answer is that it should not. The provided driver
module for each environment supplies a pool-shaped shim that hands out its
single synchronous connection, `engine.rs` is left alone, and `extern/bb8.rs`
declares the pool types so the shim has a shape to match.

`SqliteConnectionManager` is deliberately *not* in `extern/bb8.rs`. Despite the
r2d2-style name it is ankurah's own struct in `storage/sqlite/src/connection.rs`,
along with the `impl bb8::ManageConnection` that fixes `type Connection =
PooledConnection` and `type Error = SqliteError`; the engine reads those off
ankurah's source. That `PooledConnection` is a different type from
`bb8::PooledConnection<'_, M>` — same leaf name, different modules, which is the
case the non-flat registry ruling exists for, and `pool.get().await?` reaches
ankurah's `with_connection` by deref through bb8's.
