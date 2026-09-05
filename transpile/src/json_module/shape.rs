//! What one value looks like as JSON, and how to read it back CHECKING it.
//!
//! For: `Deserialize` is a validator. serde refuses `{}` for a struct with a
//! required field, refuses `"x"` where a number belongs, and refuses a tuple of
//! the wrong length. The port used to write `v as T` — a cast, which checks
//! nothing at run time — so `PathExpr.fromJson({})` answered `Ok` with
//! `steps === undefined`, and every reader downstream saw a value serde would
//! have refused.
//!
//! A shape therefore carries two things: how to WRITE the value, and how to
//! READ it back as a `Result`. The read is an expression of type
//! `Result<T, JsonError>` over a bound `v`, so a failure is a value the caller
//! owns rather than an exception — which is what lets the reader release the
//! fields it has already built.

use crate::registry::TypeRegistry;
use crate::types::FieldInfo;

/// What a value of one type is, as JSON.
pub(super) struct Shape {
    /// `{}` with the value substituted for `V`: what goes out.
    write: String,
    /// An expression of type `Result<T, JsonError>` over a bound `v`: what
    /// comes back, checked.
    read: String,
    /// Does reading this build a value the reader has to RELEASE if a later
    /// field fails? A number does not; a class, or a list of them, does.
    pub owns: bool,
}

impl Shape {
    /// The value, written for the format.
    pub(super) fn write(&self, value: &str) -> String {
        self.write.replace("$V", value)
    }

    /// A `Result<T, JsonError>` over `v`.
    pub(super) fn read(&self) -> &str {
        &self.read
    }
}


/// Is `v` an array of BYTES?
///
/// One predicate, because two places read one: the ordinary `Vec<u8>` field and
/// a field routed through `#[serde(with = "json_as_bytes")]`. A `Uint8Array`
/// truncates whatever it is handed, so `[-1, 256, 1.5]` became `[255, 0, 1]`
/// and the document was accepted; serde reads each element as a `u8`.
const BYTE_ARRAY: &str = "Array.isArray(v) && v.every((b) => typeof b === 'number' \
                          && Number.isInteger(b) && b >= 0 && b <= 255)";

/// The shape of one field, `#[serde(with = "..")]` included.
pub(super) fn of_field(
    reg: &TypeRegistry,
    field: &FieldInfo,
    ts_ty: &str,
) -> Result<Shape, String> {
    match field.serde_with.as_deref() {
        None => of_type(reg, ts_ty, field.ty.as_ref()),
        // `json_as_bytes` writes the value's JSON text as an array of bytes.
        Some("json_as_bytes") => Ok(Shape {
            write: "Array.from(new TextEncoder().encode(serde_json.stringify($V).unwrap()))"
                .to_string(),
            // The SAME byte test the ordinary `Vec<u8>` reader makes: serde
            // reads each element as a `u8` before anything decodes them, and a
            // `Uint8Array` truncates whatever it is handed — `[305]` became
            // byte 49 and this answered `Ok(1)` where serde answers `Err`.
            read: format!(
                "({bytes} \
                 ? serde_json.parse(new TextDecoder().decode(new Uint8Array(v as number[]))) \
                 : Result.Err(JsonError.custom('expected an array of bytes')))",
                bytes = BYTE_ARRAY
            ),
            owns: false,
        }),
        Some(module) => Err(format!(
            "a field routed through `#[serde(with = \"{}\")]` has no JSON translation in the \
             port, so the type it belongs to gets no `toJSON`/`fromJson` pair",
            module
        )),
    }
}

