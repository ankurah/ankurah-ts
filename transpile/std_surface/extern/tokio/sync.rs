//! `tokio::sync` 1.48.0 — the parts ankurah uses.
//!
//! `tokio::sync::Mutex` is not `std::sync::Mutex`: `lock()` is `async` and
//! returns the guard directly rather than a `LockResult`. Keeping the two in
//! separate modules with their real signatures is the whole reason the registry
//! is not flat.

pub struct Notify;

impl Notify {
    pub fn new() -> Notify { todo!() }
    /// Returns a named future, not `()`, and the name is load-bearing.
    ///
    /// Creating a `Notified` records the current `notify_waiters` generation.
    /// It joins the `notify_one` queue — and consumes a stored permit if one is
    /// waiting — only at its first poll, or when `enable()` is called. So in
    /// `let n = notify.notified(); do_thing(); n.await;` the future cannot miss
    /// a `notify_waiters` issued during `do_thing`, because the generation was
    /// recorded before it ran; but it *can* miss a `notify_one` issued then,
    /// unless `enable()` was called first. That gap is the documented reason
    /// `enable()` exists.
    ///
    /// Collapsing this to an `async fn` returning `()` would type the whole
    /// distinction away.
    pub fn notified(&self) -> Notified<'_> { todo!() }
    pub fn notify_one(&self) { todo!() }
    pub fn notify_last(&self) { todo!() }
    pub fn notify_waiters(&self) { todo!() }
}

pub struct Notified<'a>;

impl<'a> Future for Notified<'a> {
    type Output = ();
    fn poll(self: Pin<&mut Notified<'a>>, cx: &mut std::task::Context<'_>) -> Poll<()> { todo!() }
}

impl<'a> Notified<'a> {
    /// Joins the `notify_one` queue now instead of at first poll, consuming a
    /// stored permit if one is waiting; returns whether it took one. This is
    /// how a caller closes the window described on `notified` above, between
    /// creating the future and first polling it.
    pub fn enable(self: Pin<&mut Notified<'a>>) -> bool { todo!() }
}

impl<'a> Debug for Notified<'a> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

impl Default for Notify { fn default() -> Notify { todo!() } }
impl Debug for Notify { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct Mutex<T: ?Sized>;

impl<T> Mutex<T> {
    pub fn new(t: T) -> Mutex<T> { todo!() }
    pub fn into_inner(self) -> T { todo!() }
}

impl<T: ?Sized> Mutex<T> {
    pub async fn lock(&self) -> MutexGuard<'_, T> { todo!() }
    pub fn blocking_lock(&self) -> MutexGuard<'_, T> { todo!() }
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, TryLockError> { todo!() }
    pub fn get_mut(&mut self) -> &mut T { todo!() }
}

impl<T: Default> Default for Mutex<T> { fn default() -> Mutex<T> { todo!() } }
impl<T: ?Sized + Debug> Debug for Mutex<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

pub struct MutexGuard<'a, T: ?Sized>;

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> { fn drop(&mut self) { todo!() } }

pub struct RwLock<T: ?Sized>;

impl<T> RwLock<T> {
    pub fn new(value: T) -> RwLock<T> { todo!() }
    pub fn into_inner(self) -> T { todo!() }
}

impl<T: ?Sized> RwLock<T> {
    pub async fn read(&self) -> RwLockReadGuard<'_, T> { todo!() }
    pub async fn write(&self) -> RwLockWriteGuard<'_, T> { todo!() }
    pub fn blocking_read(&self) -> RwLockReadGuard<'_, T> { todo!() }
    pub fn blocking_write(&self) -> RwLockWriteGuard<'_, T> { todo!() }
    pub fn try_read(&self) -> Result<RwLockReadGuard<'_, T>, TryLockError> { todo!() }
    pub fn try_write(&self) -> Result<RwLockWriteGuard<'_, T>, TryLockError> { todo!() }
    pub fn get_mut(&mut self) -> &mut T { todo!() }
}

pub struct RwLockReadGuard<'a, T: ?Sized>;
pub struct RwLockWriteGuard<'a, T: ?Sized>;

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}
impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> { fn drop(&mut self) { todo!() } }

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}
impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}
impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> { fn drop(&mut self) { todo!() } }

