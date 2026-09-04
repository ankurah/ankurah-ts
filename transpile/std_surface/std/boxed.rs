//! `std::boxed`

pub struct Box<T: ?Sized>;

impl<T> Box<T> {
    pub fn new(x: T) -> Box<T> { todo!() }
    pub fn pin(x: T) -> Pin<Box<T>> { todo!() }
}

impl<T: ?Sized> Box<T> {
    pub fn into_raw(b: Box<T>) -> *mut T { todo!() }
    pub unsafe fn from_raw(raw: *mut T) -> Box<T> { todo!() }
    pub fn leak<'a>(b: Box<T>) -> &'a mut T { todo!() }
    pub fn as_ref(&self) -> &T { todo!() }
    pub fn as_mut(&mut self) -> &mut T { todo!() }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T: ?Sized> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<T: Clone> Clone for Box<T> { fn clone(&self) -> Box<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for Box<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: ?Sized + Display> Display for Box<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: Default> Default for Box<T> { fn default() -> Box<T> { todo!() } }
impl<T: ?Sized + PartialEq<T>> PartialEq for Box<T> { fn eq(&self, other: &Box<T>) -> bool { todo!() } }
impl<T: ?Sized + Eq> Eq for Box<T> {}
impl<T: ?Sized + PartialOrd<T>> PartialOrd for Box<T> { fn partial_cmp(&self, other: &Box<T>) -> Option<std::cmp::Ordering> { todo!() } }
impl<T: ?Sized + Ord> Ord for Box<T> { fn cmp(&self, other: &Box<T>) -> std::cmp::Ordering { todo!() } }
impl<T: ?Sized + Hash> Hash for Box<T> { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl<T> From<T> for Box<T> { fn from(t: T) -> Box<T> { todo!() } }
impl From<&str> for Box<str> { fn from(s: &str) -> Box<str> { todo!() } }
impl<T> From<Vec<T>> for Box<[T]> { fn from(v: Vec<T>) -> Box<[T]> { todo!() } }

// `?` on an error inside a function returning `Box<dyn Error ..>` selects this.
impl<'a, E: std::error::Error + 'a> From<E> for Box<dyn std::error::Error + 'a> {
    fn from(err: E) -> Box<dyn std::error::Error + 'a> { todo!() }
}
impl<'a, E: std::error::Error + Send + Sync + 'a> From<E> for Box<dyn std::error::Error + Send + Sync + 'a> {
    fn from(err: E) -> Box<dyn std::error::Error + Send + Sync + 'a> { todo!() }
}
impl<'a> From<String> for Box<dyn std::error::Error + Send + Sync + 'a> {
    fn from(err: String) -> Box<dyn std::error::Error + Send + Sync + 'a> { todo!() }
}
impl<'a> From<&str> for Box<dyn std::error::Error + Send + Sync + 'a> {
    fn from(err: &str) -> Box<dyn std::error::Error + Send + Sync + 'a> { todo!() }
}

// `Box<F>` is a `Future` only when `F: Unpin`; otherwise the future has to be
// pinned first, which is what `Box::pin` is for and what every corpus site does.
impl<T: ?Sized + Future + Unpin> Future for Box<T> {
    type Output = <T as Future>::Output;
    fn poll(self: Pin<&mut Box<T>>, cx: &mut std::task::Context<'_>) -> Poll<<T as Future>::Output> { todo!() }
}

impl<T> IntoIterator for Box<[T]> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> std::vec::IntoIter<T> { todo!() }
}

impl<'a, T> IntoIterator for &'a Box<[T]> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> { todo!() }
}

impl<'a, T> IntoIterator for &'a mut Box<[T]> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> std::slice::IterMut<'a, T> { todo!() }
}

impl<T> FromIterator<T> for Box<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Box<[T]> { todo!() }
}
