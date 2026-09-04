//! `std::clone`

pub trait Clone: Sized {
    fn clone(&self) -> Self;
    fn clone_from(&mut self, source: &Self);
}

// `impl<T: ?Sized> Clone for &T` is what makes `(&x).clone()` return `&T` rather
// than `T` — the shadowing that bites when `x: &Vec<_>` and the author wanted a
// deep copy. The engine has to see it to reproduce Rust's answer.
impl<T: ?Sized> Clone for &T {
    fn clone(&self) -> &T { todo!() }
    fn clone_from(&mut self, source: &&T) { todo!() }
}
