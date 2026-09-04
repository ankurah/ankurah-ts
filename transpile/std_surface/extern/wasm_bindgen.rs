//! `wasm-bindgen` 0.2.106
//!
//! `JsValue` is the browser boundary. `JsCast` is where `dyn_into`,
//! `unchecked_into`, `unchecked_ref` and `dyn_ref` live — the oracle recorded
//! four `JsCast::dyn_into` resolutions, all reaching the trait's own
//! declaration rather than an impl, so the trait's signature is the answer.

pub struct JsValue;

impl JsValue {
    pub const NULL: JsValue = JsValue;
    pub const UNDEFINED: JsValue = JsValue;
    pub const TRUE: JsValue = JsValue;
    pub const FALSE: JsValue = JsValue;

    pub fn from_str(s: &str) -> JsValue { todo!() }
    pub fn from_f64(n: f64) -> JsValue { todo!() }
    pub fn from_bool(b: bool) -> JsValue { todo!() }
    pub fn null() -> JsValue { todo!() }
    pub fn undefined() -> JsValue { todo!() }
    pub fn symbol(description: Option<&str>) -> JsValue { todo!() }

    pub fn as_string(&self) -> Option<String> { todo!() }
    pub fn as_f64(&self) -> Option<f64> { todo!() }
    pub fn as_bool(&self) -> Option<bool> { todo!() }

    pub fn is_null(&self) -> bool { todo!() }
    pub fn is_undefined(&self) -> bool { todo!() }
    pub fn is_object(&self) -> bool { todo!() }
    pub fn is_array(&self) -> bool { todo!() }
    pub fn is_function(&self) -> bool { todo!() }
    pub fn is_string(&self) -> bool { todo!() }
    pub fn is_truthy(&self) -> bool { todo!() }
    pub fn is_falsy(&self) -> bool { todo!() }
    pub fn js_typeof(&self) -> JsValue { todo!() }
    pub fn dyn_into<T: JsCast>(self) -> Result<T, JsValue> { todo!() }
    pub fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    pub fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    pub fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
}

impl Clone for JsValue { fn clone(&self) -> JsValue { todo!() } }
impl Debug for JsValue { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for JsValue { fn eq(&self, other: &JsValue) -> bool { todo!() } }
impl Default for JsValue { fn default() -> JsValue { todo!() } }
impl From<&str> for JsValue { fn from(s: &str) -> JsValue { todo!() } }
impl From<String> for JsValue { fn from(s: String) -> JsValue { todo!() } }
impl From<&String> for JsValue { fn from(s: &String) -> JsValue { todo!() } }
impl From<f64> for JsValue { fn from(n: f64) -> JsValue { todo!() } }
impl From<u32> for JsValue { fn from(n: u32) -> JsValue { todo!() } }
impl From<i32> for JsValue { fn from(n: i32) -> JsValue { todo!() } }
impl From<bool> for JsValue { fn from(b: bool) -> JsValue { todo!() } }
impl From<JsError> for JsValue { fn from(e: JsError) -> JsValue { todo!() } }

/// `JsCast` is `wasm_bindgen`'s downcast between JS wrapper types. `dyn_into`
/// checks with `instanceof` and returns the original value on failure;
/// `unchecked_into` skips the check.
pub trait JsCast: AsRef<JsValue> + Into<JsValue> {
    fn instanceof(val: &JsValue) -> bool;
    fn is_type_of(val: &JsValue) -> bool;
    fn unchecked_from_js(val: JsValue) -> Self;
    fn unchecked_from_js_ref(val: &JsValue) -> &Self;

