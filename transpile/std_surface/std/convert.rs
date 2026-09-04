//! `std::convert`
//!
//! The blanket impls at the bottom are the ones the corpus leans on hardest:
//! every `.into()` with an expected type, and every `?` that changes the error
//! type, selects one of them.

pub trait From<T>: Sized {
    fn from(value: T) -> Self;
}

pub trait Into<T>: Sized {
    fn into(self) -> T;
}

pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait TryInto<T>: Sized {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

pub trait AsRef<T: ?Sized> {
    fn as_ref(&self) -> &T;
}

pub trait AsMut<T: ?Sized> {
    fn as_mut(&mut self) -> &mut T;
}

pub enum Infallible {}

impl Debug for Infallible { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for Infallible { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for Infallible { fn clone(&self) -> Infallible { todo!() } }
impl PartialEq for Infallible { fn eq(&self, other: &Infallible) -> bool { todo!() } }
impl Eq for Infallible {}
impl std::error::Error for Infallible {}

pub fn identity<T>(x: T) -> T { todo!() }

// ── Blanket impls ────────────────────────────────────────────────────────────

impl<T> From<T> for T {
    fn from(value: T) -> T { todo!() }
}

impl<T, U> Into<U> for T where U: From<T> {
    fn into(self) -> U { todo!() }
}

impl<T, U> TryFrom<U> for T where U: Into<T> {
    type Error = Infallible;
    fn try_from(value: U) -> Result<T, Infallible> { todo!() }
}

impl<T, U> TryInto<U> for T where U: TryFrom<T> {
    type Error = <U as TryFrom<T>>::Error;
    fn try_into(self) -> Result<U, <U as TryFrom<T>>::Error> { todo!() }
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &T where T: AsRef<U> {
    fn as_ref(&self) -> &U { todo!() }
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &mut T where T: AsRef<U> {
    fn as_ref(&self) -> &U { todo!() }
}

impl<T: ?Sized, U: ?Sized> AsMut<U> for &mut T where T: AsMut<U> {
    fn as_mut(&mut self) -> &mut U { todo!() }
}

// ── Concrete impls the corpus reaches ────────────────────────────────────────

impl From<&str> for String { fn from(value: &str) -> String { todo!() } }
impl From<char> for String { fn from(value: char) -> String { todo!() } }
impl From<&String> for String { fn from(value: &String) -> String { todo!() } }
impl From<Box<str>> for String { fn from(value: Box<str>) -> String { todo!() } }
impl From<String> for Vec<u8> { fn from(value: String) -> Vec<u8> { todo!() } }
impl<T: Clone> From<&[T]> for Vec<T> { fn from(value: &[T]) -> Vec<T> { todo!() } }
impl<T> From<Vec<T>> for VecDeque<T> { fn from(value: Vec<T>) -> VecDeque<T> { todo!() } }
impl<T, const N: usize> From<[T; N]> for Vec<T> { fn from(value: [T; N]) -> Vec<T> { todo!() } }
impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for HashMap<K, V> { fn from(value: [(K, V); N]) -> HashMap<K, V> { todo!() } }
impl<K: Ord, V, const N: usize> From<[(K, V); N]> for BTreeMap<K, V> { fn from(value: [(K, V); N]) -> BTreeMap<K, V> { todo!() } }
impl<T: Eq + Hash, const N: usize> From<[T; N]> for HashSet<T> { fn from(value: [T; N]) -> HashSet<T> { todo!() } }
impl From<u8> for u16 { fn from(value: u8) -> u16 { todo!() } }
impl From<u8> for u32 { fn from(value: u8) -> u32 { todo!() } }
impl From<u8> for u64 { fn from(value: u8) -> u64 { todo!() } }
impl From<u8> for usize { fn from(value: u8) -> usize { todo!() } }
impl From<u16> for u32 { fn from(value: u16) -> u32 { todo!() } }
impl From<u16> for u64 { fn from(value: u16) -> u64 { todo!() } }
impl From<u32> for u64 { fn from(value: u32) -> u64 { todo!() } }
impl From<u32> for i64 { fn from(value: u32) -> i64 { todo!() } }
impl From<u32> for f64 { fn from(value: u32) -> f64 { todo!() } }
impl From<i16> for i32 { fn from(value: i16) -> i32 { todo!() } }
impl From<i16> for i64 { fn from(value: i16) -> i64 { todo!() } }
impl From<i32> for i64 { fn from(value: i32) -> i64 { todo!() } }
impl From<i32> for f64 { fn from(value: i32) -> f64 { todo!() } }
impl From<f32> for f64 { fn from(value: f32) -> f64 { todo!() } }
impl From<bool> for i32 { fn from(value: bool) -> i32 { todo!() } }
impl From<bool> for i64 { fn from(value: bool) -> i64 { todo!() } }
impl TryFrom<u64> for u32 { type Error = std::num::TryFromIntError; fn try_from(value: u64) -> Result<u32, std::num::TryFromIntError> { todo!() } }
impl TryFrom<u64> for usize { type Error = std::num::TryFromIntError; fn try_from(value: u64) -> Result<usize, std::num::TryFromIntError> { todo!() } }
impl TryFrom<usize> for u32 { type Error = std::num::TryFromIntError; fn try_from(value: usize) -> Result<u32, std::num::TryFromIntError> { todo!() } }
impl TryFrom<i64> for i32 { type Error = std::num::TryFromIntError; fn try_from(value: i64) -> Result<i32, std::num::TryFromIntError> { todo!() } }
impl TryFrom<i64> for u64 { type Error = std::num::TryFromIntError; fn try_from(value: i64) -> Result<u64, std::num::TryFromIntError> { todo!() } }
impl TryFrom<i64> for usize { type Error = std::num::TryFromIntError; fn try_from(value: i64) -> Result<usize, std::num::TryFromIntError> { todo!() } }
impl TryFrom<usize> for i64 { type Error = std::num::TryFromIntError; fn try_from(value: usize) -> Result<i64, std::num::TryFromIntError> { todo!() } }

impl AsRef<str> for str { fn as_ref(&self) -> &str { todo!() } }
impl AsRef<[u8]> for str { fn as_ref(&self) -> &[u8] { todo!() } }
impl AsRef<str> for String { fn as_ref(&self) -> &str { todo!() } }
impl AsRef<[u8]> for String { fn as_ref(&self) -> &[u8] { todo!() } }
impl<T> AsRef<[T]> for [T] { fn as_ref(&self) -> &[T] { todo!() } }
impl<T> AsRef<[T]> for Vec<T> { fn as_ref(&self) -> &[T] { todo!() } }
impl<T> AsMut<[T]> for Vec<T> { fn as_mut(&mut self) -> &mut [T] { todo!() } }
impl<T: ?Sized> AsRef<T> for Box<T> { fn as_ref(&self) -> &T { todo!() } }
impl<T: ?Sized> AsMut<T> for Box<T> { fn as_mut(&mut self) -> &mut T { todo!() } }
impl<T: ?Sized> AsRef<T> for Arc<T> { fn as_ref(&self) -> &T { todo!() } }
impl<T: ?Sized> AsRef<T> for Rc<T> { fn as_ref(&self) -> &T { todo!() } }
