//! `std::iter` — the traits.
//!
//! Every method is declared bodiless. Real std splits `Iterator` into one
//! required method (`next`) and ~70 provided ones; this file does not record
//! that split, because the engine resolves calls and never checks an impl for
//! completeness. An `impl Iterator` in the corpus that supplies only `next` is
//! therefore not a hole the engine will complain about.

pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
    fn size_hint(&self) -> (usize, Option<usize>);
    fn count(self) -> usize where Self: Sized;
    fn last(self) -> Option<Self::Item> where Self: Sized;
    fn nth(&mut self, n: usize) -> Option<Self::Item>;

    fn step_by(self, step: usize) -> StepBy<Self> where Self: Sized;
    fn chain<U: IntoIterator<Item = Self::Item>>(self, other: U) -> Chain<Self, <U as IntoIterator>::IntoIter> where Self: Sized;
    fn zip<U: IntoIterator>(self, other: U) -> Zip<Self, <U as IntoIterator>::IntoIter> where Self: Sized;
    fn map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Map<Self, F> where Self: Sized;
    fn for_each<F: FnMut(Self::Item)>(self, f: F) where Self: Sized;
    fn filter<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> Filter<Self, P> where Self: Sized;
    fn filter_map<B, F: FnMut(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F> where Self: Sized;
    fn enumerate(self) -> Enumerate<Self> where Self: Sized;
    fn peekable(self) -> Peekable<Self> where Self: Sized;
    fn skip_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> SkipWhile<Self, P> where Self: Sized;
    fn take_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> TakeWhile<Self, P> where Self: Sized;
    fn map_while<B, P: FnMut(Self::Item) -> Option<B>>(self, predicate: P) -> MapWhile<Self, P> where Self: Sized;
    fn skip(self, n: usize) -> Skip<Self> where Self: Sized;
    fn take(self, n: usize) -> Take<Self> where Self: Sized;
    fn scan<St, B, F: FnMut(&mut St, Self::Item) -> Option<B>>(self, initial_state: St, f: F) -> Scan<Self, St, F> where Self: Sized;
    fn flat_map<U: IntoIterator, F: FnMut(Self::Item) -> U>(self, f: F) -> FlatMap<Self, U, F> where Self: Sized;
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Item: IntoIterator;
    fn fuse(self) -> Fuse<Self> where Self: Sized;
    fn inspect<F: FnMut(&Self::Item)>(self, f: F) -> Inspect<Self, F> where Self: Sized;
    fn by_ref(&mut self) -> &mut Self where Self: Sized;

    fn collect<B: FromIterator<Self::Item>>(self) -> B where Self: Sized;
    fn partition<B: Default + Extend<Self::Item>, F: FnMut(&Self::Item) -> bool>(self, f: F) -> (B, B) where Self: Sized;
    fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B where Self: Sized;
    /// Simplified. Real std is `fn try_fold<B, F, R>(&mut self, init: B, f: F)
    /// -> R where R: Try<Output = B>`, so the fold can short-circuit on an
    /// `Option` as well as a `Result`. `Try` is unstable and the corpus only
    /// ever folds into a `Result`; a corpus that folds into an `Option` will
    /// fail to resolve here rather than resolve wrongly.
    fn try_fold<B, F: FnMut(B, Self::Item) -> Result<B, E>, E>(&mut self, init: B, f: F) -> Result<B, E> where Self: Sized;
    fn reduce<F: FnMut(Self::Item, Self::Item) -> Self::Item>(self, f: F) -> Option<Self::Item> where Self: Sized;
    fn all<F: FnMut(Self::Item) -> bool>(&mut self, f: F) -> bool;
    fn any<F: FnMut(Self::Item) -> bool>(&mut self, f: F) -> bool;
    fn find<P: FnMut(&Self::Item) -> bool>(&mut self, predicate: P) -> Option<Self::Item>;
    fn find_map<B, F: FnMut(Self::Item) -> Option<B>>(&mut self, f: F) -> Option<B>;
    fn position<P: FnMut(Self::Item) -> bool>(&mut self, predicate: P) -> Option<usize>;

    fn max(self) -> Option<Self::Item> where Self: Sized, Self::Item: Ord;
    fn min(self) -> Option<Self::Item> where Self: Sized, Self::Item: Ord;
    fn max_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(self, f: F) -> Option<Self::Item> where Self: Sized;
    fn max_by<F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering>(self, compare: F) -> Option<Self::Item> where Self: Sized;
    fn min_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(self, f: F) -> Option<Self::Item> where Self: Sized;
    fn min_by<F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering>(self, compare: F) -> Option<Self::Item> where Self: Sized;

    fn rev(self) -> Rev<Self> where Self: Sized + DoubleEndedIterator;
    fn unzip<A, B, FromA: Default + Extend<A>, FromB: Default + Extend<B>>(self) -> (FromA, FromB) where Self: Sized + Iterator<Item = (A, B)>;
    fn copied<'a, T: 'a + Copy>(self) -> Copied<Self> where Self: Sized + Iterator<Item = &'a T>;
    fn cloned<'a, T: 'a + Clone>(self) -> Cloned<Self> where Self: Sized + Iterator<Item = &'a T>;
    fn cycle(self) -> Cycle<Self> where Self: Sized + Clone;
    fn sum<S: Sum<Self::Item>>(self) -> S where Self: Sized;
    fn product<P: Product<Self::Item>>(self) -> P where Self: Sized;

    fn cmp<I: IntoIterator<Item = Self::Item>>(self, other: I) -> std::cmp::Ordering where Self::Item: Ord, Self: Sized;
    fn partial_cmp<I: IntoIterator>(self, other: I) -> Option<std::cmp::Ordering> where Self::Item: PartialOrd<<I as IntoIterator>::Item>, Self: Sized;
    fn eq<I: IntoIterator>(self, other: I) -> bool where Self::Item: PartialEq<<I as IntoIterator>::Item>, Self: Sized;
    fn ne<I: IntoIterator>(self, other: I) -> bool where Self::Item: PartialEq<<I as IntoIterator>::Item>, Self: Sized;
}