/// The shape of a type: its TypeScript spelling, and the resolved type behind
/// it where the engine named one.
///
/// The spelling alone is what the reader used to have, and it is a leaf: two
/// crates both declaring a `State` are one spelling and two types, so asking
/// "does `State` have a `fromJson`" of a name found the wrong one. The resolved
/// type is the identity, and it is threaded through the containers the way the
/// bincode half threads it.
pub(super) fn of_type(
    reg: &TypeRegistry,
    ts_ty: &str,
    ty: Option<&crate::ty::Ty>,
) -> Result<Shape, String> {
    // A `Box<Expr>` IS an `Expr` in the port — the wrapper is erased and the
    // spelling is already the inner one — so the identity behind the spelling
    // is the inner type's, not `Box`'s. Reading `Box`'s made every recursive
    // type refuse itself.
    let peeled = ty.and_then(|ty| match crate::name_map::shape::js_shape(reg, ty) {
        crate::name_map::shape::JsShape::SameAs(inner) => Some(inner),
        _ => None,
    });
    let ty = match &peeled {
        Some(inner) if crate::name_map::map_ty(reg, inner) == ts_ty => Some(inner),
        _ => ty,
    };
    let inner_ty = || element_of(ty);
    match ts_ty {
        // A `char` is a `string` here and exactly ONE code point in Rust:
        // serde reads `"ab"` and `""` as errors, and the port read any string
        // at all. `[...v].length` counts code points, not UTF-16 units, so an
        // astral character is one.
        "string" if is_char(ty) => Ok(checked(
            "typeof v === 'string' && [...v].length === 1",
            "a char",
            "string",
        )),
        "string" => Ok(checked("typeof v === 'string'", "a string", "string")),
        "boolean" => Ok(checked("typeof v === 'boolean'", "a boolean", "boolean")),
        "number" => Ok(match integer_prim(ty) {
            // serde reads an integer field by its Rust type: `1.5`, `-1` and
            // `256` are each a `u8` error, and each is `typeof v === 'number'`.
            Some(prim) => {
                let (low, high) = prim.range().expect("an integer width has a range");
                checked(
                    &format!(
                        "typeof v === 'number' && Number.isInteger(v) && v >= {low} && v <= {high}"
                    ),
                    &article(prim),
                    "number",
                )
            }
            None => checked("typeof v === 'number'", "a number", "number"),
        }),
        // `PhantomData` carries nothing. serde writes it as a unit, and the
        // emitted class has no field for it at all, so nothing here reads or
        // writes one — but a type that HAS one still has a JSON half, which is
        // what `Ref<T>`'s `#[serde(skip)] _phantom` needs.
        t if is_zero_sized(t) => Ok(Shape {
            write: "null".to_string(),
            read: "Result.Ok(undefined)".to_string(),
            owns: false,
        }),
        // `serde_json::Value` is a parsed JSON document; anything is one.
        "unknown" => Ok(Shape {
            write: "$V".to_string(),
            read: "Result.Ok(v)".to_string(),
            owns: false,
        }),
        // R3: serde_json keeps an integer token exactly, so the port's reader
        // hands a wide one back as a `bigint` and a small one as a `number`.
        // Both are the same Rust integer, and both read here; the value goes
        // out as the `bigint` it is, which `serde_json.stringify` writes as a
        // bare integer token.
        "bigint" => {
            // A `number` this wide has already lost digits: `2 ** 53 + 1` reads
            // back as `2 ** 53`, and `BigInt()` of it invents the difference.
            // `Number.isSafeInteger` is what refuses that instead of rounding.
            let (test, expected) = match integer_prim(ty) {
                Some(prim) => {
                    let (low, high) = prim.range().expect("an integer width has a range");
                    (
                        format!(
                            "(typeof v === 'bigint' && v >= {low}n && v <= {high}n) \
                             || (typeof v === 'number' && Number.isSafeInteger(v) \
                             && v >= {low_clamped} && v <= {high_clamped})",
                            low = low,
                            high = high,
                            low_clamped = low.max(-9_007_199_254_740_991),
                            high_clamped = high.min(9_007_199_254_740_991),
                        ),
                        article(prim),
                    )
                }
                None => (
                    "typeof v === 'bigint' \
                     || (typeof v === 'number' && Number.isSafeInteger(v))"
                        .to_string(),
                    "an integer".to_string(),
                ),
            };
            Ok(Shape {
                write: "$V".to_string(),
                read: format!(
                    "({test} ? Result.Ok(BigInt(v as bigint | number)) \
                     : Result.Err(JsonError.custom('expected {expected}')))"
                ),
                owns: false,
            })
        }
        // `Vec<u8>` is a `Uint8Array` here and an array of numbers in serde.
        "Uint8Array" => Ok(Shape {
            write: "Array.from($V)".to_string(),
            // A `Uint8Array` truncates whatever it is handed, so `[-1, 256, 1.5]`
            // became `[255, 0, 1]` and the document was accepted. serde reads
            // each element as a `u8`.
            read: format!(
                "({bytes} ? Result.Ok(new Uint8Array(v as number[])) \
                 : Result.Err(JsonError.custom('expected an array of bytes')))",
                bytes = BYTE_ARRAY
            ),
            owns: false,
        }),
        t if t.ends_with(" | null") => {
            // serde reads a missing key and a `null` alike as `None`, which is
            // exactly what the port writes for it.
            let inner = of_type(reg, &t[..t.len() - 7], inner_ty())?;
            // A value the format carries as it stands carries as it stands
            // inside an option too: `(x == null ? null : x)` is `x`.
            let written = if inner.write("$V") == "$V" {
                "$V".to_string()
            } else {
                format!("($V == null ? null : {})", inner.write("$V"))
            };
            Ok(Shape {
                write: written,
                read: format!(
                    "(v == null ? Result.Ok(null) : ((v: unknown) => {})(v))",
                    inner.read()
                ),
                owns: inner.owns,
            })
        }
        t if t.ends_with("[]") => {
            let inner = of_type(reg, &t[..t.len() - 2], inner_ty())?;
            let written = if inner.write("x") == "x" {
                "$V".to_string()
            } else {
                format!("$V.map((x) => {})", inner.write("x"))
            };
            Ok(Shape {
                write: written,
                read: format!(
                    "(Array.isArray(v) ? jsonAll(v.map((v) => {})) \
                     : Result.Err(JsonError.custom('expected an array')))",
                    inner.read()
                ),
                owns: inner.owns,
            })
        }
        // serde writes a map as a JSON object when its key serializes as a
        // string, and as an array of pairs otherwise. A `String` key is the
        // first; anything else is refused, because the port cannot tell from
        // the spelling which serde form the key takes.
        t if t.starts_with("HashMap<") => {
            let inner = &t[8..t.len() - 1];
            let Some((key, value)) = split_arguments(inner) else {
                return Err(format!("`{}` is a map the port cannot read the arguments of", t));
            };
            if key != "string" {
                return Err(format!(
                    "`{}` is a map whose key is not a string, and serde writes such a map as an \
                     array of pairs rather than as an object",
                    t
                ));
            }
            let member = of_type(reg, &value, argument_of(ty, 1))?;
            Ok(Shape {
                write: format!(
                    "Object.fromEntries([...$V.entries()].map(([k, x]) => [k, {}]))",
                    member.write("x")
                ),
                read: format!(
                    "(v !== null && typeof v === 'object' && !Array.isArray(v) \
                     ? jsonMap(jsonAll(Object.entries(v as Record<string, unknown>).map(([k, v]) => \
                       jsonMap(((v: unknown) => {read})(v), (x) => [k, x] as [string, {value}]))), \
                       (entries) => new HashMap<string, {value}>(entries)) \
                     : Result.Err(JsonError.custom('expected an object')))",
                    read = member.read(),
                    value = value
                ),
                // The CONTAINER is what this read builds, and the runtime
                // `HashMap` is tracked whatever it holds: a map of strings owes
                // a `drop()` exactly as a map of entities does. Taking `owns`
                // from the member left a partly decoded map unreleased on the
                // error path of every later field.
                owns: true,
            })
        }
        t if t.starts_with("HashSet<") => Err(format!(
            "`{}` is a set, and serde writes one as an array whose order the port cannot \
             reproduce",
            t
        )),
        // A tuple. serde writes one as an array of its parts, and the length is
        // part of the format.
        t if t.starts_with('[') && t.ends_with(']') => {
            let Some(parts) = tuple_parts(&t[1..t.len() - 1]) else {
                return Err(format!("`{}` is a tuple the port cannot read the parts of", t));
            };
            let mut writes = Vec::new();
            let mut reads = Vec::new();
            let mut owns = false;
            for (i, part) in parts.iter().enumerate() {
                let shape = of_type(reg, part, argument_of(ty, i))?;
                owns |= shape.owns;
                writes.push(shape.write(&format!("$V[{}]", i)));
                reads.push(format!("((v: unknown) => {})(a[{}])", shape.read(), i));
            }
            Ok(Shape {
                write: format!("[{}]", writes.join(", ")),
                read: format!(
                    "(Array.isArray(v) && v.length === {n} \
                     ? ((a: unknown[]) => jsonAll([{reads}]))(v) \
                     : Result.Err(JsonError.custom('expected an array of {n}')))",
                    n = parts.len(),
                    reads = reads.join(", ")
                ),
                owns,
            })
        }
        // A class of the port. `toJSON` is what writes it, and its own
        // `fromJson` is what reads it back — but only where the port really
        // emits one. Deciding that from the CAPITAL LETTER put ten calls in the
        // corpus to a static no class declares, and the four wire types that
        // contained one raised on their first call.
        t if starts_upper(t) => {
            let class = t.split('<').next().unwrap_or(t);
            if !reads_json(reg, class, ty) {
                return Err(format!(
                    "`{}` has no `fromJson` in the port, so a type with a field of it gets no \
                     `toJSON`/`fromJson` pair either",
                    class
                ));
            }
            Ok(Shape {
                write: "$V.toJSON()".to_string(),
                read: format!("{}.fromJson(v)", class),
                owns: true,
            })
        }
        other => Err(format!(
            "`{}` has no JSON spelling in the port, so the type it is a field of gets no \
             `toJSON`/`fromJson` pair",
            other
        )),
    }
}

