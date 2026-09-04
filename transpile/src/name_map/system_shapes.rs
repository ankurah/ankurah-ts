//! What the port's runtime makes of each declared std type.
//!
//! The std surface says what Rust says; this says what the TypeScript is. They
//! are different questions and neither belongs in the other's file: `Arc<T>`
//! dereferences to `T` because `std/sync/arc.rs` declares `impl Deref for
//! Arc<T>`, and reaching through it is written `.value` because that is how
//! `@ankurah/base` writes an `Arc`.
//!
//! Every entry here names a type by the full Rust path it is declared at, and
//! is resolved to that type's id once, when the surface is loaded. Nothing
//! downstream compares a leaf name: `std::sync::atomic::Ordering` and
//! `std::cmp::Ordering` share one, and a crate type called `Vec` is its own
//! type.

use std::collections::HashMap;

use crate::registry::TypeRegistry;
use crate::ty::TypeId;

/// How reaching through a wrapper is written in TypeScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accessor {
    /// `@ankurah/base` holds the value in a field of this name.
    Field(&'static str),
    /// The wrapper is the value: `Box<T>` is a `T`, and a `Vec<T>` is already
    /// the array its slice would be.
    Transparent,
}

/// The TypeScript form a declared system type takes. The argument types come
/// from the resolved `Ty`, so this only says which form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// `Vec<u8>` is a `Uint8Array` and every other `Vec` a JavaScript array.
    VecOrBytes,
    Nullable,
    Result,
    Map,
    Set,
    /// A reference-counted pointer, emitted under its own name.
    Rc,
    /// The value is the type's only argument: `Box<T>` is a `T`.
    Transparent,
    Str,
    Number,
    Boolean,
    Never,
}

/// Where the port keeps the value inside a wrapper. A type with no entry is not
/// a wrapper, whatever it dereferences to in Rust.
const ACCESSORS: [(&str, Accessor); 13] = [
    ("std::sync::Arc", Accessor::Field("value")),
    // tokio's guards are the same shape as std's: the value sits in `.value`.
    ("tokio::sync::MutexGuard", Accessor::Field("value")),
    ("tokio::sync::RwLockReadGuard", Accessor::Field("value")),
    ("tokio::sync::RwLockWriteGuard", Accessor::Field("value")),
    // `tokio::sync::watch::Ref` was here. The browser target provides no watch
    // channel, so `std_surface/extern/tokio/sync.rs` no longer declares the
    // module, and a row naming a type the surface does not declare is itself
    // reported.
    ("std::rc::Rc", Accessor::Field("value")),
    ("std::sync::MutexGuard", Accessor::Field("value")),
    ("std::sync::RwLockReadGuard", Accessor::Field("value")),
    ("std::sync::RwLockWriteGuard", Accessor::Field("value")),
    ("std::cell::Ref", Accessor::Field("value")),
    ("std::cell::RefMut", Accessor::Field("value")),
    ("std::boxed::Box", Accessor::Transparent),
    ("std::vec::Vec", Accessor::Transparent),
    ("std::string::String", Accessor::Transparent),
];

/// What each declared type is written as.
const FORMS: [(&str, Form); 18] = [
    ("std::vec::Vec", Form::VecOrBytes),
    ("std::option::Option", Form::Nullable),
    ("std::result::Result", Form::Result),
    ("std::collections::HashMap", Form::Map),
    ("std::collections::BTreeMap", Form::Map),
    ("std::collections::HashSet", Form::Set),
    ("std::collections::BTreeSet", Form::Set),
    ("std::sync::Arc", Form::Rc),
    ("std::sync::Weak", Form::Rc),
    ("std::rc::Rc", Form::Rc),
    ("std::string::String", Form::Str),
    ("std::boxed::Box", Form::Transparent),
    // A `Duration` crosses as a number of milliseconds. It is `Copy` and holds
    // nothing, and the port's `sleep` and `timeout` take milliseconds, so the
    // runtime exports no `Duration` class for a signature to name.
    ("std::time::Duration", Form::Number),
    // Atomics are plain values in single-threaded JavaScript.
    ("std::sync::atomic::AtomicUsize", Form::Number),
    ("std::sync::atomic::AtomicU32", Form::Number),
    ("std::sync::atomic::AtomicU64", Form::Number),
    ("std::sync::atomic::AtomicBool", Form::Boolean),
    ("std::convert::Infallible", Form::Never),
];

