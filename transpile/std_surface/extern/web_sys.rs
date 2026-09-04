//! `web-sys` 0.3.83 — only the IndexedDB and WebSocket items the two wasm
//! crates use.
//!
//! web_sys is enormous and generated; the crates declare exactly which items
//! they want in their `Cargo.toml` feature lists, and this file follows those
//! lists rather than the whole API. Every fallible call returns
//! `Result<_, JsValue>` because that is what the generated bindings do, and
//! that is why `.require("...")` (the crate's own extension trait) and `?`
//! appear at nearly every call site.
//!
//! The `Deref` chains are the generated ones: `IdbOpenDbRequest` -> `IdbRequest`
//! -> `EventTarget` -> `Object` -> `JsValue`. The engine walks them to find
//! `add_event_listener_with_callback` on a request.

pub fn window() -> Option<Window> { todo!() }

pub struct Window;

impl Window {
    pub fn indexed_db(&self) -> Result<Option<IdbFactory>, JsValue> { todo!() }
    pub fn navigator(&self) -> Navigator { todo!() }
    pub fn location(&self) -> Location { todo!() }
    pub fn set_timeout_with_callback_and_timeout_and_arguments_0(&self, handler: &Function, timeout: i32) -> Result<i32, JsValue> { todo!() }
    pub fn clear_timeout_with_handle(&self, handle: i32) { todo!() }
}

impl Deref for Window { type Target = EventTarget; fn deref(&self) -> &EventTarget { todo!() } }
impl AsRef<JsValue> for Window { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Window { fn clone(&self) -> Window { todo!() } }

pub struct Navigator;

impl Navigator {
    pub fn user_agent(&self) -> Result<String, JsValue> { todo!() }
    pub fn online(&self) -> bool { todo!() }
}

impl Deref for Navigator { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Navigator { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Navigator { fn clone(&self) -> Navigator { todo!() } }

pub struct Location;

impl Location {
    pub fn href(&self) -> Result<String, JsValue> { todo!() }
    pub fn host(&self) -> Result<String, JsValue> { todo!() }
    pub fn protocol(&self) -> Result<String, JsValue> { todo!() }
}

impl Deref for Location { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Location { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Location { fn clone(&self) -> Location { todo!() } }

// ── Events ───────────────────────────────────────────────────────────────────

pub struct EventTarget;

impl EventTarget {
    pub fn add_event_listener_with_callback(&self, type_: &str, listener: &Function) -> Result<(), JsValue> { todo!() }
    pub fn remove_event_listener_with_callback(&self, type_: &str, listener: &Function) -> Result<(), JsValue> { todo!() }
    pub fn dispatch_event(&self, event: &Event) -> Result<bool, JsValue> { todo!() }
}

impl Deref for EventTarget { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for EventTarget { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for EventTarget { fn clone(&self) -> EventTarget { todo!() } }
pub struct Event;

impl Event {
    pub fn new(type_: &str) -> Result<Event, JsValue> { todo!() }
    pub fn type_(&self) -> String { todo!() }
    pub fn target(&self) -> Option<EventTarget> { todo!() }
    pub fn current_target(&self) -> Option<EventTarget> { todo!() }
    pub fn prevent_default(&self) { todo!() }
    pub fn stop_propagation(&self) { todo!() }
}

impl Deref for Event { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for Event { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for Event { fn clone(&self) -> Event { todo!() } }

pub struct MessageEvent;

impl MessageEvent {
    pub fn data(&self) -> JsValue { todo!() }
    pub fn origin(&self) -> String { todo!() }
    pub fn last_event_id(&self) -> String { todo!() }
}

impl Deref for MessageEvent { type Target = Event; fn deref(&self) -> &Event { todo!() } }
impl AsRef<JsValue> for MessageEvent { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for MessageEvent { fn clone(&self) -> MessageEvent { todo!() } }

pub struct CloseEvent;

impl CloseEvent {
    pub fn code(&self) -> u16 { todo!() }
    pub fn reason(&self) -> String { todo!() }
    pub fn was_clean(&self) -> bool { todo!() }
}

impl Deref for CloseEvent { type Target = Event; fn deref(&self) -> &Event { todo!() } }
impl AsRef<JsValue> for CloseEvent { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for CloseEvent { fn clone(&self) -> CloseEvent { todo!() } }

pub struct ErrorEvent;

impl ErrorEvent {
    pub fn message(&self) -> String { todo!() }
    pub fn filename(&self) -> String { todo!() }
    pub fn error(&self) -> JsValue { todo!() }
}

impl Deref for ErrorEvent { type Target = Event; fn deref(&self) -> &Event { todo!() } }
impl AsRef<JsValue> for ErrorEvent { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for ErrorEvent { fn clone(&self) -> ErrorEvent { todo!() } }

// ── WebSocket ────────────────────────────────────────────────────────────────

pub struct WebSocket;

impl WebSocket {
    pub const CONNECTING: u16 = 0;
    pub const OPEN: u16 = 1;
    pub const CLOSING: u16 = 2;
    pub const CLOSED: u16 = 3;

