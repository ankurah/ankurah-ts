//! `yrs` 0.24.0 (locked; `Cargo.lock` confirms it) — the Y-CRDT surface `core/src/property/backend/yrs.rs` uses.
//!
//! Not on the deliverable's list, and the reason it is here is worth stating:
//! `transpile.toml [provided_redirect.yrs]` already maps `Doc`, `Text` and
//! `Map` to `@ankurah/base/yrs-compat`. That redirect renames types for
//! emission; it says nothing about signatures, and the engine still has to type
//! `doc.transact().state_vector()` one hop at a time. Without these
//! declarations that chain is ten unresolvable calls in the middle of the
//! LWW/Yrs property backend.
//!
//! The trait split is yrs's own and it matters: `transact` and `transact_mut`
//! are on `Transact`, the reads are on `ReadTxn`, `get_or_insert_text` is on
//! `WriteTxn`, and `get_string` is on `GetString`. `properties()` in the corpus
//! even calls `Transact::transact(&self.doc)` in fully-qualified form, which
//! only resolves if the trait is declared.

pub struct Doc;

impl Doc {
    pub fn new() -> Doc { todo!() }
    pub fn with_client_id(client_id: u64) -> Doc { todo!() }
    pub fn client_id(&self) -> u64 { todo!() }
}

