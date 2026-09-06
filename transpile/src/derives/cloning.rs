//! `#[derive(Clone)]`: one value copied, by the type the port writes it as.
//!
//! For: HOW a value is copied depends on what it IS. A `Uint8Array` is rebuilt
//! by its constructor, an array is mapped over, a runtime container has a
//! `clone()` that walks its own keys and values, a primitive is itself, and
//! everything else answers `clone()`. Written as a chain of top-level cases the
//! rule stopped ONE LEVEL DOWN: a `Vec<Vec<u8>>` field came out
//! `this.rows.map(e => e.clone())`, and a `Uint8Array` has no `clone()`. The
//! rule is the same at every depth, so it is asked again for whatever a
//! container holds.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;

/// One value cloned, from the place it is read from and its RESOLVED type.
///
/// The type is the resolved `Ty`, never the TypeScript the field is written
/// with. Reading the rendered text made the rule a parser of its own output:
/// `(Token | null)[]` is an array of nullables, and the `| null` suffix matched
/// the whole spelling, so `e.clone()` ran on a `null`.
///
/// A value whose type is one of the declaring type's own PARAMETERS is one the
/// copy emission cannot fix — `T` is a number in `Holder<u32>` and a class in
/// `Holder<Item>`, and `.clone()` on a number is a TypeError — so it goes to
/// `derivedClone`, which decides by the value's own surface at run time.
pub(crate) fn clone_within(reg: &TypeRegistry, place: &str, ty: Option<&Ty>) -> String {
    match ty {
        Some(ty) => clone_at(reg, place, ty, 0),
        // A field the engine could not type is reported where it is declared;
        // here it is a value of unknown surface, which is what `derivedClone`
        // answers for.
        None => format!("derivedClone({})", place),
    }
}

/// The same, told how deep inside a container it is, so a `map` inside a `map`
/// names its own element.
fn clone_at(reg: &TypeRegistry, place: &str, ty: &Ty, depth: usize) -> String {
    if matches!(ty, Ty::Param(_) | Ty::Assoc { .. } | Ty::Infer) {
        return format!("derivedClone({})", place);
    }
    // Every width, `char` and `bool` are their own copy. `char` is asked here
    // rather than through the shape table, which calls it `Plain` because it is
    // the one width the port writes as a string.
    if matches!(ty, Ty::Prim(_) | Ty::Str | Ty::Unit) {
        return place.to_string();
    }
    match js_shape(reg, ty) {
        JsShape::SameAs(inner) => clone_at(reg, place, &inner, depth),
        JsShape::Number | JsShape::BigInt | JsShape::Boolean | JsShape::Str | JsShape::Void => {
            place.to_string()
        }
        JsShape::Nullable(inner) => {
            let copy = clone_at(reg, place, &inner, depth);
            // A value that is its own copy — a primitive — is one whether or
            // not it is there, so there is nothing to guard.
            if copy == place {
                return copy;
            }
            // `x?.clone() ?? null` is the shorter spelling of the guard, and
            // the one the emitted files already carry, for the case that is
            // only a `clone()`.
            if copy == format!("{}.clone()", place) {
                return format!("{}?.clone() ?? null", place);
            }
            // Parenthesised: this stands as an argument, an initialiser and a
            // field of an object literal, and a ternary written bare next to
            // anything that binds tighter is a defect waiting for the day
            // something is written beside it.
            format!("({} != null ? {} : null)", place, copy)
        }
        JsShape::Bytes => format!("new Uint8Array({})", place),
        // The runtime container's own clone, which walks its keys and values by
        // their Clone shape. `new Map(...)` built a JavaScript `Map` —
        // identity-keyed, and shallow, so both maps then owned one set of
        // values.
        JsShape::Map(..) | JsShape::Set(_) => format!("{}.clone()", place),
        JsShape::Array(elem) => {
            let each = clone_at(reg, "\u{1}", &elem, depth + 1);
            if each == "\u{1}" {
                return format!("[...{}]", place);
            }
            let name = if depth == 0 { "e".to_string() } else { format!("e{}", depth) };
            format!("{}.map({} => {})", place, name, each.replace('\u{1}', &name))
        }
        JsShape::Tuple(parts) => {
            let cloned: Vec<String> = parts
                .iter()
                .enumerate()
                .map(|(at, part)| clone_at(reg, &format!("{}[{}]", place, at), part, depth + 1))
                .collect();
            format!(
                "[{}] as {}",
                cloned.join(", "),
                crate::name_map::map_ty(reg, ty)
            )
        }
        _ => format!("{}.clone()", place),
    }
}

