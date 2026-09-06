# Ownership: Rust to TypeScript

**Goal**: translated TS reads as close to the Rust source as possible, and behaves
the same. The ownership types live in `@ankurah/base` (`packages/base/src`).

Rust's compiler enforces ownership before the program runs. TypeScript cannot, so
the polyfills enforce the same rules while it runs: they drop what Rust drops, in
the order Rust drops it, and they stop the program when they see a state the Rust
compiler would have refused to build.

---

## The model

Every ported Rust type extends `AkObject`. An `AkObject` is alive, dropped, or
moved, and it is registered with the leak detector from construction until one of
the latter two happens.

**Dropping** releases a value and everything it owns. `AkObject.drop()` is the
whole template, and nothing overrides it:

1. refuse if this value was already dropped or moved;
2. mark dropped and leave the leak registry;
3. call `onDrop()` — the type's own cleanup, with every field still alive;
4. in a `finally`, drop each of `ownedFields()`.

Step 3 before step 4 is not an implementation detail: Rust runs `Drop::drop`
before dropping fields, so a cleanup body that reads a field must still find it
usable.

**Moving** hands a value's contents to somebody else. A moved value is not
dropped and is not a leak — there is nothing left in it to release — and every
later use of it is fatal. `Result`'s `self`-taking methods move, and a released
`Arc` handle reports use-after-move for the same reason: the handle was this
scope's owner, and in Rust the moved-out binding is no longer nameable. A
consuming `match` for enums comes with the emitter.

**The cascade** walks what a value owns. `dropOwned(v)` drops anything with a
`drop()` method, walks arrays, Maps (keys *and* values, because `HashMap<K, V>`
owns both), Sets and plain objects to any depth, and lets primitives go. An `Arc`
ends the walk: it decrements, and only the last strong drop cascades into the
contents. Reaching the same object twice in one cascade is aliased ownership and
the second drop reports it.

---

## What the transpiler emits against

**`impl Drop for T` becomes `protected override onDrop()`.** Never an override of
`drop()`. Emitting `drop()` puts the cleanup after the cascade, which is the
wrong order and hands the body dead fields.

**Fields are dropped in the order the constructor assigned them.** The cascade
walks own properties, and `Object.getOwnPropertyNames` returns identifier keys in
insertion order, so the emitter must assign fields in declaration order to match
Rust's field drop order. (Integer-like keys — `"0"`, `"1"` — sort ahead of all
identifiers, so the guarantee holds for the field names the emitter produces, not
for an object keyed by numbers.)

**A type that keeps state in `#private` fields overrides `ownedFields()`.** The
cascade cannot see private state by walking properties. `Enum` does this for its
variant payload; the containers do it for their contents.

**A value owned by a block is dropped in a `finally`.**

```typescript
const entity = await node.get(id);
try {
  // ... use entity ...
} finally {
  entity.drop();
}
```

**A guard temporary is released at the end of its statement and again in the
enclosing `finally`.** That is why a guard's second drop is deliberately a no-op,
and it is the only place in the runtime where a second drop is not fatal. Nothing
but `Guard` in `std/guard.ts` overrides `drop()`, and the reason is written next
to it.

**`Copy` types have no drop glue.** A `Copy` type cannot implement `Drop` in
Rust, so the emitter gives it no `drop()` method and no registry entry — whatever
class shape it picks for it is the emitter's decision. The runtime tests for the
absence of drop glue, not for "is it a primitive". That is what makes
`guard.value = 5`, or re-storing a `Copy` struct, legal, while re-storing an
owned object the container already holds is fatal.

**`thread_local!` becomes `ThreadLocal`.** A static that lives for the whole
program: never a struct field, never dropped, not leak-tracked.

**A Rust function returning `Result` returns a `Result` value.** It does not
throw. `Result`'s `self`-taking methods (`unwrap`, `expect`, `map`, `andThen`,
`ok`, …) consume the receiver, exactly as Rust does; `isOk`/`isErr` borrow.
`Option<T>` is `T | null`.

**A `match` on a scrutinee taken by value becomes `intoMatch(arms)`.**
`match(arms)` borrows — it reads the payload and leaves the enum whole, which is
what a match on a reference needs, and it is what the emitter uses in borrow
position. `intoMatch(arms)` is the by-value form: it hands the payload to the
arm as the arm's own, marks the enum moved, and takes the payload out of
`ownedFields()` so the cascade never reaches it. The enum is moved rather than
dropped, so the emitter emits no drop after one and every later use of the
binding is fatal. `Result` inherits `intoMatch`; its self-taking methods are the
same move written out for the two variants it has.

**A consuming arm has ONE unwind owner, and it is the arm.** The arm owns the
whole payload from the moment it is called, on every path out of it — a normal
return, a `return` from inside, and a throw alike. So an arm takes a name out of
the payload for every part it uses and releases the rest with
`dropUnbound(payload, bound)`, where `bound` names the keys it did take; both go
in the arm's own `finally`. `intoMatch` releases nothing of its own when an arm
throws. It used to, and that gave a throwing arm two unwind owners: the arm's
`finally` released a binding, `intoMatch` released the payload it came out of,
and the exception the arm actually raised was replaced by `BUG: … was dropped
twice`. An arm that leaves part of the payload unowned now leaks it, which the
registry reports against the arm that caused it rather than against whichever
innocent value was released second.

**A partial move — `let x = s.field;` — becomes `s.takeField('field')`.** The
field's value becomes the caller's, the cascade stops releasing it, and reading
it again is fatal, because the read is what Rust would have rejected. The struct
itself is *not* moved: its remaining fields are still its own and its drop still
runs, which is what Rust does with the rest of a partially moved struct.
`takeField` is the one member of `AkObject` that emitted code calls from outside
the type, and it is public for that reason — a partial move happens in the scope
that owns the struct, not inside it. Re-initialising a moved-out field, which
Rust does allow, has no form here.

**A `move` closure that captures values with drop glue becomes an
`OwnedClosure`.** A Rust `move` closure owns what it captured and releases it
when the closure is dropped — a listener holding an `Arc` keeps that `Arc` alive
for exactly as long as the listener lives. A JS closure captures the same values
and the cascade cannot see any of them: it walks own properties, and a capture
is not a property. So the emitter writes
`new OwnedClosure(captures, fn, label?)`, listing the captured values as an
array or a record beside the body that closes over them, and from there they are
ordinary owned fields: dropping the closure cascades into them, and a closure
stored in a `Vec` is released by whatever drops the `Vec`. It is invoked as
`closure.call(...args)` and not directly, because a callable object would let
emitted code write `f(x)` and reach the body without passing the liveness check
that is the whole point — calling a dropped closure is fatal. A closure that
captures nothing droppable stays a plain function; there is nothing for the
cascade to find, and wrapping it would only add a drop the emitter then has to
place.

