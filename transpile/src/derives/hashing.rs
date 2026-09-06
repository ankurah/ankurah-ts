//! `#[derive(Hash)]`, written out.
//!
//! For: the port's `HashMap` and `HashSet` are keyed containers, not
//! JavaScript's `Map` and `Set` — a `Map` compares its keys by identity, and a
//! key read back off the wire is never the object it was stored under. They
//! hash a key with its own `hash()` and compare with its own `equals()`, and a
//! key that declares no `hash()` is refused by name at run time rather than
//! silently answering nothing. So every type Rust derives `Hash` for owes one.
//!
//! Rust's derive feeds every field to the hasher in declaration order, and its
//! `Eq` compares the same fields in the same order. Both halves have to agree:
//! two values that are `equals` must hash alike, or a lookup misses a key that
//! is there. The text below is written from the same field list `equals` is.
//!
//! The parts are LENGTH-PREFIXED, not joined by a separator. Rust's hasher is
//! fed each field's bytes in turn and the field boundaries are part of what it
//! sees; a separator has to be a byte the parts cannot contain, and a `String`
//! field can contain any of them. Joined with `|`, `Pair("x|s:y", "z")` and
//! `Pair("x", "y|s:z")` hashed alike — two different keys in one bucket, which
//! the map then tells apart by `equals` only if it looks, and answers the wrong
//! value for if it does not.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, FieldInfo, StructInfo};

/// `hash()` for a struct: its fields in declaration order.
pub fn struct_hash(reg: &TypeRegistry, s: &StructInfo) -> String {
    let parts = fields_hash(reg, &s.fields, &|name| format!("this.{}", name));
    format!(
        "\n  /** The key hash `HashMap` and `HashSet` file this under. */\n  \
         hash(): string {{\n    return {};\n  }}\n",
        joined(&parts)
    )
}

/// `hash()` for an enum: the variant, and then the payload of that variant.
///
/// Rust hashes the discriminant first, which is what keeps two variants
/// carrying the same payload apart.
pub fn enum_hash(reg: &TypeRegistry, e: &EnumInfo) -> String {
    let mut out = String::from(
        "\n  /** The key hash `HashMap` and `HashSet` file this under. */\n  hash(): string {\n",
    );
    let carrying: Vec<&crate::types::VariantInfo> =
        e.variants.iter().filter(|v| !v.fields.is_empty()).collect();
    if carrying.is_empty() {
        out.push_str("    return String(this.type);\n  }\n");
        return out;
    }
    out.push_str("    switch (this.type) {\n");
    for variant in carrying {
        let parts = fields_hash(reg, &variant.fields, &|name| {
            format!("(this.value as any).{}", name)
        });
        let mut carried = vec![format!("'{}'", variant.name)];
        carried.extend(parts);
        out.push_str(&format!(
            "      case '{}': return {};\n",
            variant.name,
            joined(&carried)
        ));
    }
    out.push_str("    }\n    return String(this.type);\n  }\n");
    out
}

/// The parts as one string, each prefixed by its own length.
///
/// `2:ab3:cde` can only have been `["ab", "cde"]`, whatever the parts hold.
fn joined(parts: &[String]) -> String {
    format!("[{}].map((p) => p.length + ':' + p).join('')", parts.join(", "))
}

