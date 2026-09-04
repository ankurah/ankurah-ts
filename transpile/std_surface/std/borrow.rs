//! `std::borrow`
//!
//! `Borrow` is here because the map and set lookup methods are declared with
//! their real `K: Borrow<Q>` signature. Resolving `map.get(&x)` means unifying
//! `&Q` with the argument, then discharging `K: Borrow<Q>` against the impls
//! below — the reflexive blanket covers the ordinary case, and the `String`/
//! `Vec` impls cover `map.get("key")` on a `HashMap<String, _>`.

pub trait Borrow<Borrowed: ?Sized> {
    fn borrow(&self) -> &Borrowed;
}

pub trait BorrowMut<Borrowed: ?Sized>: Borrow<Borrowed> {
    fn borrow_mut(&mut self) -> &mut Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    fn borrow(&self) -> &T { todo!() }
}

impl<T: ?Sized> BorrowMut<T> for T {
    fn borrow_mut(&mut self) -> &mut T { todo!() }
}

impl<T: ?Sized> Borrow<T> for &T {
    fn borrow(&self) -> &T { todo!() }
}

impl<T: ?Sized> Borrow<T> for &mut T {
    fn borrow(&self) -> &T { todo!() }
}

impl<T: ?Sized> BorrowMut<T> for &mut T {
    fn borrow_mut(&mut self) -> &mut T { todo!() }
}

impl BorrowMut<str> for String {
    fn borrow_mut(&mut self) -> &mut str { todo!() }
}

impl<T> BorrowMut<[T]> for Vec<T> {
    fn borrow_mut(&mut self) -> &mut [T] { todo!() }
}

impl<T: ?Sized> BorrowMut<T> for Box<T> {
    fn borrow_mut(&mut self) -> &mut T { todo!() }
}

impl Borrow<str> for String {
    fn borrow(&self) -> &str { todo!() }
}

impl<T> Borrow<[T]> for Vec<T> {
    fn borrow(&self) -> &[T] { todo!() }
}

impl<T: ?Sized> Borrow<T> for Box<T> {
    fn borrow(&self) -> &T { todo!() }
}

impl<T: ?Sized> Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T { todo!() }
}

impl<T: ?Sized> Borrow<T> for Rc<T> {
    fn borrow(&self) -> &T { todo!() }
}

pub trait ToOwned {
    type Owned: Borrow<Self>;
    fn to_owned(&self) -> Self::Owned;
    fn clone_into(&self, target: &mut Self::Owned);
}

impl ToOwned for str {
    type Owned = String;
    fn to_owned(&self) -> String { todo!() }
    fn clone_into(&self, target: &mut String) { todo!() }
}

impl<T: Clone> ToOwned for [T] {
    type Owned = Vec<T>;
    fn to_owned(&self) -> Vec<T> { todo!() }
    fn clone_into(&self, target: &mut Vec<T>) { todo!() }
}

impl<T: Clone> ToOwned for T {
    type Owned = T;
    fn to_owned(&self) -> T { todo!() }
    fn clone_into(&self, target: &mut T) { todo!() }
}

pub enum Cow<'a, B: ?Sized + ToOwned> {
    Borrowed(&'a B),
    Owned(<B as ToOwned>::Owned),
}

impl<'a, B: ?Sized + ToOwned> Cow<'a, B> {
    pub fn into_owned(self) -> <B as ToOwned>::Owned { todo!() }
    pub fn to_mut(&mut self) -> &mut <B as ToOwned>::Owned { todo!() }
}

impl<'a, B: ?Sized + ToOwned> Deref for Cow<'a, B> {
    type Target = B;
    fn deref(&self) -> &B { todo!() }
}

impl<'a, B: ?Sized + ToOwned> Clone for Cow<'a, B> { fn clone(&self) -> Cow<'a, B> { todo!() } }
impl<'a> From<&'a str> for Cow<'a, str> { fn from(value: &'a str) -> Cow<'a, str> { todo!() } }
impl<'a> From<String> for Cow<'a, str> { fn from(value: String) -> Cow<'a, str> { todo!() } }
