//! `std::sync` — `OnceLock`.
//!
//! Three `OnceLock::new()` sites in the corpus; the type carries the
//! initialize-once system catalogue.

pub struct OnceLock<T>;

impl<T> OnceLock<T> {
    pub fn new() -> OnceLock<T> { todo!() }
    pub fn get(&self) -> Option<&T> { todo!() }
    pub fn get_mut(&mut self) -> Option<&mut T> { todo!() }
    pub fn set(&self, value: T) -> Result<(), T> { todo!() }
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T { todo!() }
    pub fn into_inner(self) -> Option<T> { todo!() }
    pub fn take(&mut self) -> Option<T> { todo!() }
}

impl<T> Default for OnceLock<T> { fn default() -> OnceLock<T> { todo!() } }
impl<T: Debug> Debug for OnceLock<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: Clone> Clone for OnceLock<T> { fn clone(&self) -> OnceLock<T> { todo!() } }

pub struct Once;

impl Once {
    pub fn new() -> Once { todo!() }
    pub fn call_once<F: FnOnce()>(&self, f: F) { todo!() }
    pub fn is_completed(&self) -> bool { todo!() }
}