/// Each field as a string the hash is built from.
fn fields_hash(
    reg: &TypeRegistry,
    fields: &[FieldInfo],
    read: &dyn Fn(&str) -> String,
) -> Vec<String> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let name = field.name.clone().unwrap_or_else(|| format!("_{}", index));
            let place = read(&name);
            let Some(ty) = field.ty.as_ref() else {
                // A field the engine could not type is a value of unknown
                // surface, which is what `derivedHash` answers for.
                return format!("derivedHash({})", place);
            };
            // A bare type PARAMETER is not a class. `T` is a number in
            // `Keyed<u32>` and a class in `Keyed<Item>`, and `hash()` on a
            // number is a TypeError — the same question `derivedEquals` and
            // `derivedClone` answer for their halves of the derive, and the one
            // this half was written without. A `char` is asked here too: the
            // port writes it as a string, and the shape table calls it `Plain`.
            if matches!(ty, crate::ty::Ty::Param(_) | crate::ty::Ty::Assoc { .. } | crate::ty::Ty::Infer) {
                return format!("derivedHash({})", place);
            }
            if matches!(ty, crate::ty::Ty::Prim(_) | crate::ty::Ty::Str | crate::ty::Ty::Unit) {
                return format!("keyHash({})", place);
            }
            match js_shape(reg, ty) {
                // A value of the port's own hashes with its own derived
                // `hash()`, exactly as Rust's derive hashes a field with its.
                JsShape::Plain => format!("{}.hash()", place),
                // Everything else is a value `keyHash` already knows how to
                // read — a number, a string, a boolean, a sequence, a nullable.
                _ => format!("keyHash({})", place),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    fn struct_hash_of(src: &str, name: &str) -> String {
        let f = Fixture::build(&[("lib.rs", src)]);
        let s = f.files[0].file.structs.iter().find(|s| s.name == name).expect("struct");
        super::struct_hash(&f.reg, s)
    }

    fn enum_hash_of(src: &str, name: &str) -> String {
        let f = Fixture::build(&[("lib.rs", src)]);
        let e = f.files[0].file.enums.iter().find(|e| e.name == name).expect("enum");
        super::enum_hash(&f.reg, e)
    }

    /// A1.11: joined by a separator, `Pair("x|s:y", "z")` and
    /// `Pair("x", "y|s:z")` hashed alike — two different keys in one bucket. A
    /// `String` field can contain any byte, so no separator is safe and the
    /// parts carry their own lengths instead.
    #[test]
    fn the_parts_carry_their_own_lengths() {
        let ts = struct_hash_of("pub struct Pair { pub a: String, pub b: String }", "Pair");
        assert!(
            ts.contains("[keyHash(this.a), keyHash(this.b)].map((p) => p.length + ':' + p).join('')"),
            "{}",
            ts
        );
        assert!(!ts.contains("join('|')"), "no separator is left:\n{}", ts);
    }

    /// Rust hashes the discriminant first, which is what keeps two variants
    /// carrying the same payload apart; it is a part like any other, so it is
    /// length-prefixed too.
    #[test]
    fn a_variant_name_is_the_first_part() {
        let ts = enum_hash_of("pub enum Tag { One(String), Two, Three(String) }", "Tag");
        assert!(
            ts.contains("case 'One': return ['One', keyHash((this.value as any)._0)]"),
            "{}",
            ts
        );
        assert!(ts.contains("case 'Three': return ['Three',"), "{}", ts);
        assert!(!ts.contains("case 'Two'"), "a variant with no payload has no parts:\n{}", ts);
        assert_eq!(
            ts.matches(".map((p) => p.length + ':' + p).join('')").count(),
            2,
            "every arm that builds parts prefixes them:\n{}",
            ts
        );
    }

    /// A field whose type is a bare PARAMETER is not a class: `T` is a number
    /// in `Keyed<u32>` and a class in `Keyed<Tag>`, and `hash()` on a number is
    /// a TypeError — on the very path a `HashMap` takes to file a key. The
    /// `equals` and `clone` halves of the derive learned this in the fourth
    /// pass; this half was written without it.
    #[test]
    fn a_field_whose_type_is_a_parameter_hashes_by_its_surface() {
        let ts = struct_hash_of("pub struct Keyed<T> { pub key: T, pub n: u32 }", "Keyed");
        assert!(ts.contains("derivedHash(this.key)"), "{}", ts);
        assert!(!ts.contains("this.key.hash()"), "{}", ts);
        assert!(ts.contains("keyHash(this.n)"), "{}", ts);
    }

    /// A `char` is the one width the port writes as a string, and the shape
    /// table calls it `Plain` for that reason — so asking the shape alone said
    /// "a class of the port's own" and wrote `.hash()` on a string.
    #[test]
    fn a_char_field_hashes_as_the_string_it_is_written_as() {
        let ts = struct_hash_of("pub struct Ch { pub c: char }", "Ch");
        assert!(ts.contains("keyHash(this.c)"), "{}", ts);
    }
}