/// `clone()` for an ENUM: the variant, and then its payload field by field.
///
/// The same rule a struct's derived clone uses, at every depth and told the
/// type's own PARAMETERS. Written out by hand in `emit.rs`, it stopped one level
/// down — a tuple field, a nested container, a `Vec` inside a nullable — and
/// knew nothing of a field written `T`, which is a number in `Slot<u32>` and a
/// class in `Slot<Item>` (fixpass4's §3.8, which reached the struct writer and
/// not this one). It also read a `Uint8Array` as needing no copy at all, so an
/// enum whose only non-primitive field was one took the shallow path and both
/// copies shared the buffer.
pub fn enum_clone(
    reg: &crate::registry::TypeRegistry,
    e: &crate::types::EnumInfo,
    self_type: &str,
) -> String {
    let copies: Vec<Vec<(String, String)>> = e
        .variants
        .iter()
        .map(|v| {
            v.fields
                .iter()
                .filter_map(|f| {
                    let name = f.name.clone()?;
                    let copy = clone_within(reg, &format!("v.{}", name), f.ty.as_ref());
                    Some((name, copy))
                })
                .collect()
        })
        .collect();
    // A variant whose every field is its own copy needs no walk: the record
    // spread is the whole clone.
    if !copies.iter().flatten().any(|(name, copy)| *copy != format!("v.{}", name)) {
        return format!(
            "\n  clone(): {} {{\n    return new {}(this.type, {{ ...this.value }});\n  }}\n",
            self_type, e.name
        );
    }
    // The class's own parameters are written on each construction. Without
    // them TypeScript infers a fresh one per arm from the payload — a variant
    // holding a `Vec<u8>` inferred `Slot<Uint8Array>`, which is not the
    // `Slot<T>` the signature promises.
    let arguments = match self_type.split_once('<') {
        Some((_, rest)) => format!("<{}", rest),
        None => String::new(),
    };
    let mut out = format!("\n  clone(): {} {{\n    return this.match({{\n", self_type);
    for (v, fields) in e.variants.iter().zip(&copies) {
        if v.fields.is_empty() {
            out.push_str(&format!(
                "      {}: () => new {}{}('{}', {{}}),\n",
                v.name, e.name, arguments, v.name
            ));
            continue;
        }
        let written: Vec<String> =
            fields.iter().map(|(name, copy)| format!("{}: {}", name, copy)).collect();
        out.push_str(&format!(
            "      {}: (v) => new {}{}('{}', {{ {} }}),\n",
            v.name,
            e.name,
            arguments,
            v.name,
            written.join(", ")
        ));
    }
    out.push_str("    });\n  }\n");
    out
}


#[cfg(test)]
mod tests {
    use super::clone_within;
    use crate::testing::Fixture;

    const PRELUDE: &str = "\
pub struct Tag { pub n: u32 }\n\
pub struct Id(pub u32);\n\
";

    /// One value copied, from a written Rust type resolved the way a field of
    /// it would be.
    fn clone_of(place: &str, rust_ty: &str) -> String {
        clone_in(place, rust_ty, &[])
    }

