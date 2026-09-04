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
const ACCESSORS: [(&str, Accessor); 10] = [
    ("std::sync::Arc", Accessor::Field("value")),
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
const FORMS: [(&str, Form); 17] = [
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
    /// A plain JavaScript value — an array, a `Map`, a `T | null` — that owns
    /// whatever is inside it. It has no `drop()` of its own, so the cascade
    /// walks it: `dropOwned(v)`.
    Cascade,
    /// Nothing to release: a number, a string, an atomic, a `Duration`.
    None,
}

/// Types whose drop the port's runtime performs through a method of their own.
///
/// Everything the std surface declares and this table does not name is a plain
/// value in TypeScript. The ones that own something — `Vec<T>`, `HashMap<K, V>`,
/// `Option<T>` — are read from their arguments instead (`Glue::Cascade`), which
/// is why they are not here.
const OWN_DROP: [(&str, Glue); 12] = [
    ("std::sync::Arc", Glue::Object),
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

/// The resolved policy: the same tables, keyed by identity.
#[derive(Debug, Default)]
pub struct SystemShapes {
    accessors: HashMap<TypeId, Accessor>,
    forms: HashMap<TypeId, Form>,
    glue: HashMap<TypeId, Glue>,
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
        shapes.copy = reg.system_type(COPY_PATH);
        shapes
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