/// A value the format carries as it stands, with the check serde makes.
fn checked(test: &str, expected: &str, ts_ty: &str) -> Shape {
    Shape {
        write: "$V".to_string(),
        read: format!(
            "({} ? Result.Ok(v as {}) : Result.Err(JsonError.custom('expected {}')))",
            test, ts_ty, expected
        ),
        owns: false,
    }
}

/// A type that carries no data: `PhantomData`, which the emitted class does not
/// declare a field for.
pub(super) fn is_zero_sized(ts_ty: &str) -> bool {
    ts_ty.contains("PhantomData")
}

/// The width, spelled for a message: `a u8`, `an i64`.
fn article(prim: crate::ty::Prim) -> String {
    let name = prim.rust_name();
    let article = if name.starts_with('i') { "an" } else { "a" };
    format!("{article} {name}")
}


/// Is this field a `char`, rather than one of the other Rust types the port
/// writes as a `string`?
fn is_char(ty: Option<&crate::ty::Ty>) -> bool {
    matches!(
        ty.map(crate::ty::Ty::peel_refs),
        Some(crate::ty::Ty::Prim(crate::ty::Prim::Char))
    )
}

/// The integer width this field really is, where the resolution settled it.
///
/// The TypeScript spelling cannot say: `u8`, `u32`, `usize` and `f64` are all
/// `number`. The `Ty` is what carries the Rust type, and the reader's checks
/// come from it.
fn integer_prim(ty: Option<&crate::ty::Ty>) -> Option<crate::ty::Prim> {
    match ty?.peel_refs() {
        crate::ty::Ty::Prim(prim) if prim.is_integer() && prim.range().is_some() => Some(*prim),
        _ => None,
    }
}