impl Clone for Doc { fn clone(&self) -> Doc { todo!() } }
impl Debug for Doc { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for Doc { fn default() -> Doc { todo!() } }

pub trait Transact {
    fn transact(&self) -> Transaction<'_>;
    fn transact_mut(&self) -> TransactionMut<'_>;
    fn try_transact(&self) -> Result<Transaction<'_>, TransactionAcqError>;
    fn try_transact_mut(&self) -> Result<TransactionMut<'_>, TransactionAcqError>;
}

impl Transact for Doc {
    fn transact(&self) -> Transaction<'_> { todo!() }
    fn transact_mut(&self) -> TransactionMut<'_> { todo!() }
    fn try_transact(&self) -> Result<Transaction<'_>, TransactionAcqError> { todo!() }
    fn try_transact_mut(&self) -> Result<TransactionMut<'_>, TransactionAcqError> { todo!() }
}

pub trait ReadTxn: Sized {
    fn state_vector(&self) -> StateVector;
    fn get_text(&self, name: &str) -> Option<TextRef>;
    fn get_map(&self, name: &str) -> Option<MapRef>;
    fn get_array(&self, name: &str) -> Option<ArrayRef>;
    fn root_refs(&self) -> RootRefs<'_>;
    fn encode_state_as_update_v1(&self, sv: &StateVector) -> Vec<u8>;
    fn encode_state_as_update_v2(&self, sv: &StateVector) -> Vec<u8>;
    fn encode_diff_v1(&self, sv: &StateVector) -> Vec<u8>;
    fn encode_diff_v2(&self, sv: &StateVector) -> Vec<u8>;
}

// 0.24 takes the root name as `N: Into<Arc<str>>`, so `&str`, `String` and an
// already-shared `Arc<str>` all work without an allocation at the call site.
pub trait WriteTxn: Sized {
    fn get_or_insert_text<N: Into<Arc<str>>>(&mut self, name: N) -> TextRef;
    fn get_or_insert_map<N: Into<Arc<str>>>(&mut self, name: N) -> MapRef;
    fn get_or_insert_array<N: Into<Arc<str>>>(&mut self, name: N) -> ArrayRef;
}

pub struct Transaction<'doc>;
pub struct TransactionMut<'doc>;
pub struct TransactionAcqError;

impl<'doc> ReadTxn for Transaction<'doc> {
    fn state_vector(&self) -> StateVector { todo!() }
    fn get_text(&self, name: &str) -> Option<TextRef> { todo!() }
    fn get_map(&self, name: &str) -> Option<MapRef> { todo!() }
    fn get_array(&self, name: &str) -> Option<ArrayRef> { todo!() }
    fn root_refs(&self) -> RootRefs<'_> { todo!() }
    fn encode_state_as_update_v1(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_state_as_update_v2(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_diff_v1(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_diff_v2(&self, sv: &StateVector) -> Vec<u8> { todo!() }
}

impl<'doc> ReadTxn for TransactionMut<'doc> {
    fn state_vector(&self) -> StateVector { todo!() }
    fn get_text(&self, name: &str) -> Option<TextRef> { todo!() }
    fn get_map(&self, name: &str) -> Option<MapRef> { todo!() }
    fn get_array(&self, name: &str) -> Option<ArrayRef> { todo!() }
    fn root_refs(&self) -> RootRefs<'_> { todo!() }
    fn encode_state_as_update_v1(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_state_as_update_v2(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_diff_v1(&self, sv: &StateVector) -> Vec<u8> { todo!() }
    fn encode_diff_v2(&self, sv: &StateVector) -> Vec<u8> { todo!() }
}

impl<'doc> WriteTxn for TransactionMut<'doc> {
    fn get_or_insert_text<N: Into<Arc<str>>>(&mut self, name: N) -> TextRef { todo!() }
    fn get_or_insert_map<N: Into<Arc<str>>>(&mut self, name: N) -> MapRef { todo!() }
    fn get_or_insert_array<N: Into<Arc<str>>>(&mut self, name: N) -> ArrayRef { todo!() }
}

impl<'doc> TransactionMut<'doc> {
    pub fn apply_update(&mut self, update: Update) -> Result<(), UpdateError> { todo!() }
    pub fn commit(&mut self) { todo!() }
}

impl Doc {
    pub fn get_or_insert_text<N: Into<Arc<str>>>(&self, name: N) -> TextRef { todo!() }
    pub fn get_or_insert_map<N: Into<Arc<str>>>(&self, name: N) -> MapRef { todo!() }
    pub fn get_or_insert_array<N: Into<Arc<str>>>(&self, name: N) -> ArrayRef { todo!() }
}

pub struct RootRefs<'a>;

impl<'a> Iterator for RootRefs<'a> {
    type Item = (&'a str, Out);
    fn next(&mut self) -> Option<(&'a str, Out)> { todo!() }
}

pub struct StateVector;

impl StateVector {
    pub fn default() -> StateVector { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
}

impl Clone for StateVector { fn clone(&self) -> StateVector { todo!() } }
impl Debug for StateVector { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for StateVector { fn default() -> StateVector { todo!() } }

pub struct TextRef;
pub struct MapRef;
pub struct ArrayRef;
pub struct Out;

impl AsRef<Branch> for TextRef { fn as_ref(&self) -> &Branch { todo!() } }
impl AsRef<Branch> for MapRef { fn as_ref(&self) -> &Branch { todo!() } }
impl AsRef<Branch> for ArrayRef { fn as_ref(&self) -> &Branch { todo!() } }
impl Clone for TextRef { fn clone(&self) -> TextRef { todo!() } }
impl Clone for MapRef { fn clone(&self) -> MapRef { todo!() } }

/// `Branch` is yrs's shared-type root. `Text` and `Observable` are both bounded
/// on `AsRef<Branch>` in 0.24, which is what ties a `TextRef` back to the
/// document it came from.
pub struct Branch;

pub trait Text: AsRef<Branch> + Sized {
    fn insert(&self, txn: &mut TransactionMut<'_>, index: u32, chunk: &str);
    fn remove_range(&self, txn: &mut TransactionMut<'_>, index: u32, len: u32);
    fn push(&self, txn: &mut TransactionMut<'_>, chunk: &str);
    fn len<T: ReadTxn>(&self, txn: &T) -> u32;
}

impl Text for TextRef {
    fn insert(&self, txn: &mut TransactionMut<'_>, index: u32, chunk: &str) { todo!() }
    fn remove_range(&self, txn: &mut TransactionMut<'_>, index: u32, len: u32) { todo!() }
    fn push(&self, txn: &mut TransactionMut<'_>, chunk: &str) { todo!() }
    fn len<T: ReadTxn>(&self, txn: &T) -> u32 { todo!() }
}

pub trait GetString {
    fn get_string<T: ReadTxn>(&self, txn: &T) -> String;
}

impl GetString for TextRef {
    fn get_string<T: ReadTxn>(&self, txn: &T) -> String { todo!() }
}

pub trait Observable: AsRef<Branch> {
    type Event;
    fn observe<F: Fn(&TransactionMut<'_>, &Self::Event) + 'static>(&self, f: F) -> Subscription;
}

impl Observable for TextRef {
    type Event = TextEvent;
    fn observe<F: Fn(&TransactionMut<'_>, &TextEvent) + 'static>(&self, f: F) -> Subscription { todo!() }
}

pub struct TextEvent;
pub struct Subscription;

impl Drop for Subscription { fn drop(&mut self) { todo!() } }

pub struct Update;

impl Update {
    /// 0.24 mutates in place and returns nothing; the pre-0.22 form returned a
    /// merged `Update`. `Cargo.lock` pins 0.24.0, so this is the shape.
    pub fn merge(&mut self, other: Update) { todo!() }
}

pub struct UpdateError;

impl Debug for UpdateError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for UpdateError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for UpdateError {}

pub mod updates {
    pub mod decoder {
        pub trait Decode: Sized {
            fn decode_v1(data: &[u8]) -> Result<Self, super::super::EncodingError>;
            fn decode_v2(data: &[u8]) -> Result<Self, super::super::EncodingError>;
        }

        impl Decode for Update {
            fn decode_v1(data: &[u8]) -> Result<Update, super::super::EncodingError> { todo!() }
            fn decode_v2(data: &[u8]) -> Result<Update, super::super::EncodingError> { todo!() }
        }

        impl Decode for StateVector {
            fn decode_v1(data: &[u8]) -> Result<StateVector, super::super::EncodingError> { todo!() }
            fn decode_v2(data: &[u8]) -> Result<StateVector, super::super::EncodingError> { todo!() }
        }
    }

    pub mod encoder {
        pub trait Encode {
            fn encode_v1(&self) -> Vec<u8>;
            fn encode_v2(&self) -> Vec<u8>;
        }

        impl Encode for StateVector {
            fn encode_v1(&self) -> Vec<u8> { todo!() }
            fn encode_v2(&self) -> Vec<u8> { todo!() }
        }
    }
}

pub struct EncodingError;

impl Debug for EncodingError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for EncodingError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for EncodingError {}
