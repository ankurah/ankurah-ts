//! `std::task`

pub enum Poll<T> {
    Ready(T),
    Pending,
}

impl<T> Poll<T> {
    pub fn is_ready(&self) -> bool { todo!() }
    pub fn is_pending(&self) -> bool { todo!() }
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<U> { todo!() }
}

impl<T, E> Poll<Result<T, E>> {
    pub fn map_ok<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<Result<U, E>> { todo!() }
    pub fn map_err<U, F: FnOnce(E) -> U>(self, f: F) -> Poll<Result<T, U>> { todo!() }
}

impl<T, E> Poll<Option<Result<T, E>>> {
    pub fn map_ok<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<Option<Result<U, E>>> { todo!() }
    pub fn map_err<U, F: FnOnce(E) -> U>(self, f: F) -> Poll<Option<Result<T, U>>> { todo!() }
}

impl<T: Clone> Clone for Poll<T> { fn clone(&self) -> Poll<T> { todo!() } }
impl<T: Copy> Copy for Poll<T> {}
impl<T: Debug> Debug for Poll<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T: PartialEq> PartialEq for Poll<T> { fn eq(&self, other: &Poll<T>) -> bool { todo!() } }
impl<T> From<T> for Poll<T> { fn from(t: T) -> Poll<T> { todo!() } }

pub struct Context<'a>;

impl<'a> Context<'a> {
    pub fn from_waker(waker: &'a Waker) -> Context<'a> { todo!() }
    pub fn waker(&self) -> &'a Waker { todo!() }
}

impl<'a> Debug for Context<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct Waker;

impl Waker {
    pub fn wake(self) { todo!() }
    pub fn wake_by_ref(&self) { todo!() }
    pub fn will_wake(&self, other: &Waker) -> bool { todo!() }
}

impl Clone for Waker { fn clone(&self) -> Waker { todo!() } }
impl Debug for Waker { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
