//! `std::cmp`

pub trait PartialEq<Rhs: ?Sized = Self> {
    fn eq(&self, other: &Rhs) -> bool;
    fn ne(&self, other: &Rhs) -> bool;
}

pub trait Eq: PartialEq<Self> {}

pub trait PartialOrd<Rhs: ?Sized = Self>: PartialEq<Rhs> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;
    fn lt(&self, other: &Rhs) -> bool;
    fn le(&self, other: &Rhs) -> bool;
    fn gt(&self, other: &Rhs) -> bool;
    fn ge(&self, other: &Rhs) -> bool;
}

pub trait Ord: Eq + PartialOrd<Self> {
    fn cmp(&self, other: &Self) -> Ordering;
    fn max(self, other: Self) -> Self where Self: Sized;
    fn min(self, other: Self) -> Self where Self: Sized;
    fn clamp(self, min: Self, max: Self) -> Self where Self: Sized;
}

pub enum Ordering {
    Less,
    Equal,
    Greater,
}

impl Ordering {
    pub fn is_eq(self) -> bool { todo!() }
    pub fn is_ne(self) -> bool { todo!() }
    pub fn is_lt(self) -> bool { todo!() }
    pub fn is_gt(self) -> bool { todo!() }
    pub fn is_le(self) -> bool { todo!() }
    pub fn is_ge(self) -> bool { todo!() }
    pub fn reverse(self) -> Ordering { todo!() }
    pub fn then(self, other: Ordering) -> Ordering { todo!() }
    pub fn then_with<F: FnOnce() -> Ordering>(self, f: F) -> Ordering { todo!() }
}

impl Clone for Ordering { fn clone(&self) -> Ordering { todo!() } }
impl Copy for Ordering {}
impl PartialEq for Ordering { fn eq(&self, other: &Ordering) -> bool { todo!() } }
impl Eq for Ordering {}
impl PartialOrd for Ordering { fn partial_cmp(&self, other: &Ordering) -> Option<Ordering> { todo!() } }
impl Ord for Ordering { fn cmp(&self, other: &Ordering) -> Ordering { todo!() } }
impl Debug for Ordering { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct Reverse<T>(pub T);

impl<T: PartialEq> PartialEq for Reverse<T> { fn eq(&self, other: &Reverse<T>) -> bool { todo!() } }
impl<T: Eq> Eq for Reverse<T> {}
impl<T: PartialOrd> PartialOrd for Reverse<T> { fn partial_cmp(&self, other: &Reverse<T>) -> Option<Ordering> { todo!() } }
impl<T: Ord> Ord for Reverse<T> { fn cmp(&self, other: &Reverse<T>) -> Ordering { todo!() } }
impl<T: Clone> Clone for Reverse<T> { fn clone(&self) -> Reverse<T> { todo!() } }

pub fn min<T: Ord>(v1: T, v2: T) -> T { todo!() }
pub fn max<T: Ord>(v1: T, v2: T) -> T { todo!() }
pub fn min_by_key<T, K: Ord, F: FnMut(&T) -> K>(v1: T, v2: T, f: F) -> T { todo!() }
pub fn max_by_key<T, K: Ord, F: FnMut(&T) -> K>(v1: T, v2: T, f: F) -> T { todo!() }

// Comparison through a reference: `(&a).cmp(&b)` and every `sort_by(|a, b| ...)`
// closure whose parameters are references depends on these.
impl<A: ?Sized + PartialEq<B>, B: ?Sized> PartialEq<&B> for &A {
    fn eq(&self, other: &&B) -> bool { todo!() }
    fn ne(&self, other: &&B) -> bool { todo!() }
}
impl<A: ?Sized + Eq> Eq for &A {}
impl<A: ?Sized + PartialOrd<B>, B: ?Sized> PartialOrd<&B> for &A {
    fn partial_cmp(&self, other: &&B) -> Option<Ordering> { todo!() }
    fn lt(&self, other: &&B) -> bool { todo!() }
    fn le(&self, other: &&B) -> bool { todo!() }
    fn gt(&self, other: &&B) -> bool { todo!() }
    fn ge(&self, other: &&B) -> bool { todo!() }
}
impl<A: ?Sized + Ord> Ord for &A {
    fn cmp(&self, other: &&A) -> Ordering { todo!() }
}
