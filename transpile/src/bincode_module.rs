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
/// Is this the one pair the wire width and the port's spelling disagree about?
///
/// `usize` and `isize` are eight bytes on the bincode wire and a `number` here.
/// Nothing else is: the widths the port spells `bigint` are already `bigint`,
/// and the narrow ones are `number` on both sides.
fn widens_at_the_wire(ty: Option<&Ty>) -> bool {
    matches!(
        ty.map(Ty::peel_refs),
        Some(Ty::Prim(Prim::Usize)) | Some(Ty::Prim(Prim::Isize))
    )
}

/// A `number` about to be written as eight bytes.
fn widened(value: &str, ty: Option<&Ty>) -> String {
    if widens_at_the_wire(ty) {
        format!("BigInt({})", value)
    } else {
        value.to_string()
    }
}

/// A `bigint` just read, about to land in a `number`.
fn narrowed(read: &str, ty: Option<&Ty>) -> String {
    if widens_at_the_wire(ty) {
        format!("Number({})", read)
    } else {
        read.to_string()
    }
}

fn width_of(ty: Option<&Ty>) -> Option<&'static str> {
    match ty?.peel_refs() {
        Ty::Prim(p) => Some(match p {
            Prim::U8 => "U8",
            Prim::U16 => "U16",
            Prim::U32 => "U32",
            Prim::U64 => "U64",
            Prim::I8 => "I8",
            Prim::I16 => "I16",
            Prim::I32 => "I32",
            Prim::I64 => "I64",
            Prim::F64 => "F64",
            // The WIRE width, which is not the memory width. serde's
            // `Serialize for usize` calls `serialize_u64`, so bincode writes
            // eight bytes whatever the target's pointer is — the comment here
            // used to say "`usize` is 4 bytes on wasm32, which is what the std
            // surface pins", and that is the in-memory width, which is the
            // mistake the signed-width fix was written to correct.
            Prim::Usize => "U64",
            Prim::Isize => "I64",
            // These four the port's writer has no method for. The correct call
            // is written and the site says which method the runtime owes: an
            // `f32` is four bytes and was written as eight, a `char` is raw
            // UTF-8 with no length prefix and was written as a length-prefixed
            // string, and a `u128`/`i128` is sixteen bytes and was written as
            // eight, unsigned either way.
            Prim::F32 => "F32",
            Prim::Char => "Char",
            Prim::U128 => "U128",
            Prim::I128 => "I128",
            Prim::Bool => return None,
        }),
        _ => None,
    }
}

/// The widths the port's `BincodeWriter`/`BincodeReader` has no method for.
///
/// Zero sites in the corpus today — no `#[derive(Serialize)]` type in the ten
/// crates has a field of one — so this is a gate rather than a gap: the first
/// such field says what the runtime owes instead of writing the wrong number of
/// bytes in silence.
const NO_RUNTIME_METHOD: [&str; 4] = ["F32", "Char", "U128", "I128"];

/// A width whose method the codec does not have, said once at the site that
/// wants it. The call IS written — `writer.writeF32(x)` names the pair the
/// runtime owes, where `writer.writeF64(x)` wrote eight bytes and said nothing.
fn report_missing_width(width: &str) {
    if !NO_RUNTIME_METHOD.contains(&width) {
        return;
    }
    crate::diag::pending::park_at(
        0,
        0,
        format!(
            "the wire format writes this value as a `{}`, and the port's codec has no \
             `write{}`/`read{}`; the call is written so the gap is loud rather than a wrong \
             number of bytes, and the runtime owes the pair",
            width, width, width
        ),
    );
}

/// Sort a map's entries the way Rust's `BTreeMap<String, _>` iterates: by the
/// key's UTF-8 BYTES, which is code-point order.
///
/// JavaScript's `<` on two strings compares UTF-16 code units, and the two
/// disagree above the BMP: `"\u{FFFD}"` sorts after `"🚀"` in UTF-16 and before
/// it in UTF-8. bincode writes a map in iteration order, so the two orders are
/// two different byte strings on the wire.
const BY_UTF8_KEY: &str = "(a, b) => { const x = [...a[0]], y = [...b[0]]; const n = Math.min(x.length, y.length); for (let i = 0; i < n; i++) { const d = (x[i].codePointAt(0) ?? 0) - (y[i].codePointAt(0) ?? 0); if (d !== 0) return d < 0 ? -1 : 1; } return x.length === y.length ? 0 : (x.length < y.length ? -1 : 1); }";