/// What dropping a value of a declared std type has to release.
///
/// Rust runs drop glue for every one of these; what differs is what the port's
/// runtime gives the emitter to call. `@ankurah/base` writes some of them as
/// objects with a `drop()` and the rest as plain JavaScript values, and the
/// difference decides what a scope's `finally` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glue {
    /// The runtime type has its own `drop()`: `Arc`, the containers, `Result`.
    Object,
    /// A lock or borrow guard. Also `drop()`, but a guard's second drop is a
    /// deliberate no-op, which is what lets a hoisted temporary be released at
    /// the end of its statement and listed in the enclosing `finally` as well.
    Guard,
    /// Nothing to release: a number, a string, an atomic, a `Duration`.
    None,
}

/// Types whose drop the port's runtime performs through a method of their own.
///
/// Everything the std surface declares and this table does not name is a plain
/// value in TypeScript. The ones that own something — `Vec<T>`, `HashMap<K, V>`,
/// `Option<T>` — are read from their arguments instead: the emitter reads their
/// arguments and answers `Drops::Cascade` itself, which is why they are not
/// here.
const OWN_DROP: [(&str, Glue); 26] = [
    ("std::sync::Arc", Glue::Object),
    // tokio's own containers, guards and named futures. Each is an object in
    // `@ankurah/base` with a `drop()` of its own, and a `Notified`, a
    // `oneshot::Receiver` and a `JoinHandle` are futures with exactly one
    // consumer: whatever takes one takes it for good, and dropping one is what
    // cancels it.
    ("tokio::sync::Mutex", Glue::Object),
    ("tokio::sync::RwLock", Glue::Object),
    ("tokio::sync::Notify", Glue::Object),
    ("tokio::sync::Notified", Glue::Object),
    ("tokio::sync::MutexGuard", Glue::Guard),
    ("tokio::sync::RwLockReadGuard", Glue::Guard),
    ("tokio::sync::RwLockWriteGuard", Glue::Guard),
    ("tokio::sync::oneshot::Sender", Glue::Object),
    ("tokio::sync::oneshot::Receiver", Glue::Object),
    ("tokio::sync::mpsc::Sender", Glue::Object),
    ("tokio::sync::mpsc::Receiver", Glue::Object),
    ("tokio::sync::mpsc::UnboundedSender", Glue::Object),
    ("tokio::sync::mpsc::UnboundedReceiver", Glue::Object),
    ("tokio::task::JoinHandle", Glue::Object),
    ("std::rc::Rc", Glue::Object),
    ("std::sync::Weak", Glue::Object),
    ("std::rc::Weak", Glue::Object),
    ("std::sync::Mutex", Glue::Object),
    ("std::sync::RwLock", Glue::Object),
    ("std::cell::RefCell", Glue::Object),
    ("std::sync::MutexGuard", Glue::Guard),
    ("std::sync::RwLockReadGuard", Glue::Guard),
    ("std::sync::RwLockWriteGuard", Glue::Guard),
    ("std::cell::Ref", Glue::Guard),
    ("std::cell::RefMut", Glue::Guard),
];

/// Declared types that own nothing at all, whatever their arguments say.
///
/// `PhantomData<T>` and a `&T` behind an alias hold no `T`; an atomic and a
/// `Duration` are numbers here. Without this a `PhantomData<Entity>` would look
/// like a value that owes a drop.
const NO_GLUE: [&str; 6] = [
    "std::marker::PhantomData",
    "std::time::Duration",
    "std::time::Instant",
    "std::string::String",
    "std::sync::atomic::AtomicBool",
    "std::convert::Infallible",
];

