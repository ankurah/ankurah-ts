//! Bincode rewrite module
//!
//! For structs/enums with #[derive(Serialize, Deserialize)], generates
//! encode(writer: BincodeWriter) and static decode(reader: BincodeReader) methods.
//!
//! The encoding is field-by-field in declaration order (bincode's default).
//! Custom serde impls (like EntityId's raw bytes) are detected and skipped —
//! those types must provide their own encode/decode.

use crate::types::{StructInfo, EnumInfo};
use crate::name_map;

/// Check if a struct/enum has derive(Serialize, Deserialize)
pub fn has_serde_derive(derives: &[String]) -> bool {
    derives.iter().any(|d| d == "Serialize") &&
    derives.iter().any(|d| d == "Deserialize")
}

/// Generate encode/decode methods for a struct with named fields
pub fn generate_struct_codec(info: &StructInfo) -> String {
    let mut out = String::new();

    // encode
    out.push_str("  encode(writer: BincodeWriter): void {\n");
    for field in &info.fields {
        if let Some(name) = &field.name {
            out.push_str(&format!("    {};\n", encode_expr(&format!("this.{}", name), &field.ty)));
        }
    }
    out.push_str("  }\n\n");

    // decode
    let full_name = format!("{}{}", info.name, info.generics);
    out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
    for field in &info.fields {
        if let Some(name) = &field.name {
            out.push_str(&format!("    const {} = {};\n", name, decode_expr(&field.ty)));
        }
    }
    let field_names: Vec<&str> = info.fields.iter()
        .filter_map(|f| f.name.as_deref())
        .collect();
    out.push_str(&format!("    return new {}({});\n", info.name, field_names.join(", ")));
    out.push_str("  }\n");

    out
}

/// Generate encode/decode methods for a tuple struct (e.g., Clock(Vec<EventId>))
pub fn generate_tuple_struct_codec(info: &StructInfo) -> String {
    let mut out = String::new();
    let full_name = format!("{}{}", info.name, info.generics);

    if info.fields.len() == 1 {
        let field = &info.fields[0];
        let field_name = field.name.as_deref().unwrap_or("_0");

        out.push_str("  encode(writer: BincodeWriter): void {\n");
        out.push_str(&format!("    {};\n", encode_expr(&format!("this.{}", field_name), &field.ty)));
        out.push_str("  }\n\n");

        out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
        out.push_str(&format!("    const {} = {};\n", field_name, decode_expr(&field.ty)));
        out.push_str(&format!("    return new {}({});\n", info.name, field_name));
        out.push_str("  }\n");
    } else {
        out.push_str("  encode(writer: BincodeWriter): void {\n");
        for field in &info.fields {
            let name = field.name.as_deref().unwrap_or("_0");
            out.push_str(&format!("    {};\n", encode_expr(&format!("this.{}", name), &field.ty)));
        }
        out.push_str("  }\n\n");

        out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
        let mut names = Vec::new();
        for field in &info.fields {
            let name = field.name.as_deref().unwrap_or("_0");
            out.push_str(&format!("    const {} = {};\n", name, decode_expr(&field.ty)));
            names.push(name);
        }
        out.push_str(&format!("    return new {}({});\n", info.name, names.join(", ")));
        out.push_str("  }\n");
    }

    out
}

