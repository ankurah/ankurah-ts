//! `serde_json` 1.0.145
//!
//! Not on the deliverable's list. `core/src/property/value/json.rs`,
//! `core/src/collation.rs` and `ankql`'s literal handling all walk a
//! `serde_json::Value`, and `as_i64`, `as_f64`, `as_str`, `is_object` and
//! `is_array` in the corpus's method inventory are that type's, not std's.

pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

impl Value {
    pub fn get<I: serde_json::Index>(&self, index: I) -> Option<&Value> { todo!() }
    pub fn get_mut<I: serde_json::Index>(&mut self, index: I) -> Option<&mut Value> { todo!() }
    pub fn pointer(&self, pointer: &str) -> Option<&Value> { todo!() }

    pub fn is_null(&self) -> bool { todo!() }
    pub fn is_boolean(&self) -> bool { todo!() }
    pub fn is_number(&self) -> bool { todo!() }
    pub fn is_string(&self) -> bool { todo!() }
    pub fn is_array(&self) -> bool { todo!() }
    pub fn is_object(&self) -> bool { todo!() }
    pub fn is_i64(&self) -> bool { todo!() }
    pub fn is_u64(&self) -> bool { todo!() }
    pub fn is_f64(&self) -> bool { todo!() }

    pub fn as_null(&self) -> Option<()> { todo!() }
    pub fn as_bool(&self) -> Option<bool> { todo!() }
    pub fn as_str(&self) -> Option<&str> { todo!() }
    pub fn as_i64(&self) -> Option<i64> { todo!() }
    pub fn as_u64(&self) -> Option<u64> { todo!() }
    pub fn as_f64(&self) -> Option<f64> { todo!() }
    pub fn as_array(&self) -> Option<&Vec<Value>> { todo!() }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> { todo!() }
    pub fn as_object(&self) -> Option<&Map<String, Value>> { todo!() }
    pub fn as_object_mut(&mut self) -> Option<&mut Map<String, Value>> { todo!() }

    pub fn take(&mut self) -> Value { todo!() }
}

impl Clone for Value { fn clone(&self) -> Value { todo!() } }
impl Debug for Value { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Value { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for Value { fn default() -> Value { todo!() } }
impl PartialEq for Value { fn eq(&self, other: &Value) -> bool { todo!() } }
impl FromStr for Value { type Err = Error; fn from_str(s: &str) -> Result<Value, Error> { todo!() } }
impl From<String> for Value { fn from(s: String) -> Value { todo!() } }
impl From<&str> for Value { fn from(s: &str) -> Value { todo!() } }
impl From<bool> for Value { fn from(b: bool) -> Value { todo!() } }
impl From<i64> for Value { fn from(n: i64) -> Value { todo!() } }
impl From<f64> for Value { fn from(n: f64) -> Value { todo!() } }
impl<T: Into<Value>> From<Vec<T>> for Value { fn from(v: Vec<T>) -> Value { todo!() } }
impl Serialize for Value { fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error> { todo!() } }
impl<'de> Deserialize<'de> for Value { fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Value, <D as serde::Deserializer<'de>>::Error> { todo!() } }

pub trait Index {}

impl Index for usize {}
impl Index for str {}
impl<'a> Index for &'a str {}
impl Index for String {}

pub struct Number;

impl Number {
    pub fn is_i64(&self) -> bool { todo!() }
    pub fn is_u64(&self) -> bool { todo!() }
    pub fn is_f64(&self) -> bool { todo!() }
    pub fn as_i64(&self) -> Option<i64> { todo!() }
    pub fn as_u64(&self) -> Option<u64> { todo!() }
    pub fn as_f64(&self) -> Option<f64> { todo!() }
    pub fn from_f64(f: f64) -> Option<Number> { todo!() }
}

impl Clone for Number { fn clone(&self) -> Number { todo!() } }
impl Debug for Number { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Number { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for Number { fn eq(&self, other: &Number) -> bool { todo!() } }
impl From<i64> for Number { fn from(n: i64) -> Number { todo!() } }
impl From<u64> for Number { fn from(n: u64) -> Number { todo!() } }

pub struct Map<K, V>;

impl Map<String, Value> {
    pub fn new() -> Map<String, Value> { todo!() }
    pub fn get(&self, key: &str) -> Option<&Value> { todo!() }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> { todo!() }
    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> { todo!() }
    pub fn remove(&mut self, key: &str) -> Option<Value> { todo!() }
    pub fn contains_key(&self, key: &str) -> bool { todo!() }
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn keys(&self) -> Keys<'_> { todo!() }
    pub fn values(&self) -> Values<'_> { todo!() }
    pub fn iter(&self) -> serde_json::Iter<'_> { todo!() }
}

pub struct Keys<'a>;
pub struct Values<'a>;
pub struct Iter<'a>;

impl<'a> Iterator for Keys<'a> { type Item = &'a String; fn next(&mut self) -> Option<&'a String> { todo!() } }
impl<'a> Iterator for Values<'a> { type Item = &'a Value; fn next(&mut self) -> Option<&'a Value> { todo!() } }
impl<'a> Iterator for Iter<'a> { type Item = (&'a String, &'a Value); fn next(&mut self) -> Option<(&'a String, &'a Value)> { todo!() } }

pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, Error> { todo!() }
pub fn from_slice<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T, Error> { todo!() }
pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, Error> { todo!() }
pub fn to_string<T: ?Sized + Serialize>(value: &T) -> Result<String, Error> { todo!() }
pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> Result<String, Error> { todo!() }
pub fn to_vec<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>, Error> { todo!() }
pub fn to_value<T: Serialize>(value: T) -> Result<Value, Error> { todo!() }

pub struct Error;

impl Error {
    pub fn line(&self) -> usize { todo!() }
    pub fn column(&self) -> usize { todo!() }
    pub fn is_syntax(&self) -> bool { todo!() }
    pub fn is_data(&self) -> bool { todo!() }
}

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for Error {}
