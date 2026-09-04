//! `js-sys` 0.3.83 — the JS builtins the wasm crates reach.
//!
//! Every type here derefs to `JsValue`, which is how `array.is_undefined()`
//! and `object.as_string()` resolve without those methods being declared twice.

pub struct Object;

impl Object {
    pub fn new() -> Object { todo!() }
    pub fn keys(obj: &Object) -> Array { todo!() }
    pub fn values(obj: &Object) -> Array { todo!() }
    pub fn entries(obj: &Object) -> Array { todo!() }
    pub fn assign(target: &Object, source: &Object) -> Object { todo!() }
    pub fn is_extensible(object: &Object) -> bool { todo!() }
}

impl Deref for Object { type Target = JsValue; fn deref(&self) -> &JsValue { todo!() } }
impl AsRef<JsValue> for Object { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Object { fn clone(&self) -> Object { todo!() } }
impl Debug for Object { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for Object { fn default() -> Object { todo!() } }
pub struct Reflect;

impl Reflect {
    pub fn get(target: &JsValue, key: &JsValue) -> Result<JsValue, JsValue> { todo!() }
    pub fn set(target: &JsValue, key: &JsValue, value: &JsValue) -> Result<bool, JsValue> { todo!() }
    pub fn has(target: &JsValue, key: &JsValue) -> Result<bool, JsValue> { todo!() }
    pub fn delete_property(target: &Object, key: &JsValue) -> Result<bool, JsValue> { todo!() }
    pub fn own_keys(target: &JsValue) -> Result<Array, JsValue> { todo!() }
}

pub struct Array;

impl Array {
    pub fn new() -> Array { todo!() }
    pub fn new_with_length(len: u32) -> Array { todo!() }
    pub fn of2(a: &JsValue, b: &JsValue) -> Array { todo!() }
    pub fn get(&self, index: u32) -> JsValue { todo!() }
    pub fn set(&self, index: u32, value: JsValue) { todo!() }
    pub fn push(&self, value: &JsValue) -> u32 { todo!() }
    pub fn length(&self) -> u32 { todo!() }
    pub fn iter(&self) -> ArrayIter<'_> { todo!() }
    pub fn to_vec(&self) -> Vec<JsValue> { todo!() }
}

pub struct ArrayIter<'a>;
pub struct ArrayIntoIter;

impl<'a> Iterator for ArrayIter<'a> { type Item = JsValue; fn next(&mut self) -> Option<JsValue> { todo!() } }
impl<'a> DoubleEndedIterator for ArrayIter<'a> { fn next_back(&mut self) -> Option<JsValue> { todo!() } }
impl<'a> ExactSizeIterator for ArrayIter<'a> { fn len(&self) -> usize { todo!() } }
impl<'a> FusedIterator for ArrayIter<'a> {}
impl Iterator for ArrayIntoIter { type Item = JsValue; fn next(&mut self) -> Option<JsValue> { todo!() } }
impl DoubleEndedIterator for ArrayIntoIter { fn next_back(&mut self) -> Option<JsValue> { todo!() } }
impl ExactSizeIterator for ArrayIntoIter { fn len(&self) -> usize { todo!() } }

impl IntoIterator for Array {
    type Item = JsValue;
    type IntoIter = ArrayIntoIter;
    fn into_iter(self) -> ArrayIntoIter { todo!() }
}

impl<'a> IntoIterator for &'a Array {
    type Item = JsValue;
    type IntoIter = ArrayIter<'a>;
    fn into_iter(self) -> ArrayIter<'a> { todo!() }
}

impl<A: AsRef<JsValue>> FromIterator<A> for Array {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Array { todo!() }
}

impl<A: AsRef<JsValue>> Extend<A> for Array {
    fn extend<I: IntoIterator<Item = A>>(&mut self, iter: I) { todo!() }
}

impl Deref for Array { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Array { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Array { fn clone(&self) -> Array { todo!() } }

pub struct Uint8Array;

impl Uint8Array {
    pub fn new(constructor_arg: &JsValue) -> Uint8Array { todo!() }
    pub fn new_with_length(length: u32) -> Uint8Array { todo!() }
    pub fn length(&self) -> u32 { todo!() }
    pub fn buffer(&self) -> ArrayBuffer { todo!() }
    pub fn to_vec(&self) -> Vec<u8> { todo!() }
    pub fn copy_from(&self, src: &[u8]) { todo!() }
    pub fn copy_to(&self, dst: &mut [u8]) { todo!() }
    pub fn subarray(&self, begin: u32, end: u32) -> Uint8Array { todo!() }
    pub fn get_index(&self, idx: u32) -> u8 { todo!() }
    pub fn set_index(&self, idx: u32, value: u8) { todo!() }
}

impl Deref for Uint8Array { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Uint8Array { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Uint8Array { fn clone(&self) -> Uint8Array { todo!() } }

pub struct ArrayBuffer;

impl ArrayBuffer {
    pub fn new(length: u32) -> ArrayBuffer { todo!() }
    pub fn byte_length(&self) -> u32 { todo!() }
    pub fn slice(&self, begin: u32) -> ArrayBuffer { todo!() }
}

impl Deref for ArrayBuffer { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for ArrayBuffer { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for ArrayBuffer { fn clone(&self) -> ArrayBuffer { todo!() } }

pub struct Function;

impl Function {
    pub fn new_no_args(body: &str) -> Function { todo!() }
    pub fn call0(&self, context: &JsValue) -> Result<JsValue, JsValue> { todo!() }
    pub fn call1(&self, context: &JsValue, arg1: &JsValue) -> Result<JsValue, JsValue> { todo!() }
    pub fn call2(&self, context: &JsValue, arg1: &JsValue, arg2: &JsValue) -> Result<JsValue, JsValue> { todo!() }
    pub fn call3(&self, context: &JsValue, arg1: &JsValue, arg2: &JsValue, arg3: &JsValue) -> Result<JsValue, JsValue> { todo!() }
    pub fn apply(&self, context: &JsValue, args: &Array) -> Result<JsValue, JsValue> { todo!() }
    pub fn bind(&self, context: &JsValue) -> Function { todo!() }
    pub fn length(&self) -> u32 { todo!() }
}

impl Deref for Function { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Function { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Function { fn clone(&self) -> Function { todo!() } }

pub struct Promise;

impl Promise {
    pub fn new(cb: &mut dyn FnMut(Function, Function)) -> Promise { todo!() }
    pub fn resolve(value: &JsValue) -> Promise { todo!() }
    pub fn reject(value: &JsValue) -> Promise { todo!() }
    pub fn then(&self, cb: &Closure<dyn FnMut(JsValue)>) -> Promise { todo!() }
    pub fn catch(&self, cb: &Closure<dyn FnMut(JsValue)>) -> Promise { todo!() }
}

impl Deref for Promise { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Promise { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Promise { fn clone(&self) -> Promise { todo!() } }

pub struct JsString;

impl JsString {
    pub fn length(&self) -> u32 { todo!() }
    pub fn as_string(&self) -> Option<String> { todo!() }
}

impl Deref for JsString { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for JsString { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for JsString { fn clone(&self) -> JsString { todo!() } }
impl From<JsString> for String { fn from(s: JsString) -> String { todo!() } }
// `Uint8Array::from(&bytes)` and `JsString::from("s")` come from these, not
// from inherent methods.
impl<'a> From<&'a [u8]> for Uint8Array { fn from(slice: &'a [u8]) -> Uint8Array { todo!() } }
impl<'a> From<&'a str> for JsString { fn from(s: &'a str) -> JsString { todo!() } }
impl From<String> for JsString { fn from(s: String) -> JsString { todo!() } }

pub struct Error;

impl Error {
    pub fn new(message: &str) -> Error { todo!() }
    pub fn message(&self) -> JsString { todo!() }
    pub fn name(&self) -> JsString { todo!() }
    pub fn set_message(&self, message: &str) { todo!() }
}

impl Deref for Error { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Error { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Error { fn clone(&self) -> Error { todo!() } }

pub struct Date;

impl Date {
    pub fn new_0() -> Date { todo!() }
    pub fn now() -> f64 { todo!() }
    pub fn get_time(&self) -> f64 { todo!() }
}

impl Deref for Date { type Target = Object; fn deref(&self) -> &Object { todo!() } }

// ── Generated cast and conversion impls ─────────────────────────────────────
//
// wasm-bindgen generates one `JsCast` impl and one `From<Wrapper> for JsValue`
// per wrapper type. Both are load-bearing and neither is optional: `JsCast`'s
// own `Into<JsValue>` supertrait is discharged by the `From`, so a wrapper
// missing its `From` cannot satisfy `JsCast` even with the impl written out.

impl From<Object> for JsValue { fn from(value: Object) -> JsValue { todo!() } }
impl JsCast for Object {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Object { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Object { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Object> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Array> for JsValue { fn from(value: Array) -> JsValue { todo!() } }
impl JsCast for Array {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Array { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Array { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Array> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Uint8Array> for JsValue { fn from(value: Uint8Array) -> JsValue { todo!() } }
impl JsCast for Uint8Array {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Uint8Array { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Uint8Array { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Uint8Array> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<ArrayBuffer> for JsValue { fn from(value: ArrayBuffer) -> JsValue { todo!() } }
impl JsCast for ArrayBuffer {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> ArrayBuffer { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &ArrayBuffer { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, ArrayBuffer> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Function> for JsValue { fn from(value: Function) -> JsValue { todo!() } }
impl JsCast for Function {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Function { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Function { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Function> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Promise> for JsValue { fn from(value: Promise) -> JsValue { todo!() } }
impl JsCast for Promise {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Promise { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Promise { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Promise> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<JsString> for JsValue { fn from(value: JsString) -> JsValue { todo!() } }
impl JsCast for JsString {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> JsString { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &JsString { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, JsString> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Error> for JsValue { fn from(value: Error) -> JsValue { todo!() } }
impl JsCast for Error {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Error { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Error { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Error> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Date> for JsValue { fn from(value: Date) -> JsValue { todo!() } }
impl JsCast for Date {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Date { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Date { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Date> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}
