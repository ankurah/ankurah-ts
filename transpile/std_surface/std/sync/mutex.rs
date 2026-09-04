//! `std::sync` — `Mutex` and its guard.
//!
//! `lock()` returns a `Result`, so `lock().unwrap()` is `Result::unwrap` on
//! `LockResult<MutexGuard<'_, T>>` and yields the guard. The engine needs no
//! special case for it; the previous TOML table had one only because it
//! described the TypeScript polyfill, whose `lock()` returns the guard
//! directly.

pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;
pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

pub struct Mutex<T: ?Sized>;

impl<T> Mutex<T> {
    pub fn new(t: T) -> Mutex<T> { todo!() }
    pub fn into_inner(self) -> LockResult<T> { todo!() }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> { todo!() }
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> { todo!() }
    pub fn is_poisoned(&self) -> bool { todo!() }
    pub fn clear_poison(&self) { todo!() }
    pub fn get_mut(&mut self) -> LockResult<&mut T> { todo!() }
}

impl<T: Default> Default for Mutex<T> { fn default() -> Mutex<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for Mutex<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> From<T> for Mutex<T> { fn from(t: T) -> Mutex<T> { todo!() } }

pub struct MutexGuard<'a, T: ?Sized>;

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) { todo!() }
}

impl<'a, T: ?Sized + Debug> Debug for MutexGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a, T: ?Sized + Display> Display for MutexGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct PoisonError<T>;

impl<T> PoisonError<T> {
    pub fn into_inner(self) -> T { todo!() }
    pub fn get_ref(&self) -> &T { todo!() }
    pub fn get_mut(&mut self) -> &mut T { todo!() }
}

impl<T> Debug for PoisonError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Display for PoisonError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> std::error::Error for PoisonError<T> {}

pub enum TryLockError<T> {
    Poisoned(PoisonError<T>),
    WouldBlock,
}

impl<T> Debug for TryLockError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Display for TryLockError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> std::error::Error for TryLockError<T> {}