/// Generate encode/decode methods for an enum
pub fn generate_enum_codec(info: &EnumInfo) -> String {
    let mut out = String::new();

    // Find serde(other) variant if any
    let serde_other_variant = info.variants.iter().find(|v| v.is_serde_other);

    // encode — match on variant, write discriminant + fields
    out.push_str("  encode(writer: BincodeWriter): void {\n");
    out.push_str("    this.match({\n");
    for (i, variant) in info.variants.iter().enumerate() {
        if variant.is_serde_other {
            // serde(other) variants are decode-only catch-alls — encoding throws
            out.push_str(&format!("      {}: () => {{\n", variant.name));
            out.push_str(&format!("        throw new Error('Cannot encode {}::{} — it is a decode-only catch-all');\n", info.name, variant.name));
            out.push_str("      },\n");
        } else {
            out.push_str(&format!("      {}: (v) => {{\n", variant.name));
            out.push_str(&format!("        writer.writeVariant({});\n", i));
            for field in &variant.fields {
                if let Some(name) = &field.name {
                    out.push_str(&format!("        {};\n", encode_expr(&format!("v.{}", name), &field.ty)));
                }
            }
            out.push_str("      },\n");
        }
    }
    out.push_str("    });\n");
    out.push_str("  }\n\n");

    // decode — read discriminant, switch on variant
    let full_name = format!("{}{}", info.name, info.generics);
    out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
    out.push_str("    const variant = reader.readVariant();\n");
    out.push_str("    switch (variant) {\n");
    for (i, variant) in info.variants.iter().enumerate() {
        if variant.is_serde_other { continue; } // handled as default case
        out.push_str(&format!("      case {}: {{\n", i));
        for field in &variant.fields {
            if let Some(name) = &field.name {
                out.push_str(&format!("        const {} = {};\n", name, decode_expr(&field.ty)));
            }
        }
        if variant.fields.is_empty() {
            out.push_str(&format!("        return new {}('{}', {{}});\n", info.name, variant.name));
        } else {
            let field_obj: Vec<String> = variant.fields.iter()
                .filter_map(|f| f.name.as_deref().map(|n| n.to_string()))
                .collect();
            out.push_str(&format!("        return new {}('{}', {{ {} }});\n",
                info.name, variant.name, field_obj.join(", ")));
        }
        out.push_str("      }\n");
    }
    // Default case: serde(other) catch-all or throw
    if let Some(other) = serde_other_variant {
        out.push_str(&format!("      default: return new {}('{}', {{}});\n", info.name, other.name));
    } else {
        out.push_str(&format!("      default: throw new Error(`Unknown {} variant: ${{variant}}`);\n", info.name));
    }
    out.push_str("    }\n");
    out.push_str("  }\n");

    out
}

/// Generate the encode expression for a value of a given TS type
/// `wr` is the writer variable name (e.g., "writer" at top level, "w" inside callbacks)
fn encode_expr_with(value: &str, ts_type: &str, wr: &str) -> String {
    match ts_type {
        "string" => format!("{}.writeString({})", wr, value),
        "boolean" => format!("{}.writeBool({})", wr, value),
        "number" => format!("{}.writeU32({})", wr, value),
        "bigint | number" => format!("{}.writeU64({})", wr, value),
        "Uint8Array" => format!("{}.writeByteVec({})", wr, value),
        t if t.ends_with("[]") => {
            let inner = &t[..t.len()-2];
            if is_primitive_type(inner) {
                format!("{}.writeVec({}, (w, item) => w.write{}(item))", wr, value, capitalize(inner))
            } else {
                format!("{}.writeVec({}, (w, item) => {})", wr, value,
                    encode_expr_with("item", inner, "w"))
            }
        }
        t if t.starts_with("Map<") => {
            // BTreeMap: sorted by key in Rust. Encode as length + sorted entries.
            let inner = &t[4..t.len()-1];
            let parts: Vec<&str> = inner.splitn(2, ", ").collect();
            if parts.len() == 2 {
                let k_enc = encode_expr_with("k", parts[0], wr);
                let v_enc = encode_expr_with("v", parts[1], wr);
                format!("{{ const _entries = [...{}.entries()].sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0); {}.writeLength(_entries.length); for (const [k, v] of _entries) {{ {}; {}; }} }}",
                    value, wr, k_enc, v_enc)
            } else {
                format!("/* TODO: Map encode */ {}.writeLength({}.size)", wr, value)
            }
        }
        t if t.ends_with(" | null") => {
            let inner = &t[..t.len()-7];
            format!("{}.writeOption({}, (w, v) => {})", wr, value,
                encode_expr_with("v", inner, "w"))
        }
        t if t.starts_with("Attested<") => {
            // Attested<T> requires callback: v.encode(w, (w2, p) => p.encode(w2))
            let inner = &t[9..t.len()-1]; // extract T from Attested<T>
            let wr2 = next_writer_var(wr);
            format!("{}.encode({}, ({}, p) => {})",
                value, wr, wr2,
                encode_expr_with("p", inner, &wr2))
        }
        t if is_tuple_type(t) => {
            // Tuple type [A, B] — encode each element
            let inner = &t[1..t.len()-1];
            let parts: Vec<&str> = inner.split(", ").collect();
            let encodes: Vec<String> = parts.iter().enumerate()
                .map(|(i, ty)| encode_expr_with(&format!("{}[{}]", value, i), ty.trim(), wr))
                .collect();
            format!("{{ {} }}", encodes.join("; "))
        }
        _ => {
            // Assume the type has its own encode method
            format!("{}.encode({})", value, wr)
        }
    }
}