fn starts_upper(t: &str) -> bool {
    t.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Does the port emit a `static fromJson` for this class?
///
/// The registry knows, from the derive, from the `[provided_impls]` entry where
/// a person wrote the file, and from the narrowing pass that makes the refusal
/// transitive. The IDENTITY is what it is asked of: a leaf scan finds the first
/// `State` of however many crates declare one. The leaf is the fallback for a
/// spelling the engine could not resolve, which is a guess and is the reason the
/// old reader was wrong.
///
/// "This class is hand-written" is NOT evidence: `auth.provided.ts` declares no
/// `fromJson`, and reading the two as one put `Attested.fromJson` in three
/// emitted call sites where nothing declares it.
fn reads_json(reg: &TypeRegistry, class: &str, ty: Option<&crate::ty::Ty>) -> bool {
    let id = match ty.and_then(|ty| ty.peel_refs().id()) {
        Some(id) => Some(id),
        None => reg.type_by_leaf(class),
    };
    id.is_some_and(|id| reg.reads_json(id))
}

/// The one type argument a wrapper carries — an `Option`'s, a `Vec`'s — the
/// same question the bincode half asks of a container.
fn element_of(ty: Option<&crate::ty::Ty>) -> Option<&crate::ty::Ty> {
    match ty?.peel_refs() {
        crate::ty::Ty::Named { args, .. } if args.len() == 1 => args.first(),
        crate::ty::Ty::Array { elem, .. } => Some(elem),
        crate::ty::Ty::Slice(elem) => Some(elem),
        _ => None,
    }
}

/// The nth argument of a map or the nth part of a tuple.
fn argument_of(ty: Option<&crate::ty::Ty>, n: usize) -> Option<&crate::ty::Ty> {
    match ty?.peel_refs() {
        crate::ty::Ty::Named { args, .. } => args.get(n),
        crate::ty::Ty::Tuple(parts) => parts.get(n),
        _ => None,
    }
}

/// `K, V` of a `HashMap<K, V>`, split on the comma that is not inside brackets.
fn split_arguments(inner: &str) -> Option<(String, String)> {
    let parts = tuple_parts(inner)?;
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0].clone(), parts[1].clone()))
}

/// The parts of a comma-separated type list, respecting nesting. Splitting on
/// `", "` alone read `[string, number]` as two arguments, which is how the
/// bincode half lost a map's widths.
fn tuple_parts(inner: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in inner.char_indices() {
        match ch {
            '<' | '[' | '(' | '{' => depth += 1,
            '>' | ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(inner[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let last = inner[start..].trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}
