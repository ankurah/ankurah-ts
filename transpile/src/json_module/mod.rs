//! The JSON half of `#[derive(Serialize, Deserialize)]`.
//!
//! serde has two formats for one derive: a binary one (bincode, in
//! `bincode_module`) and a human-readable one. `toJSON()` is what the
//! human-readable `Serialize` writes, and `static fromJson(value)` is
//! `Deserialize` for the same format, answering the `Result` Rust answers.
//!
//! serde's shapes, which this writes exactly:
//!   * a struct with named fields → an object keyed by the RUST field names
//!   * a newtype struct, and any container `#[serde(transparent)]` → the inner
//!     value, with no wrapper
//!   * a tuple struct → an array
//!   * an enum, externally tagged (serde's default): a unit variant is the
//!     variant's name as a string; anything else is a one-key object whose key
//!     is the variant's name. `#[serde(other)]` names the variant an unknown
//!     tag reads as.
//!
//! Both halves are built from ONE resolved description (`schema`), and the read
//! CHECKS what it was given (`shape`) rather than casting it.
//!
//! **Ownership (R4).** A decoder owns what it has built until it returns. The
//! reader below therefore never throws: each field's read is a `Result`, an
//! `Err` is handed straight out — the inner `JsonError` passes through, never
//! re-created — and every owned field already built is released first. A
//! `catch` is still written around the whole thing, because a `toJSON` on a
//! moved value raises, and it rethrows an `OwnershipFatal` untouched: swallowing
//! one would disarm the leak registry inside every emitted reader.

mod schema;
mod shape;

#[cfg(test)]
mod tests;

use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, StructInfo};
use schema::{Body, Member};

/// The error an emitted `fromJson` answers with.
pub const ERROR_TYPE: &str = "JsonError";

/// Does a value of this TypeScript type have a JSON spelling in the port?
///
/// Read by the registry, which narrows `reads_json` to a fixed point: a type
/// whose own half is refused has no `fromJson`, and neither does anything that
/// holds one. Without that the refusal was not transitive — ten call sites in
/// the corpus named a static no class declares, and the four wire types that
/// held one raised on their first call.
pub fn json_shape_refusal(
    reg: &TypeRegistry,
    ts_ty: &str,
    ty: Option<&crate::ty::Ty>,
) -> Option<String> {
    shape::of_type(reg, ts_ty, ty).err()
}

