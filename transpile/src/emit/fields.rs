//! A struct's PROPERTIES and its constructor, written from one answer about
//! how each field is spelled.
//!
//! Split out of `emit.rs`. The property, the constructor parameter and the
//! construction site all have to agree about a field's type, and a field the
//! declaration walks as a CURSOR is the shape where they did not (FF4): the
//! property said `I`, which promises only what the bound promises, while the
//! caller handed over a `SeqCursor`.

use super::{is_phantom_field, FieldInfo, StructInfo};
use crate::body::cursors::field_spelling;
use crate::registry::TypeRegistry;
use std::collections::HashSet;

/// Writes the properties and the constructor, and hands back the fields that
/// are really there — a `PhantomData` field is not one, and everything below
/// (the drop glue, the codecs) reads the same list.
pub(super) fn emit_properties_and_constructor<'a>(
    out: &mut String,
    reg: &TypeRegistry,
    s: &'a StructInfo,
    self_id: Option<crate::ty::TypeId>,
    assigned: &HashSet<String>,
) -> Vec<&'a FieldInfo> {
    // Fields — Rust's "private" means module-private (same file), not class-private.
    // Since types within the same Rust module routinely access each other's fields,
    // we don't emit TS `private` — all fields are accessible (default public in TS classes).
    // A public Rust field is `readonly` for external consumers, unless one of
    // this type's own methods writes it: `fn drop(&mut self)` and every other
    // `&mut self` body assigns through the receiver, and TypeScript refuses
    // that on a `readonly` property.
    for f in &s.fields {
        if is_phantom_field(reg, f) { continue; }
        if let Some(name) = &f.name {
            if f.is_pub && !assigned.contains(name.as_str()) {
                out.push_str(&format!("  readonly {}: {};\n", name, field_spelling(reg, self_id, f)));
            } else {
                out.push_str(&format!("  {}: {};\n", name, field_spelling(reg, self_id, f)));
            }
        }
    }

    // Constructor with field assignments (skip PhantomData fields)
    let real_fields: Vec<&FieldInfo> = s.fields.iter().filter(|f| !is_phantom_field(reg, f)).collect();
    if !real_fields.is_empty() {
        out.push('\n');
        let params: Vec<String> = real_fields.iter()
            .filter_map(|f| f.name.as_ref().map(|n| format!("{}: {}", n, field_spelling(reg, self_id, f))))
            .collect();
        out.push_str(&format!("  constructor({}) {{\n    super();\n", params.join(", ")));
        for f in &real_fields {
            if let Some(name) = &f.name {
                out.push_str(&format!("    this.{} = {};\n", name, name));
            }
        }
        out.push_str("  }\n");
    }
    real_fields
}
