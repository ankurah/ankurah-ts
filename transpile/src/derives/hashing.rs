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

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, FieldInfo, StructInfo};

/// `hash()` for a struct: its fields in declaration order.
pub fn struct_hash(reg: &TypeRegistry, s: &StructInfo) -> String {
    let parts = fields_hash(reg, &s.fields, &|name| format!("this.{}", name));
    format!(
        "\n  /** The key hash `HashMap` and `HashSet` file this under. */\n  \
         hash(): string {{\n    return [{}].join('|');\n  }}\n",
        parts.join(", ")
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
        out.push_str(&format!(
            "      case '{}': return ['{}', {}].join('|');\n",
            variant.name,
            variant.name,
            parts.join(", ")
        ));
    }
    out.push_str("    }\n    return String(this.type);\n  }\n");
    out
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
            match field.ty.as_ref().map(|ty| js_shape(reg, ty)) {
                // A value of the port's own hashes with its own derived
                // `hash()`, exactly as Rust's derive hashes a field with its.
                Some(JsShape::Plain) => format!("{}.hash()", place),
                // Everything else is a value `keyHash` already knows how to
                // read — a number, a string, a boolean, a sequence, a nullable.
                _ => format!("keyHash({})", place),
            }
        })
        .collect()
}
