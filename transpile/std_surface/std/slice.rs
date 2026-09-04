//! `std::slice` — the primitive `[T]` and its iterators.
//!
//! `impl<T> [T]` is not writable outside core; syn parses it, and it is the
//! honest place for these methods. A `Vec<T>` receiver reaches them by
//! `Deref<Target = [T]>` plus an autoref, exactly as the oracle records
//! (`Vec<EventId>` -> `[EventId]` -> `&[EventId]`).

impl<T> [T] {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }

    pub fn first(&self) -> Option<&T> { todo!() }
    pub fn first_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn last(&self) -> Option<&T> { todo!() }
    pub fn last_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn split_first(&self) -> Option<(&T, &[T])> { todo!() }
    pub fn split_last(&self) -> Option<(&T, &[T])> { todo!() }
    pub fn split_at(&self, mid: usize) -> (&[T], &[T]) { todo!() }
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) { todo!() }

    pub fn get<I: SliceIndex<[T]>>(&self, index: I) -> Option<&<I as SliceIndex<[T]>>::Output> { todo!() }
    pub fn get_mut<I: SliceIndex<[T]>>(&mut self, index: I) -> Option<&mut <I as SliceIndex<[T]>>::Output> { todo!() }

    pub fn iter(&self) -> std::slice::Iter<'_, T> { todo!() }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> { todo!() }
    pub fn chunks(&self, chunk_size: usize) -> Chunks<'_, T> { todo!() }
    pub fn chunks_exact(&self, chunk_size: usize) -> ChunksExact<'_, T> { todo!() }
    pub fn windows(&self, size: usize) -> Windows<'_, T> { todo!() }
    pub fn split<F: FnMut(&T) -> bool>(&self, pred: F) -> Split<'_, T, F> { todo!() }

    pub fn swap(&mut self, a: usize, b: usize) { todo!() }
    pub fn reverse(&mut self) { todo!() }
    pub fn fill(&mut self, value: T) where T: Clone { todo!() }
    pub fn as_ptr(&self) -> *const T { todo!() }
    pub fn as_mut_ptr(&mut self) -> *mut T { todo!() }

    pub fn sort_by<F: FnMut(&T, &T) -> std::cmp::Ordering>(&mut self, compare: F) { todo!() }
    pub fn sort_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F) { todo!() }
    pub fn sort_unstable_by<F: FnMut(&T, &T) -> std::cmp::Ordering>(&mut self, compare: F) { todo!() }
    pub fn sort_unstable_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F) { todo!() }

    pub fn binary_search_by<F: FnMut(&T) -> std::cmp::Ordering>(&self, f: F) -> Result<usize, usize> { todo!() }
    pub fn binary_search_by_key<B: Ord, F: FnMut(&T) -> B>(&self, b: &B, f: F) -> Result<usize, usize> { todo!() }

    pub fn contains(&self, x: &T) -> bool where T: PartialEq<T> { todo!() }
    pub fn starts_with(&self, needle: &[T]) -> bool where T: PartialEq<T> { todo!() }
    pub fn ends_with(&self, needle: &[T]) -> bool where T: PartialEq<T> { todo!() }
}

impl<T: Ord> [T] {
    pub fn sort(&mut self) { todo!() }
    pub fn sort_unstable(&mut self) { todo!() }
    pub fn binary_search(&self, x: &T) -> Result<usize, usize> { todo!() }
}

impl<T: Clone> [T] {
    pub fn to_vec(&self) -> Vec<T> { todo!() }
}

impl<T> [T] {
    pub fn concat<Item: ?Sized>(&self) -> <[T] as Concat<Item>>::Output where [T]: Concat<Item> { todo!() }
    pub fn join<Separator>(&self, sep: Separator) -> <[T] as Join<Separator>>::Output where [T]: Join<Separator> { todo!() }
}

/// std routes `concat` and `join` through these two traits rather than through
/// inherent methods, and the `Output` is where the element relationship lives:
/// a `[String]` joined by `&str` gives a `String`, a `[Vec<T>]` joined by `&T`
/// gives a `Vec<T>`, and a flat `[u8]` has neither impl, so `[u8]::concat()`
/// correctly fails to resolve.
pub trait Concat<Item: ?Sized> {
    type Output;
    fn concat(slice: &Self) -> Self::Output;
}

pub trait Join<Separator> {
    type Output;
    fn join(slice: &Self, sep: Separator) -> Self::Output;
}

impl<T: Clone, V: Borrow<[T]>> Concat<T> for [V] {
    type Output = Vec<T>;
    fn concat(slice: &[V]) -> Vec<T> { todo!() }
}

impl<T: Clone, V: Borrow<[T]>> Join<&T> for [V] {
    type Output = Vec<T>;
    fn join(slice: &[V], sep: &T) -> Vec<T> { todo!() }
}

impl<T: Clone, V: Borrow<[T]>> Join<&[T]> for [V] {
    type Output = Vec<T>;
    fn join(slice: &[V], sep: &[T]) -> Vec<T> { todo!() }
}

impl<S: Borrow<str>> Concat<str> for [S] {
    type Output = String;
    fn concat(slice: &[S]) -> String { todo!() }
}

impl<S: Borrow<str>> Join<&str> for [S] {
    type Output = String;
    fn join(slice: &[S], sep: &str) -> String { todo!() }
}

