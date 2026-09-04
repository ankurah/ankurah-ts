//! `std::cell`
//!
//! The port's `RefCell`, `Ref` and `RefMut` polyfills drop their contents when
//! dropped, and dropping a `RefCell` while a guard is outstanding is fatal
//! (spec section 1). Nothing about that is visible in the Rust signature; it is
//! the emission layer's contract. What this file declares is Rust's shape, so
//! the engine walks `Ref<T>` -> `T` the same way it walks `MutexGuard<T>` -> `T`.

pub struct RefCell<T: ?Sized>;

impl<T> RefCell<T> {
    pub fn new(value: T) -> RefCell<T> { todo!() }
    pub fn into_inner(self) -> T { todo!() }
    pub fn replace(&self, t: T) -> T { todo!() }
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T { todo!() }
    pub fn take(&self) -> T where T: Default { todo!() }
}

impl<T: ?Sized> RefCell<T> {
    pub fn borrow(&self) -> Ref<'_, T> { todo!() }
    pub fn borrow_mut(&self) -> RefMut<'_, T> { todo!() }
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> { todo!() }
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> { todo!() }
    pub fn get_mut(&mut self) -> &mut T { todo!() }
    pub fn as_ptr(&self) -> *mut T { todo!() }
}

impl<T: Clone> Clone for RefCell<T> { fn clone(&self) -> RefCell<T> { todo!() } }
impl<T: Default> Default for RefCell<T> { fn default() -> RefCell<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for RefCell<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: PartialEq<T>> PartialEq for RefCell<T> { fn eq(&self, other: &RefCell<T>) -> bool { todo!() } }
impl<T> From<T> for RefCell<T> { fn from(t: T) -> RefCell<T> { todo!() } }

pub struct Ref<'a, T: ?Sized>;

impl<'a, T: ?Sized> Ref<'a, T> {
    pub fn clone(orig: &Ref<'a, T>) -> Ref<'a, T> { todo!() }
    pub fn map<U: ?Sized, F: FnOnce(&T) -> &U>(orig: Ref<'a, T>, f: F) -> Ref<'a, U> { todo!() }
}

impl<'a, T: ?Sized> Deref for Ref<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> Drop for Ref<'a, T> { fn drop(&mut self) { todo!() } }
impl<'a, T: ?Sized + Debug> Debug for Ref<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<'a, T: ?Sized + std::fmt::Display> std::fmt::Display for Ref<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct RefMut<'a, T: ?Sized>;

impl<'a, T: ?Sized> RefMut<'a, T> {
    pub fn map<U: ?Sized, F: FnOnce(&mut T) -> &mut U>(orig: RefMut<'a, T>, f: F) -> RefMut<'a, U> { todo!() }
}

impl<'a, T: ?Sized> Deref for RefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<'a, T: ?Sized> Drop for RefMut<'a, T> { fn drop(&mut self) { todo!() } }
impl<'a, T: ?Sized + Debug> Debug for RefMut<'a, T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct BorrowError;
pub struct BorrowMutError;

impl Debug for BorrowError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for BorrowError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for BorrowError {}
impl Debug for BorrowMutError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for BorrowMutError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for BorrowMutError {}

pub struct Cell<T: ?Sized>;

impl<T> Cell<T> {
    pub fn new(value: T) -> Cell<T> { todo!() }
    pub fn set(&self, val: T) { todo!() }
    pub fn replace(&self, val: T) -> T { todo!() }
    pub fn into_inner(self) -> T { todo!() }
    pub fn take(&self) -> T where T: Default { todo!() }
}

impl<T: Copy> Cell<T> {
    pub fn get(&self) -> T { todo!() }
}

impl<T: ?Sized> Cell<T> {
    pub fn get_mut(&mut self) -> &mut T { todo!() }
    pub fn as_ptr(&self) -> *mut T { todo!() }
}

impl<T: Copy> Clone for Cell<T> { fn clone(&self) -> Cell<T> { todo!() } }
impl<T: Default> Default for Cell<T> { fn default() -> Cell<T> { todo!() } }
impl<T: Copy + Debug> Debug for Cell<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> From<T> for Cell<T> { fn from(t: T) -> Cell<T> { todo!() } }

pub struct OnceCell<T>;

impl<T> OnceCell<T> {
    pub fn new() -> OnceCell<T> { todo!() }
    pub fn get(&self) -> Option<&T> { todo!() }
    pub fn get_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn set(&self, value: T) -> Result<(), T> { todo!() }
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T { todo!() }
    pub fn into_inner(self) -> Option<T> { todo!() }
    pub fn take(&mut self) -> Option<T> { todo!() }
}

impl<T> Default for OnceCell<T> { fn default() -> OnceCell<T> { todo!() } }
impl<T: Debug> Debug for OnceCell<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct UnsafeCell<T: ?Sized>;

impl<T> UnsafeCell<T> {
    pub fn new(value: T) -> UnsafeCell<T> { todo!() }
    pub fn into_inner(self) -> T { todo!() }
}

impl<T: ?Sized> UnsafeCell<T> {
    pub fn get(&self) -> *mut T { todo!() }
    pub fn get_mut(&mut self) -> &mut T { todo!() }
}
