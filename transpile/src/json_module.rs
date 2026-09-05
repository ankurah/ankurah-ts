//! The JSON half of `#[derive(Serialize, Deserialize)]`.
//!
//! serde has two formats for one derive: a binary one (bincode, in
//! `bincode_module`) and a human-readable one. `toJSON()` is what the
//! human-readable `Serialize` writes — `JSON.stringify` calls it by name, so
//! `serde_json::to_string(&x)` needs nothing but `JSON.stringify(x)` — and
//! `static fromJson(value)` is `Deserialize` for the same format, answering the
//! `Result` Rust answers.
//!
//! serde's shapes, which this writes exactly:
//!   * a struct with named fields → an object keyed by the RUST field names
//!   * a newtype struct → the inner value, with no wrapper
//!   * a tuple struct → an array
//!   * an enum, externally tagged (serde's default): a unit variant is the
//!     variant's name as a string; anything else is a one-key object whose key
//!     is the variant's name.
//!
//! A field whose type has no JSON spelling here is reported, and the type gets
//! no JSON half at all rather than half a one.

use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, StructInfo};

/// The error an emitted `fromJson` answers with.
pub const ERROR_TYPE: &str = "JsonError";

/// What a value of this TypeScript type looks like as JSON, and how to read it
/// back. `None` where the port has no answer, with the reason.
enum Shape {
    /// The value is already JSON and needs no conversion either way.
    Plain,
    /// `write` renders the value; `read` builds it from `v`.
    Convert { write: String, read: String },
}

/// Does this type get a JSON half, and what does it owe?
///
/// `value` is the expression holding it on the way out; on the way in the
/// expression is always `v`, because the reader binds each piece to `v` before
/// converting it.
fn shape_of(field: &crate::types::FieldInfo, ts_type: &str, value: &str) -> Result<Shape, String> {
    // `#[serde(with = "..")]` changes BOTH formats, so the JSON half honours it
    // the way the bincode half does.
    match field.serde_with.as_deref() {
        None => shape(ts_type, value),
        Some("json_as_bytes") => Ok(Shape::Convert {
            write: format!(
                "Array.from(new TextEncoder().encode(JSON.stringify({})))",
                value
            ),
            read: "JSON.parse(new TextDecoder().decode(new Uint8Array(v as number[])))"
                .to_string(),
        }),
        Some(module) => Err(format!(
            "a field routed through `#[serde(with = \"{}\")]` has no JSON translation in the \
             port, so the type it belongs to gets no `toJSON`/`fromJson` pair",
            module
        )),
    }
}

fn shape(ts_type: &str, value: &str) -> Result<Shape, String> {
    match ts_type {
        "string" | "boolean" | "number" => Ok(Shape::Plain),
        // `serde_json::Value` is a parsed JSON document, which is what the
        // format carries; there is nothing to convert either way.
        "unknown" => Ok(Shape::Plain),
        // serde_json writes a u64/i64 as a JSON number. JavaScript's `bigint`
        // has no JSON form at all — `JSON.stringify` throws on one — so it is
        // written as a number, which is what serde produces and what every
        // reader of this format expects.
        "bigint" => Ok(Shape::Convert {
            write: format!("Number({})", value),
            read: "BigInt(v as number)".to_string(),
        }),
        // `Vec<u8>` is a `Uint8Array` here and an array of numbers in serde.
        "Uint8Array" => Ok(Shape::Convert {
            write: format!("Array.from({})", value),
            read: "new Uint8Array(v as number[])".to_string(),
        }),
        t if t.ends_with(" | null") => {
            let inner = &t[..t.len() - 7];
            let s = shape(inner, "x")?;
            let write = match write_of(&s, "x") {
                // The element goes out as it stands, so the option does too.
                w if w == "x" => value.to_string(),
                w => format!("({v} == null ? null : ((x) => {w})({v}))", v = value, w = w),
            };
            let read = format!("(v == null ? null : ((v) => {})(v))", read_of(&s, inner));
            if write == value && read_is_cast(&s) {
                return Ok(Shape::Plain);
            }
            Ok(Shape::Convert { write, read })
        }
        t if t.ends_with("[]") => {
            let inner = &t[..t.len() - 2];
            let s = shape(inner, "x")?;
            let write = match write_of(&s, "x") {
                w if w == "x" => value.to_string(),
                w => format!("{}.map((x) => {})", value, w),
            };
            let read = format!("(v as unknown[]).map((v) => {})", read_of(&s, inner));
            if write == value && read_is_cast(&s) {
                return Ok(Shape::Plain);
            }
            Ok(Shape::Convert { write, read })
        }
        // A JSON object's keys are strings; a Rust map's are anything. serde
        // writes a `HashMap<K, V>` as an object only where `K` serializes as a
        // string, and the port cannot tell from the TypeScript spelling which
        // of those it has — so it says so rather than writing a codec that is
        // right for some key types and silently wrong for the rest.
        t if t.starts_with("HashMap<") || t.starts_with("HashSet<") => Err(format!(
            "`{}` has no JSON spelling here: a JSON object's keys are strings and serde \
             writes a map that way only when the key does",
            t
        )),
        // A class of the port. `JSON.stringify` finds its `toJSON`, so the
        // value goes out as it stands; reading it back is its own `fromJson`,
        // whose error is this one and so passes straight through.
        t if starts_upper(t) => {
            let class = t.split('<').next().unwrap_or(t);
            Ok(Shape::Convert {
                write: value.to_string(),
                read: format!("_take({}.fromJson(v))", class),
            })
        }
        other => Err(format!(
            "`{}` has no JSON spelling in the port, so the type it is a field of gets no \
             `toJSON`/`fromJson` pair",
            other
        )),
    }
}