/// Top-level encode_expr using "writer" as the variable name
fn encode_expr(value: &str, ts_type: &str) -> String {
    encode_expr_with(value, ts_type, "writer")
}

/// Generate the decode expression for a given TS type
/// `rd` is the reader variable name
fn decode_expr_with(ts_type: &str, rd: &str) -> String {
    match ts_type {
        "string" => format!("{}.readString()", rd),
        "boolean" => format!("{}.readBool()", rd),
        "number" => format!("{}.readU32()", rd),
        "bigint | number" => format!("{}.readU64()", rd),
        "Uint8Array" => format!("{}.readByteVec()", rd),
        t if t.ends_with("[]") => {
            let inner = &t[..t.len()-2];
            if is_primitive_type(inner) {
                format!("{}.readVec((r) => r.read{}())", rd, capitalize(inner))
            } else {
                format!("{}.readVec((r) => {})", rd, decode_expr_with(inner, "r"))
            }
        }
        t if t.starts_with("Map<") => {
            // BTreeMap: decode as length + entries into Map
            let inner = &t[4..t.len()-1];
            let parts: Vec<&str> = inner.splitn(2, ", ").collect();
            if parts.len() == 2 {
                let k_dec = decode_expr_with(parts[0], rd);
                let v_dec = decode_expr_with(parts[1], rd);
                format!("(() => {{ const _m = new Map(); const _len = {}.readLength(); for (let _i = 0; _i < _len; _i++) {{ _m.set({}, {}); }} return _m; }})()",
                    rd, k_dec, v_dec)
            } else {
                format!("new Map() /* TODO: Map decode */")
            }
        }
        t if t.ends_with(" | null") => {
            let inner = &t[..t.len()-7];
            format!("{}.readOption((r) => {})", rd, decode_expr_with(inner, "r"))
        }
        t if t.starts_with("Attested<") => {
            // Attested<T> requires callback: Attested.decode(r, (r2) => T.decode(r2))
            let inner = &t[9..t.len()-1]; // extract T from Attested<T>
            let rd2 = next_reader_var(rd);
            format!("Attested.decode({}, ({}) => {})", rd, rd2, decode_expr_with(inner, &rd2))
        }
        t if is_tuple_type(t) => {
            // Tuple type [A, B] — decode each element in order
            let inner = &t[1..t.len()-1];
            let parts: Vec<&str> = inner.split(", ").collect();
            let decodes: Vec<String> = parts.iter()
                .map(|ty| decode_expr_with(ty.trim(), rd))
                .collect();
            format!("[{}] as {}", decodes.join(", "), ts_type)
        }
        t => {
            let base = t.split('<').next().unwrap_or(t);
            format!("{}.decode({})", base, rd)
        }
    }
}

/// Top-level decode_expr using "reader" as the variable name
fn decode_expr(ts_type: &str) -> String {
    decode_expr_with(ts_type, "reader")
}

/// Inline encode — uses "w" as the writer variable name
fn encode_inline(value: &str, ts_type: &str) -> String {
    match ts_type {
        "string" => format!("w.writeString({})", value),
        "boolean" => format!("w.writeBool({})", value),
        "number" => format!("w.writeU32({})", value),
        "Uint8Array" => format!("w.writeByteVec({})", value),
        _ => format!("{}.encode(w)", value),
    }
}

/// Check if a type is a tuple like [A, B] (not an array like A[])
fn is_tuple_type(t: &str) -> bool {
    t.starts_with('[') && t.ends_with(']') && t.contains(',') && !t.ends_with("[]")
}

fn is_primitive_type(t: &str) -> bool {
    matches!(t, "string" | "boolean" | "number" | "bigint | number")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

/// Generate a unique writer variable name to avoid capture in nested callbacks
fn next_writer_var(current: &str) -> String {
    match current {
        "writer" => "w".to_string(),
        "w" => "w2".to_string(),
        _ => format!("{}x", current),
    }
}

/// Generate a unique reader variable name to avoid capture in nested callbacks
fn next_reader_var(current: &str) -> String {
    match current {
        "reader" => "r".to_string(),
        "r" => "r2".to_string(),
        _ => format!("{}x", current),
    }
}
