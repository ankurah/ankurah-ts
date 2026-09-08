//! What the registry HOLDS for a declared name: a type, a value, an alias.
//!
//! Split out of `registry/mod.rs`, which is the registry's own behaviour — the
//! tables, the lookups, the interning. These are the records those tables keep,
//! and nearly every other module reads them, so they earn a file a reader can
//! open without the machinery around them.

use super::{impls, MethodSig, ModuleId, TypeKind, Vis};
use crate::ty::Ty;

/// A declared type. Fields and method signatures are filled in after every
/// type has been declared, because resolving them needs the other declarations.
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub module: ModuleId,
    /// The leaf name as Rust writes it: `Broadcast`, `Arc`, `HashMap`.
    pub name: String,
    pub kind: TypeKind,
    /// Field name (in the TypeScript spelling emission uses) to field type.
    ///
    /// A field whose type the engine could not resolve is NOT here — the pair
    /// has nowhere to put a type it does not have. `field_order` carries every
    /// field, which is what a constructor call needs.
    pub fields: Vec<(String, Ty)>,
    /// Every field this declaration has, in the order it declares them, whether
    /// or not its type resolved. The emitted constructor takes its parameters
    /// in exactly this order, so a struct literal is written from it.
    pub field_order: Vec<String>,
    /// Declared generic parameter names, in order.
    pub type_params: Vec<String>,
    /// What the declaration REQUIRES of those parameters, inline and in its
    /// `where` clause alike, resolved in the module that wrote it. A struct
    /// whose field is a bounded parameter is the only reader today:
    /// `Holder<I: Iterator<Item = Token>> { walk: I }` holds a cursor, and the
    /// answer is not in the field's type — it is in the bound (FF4).
    pub bounds: Vec<impls::Bound>,
    /// What a parameter falls back to where the use site leaves it unwritten:
    /// `HashMap<K, V, S = RandomState>` is declared with three and always
    /// written with two. Positional alongside `type_params`.
    pub param_defaults: Vec<Option<Ty>>,
}

/// What `declare_type` needs. The crate's own structs and enums, the system
/// types, and (from the std-surface step) types parsed out of Rust stub files
/// all arrive through this one door.
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub kind: TypeKind,
    pub type_params: Vec<String>,
    pub vis: Vis,
}

/// A type alias. Aliases are expanded where they are used rather than given an
/// identity of their own, which is what Rust means by them.
#[derive(Debug, Clone)]
pub struct AliasDef {
    pub module: ModuleId,
    pub name: String,
    pub type_params: Vec<String>,
    /// `type Result<T, E = Error> = ..` — the fallback for a parameter the use
    /// site leaves unwritten, as written, resolved where the alias was declared.
    pub param_defaults: Vec<Option<syn::Type>>,
    pub rust_ty: syn::Type,
}

/// A function, constant or static.
#[derive(Debug, Clone)]
pub struct ValueDef {
    pub name: String,
    /// The declared type of a constant or static, and a function's return type;
    /// `None` where the engine could not name it.
    pub ty: Option<Ty>,
    /// What a free function declares, so a call to one can hand each argument
    /// the type its parameter was written with. Without it, `wants(x.into())`
    /// and `wants(|v| ..)` have nothing to read: only associated functions were
    /// reached, and 89 closures and 48 `.into()`s in the corpus stood in a
    /// position that said nothing.
    pub sig: Option<MethodSig>,
    /// Is every use of this name a FRESH value?
    ///
    /// A Rust `const` is inlined at each use, so `let mut a = ORIGIN; a.x = 9;`
    /// mutates a value of its own and `let b = ORIGIN;` gets another. Bound to
    /// one module object, the two uses shared an identity, a mutation and a
    /// release — the second `.drop()` on that object aborts the run. A `static`
    /// is the opposite: ONE place for the life of the program, and shared on
    /// purpose. Only a non-`Copy` `const` is fresh, and the emitted name is a
    /// function each use calls.
    pub fresh_at_each_use: bool,
}
