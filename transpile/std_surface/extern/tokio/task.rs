//! `tokio::task` and `tokio::spawn`, tokio 1.48.0.
//!
//! `core/src/task.rs` calls `tokio::spawn` on the non-wasm branch and
//! `wasm_bindgen_futures::spawn_local` on the wasm one. The port's cfg is the
//! wasm32 configuration, so this surface is here for the branch the cfg
//! evaluator currently keeps, not because the browser build reaches it.
//!
//! ## What this file leaves undeclared on purpose
//!
//! `port/ownership.md` names the tokio surface the browser target does not
//! provide, and asks the transpiler to refuse a call to one rather than write a
//! call that resolves here and then finds nothing at the other end. A
//! declaration is what makes such a call resolve, so this one is left out and
//! the call is reported where it is written:
//!
//! - **`spawn_blocking`.** tokio declares
//!   `pub fn spawn_blocking<F: FnOnce() -> R + Send + 'static, R: Send + 'static>(f: F) -> JoinHandle<R>`.
//!   A browser has no thread pool to move the closure onto, and running it
//!   inline would block the event loop — the one thing the call exists to
//!   avoid. The corpus calls it from the native sqlite and sled engines, which
//!   the browser build does not reach.
//!
//! What is missing is the *reason* travelling with the refusal: the engine
//! reports this as an undeclared name, which reads as "the surface is
//! incomplete". Carrying the sentence above to the call site needs the resolver
//! to know which absences are deliberate.

pub fn spawn<F: Future + Send + 'static>(future: F) -> JoinHandle<<F as Future>::Output>
where <F as Future>::Output: Send + 'static { todo!() }

// `spawn_blocking` is left undeclared; see the note at the top of this file.

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
impl std::fmt::Display for JoinError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for JoinError {}
