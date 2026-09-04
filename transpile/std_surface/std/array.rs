//! `std::array`
//!
//! `[T; N]: IntoIterator` in `std/slice.rs` returns this, so `for b in
//! id.to_be_bytes()` has an iterator type. Declared here rather than in
//! `slice.rs` because std puts it in its own module and the file path is the
//! module path.
//!
//! Simplification: std writes `IntoIter<T, const N: usize>`; the `N` is dropped
//! here. It carries nothing the engine reads — the `Item` is `T` at every
//! length — and it was the second place a const generic flowed from an impl's
//! parameter list into a struct argument, which the loader cannot represent.
//! Same reasoning as dropping `Vec`'s allocator parameter. `[T; N]` as a *self
//! type* is untouched; only the argument to this struct is.

pub struct IntoIter<T>;

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { todo!() }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> { todo!() }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize { todo!() }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> IntoIter<T> { todo!() }
}

pub struct TryFromSliceError;

impl Debug for TryFromSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for TryFromSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for TryFromSliceError { fn clone(&self) -> TryFromSliceError { todo!() } }
impl std::error::Error for TryFromSliceError {}

impl<'a, T, const N: usize> TryFrom<&'a [T]> for [T; N] where T: Copy {
    type Error = TryFromSliceError;
    fn try_from(slice: &'a [T]) -> Result<[T; N], TryFromSliceError> { todo!() }
}
