// MIRRORS: ankurah/core/src/util/cast.rs
//
// `into!` and `create!` exist so an application can write a model literal without
// spelling `.into()` on every field: `into!(ConnectionEvent { &user, session:
// &session_view, timestamp: ts })` is `ConnectionEvent { user: (&user).into(),
// session: (&session_view).into(), timestamp: ts.into() }`. The facade re-exports
// both, so they are the port's public API, and nothing inside the corpus calls
// them — every call site is in an application.
//
// TWO DELIBERATE DIVERGENCES, because a macro reads the call site at compile time
// and a function cannot:
//
// 1. FIELDS ARE POSITIONAL HERE, NOT NAMED. The macro writes a struct literal, so
//    Rust checks each value against the field it was written beside. A ported
//    struct is built through its constructor, whose parameters are its fields in
//    declaration order, so `into(ConnectionEvent, user, sessionView, ts)` passes
//    the same three values in the same order — and two same-typed fields written
//    the wrong way round are accepted here where Rust would refuse them.
//
// 2. THE CONVERSION IS PICKED BY THE SOURCE VALUE, NOT THE TARGET FIELD.
//    `Into::into` in Rust resolves against the field's declared type, so one type
//    may convert into several. Nothing at runtime here knows what a constructor
//    parameter's type is, so a value converts through its own `into()` if it has
//    one and is passed through untouched otherwise — which is Rust's reflexive
//    `impl<T> Into<T> for T`. A source type with two `Into` impls cannot be
//    expressed; it has to convert at the call site instead.

/** A value that converts itself on the way into a model field: the port's `Into`. */
export interface Into<T> {
  into(): T;
}

function hasInto(value: unknown): value is Into<unknown> {
  return typeof (value as { into?: unknown } | null)?.into === 'function';
}

/** One field value, through its own conversion where it declares one. */
function intoValue(value: unknown): unknown {
  return hasInto(value) ? value.into() : value;
}

/**
 * Rust: `into!` — build `Ctor` from its field values, each through `.into()`.
 *
 * `args` are the fields in declaration order, which is the order the emitted
 * constructor takes them in.
 */
export function into<T, A extends unknown[]>(Ctor: new (...args: A) => T, ...args: A): T {
  return new Ctor(...(args.map(intoValue) as A));
}

/**
 * Rust: `create!` — `trx.create(&into!(Ty { .. }))`.
 *
 * `Transaction::create` is async and answers a Result, so this awaits and passes
 * that Result straight out; the caller tests it exactly as it would the
 * transaction's own answer.
 */
export function create<T, R, A extends unknown[]>(
  trx: { create(model: T): Promise<R> },
  Ctor: new (...args: A) => T,
  ...args: A
): Promise<R> {
  return trx.create(into(Ctor, ...args));
}