impl<T: Copy> [T] {
    pub fn copy_from_slice(&mut self, src: &[T]) { todo!() }
}

/// `unsafe` in std, because an implementor promises its indices are in bounds.
/// The engine does not track trait unsafety today; the keyword is written
/// because it is part of the declaration.
pub unsafe trait SliceIndex<T: ?Sized> {
    type Output: ?Sized;
    fn get(self, slice: &T) -> Option<&Self::Output>;
    fn get_mut(self, slice: &mut T) -> Option<&mut Self::Output>;
    fn index(self, slice: &T) -> &Self::Output;
    fn index_mut(self, slice: &mut T) -> &mut Self::Output;
}

unsafe impl<T> SliceIndex<[T]> for usize {
    type Output = T;
    fn get(self, slice: &[T]) -> Option<&T> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut T> { todo!() }
    fn index(self, slice: &[T]) -> &T { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut T { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for std::ops::Range<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for RangeFrom<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for RangeTo<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for RangeFull {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for RangeInclusive<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl<T> SliceIndex<[T]> for RangeToInclusive<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> { todo!() }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> { todo!() }
    fn index(self, slice: &[T]) -> &[T] { todo!() }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] { todo!() }
}
unsafe impl SliceIndex<str> for std::ops::Range<usize> {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}
unsafe impl SliceIndex<str> for RangeFrom<usize> {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}
unsafe impl SliceIndex<str> for RangeTo<usize> {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}
unsafe impl SliceIndex<str> for RangeFull {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}
unsafe impl SliceIndex<str> for RangeInclusive<usize> {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}
unsafe impl SliceIndex<str> for RangeToInclusive<usize> {
    type Output = str;
    fn get(self, slice: &str) -> Option<&str> { todo!() }
    fn get_mut(self, slice: &mut str) -> Option<&mut str> { todo!() }
    fn index(self, slice: &str) -> &str { todo!() }
    fn index_mut(self, slice: &mut str) -> &mut str { todo!() }
}

pub struct Iter<'a, T>;
pub struct IterMut<'a, T>;
pub struct Chunks<'a, T>;
pub struct ChunksExact<'a, T>;
pub struct Windows<'a, T>;
pub struct Split<'a, T, P>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> DoubleEndedIterator for Iter<'a, T> { fn next_back(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> ExactSizeIterator for Iter<'a, T> { fn len(&self) -> usize { todo!() } }
impl<'a, T> Clone for Iter<'a, T> { fn clone(&self) -> Iter<'a, T> { todo!() } }

impl<'a, T> Iterator for IterMut<'a, T> { type Item = &'a mut T; fn next(&mut self) -> Option<&'a mut T> { todo!() } }
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> { fn next_back(&mut self) -> Option<&'a mut T> { todo!() } }
impl<'a, T> ExactSizeIterator for IterMut<'a, T> { fn len(&self) -> usize { todo!() } }

impl<'a, T> Iterator for Chunks<'a, T> { type Item = &'a [T]; fn next(&mut self) -> Option<&'a [T]> { todo!() } }
impl<'a, T> Iterator for ChunksExact<'a, T> { type Item = &'a [T]; fn next(&mut self) -> Option<&'a [T]> { todo!() } }
impl<'a, T> Iterator for Windows<'a, T> { type Item = &'a [T]; fn next(&mut self) -> Option<&'a [T]> { todo!() } }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for Split<'a, T, P> { type Item = &'a [T]; fn next(&mut self) -> Option<&'a [T]> { todo!() } }

impl<'a, T> IntoIterator for &'a [T] {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> { todo!() }
}

impl<'a, T> IntoIterator for &'a mut [T] {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> std::slice::IterMut<'a, T> { todo!() }
}

impl<T: Debug> Debug for [T] { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: PartialEq<T>> PartialEq for [T] { fn eq(&self, other: &[T]) -> bool { todo!() } }
impl<T: Eq> Eq for [T] {}
impl<T: Ord> Ord for [T] { fn cmp(&self, other: &[T]) -> std::cmp::Ordering { todo!() } }

pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] { todo!() }
pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] { todo!() }

// Arrays: `[T; N]` is not `[T]`, and `for x in [a, b]` reaches this impl.
impl<T, const N: usize> IntoIterator for [T; N] {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, N>;
    fn into_iter(self) -> std::array::IntoIter<T, N> { todo!() }
}

impl<'a, T, const N: usize> IntoIterator for &'a [T; N] {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> { todo!() }
}

impl<T, const N: usize> [T; N] {
    pub fn as_slice(&self) -> &[T] { todo!() }
    pub fn map<F: FnMut(T) -> U, U>(self, f: F) -> [U; N] { todo!() }
}

impl<T, const N: usize> AsRef<[T]> for [T; N] {
    fn as_ref(&self) -> &[T] { todo!() }
}

/// `[T; N]` reaches slice methods by an unsize coercion, not by `Deref` — the
/// oracle keeps the two apart (`Pointer(Unsize)` versus
/// `Deref(Some(OverloadedDeref(..)))`), so declaring a `Deref` here would teach
/// the engine a relation rustc does not have. `Unsize` is unstable in real std;
/// it is written out because the engine's deref chain needs the fact.
pub trait Unsize<T: ?Sized> {}

impl<T, const N: usize> Unsize<[T]> for [T; N] {}
impl<T: Debug> Unsize<dyn Debug> for T {}