`closure.$arity` is how many arguments the body declares. The open-bound
dispatcher needs it: two impls can differ only in the arity of the callable they
are written for — `Arc<dyn Fn(T)>` beside `Arc<dyn Fn()>` — and Rust picks
between them by type, where the port has to ask the value. The function inside
an `OwnedClosure` is `#private`, so nothing outside could ask without this. It
borrows, and it checks liveness first like every other read here: a dispatcher
only ever asks a value it is about to call, so a dropped closure reaching one
says the emitted scope released it too early. It reports what `Function.length`
reports — the parameters before the first default or rest parameter — and an
emitted closure has neither, so for emitted code the two counts are one number.

A `FnOnce` call is `closure.callOnce(...)` rather than `call`. It consumes the
closure: the captures become the body's, so the closure stops owning them and is
left moved, and a second call or a drop after one is fatal. The body closes over
the captures lexically either way — what `callOnce` changes is who releases
them, and from there it is the body's job, exactly as it is in a Rust `FnOnce`
whose captures become locals in it.

**A Rust error boxed into `anyhow::Error` becomes an `AnyhowError`.** It is what
the 29 `?` sites in core convert into, and it is a tracked value like anything
else, because it owns the error values in its chain. `AnyhowError.from(e)` is
Rust's blanket `impl<E: std::error::Error> From<E>` — the conversion `?`
performs — and it is the identity on an error that already is one, the way `?`
on an `anyhow::Error` does not box it twice. `AnyhowError.msg(text)` is
`Error::msg`. `err.context(msg)` takes `self`, so it consumes the error it was
called on and moves its chain into the one it returns; the emitter emits no drop
after it. `toString()` is anyhow's `Display` — the outermost message —
and `toStringAlternate()` is `{:#}`, the whole chain joined with `": "`.
`downcast_ref(Ctor)` and `root_cause()` borrow, exactly as their Rust
counterparts do: what they hand back still belongs to the chain, and the caller
must not drop it. `anyhow::Result<T>` needs nothing of its own — it is
`Result<T, AnyhowError>`.

**A JSON decode fails with a `JsonError`, which is `serde_json::Error`.** It is
what an emitted `static fromJson(value)` returns an `Err` of, and it exists
because a `Deserialize` impl fails with the *format's* error and every crate the
port emits has to be able to name it — which is what rules out each crate's own
decode error. It is a tracked value like `AnyhowError`, because
`serde_json::Error` boxes its contents and so has drop glue: a caller that takes
one out of a `Result` owns it and drops it. Nothing inside it has drop glue of
its own. `JsonError.custom(message)` is `serde::de::Error::custom`, the
conversion a hand-written decoder with a richer error of its own performs — it
keeps the rendered text and nothing else, which is all Rust keeps.
`JsonError.syntax(message, line?, column?)` is a failure with a place in the
text. `toString()` is serde_json's `Display`: the message alone when there is no
position, and `"<message> at line L column C"` when there is, which is how
serde_json decides it — on the line alone. `JsonError.fromException(thrown)`
wraps what `JSON.parse` threw and DELIBERATELY loses the position: serde_json
knows the line and column because it drives the parse, and `JSON.parse` reports
what the host chose to report, in text that differs between V8,
JavaScriptCore and Hermes. `serde_json::Result<T>` needs nothing of its own — it
is `Result<T, JsonError>`.

**`HashMap<K, V>` and `HashSet<T>` are keyed by value, not by identity.** A
JavaScript `Map` keys objects by identity and Rust keys them by `Hash` and `Eq`,
so `HashMap<EntityId, Peer>` cannot be a `Map`: two `EntityId`s over the same
bytes are one key in Rust and two in a `Map`. A key's `hash()` picks the bucket
and its `equals()` decides which entry in that bucket is the one, so a key type
is free to hash coarsely — a collision costs a comparison and nothing more. What
may be a key is the family Rust's `Hash + Eq` covers: a primitive or `null`; a
sequence (an array or a typed array, which is how the port spells a tuple and a
`Vec<u8>`), hashed and compared element by element as Rust's
`impl Hash for (A, B)` and `for [T]` do; or an object declaring `hash(): string`
and `equals(other): boolean`, which is what `#[derive(Hash, PartialEq, Eq)]`
emits. Anything else is refused by name — a plain throw, because the insert did
not happen and nothing is corrupted — rather than falling back to identity,
which is the bug these containers exist to prevent.

A sequence's bucket label carries each part's LENGTH, so no separator can be
forged out of the parts themselves: joining with a comma made `['a', 'b']` and
`['a,s:b']` one label, and a `Vec<String>` field of a derived key then collided
with a single string that happened to spell the join. The derived `hash()` the
emitter writes length-prefixes its fields for the same reason; this is that rule
for the sequence a tuple and a `Vec` are written as.

A field whose type is one of the declaring type's own PARAMETERS is one the
emitter cannot write a member call for: `T` is a number in `Keyed<u32>` and a
class in `Keyed<Tag>`, and `hash()`, `equals()` and `clone()` on a number are
all TypeErrors. `derivedHash`, `derivedEquals` and `derivedClone` are the three
that decide by the value's own surface at run time, and each refuses a value
that declares neither the member nor a primitive shape — `#[derive(Hash)]` on a
generic carries `T: Hash`, so such a value is one the port put there and Rust
would not have.

The map owns its keys and its values, and dropping it releases both. Rust's
`insert(k, v)` keeps the key it already has and drops the one it was handed, so
`insert` returns the displaced *value* and releases the surplus *key*; `set(k, v)`
is `insert` with the displaced value released too, which is what
`map.insert(k, v);` as a statement means in Rust. `remove(&k)` hands the value
to the caller and drops the stored key; `delete(k)` releases both and answers
whether there was an entry, which is what `map.remove(&k);` as a statement
means. `clear()` releases every key and value. `get`, `entries`, `keys`,
`values` and iteration all borrow: what they hand out is still the container's.
Each iterator is a snapshot, so a loop that deletes as it goes — which is what
`retain` is emitted as — is safe. `get` returns `V | null`, because
`HashMap::get` returns an `Option<&V>` and this port spells `Option<T>` as
`T | null`; **an emitted `=== undefined` test against a map lookup is therefore
wrong**, and so is `HashMap<K, Option<X>>` asking `remove` to tell an absent key
from a stored `None` — ask `has` first. `HashSet<T>` is the same table with the
value half unused: `insert(v)` answers whether the value was new and drops a
duplicate, and `add`/`has`/`delete` are the names the emitter writes.

---

### Reserved member names

The runtime lives on the same objects the port's data does, so its members and a
Rust struct's fields share one namespace. Two rules keep them apart.