// `for x in it` and every `.into_iter()` on something already an iterator goes
// through this. Without it, `Map<..>` has no `IntoIterator::Item` to project.
impl<I: Iterator> IntoIterator for I {
    type Item = <I as Iterator>::Item;
    type IntoIter = I;
    fn into_iter(self) -> I { todo!() }
}

// `&mut it` is an iterator; `by_ref()` returns one.
impl<I: Iterator + ?Sized> Iterator for &mut I {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

impl<I: Iterator + ?Sized> Iterator for Box<I> {
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> { todo!() }
}

pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}

pub trait FromIterator<A>: Sized {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}

pub trait Extend<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T);
}

pub trait DoubleEndedIterator: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;
    fn nth_back(&mut self, n: usize) -> Option<Self::Item>;
    fn rfind<P: FnMut(&Self::Item) -> bool>(&mut self, predicate: P) -> Option<Self::Item>;
    fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B where Self: Sized;
}

pub trait ExactSizeIterator: Iterator {
    fn len(&self) -> usize;
}

pub trait FusedIterator: Iterator {}

pub trait Sum<A = Self>: Sized {
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self;
}

pub trait Product<A = Self>: Sized {
    fn product<I: Iterator<Item = A>>(iter: I) -> Self;
}

// std generates these by macro for every integer and float, in both the
// owned and the borrowed form. `.sum()` and `.product()` are unresolvable
// for any type not listed, so the list is the full one, not a sample.

