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

use super::top_level_parts;

/// One value cloned, from the place it is read from.
pub(crate) fn clone_of(place: &str, ty: &str) -> String {
    clone_at(place, ty, 0, &[])
}

/// The same, told the declaring type's own PARAMETERS.
///
/// A field written as `T` is one whose copy emission cannot fix: `T` is a number
/// in `Holder<u32>` and a class in `Holder<Item>`, and `.clone()` on a number is
/// a TypeError. `cloned` decides at run time, by the value's own surface.
pub(crate) fn clone_within(place: &str, ty: &str, params: &[String]) -> String {
    clone_at(place, ty, 0, params)
}

/// The same, told how deep inside a container it is, so a `map` inside a `map`
/// names its own element.
fn clone_at(place: &str, ty: &str, depth: usize, params: &[String]) -> String {
    let ty = ty.trim();
    if params.iter().any(|p| p == ty) {
        return format!("derivedClone({})", place);
    }
    if let Some(inner) = ty.strip_suffix("| null").map(str::trim) {
        let copy = clone_at(place, inner, depth, params);
        // A value that is its own copy — a primitive — is one whether or not it
        // is there, so there is nothing to guard.
        if copy == place {
            return copy;
        }
        // `x?.clone() ?? null` is the shorter spelling of the guard, and the
        // one the emitted files already carry, for the case that is only a
        // `clone()`.
        if copy == format!("{}.clone()", place) {
            return format!("{}?.clone() ?? null", place);
        }
        // Parenthesised: this stands as an argument, an initialiser and a
        // field of an object literal, and a ternary written bare next to
        // anything that binds tighter is a defect waiting for the day something
        // is written beside it.
        return format!("({} != null ? {} : null)", place, copy);
    }
    if crate::emit::is_primitive_ts_type(ty) {
        return place.to_string();
    }
    if ty == "Uint8Array" {
        return format!("new Uint8Array({})", place);
    }
    // The runtime container's own clone, which walks its keys and values by
    // their Clone shape. `new Map(...)` built a JavaScript `Map` —
    // identity-keyed, and shallow, so both maps then owned one set of values.
    if ["HashMap<", "HashSet<", "BTreeMap<", "BTreeSet<"].iter().any(|head| ty.starts_with(head)) {
        return format!("{}.clone()", place);
    }
    if let Some(inner) = ty.strip_suffix("[]") {
        if crate::emit::is_primitive_ts_type(inner.trim()) {
            return format!("[...{}]", place);
        }
        let element = if depth == 0 { "e".to_string() } else { format!("e{}", depth) };
        return format!("{}.map({} => {})", place, element, clone_at(&element, inner, depth + 1, params));
    }
    if let Some(parts) = tuple_parts(ty) {
        let cloned: Vec<String> = parts
            .iter()
            .enumerate()
            .map(|(at, part)| clone_at(&format!("{}[{}]", place, at), part, depth + 1, params))
            .collect();
        return format!("[{}] as {}", cloned.join(", "), ty);
    }
    format!("{}.clone()", place)
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
    params: &[String],
) -> String {
    let copies: Vec<Vec<(String, String)>> = e
        .variants
        .iter()
        .map(|v| {
            v.fields
                .iter()
                .filter_map(|f| {
                    let name = f.name.clone()?;
                    let copy = clone_within(&format!("v.{}", name), &f.ts_ty(reg), params);
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

/// The element types of a written tuple, or nothing where the type is not one.
///
/// A ONE-element tuple is a tuple: `(Owned,)` is written `[Owned]` here, and
/// requiring two parts sent it to `.clone()` on a JavaScript array. The written
/// form an array shares — `T[]` — is not this shape, because the brackets are a
/// suffix there and this asks for them around the whole type.
pub(crate) fn tuple_parts(ty: &str) -> Option<Vec<String>> {
    let inner = ty.trim().strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return None;
    }
    Some(top_level_parts(inner))
}

#[cfg(test)]
mod tests {
    use super::clone_of;

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 3): the rule this pins stopped
    /// one level down — a `Vec<Vec<u8>>` came out `this.rows.map(e =>
    /// e.clone())`, and a `Uint8Array` has no `clone()`.
    #[test]
    fn a_container_inside_a_container_is_cloned_by_what_it_holds() {
        assert_eq!(clone_of("this.rows", "Uint8Array[]"), "this.rows.map(e => new Uint8Array(e))");
        assert_eq!(clone_of("this.x", "Uint8Array"), "new Uint8Array(this.x)");
        assert_eq!(clone_of("this.x", "Tag[][]"), "this.x.map(e => e.map(e1 => e1.clone()))");
        assert_eq!(clone_of("this.x", "number[]"), "[...this.x]");
        assert_eq!(clone_of("this.x", "Tag[]"), "this.x.map(e => e.clone())");
    }

    /// A tuple is cloned element by element, and each element by its own rule.
    #[test]
    fn a_tuple_is_cloned_element_by_element() {
        assert_eq!(
            clone_of("this.x", "[number, Tag]"),
            "[this.x[0], this.x[1].clone()] as [number, Tag]"
        );
        // A nested argument list is not a separator.
        assert_eq!(
            clone_of("this.x", "[HashMap<string, Tag>, number]"),
            "[this.x[0].clone(), this.x[1]] as [HashMap<string, Tag>, number]"
        );
    }

    /// A field written as one of the type's own PARAMETERS is one the emitter
    /// cannot copy, for the same reason it cannot compare it.
    #[test]
    fn a_field_written_as_a_type_parameter_is_copied_at_run_time() {
        use super::clone_within;
        let params = vec!["T".to_string()];
        assert_eq!(clone_within("this.x", "T", &params), "derivedClone(this.x)");
        assert_eq!(clone_within("this.xs", "T[]", &params), "this.xs.map(e => derivedClone(e))");
        assert_eq!(clone_within("this.x", "Id", &params), "this.x.clone()");
    }

    /// A nullable is copied only where there is something to copy.
    #[test]
    fn a_nullable_is_copied_where_there_is_something_to_copy() {
        assert_eq!(
            clone_of("this.x", "Uint8Array | null"),
            "(this.x != null ? new Uint8Array(this.x) : null)"
        );
        // A primitive is its own copy, there or not.
        assert_eq!(clone_of("this.x", "number | null"), "this.x");
        // and the shorter spelling stands where the copy is only a `clone()`.
        assert_eq!(clone_of("this.x", "Tag | null"), "this.x?.clone() ?? null");
    }
}
