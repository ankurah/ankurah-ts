//! `wasm-bindgen-futures` 0.4.56
//!
//! `core/src/task.rs` spawns with `spawn_local` under the wasm cfg, which is
//! the port's configuration; `JsFuture::from(promise).await` is how every
//! IndexedDB request and navigator lock becomes an `await` in the corpus.

pub fn spawn_local<F: Future<Output = ()> + 'static>(future: F) { todo!() }

// `JsFuture::from(promise)` is the `From` impl below, not an inherent method.
// The call syntax and the return type coincide, but the provenance does not,
// and the engine records which trait a call resolved through.
pub struct JsFuture;

impl Future for JsFuture {
    type Output = Result<JsValue, JsValue>;
    fn poll(self: Pin<&mut JsFuture>, cx: &mut std::task::Context<'_>) -> Poll<Result<JsValue, JsValue>> { todo!() }
}

impl From<js_sys::Promise> for JsFuture {
    fn from(promise: js_sys::Promise) -> JsFuture { todo!() }
}

pub fn future_to_promise<F: Future<Output = Result<JsValue, JsValue>> + 'static>(future: F) -> js_sys::Promise { todo!() }
