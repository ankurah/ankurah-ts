//! Leg A: an OPAQUE iterator is a cursor, not the whole sequence.
//!
//! The port writes an iterator as the array it walks, which is right for every
//! chain it can see through. It is wrong for the one shape the array cannot
//! express: a generic body that takes `I: Iterator<Item = V>` and calls
//! `next()` by hand. `next` moves ONE element out and leaves the rest in the
//! iterator, and an array has nowhere to record how far the walk has got — so
//! `next` there was a hole. `ankql`'s `Predicate::populate` is exactly this
//! shape: it pulls one value per placeholder and then asks the iterator whether
//! anything is left over.

pub struct Token {
    pub n: u32,
}

pub struct Refused;

/// Pull `wanted` elements out of the iterator by hand and hand back their sum.
/// Everything the walk did not reach is still the ITERATOR's, and Rust drops it
/// when the iterator goes out of scope.
pub fn take_some<I: Iterator<Item = Token>>(values: &mut I, wanted: u32) -> Result<u32, Refused> {
    let mut total = 0;
    let mut taken = 0;
    while taken < wanted {
        match values.next() {
            Some(token) => {
                total += token.n;
                taken += 1;
            }
            None => return Err(Refused),
        }
    }
    Ok(total)
}

/// The whole shape: build the iterator, walk part of it, and then ask whether
/// anything is left. The iterator is dropped at the end of the function, and
/// what it still holds goes with it.
pub fn sum_first<I: IntoIterator<Item = Token>>(values: I, wanted: u32) -> Result<u32, Refused> {
    let mut walk = values.into_iter();
    let total = take_some(&mut walk, wanted)?;
    if walk.next().is_some() {
        return Err(Refused);
    }
    Ok(total)
}

/// A cursor asked for anything but `next`: `for v in walk` and
/// `walk.collect()` each consume the iterator and see exactly what it has not
/// handed out. Written without that, the loop iterated a value with no
/// `Symbol.iterator` and `collect` answered a cursor where an array stood.
pub fn rest_of<I: IntoIterator<Item = Token>>(values: I, skip: u32) -> Result<Vec<Token>, Refused> {
    let mut walk = values.into_iter();
    take_some(&mut walk, skip)?;
    let mut kept: Vec<Token> = Vec::new();
    for token in walk {
        kept.push(token);
    }
    Ok(kept)
}
