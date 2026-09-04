//! `std::rc`
//!
//! `signals` reaches for `Rc` in two places under the single-threaded
//! configuration; the shape is `Arc`'s without the atomics.

pub struct Rc<T: ?Sized>;
pub struct Weak<T: ?Sized>;

impl<T> Rc<T> {
    pub fn new(value: T) -> Rc<T> { todo!() }
    pub fn try_unwrap(this: Rc<T>) -> Result<T, Rc<T>> { todo!() }
    pub fn into_inner(this: Rc<T>) -> Option<T> { todo!() }
}

impl<T: ?Sized> Rc<T> {
    pub fn downgrade(this: &Rc<T>) -> Weak<T> { todo!() }
    pub fn strong_count(this: &Rc<T>) -> usize { todo!() }
    pub fn weak_count(this: &Rc<T>) -> usize { todo!() }
    pub fn ptr_eq(this: &Rc<T>, other: &Rc<T>) -> bool { todo!() }
    pub fn as_ptr(this: &Rc<T>) -> *const T { todo!() }
    pub fn get_mut(this: &mut Rc<T>) -> Option<&mut T> { todo!() }
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T: ?Sized> Clone for Rc<T> { fn clone(&self) -> Rc<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for Rc<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + std::fmt::Display> std::fmt::Display for Rc<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: Default> Default for Rc<T> { fn default() -> Rc<T> { todo!() } }
impl<T: ?Sized + PartialEq<T>> PartialEq for Rc<T> { fn eq(&self, other: &Rc<T>) -> bool { todo!() } }
impl<T: ?Sized + Eq> Eq for Rc<T> {}
impl<T: ?Sized + Hash> Hash for Rc<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T> From<T> for Rc<T> { fn from(t: T) -> Rc<T> { todo!() } }

impl<T> Weak<T> {
    pub fn new() -> Weak<T> { todo!() }
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Rc<T>> { todo!() }
    pub fn strong_count(&self) -> usize { todo!() }
    pub fn weak_count(&self) -> usize { todo!() }
    pub fn as_ptr(&self) -> *const T { todo!() }
    pub fn ptr_eq(&self, other: &Weak<T>) -> bool { todo!() }
}

impl<T: ?Sized> Clone for Weak<T> { fn clone(&self) -> Weak<T> { todo!() } }
impl<T: ?Sized> Debug for Weak<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Default for Weak<T> { fn default() -> Weak<T> { todo!() } }