/// Is reading this shape back nothing but a cast? Then a list or an option of
/// it is a cast too, and neither needs a `map`.
fn read_is_cast(shape: &Shape) -> bool {
    matches!(shape, Shape::Plain)
}

fn starts_upper(t: &str) -> bool {
    t.chars().next().is_some_and(|c| c.is_uppercase())
}

fn write_of(shape: &Shape, value: &str) -> String {
    match shape {
        Shape::Plain => value.to_string(),
        Shape::Convert { write, .. } => write.clone(),
    }
}

fn read_of(shape: &Shape, ts_type: &str) -> String {
    match shape {
        Shape::Plain => format!("v as {}", ts_type),
        Shape::Convert { read, .. } => read.clone(),
    }
}

/// Does reading this shape back call another type's `fromJson`? Only then does
/// the reader need the helper that unwraps it, and declaring one nothing calls
/// is a name TypeScript reports as unused.
fn reads_nested(shape: &Shape) -> bool {
    matches!(shape, Shape::Convert { read, .. } if read.contains("_take("))
}

/// The reader's helper: unwrap a nested `fromJson`, or leave with its error.
/// Written once per emitted `fromJson` that needs it.
const TAKE: &str = "      const _take = <T,>(r: Result<T, JsonError>): T => { \
if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };\n";

/// The JSON half for a struct, or the reason there is none.
pub fn struct_json(reg: &TypeRegistry, info: &StructInfo) -> Result<String, String> {
    let full_name = format!("{}{}", info.name, info.generics);
    let named = info.fields.iter().all(|f| f.rust_name.is_some());
    let mut out = String::new();
    let mut nested = false;

    // ── Serialize ──
    out.push_str("  toJSON(): unknown {\n");
    if info.fields.is_empty() {
        out.push_str("    return null;\n");
    } else if named {
        out.push_str("    return {\n");
        for field in &info.fields {
            let (ts_name, rust_name) = names(field)?;
            let s = shape_of(field, &field.ts_ty(reg), &format!("this.{}", ts_name))?;
            nested |= reads_nested(&s);
            out.push_str(&format!(
                "      {}: {},\n",
                json_key(&rust_name),
                write_of(&s, &format!("this.{}", ts_name))
            ));
        }
        out.push_str("    };\n");
    } else if info.fields.len() == 1 {
        // A newtype struct is transparent to serde: the inner value IS the JSON.
        let field = &info.fields[0];
        let ts_name = field.name.clone().unwrap_or_else(|| "_0".to_string());
        let s = shape_of(field, &field.ts_ty(reg), &format!("this.{}", ts_name))?;
        out.push_str(&format!("    return {};\n", write_of(&s, &format!("this.{}", ts_name))));
    } else {
        out.push_str("    return [\n");
        for field in &info.fields {
            let ts_name = field.name.clone().unwrap_or_default();
            let s = shape_of(field, &field.ts_ty(reg), &format!("this.{}", ts_name))?;
            out.push_str(&format!("      {},\n", write_of(&s, &format!("this.{}", ts_name))));
        }
        out.push_str("    ];\n");
    }
    out.push_str("  }\n\n");

    // ── Deserialize ──
    out.push_str(&format!(
        "  static fromJson(value: unknown): Result<{}, {}> {{\n",
        full_name, ERROR_TYPE
    ));
    out.push_str("    try {\n");
    let mut body = String::new();
    if info.fields.is_empty() {
        body.push_str(&format!("return Result.Ok(new {}());\n", info.name));
    } else if named {
        body.push_str("const o = value as Record<string, unknown>;\n");
        let mut args = Vec::new();
        for field in &info.fields {
            let (ts_name, rust_name) = names(field)?;
            let ts_ty = field.ts_ty(reg);
            let s = shape_of(field, &ts_ty, "x")?;
            nested |= reads_nested(&s);
            body.push_str(&format!(
                "const {} = ((v: unknown) => {})(o[{}]);\n",
                ts_name,
                read_of(&s, &ts_ty),
                json_key(&rust_name)
            ));
            args.push(ts_name);
        }
        body.push_str(&format!(
            "return Result.Ok(new {}({}));\n",
            info.name,
            args.join(", ")
        ));
    } else if info.fields.len() == 1 {
        let field = &info.fields[0];
        let ts_ty = field.ts_ty(reg);
        let s = shape_of(field, &ts_ty, "x")?;
        nested |= reads_nested(&s);
        body.push_str(&format!(
            "return Result.Ok(new {}(((v: unknown) => {})(value)));\n",
            info.name,
            read_of(&s, &ts_ty)
        ));
    } else {
        body.push_str("const a = value as unknown[];\n");
        let mut args = Vec::new();
        for (i, field) in info.fields.iter().enumerate() {
            let ts_ty = field.ts_ty(reg);
            let s = shape_of(field, &ts_ty, "x")?;
            nested |= reads_nested(&s);
            body.push_str(&format!(
                "const _{} = ((v: unknown) => {})(a[{}]);\n",
                i,
                read_of(&s, &ts_ty),
                i
            ));
            args.push(format!("_{}", i));
        }
        body.push_str(&format!(
            "return Result.Ok(new {}({}));\n",
            info.name,
            args.join(", ")
        ));
    }
    if nested {
        out.push_str(TAKE);
    }
    for line in body.lines() {
        out.push_str(&format!("      {}\n", line));
    }
    out.push_str(&format!(
        "    }} catch (e) {{\n      return Result.Err({}.fromException(e));\n    }}\n  }}\n",
        ERROR_TYPE
    ));
    Ok(out)
}

