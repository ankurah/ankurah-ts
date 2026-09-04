//! `tokio::task` and `tokio::spawn`, tokio 1.48.0.
//!
//! `core/src/task.rs` calls `tokio::spawn` on the non-wasm branch and
//! `wasm_bindgen_futures::spawn_local` on the wasm one. The port's cfg is the
//! wasm32 configuration, so this surface is here for the branch the cfg
//! evaluator currently keeps, not because the browser build reaches it.

pub fn spawn<F: Future + Send + 'static>(future: F) -> JoinHandle<<F as Future>::Output>
where <F as Future>::Output: Send + 'static { todo!() }

pub fn spawn_blocking<F: FnOnce() -> R + Send + 'static, R: Send + 'static>(f: F) -> JoinHandle<R> { todo!() }

pub fn spawn_local<F: Future + 'static>(future: F) -> JoinHandle<<F as Future>::Output> { todo!() }

pub async fn yield_now() { todo!() }

pub struct JoinHandle<T>;

impl<T> JoinHandle<T> {
    pub fn abort(&self) { todo!() }
    pub fn is_finished(&self) -> bool { todo!() }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut JoinHandle<T>>, cx: &mut std::task::Context<'_>) -> Poll<Result<T, JoinError>> { todo!() }
}

pub struct JoinError;

impl Debug for JoinError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for JoinError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for JoinError {}
