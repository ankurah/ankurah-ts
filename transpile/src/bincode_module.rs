//! Bincode rewrite module
//!
//! For structs/enums with #[derive(Serialize, Deserialize)], generates
//! encode(writer: BincodeWriter) and static decode(reader: BincodeReader) methods.
//!
//! The encoding is field-by-field in declaration order (bincode's default).
//! Custom serde impls (like EntityId's raw bytes) are detected and skipped —
//! those types must provide their own encode/decode.

use crate::registry::TypeRegistry;
use crate::ty::{Prim, Ty};
use crate::types::{StructInfo, EnumInfo};

/// Check if a struct/enum has derive(Serialize, Deserialize)
pub fn has_serde_derive(derives: &[String]) -> bool {
    derives.iter().any(|d| d == "Serialize") &&
    derives.iter().any(|d| d == "Deserialize")
}

/// Generate encode/decode methods for a struct with named fields
pub fn generate_struct_codec(reg: &TypeRegistry, info: &StructInfo) -> String {
    let mut out = String::new();

    // encode
    out.push_str("  encode(writer: BincodeWriter): void {\n");
    for field in &info.fields {
        if let Some(name) = &field.name {
            out.push_str(&format!("    {};\n", field_codec(reg, field, &format!("this.{}", name), &info.name).0));
        }
    }
    out.push_str("  }\n\n");

    // decode
    let full_name = format!("{}{}", info.name, info.generics);
    out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
    for field in &info.fields {
        if let Some(name) = &field.name {
            out.push_str(&format!("    const {} = {};\n", name, field_codec(reg, field, "", &info.name).1));
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
pub fn generate_tuple_struct_codec(reg: &TypeRegistry, info: &StructInfo) -> String {
    let mut out = String::new();
    let full_name = format!("{}{}", info.name, info.generics);

    if info.fields.len() == 1 {
        let field = &info.fields[0];
        let field_name = field.name.as_deref().unwrap_or("_0");

        out.push_str("  encode(writer: BincodeWriter): void {\n");
        out.push_str(&format!("    {};\n", field_codec(reg, field, &format!("this.{}", field_name), &info.name).0));
        out.push_str("  }\n\n");

        out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
        out.push_str(&format!("    const {} = {};\n", field_name, field_codec(reg, field, "", &info.name).1));
        out.push_str(&format!("    return new {}({});\n", info.name, field_name));
        out.push_str("  }\n");
    } else {
        out.push_str("  encode(writer: BincodeWriter): void {\n");
        for field in &info.fields {
            let name = field.name.as_deref().unwrap_or("_0");
            out.push_str(&format!("    {};\n", field_codec(reg, field, &format!("this.{}", name), &info.name).0));
        }
        out.push_str("  }\n\n");

        out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
        let mut names = Vec::new();
        for field in &info.fields {
            let name = field.name.as_deref().unwrap_or("_0");
            out.push_str(&format!("    const {} = {};\n", name, field_codec(reg, field, "", &info.name).1));
            names.push(name);
        }
        out.push_str(&format!("    return new {}({});\n", info.name, names.join(", ")));
        out.push_str("  }\n");
    }

    out
}

/// Generate encode/decode methods for an enum
pub fn generate_enum_codec(reg: &TypeRegistry, info: &EnumInfo) -> String {
    let mut out = String::new();

    // Find serde(other) variant if any
    let serde_other_variant = info.variants.iter().find(|v| v.is_serde_other);

    // encode — match on variant, write discriminant + fields
    out.push_str("  encode(writer: BincodeWriter): void {\n");
    out.push_str("    this.match({\n");
    for (i, variant) in info.variants.iter().enumerate() {
        // `#[serde(other)]` says what a *decoder* does with a tag it does not
        // know; it takes nothing away from the variant itself, which keeps its
        // own index and is written like any other. Refusing to encode it made
        // `Item::Other` — a value the Rust fixture round-trips — unwritable.
        out.push_str(&format!("      {}: (v) => {{\n", variant.name));
        out.push_str(&format!("        writer.writeVariant({});\n", i));
        for field in &variant.fields {
            if let Some(name) = &field.name {
                out.push_str(&format!("        {};\n", field_codec(reg, field, &format!("v.{}", name), &info.name).0));
            }
        }
        out.push_str("      },\n");
    }
    out.push_str("    });\n");
    out.push_str("  }\n\n");

    // decode — read discriminant, switch on variant
    let full_name = format!("{}{}", info.name, info.generics);
    out.push_str(&format!("  static decode(reader: BincodeReader): {} {{\n", full_name));
    out.push_str("    const variant = reader.readVariant();\n");
    out.push_str("    switch (variant) {\n");
    for (i, variant) in info.variants.iter().enumerate() {
        // The catch-all is reached through `default` as well, for every tag
        // this decoder does not know; its own index still decodes to it.
        out.push_str(&format!("      case {}: {{\n", i));
        for field in &variant.fields {
            if let Some(name) = &field.name {
                out.push_str(&format!("        const {} = {};\n", name, field_codec(reg, field, "", &info.name).1));
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

/// A `#[serde(with = "..")]` module, as the port writes the two halves it
/// stands for.
///
/// The module is Rust code the engine will not expand, so it is hooked by
/// identity: one entry per module the corpus writes, and a module with no entry
/// is reported rather than emitted as an ordinary field, which would put
/// different bytes on the wire.
///
/// `json_as_bytes` (ankql's `Literal::Json`) serializes the document to bytes
/// and encodes those, so the wire carries a byte vector and the value is a
/// parsed document either side of it.
fn serde_with(module: &str, value: &str, wr: &str) -> Option<(String, String)> {
    match module {
        "json_as_bytes" => Some((
            format!(
                "{}.writeByteVec(new TextEncoder().encode(JSON.stringify({})))",
                wr, value
            ),
            format!(
                "JSON.parse(new TextDecoder().decode({}.readByteVec()))",
                wr
            ),
        )),
        _ => None,
    }
}

/// Report a `#[serde(with = ..)]` module the port has no hook for.
fn unknown_serde_with(module: &str, owner: &str) {
    crate::diag::pending::park_at(
        0,
        0,
        format!(
            "`{}` routes a field through `#[serde(with = \"{}\")]`, and the port has no \
             translation for that module, so the field is encoded as if the attribute were \
             not there — which puts different bytes on the wire",
            owner, module
        ),
    );
}

/// The bincode width and signedness a Rust primitive is written with.
///
/// The TypeScript spelling cannot say it: `i16`, `i32`, `u32` and `usize` are
/// all `number`, and reading a signed value back as unsigned turns
/// `Literal::I32(-1234567890)` into `3060399406`, which the wire-format oracle
/// caught at 26 sites. The Rust type is what decides, so the resolved type
/// travels beside the spelling.
fn width_of(ty: Option<&Ty>) -> Option<&'static str> {
    match ty?.peel_refs() {
        Ty::Prim(p) => Some(match p {
            Prim::U8 => "U8",
            Prim::U16 => "U16",
            // `usize` is 4 bytes on wasm32, which is what the std surface pins.
            Prim::U32 | Prim::Usize => "U32",
            Prim::U64 => "U64",
            Prim::I8 => "I8",
            Prim::I16 => "I16",
            Prim::I32 | Prim::Isize => "I32",
            Prim::I64 => "I64",
            Prim::F32 | Prim::F64 => "F64",
            Prim::Bool | Prim::Char | Prim::U128 | Prim::I128 => return None,
        }),
        _ => None,
    }
}

/// The element type inside a `Vec<T>`, an `Option<T>` or an array, so the width
/// question can be asked of it too.
fn element_of<'t>(ty: Option<&'t Ty>) -> Option<&'t Ty> {
    match ty?.peel_refs() {
        Ty::Named { args, .. } if args.len() == 1 => Some(&args[0]),
        Ty::Array { elem, .. } | Ty::Slice(elem) => Some(elem),
        _ => None,
    }
}

/// The `n`th element of a tuple type.
fn tuple_element<'t>(ty: Option<&'t Ty>, n: usize) -> Option<&'t Ty> {
    match ty?.peel_refs() {
        Ty::Tuple(parts) => parts.get(n),
        _ => None,
    }
}

/// Generate the encode expression for a value of a given TS type
/// `wr` is the writer variable name (e.g., "writer" at top level, "w" inside callbacks)
fn encode_expr_with(value: &str, ts_type: &str, wr: &str, ty: Option<&Ty>) -> String {
    if let Some(width) = width_of(ty) {
        return format!("{}.write{}({})", wr, width, value);
    }
    match ts_type {
        "string" => format!("{}.writeString({})", wr, value),
        "boolean" => format!("{}.writeBool({})", wr, value),
        "number" => format!("{}.writeU32({})", wr, value),
        "bigint" => format!("{}.writeU64({})", wr, value),
        "Uint8Array" => format!("{}.writeByteVec({})", wr, value),
        t if t.ends_with("[]") => {
            let inner = &t[..t.len()-2];
            let elem = element_of(ty);
            match width_of(elem) {
                Some(width) => format!("{}.writeVec({}, (w, item) => w.write{}(item))", wr, value, width),
                None if is_primitive_type(inner) && elem.is_none() => {
                    format!("{}.writeVec({}, (w, item) => w.write{}(item))", wr, value, capitalize(inner))
                }
                None => format!("{}.writeVec({}, (w, item) => {})", wr, value,
                    encode_expr_with("item", inner, "w", elem)),
            }
        }
        t if t.starts_with("Map<") => {
            // BTreeMap: sorted by key in Rust. Encode as length + sorted entries.
            let inner = &t[4..t.len()-1];
            let parts: Vec<&str> = inner.splitn(2, ", ").collect();
            if parts.len() == 2 {
                let k_enc = encode_expr_with("k", parts[0], wr, None);
                let v_enc = encode_expr_with("v", parts[1], wr, None);
                format!("{{ const _entries = [...{}.entries()].sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0); {}.writeLength(_entries.length); for (const [k, v] of _entries) {{ {}; {}; }} }}",
                    value, wr, k_enc, v_enc)
            } else {
                format!("/* TODO: Map encode */ {}.writeLength({}.size)", wr, value)
            }
        }
        t if t.ends_with(" | null") => {
            let inner = &t[..t.len()-7];
            format!("{}.writeOption({}, (w, v) => {})", wr, value,
                encode_expr_with("v", inner, "w", element_of(ty)))
        }
        t if t.starts_with("Attested<") => {
            // Attested<T> requires callback: v.encode(w, (w2: BincodeWriter, p: T) => p.encode(w2))
            let inner = &t[9..t.len()-1];
            let wr2 = next_writer_var(wr);
            format!("{}.encode({}, ({}: BincodeWriter, p: {}) => {})",
                value, wr, wr2, inner,
                encode_expr_with("p", inner, &wr2, element_of(ty)))
        }
        t if is_tuple_type(t) => {
            // Tuple type [A, B] — encode each element
            let inner = &t[1..t.len()-1];
            let parts: Vec<&str> = inner.split(", ").collect();
            let encodes: Vec<String> = parts.iter().enumerate()
                .map(|(i, part)| encode_expr_with(&format!("{}[{}]", value, i), part.trim(), wr, tuple_element(ty, i)))
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
fn encode_expr(value: &str, ts_type: &str, ty: Option<&Ty>) -> String {
    encode_expr_with(value, ts_type, "writer", ty)
}

/// One field's two halves, with its `#[serde(with = ..)]` applied.
fn field_codec(reg: &TypeRegistry, field: &crate::types::FieldInfo, value: &str, owner: &str)
    -> (String, String)
{
    if let Some(module) = &field.serde_with {
        match serde_with(module, value, "writer") {
            Some((enc, _)) => {
                let (_, dec) = serde_with(module, value, "reader").unwrap_or_default();
                return (enc, dec);
            }
            None => unknown_serde_with(module, owner),
        }
    }
    let pair = (
        encode_expr(value, &field.ts_ty(reg), field.ty.as_ref()),
        decode_expr(&field.ts_ty(reg), field.ty.as_ref()),
    );
    report_if_unwritable(reg, field, owner, &pair.1);
    pair
}

/// A field whose codec falls through to `<Type>.decode(reader)` when nothing
/// emits a class called `<Type>`.
///
/// The fallthrough is right for a type this crate declares and wrong for a
/// declared system type — `ulid::Ulid` is `pub struct Ulid(pub u128)` in the
/// surface and has no TypeScript at all, so `Ulid.decode(reader)` is a
/// `ReferenceError` the moment the variant is read. It used to be emitted with
/// nothing said.
fn report_if_unwritable(
    reg: &TypeRegistry,
    field: &crate::types::FieldInfo,
    owner: &str,
    decode: &str,
) {
    let Some(head) = decode.split('.').next() else { return };
    if !decode.ends_with(".decode(reader)") || head.is_empty() {
        return;
    }
    // The question is about the type the NAME stands for, and emission erases
    // the wrappers on the way there: `Box<Expr>` is written `Expr`, and `Expr`
    // is this crate's own.
    let Some(mut ty) = field.ty.clone() else { return };
    loop {
        match crate::name_map::shape::js_shape(reg, &ty) {
            crate::name_map::shape::JsShape::SameAs(inner) => ty = inner,
            _ => break,
        }
    }
    let Some(id) = ty.peel_refs().id() else { return };
    if !id.is_foreign() && !reg.is_system(id) {
        return;
    }
    crate::diag::pending::park_at(
        0,
        0,
        format!(
            "`{}`'s `{}` is encoded by calling `{}.decode`, and nothing in the port emits a \
             class of that name, so reading one raises at run time",
            owner,
            field.name.clone().unwrap_or_default(),
            head
        ),
    );
}

/// Generate the decode expression for a given TS type
/// `rd` is the reader variable name
pub(crate) fn decode_expr_with(ts_type: &str, rd: &str, ty: Option<&Ty>) -> String {
    if let Some(width) = width_of(ty) {
        return format!("{}.read{}()", rd, width);
    }
    match ts_type {
        "string" => format!("{}.readString()", rd),
        "boolean" => format!("{}.readBool()", rd),
        "number" => format!("{}.readU32()", rd),
        "bigint" => format!("{}.readU64()", rd),
        "Uint8Array" => format!("{}.readByteVec()", rd),
        t if t.ends_with("[]") => {
            let inner = &t[..t.len()-2];
            let elem = element_of(ty);
            match width_of(elem) {
                Some(width) => format!("{}.readVec((r) => r.read{}())", rd, width),
                None if is_primitive_type(inner) && elem.is_none() => {
                    format!("{}.readVec((r) => r.read{}())", rd, capitalize(inner))
                }
                None => format!("{}.readVec((r) => {})", rd, decode_expr_with(inner, "r", elem)),
            }
        }
        t if t.starts_with("Map<") => {
            // BTreeMap: decode as length + entries into Map
            let inner = &t[4..t.len()-1];
            let parts: Vec<&str> = inner.splitn(2, ", ").collect();
            if parts.len() == 2 {
                let k_dec = decode_expr_with(parts[0], rd, None);
                let v_dec = decode_expr_with(parts[1], rd, None);
                format!("(() => {{ const _m = new Map(); const _len = {}.readLength(); for (let _i = 0; _i < _len; _i++) {{ _m.set({}, {}); }} return _m; }})()",
                    rd, k_dec, v_dec)
            } else {
                format!("new Map() /* TODO: Map decode */")
            }
        }
        t if t.ends_with(" | null") => {
            let inner = &t[..t.len()-7];
            format!("{}.readOption((r) => {})", rd, decode_expr_with(inner, "r", element_of(ty)))
        }
        t if t.starts_with("Attested<") => {
            // Attested<T> requires callback: Attested.decode(r, (r2: BincodeReader) => T.decode(r2))
            let inner = &t[9..t.len()-1];
            let rd2 = next_reader_var(rd);
            format!("Attested.decode({}, ({}: BincodeReader) => {})", rd, rd2,
                decode_expr_with(inner, &rd2, element_of(ty)))
        }
        t if is_tuple_type(t) => {
            // Tuple type [A, B] — decode each element in order
            let inner = &t[1..t.len()-1];
            let parts: Vec<&str> = inner.split(", ").collect();
            let decodes: Vec<String> = parts.iter().enumerate()
                .map(|(i, part)| decode_expr_with(part.trim(), rd, tuple_element(ty, i)))
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
fn decode_expr(ts_type: &str, ty: Option<&Ty>) -> String {
    decode_expr_with(ts_type, "reader", ty)
}

/// Check if a type is a tuple like [A, B] (not an array like A[])
fn is_tuple_type(t: &str) -> bool {
    t.starts_with('[') && t.ends_with(']') && t.contains(',') && !t.ends_with("[]")
}

fn is_primitive_type(t: &str) -> bool {
    matches!(t, "string" | "boolean" | "number" | "bigint")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    fn enum_codec(src: &str, name: &str) -> String {
        let f = Fixture::build(&[("lib.rs", src)]);
        let info = f.files[0]
            .file
            .enums
            .iter()
            .find(|e| e.name == name)
            .expect("enum");
        generate_enum_codec(&f.reg, info)
    }

    /// `#[serde(other)]` says what a decoder does with a tag it does not know.
    /// It takes nothing away from the variant, which keeps its own index and is
    /// written like any other — the Rust fixture round-trips `Item::Other`.
    #[test]
    fn a_serde_other_variant_encodes_under_its_own_index() {
        let ts = enum_codec(
            "use serde::{Serialize, Deserialize};\n\
             #[derive(Serialize, Deserialize)] pub enum Item {\n\
               SysRoot,\n\
               Collection { name: String },\n\
               #[serde(other)] Other,\n\
             }",
            "Item",
        );
        assert!(!ts.contains("decode-only"), "{}", ts);
        assert!(ts.contains("writer.writeVariant(2);"), "{}", ts);
    }

    /// It is still the catch-all a decoder falls back to for an unknown tag.
    #[test]
    fn a_serde_other_variant_is_still_the_default_case() {
        let ts = enum_codec(
            "use serde::{Serialize, Deserialize};\n\
             #[derive(Serialize, Deserialize)] pub enum Item {\n\
               SysRoot,\n\
               #[serde(other)] Other,\n\
             }",
            "Item",
        );
        assert!(ts.contains("default: return new Item('Other', {});"), "{}", ts);
        assert!(ts.contains("case 1: {"), "{}", ts);
    }
}