**Contract members. An emitted class must never declare a field or method with
one of these names.** They are the mechanism, not a convenience, and there is no
way for the runtime to give them up. Shadowing one fails quietly or not at all: a
class field declaration creates an own property that hides the prototype member,
so the cascade, a liveness check or a match would run against the struct's data
instead of the runtime's. The emitter renames a Rust field that collides.

| Where | Names |
|---|---|
| Every ported type (`Struct`, `Enum`, `Drop`, anything extending `AkObject`) | `drop`, `onDrop`, `ownedFields`, `takeField`, `isDropped`, `isMoved`, and the protected helpers `assertNotDropped` and `markMoved` |
| Enums, including `Result` | `match`, `intoMatch`, `type`, `value` |
| `OwnedClosure` | `call`, `callOnce` |

`[Symbol.dispose]` is not on the list: it is a computed symbol key, and no Rust
identifier can produce one.

**Convenience members are namespaced with `$`, so they can never collide.** A
Rust identifier cannot start with `$`, so a `$` name is unreachable from a field
name by construction. Anything the runtime offers that is a convenience rather
than the mechanism takes one.

| Member | On | What it is |
|---|---|---|
| `$label` | `AkObject` (protected) | What diagnostics call this value. |
| `$arity` | `OwnedClosure` | How many arguments the body declares. Borrows. |

`$label` was `label` until a ported struct with a `label` field collided with it
— which did not merely shadow it but failed to compile, since a public field
cannot stand over a protected base member. The runtime moved, and any
convenience member added later takes a `$` name for the same reason. Note that
the `label` *parameters* the provided containers accept — `new Mutex(v, 'X')`,
`oneshot.channel('peerRequest')`, `spawn(fut, 'retryLoop')` — are parameters and
options keys, not members, and are not part of this.

## Fatal, panic, and the latch

**Fatal** is for anything Rust rejects at compile time. It means the emitter is
wrong, and every one goes through `fatal()` in `drop_registry.ts`:

| Condition | Reported by |
|---|---|
| Dropping a value twice | `fatalDoubleDrop` |
| Using a dropped value | `fatalUseAfterDrop` |
| Using a moved value | `fatalUseAfterMove` |
| Dropping a container while a guard on it is outstanding | `fatalOutstandingGuard` |
| Assigning a container the object it already holds | `fatalSelfAssignment` |
| A match with a missing arm | `fatalNonExhaustiveMatch` |
| Collecting a value that was never dropped | the leak registry |

**A plain throw** is for anything the emitted code got right but the program got
wrong: `RefCell` borrow conflicts and `unwrap()` on an `Err`, both of which panic
in Rust too. Re-locking a `std::sync::Mutex` on one thread is the exception that
proves the rule — Rust deadlocks rather than panicking, so this throws instead,
reporting the same bug where a hang would be undiagnosable.

**The poison latch.** `fatal()` sets a latch before it throws, and every liveness
check reads the latch first. A host can swallow a throw — an `uncaughtException`
handler, a browser page that keeps painting — and the program would then run on
over corrupted ownership. After the first fatal, the next check refuses instead.

**Every fatal is thrown as an `OwnershipFatal`** (exported from the package), so
emitted code can tell an ownership bug from a Rust error value. A `catch` block
that handles a Rust error type must test for `OwnershipFatal` and rethrow it
unconditionally — the runtime has already found something Rust would not have
compiled, and nothing after it can be trusted.

**And it must rethrow an `UnsupportedShape` the same way.** That is what an R12
hole throws, and it says the ENGINE has no lowering for a Rust shape — not that
the data was bad. A `catch` that answers `Err` for one turns a loud refusal into
a silent wrong answer, which is the trade R12 exists to refuse. So the first
line inside every generated `catch` is `if (e instanceof OwnershipFatal || e
instanceof UnsupportedShape) throw e;`, and
`transpile/tests/parse_gate.rs::no_emitted_catch_swallows_an_ownership_fatal`
reads the emitted text of all ten crates to hold it: the rule is a property of
the OUTPUT, so a `catch` written by some future emitter is caught the day it
appears.

**`setOnFatal(handler)`** replaces what a fatal does, for a host that has to stop
differently — killing a worker, failing one request. The default throws an
`OwnershipFatal`.

**Tests**: the root `bunfig.toml` preloads `packages/base/src/testing.ts`, and
loading it installs the hooks — a suite needs no setup of its own. They reset the
latch before each test and fail any test that raised a fatal and swallowed it.
Without the reset, one test's fatal fails every test after it and the failure
lands nowhere near the bug. A test that provokes a fatal on purpose asserts it
and then calls `clearFatalLatch()` to acknowledge it.
`installOwnershipTestHooks()` is idempotent, so a file may call it explicitly to
make the dependency visible.

---

## Leak detection

Every `AkObject`, every `Arc` and `Weak` handle, and every container (`Mutex`,
`RwLock`, `RefCell`, `AsyncMutex`) is registered at construction and unregistered
when dropped or moved. A registered value that is garbage collected was never
dropped, and that is fatal, reported from a microtask because a
FinalizationRegistry callback has no caller to throw to.

`FinalizationRegistry` is feature-detected at module load. Where it is missing the
runtime installs a no-op registry and warns once, loudly: every other ownership
check still works, but a value that is simply forgotten goes unreported.

