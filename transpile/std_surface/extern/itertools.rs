//! `itertools` 0.14.0
//!
//! Not on the deliverable's list, but the corpus reaches it in non-test code:
//! `core/src/node.rs` calls `exactly_one()` twice to enforce "only one cdata is
//! permitted", and `core/src/livequery.rs::ids_sorted` calls `sorted()`. Both
//! are `Itertools` blanket methods over `Iterator`; without the declaration
//! those two receivers have no method and the run stops.

pub trait Itertools: Iterator {
    fn exactly_one(self) -> Result<Self::Item, ExactlyOneError<Self>> where Self: Sized;
    fn at_most_one(self) -> Result<Option<Self::Item>, ExactlyOneError<Self>> where Self: Sized;
    fn sorted(self) -> std::vec::IntoIter<Self::Item> where Self: Sized, Self::Item: Ord;
    fn sorted_by<F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering>(self, cmp: F) -> std::vec::IntoIter<Self::Item> where Self: Sized;
    fn sorted_by_key<K: Ord, F: FnMut(&Self::Item) -> K>(self, f: F) -> std::vec::IntoIter<Self::Item> where Self: Sized;
    fn unique(self) -> Unique<Self> where Self: Sized, Self::Item: Clone + Eq + Hash;
    fn dedup(self) -> Dedup<Self> where Self: Sized, Self::Item: PartialEq<Self::Item>;
    fn join(&mut self, sep: &str) -> String where Self::Item: Display;
    fn collect_vec(self) -> Vec<Self::Item> where Self: Sized;
    fn chunks(self, size: usize) -> IntoChunks<Self> where Self: Sized;
    fn tuple_windows<T: HomogeneousTuple>(self) -> TupleWindows<Self, T> where Self: Sized + Iterator<Item = <T as HomogeneousTuple>::Item>, <T as HomogeneousTuple>::Item: Clone;
    fn find_position<P: FnMut(&Self::Item) -> bool>(&mut self, pred: P) -> Option<(usize, Self::Item)>;
}

impl<T: ?Sized + Iterator> Itertools for T {}

pub struct ExactlyOneError<I>;
pub struct Unique<I>;
pub struct Dedup<I>;
pub struct IntoChunks<I>;
pub struct TupleWindows<I, T>;

impl<I: Iterator> Debug for ExactlyOneError<I> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<I: Iterator> Display for ExactlyOneError<I> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<I: Iterator> Iterator for Unique<I> where <I as Iterator>::Item: Clone + Eq + Hash {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}
impl<I: Iterator> Iterator for Dedup<I> where <I as Iterator>::Item: PartialEq<<I as Iterator>::Item> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

/// The bound `tuple_windows` places on its output tuple. Every element has the
/// same type, which is what makes the window's item type inferable.
pub trait HomogeneousTuple {
    type Item;
}

impl<T> HomogeneousTuple for (T, T) { type Item = T; }
impl<T> HomogeneousTuple for (T, T, T) { type Item = T; }
impl<T> HomogeneousTuple for (T, T, T, T) { type Item = T; }

/// `exactly_one` hands back an iterator over what it found, so a caller that
/// wanted one and got several can still see them.
impl<I: Iterator> Iterator for ExactlyOneError<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

impl<I: Iterator + Clone, T: HomogeneousTuple> Iterator for TupleWindows<I, T>
where <T as HomogeneousTuple>::Item: Clone {
    type Item = T;
    fn next(&mut self) -> Option<T> { todo!() }
}
