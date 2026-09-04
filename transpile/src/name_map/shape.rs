//! What a resolved Rust type becomes in JavaScript.
//!
//! One table, read by two consumers that used to carry a copy each: `emit_ty`,
//! which writes the TypeScript type, and `native_types`, which decides how a
//! method call on a value of that type is translated. When they disagreed, the
//! emitted type and the emitted call disagreed.
//!
//! Only *declared system types* get a special shape. A crate type or a foreign
//! type called `Vec` is its own type and is emitted and dispatched as itself.

use crate::registry::TypeRegistry;
use crate::ty::{Prim, TraitRef, Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum JsShape {
    /// `Uint8Array`: `Vec<u8>`, and — a wart kept from the syntactic mapping —
    /// any slice or array whose element is written as a number.
    Bytes,
    Array(Ty),
    Nullable(Ty),
    Map(Ty, Ty),
    Set(Ty),
    /// `Arc` / `Weak` / `Rc`, which emission writes like any other named type
    /// and the native translations dispatch on by name.
    Rc(String),
    Result(Vec<Ty>),
    Tuple(Vec<Ty>),
    Fn {
        params: Vec<Ty>,
        ret: Ty,
    },
    Future(Option<Ty>),
    /// A trait object or `impl Trait` whose bound says nothing about shape;
    /// emission writes the trait's name.
    Trait(String),
    /// This type is written exactly as another one: `Box<T>` is `T`, and so is
    /// `impl Into<T>`.
    SameAs(Ty),
    Str,
    Number,
    /// `u64` / `i64`, which the port still emits as `bigint | number`.
    BigInt,
    Boolean,
    Void,
    Never,
    Unknown,
    /// Nothing special: emission writes the type's own name, and no
    /// native-type translation applies.
    Plain,
}

pub fn js_shape(reg: &TypeRegistry, ty: &Ty) -> JsShape {
    match ty {
        Ty::Ref { inner, .. } => JsShape::SameAs((**inner).clone()),
        Ty::Str => JsShape::Str,
        Ty::Unit => JsShape::Void,
        Ty::Never => JsShape::Never,
        Ty::Infer => JsShape::Unknown,
        Ty::Param(_) | Ty::Assoc { .. } => JsShape::Plain,
        Ty::Tuple(elems) => JsShape::Tuple(elems.clone()),
        Ty::Prim(p) => prim_shape(*p),
        Ty::Slice(elem) | Ty::Array { elem, .. } => {
            if js_shape(reg, elem) == JsShape::Number {
                JsShape::Bytes
            } else {
                JsShape::Array((**elem).clone())
            }
        }
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits
            .iter()
            .find_map(|t| trait_shape(reg, t))
            .unwrap_or(JsShape::Plain),
        Ty::Named { id, args } => {
            if !reg.is_system(*id) {
                return JsShape::Plain;
            }
            named_shape(&reg.name_of(*id), args)
        }
    }
}

fn prim_shape(p: Prim) -> JsShape {
    match p {
        Prim::U8
        | Prim::U16
        | Prim::U32
        | Prim::Usize
        | Prim::I8
        | Prim::I16
        | Prim::I32
        | Prim::F32
        | Prim::F64 => JsShape::Number,
        Prim::U64 | Prim::I64 => JsShape::BigInt,
        Prim::Bool => JsShape::Boolean,
        // `char`, `isize` and the 128-bit widths have no mapping of their own.
        Prim::Char | Prim::Isize | Prim::U128 | Prim::I128 => JsShape::Plain,
    }
}

fn named_shape(name: &str, args: &[Ty]) -> JsShape {
    match name {
        // `Vec<u8>` is a byte buffer; every other `Vec` is a JavaScript array.
        "Vec" if args.len() == 1 => {
            if matches!(args[0], Ty::Prim(Prim::U8)) {
                JsShape::Bytes
            } else {
                JsShape::Array(args[0].clone())
            }
        }
        "Option" if args.len() == 1 => JsShape::Nullable(args[0].clone()),
        "Result" => JsShape::Result(args.to_vec()),
        "HashMap" | "BTreeMap" if args.len() == 2 => JsShape::Map(args[0].clone(), args[1].clone()),
        "HashSet" | "BTreeSet" if args.len() == 1 => JsShape::Set(args[0].clone()),
        "Arc" | "Weak" | "Rc" => JsShape::Rc(name.to_string()),
        "String" => JsShape::Str,
        // Box is transparent: the value is whatever it holds.
        "Box" if args.len() == 1 => JsShape::SameAs(args[0].clone()),
        // Atomics are plain values in single-threaded JavaScript.
        "AtomicUsize" | "AtomicU32" => JsShape::Number,
        "AtomicBool" => JsShape::Boolean,
        "Infallible" => JsShape::Never,
        _ => JsShape::Plain,
    }
}

/// The shape a trait bound implies for the value behind it. `None` when this
/// bound says nothing, so the caller moves on to the next one — that is how
/// `dyn Fn(T) + Send + Sync` stays a function.
fn trait_shape(reg: &TypeRegistry, tr: &TraitRef) -> Option<JsShape> {
    let name = reg.name_of(tr.id);
    let binding = |key: &str| {
        tr.bindings
            .iter()
            .find(|(n, _)| n == key)
            .map(|(_, t)| t.clone())
    };
    match name.as_str() {
        "Fn" | "FnMut" | "FnOnce" => {
            let ret = binding("Output")?;
            let params = match tr.args.first()? {
                Ty::Unit => Vec::new(),
                Ty::Tuple(elems) => elems.clone(),
                single => vec![single.clone()],
            };
            Some(JsShape::Fn { params, ret })
        }
        "Into" | "AsRef" => tr.args.first().cloned().map(JsShape::SameAs),
        "Iterator" | "IntoIterator" => binding("Item").map(JsShape::Array),
        "Future" => Some(JsShape::Future(binding("Output"))),
        _ => Some(JsShape::Trait(name)),
    }
}