impl Sum<i8> for i8 { fn sum<I: Iterator<Item = i8>>(iter: I) -> i8 { todo!() } }
impl Product<i8> for i8 { fn product<I: Iterator<Item = i8>>(iter: I) -> i8 { todo!() } }
impl<'a> Sum<&'a i8> for i8 { fn sum<I: Iterator<Item = &'a i8>>(iter: I) -> i8 { todo!() } }
impl<'a> Product<&'a i8> for i8 { fn product<I: Iterator<Item = &'a i8>>(iter: I) -> i8 { todo!() } }
impl Sum<i16> for i16 { fn sum<I: Iterator<Item = i16>>(iter: I) -> i16 { todo!() } }
impl Product<i16> for i16 { fn product<I: Iterator<Item = i16>>(iter: I) -> i16 { todo!() } }
impl<'a> Sum<&'a i16> for i16 { fn sum<I: Iterator<Item = &'a i16>>(iter: I) -> i16 { todo!() } }
impl<'a> Product<&'a i16> for i16 { fn product<I: Iterator<Item = &'a i16>>(iter: I) -> i16 { todo!() } }
impl Sum<i32> for i32 { fn sum<I: Iterator<Item = i32>>(iter: I) -> i32 { todo!() } }
impl Product<i32> for i32 { fn product<I: Iterator<Item = i32>>(iter: I) -> i32 { todo!() } }
impl<'a> Sum<&'a i32> for i32 { fn sum<I: Iterator<Item = &'a i32>>(iter: I) -> i32 { todo!() } }
impl<'a> Product<&'a i32> for i32 { fn product<I: Iterator<Item = &'a i32>>(iter: I) -> i32 { todo!() } }
impl Sum<i64> for i64 { fn sum<I: Iterator<Item = i64>>(iter: I) -> i64 { todo!() } }
impl Product<i64> for i64 { fn product<I: Iterator<Item = i64>>(iter: I) -> i64 { todo!() } }
impl<'a> Sum<&'a i64> for i64 { fn sum<I: Iterator<Item = &'a i64>>(iter: I) -> i64 { todo!() } }
impl<'a> Product<&'a i64> for i64 { fn product<I: Iterator<Item = &'a i64>>(iter: I) -> i64 { todo!() } }
impl Sum<i128> for i128 { fn sum<I: Iterator<Item = i128>>(iter: I) -> i128 { todo!() } }
impl Product<i128> for i128 { fn product<I: Iterator<Item = i128>>(iter: I) -> i128 { todo!() } }
impl<'a> Sum<&'a i128> for i128 { fn sum<I: Iterator<Item = &'a i128>>(iter: I) -> i128 { todo!() } }
impl<'a> Product<&'a i128> for i128 { fn product<I: Iterator<Item = &'a i128>>(iter: I) -> i128 { todo!() } }
impl Sum<isize> for isize { fn sum<I: Iterator<Item = isize>>(iter: I) -> isize { todo!() } }
impl Product<isize> for isize { fn product<I: Iterator<Item = isize>>(iter: I) -> isize { todo!() } }
impl<'a> Sum<&'a isize> for isize { fn sum<I: Iterator<Item = &'a isize>>(iter: I) -> isize { todo!() } }
impl<'a> Product<&'a isize> for isize { fn product<I: Iterator<Item = &'a isize>>(iter: I) -> isize { todo!() } }
impl Sum<u8> for u8 { fn sum<I: Iterator<Item = u8>>(iter: I) -> u8 { todo!() } }
impl Product<u8> for u8 { fn product<I: Iterator<Item = u8>>(iter: I) -> u8 { todo!() } }
impl<'a> Sum<&'a u8> for u8 { fn sum<I: Iterator<Item = &'a u8>>(iter: I) -> u8 { todo!() } }
impl<'a> Product<&'a u8> for u8 { fn product<I: Iterator<Item = &'a u8>>(iter: I) -> u8 { todo!() } }
impl Sum<u16> for u16 { fn sum<I: Iterator<Item = u16>>(iter: I) -> u16 { todo!() } }
impl Product<u16> for u16 { fn product<I: Iterator<Item = u16>>(iter: I) -> u16 { todo!() } }
impl<'a> Sum<&'a u16> for u16 { fn sum<I: Iterator<Item = &'a u16>>(iter: I) -> u16 { todo!() } }
impl<'a> Product<&'a u16> for u16 { fn product<I: Iterator<Item = &'a u16>>(iter: I) -> u16 { todo!() } }
impl Sum<u32> for u32 { fn sum<I: Iterator<Item = u32>>(iter: I) -> u32 { todo!() } }
impl Product<u32> for u32 { fn product<I: Iterator<Item = u32>>(iter: I) -> u32 { todo!() } }
impl<'a> Sum<&'a u32> for u32 { fn sum<I: Iterator<Item = &'a u32>>(iter: I) -> u32 { todo!() } }
impl<'a> Product<&'a u32> for u32 { fn product<I: Iterator<Item = &'a u32>>(iter: I) -> u32 { todo!() } }
impl Sum<u64> for u64 { fn sum<I: Iterator<Item = u64>>(iter: I) -> u64 { todo!() } }
impl Product<u64> for u64 { fn product<I: Iterator<Item = u64>>(iter: I) -> u64 { todo!() } }
impl<'a> Sum<&'a u64> for u64 { fn sum<I: Iterator<Item = &'a u64>>(iter: I) -> u64 { todo!() } }
impl<'a> Product<&'a u64> for u64 { fn product<I: Iterator<Item = &'a u64>>(iter: I) -> u64 { todo!() } }
impl Sum<u128> for u128 { fn sum<I: Iterator<Item = u128>>(iter: I) -> u128 { todo!() } }
impl Product<u128> for u128 { fn product<I: Iterator<Item = u128>>(iter: I) -> u128 { todo!() } }
impl<'a> Sum<&'a u128> for u128 { fn sum<I: Iterator<Item = &'a u128>>(iter: I) -> u128 { todo!() } }
impl<'a> Product<&'a u128> for u128 { fn product<I: Iterator<Item = &'a u128>>(iter: I) -> u128 { todo!() } }
impl Sum<usize> for usize { fn sum<I: Iterator<Item = usize>>(iter: I) -> usize { todo!() } }
impl Product<usize> for usize { fn product<I: Iterator<Item = usize>>(iter: I) -> usize { todo!() } }
impl<'a> Sum<&'a usize> for usize { fn sum<I: Iterator<Item = &'a usize>>(iter: I) -> usize { todo!() } }
impl<'a> Product<&'a usize> for usize { fn product<I: Iterator<Item = &'a usize>>(iter: I) -> usize { todo!() } }
impl Sum<f32> for f32 { fn sum<I: Iterator<Item = f32>>(iter: I) -> f32 { todo!() } }
impl Product<f32> for f32 { fn product<I: Iterator<Item = f32>>(iter: I) -> f32 { todo!() } }
impl<'a> Sum<&'a f32> for f32 { fn sum<I: Iterator<Item = &'a f32>>(iter: I) -> f32 { todo!() } }
impl<'a> Product<&'a f32> for f32 { fn product<I: Iterator<Item = &'a f32>>(iter: I) -> f32 { todo!() } }
impl Sum<f64> for f64 { fn sum<I: Iterator<Item = f64>>(iter: I) -> f64 { todo!() } }
impl Product<f64> for f64 { fn product<I: Iterator<Item = f64>>(iter: I) -> f64 { todo!() } }
impl<'a> Sum<&'a f64> for f64 { fn sum<I: Iterator<Item = &'a f64>>(iter: I) -> f64 { todo!() } }
impl<'a> Product<&'a f64> for f64 { fn product<I: Iterator<Item = &'a f64>>(iter: I) -> f64 { todo!() } }

// `collect()` into a `String` from an iterator of `String`/`&str` is the
// FromIterator path; summing durations is the one non-numeric `Sum` the
// corpus could plausibly reach.
impl Sum<Duration> for Duration { fn sum<I: Iterator<Item = Duration>>(iter: I) -> Duration { todo!() } }
impl<'a> Sum<&'a Duration> for Duration { fn sum<I: Iterator<Item = &'a Duration>>(iter: I) -> Duration { todo!() } }
