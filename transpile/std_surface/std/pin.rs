//! `std::pin`
//!
//! Not a file the deliverable listed, but `Pin<&mut Self>` is in every
//! hand-written `Stream` and `Future` impl in `core/src/util/`, and `Pin::new`
//! and `Box::pin` both appear in the corpus, so the type has to exist for those
//! signatures to resolve.

pub struct Pin<P>;

impl<P: Deref> Pin<P> {
    pub fn as_ref(&self) -> Pin<&<P as Deref>::Target> { todo!() }
    pub unsafe fn new_unchecked(pointer: P) -> Pin<P> { todo!() }
    pub unsafe fn into_inner_unchecked(pin: Pin<P>) -> P { todo!() }
}

impl<P: DerefMut> Pin<P> {
    pub fn as_mut(&mut self) -> Pin<&mut <P as Deref>::Target> { todo!() }
    pub fn set(&mut self, value: <P as Deref>::Target) where <P as Deref>::Target: Sized { todo!() }
}

impl<P: Deref> Pin<P> where <P as Deref>::Target: Unpin {
    pub fn new(pointer: P) -> Pin<P> { todo!() }
    pub fn into_inner(pin: Pin<P>) -> P { todo!() }
}

impl<'a, T: ?Sized> Pin<&'a mut T> {
    pub fn get_mut(self) -> &'a mut T where T: Unpin { todo!() }
    pub unsafe fn get_unchecked_mut(self) -> &'a mut T { todo!() }
}

impl<'a, T: ?Sized> Pin<&'a T> {
    pub fn get_ref(self) -> &'a T { todo!() }
}

impl<P: Deref> Deref for Pin<P> {
    type Target = <P as Deref>::Target;
    fn deref(&self) -> &<P as Deref>::Target { todo!() }
}

impl<P: DerefMut> DerefMut for Pin<P> where <P as Deref>::Target: Unpin {
    fn deref_mut(&mut self) -> &mut <P as Deref>::Target { todo!() }
}

impl<P: Clone> Clone for Pin<P> { fn clone(&self) -> Pin<P> { todo!() } }
impl<P: Copy> Copy for Pin<P> {}
impl<P: Debug> Debug for Pin<P> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
