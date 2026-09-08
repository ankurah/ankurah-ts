// What the CALLEE's `Fn` bound says a closure is, and what the call site fixes
// of it.
//
// A closure written at a bounded parameter has no type of its own: what its
// parameter holds is what the callee's `Fn` bound says. Two ways that answer
// was missed. Parentheses are punctuation, and matching a literal
// `Expr::Closure` meant `apply((|held| held.n))` read no bound at all — the
// parameter was typed by nothing, so the value it was handed BY VALUE was
// released by nobody. And the bound was read with the callee's own parameters
// still OPEN: `generic_apply<T, F: Fn(T) -> u32>(v: T, f: F)` called as
// `generic_apply(token, |x| ..)` typed `x` as `T` — "no field `n` on `T`" — and
// released nothing either. What the SIBLING actual fixes has to be bound first.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn apply<F>(f: F) -> i64
where
    F: Fn(Token) -> i64,
{
    f(Token { n: 1 })
}

pub fn generic_apply<T, F>(v: T, f: F) -> i64
where
    F: Fn(T) -> i64,
{
    f(v)
}

/// A closure inside parentheses is still a closure.
pub fn parenthesised() -> i64 {
    apply((|held| held.n))
}

/// The same without them, which always worked.
pub fn plain() -> i64 {
    apply(|held| held.n)
}

/// The callee's `T` is fixed by the argument beside the closure.
pub fn from_a_sibling(t: Token) -> i64 {
    generic_apply(t, |x| x.n)
}
