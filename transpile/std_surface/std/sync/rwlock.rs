//! `std::sync` — `RwLock` and its two guards.

pub struct RwLock<T: ?Sized>;

impl<T> RwLock<T> {
    pub fn new(t: T) -> RwLock<T> { todo!() }
    pub fn into_inner(self) -> LockResult<T> { todo!() }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> { todo!() }
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> { todo!() }
    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> { todo!() }
    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> { todo!() }
    pub fn is_poisoned(&self) -> bool { todo!() }
    pub fn clear_poison(&self) { todo!() }
    pub fn get_mut(&mut self) -> LockResult<&mut T> { todo!() }
}

impl<T: Default> Default for RwLock<T> { fn default() -> RwLock<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for RwLock<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> From<T> for RwLock<T> { fn from(t: T) -> RwLock<T> { todo!() } }

pub struct RwLockReadGuard<'a, T: ?Sized>;

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) { todo!() }
}

impl<'a, T: ?Sized + Debug> Debug for RwLockReadGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a, T: ?Sized + Display> Display for RwLockReadGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct RwLockWriteGuard<'a, T: ?Sized>;

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) { todo!() }
}

impl<'a, T: ?Sized + Debug> Debug for RwLockWriteGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a, T: ?Sized + Display> Display for RwLockWriteGuard<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