    pub fn new(url: &str) -> Result<WebSocket, JsValue> { todo!() }
    pub fn new_with_str(url: &str, protocols: &str) -> Result<WebSocket, JsValue> { todo!() }

    pub fn url(&self) -> String { todo!() }
    pub fn ready_state(&self) -> u16 { todo!() }
    pub fn buffered_amount(&self) -> u32 { todo!() }
    pub fn binary_type(&self) -> BinaryType { todo!() }
    pub fn set_binary_type(&self, value: BinaryType) { todo!() }

    pub fn send_with_str(&self, data: &str) -> Result<(), JsValue> { todo!() }
    pub fn send_with_u8_array(&self, data: &[u8]) -> Result<(), JsValue> { todo!() }
    pub fn send_with_array_buffer(&self, data: &ArrayBuffer) -> Result<(), JsValue> { todo!() }

    pub fn close(&self) -> Result<(), JsValue> { todo!() }
    pub fn close_with_code(&self, code: u16) -> Result<(), JsValue> { todo!() }
    pub fn close_with_code_and_reason(&self, code: u16, reason: &str) -> Result<(), JsValue> { todo!() }

    pub fn set_onopen(&self, value: Option<&Function>) { todo!() }
    pub fn set_onmessage(&self, value: Option<&Function>) { todo!() }
    pub fn set_onerror(&self, value: Option<&Function>) { todo!() }
    pub fn set_onclose(&self, value: Option<&Function>) { todo!() }
}

impl Deref for WebSocket { type Target = EventTarget; fn deref(&self) -> &EventTarget { todo!() } }
impl AsRef<JsValue> for WebSocket { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for WebSocket { fn clone(&self) -> WebSocket { todo!() } }

pub enum BinaryType {
    Blob,
    Arraybuffer,
}

impl Clone for BinaryType { fn clone(&self) -> BinaryType { todo!() } }
impl Copy for BinaryType {}
impl PartialEq for BinaryType { fn eq(&self, other: &BinaryType) -> bool { todo!() } }

// ── IndexedDB ────────────────────────────────────────────────────────────────

pub struct IdbFactory;

impl IdbFactory {
    pub fn open(&self, name: &str) -> Result<IdbOpenDbRequest, JsValue> { todo!() }
    pub fn open_with_u32(&self, name: &str, version: u32) -> Result<IdbOpenDbRequest, JsValue> { todo!() }
    pub fn open_with_f64(&self, name: &str, version: f64) -> Result<IdbOpenDbRequest, JsValue> { todo!() }
    pub fn delete_database(&self, name: &str) -> Result<IdbOpenDbRequest, JsValue> { todo!() }
    pub fn cmp(&self, first: &JsValue, second: &JsValue) -> Result<i16, JsValue> { todo!() }
}

impl Deref for IdbFactory { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for IdbFactory { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbFactory { fn clone(&self) -> IdbFactory { todo!() } }

pub struct IdbRequest;

impl IdbRequest {
    pub fn result(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn error(&self) -> Result<Option<DomException>, JsValue> { todo!() }
    pub fn source(&self) -> Option<Object> { todo!() }
    pub fn transaction(&self) -> Option<IdbTransaction> { todo!() }
    pub fn ready_state(&self) -> Result<IdbRequestReadyState, JsValue> { todo!() }
    pub fn set_onsuccess(&self, value: Option<&Function>) { todo!() }
    pub fn set_onerror(&self, value: Option<&Function>) { todo!() }
}

impl Deref for IdbRequest { type Target = EventTarget; fn deref(&self) -> &EventTarget { todo!() } }
impl AsRef<JsValue> for IdbRequest { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbRequest { fn clone(&self) -> IdbRequest { todo!() } }
pub enum IdbRequestReadyState {
    Pending,
    Done,
}

pub struct IdbOpenDbRequest;

impl IdbOpenDbRequest {
    pub fn set_onupgradeneeded(&self, value: Option<&Function>) { todo!() }
    pub fn set_onblocked(&self, value: Option<&Function>) { todo!() }
}

impl Deref for IdbOpenDbRequest { type Target = IdbRequest; fn deref(&self) -> &IdbRequest { todo!() } }
impl AsRef<JsValue> for IdbOpenDbRequest { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbOpenDbRequest { fn clone(&self) -> IdbOpenDbRequest { todo!() } }

pub struct IdbDatabase;

impl IdbDatabase {
    pub fn name(&self) -> String { todo!() }
    pub fn version(&self) -> f64 { todo!() }
    pub fn object_store_names(&self) -> DomStringList { todo!() }
    pub fn transaction_with_str(&self, store_names: &str) -> Result<IdbTransaction, JsValue> { todo!() }
    pub fn transaction_with_str_and_mode(&self, store_names: &str, mode: IdbTransactionMode) -> Result<IdbTransaction, JsValue> { todo!() }
    pub fn transaction_with_str_sequence(&self, store_names: &JsValue) -> Result<IdbTransaction, JsValue> { todo!() }
    pub fn transaction_with_str_sequence_and_mode(&self, store_names: &JsValue, mode: IdbTransactionMode) -> Result<IdbTransaction, JsValue> { todo!() }
    pub fn create_object_store(&self, name: &str) -> Result<IdbObjectStore, JsValue> { todo!() }
    pub fn delete_object_store(&self, name: &str) -> Result<(), JsValue> { todo!() }
    pub fn close(&self) { todo!() }
    pub fn set_onversionchange(&self, value: Option<&Function>) { todo!() }
    pub fn set_onclose(&self, value: Option<&Function>) { todo!() }
    pub fn set_onabort(&self, value: Option<&Function>) { todo!() }
    pub fn set_onerror(&self, value: Option<&Function>) { todo!() }
}

impl Deref for IdbDatabase { type Target = EventTarget; fn deref(&self) -> &EventTarget { todo!() } }
impl AsRef<JsValue> for IdbDatabase { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbDatabase { fn clone(&self) -> IdbDatabase { todo!() } }
pub struct IdbTransaction;

impl IdbTransaction {
    pub fn db(&self) -> IdbDatabase { todo!() }
    pub fn mode(&self) -> Result<IdbTransactionMode, JsValue> { todo!() }
    pub fn error(&self) -> Option<DomException> { todo!() }
    pub fn object_store(&self, name: &str) -> Result<IdbObjectStore, JsValue> { todo!() }
    pub fn object_store_names(&self) -> DomStringList { todo!() }
    pub fn abort(&self) -> Result<(), JsValue> { todo!() }
    pub fn commit(&self) -> Result<(), JsValue> { todo!() }
    pub fn set_oncomplete(&self, value: Option<&Function>) { todo!() }
    pub fn set_onerror(&self, value: Option<&Function>) { todo!() }
    pub fn set_onabort(&self, value: Option<&Function>) { todo!() }
}

impl Deref for IdbTransaction { type Target = EventTarget; fn deref(&self) -> &EventTarget { todo!() } }
impl AsRef<JsValue> for IdbTransaction { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbTransaction { fn clone(&self) -> IdbTransaction { todo!() } }

pub enum IdbTransactionMode {
    Readonly,
    Readwrite,
    Versionchange,
}

impl Clone for IdbTransactionMode { fn clone(&self) -> IdbTransactionMode { todo!() } }
impl Copy for IdbTransactionMode {}
impl Debug for IdbTransactionMode { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for IdbTransactionMode { fn eq(&self, other: &IdbTransactionMode) -> bool { todo!() } }

pub struct IdbObjectStore;

impl IdbObjectStore {
    pub fn name(&self) -> String { todo!() }
    pub fn key_path(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn index_names(&self) -> DomStringList { todo!() }
    pub fn transaction(&self) -> IdbTransaction { todo!() }

    pub fn get(&self, key: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn get_all(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn put(&self, value: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn put_with_key(&self, value: &JsValue, key: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn add(&self, value: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn add_with_key(&self, value: &JsValue, key: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn delete(&self, key: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn clear(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn count(&self) -> Result<IdbRequest, JsValue> { todo!() }

    pub fn index(&self, name: &str) -> Result<IdbIndex, JsValue> { todo!() }
    pub fn create_index_with_str(&self, name: &str, key_path: &str) -> Result<IdbIndex, JsValue> { todo!() }
    pub fn create_index_with_str_sequence(&self, name: &str, key_path: &JsValue) -> Result<IdbIndex, JsValue> { todo!() }
    pub fn create_index_with_str_and_optional_parameters(&self, name: &str, key_path: &str, options: &IdbIndexParameters) -> Result<IdbIndex, JsValue> { todo!() }
    pub fn create_index_with_str_sequence_and_optional_parameters(&self, name: &str, key_path: &JsValue, options: &IdbIndexParameters) -> Result<IdbIndex, JsValue> { todo!() }
    pub fn delete_index(&self, name: &str) -> Result<(), JsValue> { todo!() }

    pub fn open_cursor(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_cursor_with_range(&self, range: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_cursor_with_range_and_direction(&self, range: &JsValue, direction: IdbCursorDirection) -> Result<IdbRequest, JsValue> { todo!() }
}

impl Deref for IdbObjectStore { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for IdbObjectStore { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbObjectStore { fn clone(&self) -> IdbObjectStore { todo!() } }

pub struct IdbIndex;

impl IdbIndex {
    pub fn name(&self) -> String { todo!() }
    pub fn key_path(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn object_store(&self) -> IdbObjectStore { todo!() }
    pub fn unique(&self) -> bool { todo!() }
    pub fn get(&self, key: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn get_all(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn count(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_cursor(&self) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_cursor_with_range(&self, range: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_cursor_with_range_and_direction(&self, range: &JsValue, direction: IdbCursorDirection) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn open_key_cursor_with_range(&self, range: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
}

impl Deref for IdbIndex { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for IdbIndex { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbIndex { fn clone(&self) -> IdbIndex { todo!() } }

pub struct IdbIndexParameters;

impl IdbIndexParameters {
    pub fn new() -> IdbIndexParameters { todo!() }
    pub fn unique(&mut self, val: bool) -> &mut IdbIndexParameters { todo!() }
    pub fn multi_entry(&mut self, val: bool) -> &mut IdbIndexParameters { todo!() }
}

pub struct IdbCursor;

impl IdbCursor {
    pub fn key(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn primary_key(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn direction(&self) -> IdbCursorDirection { todo!() }
    pub fn source(&self) -> Object { todo!() }
    pub fn continue_(&self) -> Result<(), JsValue> { todo!() }
    pub fn continue_with_key(&self, key: &JsValue) -> Result<(), JsValue> { todo!() }
    pub fn advance(&self, count: u32) -> Result<(), JsValue> { todo!() }
    pub fn update(&self, value: &JsValue) -> Result<IdbRequest, JsValue> { todo!() }
    pub fn delete(&self) -> Result<IdbRequest, JsValue> { todo!() }
}

impl Deref for IdbCursor { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for IdbCursor { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbCursor { fn clone(&self) -> IdbCursor { todo!() } }

pub struct IdbCursorWithValue;

impl IdbCursorWithValue {
    pub fn value(&self) -> Result<JsValue, JsValue> { todo!() }
}

impl Deref for IdbCursorWithValue { type Target = IdbCursor; fn deref(&self) -> &IdbCursor { todo!() } }
impl AsRef<JsValue> for IdbCursorWithValue { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbCursorWithValue { fn clone(&self) -> IdbCursorWithValue { todo!() } }
pub enum IdbCursorDirection {
    Next,
    Nextunique,
    Prev,
    Prevunique,
}

impl Clone for IdbCursorDirection { fn clone(&self) -> IdbCursorDirection { todo!() } }
impl Copy for IdbCursorDirection {}
impl Debug for IdbCursorDirection { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for IdbCursorDirection { fn eq(&self, other: &IdbCursorDirection) -> bool { todo!() } }

pub struct IdbKeyRange;

impl IdbKeyRange {
    pub fn only(value: &JsValue) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn lower_bound(lower: &JsValue) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn lower_bound_with_open(lower: &JsValue, open: bool) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn upper_bound(upper: &JsValue) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn upper_bound_with_open(upper: &JsValue, open: bool) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn bound(lower: &JsValue, upper: &JsValue) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn bound_with_lower_open(lower: &JsValue, upper: &JsValue, lower_open: bool) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn bound_with_lower_open_and_upper_open(lower: &JsValue, upper: &JsValue, lower_open: bool, upper_open: bool) -> Result<IdbKeyRange, JsValue> { todo!() }
    pub fn lower(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn upper(&self) -> Result<JsValue, JsValue> { todo!() }
    pub fn lower_open(&self) -> bool { todo!() }
    pub fn upper_open(&self) -> bool { todo!() }
    pub fn includes(&self, value: &JsValue) -> Result<bool, JsValue> { todo!() }
}

impl Deref for IdbKeyRange { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for IdbKeyRange { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbKeyRange { fn clone(&self) -> IdbKeyRange { todo!() } }

pub struct IdbVersionChangeEvent;

impl IdbVersionChangeEvent {
    pub fn old_version(&self) -> f64 { todo!() }
    pub fn new_version(&self) -> Option<f64> { todo!() }
}

impl Deref for IdbVersionChangeEvent { type Target = Event; fn deref(&self) -> &Event { todo!() } }
impl AsRef<JsValue> for IdbVersionChangeEvent { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for IdbVersionChangeEvent { fn clone(&self) -> IdbVersionChangeEvent { todo!() } }

pub struct DomStringList;

impl DomStringList {
    pub fn length(&self) -> u32 { todo!() }
    pub fn item(&self, index: u32) -> Option<String> { todo!() }
    pub fn contains(&self, string: &str) -> bool { todo!() }
    pub fn get(&self, index: u32) -> Option<String> { todo!() }
}

impl Deref for DomStringList { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for DomStringList { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for DomStringList { fn clone(&self) -> DomStringList { todo!() } }

pub struct DomException;

impl DomException {
    pub fn name(&self) -> String { todo!() }
    pub fn message(&self) -> String { todo!() }
    pub fn code(&self) -> u16 { todo!() }
}

impl Deref for DomException { type Target = Object; fn deref(&self) -> &Object { todo!() } }
impl AsRef<JsValue> for DomException { fn as_ref(&self) -> &JsValue { todo!() } }
impl Clone for DomException { fn clone(&self) -> DomException { todo!() } }

// ── Generated cast and conversion impls ─────────────────────────────────────
//
// web-sys generates these for every wrapper in the enabled feature list. The
// corpus leans on them constantly: `event.target()` yields an `EventTarget`
// that is immediately `unchecked_into::<IdbOpenDbRequest>()`, and
// `open_request.result()?` yields a `JsValue` that is
// `unchecked_into::<IdbDatabase>()`. Each needs both halves.

impl From<Window> for JsValue { fn from(value: Window) -> JsValue { todo!() } }
impl JsCast for Window {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Window { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Window { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Window> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Navigator> for JsValue { fn from(value: Navigator) -> JsValue { todo!() } }
impl JsCast for Navigator {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Navigator { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Navigator { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Navigator> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Location> for JsValue { fn from(value: Location) -> JsValue { todo!() } }
impl JsCast for Location {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Location { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Location { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Location> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<EventTarget> for JsValue { fn from(value: EventTarget) -> JsValue { todo!() } }
impl JsCast for EventTarget {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> EventTarget { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &EventTarget { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, EventTarget> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<Event> for JsValue { fn from(value: Event) -> JsValue { todo!() } }
impl JsCast for Event {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> Event { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &Event { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, Event> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<MessageEvent> for JsValue { fn from(value: MessageEvent) -> JsValue { todo!() } }
impl JsCast for MessageEvent {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> MessageEvent { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &MessageEvent { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, MessageEvent> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<CloseEvent> for JsValue { fn from(value: CloseEvent) -> JsValue { todo!() } }
impl JsCast for CloseEvent {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> CloseEvent { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &CloseEvent { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, CloseEvent> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<ErrorEvent> for JsValue { fn from(value: ErrorEvent) -> JsValue { todo!() } }
impl JsCast for ErrorEvent {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> ErrorEvent { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &ErrorEvent { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, ErrorEvent> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<WebSocket> for JsValue { fn from(value: WebSocket) -> JsValue { todo!() } }
impl JsCast for WebSocket {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> WebSocket { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &WebSocket { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, WebSocket> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbFactory> for JsValue { fn from(value: IdbFactory) -> JsValue { todo!() } }
impl JsCast for IdbFactory {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbFactory { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbFactory { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbFactory> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbRequest> for JsValue { fn from(value: IdbRequest) -> JsValue { todo!() } }
impl JsCast for IdbRequest {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbRequest { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbRequest { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbRequest> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbOpenDbRequest> for JsValue { fn from(value: IdbOpenDbRequest) -> JsValue { todo!() } }
impl JsCast for IdbOpenDbRequest {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbOpenDbRequest { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbOpenDbRequest { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbOpenDbRequest> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbDatabase> for JsValue { fn from(value: IdbDatabase) -> JsValue { todo!() } }
impl JsCast for IdbDatabase {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbDatabase { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbDatabase { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbDatabase> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbTransaction> for JsValue { fn from(value: IdbTransaction) -> JsValue { todo!() } }
impl JsCast for IdbTransaction {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbTransaction { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbTransaction { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbTransaction> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbObjectStore> for JsValue { fn from(value: IdbObjectStore) -> JsValue { todo!() } }
impl JsCast for IdbObjectStore {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbObjectStore { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbObjectStore { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbObjectStore> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbIndex> for JsValue { fn from(value: IdbIndex) -> JsValue { todo!() } }
impl JsCast for IdbIndex {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbIndex { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbIndex { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbIndex> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbCursor> for JsValue { fn from(value: IdbCursor) -> JsValue { todo!() } }
impl JsCast for IdbCursor {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbCursor { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbCursor { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbCursor> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbCursorWithValue> for JsValue { fn from(value: IdbCursorWithValue) -> JsValue { todo!() } }
impl JsCast for IdbCursorWithValue {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbCursorWithValue { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbCursorWithValue { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbCursorWithValue> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbKeyRange> for JsValue { fn from(value: IdbKeyRange) -> JsValue { todo!() } }
impl JsCast for IdbKeyRange {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbKeyRange { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbKeyRange { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbKeyRange> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<IdbVersionChangeEvent> for JsValue { fn from(value: IdbVersionChangeEvent) -> JsValue { todo!() } }
impl JsCast for IdbVersionChangeEvent {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> IdbVersionChangeEvent { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &IdbVersionChangeEvent { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, IdbVersionChangeEvent> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<DomStringList> for JsValue { fn from(value: DomStringList) -> JsValue { todo!() } }
impl JsCast for DomStringList {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> DomStringList { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &DomStringList { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, DomStringList> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}

impl From<DomException> for JsValue { fn from(value: DomException) -> JsValue { todo!() } }
impl JsCast for DomException {
    fn instanceof(val: &JsValue) -> bool { todo!() }
    fn is_type_of(val: &JsValue) -> bool { todo!() }
    fn unchecked_from_js(val: JsValue) -> DomException { todo!() }
    fn unchecked_from_js_ref(val: &JsValue) -> &DomException { todo!() }
    fn has_type<T: JsCast>(&self) -> bool { todo!() }
    fn dyn_into<T: JsCast>(self) -> Result<T, DomException> { todo!() }
    fn dyn_ref<T: JsCast>(&self) -> Option<&T> { todo!() }
    fn unchecked_into<T: JsCast>(self) -> T { todo!() }
    fn unchecked_ref<T: JsCast>(&self) -> &T { todo!() }
    fn is_instance_of<T: JsCast>(&self) -> bool { todo!() }
}
