//! `serde` 1.0.228
//!
//! `#[derive(Serialize, Deserialize)]` is handled by the derive hook; what the
//! engine needs from serde itself is the two traits, so that a hand-written
//! `impl Serialize for EntityId` in the corpus has a trait to implement and
//! `is_human_readable` and `serialize_str` have signatures. The `Serializer`
//! and `Deserializer` surfaces are cut to the methods `proto` actually calls.

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>;
}

pub trait Deserialize<'de>: Sized {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>;
}

pub trait DeserializeOwned: for<'de> Deserialize<'de> {}

// Without this blanket, `serde_json::from_str`, `bincode::deserialize` and
// `serde_wasm_bindgen::from_value` reject every ordinary deserializable type:
// their `T: DeserializeOwned` bound has nothing to discharge against.
impl<T> DeserializeOwned for T where T: for<'de> Deserialize<'de> {}

pub trait Serializer: Sized {
    type Ok;
    type Error: ser::Error;
    type SerializeSeq: ser::SerializeSeq<Ok = Self::Ok, Error = Self::Error>;
    type SerializeTuple: ser::SerializeTuple<Ok = Self::Ok, Error = Self::Error>;
    type SerializeMap: ser::SerializeMap<Ok = Self::Ok, Error = Self::Error>;
    type SerializeStruct: ser::SerializeStruct<Ok = Self::Ok, Error = Self::Error>;

    fn is_human_readable(&self) -> bool;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error>;
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error>;
    fn serialize_none(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error>;
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error>;
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error>;
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error>;
    fn collect_str<T: ?Sized + std::fmt::Display>(self, value: &T) -> Result<Self::Ok, Self::Error>;
}

pub trait Deserializer<'de>: Sized {
    type Error: de::Error;

    fn is_human_readable(&self) -> bool;
    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_byte_buf<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<<V as de::Visitor<'de>>::Value, Self::Error>;
}

pub mod ser {
    pub trait Error: Sized + std::error::Error {
        fn custom<T: std::fmt::Display>(msg: T) -> Self;
    }

    pub trait SerializeSeq {
        type Ok;
        type Error: Error;
        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error>;
        fn end(self) -> Result<Self::Ok, Self::Error>;
    }

    pub trait SerializeTuple {
        type Ok;
        type Error: Error;
        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error>;
        fn end(self) -> Result<Self::Ok, Self::Error>;
    }

    pub trait SerializeMap {
        type Ok;
        type Error: Error;
        fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error>;
        fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error>;
        fn end(self) -> Result<Self::Ok, Self::Error>;
    }

    pub trait SerializeStruct {
        type Ok;
        type Error: Error;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>;
        fn end(self) -> Result<Self::Ok, Self::Error>;
    }
}

pub mod de {
    pub trait Error: Sized + std::error::Error {
        fn custom<T: std::fmt::Display>(msg: T) -> Self;
        fn invalid_length(len: usize, exp: &dyn Expected) -> Self;
    }

    pub trait Expected {}

    pub trait Visitor<'de>: Sized {
        type Value;
        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result;
        fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E>;
        fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E>;
        fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E>;
        fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E>;
        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E>;
        fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E>;
        fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E>;
        fn visit_none<E: Error>(self) -> Result<Self::Value, E>;
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>;
        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, <A as SeqAccess<'de>>::Error>;
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, <A as MapAccess<'de>>::Error>;
    }

    pub trait SeqAccess<'de> {
        type Error: Error;
        fn next_element<T: Deserialize<'de>>(&mut self) -> Result<Option<T>, Self::Error>;
        fn size_hint(&self) -> Option<usize>;
    }

    pub trait MapAccess<'de> {
        type Error: Error;
        fn next_key<K: Deserialize<'de>>(&mut self) -> Result<Option<K>, Self::Error>;
        fn next_value<V: Deserialize<'de>>(&mut self) -> Result<V, Self::Error>;
    }
}

// The primitive and container impls `T::deserialize(d)` resolves against. Only
// the ones the corpus names directly are written out.
impl Serialize for String { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for str { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for bool { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for u8 { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for u64 { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for i64 { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for usize { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl Serialize for f64 { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl<T: Serialize> Serialize for Vec<T> { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl<T: Serialize> Serialize for Option<T> { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl<T: Serialize + ?Sized> Serialize for &T { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl<K: Serialize, V: Serialize> Serialize for BTreeMap<K, V> { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }
impl<K: Serialize, V: Serialize> Serialize for HashMap<K, V> { fn serialize<S: Serializer>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> { todo!() } }

impl<'de> Deserialize<'de> for String { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<String, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for bool { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<bool, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for u8 { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<u8, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for u64 { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<u64, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for i64 { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<i64, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for usize { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<usize, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de> Deserialize<'de> for f64 { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<f64, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec<T> { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Vec<T>, <D as Deserializer<'de>>::Error> { todo!() } }
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Option<T> { fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Option<T>, <D as Deserializer<'de>>::Error> { todo!() } }