    fn has_type<T: JsCast>(&self) -> bool;
    fn dyn_into<T: JsCast>(self) -> Result<T, Self> where Self: Sized;
    fn dyn_ref<T: JsCast>(&self) -> Option<&T>;
    fn unchecked_into<T: JsCast>(self) -> T where Self: Sized;
    fn unchecked_ref<T: JsCast>(&self) -> &T;
    fn is_instance_of<T: JsCast>(&self) -> bool;
}

impl JsCast for JsValue {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> JsValue { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &JsValue { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, JsValue> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl AsRef<JsValue> for JsValue { fn as_ref(&self) -> &JsValue { todo!() } }

pub struct JsError;

impl JsError {
    pub fn new(s: &str) -> JsError { todo!() }
}

/// `Closure<T>` owns a JS callback and frees it when dropped — the leak the
/// ownership memo cares about. `Closure::wrap` takes an already-boxed closure;
/// `Closure::new` boxes for you. `as_ref()` yields the `JsValue` the browser
/// APIs take, which is why `closure.as_ref().unchecked_ref()` is the idiom at
/// every `set_on*` and `add_event_listener_with_callback` site.
///
/// The `WasmClosure` family is what confines `T` to shapes wasm-bindgen can
/// actually build a JS function from, and what lets the closure's argument and
/// return types be inferred at a `Closure::new(|e: Event| ..)` site.
pub trait WasmClosure {}
pub trait IntoWasmClosure<T: ?Sized> {}
pub trait WasmClosureFnOnce<A, R> {}

pub struct Closure<T: ?Sized + WasmClosure>;

impl<T: ?Sized + WasmClosure> Closure<T> {
    pub fn wrap(data: Box<T>) -> Closure<T> { todo!() }
    pub fn new<F: IntoWasmClosure<T> + 'static>(t: F) -> Closure<T> { todo!() }
    pub fn forget(self) { todo!() }
    pub fn into_js_value(self) -> JsValue { todo!() }
    pub fn as_ref(&self) -> &JsValue { todo!() }
}

impl Closure<dyn FnOnce()> {
    /// Consumes the closure and hands JS a value that frees itself after one
    /// call — `navigator_lock.rs` uses it so the lock callback is not leaked.
    pub fn once_into_js<F: WasmClosureFnOnce<A, R>, A, R>(fn_once: F) -> JsValue { todo!() }
    pub fn once<F: WasmClosureFnOnce<A, R>, A, R>(fn_once: F) -> Closure<dyn FnOnce()> { todo!() }
}

impl<A, R> WasmClosure for dyn FnMut(A) -> R {}
impl<A, B, R> WasmClosure for dyn FnMut(A, B) -> R {}
impl<R> WasmClosure for dyn FnMut() -> R {}
impl WasmClosure for dyn FnOnce() {}
impl<F: FnMut(A) -> R, A, R> IntoWasmClosure<dyn FnMut(A) -> R> for F {}
impl<F: FnMut() -> R, R> IntoWasmClosure<dyn FnMut() -> R> for F {}
impl<F: FnOnce() -> R, R> WasmClosureFnOnce<(), R> for F {}

impl<T: ?Sized + WasmClosure> AsRef<JsValue> for Closure<T> {
    fn as_ref(&self) -> &JsValue { todo!() }
}

impl<T: ?Sized + WasmClosure> Drop for Closure<T> {
    fn drop(&mut self) { todo!() }
}

pub mod prelude {
    pub use super::{Closure, JsCast, JsError, JsValue, UnwrapThrowExt};
}

pub mod closure {
    pub use super::Closure;
}

pub mod convert {
    pub trait FromWasmAbi {}
    pub trait IntoWasmAbi {}
    pub trait RefFromWasmAbi {}
}

/// The real name is `UnwrapThrowExt`, and it is a trait, not a struct. An
/// earlier version of this file declared a struct by that name plus an
/// unrelated `UnwrapThrow<T>` trait with no impls, so every real
/// `.unwrap_throw()` failed lookup against a trait that does not exist.
pub trait UnwrapThrowExt<T>: Sized {
    fn unwrap_throw(self) -> T;
    fn expect_throw(self, message: &str) -> T;
}

impl<T> UnwrapThrowExt<T> for Option<T> {
    fn unwrap_throw(self) -> T { todo!() }
    fn expect_throw(self, message: &str) -> T { todo!() }
}

impl<T, E: Debug> UnwrapThrowExt<T> for Result<T, E> {
    fn unwrap_throw(self) -> T { todo!() }
    fn expect_throw(self, message: &str) -> T { todo!() }
}
