//! Does a value of this type have to be released, and how?
//!
//! Rust runs drop glue for every value it owns. TypeScript runs none, so the
//! emitter writes the release itself, and this is the question it asks first:
//! given the type the engine resolved, what does the enclosing scope owe?
//!
//! The answers are the four shapes `@ankurah/base` offers plus "the engine
//! cannot say". Nothing here guesses: an unresolved answer emits no release and
//! is reported, because a release written against a type nobody could name is
//! how a live value gets dropped out from under its owner.

use crate::name_map::system_shapes::Glue;
use crate::registry::Probe;
use crate::ty::{Ty, TypeId};

/// What a scope owes a value of some type when the value goes out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drops {
    /// `x.drop()` — the runtime type releases itself and cascades into what it
    /// owns.
    Own,
    /// `x.drop()` on a lock or borrow guard, whose second call is a deliberate
    /// no-op. That is what lets a hoisted temporary be released at the end of
    /// its statement and listed in the enclosing `finally` as well.
    Guard,
    /// `dropOwned(x)` — a plain JavaScript value (an array, a `Map`, a
    /// `T | null`, a tuple) that owns what is inside it. It has no `drop()` of
    /// its own, so the cascade walks it.
    Cascade,
    /// Nothing: a `Copy` type, a primitive, a `&T`, a string.
    Nothing,
    /// The engine could not name what this is — a bare type parameter, an
    /// unnormalised projection. Nothing is emitted and the site is reported.
    Unknown,
}

impl Drops {
    /// Does this owe the scope anything at all?
    pub fn is_droppable(self) -> bool {
        matches!(self, Drops::Own | Drops::Guard | Drops::Cascade)
    }

    /// The release, written against a name, as a statement.
    pub fn release(self, name: &str) -> Option<String> {
        self.release_expr(name).map(|call| format!("{};", call))
    }

    /// The same call without the semicolon, for the places a release has to
    /// stand inside an expression — an explicit `drop(x)`, a value a statement
    /// threw away.
    pub fn release_expr(self, name: &str) -> Option<String> {
        match self {
            Drops::Own | Drops::Guard => Some(format!("{}.drop()", name)),
            Drops::Cascade => Some(format!("dropOwned({})", name)),
            Drops::Nothing | Drops::Unknown => None,
        }
    }
}

/// What the scope owes a value of `ty`.
///
/// The receiver is a `Probe` rather than the bare registry because two of the
/// answers are impl-table questions: whether a type is `Copy`, and what a
/// projection normalises to.
pub fn drops_of(probe: &Probe, ty: &Ty) -> Drops {
    match ty {
        // A borrow owns nothing, and Rust's drop of one releases nothing.
        Ty::Ref { .. } => Drops::Nothing,
        Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never => Drops::Nothing,

        // A tuple, an array and a slice are JavaScript arrays here, and Rust
        // drops every element. The cascade walks them; a sequence of numbers
        // owes nothing.
        Ty::Tuple(elems) => cascade_over(probe, elems.iter()),
        Ty::Array { elem, .. } | Ty::Slice(elem) => cascade_over(probe, std::iter::once(&**elem)),

        // An owned trait object is whatever implements the trait, and every
        // implementor the port emits is a class with a `drop()`. The cascade
        // finds it without the emitter having to name the type.
        Ty::Dyn { .. } => Drops::Cascade,

        Ty::Named { id, args } => named(probe, *id, args, ty),

        // A projection the impl table can settle is settled; `<I as
        // Iterator>::Item` with no impl behind it is not.
        Ty::Assoc { .. } => {
            let normalized = probe.normalize(ty);
            if normalized == *ty {
                Drops::Unknown
            } else {
                drops_of(probe, &normalized)
            }
        }

        Ty::Param(_) | Ty::ImplTrait { .. } | Ty::Infer => Drops::Unknown,
    }
}

fn named(probe: &Probe, id: TypeId, args: &[Ty], ty: &Ty) -> Drops {
    // A type the corpus mentions and nothing declares — `ulid::Ulid`,
    // `anyhow::Error` — is emitted as whatever its foreign package supplies.
    // It has no `drop()` to call, and reaching one with the cascade would earn
    // a "no drop glue" warning per constructor rather than release anything.
    if id.is_foreign() {
        return Drops::Nothing;
    }

    if probe.reg.is_system(id) {
        // What `@ankurah/base` writes this std type as decides it, where the
        // runtime has an answer of its own.
        if let Some(glue) = probe.reg.shapes().glue(id) {
            return match glue {
                Glue::Object => Drops::Own,
                Glue::Guard => Drops::Guard,
                Glue::None => Drops::Nothing,
            };
        }
        // Everything else the surface declares is a plain JavaScript value —
        // an array for a `Vec`, a `Map` for a `HashMap`, `T | null` for an
        // `Option`, a `Result` for a `Result`. It owns its arguments and
        // nothing more, so they decide.
        //
        // `Result` is the one exception: the runtime writes it as an object of
        // its own, so its own `drop()` releases both it and its payload.
        if probe
            .reg
            .system_type("std::result::Result")
            .is_some_and(|r| r == id)
        {
            return Drops::Own;
        }
        return cascade_over(probe, args.iter());
    }

    // A crate type is a class extending Struct, Enum or Drop, all of which are
    // AkObjects with a `drop()`. A `Copy` type cannot implement `Drop` in Rust,
    // so the emitter gives it no drop glue and the scope owes it nothing.
    if is_copy(probe, ty) {
        return Drops::Nothing;
    }
    Drops::Own
}

/// A container's answer, read off what it holds: `Cascade` where any argument
/// owes a release, nothing where none does. An argument the engine could not
/// name makes the container unknown rather than empty, because a `Vec<T>` whose
/// `T` might own something is not a `Vec<u8>`.
fn cascade_over<'t>(probe: &Probe, args: impl Iterator<Item = &'t Ty>) -> Drops {
    let mut answer = Drops::Nothing;
    for arg in args {
        match drops_of(probe, arg) {
            Drops::Nothing => {}
            Drops::Unknown => answer = Drops::Unknown,
            _ => return Drops::Cascade,
        }
    }
    answer
}

fn is_copy(probe: &Probe, ty: &Ty) -> bool {
    probe
        .reg
        .shapes()
        .copy_trait()
        .is_some_and(|copy| probe.implements(ty, copy))
}
