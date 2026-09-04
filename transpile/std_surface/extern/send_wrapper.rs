//! `send_wrapper` 0.6.0
//!
//! Not on the deliverable's list, and load-bearing: `SendWrapper<T>` derefs to
//! `T`, so it is a step in the deref chain at 13 sites across `core/src/model.rs`
//! and the IndexedDB crate. Missing it means `wrapper.some_method()` cannot
//! resolve. It exists in ankurah because wasm futures are not `Send` but the
//! async stack's bounds ask for `Send`; in the port that distinction disappears,
//! which is an emission decision, not a declaration one.

pub struct SendWrapper<T>;

impl<T> SendWrapper<T> {
    pub fn new(data: T) -> SendWrapper<T> { todo!() }
    pub fn take(self) -> T { todo!() }
    pub fn valid(&self) -> bool { todo!() }
}

impl<T> Deref for SendWrapper<T> {
    type Target = T;
    fn deref(&self) -> &T { todo!() }
}

impl<T> DerefMut for SendWrapper<T> {
    fn deref_mut(&mut self) -> &mut T { todo!() }
}

impl<T> Drop for SendWrapper<T> { fn drop(&mut self) { todo!() } }
impl<T: Clone> Clone for SendWrapper<T> { fn clone(&self) -> SendWrapper<T> { todo!() } }
impl<T: Debug> Debug for SendWrapper<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }

// 0.6.0 has neither `Default` nor `From<T>`; construction is `SendWrapper::new`
// only, which is what all 13 corpus sites write.

// Both under the `futures` feature, which the wasm crates enable.
impl<F: Future> Future for SendWrapper<F> {
    type Output = <F as Future>::Output;
    fn poll(self: Pin<&mut SendWrapper<F>>, cx: &mut std::task::Context<'_>) -> Poll<<F as Future>::Output> { todo!() }
}

impl<S: Stream> Stream for SendWrapper<S> {
    type Item = <S as Stream>::Item;
    fn poll_next(self: Pin<&mut SendWrapper<S>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<S as Stream>::Item>> { todo!() }
    fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
}
