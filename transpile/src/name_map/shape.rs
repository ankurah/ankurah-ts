//! What a resolved Rust type becomes in JavaScript.
//!
//! One table, read by two consumers that used to carry a copy each: `emit_ty`,
//! which writes the TypeScript type, and `native_types`, which decides how a
//! method call on a value of that type is translated. When they disagreed, the
//! emitted type and the emitted call disagreed.
//!
//! Only *declared system types* get a special shape. A crate type or a foreign
//! type called `Vec` is its own type and is emitted and dispatched as itself.

use super::system_shapes::Form;
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
        Ty::Named { id, args } => named_shape(reg, *id, args),
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

/// The shape a declared std type takes, looked up by the type's identity.
///
/// The lookup is never by leaf name: `std::collections::hash_map::Iter` and
/// `std::slice::Iter` share one, `std::sync::atomic::Ordering` and
/// `std::cmp::Ordering` share one, and a crate type called `Vec` is its own
/// type. What each path becomes is `system_shapes`; how the arguments are read
/// out of it is here.
fn named_shape(reg: &TypeRegistry, id: crate::ty::TypeId, args: &[Ty]) -> JsShape {
    let Some(form) = reg.shapes().form(id) else {
        // Every iterator the port produces is a JavaScript array: `.iter()` and
        // `.values()` spread into one, and the adaptors a chain builds up —
        // `Cloned<Values<'_, K, V>>` — are written as operations on it. Which
        // types those are is not a list: it is whatever the impl table says
        // implements `Iterator`, and the element type is that impl's `Item`.
        return iterator_shape(reg, id, args).unwrap_or(JsShape::Plain);
    };
    let arg = |n: usize| args.get(n).cloned();
    match form {
        // `Vec<u8>` is a byte buffer; every other `Vec` is a JavaScript array.
        Form::VecOrBytes => match arg(0) {
            Some(Ty::Prim(Prim::U8)) => JsShape::Bytes,
            Some(elem) => JsShape::Array(elem),
            None => JsShape::Plain,
        },
        Form::Nullable => arg(0).map(JsShape::Nullable).unwrap_or(JsShape::Plain),
        Form::Result => JsShape::Result(args.to_vec()),
        Form::Map => match (arg(0), arg(1)) {
            (Some(k), Some(v)) => JsShape::Map(k, v),
            _ => JsShape::Plain,
        },
        Form::Set => arg(0).map(JsShape::Set).unwrap_or(JsShape::Plain),
        Form::Rc => JsShape::Rc(reg.name_of(id)),
        // `Box<T>` is transparent: the value is whatever it holds.
        Form::Transparent => arg(0).map(JsShape::SameAs).unwrap_or(JsShape::Plain),
        Form::Str => JsShape::Str,
        Form::Number => JsShape::Number,
        Form::Boolean => JsShape::Boolean,
        Form::Never => JsShape::Never,
    }
}

/// `Array(Item)` for a type that implements `Iterator`, and nothing for one
/// that does not.
fn iterator_shape(reg: &TypeRegistry, id: crate::ty::TypeId, args: &[Ty]) -> Option<JsShape> {
    // Only a declared type. A crate type that implements `Iterator` is emitted
    // as its own class with its own methods, so writing a call on it as an array
    // operation would be wrong — and asking the impl table about every crate
    // type would be the shape query's whole cost.
    if !reg.is_system(id) {
        return None;
    }
    let iterator = reg.system_type("std::iter::Iterator")?;
    let ty = Ty::Named {
        id,
        args: args.to_vec(),
    };
    // Asked from the crate root: which module the question comes from decides
    // trait visibility for a *call*, and this is not one.
    let probe = crate::registry::Probe::new(reg, reg.crate_root());
    let projection = Ty::Assoc {
        base: Box::new(ty),
        trait_: Some(Box::new(TraitRef {
            id: iterator,
            args: Vec::new(),
            bindings: Vec::new(),
        })),
        name: "Item".to_string(),
    };
    let item = probe.normalize(&projection);
    if item != projection {
        return Some(JsShape::Array(item));
    }
    // An adaptor whose element type is not settled yet is still an iterator, and
    // still a JavaScript array. `Map<I, F>`'s `Item` is the closure's return
    // type, which the closures step (spec 4.5) supplies; until then the element
    // is unknown and the chain around it is not.
    probe
        .implements(&Ty::Named { id, args: args.to_vec() }, iterator)
        .then(|| JsShape::Array(Ty::Infer))
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