/// The JSON half for an enum, externally tagged the way serde writes it.
pub fn enum_json(reg: &TypeRegistry, info: &EnumInfo) -> Result<String, String> {
    let full_name = format!("{}{}", info.name, info.generics);
    let mut out = String::new();
    let mut nested = false;

    // Every arm is annotated, because the runtime's `match` infers one type
    // from the arms and a unit variant's `'Name'` would make that `string` —
    // which the object an arm with a payload returns is not.
    out.push_str("  toJSON(): unknown {\n    return this.match<unknown>({\n");
    for variant in &info.variants {
        if variant.fields.is_empty() {
            out.push_str(&format!(
                "      {}: () => {},\n",
                variant.name,
                json_key(&variant.name)
            ));
            continue;
        }
        let named = variant.fields.iter().all(|f| f.rust_name.is_some());
        let mut parts = Vec::new();
        for (i, field) in variant.fields.iter().enumerate() {
            let ts_name = field.name.clone().unwrap_or_else(|| format!("_{}", i));
            let s = shape_of(field, &field.ts_ty(reg), &format!("v.{}", ts_name))?;
            nested |= reads_nested(&s);
            let written = write_of(&s, &format!("v.{}", ts_name));
            if named {
                let rust_name = field.rust_name.clone().unwrap_or(ts_name);
                parts.push(format!("{}: {}", json_key(&rust_name), written));
            } else {
                parts.push(written);
            }
        }
        let payload = if named {
            format!("{{ {} }}", parts.join(", "))
        } else if parts.len() == 1 {
            parts.remove(0)
        } else {
            format!("[{}]", parts.join(", "))
        };
        out.push_str(&format!(
            "      {}: (v) => ({{ {}: {} }}),\n",
            variant.name,
            json_key(&variant.name),
            payload
        ));
    }
    out.push_str("    });\n  }\n\n");

    out.push_str(&format!(
        "  static fromJson(value: unknown): Result<{}, {}> {{\n    try {{\n",
        full_name, ERROR_TYPE
    ));
    let mut body = String::new();
    body.push_str("if (typeof value === 'string') {\n");
    body.push_str("  switch (value) {\n");
    let mut has_unit = false;
    for variant in info.variants.iter().filter(|v| v.fields.is_empty()) {
        has_unit = true;
        body.push_str(&format!(
            "    case {}: return Result.Ok(new {}('{}', {{}}));\n",
            json_key(&variant.name),
            info.name,
            variant.name
        ));
    }
    body.push_str("  }\n}\n");
    if !has_unit {
        body.clear();
    }
    body.push_str("const o = value as Record<string, unknown>;\n");
    for variant in info.variants.iter().filter(|v| !v.fields.is_empty()) {
        let key = json_key(&variant.name);
        body.push_str(&format!("if ({} in o) {{\n", key));
        let named = variant.fields.iter().all(|f| f.rust_name.is_some());
        let single = variant.fields.len() == 1 && !named;
        body.push_str(&format!("  const p = o[{}];\n", key));
        let mut parts = Vec::new();
        for (i, field) in variant.fields.iter().enumerate() {
            let ts_name = field.name.clone().unwrap_or_else(|| format!("_{}", i));
            let ts_ty = field.ts_ty(reg);
            let s = shape_of(field, &ts_ty, "x")?;
            nested |= reads_nested(&s);
            let source = if single {
                "p".to_string()
            } else if named {
                let rust_name = field.rust_name.clone().unwrap_or_else(|| ts_name.clone());
                format!("(p as Record<string, unknown>)[{}]", json_key(&rust_name))
            } else {
                format!("(p as unknown[])[{}]", i)
            };
            parts.push(format!(
                "{}: ((v: unknown) => {})({})",
                ts_name,
                read_of(&s, &ts_ty),
                source
            ));
        }
        body.push_str(&format!(
            "  return Result.Ok(new {}('{}', {{ {} }}));\n}}\n",
            info.name,
            variant.name,
            parts.join(", ")
        ));
    }
    body.push_str(&format!(
        "return Result.Err({}.custom('no variant of `{}` matches this JSON'));\n",
        ERROR_TYPE, info.name
    ));
    if nested {
        out.push_str(TAKE);
    }
    for line in body.lines() {
        out.push_str(&format!("      {}\n", line));
    }
    out.push_str(&format!(
        "    }} catch (e) {{\n      return Result.Err({}.fromException(e));\n    }}\n  }}\n",
        ERROR_TYPE
    ));
    Ok(out)
}