/// What `@ankurah/base` calls a declared std type, where its own leaf name is
/// not what the runtime exports.
///
/// tokio's `Mutex` and `RwLock` are not std's, and the port has both, so the
/// tokio pair carry an `Async` prefix. Emitting them by leaf name handed a
/// `tokio::sync::Mutex<T>` to the std `Mutex<T>` class, whose `lock()` is not
/// async and whose guard is a different object.
///
/// The channel types keep their module in the name. `@ankurah/base` exports
/// `oneshot` and `mpsc` as namespaces, and both declare a `Receiver`, a
/// `Sender` and a `TryRecvError`; a bare leaf name picks whichever the
/// importing file happened to bring in, or nothing at all.
const RUNTIME_NAMES: [(&str, &str); 22] = [
    ("tokio::sync::Mutex", "AsyncMutex"),
    ("tokio::sync::MutexGuard", "AsyncMutexGuard"),
    ("tokio::sync::RwLock", "AsyncRwLock"),
    ("tokio::sync::RwLockReadGuard", "AsyncRwLockReadGuard"),
    ("tokio::sync::RwLockWriteGuard", "AsyncRwLockWriteGuard"),
    ("tokio::sync::Notify", "Notify"),
    ("tokio::sync::Notified", "Notified"),
    ("tokio::sync::TryLockError", "TryLockError"),
    ("tokio::sync::oneshot::Sender", "oneshot.Sender"),
    ("tokio::sync::oneshot::Receiver", "oneshot.Receiver"),
    ("tokio::sync::oneshot::error::RecvError", "oneshot.RecvError"),
    ("tokio::sync::oneshot::error::TryRecvError", "oneshot.TryRecvError"),
    // The four channel ends the runtime exports flat, by the ruling: only
    // `oneshot`'s stay behind their namespace, because both channels declare a
    // `Sender` and a `Receiver` and only one pair can have the bare name.
    ("tokio::sync::mpsc::Sender", "Sender"),
    ("tokio::sync::mpsc::Receiver", "Receiver"),
    ("tokio::sync::mpsc::UnboundedSender", "UnboundedSender"),
    ("tokio::sync::mpsc::UnboundedReceiver", "UnboundedReceiver"),
    ("tokio::sync::mpsc::error::SendError", "mpsc.SendError"),
    ("tokio::sync::mpsc::error::TrySendError", "mpsc.TrySendError"),
    ("tokio::sync::mpsc::error::TryRecvError", "mpsc.TryRecvError"),
    ("tokio::task::JoinHandle", "JoinHandle"),
    ("tokio::task::JoinError", "JoinError"),
    // `anyhow::Error` is `AnyhowError` in the runtime: `Error` is JavaScript's
    // own, and a signature saying `Result<T, Error>` promised that one.
    ("anyhow::Error", "AnyhowError"),
];

/// The resolved policy: the same tables, keyed by identity.
#[derive(Debug, Default)]
pub struct SystemShapes {
    accessors: HashMap<TypeId, Accessor>,
    forms: HashMap<TypeId, Form>,
    glue: HashMap<TypeId, Glue>,
    names: HashMap<TypeId, &'static str>,
    copy: Option<TypeId>,
    /// The paths no declaration answered to, so a surface that stops declaring
    /// `std::sync::Arc` says so instead of quietly emitting a plain class.
    pub unresolved: Vec<&'static str>,
}

impl SystemShapes {
    pub fn resolve(reg: &TypeRegistry) -> SystemShapes {
        let mut shapes = SystemShapes::default();
        for (path, accessor) in ACCESSORS {
            match reg.system_type(path) {
                Some(id) => {
                    shapes.accessors.insert(id, accessor);
                }
                None => shapes.unresolved.push(path),
            }
        }
        for (path, form) in FORMS {
            match reg.system_type(path) {
                Some(id) => {
                    shapes.forms.insert(id, form);
                }
                None => shapes.unresolved.push(path),
            }
        }
        for (path, glue) in OWN_DROP {
            match reg.system_type(path) {
                Some(id) => {
                    shapes.glue.insert(id, glue);
                }
                // `std::rc::Weak` is the only path here the corpus never
                // mentions, and a surface that stops declaring one of the rest
                // is a real loss. They are reported the same way the other two
                // tables are.
                None => shapes.unresolved.push(path),
            }
        }
        for path in NO_GLUE {
            if let Some(id) = reg.system_type(path) {
                shapes.glue.insert(id, Glue::None);
            }
        }
        for (path, name) in RUNTIME_NAMES {
            match reg.system_type(path) {
                Some(id) => {
                    shapes.names.insert(id, name);
                }
                None => shapes.unresolved.push(path),
            }
        }
        shapes.copy = reg.system_type(COPY_PATH);
        shapes
    }