**Hermes**: `FinalizationRegistry` support landed on the `static_h` branch and
shipped in `260318099.0.0` — see facebook/hermes issue 1440, comment of
2026-04-30 by lavenzg ("FinalizationRegistry support has been landed in the
static_h branch ... included in the staging branch (260318099.0.0-staging)"), and
the Hermes release note of 2026-06-05 for `260318099.0.0`. Expo Go builds older
than that run the port with leak detection off, which is why a missing registry
has to be a warning the port survives rather than a crash.

Allocation stacks cost about a microsecond per construction, so they are behind
`setCaptureStacks(enabled)`, on by default except when `NODE_ENV=production`.
Labels are always recorded, so a report always names the type even without a
stack.

---

## Type mapping

| Rust | TS | Notes |
|------|-----|-------|
| `struct Foo` | `class Foo extends Struct` | Cascade drops owned fields. |
| `enum Foo` | `class Foo extends Enum<V>` | `match()`, `is()`, typed variants; cascade drops the payload. |
| `impl Drop for T` | `protected override onDrop()` | Runs before the fields are dropped. |
| `Arc<T>` / `Rc<T>` | `Arc<T>` | Refcounted. `arc.clone()` — a bare assignment does **not** increment. Inner drops when the last handle drops. |
| `Weak<T>` | `Weak<T>` | `upgrade()` returns `Arc<T> \| null`. Tracked and dropped like anything else. |
| `&T` / `&mut T` in fields | `Borrow<T>` / `BorrowMut<T>` | Non-owning; the cascade steps over them. |
| `Box<T>` | `T` | Unique ownership; the cascade handles it. |
| `Mutex<T>` | `Mutex<T>` | `lock()` returns a `MutexGuard<T>`. |
| `RwLock<T>` | `RwLock<T>` | Its own type: `read()` and `write()` return distinct guards. |
| `RefCell<T>` | `RefCell<T>` | `borrow()` / `borrowMut()`, with Rust's runtime borrow rules. |
| `tokio::sync::Mutex<T>` | `AsyncMutex<T>` | Serializes across `await`; `acquire()` returns a guard. |
| `Option<T>` | `T \| null` | |
| `Result<T, E>` | `Result<T, E>` | A returned value, not a throw. |
| `thread_local!` | `ThreadLocal<T>` | Static; not tracked. |
| `AtomicBool` / `AtomicU32` | `boolean` / `number` | Single-threaded JS. |
| Lifetimes (`'a`) | runtime liveness checks | No compile-time lifetimes. |
| `fn method(self)` (move) | consuming method + moved state | No move semantics in JS. |
| `match value { … }` (by value) | `enum.intoMatch(arms)` | Arm owns the whole payload on every path out, unwinds included; the enum is left moved. `match()` borrows. |
| a payload field an arm took no name for | `dropUnbound(payload, bound)` | Releases every field of the arm's payload except the keys in `bound`. |
| `a & b`, `a \| b` on `bool` | `boolAnd(a, b)`, `boolOr(a, b)` | Rust's `&`/`\|` evaluate both operands; `&&`/`\|\|` do not. `^` is `!==`, which is already eager. |
| `let x = s.field` (partial move) | `s.takeField('field')` | The rest of the struct stays usable and droppable. |
| `move \|…\| { … }` with droppable captures | `OwnedClosure` | `new OwnedClosure(captures, fn)`, invoked with `.call(...)`; `.callOnce(...)` for `FnOnce`. |
| `anyhow::Error` | `AnyhowError`, or `anyhow.Error` | Owns its chain. `from`, `msg`, `context`, `downcast_ref`, `root_cause`. |
| `anyhow::Result<T>` | `Result<T, AnyhowError>` | No type of its own. |
| `serde_json::Error` | `JsonError`, or `serde_json.Error` | Tracked, and owns nothing further. `custom`, `syntax`, `fromException`. |
| `serde_json::Result<T>` | `Result<T, JsonError>` | No type of its own. |
| `HashMap<K, V>` | `HashMap<K, V>` | Keyed by `hash()` and `equals()`, never by identity. Owns keys and values. |
| `HashSet<T>` | `HashSet<T>` | The same table with no value half. |
| `BTreeMap<K, V>` / `BTreeSet<T>` | — | Not provided. An ordered map iterates in `Ord` order and `HashMap` iterates in bucket order, so standing one in for the other changes what a traversal produces. |
| `tracing::info!("…")` and its four siblings | `tracing.info(msg)`, … | One already-rendered string per call. |

A container owns its contents: dropping a `Mutex<T>` drops the `T`, as in Rust.
A guard does not own what it points at — it reads and writes through the
container's own storage, so `*guard = v` replaces what the container holds and
drops what was there.

---

## The tracing layer

`tracing::trace!` and its four siblings become `tracing.trace(message)` and
theirs — five functions, each taking one string, exported as a namespace from
`@ankurah/base`. Nothing here is tracked and nothing here owns anything: a
rendered message is a string.

In Rust the macro builds an *event*, with a level, a target and a set of typed
fields, and hands it to whatever subscriber is installed. None of that survives
the crossing. The transpiler renders the format string at the call site and
emits one already-rendered string, and a `tracing::warn!` that carries
structured fields instead of a format string is refused by the transpiler rather
than losing its fields quietly at this end.

`tracing.setSink(sink)` replaces where the five write, so a host can forward its
log and a test can capture it; `tracing.consoleSink` is the default and puts it
back. Two things about that default differ from Rust deliberately. `trace` goes
to `console.debug` and not `console.trace`, because `console.trace` prints a
stack trace and `tracing::trace!` does not. And an event reaches the console
with no subscriber installed, where in Rust it would be dropped — silence is the
worse default in a port whose purpose is to be watched while it runs.

## The tokio layer

Ankurah's core is written against tokio, and the transpiler does not translate
tokio: it is a Rust runtime, not part of the family of code the port carries
over. `packages/base/src/tokio/` provides stand-ins that behave the way the
tokio types behave, and the transpiler maps the crate onto them by identity —
a path rewrite and nothing else.

Two spellings reach the same objects. `tokio` mirrors the crate's module tree,
for a path-qualified `tokio::sync::mpsc::channel(1024)`; the flat names are for
`use tokio::sync::Notify;`, which is how the corpus almost always writes it.
tokio's `Mutex` and `RwLock` carry an `Async` prefix in the flat spelling,
because `std::sync::Mutex` and `std::sync::RwLock` are ported too and a bare
name would be ambiguous; under `tokio.sync` they are spelled as tokio spells
them, and both classes answer to tokio's method names as well as this runtime's.

| Rust | TS | Notes |
|------|-----|-------|
| `tokio::spawn(fut)` | `spawn(fut)` | Also accepts an async function, which is called from a fresh turn — tokio does not poll a spawned future on the spawning thread. Returns a `JoinHandle`. |
| `tokio::task::spawn_local` | `spawn_local` | The same thing here: one thread, so tokio's distinction has nothing to distinguish. |
| `tokio::task::yield_now()` | `yield_now()` | A macrotask turn, so timers get their turn too. |
| `tokio::task::JoinHandle<T>` | `JoinHandle<T>` | `abort()`, `is_finished()`; awaiting yields `Result<T, JoinError>`. Dropping it detaches the task. |
| `tokio::task::JoinError` | `JoinError` | `is_cancelled()`, `is_panic()` borrow; `into_panic()` and `try_into_panic()` take `self` and move the payload out. |
| `tokio::select! { … }` | `select([{ tag, promise }, …])` | Resolves to `{ tag, value }`. A macro cannot be a function, so the arm bodies stay with the emitter. |
| `tokio::sync::Notify` | `Notify` | `notified()`, `notify_one()`, `notify_last()`, `notify_waiters()`. One permit, one broadcast generation. |
| `tokio::sync::Notified<'_>` | `Notified` | Records the broadcast generation at construction and joins the queue at its first poll. `enable()` performs that poll. |
| `tokio::sync::oneshot::channel()` | `oneshot.channel()` | Returns `[Sender, Receiver]`, in Rust's order. |
| `oneshot::Sender::send(self, v)` | `sender.send(v)` | `Result<undefined, T>`. Takes `self`, so it moves the sender. |
| `oneshot::Receiver<T>` | `oneshot.Receiver<T>` | Awaiting yields `Result<T, RecvError>`; also `try_recv()`, `close()`. |
| `oneshot::error::{RecvError, TryRecvError}` | `oneshot.RecvError`, `oneshot.TryRecvError` | The `error` module is flattened into its parent. |
| `tokio::sync::mpsc::channel(n)` | `mpsc.channel(n)` | Sending waits for capacity. A buffer of 0 panics, as in tokio. |
| `mpsc::unbounded_channel()` | `mpsc.unbounded_channel()` | |
| `mpsc::{Sender, Receiver, UnboundedSender, UnboundedReceiver}` | the same four names, flat | For `use tokio::sync::mpsc::Sender;`. These four hold the bare names, so oneshot's two ends keep the namespace form — `oneshot.Sender`, `oneshot.Receiver`. |
| `Sender::{send, try_send, clone}` | the same names | A send that fails hands its value back to the caller. |
| `Receiver::recv()` | `receiver.recv()` | `Promise<T \| null>`; `null` is Rust's `None` — every sender dropped and the buffer drained. |
| `mpsc::error::{SendError, TrySendError, TryRecvError}` | `mpsc.SendError`, … | Flattened the same way. |
| `tokio::sync::Mutex<T>` | `AsyncMutex<T>`, or `tokio.sync.Mutex` | `lock()` is tokio's name for this runtime's `acquire()` and is the same call; also `try_lock()`, `into_inner()`, `get_mut()`. |
| `tokio::sync::RwLock<T>` | `AsyncRwLock<T>`, or `tokio.sync.RwLock` | `read()`/`write()` are async; `try_read()`/`try_write()` return a `Result`; also `into_inner()`, `get_mut()`. First-come-first-served, so a waiting writer cannot be starved. |
| `tokio::sync::TryLockError` | `TryLockError` | |
| `tokio::time::sleep(d)` | `sleep(ms)` | `Duration` is `Copy` and has no drop glue, so it crosses as a number of milliseconds. |
| `tokio::time::timeout(d, fut)` | `timeout(ms, promise)` | `Result<T, Elapsed>`. |

### Named futures have one consumer

`Notified`, `oneshot::Receiver` and `JoinHandle` are structs with `impl Future`
in Rust, not `async fn` calls, so the source can hold one, hand it to `select!`,
and drop it. Each has exactly one owner, and the first thing that takes it takes
it for good.

**Awaiting one moves it.** `.await` takes a future by value, exactly as
`Result`'s self-taking methods take a Result, so the emitter emits no drop for
an awaited future and every later use of the binding — a second await, a
`try_recv()`, a `drop()` — is a fatal use-after-move. Nothing is lost by not
running the drop glue: a future only completes once its own cleanup has nothing
left to do, because a `Notified` that completes has already left the waiter
queue and a `oneshot::Receiver` that completes has already outlived its sender.

**`select` takes a different claim.** It borrows each branch for the length of
the race — enough to make a competing await fatal — and leaves ownership with
the emitted scope. It takes the winner's output and nothing else.

**Cancelling one is dropping it before it completes**, and each `onDrop()` does
what tokio does there: a `Notified` leaves the waiter queue and hands any
one-at-a-time notification it was given but never received on to the next
waiter; a `oneshot::Receiver` closes the channel and releases a value in flight;
a `JoinHandle` detaches its task and releases a result nobody took.

**A `Notified` registers at its first poll, not at construction.** It records
the broadcast generation when it is created — that is what makes
`const n = notify.notified(); await doThing(); await n;` immune to a
`notify_waiters()` during `doThing` — but it consumes a stored permit and joins
the `notify_one` queue only when polled or `enable()`d. So a permit goes to the
waiter that is polled first, not the one created first, and wake order among
queued waiters is poll order.

### A channel end owns nothing

The queue belongs to the channel, so dropping one `Sender` out of five releases
no messages; the receiving end releases what is still queued when it drops. A
value nobody can receive any more is always released rather than stranded —
handed back to the caller where `send` can do that, dropped where it cannot.

**Dropping a `Notify` with a waiter queued on it, an `AsyncRwLock` under a
guard, or calling `into_inner()`/`get_mut()` on a lock somebody holds, is
fatal** — `fatalOutstandingGuard`, the same contract the other containers keep.
Rust's borrow checker makes all of them impossible, so reaching one means the
emitted scope is wrong. `into_inner()` moves the value to its caller and takes
the container out of the leak registry without dropping it, so the emitter
emits no drop after one.

**Constructors take an optional TS-only `label`**, as `Mutex` and `RwLock`
already do — `new Notify('systemReady')`, `oneshot.channel('peerRequest')`,
`spawn(fut, 'retryLoop')` — so a leak report names the site rather than only the
type.

### Cancellation does not carry over

`select!` drops the futures that lost, which cancels them: a `sleep` stops, a
`recv` gives up its place in the queue, a spawned task stops at its next await
point. Nothing cancels a Promise. A losing branch here runs to completion and
everything it does on the way it still does. The same gap runs through
`timeout`, which drops the future when the deadline wins, and `abort()`, which
in tokio stops the task.

What the layer does instead:

- `select` transfers exactly one branch's output and releases everything that
  arrives after the decision — which is what stops a losing lock guard from
  holding its lock forever.
- `timeout` releases a value that arrives after the deadline, so a late result
  is not reported as a leak.
- `abort()` resolves the handle with a cancelled `JoinError` at once and drops
  whatever the task eventually produces. `is_finished()` therefore reports the
  handle, not the task: it is true straight after `abort()` while the body is
  still running.

**What the emitter owes a select.** `select!` drops every branch future when it
returns, winner and losers alike, and an unwind out of it drops them too — so
the emitted scope drops all of them in a `finally`, not just the losers and not
only on the path where the select returned a tag. For a `Notified`, a
`oneshot::Receiver` or a `JoinHandle` that drop *is* the cancellation, and for a
`Notified` it is also what hands its notification on to the next waiter. A
losing `sleep`, `timeout` or spawned task cannot be cancelled at all.

**Arbitration is deterministic.** Among branches that are ready at the select's
first checkpoint, the earliest in the list wins; after that it is first past the
post. tokio's unbiased `select!` picks a random polling order among ready
branches, so a fixed order is one of its permitted outcomes — and it is exactly
what `biased;` asks for.

### A fatal inside a promise reaction

An ownership fatal that surfaces in a task body, a losing select branch, or the
release of a value that arrived after its deadline has no caller to be thrown
to: a throw there becomes a rejection nobody is listening to. Every one of those
sites routes through `reportAsyncFatal()`, which sets the poison latch and
re-raises the error from a fresh host task, so the throw still reaches the host.
Such a fatal is **never wrapped in a `JoinError`** — a `JoinError` is a Rust
error value the emitted code is entitled to handle, and an ownership bug is not
one — so a task that raises one leaves its handle unsettled.

`hostTask()` is `queueMicrotask` where the host has one and `setTimeout(cb, 0)`
where it does not; a Hermes build can have `FinalizationRegistry` without
`queueMicrotask`, and a `ReferenceError` there would bury the very report it was
called to deliver.

Anything that is *not* an ownership bug and has no caller — a detached task that
threw, a losing branch that failed after the select returned — goes to
`setOnDiagnostic(handler)`, which does nothing by default. tokio's runtime drops
a panic once the JoinHandle is gone, so silence is the faithful default; a host
that wants to see them wires the handler up.

### Durations

`Duration` is `Copy`, so it crosses as a number of milliseconds:
`Duration::from_millis(n)` is `n`, `Duration::from_secs(n)` is `n * 1000`. A
host timer cannot hold an arbitrary `Duration` — `setTimeout` keeps its delay in
a signed 32-bit field — so `sleep` and `timeout` chase a monotonic deadline in
hops the host can hold, which also stops a wait from finishing early when the
host coalesces a timer. Anything under a millisecond rounds up.

### Deliberately not provided

The emitter must reject these with a target-specific diagnostic rather than
emitting a call that resolves against the stub and then fails at runtime:

- **`tokio::sync::watch`** and **`tokio::time::interval`** — nothing in the
  corpus uses either.
- **`spawn_blocking`** — there is no thread pool in a browser, and running the
  closure inline would block the event loop, which is the one thing the call
  exists to avoid. The corpus uses it only in the native sqlite and sled
  engines, which the browser build does not reach.
- **`blocking_lock` / `blocking_read` / `blocking_write`** — a blocking lock on
  a single-threaded event loop is a deadlock, not a wait.
- **`tokio::net`** — the native websocket connectors, not the browser target.
- **`#[tokio::test]`** — a test-harness attribute, not a runtime primitive.

## Inherent limitations

- **No compile-time lifetimes.** Every check here is a runtime check, so a path
  that never runs is never checked.
- **FinalizationRegistry is non-deterministic.** A leak may be reported late, or
  not at all, and never on a host without the registry.
- **No move semantics.** The moved flag is the only thing standing between a
  consumed value and its next use.
- **`const x = arc` does not increment the refcount.** Only `arc.clone()` does.
- **The cascade cannot see `#private` fields**, which is why `ownedFields()`
  exists.
- **Foreign objects.** A class instance with no drop glue that the cascade
  reaches gets one `console.warn` per constructor name: whatever it owns will not
  be released, and it needs a provided type to wrap it.

## Retired

`using` declarations and `[Symbol.dispose]` were the original model. They are
retired: Hermes refuses `using`, so the transpiler emits explicit `.drop()` calls
and `try/finally` blocks instead. `[Symbol.dispose]` still exists on `AkObject`
and `Arc` and still delegates to `drop()`, but it is not the model and no new
code should reach for it; the cascade dispatches on `drop()`.

## A `&mut` to a value JavaScript copies is a cell

Rust's `&mut T` is a place the callee writes and the caller reads back. Where
`T` is something the port writes as a JavaScript object — a class, an array, a
`HashMap` — the reference is free: JavaScript already passes the object, and the
callee's writes land in the caller's value. Where `T` is a number, a string, a
boolean, a bigint or an `Option` of one, JavaScript passes a COPY, and the
callee's writes go nowhere at all. ankql's SQL generator takes
`buffer: &mut String` and `found_placeholders: &mut usize`, and every axis of
`selection/sql.ts` answered the empty string because the buffer the callee
filled was not the buffer the caller read.

So such a parameter is a `BorrowMut<T>` — the cell `@ankurah/base` already
exports for `&mut T` — and the emitted body reads and writes it through
`.value`. A LOCAL the body hands out that way is declared as the cell at its
`let`, so nothing has to be unboxed after the call; every read of it is
`.value` too.

The one place a cell is passed whole rather than read is an argument position
where the name is itself a `&mut` PARAMETER: Rust reborrows there and needs no
`&mut` to say so, and handing `buffer.value` on would give the callee a copy —
which is the defect this rule exists to remove.

**Only a LOCAL can be held in a cell.** `&mut c.field`, a returned `&mut usize`,
a `&'a mut String` in a struct: each of those is a place the port has no cell
for, and passing the field's value hands the callee a copy whose writes reach
nobody. The site says so and stops there (R12) rather than running an update
that goes nowhere. A cell that could stand for an arbitrary place — a getter and
a setter closed over the owner — would settle it, and nothing in the corpus has
needed one yet.

| Rust | TypeScript |
|---|---|
| `fn f(buffer: &mut String)` | `function f(buffer: BorrowMut<string>)` |
| `*count += 1` inside such a body | `count.value += 1` |
| `buffer.push_str("?")` inside such a body | `buffer.value += '?'` |
| `let mut buffer = String::new(); g(&mut buffer)` | `const buffer = new BorrowMut(''); g(buffer)` |
| `g(buffer)` where `buffer: &mut String` (a reborrow) | `g(buffer)` — the cell, not `buffer.value` |
| `fn f(v: &mut Vec<u8>)` | `function f(v: Uint8Array)` — an object is already a reference |

`BorrowMut` is `nonOwning`: it is a borrow, the value inside belongs to
somebody else, and dropping the cell releases nothing.

## A shape with no lowering is a hole, never the nearest thing

A Rust shape the transpiler cannot translate used to be reported to whoever ran
the transpiler and emitted anyway, as whatever the engine could write for it: a
consuming arm whose guard was dropped ran for the whole variant, an arm that
tests inside a payload ran for every value of it, a struct literal lost its
`..rest`. Each of those is code that RUNS and answers something Rust would not,
and a wrong answer at run time is a bug nobody traces back to a line printed
during a build weeks earlier.

So a known-wrong emission is written as a call to `unsupported('<the shape>')`
from `@ankurah/base`. It throws `UnsupportedShape`, naming the Rust shape it is
standing on, and its return type is `never`, so the hole stands wherever the
expression it replaces stood — an arm's value, a return, an argument — and
TypeScript narrows around it exactly as it did.

The diagnostic does not go away: it is still what tells the port's authors which
gaps are open, and the hole is what the running program does when it reaches
one. A hole is not an error anything catches, and nothing in the port handles
`UnsupportedShape`: it is the port saying this path was never translated.

| Rust | TypeScript |
|---|---|
| a shape the engine reports and cannot write | `unsupported('a consuming match arm with a guard')` |
| that hole reached at run time | `UnsupportedShape: the port has no translation for this: …` |

## A parse answers a `Result`, and never throws past its caller

`serde_json::from_str` answers `Result<T, serde_json::Error>`, and the port's
`serde_json.parse` answers `Result<unknown, JsonError>` for the same reason: at
seven live boundaries — storage-sqlite's engine, core's `system`, the property
value reader — the caller reads the failure as a value it owns and releases.

A reader that throws instead breaks that in two ways. The caller has no `catch`
around a call it was told answers a `Result`, so the throw travels to whatever
`await` is on the stack; and the `JsonError` the failure would have carried is
never built, so the position and the message are lost. The port's reader
therefore raises its own `Fault` for every refusal and `parse` turns each into
an `Err` — including the ones the host's `JSON.parse` refuses by throwing, which
are wrapped rather than let past:

| document | serde_json | the port |
|---|---|---|
| `"a<U+0001>b"` | `Err(control character … while parsing a string)` | the same, as an `Err` |
| `"\uZZZZ"` | `Err(invalid escape)` | `Err(invalid string)` |
| `01`, `1.`, `.5`, `1e`, `1e+` | `Err` — none is a JSON number | `Err(invalid number)` |
| `1e999` | `Err(number out of range)` | the same |

`Number()` accepts all five of the malformed tokens (`Number('01')` is 1,
`Number('.5')` is 0.5, `Number('1e999')` is `Infinity`), which is why reading a
token with it accepted documents Rust refuses.

## `entry` owns the key it was handed

`map.entry(k)` is the one place Rust's map API takes the key BEFORE it knows
whether it needs it, and every one of the three ways of finishing the entry —
`or_insert(v)`, `or_insert_with(f)`, `or_default()` — consumes it. So the port's
`MapEntry` releases what the map turned out not to need:

| Rust | occupied | vacant |
|---|---|---|
| `entry(k).or_insert(v)` | `k` and `v` are both released | `k` and `v` go into the map |
| `entry(k).or_insert_with(f)` | `k` and `f` are released; `f` is not called | `f` is invoked and its answer goes in |
| `entry(k).or_default()` | the port spells it `orDefault(thunk)` — there is no `V: Default` to read | the same |

What `or_insert` hands back is a `&mut V` INTO THE MAP, not a copy of the value:
`*counts.entry(w).or_insert(0) += 1` has to count, and a plain `BorrowMut` holds
a copy, so the increment landed on the copy and the map never changed. Writing
through it releases what the map held, which is what Rust's `*slot = v` does.

The `&mut V` points at the STORED ENTRY, never at the lookup key. An occupied
entry releases the key it was handed — the map keeps the one it already has —
and a slot holding that released key hashed a dropped value the first time
anything read it. The entry is the stable thing: one object for as long as the
map holds it, and no second lookup.

The key is the entry's until the map takes it. A vacant `or_insert_with(f)`
whose `f` throws leaves the key with nobody, and Rust's unwind does not: the
throw path releases it.

`clone()` on either container walks what it holds. A value's own `clone()` where
it has one; an array element by element, by the same rule, because that is what
the port writes a `Vec<T>` and a tuple as; a typed array copied through its own
constructor, because that is what it writes a `Vec<u8>` as. Neither of the last
two has a `clone()` of its own, so both used to come back as the very same
object — and a cloned map that shared its arrays owned one set of elements
twice, so dropping both maps dropped each element twice.

## An eager combinator argument is built before the branch and released in the other one

`Option::ok_or(err)` and `Option::map_or(default, f)` take VALUES, not closures:
Rust builds the argument before it looks at the option, and drops it again on
the path that hands it nowhere. Two things follow, and the port owes both.

The value is named before the branch, so the work it does happens where Rust
does it. And the branch that does NOT hand it on releases it:

```ts
const _m1 = new RetrievalError('NoDurablePeers', {});
return (_m0 != null ? (_m1.drop(), Result.Ok(_m0!)) : Result.Err(_m1));
```

The release stands FIRST inside that branch, before the value the branch hands
on. Rust drops the argument whether the branch returns or panics, and a release
written before the branch's own work runs on both of those paths without a
`try`. Nothing the branch does can observe the difference, because the argument
was moved into the call and no other name reaches it.

An argument that is already a place — a local handed to `ok_or` — is not named
again, and still gets the release: the move is a move whether or not the port
had to write a name for it.

The one case with no release is an argument whose type the engine could not
name. A release written against a guess would drop a value somebody else owns,
so none is written and the site is reported. Two sites in the corpus, both
`Poll::Ready(None)` under an open item type in `storage/common/src/sorting.rs`.

## A callable parameter written by value is the body's

Rust's `fn f<F: Fn(u32) -> u32>(g: F)` takes `g` BY VALUE. It is dropped at the
end of the body; only the CALL borrows it, and the body may call it as often as
the bound allows. So the port owes two separate things, and reading them as one
is what leaked.

The CALL goes through `invokeRef`, which calls and leaves the closure whole —
for an `Fn` or `FnMut` bound whether the parameter is written `F`, `&F` or
`&mut F`, because a call under any of those borrows. `invoke`, which calls and
then releases, is for the one case where the call itself consumes the closure: a
bound that is `FnOnce` and nothing else, on a parameter written by value.

The RELEASE belongs to the parameter, not to the call: a by-value callable
parameter is one of the body's owned values, released in the same `finally` as
any other, and released nowhere if the body hands it on. It is written
`dropOwned(f)`, because either shape may arrive — a plain function reaches none
of `dropOwned`'s branches and is left alone, and an `OwnedClosure` has a
`drop()` for it to find.

A parameter written `&F` or `&mut F` is somebody else's and the body releases
nothing.

## The five `Result` methods that take a closure release it either way

`map`, `map_err`, `and_then`, `or_else` and `unwrap_or_else` take `f` by value in
Rust, so `f` is dropped at the end of the call whichever variant the `Result`
turned out to be: `Ok(7).unwrap_or_else(f)` never calls `f` and still drops it.
The branch that calls it leaves the release to `invoke`; the branch that does not
call it releases `f` itself.

## A `Result` matched against a reference is read, not taken apart

Rust's `match &result { Ok(v) => … }` binds `v: &T` and leaves the `Result`
whole; `match result { Ok(v) => … }` binds `v: T` and consumes it. `unwrap()`
and `unwrapErr()` are the `self` forms — each takes the payload out and marks
the `Result` moved — so reading a borrowed match with one of them made the
second read of the same value `Result was used after being moved`.

`okRef()` and `errRef()` are the `&self` forms. They check the variant, read the
payload and leave the `Result` its owner's, and they are what the emitter writes
wherever the value being taken apart is a reference — a `match`, an `if let`, a
`while let`, and an inner `Result` under a borrowed `Option`.

## A decoder's cleanup is a bag and a `finally`, not a closure per return

R4: a decoder owns what it has built until it RETURNS one. The emitted
`fromJson` used to write the release into every early `return` — a closure that
dropped the fields decoded so far and then answered `Err`. That covers an
expected failure and not an EXCEPTION: a throwing property getter on a late
field left every earlier field with nobody, because the outer `catch` answers
`Err` and no release edge ran.

So a reader that owns anything declares a bag and a flag, fills the bag as it
goes, sets the flag once it has built the value, and releases the bag in a
`finally` unless the flag is set:

```ts
const $built: unknown[] = [];
let $kept = false;
try {
  …
  $built.push(kid);
  …
  const $out = new Outer(kid, kids, last);
  $kept = true;
  return Result.Ok($out);
} catch (e) { … } finally {
  if (!$kept) dropOwned($built);
}
```

The flag is set AFTER the value is built and before it is returned, so a
constructor that raised would still leave the fields to the `finally` — which is
what Rust's unwind does. A reader with nothing to release writes neither the bag
nor the `finally`.

## The arithmetic helpers own nothing, and the two that can refuse say so

`@ankurah/base`'s `ops.ts` holds the integer arithmetic every emitted body goes
through, and none of it participates in ownership: the operands are numbers and
`bigint`s, which have no drop glue, and the helpers return fresh values. They
are listed here because emitted code calls them constantly and a reader looking
for who releases what should find the answer rather than nothing.

`checkedAdd` and its four siblings PANIC where Rust's debug build panics.
`checkedAddOption` and its siblings answer the `Option` Rust's `checked_*`
methods answer, which the port writes as `T | null` — a value that owns nothing,
so a discarded `None` releases nothing and a `?` across one carries nothing.
`checkedDivOption` and `checkedRemOption` answer `null` on exactly the two cases
`checkedDiv` and `checkedRem` raise on: a zero divisor, and `MIN` over `-1`,
whose quotient the type cannot hold.

| Rust | TypeScript |
|---|---|
| `a.checked_div(b)` | `checkedDivOption(a, b, 'i32')` — `null` for a zero divisor and for `MIN / -1` |
| `a.checked_rem(b)` | `checkedRemOption(a, b, 'i32')` — the same two |
| `a / b` on integers | `checkedDiv(a, b, 'i32')` — raises where Rust panics |

## A clone that throws part-way releases what it had already cloned

`#[derive(Clone)]` on a container clones every element, and an element's own
`clone()` can throw — a panic inside it, or the runtime refusing to clone a
value that declares no `clone()`. Whatever has been cloned by then belongs to
nobody: the caller never received the new container, so no emitted `finally`
names it and the only thing that notices is the leak check, long after.

So the runtime's container clones build into a LOCAL list, release the whole
list if any element throws, and only then construct the container and fill it.
The destination is built last on purpose: a `HashMap` registers itself with the
drop registry when it is constructed, so a destination built first and filled as
the walk went left a registered half-built map behind as well as the pairs in
it. A map clones each key into the list before cloning its value, so a throwing
value clone does not orphan the key beside it.

| Rust | TypeScript |
|---|---|
| `map.clone()` where an element panics | every clone made so far is dropped, no map is built, and the panic passes out |
| `vec.clone()` where an element panics | every clone made so far is dropped, and the panic passes out |

## Text crossing into the port is UTF-8, and carries no lone surrogate

Rust's `String` and `str` are UTF-8: every byte sequence in one is valid UTF-8,
and no code point in one is a surrogate, because a surrogate cannot be encoded
in UTF-8 at all. A JavaScript string is a sequence of UTF-16 code units and
holds a lone surrogate happily, `TextDecoder` replaces an invalid byte sequence
with U+FFFD rather than refusing it, and `JSON.parse` and `JSON.stringify` both
pass a lone surrogate straight through. So each of those checks is the port's to
make, and `packages/base/src/std/utf8.ts` is where the two of them live.

`serde_json.fromSlice(bytes)` is `serde_json::from_slice`: the bytes are decoded
fatally and an invalid sequence answers `Err`, where the host's decoder answered
a document with a replacement character in it. It owns nothing the caller does
not already own — the `JsonError` in an `Err` is the caller's, as it is for
`parse`.

| Rust | TypeScript |
|---|---|
| `serde_json::from_slice(&bytes)` | `serde_json.fromSlice(bytes)` |
| bytes that are not UTF-8 | `Err(invalid utf-8 sequence)`, not a U+FFFD |
| a document carrying a lone surrogate, raw or escaped | `Err`, as serde_json answers |
| a `String` carrying a lone surrogate, written out | `Err`, because Rust could not have held it |

`for (k, v) in map` MOVES the map into Rust's `IntoIter`, which hands out an
owned pair each turn and drops whatever it has not handed out when the loop
ends — however it ends. `intoEntries()` is that move: it empties the map, marks
it dropped and hands the pairs over, releasing nothing, so from there every pair
belongs to whoever walks the array and the tail nobody reached is that caller's.
A map used after it reports a use after drop, which is the run-time spelling of
the move Rust refuses at compile time. A loop over `&map` is none of this: it
borrows, and `entries()` is what it reads.

| Rust | TypeScript |
|---|---|
| `for (k, v) in map` | `const pairs = map.intoEntries();` then the index walk, with `dropOwned(pairs.slice(at))` in its `finally` |
| `for (k, v) in &map` | `for (const [k, v] of map.entries())`, releasing neither |
| a `break` out of either | the tail is released by the first, and nothing by the second |

The BINCODE readers owe the same answer, because they read the same bytes: a
bincode `String` field is a length and a byte run that Rust wrote out of a
`String`, so a run that is not valid UTF-8 could not have come from one and
`serde` errors there. `packages/proto/src/codec.ts`,
`packages/ankql/src/codec.ts` and the `Json` variant of
`packages/core/src/property/backend/lww.ts` each hold a `TextDecoder` of their
own, and each is fatal, with the exception turned into that codec's own error.
A U+FFFD in its place is a different string flowing on as though it had been
read — a silent corruption where Rust reports.

## An arithmetic panic names the operation Rust names

`checkedAdd`, `checkedSub`, `checkedNeg`, `checkedMul`, `checkedDiv` and
`checkedRem` each raise where Rust's debug build panics, with the message Rust
prints — "attempt to negate with overflow", not "attempt to subtract with
overflow". A signed width has one more negative value than positive, so `MIN`
has no positive and `i64::MIN.abs()` is that panic: written as a subtraction it
raised at exactly the right value and named the wrong operation, which is the
line a reader greps for.