/// The JSON half for a struct, or the reason there is none.
pub fn struct_json(reg: &TypeRegistry, info: &StructInfo) -> Result<String, String> {
    let plan = schema::of_struct(reg, info)?;
    let mut out = String::new();
    out.push_str("  toJSON(): unknown {\n");
    out.push_str(&write_body(&plan.body, "this."));
    out.push_str("  }\n\n");
    let mut read = read_body(&plan.body, &info.name, "value");
    let built = match &plan.body {
        Body::Unit => format!("return Result.Ok(new {}());", info.name),
        Body::Transparent(member) => {
            format!("return Result.Ok(new {}({}));", info.name, member.ts_name)
        }
        Body::Named(members) | Body::Positional(members) => format!(
            "return Result.Ok(new {}({}));",
            info.name,
            members
                .iter()
                .map(|m| m.ts_name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    read.statements.push_str(&built);
    read.statements.push('\n');
    out.push_str(&reader(
        reg,
        &info.name,
        &info.generics,
        &info.type_params,
        &read,
    ));
    Ok(out)
}

/// The JSON half for an enum, externally tagged the way serde writes it.
pub fn enum_json(reg: &TypeRegistry, info: &EnumInfo) -> Result<String, String> {
    let plan = schema::of_enum(reg, info)?;
    let mut out = String::new();

    // Every arm is annotated, because the runtime's `match` infers one type
    // from the arms and a unit variant's `'Name'` would make that `string` —
    // which the object an arm with a payload returns is not.
    out.push_str("  toJSON(): unknown {\n    return this.match<unknown>({\n");
    for variant in &plan.variants {
        match &variant.body {
            Body::Unit => out.push_str(&format!(
                "      {}: () => {},\n",
                variant.name,
                key(&variant.key)
            )),
            body => {
                let payload = write_value(body, "v.");
                out.push_str(&format!(
                    "      {}: (v) => ({{ {}: {} }}),\n",
                    variant.name,
                    key(&variant.key),
                    payload
                ));
            }
        }
    }
    out.push_str("    });\n  }\n\n");

    let mut body = String::new();
    let units: Vec<&schema::Variant> = plan
        .variants
        .iter()
        .filter(|v| matches!(v.body, Body::Unit))
        .collect();
    // `#[serde(other)]` — the variant serde yields for a tag it does not know.
    // The bincode half has always read it; this one ended every reader with an
    // `Err`, so `Item.fromJson('Nope')` answered `Err` where Rust answers
    // `Item::Other`.
    let other = plan.variants.iter().find(|v| v.is_other);
    if !units.is_empty() {
        body.push_str("if (typeof value === 'string') {\n  switch (value) {\n");
        for variant in &units {
            body.push_str(&format!(
                "    case {}: return Result.Ok(new {}('{}', {{}}));\n",
                key(&variant.key),
                info.name,
                variant.name
            ));
        }
        body.push_str("  }\n");
        if let Some(other) = other {
            body.push_str(&format!(
                "  return Result.Ok(new {}('{}', {{}}));\n",
                info.name, other.name
            ));
        }
        body.push_str("}\n");
    }
    body.push_str(
        "if (value === null || typeof value !== 'object' || Array.isArray(value)) {\n",
    );
    body.push_str(&format!(
        "  return Result.Err({}.custom('expected a variant of `{}`'));\n}}\n",
        ERROR_TYPE, info.name
    ));
    body.push_str("const o = value as Record<string, unknown>;\n");
    for variant in plan.variants.iter().filter(|v| !matches!(v.body, Body::Unit)) {
        body.push_str(&format!("if ({} in o) {{\n", key(&variant.key)));
        let read = read_body(&variant.body, &info.name, &format!("o[{}]", key(&variant.key)));
        let built = match &variant.body {
            Body::Unit => String::new(),
            Body::Transparent(member) => format!(
                "return Result.Ok(new {}('{}', {{ {}: {} }}));",
                info.name, variant.name, member.ts_name, member.ts_name
            ),
            Body::Named(members) | Body::Positional(members) => format!(
                "return Result.Ok(new {}('{}', {{ {} }}));",
                info.name,
                variant.name,
                members
                    .iter()
                    .map(|m| format!("{}: {}", m.ts_name, m.ts_name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        for line in format!("{}\n{}", read.statements, built).lines() {
            body.push_str(&format!("  {}\n", line));
        }
        body.push_str("}\n");
    }
    match other {
        Some(other) if matches!(other.body, Body::Unit) => body.push_str(&format!(
            "return Result.Ok(new {}('{}', {{}}));\n",
            info.name, other.name
        )),
        _ => body.push_str(&format!(
            "return Result.Err({}.custom('no variant of `{}` matches this JSON'));\n",
            ERROR_TYPE, info.name
        )),
    }

    out.push_str(&reader(
        reg,
        &info.name,
        &info.generics,
        &info.type_params,
        &Read {
            statements: body,
            owned: Vec::new(),
        },
    ));
    Ok(out)
}

/// The statements a body's read is, and which of the names they bind own
/// something the reader has to release if a later one fails.
struct Read {
    statements: String,
    owned: Vec<String>,
}

/// `toJSON`'s body for one container shape.
fn write_body(body: &Body, receiver: &str) -> String {
    match body {
        Body::Unit => "    return null;\n".to_string(),
        other => format!("    return {};\n", write_value(other, receiver)),
    }
}

/// The VALUE one container shape writes, with `receiver` in front of each
/// member's name (`this.` for a struct, `v.` for an enum payload).
fn write_value(body: &Body, receiver: &str) -> String {
    match body {
        Body::Unit => "null".to_string(),
        Body::Transparent(member) => member.shape.write(&format!("{}{}", receiver, member.ts_name)),
        Body::Named(members) => {
            let parts: Vec<String> = members
                .iter()
                .map(|m| {
                    format!(
                        "{}: {}",
                        key(&m.key),
                        m.shape.write(&format!("{}{}", receiver, m.ts_name))
                    )
                })
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        Body::Positional(members) => {
            let parts: Vec<String> = members
                .iter()
                .map(|m| m.shape.write(&format!("{}{}", receiver, m.ts_name)))
                .collect();
            format!("[{}]", parts.join(", "))
        }
    }
}

/// The statements that read one container shape out of `source`, binding each
/// member to its own name.
fn read_body(body: &Body, owner: &str, source: &str) -> Read {
    let mut out = Read {
        statements: String::new(),
        owned: Vec::new(),
    };
    match body {
        Body::Unit => {}
        Body::Transparent(member) => read_member(&mut out, member, source),
        Body::Named(members) => {
            out.statements.push_str(&format!(
                "if ({s} === null || typeof {s} !== 'object' || Array.isArray({s})) {{\n  \
                 return Result.Err({e}.custom('expected an object for `{o}`'));\n}}\n",
                s = source,
                e = ERROR_TYPE,
                o = owner
            ));
            out.statements.push_str(&format!(
                "const _o = {} as Record<string, unknown>;\n",
                source
            ));
            for member in members {
                // serde reads a missing key as `None` for an `Option` and
                // refuses it for anything else. `{}` used to answer `Ok` with
                // every field `undefined`.
                if !member.ts_ty.ends_with(" | null") {
                    out.statements.push_str(&format!(
                        "if (!({k} in _o)) {{\n  return {f}({e}.custom('missing field `{n}`'));\n}}\n",
                        k = key(&member.key),
                        f = fail(&out.owned),
                        e = ERROR_TYPE,
                        n = member.key
                    ));
                }
                read_member(&mut out, member, &format!("_o[{}]", key(&member.key)));
            }
        }
        Body::Positional(members) => {
            out.statements.push_str(&format!(
                "if (!Array.isArray({s}) || {s}.length !== {n}) {{\n  \
                 return Result.Err({e}.custom('expected an array of {n} for `{o}`'));\n}}\n",
                s = source,
                n = members.len(),
                e = ERROR_TYPE,
                o = owner
            ));
            out.statements
                .push_str(&format!("const _a = {} as unknown[];\n", source));
            for (i, member) in members.iter().enumerate() {
                read_member(&mut out, member, &format!("_a[{}]", i));
            }
        }
    }
    out
}

/// One member: read it, hand its `Err` out — releasing what is already built —
/// and bind the value.
fn read_member(out: &mut Read, member: &Member, source: &str) {
    let temp = format!("_r{}", member.ts_name);
    out.statements.push_str(&format!(
        "const {t} = ((v: unknown) => {r})({s});\n\
         if ({t}.isErr()) return {f}({t}.unwrapErr());\n\
         const {n} = {t}.unwrap();\n",
        t = temp,
        r = member.shape.read(),
        s = source,
        f = fail(&out.owned),
        n = member.ts_name
    ));
    if member.shape.owns {
        out.owned.push(member.ts_name.clone());
    }
}

/// What a failing read returns: the inner error, passed straight out, with
/// everything already built released first.
///
/// The old reader threw the inner `JsonError` and let the outer `catch` build a
/// NEW one from its rendered text — so the original was abandoned (the leak
/// registry saw it) and its position was lost. R4: the inner `Err` propagates.
fn fail(owned: &[String]) -> String {
    if owned.is_empty() {
        return "Result.Err".to_string();
    }
    format!(
        "((e: {}) => {{ dropOwned([{}]); return Result.Err(e); }})",
        ERROR_TYPE,
        owned.join(", ")
    )
}

/// The `static fromJson` around a body.
///
/// A static cannot name the class's own type parameters, so it declares them
/// itself: `static fromJson<T>(value: unknown): Result<Ref<T>, JsonError>`.
/// Writing `Ref<T>` into a static that declares no `T` is TS2302.
fn reader(
    _reg: &TypeRegistry,
    name: &str,
    generics: &str,
    type_params: &[String],
    body: &Read,
) -> String {
    let declared = if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    };
    let full = format!("{}{}", name, generics);
    let mut out = format!(
        "  static fromJson{}(value: unknown): Result<{}, {}> {{\n    try {{\n",
        declared, full, ERROR_TYPE
    );
    for line in body.statements.lines() {
        out.push_str(&format!("      {}\n", line));
    }
    // The reader never throws for a decode failure, and two other things still
    // can. `toJSON` on a moved value raises, and so does the ownership runtime:
    // an `OwnershipFatal` is rethrown, because a `catch` that swallows one
    // disarms the leak registry inside every emitted reader. And an R12 HOLE
    // throws an `UnsupportedShape`, which says the port has no lowering for a
    // Rust shape: a `catch` that turns it into a decode error answers `Err` for
    // a gap in the ENGINE, which is the loud-into-silent trade R12 exists to
    // refuse (`port/ownership.md`).
    out.push_str(&format!(
        "    }} catch (e) {{\n      if (e instanceof OwnershipFatal || e instanceof \
         UnsupportedShape) throw e;\n      return Result.Err({}.fromException(e));\n    \
         }}\n  }}\n",
        ERROR_TYPE
    ));
    out
}

/// A JSON key, quoted the way the emitter quotes every other string.
fn key(name: &str) -> String {
    crate::body::quoted(name)
}
