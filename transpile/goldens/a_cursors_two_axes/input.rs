// The two ownership axes of a cursor: its ELEMENTS and its REST.
//
// A cursor is what the port holds an opaque `Iterator` as. It owns two
// different things, and each has its own question. The ELEMENTS: `Item = Token`
// walks values the cursor owns and must release, `Item = &Token` walks the
// caller's and must release none of them. The REST: a method that takes the
// iterator by VALUE consumes the cursor, and one that takes it by `&mut self`
// leaves it with its owner — and the short-circuiting ones leave in it
// everything they did not walk past.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

/// `any` takes `&mut self` and stops at the first match: the tokens after it
/// are still the walk's, and the frame's own drop of the cursor releases them.
/// The predicate is handed each token BY VALUE and is its owner.
pub fn seen<I>(mut walk: I) -> bool
where
    I: Iterator<Item = Token>,
{
    walk.any(|t| t.n == 1)
}

/// `all` stops at the first failure, the same way.
pub fn every<I>(mut walk: I) -> bool
where
    I: Iterator<Item = Token>,
{
    walk.all(|t| t.n > 0)
}

/// `nth` drops what it steps over, hands back the one it lands on, and keeps
/// the tail.
pub fn third<I>(mut walk: I) -> Option<Token>
where
    I: Iterator<Item = Token>,
{
    walk.nth(2)
}

/// `size_hint` takes `&self`: it reads the cursor and consumes nothing.
pub fn how_many_left<I>(walk: &I) -> usize
where
    I: Iterator<Item = Token>,
{
    walk.size_hint().0
}

/// `find` and then `next`: the second call has to find the cursor alive, with
/// the tail the first one did not walk past still in it.
pub fn find_then_next<I>(mut walk: I) -> (Option<Token>, Option<Token>)
where
    I: Iterator<Item = Token>,
{
    let found = walk.find(|t| t.n == 2);
    let after = walk.next();
    (found, after)
}

/// `any` and then `count`: the first leaves the cursor with its owner, and the
/// second consumes it.
pub fn any_then_count<I>(mut walk: I) -> usize
where
    I: Iterator<Item = Token>,
{
    let _ = walk.any(|t| t.n == 1);
    walk.count()
}

/// A walk over the CALLER's tokens, read to the end: the cursor owns none of
/// them, so dropping it releases nothing.
pub fn count_refs<'a, I>(walk: I) -> usize
where
    I: Iterator<Item = &'a Token>,
{
    let mut n = 0;
    for _t in walk {
        n += 1;
    }
    n
}

/// The same walk left UNREAD, and the same walk read part way: the frame drops
/// the cursor with elements still in it, and they are the caller's.
pub fn ignore_refs<'a, I>(_walk: I) -> usize
where
    I: Iterator<Item = &'a Token>,
{
    0
}

pub fn one_ref<'a, I>(mut walk: I) -> bool
where
    I: Iterator<Item = &'a Token>,
{
    walk.next().is_some()
}

/// The caller side of a BORROWED walk: `tokens.iter()` hands over a walk that
/// points at tokens this frame does not own, so the cursor built for it
/// releases none of them.
pub fn borrowed_shapes(tokens: &Vec<Token>) -> usize {
    let read = count_refs(tokens.iter());
    let unread = ignore_refs(tokens.iter());
    let partial = one_ref(tokens.iter());
    read + unread + (partial as usize)
}

/// The caller side of an OWNED walk: `into_iter()` hands the tokens over, and
/// what the callee does not walk past it drops.
pub fn owned_shapes(tokens: Vec<Token>) -> bool {
    seen(tokens.into_iter())
}
