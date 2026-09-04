//! `std::array`
//!
//! `[T; N]: IntoIterator` in `std/slice.rs` returns this, so `for b in
//! id.to_be_bytes()` has an iterator type. Declared here rather than in
//! `slice.rs` because std puts it in its own module and the file path is the
//! module path.

pub struct IntoIter<T, const N: usize>;

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<T> { todo!() }
}

impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> { todo!() }
}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {
    fn len(&self) -> usize { todo!() }
}

impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}

impl<T: Clone, const N: usize> Clone for IntoIter<T, N> {
    fn clone(&self) -> IntoIter<T, N> { todo!() }
}

pub struct TryFromSliceError;

impl Debug for TryFromSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for TryFromSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for TryFromSliceError { fn clone(&self) -> TryFromSliceError { todo!() } }
impl std::error::Error for TryFromSliceError {}

impl<'a, T, const N: usize> TryFrom<&'a [T]> for [T; N] where T: Copy {
    type Error = TryFromSliceError;
    fn try_from(slice: &'a [T]) -> Result<[T; N], TryFromSliceError> { todo!() }
}