/// A field's emitted property name and the key serde writes for it.
fn names(field: &crate::types::FieldInfo) -> Result<(String, String), String> {
    let ts_name = field
        .name
        .clone()
        .ok_or_else(|| "a named field with no name".to_string())?;
    let rust_name = field.rust_name.clone().unwrap_or_else(|| ts_name.clone());
    Ok((ts_name, rust_name))
}

/// A JSON key, quoted the way the emitter quotes every other string.
fn json_key(name: &str) -> String {
    crate::body::quoted(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bigint_goes_out_as_a_number() {
        // serde_json writes a u64 as a JSON number; `JSON.stringify` throws on
        // a bigint, so the conversion is not optional.
        let Ok(Shape::Convert { write, read }) = shape("bigint", "this.n") else {
            panic!("bigint has a JSON shape");
        };
        assert_eq!(write, "Number(this.n)");
        assert_eq!(read, "BigInt(v as number)");
    }

    #[test]
    fn a_class_reads_through_its_own_from_json() {
        let Ok(Shape::Convert { write, read }) = shape("EntityId", "this.id") else {
            panic!("a class has a JSON shape");
        };
        assert_eq!(write, "this.id");
        assert!(read.contains("EntityId.fromJson(v)"), "{read}");
    }

    #[test]
    fn a_plain_field_is_left_alone() {
        assert!(matches!(shape("string", "this.s"), Ok(Shape::Plain)));
        assert!(matches!(shape("string | null", "this.s"), Ok(Shape::Plain)));
        assert!(matches!(shape("string[]", "this.s"), Ok(Shape::Plain)));
    }

    #[test]
    fn a_type_with_no_json_spelling_is_refused() {
        // A `Map` key can be anything in Rust and only a string in JSON; the
        // port says so rather than writing half a codec.
        assert!(shape("HashMap<EntityId, string>", "this.m").is_err());
    }

    /// `serde_json::from_str::<T>(text)` is written as `T.fromJson(..)`, and a
    /// `T` that derives no `Deserialize` has no such static — the call turned a
    /// parse error into a `TypeError` with nothing said.
    #[test]
    fn from_str_is_written_only_where_the_static_exists() {
        let source = "use serde::{Serialize, Deserialize};\n\
                      #[derive(Serialize, Deserialize)]\n\
                      pub struct Config { pub n: u32 }\n\
                      pub struct Plain { pub n: u32 }\n";
        let mut f = crate::testing::Fixture::build(&[(
            "lib.rs",
            &format!("{}pub fn read(t: &str) -> Result<Config, serde_json::Error> {{ serde_json::from_str::<Config>(t) }}", source),
        )]);
        assert!(f.translated_method("lib.rs", "read").contains("Config.fromJson(JSON.parse(t))"));

        let mut f = crate::testing::Fixture::build(&[(
            "lib.rs",
            &format!("{}pub fn read(t: &str) -> Result<Plain, serde_json::Error> {{ serde_json::from_str::<Plain>(t) }}", source),
        )]);
        let ts = f.translated_method("lib.rs", "read");
        assert!(!ts.contains("fromJson"), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("reads itself out of the parsed value")),
            "{:?}",
            f.messages()
        );
    }
}
