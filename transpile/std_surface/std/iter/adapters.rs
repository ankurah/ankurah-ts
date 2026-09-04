//! `std::iter` — the adaptor structs.
//!
//! The `Item` on each `impl Iterator` is what makes a chain typeable:
//! `v.values().cloned().collect::<Vec<_>>()` needs `Values::Item = &V`, then
//! `Cloned<I>::Item = V`, then `Vec<V>: FromIterator<V>`.

pub struct Map<I, F>;
impl<B, I: Iterator, F: FnMut(<I as Iterator>::Item) -> B> Iterator for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { todo!() }
}
impl<B, I: DoubleEndedIterator, F: FnMut(<I as Iterator>::Item) -> B> DoubleEndedIterator for Map<I, F> {
    fn next_back(&mut self) -> Option<B> { todo!() }
}
impl<B, I: ExactSizeIterator, F: FnMut(<I as Iterator>::Item) -> B> ExactSizeIterator for Map<I, F> {
    fn len(&self) -> usize { todo!() }
}
impl<I: Clone, F: Clone> Clone for Map<I, F> { fn clone(&self) -> Map<I, F> { todo!() } }

pub struct Filter<I, P>;
impl<I: Iterator, P: FnMut(&<I as Iterator>::Item) -> bool> Iterator for Filter<I, P> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: DoubleEndedIterator, P: FnMut(&<I as Iterator>::Item) -> bool> DoubleEndedIterator for Filter<I, P> {
    fn next_back(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct FilterMap<I, F>;
impl<B, I: Iterator, F: FnMut(<I as Iterator>::Item) -> Option<B>> Iterator for FilterMap<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { todo!() }
}

pub struct Cloned<I>;
impl<'a, T: 'a + Clone, I: Iterator<Item = &'a T>> Iterator for Cloned<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> { todo!() }
}
impl<'a, T: 'a + Clone, I: DoubleEndedIterator<Item = &'a T>> DoubleEndedIterator for Cloned<I> {
    fn next_back(&mut self) -> Option<T> { todo!() }
}
impl<'a, T: 'a + Clone, I: ExactSizeIterator<Item = &'a T>> ExactSizeIterator for Cloned<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct Copied<I>;
impl<'a, T: 'a + Copy, I: Iterator<Item = &'a T>> Iterator for Copied<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> { todo!() }
}
impl<'a, T: 'a + Copy, I: ExactSizeIterator<Item = &'a T>> ExactSizeIterator for Copied<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct Enumerate<I>;
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, <I as Iterator>::Item);
    fn next(&mut self) -> Option<(usize, <I as Iterator>::Item)> { todo!() }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Enumerate<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct Zip<A, B>;
impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> {
    type Item = (<A as Iterator>::Item, <B as Iterator>::Item);
    fn next(&mut self) -> Option<(<A as Iterator>::Item, <B as Iterator>::Item)> { todo!() }
}
impl<A: ExactSizeIterator, B: ExactSizeIterator> ExactSizeIterator for Zip<A, B> {
    fn len(&self) -> usize { todo!() }
}

pub struct Chain<A, B>;
impl<A: Iterator, B: Iterator<Item = <A as Iterator>::Item>> Iterator for Chain<A, B> {
    type Item = <A as Iterator>::Item;
    fn next(&mut self) -> Option<<A as Iterator>::Item> { todo!() }
}

pub struct Rev<I>;
impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    fn next_back(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: DoubleEndedIterator + ExactSizeIterator> ExactSizeIterator for Rev<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct Peekable<I: Iterator>;
impl<I: Iterator> Peekable<I> {
    pub fn peek(&mut self) -> Option<&<I as Iterator>::Item> { todo!() }
    pub fn peek_mut(&mut self) -> Option<&mut <I as Iterator>::Item> { todo!() }
    pub fn next_if<F: FnOnce(&<I as Iterator>::Item) -> bool>(&mut self, func: F) -> Option<<I as Iterator>::Item> { todo!() }
    pub fn next_if_eq<T: ?Sized>(&mut self, expected: &T) -> Option<<I as Iterator>::Item> where <I as Iterator>::Item: PartialEq<T> { todo!() }
}
impl<I: Iterator> Iterator for Peekable<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct Take<I>;
impl<I: Iterator> Iterator for Take<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct Skip<I>;
impl<I: Iterator> Iterator for Skip<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {
    fn len(&self) -> usize { todo!() }
}

pub struct TakeWhile<I, P>;
impl<I: Iterator, P: FnMut(&<I as Iterator>::Item) -> bool> Iterator for TakeWhile<I, P> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct SkipWhile<I, P>;
impl<I: Iterator, P: FnMut(&<I as Iterator>::Item) -> bool> Iterator for SkipWhile<I, P> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct MapWhile<I, P>;
impl<B, I: Iterator, P: FnMut(<I as Iterator>::Item) -> Option<B>> Iterator for MapWhile<I, P> {
    type Item = B;
    fn next(&mut self) -> Option<B> { todo!() }
}

pub struct StepBy<I>;
impl<I: Iterator> Iterator for StepBy<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct FlatMap<I, U, F>;
impl<I: Iterator, U: IntoIterator, F: FnMut(<I as Iterator>::Item) -> U> Iterator for FlatMap<I, U, F> {
    type Item = <U as IntoIterator>::Item;
    fn next(&mut self) -> Option<<U as IntoIterator>::Item> { todo!() }
}

pub struct Flatten<I: Iterator>;
impl<I: Iterator> Iterator for Flatten<I> where <I as Iterator>::Item: IntoIterator {
    type Item = <<I as Iterator>::Item as IntoIterator>::Item;
    fn next(&mut self) -> Option<<<I as Iterator>::Item as IntoIterator>::Item> { todo!() }
}

pub struct Scan<I, St, F>;
impl<B, I: Iterator, St, F: FnMut(&mut St, <I as Iterator>::Item) -> Option<B>> Iterator for Scan<I, St, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { todo!() }
}

pub struct Fuse<I>;
impl<I: Iterator> Iterator for Fuse<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct Inspect<I, F>;
impl<I: Iterator, F: FnMut(&<I as Iterator>::Item)> Iterator for Inspect<I, F> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub struct Cycle<I>;
impl<I: Clone + Iterator> Iterator for Cycle<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
