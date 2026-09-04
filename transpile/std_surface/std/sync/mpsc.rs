//! `std::sync::mpsc`
//!
//! `signals/src/broadcast.rs` and `porcelain/subscribe.rs` implement
//! `IntoBroadcastListener` and `IntoSubscribeListener` for
//! `std::sync::mpsc::Sender<T>`, so the type has to exist for those corpus
//! impls to have a subject.

pub struct Sender<T>;
pub struct SyncSender<T>;
pub struct Receiver<T>;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) { todo!() }
pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) { todo!() }

impl<T> Sender<T> {
    pub fn send(&self, t: T) -> Result<(), SendError<T>> { todo!() }
}

impl<T> Clone for Sender<T> { fn clone(&self) -> Sender<T> { todo!() } }
impl<T> Debug for Sender<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

impl<T> SyncSender<T> {
    pub fn send(&self, t: T) -> Result<(), SendError<T>> { todo!() }
    pub fn try_send(&self, t: T) -> Result<(), TrySendError<T>> { todo!() }
}

impl<T> Clone for SyncSender<T> { fn clone(&self) -> SyncSender<T> { todo!() } }

impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, RecvError> { todo!() }
    pub fn try_recv(&self) -> Result<T, TryRecvError> { todo!() }
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> { todo!() }
    pub fn iter(&self) -> std::sync::mpsc::Iter<'_, T> { todo!() }
    pub fn try_iter(&self) -> TryIter<'_, T> { todo!() }
}

pub struct Iter<'a, T>;
pub struct TryIter<'a, T>;
pub struct IntoIter<T>;

impl<'a, T> Iterator for Iter<'a, T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<'a, T> Iterator for TryIter<'a, T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }
impl<T> Iterator for IntoIter<T> { type Item = T; fn next(&mut self) -> Option<T> { todo!() } }

impl<T> IntoIterator for Receiver<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { todo!() }
}

pub struct SendError<T>(pub T);
pub struct RecvError;

pub enum TrySendError<T> {
    Full(T),
    Disconnected(T),
}

pub enum TryRecvError {
    Empty,
    Disconnected,
}

pub enum RecvTimeoutError {
    Timeout,
    Disconnected,
}

impl<T> Debug for SendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> Display for SendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl<T> std::error::Error for SendError<T> {}
impl Debug for RecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for RecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for RecvError {}
impl Debug for TryRecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for TryRecvError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for TryRecvError {}
