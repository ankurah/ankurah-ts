//! `std::iter` — the free constructors.
//!
//! Not one of the file names the deliverable listed; `once`, `empty` and
//! `repeat` are sources rather than adaptors, and putting them in
//! `adapters.rs` would misfile them against std's own module.

pub struct Once<T>;
pub struct Empty<T>;
pub struct Repeat<A>;
pub struct RepeatWith<F>;
pub struct Successors<T, F>;
pub struct FromFn<F>;

pub fn once<T>(value: T) -> Once<T> { todo!() }
pub fn empty<T>() -> Empty<T> { todo!() }
pub fn repeat<T: Clone>(elt: T) -> Repeat<T> { todo!() }
pub fn repeat_with<A, F: FnMut() -> A>(repeater: F) -> RepeatWith<F> { todo!() }
pub fn successors<T, F: FnMut(&T) -> Option<T>>(first: Option<T>, succ: F) -> Successors<T, F> { todo!() }
pub fn from_fn<T, F: FnMut() -> Option<T>>(f: F) -> FromFn<F> { todo!() }

impl<T> Iterator for Once<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T> Iterator for Empty<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T: Clone> Iterator for Repeat<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<A, F: FnMut() -> A> Iterator for RepeatWith<F> { type Item = A; fn next(&mut self) -> Option<A> { todo!() } }
impl<T, F: FnMut(&T) -> Option<T>> Iterator for Successors<T, F> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

// `for i in 0..n` — the range is the iterator.
impl<A: Step> Iterator for std::ops::Range<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> { todo!() }
}
impl<A: Step> DoubleEndedIterator for std::ops::Range<A> {
    fn next_back(&mut self) -> Option<A> { todo!() }
}
// std implements `ExactSizeIterator` for `Range<T>` only where the range's
// length is guaranteed to fit a `usize`, by macro over a fixed list. `u32` and
// `i32` are on it for backwards compatibility even though they are wrong on a
// 16-bit target; `u64`, `i64` and `char` are not on it at all.
impl ExactSizeIterator for std::ops::Range<usize> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<u8> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<u16> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<u32> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<isize> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<i8> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<i16> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for std::ops::Range<i32> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for RangeInclusive<usize> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for RangeInclusive<u8> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for RangeInclusive<u16> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for RangeInclusive<i8> { fn len(&self) -> usize { todo!() } }
impl ExactSizeIterator for RangeInclusive<i16> { fn len(&self) -> usize { todo!() } }
impl<A: Step> Iterator for RangeInclusive<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> { todo!() }
}
impl<A: Step> Iterator for RangeFrom<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> { todo!() }
}

/// `Step` is unstable in real std; it is written here because `Range<A>: Iterator`
/// has to be bounded by something and an unbounded impl would let `Range<String>`
/// resolve as an iterator.
pub trait Step: Clone + PartialOrd<Self> {}

impl Step for u8 {}
impl Step for u16 {}
impl Step for u32 {}
impl Step for u64 {}
impl Step for usize {}
impl Step for i8 {}
impl Step for i16 {}
impl Step for i32 {}
impl Step for i64 {}
impl Step for isize {}
impl Step for char {}
