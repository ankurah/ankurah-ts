//! `ulid` 1.2.1
//!
//! `EntityId`, `EventId`, `TransactionId`, `RequestId`, `QueryId` and
//! `UpdateId` all wrap a `Ulid`; `transpile.toml [provided_impls]` hands the
//! wrappers to hand-written TypeScript today, but the wrapper types' own
//! methods still call through to these.

pub struct Ulid(pub u128);

impl Ulid {
    pub fn new() -> Ulid { todo!() }
    pub fn nil() -> Ulid { todo!() }
    pub fn from_parts(timestamp_ms: u64, random: u128) -> Ulid { todo!() }
    pub fn from_bytes(bytes: [u8; 16]) -> Ulid { todo!() }
    pub fn from_string(encoded: &str) -> Result<Ulid, ulid::DecodeError> { todo!() }
    pub fn to_bytes(&self) -> [u8; 16] { todo!() }
    pub fn timestamp_ms(&self) -> u64 { todo!() }
    pub fn random(&self) -> u128 { todo!() }
    pub fn is_nil(&self) -> bool { todo!() }
}

impl Clone for Ulid { fn clone(&self) -> Ulid { todo!() } }
impl Copy for Ulid {}
impl Debug for Ulid { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for Ulid { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for Ulid { fn default() -> Ulid { todo!() } }
impl PartialEq for Ulid { fn eq(&self, other: &Ulid) -> bool { todo!() } }
impl Eq for Ulid {}
impl PartialOrd for Ulid { fn partial_cmp(&self, other: &Ulid) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for Ulid { fn cmp(&self, other: &Ulid) -> std::cmp::Ordering { todo!() } }
impl Hash for Ulid { fn hash<H: Hasher>(&self, state: &mut H) { todo!() } }
impl FromStr for Ulid { type Err = ulid::DecodeError; fn from_str(s: &str) -> Result<Ulid, ulid::DecodeError> { todo!() } }
impl From<u128> for Ulid { fn from(value: u128) -> Ulid { todo!() } }
impl From<Ulid> for u128 { fn from(value: Ulid) -> u128 { todo!() } }
impl Serialize for Ulid { fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error> { todo!() } }
impl<'de> Deserialize<'de> for Ulid { fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Ulid, <D as serde::Deserializer<'de>>::Error> { todo!() } }

pub enum DecodeError {
    InvalidLength,
    InvalidChar,
}

impl Debug for DecodeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::fmt::Display for DecodeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for DecodeError {}
