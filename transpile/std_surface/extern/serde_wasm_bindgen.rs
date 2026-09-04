//! `serde-wasm-bindgen` 0.6.5
//!
//! Not on the deliverable's list. Its `Serializer` builder is how
//! `core/src/property/value/json.rs` gets plain JS objects instead of `Map`
//! instances out of a `serde_json::Value`, and each builder method returns the
//! serializer, so the chain has to be typeable.

pub struct Serializer;

impl Serializer {
    pub fn new() -> Serializer { todo!() }
    pub fn json_compatible() -> Serializer { todo!() }
    pub fn serialize_maps_as_objects(self, value: bool) -> Serializer { todo!() }
    pub fn serialize_missing_as_null(self, value: bool) -> Serializer { todo!() }
    pub fn serialize_large_number_types_as_bigints(self, value: bool) -> Serializer { todo!() }
    pub fn serialize_bytes_as_arrays(self, value: bool) -> Serializer { todo!() }
}

impl Default for Serializer { fn default() -> Serializer { todo!() } }

// `value.serialize(&serializer)` is the corpus idiom, and it resolves only
// because `serde::Serializer` is implemented for `&Serializer`, not for
// `Serializer`.
impl serde::Serializer for &Serializer {
    type Ok = JsValue;
    type Error = Error;
    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeTuple;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeStruct;

    fn is_human_readable(&self) -> bool { todo!() }
    fn serialize_bool(self, v: bool) -> Result<JsValue, Error> { todo!() }
    fn serialize_i32(self, v: i32) -> Result<JsValue, Error> { todo!() }
    fn serialize_i64(self, v: i64) -> Result<JsValue, Error> { todo!() }
    fn serialize_u8(self, v: u8) -> Result<JsValue, Error> { todo!() }
    fn serialize_u32(self, v: u32) -> Result<JsValue, Error> { todo!() }
    fn serialize_u64(self, v: u64) -> Result<JsValue, Error> { todo!() }
    fn serialize_f64(self, v: f64) -> Result<JsValue, Error> { todo!() }
    fn serialize_str(self, v: &str) -> Result<JsValue, Error> { todo!() }
    fn serialize_bytes(self, v: &[u8]) -> Result<JsValue, Error> { todo!() }
    fn serialize_none(self) -> Result<JsValue, Error> { todo!() }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<JsValue, Error> { todo!() }
    fn serialize_unit(self) -> Result<JsValue, Error> { todo!() }
    fn serialize_seq(self, len: Option<usize>) -> Result<SerializeSeq, Error> { todo!() }
    fn serialize_tuple(self, len: usize) -> Result<SerializeTuple, Error> { todo!() }
    fn serialize_map(self, len: Option<usize>) -> Result<SerializeMap, Error> { todo!() }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<SerializeStruct, Error> { todo!() }
    fn collect_str<T: ?Sized + Display>(self, value: &T) -> Result<JsValue, Error> { todo!() }
}

pub struct SerializeSeq;
pub struct SerializeTuple;
pub struct SerializeMap;
pub struct SerializeStruct;

pub struct Deserializer;

impl From<JsValue> for Deserializer { fn from(value: JsValue) -> Deserializer { todo!() } }

impl<'de> serde::Deserializer<'de> for Deserializer {
    type Error = Error;
    fn is_human_readable(&self) -> bool { todo!() }
    fn deserialize_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_bool<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_i64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_u64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_f64<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_str<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_string<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_bytes<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_byte_buf<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_option<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_seq<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
    fn deserialize_map<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<<V as serde::de::Visitor<'de>>::Value, Error> { todo!() }
}

pub fn to_value<T: ?Sized + Serialize>(value: &T) -> Result<JsValue, Error> { todo!() }
pub fn from_value<T: DeserializeOwned>(value: JsValue) -> Result<T, Error> { todo!() }

pub struct Error;

impl Error {
    /// The inherent constructor is `new`, not `custom`. `custom` reaches this
    /// type only through the two serde `Error` traits below.
    pub fn new<T: Display>(msg: T) -> Error { todo!() }
}

impl From<JsValue> for Error { fn from(value: JsValue) -> Error { todo!() } }

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Error { todo!() }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Error { todo!() }
    fn invalid_length(len: usize, exp: &dyn serde::de::Expected) -> Error { todo!() }
}

impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for Error { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for Error {}
impl From<Error> for JsValue { fn from(e: Error) -> JsValue { todo!() } }
