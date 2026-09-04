//! `std::sync` — `Arc` and `Weak`.
//!
//! `clone` is a trait impl on `Arc<T>` itself, as in std, and `arc.clone()`
//! resolves to it: method lookup tries the receiver's own type at the `&Arc<T>`
//! autoref step and only then takes a deref step, so it finds `Clone for
//! Arc<T>` before it can ever see the pointee's `clone`. If the resolver did
//! reach the pointee, that would be a resolver bug, not a reason to declare
//! `clone` as something Rust does not.
//!
//! `downgrade`, `strong_count`, `weak_count`, `ptr_eq`, `as_ptr`, `get_mut` and
//! `make_mut` *are* associated functions taking `this: &Arc<T>`, because that
//! is what std does — precisely so that they cannot shadow a method of the same
//! name on the pointee. `Weak`'s equivalents are `&self` methods, again as std
//! has them, because `Weak<T>` does not deref to `T`.

pub struct Arc<T: ?Sized>;
pub struct Weak<T: ?Sized>;

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> { todo!() }
    pub fn try_unwrap(this: Arc<T>) -> Result<T, Arc<T>> { todo!() }
    pub fn into_inner(this: Arc<T>) -> Option<T> { todo!() }
    pub fn unwrap_or_clone(this: Arc<T>) -> T where T: Clone { todo!() }
}

impl<T: ?Sized> Arc<T> {
    pub fn downgrade(this: &Arc<T>) -> Weak<T> { todo!() }
    pub fn strong_count(this: &Arc<T>) -> usize { todo!() }
    pub fn weak_count(this: &Arc<T>) -> usize { todo!() }
    pub fn ptr_eq(this: &Arc<T>, other: &Arc<T>) -> bool { todo!() }
    pub fn as_ptr(this: &Arc<T>) -> *const T { todo!() }
    pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> { todo!() }
    pub fn make_mut(this: &mut Arc<T>) -> &mut T where T: Clone { todo!() }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T: ?Sized> Clone for Arc<T> { fn clone(&self) -> Arc<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for Arc<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + Display> Display for Arc<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: Default> Default for Arc<T> { fn default() -> Arc<T> { todo!() } }
impl<T: ?Sized + PartialEq<T>> PartialEq for Arc<T> { fn eq(&self, other: &Arc<T>) -> bool { todo!() } }
impl<T: ?Sized + Eq> Eq for Arc<T> {}
impl<T: ?Sized + PartialOrd<T>> PartialOrd for Arc<T> { fn partial_cmp(&self, other: &Arc<T>) -> Option<std::cmp::Ordering> { todo!() } }
impl<T: ?Sized + Ord> Ord for Arc<T> { fn cmp(&self, other: &Arc<T>) -> std::cmp::Ordering { todo!() } }
impl<T: ?Sized + Hash> Hash for Arc<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T> From<T> for Arc<T> { fn from(t: T) -> Arc<T> { todo!() } }
impl<T> From<Vec<T>> for Arc<[T]> { fn from(v: Vec<T>) -> Arc<[T]> { todo!() } }
impl<T> FromIterator<T> for Arc<[T]> { fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Arc<[T]> { todo!() } }

// `Arc<dyn Any + Send + Sync>::downcast` — the corpus's `as_arc_dyn_any` path.
impl Arc<dyn Any + Send + Sync> {
    pub fn downcast<T: Any + Send + Sync>(self) -> Result<Arc<T>, Arc<dyn Any + Send + Sync>> { todo!() }
}

impl<T> Weak<T> {
    pub fn new() -> Weak<T> { todo!() }
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Arc<T>> { todo!() }
    pub fn strong_count(&self) -> usize { todo!() }
    pub fn weak_count(&self) -> usize { todo!() }
    pub fn as_ptr(&self) -> *const T { todo!() }
    pub fn ptr_eq(&self, other: &Weak<T>) -> bool { todo!() }
}

impl<T: ?Sized> Clone for Weak<T> { fn clone(&self) -> Weak<T> { todo!() } }
impl<T: ?Sized> Debug for Weak<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Default for Weak<T> { fn default() -> Weak<T> { todo!() } }