    /// The same, inside a type that declares parameters of its own.
    fn clone_in(place: &str, rust_ty: &str, params: &[&str]) -> String {
        let f = Fixture::build(&[("lib.rs", PRELUDE)]);
        let ty = f.ty_in("lib.rs", rust_ty, params).expect("resolves");
        clone_within(&f.reg, place, Some(&ty))
    }

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 3): the rule this pins stopped
    /// one level down — a `Vec<Vec<u8>>` came out `this.rows.map(e =>
    /// e.clone())`, and a `Uint8Array` has no `clone()`.
    #[test]
    fn a_container_inside_a_container_is_cloned_by_what_it_holds() {
        assert_eq!(
            clone_of("this.rows", "Vec<Vec<u8>>"),
            "this.rows.map(e => new Uint8Array(e))"
        );
        assert_eq!(clone_of("this.x", "Vec<u8>"), "new Uint8Array(this.x)");
        assert_eq!(
            clone_of("this.x", "Vec<Vec<Tag>>"),
            "this.x.map(e => e.map(e1 => e1.clone()))"
        );
        assert_eq!(clone_of("this.x", "Vec<u32>"), "[...this.x]");
        assert_eq!(clone_of("this.x", "Vec<Tag>"), "this.x.map(e => e.clone())");
    }

    /// A tuple is cloned element by element, and each element by its own rule.
    #[test]
    fn a_tuple_is_cloned_element_by_element() {
        assert_eq!(
            clone_of("this.x", "(u32, Tag)"),
            "[this.x[0], this.x[1].clone()] as [number, Tag]"
        );
        assert_eq!(
            clone_of("this.x", "(std::collections::HashMap<String, Tag>, u32)"),
            "[this.x[0].clone(), this.x[1]] as [HashMap<string, Tag>, number]"
        );
    }

    /// A field whose type is one of the declaring type's own PARAMETERS is one
    /// the emitter cannot copy, for the same reason it cannot compare it.
    #[test]
    fn a_field_written_as_a_type_parameter_is_copied_at_run_time() {
        assert_eq!(clone_in("this.x", "T", &["T"]), "derivedClone(this.x)");
        assert_eq!(
            clone_in("this.xs", "Vec<T>", &["T"]),
            "this.xs.map(e => derivedClone(e))"
        );
        assert_eq!(clone_in("this.x", "Id", &["T"]), "this.x.clone()");
    }

    /// A nullable is copied only where there is something to copy.
    #[test]
    fn a_nullable_is_copied_where_there_is_something_to_copy() {
        assert_eq!(
            clone_of("this.x", "Option<Vec<u8>>"),
            "(this.x != null ? new Uint8Array(this.x) : null)"
        );
        // A primitive is its own copy, there or not.
        assert_eq!(clone_of("this.x", "Option<u32>"), "this.x");
        // and the shorter spelling stands where the copy is only a `clone()`.
        assert_eq!(clone_of("this.x", "Option<Tag>"), "this.x?.clone() ?? null");
    }

    /// An array of NULLABLES is not a nullable array, and the difference was
    /// invisible in the rendered TypeScript the rule used to read: `(Tag |
    /// null)[]` ends in `[]`, and stripping a ` | null` suffix off it stripped
    /// the ELEMENT's, so `e.clone()` ran on a `null`.
    #[test]
    fn an_array_of_nullables_copies_each_element_where_it_is_there() {
        assert_eq!(
            clone_of("this.x", "Vec<Option<Tag>>"),
            "this.x.map(e => e?.clone() ?? null)"
        );
    }

    /// A type ALIAS is copied by what it stands for.
    #[test]
    fn an_alias_is_copied_by_what_it_stands_for() {
        let f = Fixture::build(&[("lib.rs", "pub type Bytes = Vec<u8>;\n")]);
        let ty = f.ty("lib.rs", "Bytes");
        assert_eq!(clone_within(&f.reg, "this.x", Some(&ty)), "new Uint8Array(this.x)");
    }

    /// A field the engine could not type is a value of unknown surface.
    #[test]
    fn a_field_with_no_resolved_type_is_copied_at_run_time() {
        let f = Fixture::build(&[("lib.rs", PRELUDE)]);
        assert_eq!(clone_within(&f.reg, "this.x", None), "derivedClone(this.x)");
    }
}