pub struct TryLockError;

impl Debug for TryLockError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for TryLockError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for TryLockError {}

pub mod oneshot {
    pub struct Sender<T>;
    pub struct Receiver<T>;

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) { todo!() }

    impl<T> Sender<T> {
        pub fn send(self, t: T) -> Result<(), T> { todo!() }
        pub fn is_closed(&self) -> bool { todo!() }
        pub async fn closed(&mut self) { todo!() }
    }

    impl<T> Receiver<T> {
        pub fn try_recv(&mut self) -> Result<T, error::TryRecvError> { todo!() }
        pub fn close(&mut self) { todo!() }
    }

    impl<T> Future for Receiver<T> {
        type Output = Result<T, error::RecvError>;
        fn poll(self: Pin<&mut Receiver<T>>, cx: &mut std::task::Context<'_>) -> Poll<Result<T, error::RecvError>> { todo!() }
    }

    pub mod error {
        pub struct RecvError;

        pub enum TryRecvError {
            Empty,
            Closed,
        }

        impl Debug for RecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::fmt::Display for RecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::error::Error for RecvError {}
        impl Debug for TryRecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::fmt::Display for TryRecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::error::Error for TryRecvError {}
    }
}

pub mod mpsc {
    pub struct Sender<T>;
    pub struct Receiver<T>;
    pub struct UnboundedSender<T>;
    pub struct UnboundedReceiver<T>;

    pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) { todo!() }
    pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) { todo!() }

    impl<T> Sender<T> {
        pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> { todo!() }
        pub fn try_send(&self, message: T) -> Result<(), error::TrySendError<T>> { todo!() }
        pub fn is_closed(&self) -> bool { todo!() }
    }
    impl<T> Clone for Sender<T> { fn clone(&self) -> Sender<T> { todo!() } }

    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> { todo!() }
        pub fn try_recv(&mut self) -> Result<T, error::TryRecvError> { todo!() }
        pub fn close(&mut self) { todo!() }
    }

    impl<T> UnboundedSender<T> {
        pub fn send(&self, message: T) -> Result<(), error::SendError<T>> { todo!() }
        pub fn is_closed(&self) -> bool { todo!() }
    }
    impl<T> Clone for UnboundedSender<T> { fn clone(&self) -> UnboundedSender<T> { todo!() } }

    impl<T> UnboundedReceiver<T> {
        pub async fn recv(&mut self) -> Option<T> { todo!() }
        pub fn try_recv(&mut self) -> Result<T, error::TryRecvError> { todo!() }
        pub fn close(&mut self) { todo!() }
    }

    pub mod error {
        pub struct SendError<T>(pub T);

        pub enum TrySendError<T> {
            Full(T),
            Closed(T),
        }

        pub enum TryRecvError {
            Empty,
            Disconnected,
        }

        impl<T> Debug for SendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl<T> std::fmt::Display for SendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl<T> std::error::Error for SendError<T> {}
        impl<T> Debug for TrySendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl Debug for TryRecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
    }
}

pub mod watch {
    pub struct Sender<T>;
    pub struct Receiver<T>;

    pub fn channel<T>(init: T) -> (Sender<T>, Receiver<T>) { todo!() }

    impl<T> Sender<T> {
        pub fn send(&self, value: T) -> Result<(), SendError<T>> { todo!() }
        pub fn borrow(&self) -> Ref<'_, T> { todo!() }
    }

    impl<T> Receiver<T> {
        pub async fn changed(&mut self) -> Result<(), RecvError> { todo!() }
        pub fn borrow(&self) -> Ref<'_, T> { todo!() }
    }

    pub struct Ref<'a, T>;
    pub struct SendError<T>(pub T);
    pub struct RecvError;

    impl<T> Clone for Sender<T> { fn clone(&self) -> Sender<T> { todo!() } }
    impl<T> Clone for Receiver<T> { fn clone(&self) -> Receiver<T> { todo!() } }

    impl<'a, T> Deref for Ref<'a, T> {
        type Target = T;
        fn deref(&self) -> &T { todo!() }
    }
}
