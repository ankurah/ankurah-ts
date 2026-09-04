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

/// The resolved policy: the same two tables, keyed by identity.
#[derive(Debug, Default)]
pub struct SystemShapes {
    accessors: HashMap<TypeId, Accessor>,
    forms: HashMap<TypeId, Form>,
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
        shapes
    }

    pub fn accessor(&self, id: TypeId) -> Option<Accessor> {
        self.accessors.get(&id).copied()
    }

    pub fn form(&self, id: TypeId) -> Option<Form> {
        self.forms.get(&id).copied()
    }
}
