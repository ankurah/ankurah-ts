//! `std::result`
//!
//! `lock().unwrap()` resolves here: `Mutex::lock` returns
//! `LockResult<MutexGuard<'_, T>>`, which is this `Result`, and `unwrap`
//! returns the guard. The old TOML table described the TypeScript polyfill and
//! made `unwrap` a special case; nothing here is special.

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool { todo!() }
    pub fn is_err(&self) -> bool { todo!() }
    pub fn is_ok_and(self, f: impl FnOnce(T) -> bool) -> bool { todo!() }
    pub fn is_err_and(self, f: impl FnOnce(E) -> bool) -> bool { todo!() }

    pub fn ok(self) -> Option<T> { todo!() }
    pub fn err(self) -> Option<E> { todo!() }

    pub fn as_ref(&self) -> Result<&T, &E> { todo!() }
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> { todo!() }
    pub fn as_deref(&self) -> Result<&<T as Deref>::Target, &E> where T: Deref { todo!() }

    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> Result<U, E> { todo!() }
    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> Result<T, F> { todo!() }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U { todo!() }
    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U { todo!() }
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Result<T, E> { todo!() }
    pub fn inspect_err<F: FnOnce(&E)>(self, f: F) -> Result<T, E> { todo!() }

    pub fn and<U>(self, res: Result<U, E>) -> Result<U, E> { todo!() }
    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, op: F) -> Result<U, E> { todo!() }
    pub fn or<F>(self, res: Result<T, F>) -> Result<T, F> { todo!() }
    pub fn or_else<F, O: FnOnce(E) -> Result<T, F>>(self, op: O) -> Result<T, F> { todo!() }

    pub fn unwrap(self) -> T where E: Debug { todo!() }
    pub fn unwrap_or(self, default: T) -> T { todo!() }
    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, op: F) -> T { todo!() }
    pub fn unwrap_or_default(self) -> T where T: Default { todo!() }
    pub fn unwrap_err(self) -> E where T: Debug { todo!() }
    pub fn expect(self, msg: &str) -> T where E: Debug { todo!() }
    pub fn expect_err(self, msg: &str) -> E where T: Debug { todo!() }

    pub fn iter(&self) -> std::result::Iter<'_, T> { todo!() }
}

impl<T, E> Result<Option<T>, E> {
    pub fn transpose(self) -> Option<Result<T, E>> { todo!() }
}

impl<T, E> Result<Result<T, E>, E> {
    pub fn flatten(self) -> Result<T, E> { todo!() }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> { fn clone(&self) -> Result<T, E> { todo!() } }
impl<T: Debug, E: Debug> Debug for Result<T, E> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: PartialEq, E: PartialEq> PartialEq for Result<T, E> { fn eq(&self, other: &Result<T, E>) -> bool { todo!() } }
impl<T: Eq, E: Eq> Eq for Result<T, E> {}
impl<T: Copy, E: Copy> Copy for Result<T, E> {}
impl<T: PartialOrd, E: PartialOrd> PartialOrd for Result<T, E> { fn partial_cmp(&self, other: &Result<T, E>) -> Option<std::cmp::Ordering> { todo!() } }
impl<T: Ord, E: Ord> Ord for Result<T, E> { fn cmp(&self, other: &Result<T, E>) -> std::cmp::Ordering { todo!() } }
impl<T: Hash, E: Hash> Hash for Result<T, E> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }

pub struct Iter<'a, T>;
pub struct IterMut<'a, T>;
pub struct IntoIter<T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = &'a T; fn next(&mut self) -> Option<&'a T> { todo!() } }
impl<'a, T> Iterator for IterMut<'a, T> { type Item = &'a mut T; fn next(&mut self) -> Option<&'a mut T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

impl<T, E> IntoIterator for Result<T, E> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}
impl<'a, T, E> IntoIterator for &'a Result<T, E> {
    type Item = &'a T;
    type IntoIter = std::result::Iter<'a, T>;
    fn into_iter(self) -> std::result::Iter<'a, T> { todo!() }
}
impl<'a, T, E> IntoIterator for &'a mut Result<T, E> {
    type Item = &'a mut T;
    type IntoIter = std::result::IterMut<'a, T>;
    fn into_iter(self) -> std::result::IterMut<'a, T> { todo!() }
}

// `collect::<Result<Vec<_>, _>>()` — the corpus writes this turbofish directly,
// and writes it again through `let` annotations.
impl<A, E, V: FromIterator<A>> FromIterator<Result<A, E>> for Result<V, E> {
    fn from_iter<I: IntoIterator<Item = Result<A, E>>>(iter: I) -> Result<V, E> { todo!() }
}
