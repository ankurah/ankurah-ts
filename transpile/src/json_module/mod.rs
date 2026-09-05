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
    let mut taken = Vec::new();
    declared_by(&plan.body, &mut taken);
    let input = free_name("value", &taken);
    let mut read = read_body(&plan.body, &info.name, &input, &free_name("_o", &taken));
    let value = match &plan.body {
        Body::Unit => format!("new {}()", info.name),
        Body::Transparent(member) => format!("new {}({})", info.name, member.ts_name),
        Body::Named(members) | Body::Positional(members) => format!(
            "new {}({})",
            info.name,
            members
                .iter()
                .map(|m| m.ts_name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let built = handed_on(&value, read.owns);
    read.statements.push_str(&built);
    read.statements.push('\n');
    out.push_str(&reader(
        reg,
        &info.name,
        &info.generics,
        &info.type_params,
        &read,
        &input,
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

    // Every name any variant's reader declares, so the parameter, the record it
    // casts to and the per-variant records take names none of them holds.
    let mut taken = Vec::new();
    for variant in &plan.variants {
        declared_by(&variant.body, &mut taken);
    }
    let input = free_name("value", &taken);
    let outer = free_name("o", &taken);
    let record = free_name("_o", &taken);

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
        body.push_str(&format!(
        "if (typeof {i} === 'string') {{\n  switch ({i}) {{\n",
        i = input
    ));
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
    body.push_str(&format!(
        "if ({i} === null || typeof {i} !== 'object' || Array.isArray({i})) {{\n",
        i = input
    ));
    body.push_str(&format!(
        "  return Result.Err({}.custom('expected a variant of `{}`'));\n}}\n",
        ERROR_TYPE, info.name
    ));
    body.push_str(&format!(
        "const {} = {} as Record<string, unknown>;\n",
        outer, input
    ));
    let mut owns_anything = false;
    for variant in plan.variants.iter().filter(|v| !matches!(v.body, Body::Unit)) {
        body.push_str(&format!("if ({} in {}) {{\n", key(&variant.key), outer));
        let read = read_body(
            &variant.body,
            &info.name,
            &format!("{}[{}]", outer, key(&variant.key)),
            &record,
        );
        owns_anything |= read.owns;
        let value = match &variant.body {
            Body::Unit => String::new(),
            Body::Transparent(member) => format!(
                "new {}('{}', {{ {}: {} }})",
                info.name, variant.name, member.ts_name, member.ts_name
            ),
            Body::Named(members) | Body::Positional(members) => format!(
                "new {}('{}', {{ {} }})",
                info.name,
                variant.name,
                members
                    .iter()
                    .map(|m| format!("{}: {}", m.ts_name, m.ts_name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        let built = handed_on(&value, read.owns);
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
            owns: owns_anything,
        },
        &input,
    ));
    Ok(out)
}

/// The statements a body's read is, and whether any of the names they bind owns
/// something the reader has to release when it does not return one.
struct Read {
    statements: String,
    owned: Vec<String>,
    /// Did anything the body read own something? The prologue and the `finally`
    /// are written only where it did.
    owns: bool,
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
fn read_body(body: &Body, owner: &str, source: &str, record: &str) -> Read {
    let mut out = Read {
        statements: String::new(),
        owned: Vec::new(),
        owns: false,
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
                "const {} = {} as Record<string, unknown>;\n",
                record, source
            ));
            for member in members {
                // serde reads a missing key as `None` for an `Option` and
                // refuses it for anything else. `{}` used to answer `Ok` with
                // every field `undefined`.
                if !member.ts_ty.ends_with(" | null") {
                    out.statements.push_str(&format!(
                        "if (!({k} in {r})) {{\n  return Result.Err({e}.custom('missing field `{n}`'));\n}}\n",
                        k = key(&member.key),
                        r = record,
                        e = ERROR_TYPE,
                        n = member.key
                    ));
                }
                read_member(&mut out, member, &format!("{}[{}]", record, key(&member.key)));
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
         if ({t}.isErr()) return Result.Err({t}.unwrapErr());\n\
         const {n} = {t}.unwrap();\n",
        t = temp,
        r = member.shape.read(),
        s = source,
        n = member.ts_name
    ));
    // R4: a decoder owns what it has built until it RETURNS one. The values
    // decoded so far go into a bag the `finally` releases unless the reader
    // handed one back — which is the only form that covers an EXCEPTION as well
    // as an expected `Err`. A per-return closure covered the second and not the
    // first, so a throwing property getter on a late field left every earlier
    // field with nobody.
    if member.shape.owns {
        out.statements.push_str(&format!("{}.push({});\n", BAG, member.ts_name));
        out.owned.push(member.ts_name.clone());
        out.owns = true;
    }
}

/// The bag of values a reader has built, and the flag that says it handed them
/// on. `$`-prefixed because no Rust field name can carry one.
const BAG: &str = "$built";
const KEPT: &str = "$kept";

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
    input: &str,
) -> String {
    let declared = if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    };
    let full = format!("{}{}", name, generics);
    let mut out = format!(
        "  static fromJson{}({}: unknown): Result<{}, {}> {{\n",
        declared, input, full, ERROR_TYPE
    );
    // R4: a decoder owns what it has built until it RETURNS one, and the only
    // form that covers an EXCEPTION as well as an expected `Err` is a `finally`
    // over a bag it fills as it goes.
    if body.owns {
        out.push_str(&format!("    const {}: unknown[] = [];\n", BAG));
        out.push_str(&format!("    let {} = false;\n", KEPT));
    }
    out.push_str("    try {\n");
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
         }}",
        ERROR_TYPE
    ));
    if body.owns {
        out.push_str(&format!(
            " finally {{\n      if (!{}) dropOwned({});\n    }}",
            KEPT, BAG
        ));
    }
    out.push_str("\n  }\n");
    out
}


/// The names the reader's own body declares, so the parameter and the record
/// it casts to can be given names none of them takes.
///
/// A field called `value` used to declare `const value = _rvalue.unwrap();` in
/// the same block as the parameter `value`, so every read of the parameter
/// above it was `Cannot access 'value' before initialization` — the whole
/// reader answered `Err` for every document.
fn declared_by(body: &Body, out: &mut Vec<String>) {
    match body {
        Body::Unit => {}
        Body::Transparent(member) => out.push(member.ts_name.clone()),
        Body::Named(members) | Body::Positional(members) => {
            out.extend(members.iter().map(|m| m.ts_name.clone()))
        }
    }
}

/// A name for the reader's own use that none of `taken` holds.
///
/// `$`-prefixed when the plain one is taken: no Rust field name can carry a
/// `$`, so one attempt is always enough.
fn free_name(plain: &str, taken: &[String]) -> String {
    if taken.iter().any(|name| name == plain) {
        format!("${}", plain)
    } else {
        plain.to_string()
    }
}


/// The successful return, with the bag marked handed on where there is one.
///
/// The flag is set AFTER the value is built and before it is returned, so a
/// constructor that raised would still leave the fields to the `finally` — as
/// Rust's unwind would.
fn handed_on(value: &str, owns: bool) -> String {
    if !owns {
        return format!("return Result.Ok({});", value);
    }
    format!(
        "const $out = {};\n{} = true;\nreturn Result.Ok($out);",
        value, KEPT
    )
}

/// A JSON key, quoted the way the emitter quotes every other string.
fn key(name: &str) -> String {
    crate::body::quoted(name)
}
