//! What `#[derive(Default)]` puts in each field.
//!
//! For: Rust's derive fills every field with `<that field's type>::default()`,
//! and the port has to fill it with the same value. The type decides what that
//! value is, so this reads the *type* — an empty `Map` for a map, `0` for a
//! number, `Arc.new(..)` around whatever the `Arc` holds — rather than the
//! type's TypeScript spelling. Reading the spelling wrote
//! `Arc<RwLock<Map<EntityId, WeakEntity>>>.default()`, which names a type where
//! a value belongs and does not parse.
//!
//! Where the port has no default for a type, that is said rather than guessed
//! at: a wrong default is a value the program then runs on.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// The TypeScript expression for this type's `Default::default()`, or the
/// reason the port cannot write one.
pub fn default_value(reg: &TypeRegistry, ty: &Ty) -> Result<String, String> {
    match js_shape(reg, ty) {
        JsShape::SameAs(inner) => default_value(reg, &inner),
        JsShape::Nullable(_) => Ok("null".to_string()),
        JsShape::Str => Ok("''".to_string()),
        JsShape::Number => Ok("0".to_string()),
        JsShape::BigInt => Ok("0n".to_string()),
        JsShape::Boolean => Ok("false".to_string()),
        JsShape::Bytes => Ok("new Uint8Array(0)".to_string()),
        JsShape::Array(_) => Ok("[]".to_string()),
        JsShape::Map(..) => Ok("new Map()".to_string()),
        JsShape::Set(_) => Ok("new Set()".to_string()),
        // An `Arc<T>` holds one `T`, and its default is a fresh `Arc` around
        // that type's default. The runtime spells the constructor `Arc.new`.
        JsShape::Rc(name) => {
            let Ty::Named { args, .. } = ty.peel_refs() else {
                return Err(format!("`{}` holds nothing the engine could name", name));
            };
            let Some(inner) = args.first() else {
                return Err(format!("`{}` holds nothing the engine could name", name));
            };
            Ok(format!("{}.new({})", name, default_value(reg, inner)?))
        }
        // A named type with a `Default` of its own: the class's static.
        JsShape::Plain => named_default(reg, ty),
        other => Err(format!(
            "the port writes `{}` as {:?}, which has no default value",
            reg.describe(ty),
            other
        )),
    }
}

/// A named type's own `default()`, where the port emits a class with one.
///
/// A `Mutex` or an `RwLock` is a wrapper the runtime constructs directly, and
/// its default is the wrapper around its contents' default; anything else has
/// to have a `Default` impl for there to be a static to call.
fn named_default(reg: &TypeRegistry, ty: &Ty) -> Result<String, String> {
    let Ty::Named { id, args } = ty.peel_refs() else {
        return Err(format!("`{}` is not a named type", reg.describe(ty)));
    };
    let path = reg.name_of(*id);
    let leaf = path.rsplit("::").next().unwrap_or(&path).to_string();
    for wrapper in ["Mutex", "RwLock", "RefCell", "Cell"] {
        if leaf == wrapper {
            let Some(inner) = args.first() else {
                return Err(format!("`{}` holds nothing the engine could name", wrapper));
            };
            return Ok(format!("new {}({})", wrapper, default_value(reg, inner)?));
        }
    }
    if reg.modules().get(reg.def(*id).map(|d| d.module).ok_or_else(|| {
        format!("`{}` is not declared, so it has no default", reg.describe(ty))
    })?).is_system
    {
        return Err(format!(
            "`{}` is a declared std type, and `@ankurah/base` supplies no default for it",
            reg.describe(ty)
        ));
    }
    Ok(format!("{}.default()", crate::name_map::map_ty(reg, ty.peel_refs())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn value(src: &str, struct_name: &str, field: &str) -> Result<String, String> {
        let f = Fixture::build(&[("lib.rs", src)]);
        let ty = f.field("lib.rs", struct_name, field);
        default_value(&f.reg, &ty)
    }

    #[test]
    fn a_map_defaults_to_an_empty_map_however_deeply_it_is_wrapped() {
        let got = value(
            "use std::collections::BTreeMap;\n\
             use std::sync::{Arc, RwLock};\n\
             pub struct Holder { pub inner: Arc<RwLock<BTreeMap<u32, u32>>> }",
            "Holder",
            "inner",
        );
        assert_eq!(got, Ok("Arc.new(new RwLock(new Map()))".to_string()));
    }

    #[test]
    fn the_primitives_default_to_their_own_zero() {
        let src = "pub struct Holder { pub a: u32, pub b: u64, pub c: bool, pub d: String }";
        assert_eq!(value(src, "Holder", "a"), Ok("0".to_string()));
        assert_eq!(value(src, "Holder", "b"), Ok("0n".to_string()));
        assert_eq!(value(src, "Holder", "c"), Ok("false".to_string()));
        assert_eq!(value(src, "Holder", "d"), Ok("''".to_string()));
    }

    #[test]
    fn an_option_defaults_to_null_and_a_vec_to_an_empty_array() {
        let src = "pub struct Holder { pub a: Option<u32>, pub b: Vec<String>, pub c: Vec<u8> }";
        assert_eq!(value(src, "Holder", "a"), Ok("null".to_string()));
        assert_eq!(value(src, "Holder", "b"), Ok("[]".to_string()));
        assert_eq!(value(src, "Holder", "c"), Ok("new Uint8Array(0)".to_string()));
    }

    #[test]
    fn a_crate_type_defaults_through_its_own_static() {
        let got = value(
            "#[derive(Default)] pub struct Inner { pub n: u32 }\n\
             pub struct Holder { pub inner: Inner }",
            "Holder",
            "inner",
        );
        assert_eq!(got, Ok("Inner.default()".to_string()));
    }

    #[test]
    fn a_declared_std_type_with_no_default_says_so() {
        let got = value(
            "use std::sync::OnceLock;\npub struct Holder { pub d: OnceLock<u32> }",
            "Holder",
            "d",
        );
        assert!(got.is_err(), "{:?}", got);
    }
}