    /// What the runtime exports this declared type as, where that is not its
    /// own leaf name.
    pub fn runtime_name(&self, id: TypeId) -> Option<&'static str> {
        self.names.get(&id).copied()
    }

    pub fn accessor(&self, id: TypeId) -> Option<Accessor> {
        self.accessors.get(&id).copied()
    }

    pub fn form(&self, id: TypeId) -> Option<Form> {
        self.forms.get(&id).copied()
    }

    /// What dropping this declared std type releases, where the port's runtime
    /// has an answer of its own. `None` means the type is not one of those and
    /// its arguments decide.
    pub fn glue(&self, id: TypeId) -> Option<Glue> {
        self.glue.get(&id).copied()
    }

    /// `std::marker::Copy`. A `Copy` type cannot implement `Drop` in Rust, so
    /// the emitter gives it no drop glue at all.
    pub fn copy_trait(&self) -> Option<TypeId> {
        self.copy
    }
}

/// The trait whose presence rules drop glue out entirely.
const COPY_PATH: &str = "std::marker::Copy";

#[cfg(test)]
mod tests {
    use crate::name_map::map_ty;
    use crate::testing::Fixture;

    /// The written type, as the emitter would spell it in a signature.
    fn written(rust: &str) -> String {
        let fixture = Fixture::build(&[("lib.rs", "pub struct Owned { pub n: u32 }\n")]);
        let ty = fixture.ty("lib.rs", rust);
        map_ty(&fixture.reg, &ty)
    }

    /// `@ankurah/base` exports `oneshot` and `mpsc` as namespaces, and both
    /// declare a `TryRecvError`. Written by leaf name, a signature named a type
    /// nothing in the file imports — and bun strips the name without complaint,
    /// so the first thing to notice was a reader.
    #[test]
    fn a_channel_error_keeps_the_module_that_tells_the_two_channels_apart() {
        assert_eq!(
            written("tokio::sync::oneshot::error::TryRecvError"),
            "oneshot.TryRecvError"
        );
        assert_eq!(
            written("tokio::sync::oneshot::error::RecvError"),
            "oneshot.RecvError"
        );
        assert_eq!(
            written("tokio::sync::mpsc::error::TryRecvError"),
            "mpsc.TryRecvError"
        );
        assert_eq!(
            written("tokio::sync::mpsc::error::SendError<Owned>"),
            "mpsc.SendError<Owned>"
        );
        assert_eq!(
            written("tokio::sync::mpsc::error::TrySendError<Owned>"),
            "mpsc.TrySendError<Owned>"
        );
    }

    /// A `Duration` crosses as a number of milliseconds, which is what the
    /// port's `sleep` and `timeout` take. There is no `Duration` class to name.
    #[test]
    fn a_duration_is_written_as_the_milliseconds_the_runtime_takes() {
        assert_eq!(written("std::time::Duration"), "number");
        assert_eq!(written("core::time::Duration"), "number");
    }

    /// The surface declares what tokio declares, minus the calls the browser
    /// target cannot honour. A call to one of those has to stop resolving, so
    /// that it is reported where it is written instead of emitting a call the
    /// runtime has nothing behind.
    #[test]
    fn the_surface_declares_no_tokio_call_the_browser_target_cannot_honour() {
        let fixture = Fixture::build(&[("lib.rs", "pub struct Owned { pub n: u32 }\n")]);
        for path in [
            "tokio::sync::watch::Sender",
            "tokio::sync::watch::Receiver",
            "tokio::sync::watch::Ref",
        ] {
            assert!(
                fixture.reg.system_type(path).is_none(),
                "`{}` is declared, so a watch channel resolves and emits a call the runtime \
                 does not answer",
                path
            );
        }
    }
}
