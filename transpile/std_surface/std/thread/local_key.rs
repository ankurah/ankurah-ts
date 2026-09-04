//! `std::thread` — `LocalKey`, the type a `thread_local!` block produces.
//!
//! `thread_local!` is a macro and the engine does not expand macros. What it
//! needs is the type the macro's item has: `signals`' `OBSERVER_STACK` is a
//! `LocalKey<RefCell<Vec<..>>>`, and every `OBSERVER_STACK.with(|s| ..)` in the
//! corpus resolves against this declaration once the macro handler assigns the
//! static that type (spec 4.10).

pub struct LocalKey<T: 'static>;

impl<T: 'static> LocalKey<T> {
    pub fn with<F, R>(&'static self, f: F) -> R where F: FnOnce(&T) -> R { todo!() }
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError> where F: FnOnce(&T) -> R { todo!() }
}

impl<T: 'static> LocalKey<Cell<T>> {
    pub fn set(&'static self, value: T) { todo!() }
    pub fn get(&'static self) -> T where T: Copy { todo!() }
    pub fn take(&'static self) -> T where T: Default { todo!() }
    pub fn replace(&'static self, value: T) -> T { todo!() }
}

impl<T: 'static> LocalKey<RefCell<T>> {
    pub fn with_borrow<F, R>(&'static self, f: F) -> R where F: FnOnce(&T) -> R { todo!() }
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R where F: FnOnce(&mut T) -> R { todo!() }
    pub fn set(&'static self, value: T) { todo!() }
    pub fn take(&'static self) -> T where T: Default { todo!() }
    pub fn replace(&'static self, value: T) -> T { todo!() }
}

impl<T: 'static> Debug for LocalKey<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct AccessError;

impl Debug for AccessError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for AccessError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for AccessError {}