/// The element type inside a `Vec<T>`, an `Option<T>` or an array, so the width
/// question can be asked of it too.
fn element_of<'t>(ty: Option<&'t Ty>) -> Option<&'t Ty> {
    match ty?.peel_refs() {
        Ty::Named { args, .. } if args.len() == 1 => Some(&args[0]),
        Ty::Array { elem, .. } | Ty::Slice(elem) => Some(elem),
        _ => None,
    }
}

/// The `n`th type argument of a container.
fn argument_of<'t>(ty: Option<&'t Ty>, n: usize) -> Option<&'t Ty> {
    match ty?.peel_refs() {
        Ty::Named { args, .. } => args.get(n),
        _ => None,
    }
}

/// `K, V` of a `HashMap<K, V>` as written, split on the comma that is not
/// inside brackets. `splitn(2, ", ")` read `[string, number]` as two arguments,
/// so a tuple key or a nested map took the wrong halves.
fn type_arguments(inner: &str) -> Option<(String, String)> {
    let mut depth = 0i32;
    for (i, ch) in inner.char_indices() {
        match ch {
            '<' | '[' | '(' | '{' => depth += 1,
            '>' | ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                return Some((
                    inner[..i].trim().to_string(),
                    inner[i + 1..].trim().to_string(),
                ));
            }
            _ => {}
        }
    }
    None
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
        report_missing_width(width);
        // D9: `usize` and `isize` occupy EIGHT bytes on the wire and are
        // spelled `number` here, and `setBigUint64` throws on a number. The
        // conversion belongs at this boundary and nowhere else — R13 keeps the
        // arithmetic 32-bit.
        return format!("{}.write{}({})", wr, width, widened(value, ty));
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
        t if t.starts_with("HashMap<") => {
            // BTreeMap: sorted by key in Rust. Encode as length + sorted entries.
            let inner = &t[8..t.len() - 1];
            match type_arguments(inner) {
                Some((key, value_ty)) => {
                    // The resolved map's own arguments, NOT the halves of a
                    // string split: the widths live in the `Ty`, and passing
                    // `None` here made a `BTreeMap<String, i16>` write its
                    // values with `writeU32` — four bytes, unsigned, where Rust
                    // writes two, signed.
                    let k_enc = encode_expr_with("k", &key, wr, argument_of(ty, 0));
                    let v_enc = encode_expr_with("v", &value_ty, wr, argument_of(ty, 1));
                    format!(
                        "{{ const _entries = [...{}.entries()].sort({}); {}.writeLength(_entries.length); for (const [k, v] of _entries) {{ {}; {}; }} }}",
                        value, BY_UTF8_KEY, wr, k_enc, v_enc
                    )
                }
                None => {
                    crate::diag::pending::park_at(
                        0,
                        0,
                        format!(
                            "`{}` is a map whose key and value types the emitter could not \
                             read apart, so only its LENGTH is written and every entry is lost",
                            t
                        ),
                    );
                    format!("{}.writeLength({}.size)", wr, value)
                }
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

/// A field whose codec calls `<Type>.decode` where nothing emits a class called
/// `<Type>`.
///
/// The fallthrough is right for a type this crate declares and wrong for a
/// declared system type — `ulid::Ulid` is `pub struct Ulid(pub u128)` in the
/// surface and has no TypeScript at all, so `Ulid.decode(reader)` is a
/// `ReferenceError` the moment the field is read. It used to be emitted with
/// nothing said.
///
/// Every `<Type>.decode(` in the field's codec is asked, not only the whole of
/// it: `Vec<Ulid>` decodes as `reader.readVec((r) => Ulid.decode(r))`, and a
/// check that matched the top-level shape alone said nothing about it. The same
/// `ReferenceError`, one wrapper further in.
fn report_if_unwritable(
    reg: &TypeRegistry,
    field: &crate::types::FieldInfo,
    owner: &str,
    decode: &str,
) {
    let Some(ty) = field.ty.clone() else { return };
    let mut asked: Vec<String> = Vec::new();
    // The whole field's own codec, asked as it always was: the emitted head is
    // what a reader will call, whatever the port's spelling of the type is.
    if let Some(head) = decode.strip_suffix(".decode(reader)") {
        if head.chars().next().is_some_and(|c| c.is_uppercase())
            && !crate::codegen::BASE_RUNTIME_SYMBOLS.contains(&head)
        {
            let mut peeled = ty.peel_refs().clone();
            while let crate::name_map::shape::JsShape::SameAs(inner) =
                crate::name_map::shape::js_shape(reg, &peeled)
            {
                peeled = inner.peel_refs().clone();
            }
            if peeled.id().is_some_and(|id| id.is_foreign() || reg.is_system(id)) {
                asked.push(head.to_string());
                report_missing_decoder(owner, field, head);
            }
        }
    }
    for inner in named_types_within(reg, &ty) {
        let written = crate::name_map::map_ty(reg, &inner);
        // Only the types whose codec IS a call to a class of their own name.
        // A `String` decodes through the reader and names no class.
        if !decode_expr(&written, Some(&inner)).contains(&format!("{}.decode(", written)) {
            continue;
        }
        let Some(id) = inner.peel_refs().id() else { continue };
        if !id.is_foreign() && !reg.is_system(id) {
            continue;
        }
        if asked.contains(&written) {
            continue;
        }
        asked.push(written.clone());
        report_missing_decoder(owner, field, &written);
    }
}

/// Every named type inside this one, the wrappers peeled: `Vec<Option<Ulid>>`
/// answers `Ulid` as well as itself.
///
/// The question — does the port emit a class of that name — used to be asked of
/// the field's whole type alone, so `Vec<Ulid>`, which decodes as
/// `reader.readVec((r) => Ulid.decode(r))`, said nothing. The same
/// `ReferenceError`, one wrapper further in.
fn named_types_within(reg: &TypeRegistry, ty: &Ty) -> Vec<Ty> {
    let mut out = Vec::new();
    let mut queue = vec![ty.clone()];
    let mut seen = 0usize;
    while let Some(next) = queue.pop() {
        seen += 1;
        // A type that refers to itself would walk forever; the corpus's deepest
        // is three, and this is a diagnostic, not a proof.
        if seen > 64 {
            break;
        }
        // A wrapper the port ERASES — `Box<Expr>` is written `Expr` — is not
        // the type the emitted name stands for. Following it to the end first
        // keeps the name and the declaration the same type, which is the whole
        // question here: `Box<Expr>` reported `Expr.decode` as missing because
        // it asked `Box`'s declaration about `Expr`'s name.
        let mut peeled = next.peel_refs().clone();
        while let crate::name_map::shape::JsShape::SameAs(inner) =
            crate::name_map::shape::js_shape(reg, &peeled)
        {
            peeled = inner.peel_refs().clone();
        }
        match &peeled {
            Ty::Named { args, .. } => {
                out.push(peeled.clone());
                queue.extend(args.iter().cloned());
            }
            Ty::Slice(elem) | Ty::Array { elem, .. } => queue.push((**elem).clone()),
            Ty::Tuple(elems) => queue.extend(elems.iter().cloned()),
            Ty::Ref { inner, .. } => queue.push((**inner).clone()),
            _ => {}
        }
    }
    out
}

fn report_missing_decoder(owner: &str, field: &crate::types::FieldInfo, head: &str) {
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
        report_missing_width(width);
        // The other half of D9: an eight-byte read answers a `bigint`, and the
        // field it lands in is a `number`.
        return narrowed(&format!("{}.read{}()", rd, width), ty);
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
        t if t.starts_with("HashMap<") => {
            // BTreeMap: decode as length + entries into the runtime's keyed map.
            let inner = &t[8..t.len() - 1];
            match type_arguments(inner) {
                Some((key, value)) => {
                    let k_dec = decode_expr_with(&key, rd, argument_of(ty, 0));
                    let v_dec = decode_expr_with(&value, rd, argument_of(ty, 1));
                    format!(
                        "(() => {{ const _m = new HashMap<{}, {}>(); const _len = {}.readLength(); for (let _i = 0; _i < _len; _i++) {{ _m.set({}, {}); }} return _m; }})()",
                        key, value, rd, k_dec, v_dec
                    )
                }
                None => {
                    crate::diag::pending::park_at(
                        0,
                        0,
                        format!(
                            "`{}` is a map whose key and value types the emitter could not \
                             read apart, so an empty map is read where the bytes hold entries",
                            t
                        ),
                    );
                    "new HashMap()".to_string()
                }
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

    /// A `<Type>.decode` call with nothing behind it is a `ReferenceError` the
    /// moment a value is read, and it is one wherever the call sits. The
    /// question used to be asked of the field's whole codec, so `Vec<Ulid>` —
    /// which decodes as `reader.readVec((r) => Ulid.decode(r))` — said nothing:
    /// the same fault, one wrapper further in.
    #[test]
    fn a_decoder_that_names_nothing_is_reported_inside_a_wrapper_too() {
        let f = Fixture::build(&[(
            "lib.rs",
            "use serde::{Serialize, Deserialize};\n\
             use ulid::Ulid;\n\
             #[derive(Serialize, Deserialize)] pub struct Bare { pub id: Ulid }\n\
             #[derive(Serialize, Deserialize)] pub struct Many { pub ids: Vec<Ulid> }\n\
             #[derive(Serialize, Deserialize)] pub struct Ours { pub one: Bare }\n",
        )]);
        for info in &f.files[0].file.structs {
            let _ = generate_struct_codec(&f.reg, info);
        }
        crate::diag::pending::drain(&f.sink);
        let said = f.messages();
        assert!(
            said.iter().any(|m| m.contains("`Bare`'s `id`") && m.contains("`Ulid.decode`")),
            "the plain field: {:?}",
            said
        );
        assert!(
            said.iter().any(|m| m.contains("`Many`'s `ids`") && m.contains("`Ulid.decode`")),
            "the same type inside a `Vec`: {:?}",
            said
        );
        assert!(
            !said.iter().any(|m| m.contains("`Ours`") && m.contains(".decode`")),
            "a type this crate emits is not reported: {:?}",
            said
        );
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

#[cfg(test)]
mod width_tests {
    use crate::testing::Fixture;

    const DERIVE: &str = "#[derive(Serialize, Deserialize)]\n";

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", src)])
    }

    /// The WIRE width, not the memory width. serde's `Serialize for usize`
    /// calls `serialize_u64`, so bincode writes eight bytes whatever the
    /// target's pointer is — and the comment that used to sit on this table
    /// conflated the two, which is the mistake the signed-width fix was written
    /// to correct.
    #[test]
    fn usize_is_eight_bytes_on_the_wire() {
        let mut f = built(&format!(
            "{}pub struct Row {{ pub n: usize, pub m: isize }}",
            DERIVE
        ));
        let ts = f.emitted("lib.rs");
        // D9: eight bytes on the wire, a `number` in the port — and
        // `setBigUint64` throws on a number, so the conversion belongs at this
        // boundary and nowhere else.
        assert!(ts.contains("writer.writeU64(BigInt(this.n))"), "{}", ts);
        assert!(ts.contains("writer.writeI64(BigInt(this.m))"), "{}", ts);
        assert!(ts.contains("Number(reader.readU64())"), "{}", ts);
        assert!(ts.contains("Number(reader.readI64())"), "{}", ts);
    }

    /// A width the port already spells `bigint` crosses no boundary, so nothing
    /// is wrapped around it.
    #[test]
    fn a_sixty_four_bit_field_is_written_as_it_stands() {
        let mut f = built(&format!("{}pub struct Row {{ pub n: u64 }}", DERIVE));
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("writer.writeU64(this.n)"), "{}", ts);
        assert!(!ts.contains("BigInt(this.n)"), "{}", ts);
        assert!(ts.contains("reader.readU64()"), "{}", ts);
        assert!(!ts.contains("Number(reader.readU64())"), "{}", ts);
    }

    /// A width the codec has no method for writes the CORRECT call and says
    /// which pair the runtime owes. `f32` was written as eight bytes, a `char`
    /// as a length-prefixed string, and a `u128`/`i128` as eight bytes unsigned.
    #[test]
    fn a_width_the_codec_has_no_method_for_is_reported() {
        let mut f = built(&format!("{}pub struct Row {{ pub x: f32 }}", DERIVE));
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("writer.writeF32(this.x)"), "{}", ts);
        assert!(!ts.contains("writeF64(this.x)"), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("has no `writeF32`/`readF32`")),
            "{:?}",
            f.messages()
        );
    }

    /// A map's key and value widths come from the resolved map's own arguments.
    /// Passing `None` made a `BTreeMap<String, i16>` write its values with
    /// `writeU32` — four bytes, unsigned, where Rust writes two, signed.
    #[test]
    fn a_maps_value_keeps_its_width_and_its_sign() {
        let mut f = built(&format!(
            "use std::collections::BTreeMap;\n{}pub struct Row {{ pub m: BTreeMap<String, i16> }}",
            DERIVE
        ));
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("w.writeI16(v)") || ts.contains("writeI16(v)"), "{}", ts);
        assert!(!ts.contains("writeU32(v)"), "{}", ts);
        assert!(ts.contains("readI16()"), "{}", ts);
    }

    /// bincode writes a map in ITERATION order, and Rust's `BTreeMap<String, _>`
    /// iterates by the key's UTF-8 bytes. JavaScript's `<` compares UTF-16 code
    /// units, and the two disagree above the BMP.
    #[test]
    fn a_maps_keys_are_ordered_by_their_utf8_bytes() {
        let mut f = built(&format!(
            "use std::collections::BTreeMap;\n{}pub struct Row {{ pub m: BTreeMap<String, u8> }}",
            DERIVE
        ));
        let ts = f.emitted("lib.rs");
        assert!(ts.contains("codePointAt(0)"), "{}", ts);
        assert!(!ts.contains("a[0] < b[0] ? -1"), "{}", ts);
    }
}
