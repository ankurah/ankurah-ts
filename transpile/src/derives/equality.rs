//! `#[derive(PartialEq)]` on an ENUM.
//!
//! For: a struct's derived `equals` compares the fields it declares, and an
//! enum declares none — its fields live on its variants. Handed the empty list
//! it wrote `equals(other) { return true; }`, so every `Literal` equalled every
//! other `Literal`, every `Value` every other `Value`, and every `ChangeKind`
//! every other `ChangeKind`: 26 emitted enums. That is load-bearing, because
//! the runtime's `HashMap` decides whether two keys are one by asking `equals`,
//! so a lookup answered the first key of the right shape it happened to hold.
//!
//! Rust's derived `PartialEq` on an enum is: the same variant, and then the
//! payload field by field. That is what `compareTo` already writes, and this
//! writes the same walk with equality where that has a comparison.

use crate::name_map::shape::{js_shape, JsShape};
use crate::registry::TypeRegistry;
use crate::ty::Ty;
use crate::types::EnumInfo;

/// `equals` for an enum: the variant first, then the payload both values carry.
///
/// A payload field whose type is one of the enum's own parameters is a number
/// in `Slot<u32>` and a class in `Slot<Item>`, and `.equals()` on a number is a
/// `TypeError`; the struct writer was told this in the fourth pass (§3.8) and
/// this one was not, so `RangeBound<T>`, `ExprOutput<T>`, `FilterResult<R>` and
/// `ItemChange<I>` all compared their payloads with a method the value may not
/// have. `compare` reads it off the resolved type now, so both writers agree.
pub fn enum_equals(reg: &TypeRegistry, e: &EnumInfo, full_type: &str) -> String {
    let mut out = format!("\n  equals(other: {}): boolean {{\n", full_type);
    out.push_str("    if (this.type !== other.type) return false;\n");
    let carrying: Vec<&crate::types::VariantInfo> =
        e.variants.iter().filter(|v| !v.fields.is_empty()).collect();
    if !carrying.is_empty() {
        out.push_str("    switch (this.type) {\n");
        for variant in carrying {
            out.push_str(&format!("      case '{}': {{\n", variant.name));
            for (index, field) in variant.fields.iter().enumerate() {
                let name = field.name.clone().unwrap_or_else(|| format!("_{}", index));
                let mine = format!("(this.value as any).{}", name);
                let theirs = format!("(other.value as any).{}", name);
                // The nullable case is `compare`'s own, written once: this
                // wrote a second spelling of it — `=== null`, which a field
                // holding `undefined` slips past, where `compare` writes
                // `== null`, which it does not.
                out.push_str(&format!(
                    "        {}\n",
                    field_eq_within(reg, &mine, &theirs, field.ty.as_ref())
                ));
            }
            out.push_str("        break;\n      }\n");
        }
        out.push_str("    }\n");
    }
    out.push_str("    return true;\n  }\n");
    out
}

// ── How two values of one type are compared ─────────────────────────

/// (How one field is compared, and why the rule recurses.)
///
/// For: how two values are the same depends on what the field IS. A
/// `Uint8Array`, a `HashMap`, a `HashSet` and an array carry no `equals` of
/// their own, and a primitive carries none either, so each has a comparison
/// written out for it. The rule is the same at every DEPTH, which is what this
/// got wrong: a `HashMap<String, Uint8Array>` compared its values with
/// `v.equals(_w)`, and proto's `StateBuffers` threw a TypeError on two maps of
/// bytes; so did `OperationSet`, whose values are `Operation[]`.
///
/// `mine` and `theirs` are written expressions, not names, so the same rules
/// serve a struct's `this.x`/`other.x` and an enum payload's
/// `(this.value as any)._0`.
/// One field compared for equality, by the two places it is read from and the
/// field's RESOLVED type.
///
/// The type is the resolved `Ty`, never the TypeScript the field is written
/// with. Reading the rendered text made the rule a parser of its own output and
/// it read three things wrong: `(Token | null)[]` is an array of nullables and
/// the `| null` suffix matched the whole spelling, so `this.rows[i].equals(..)`
/// ran on a `null`; a type ALIAS for `Vec<u8>` was compared with an `equals` a
/// `Uint8Array` does not have; and a `char`, which the port writes as a string,
/// was neither.
///
/// A field whose type is one of the declaring type's own PARAMETERS is one the
/// emission cannot fix — `T` is a number in `Holder<u32>` and a class in
/// `Holder<Item>`, and `.equals()` on a number is a TypeError — so it goes to
/// `derivedEquals`, which decides by the value's own surface at run time.
pub(crate) fn field_eq_within(
    reg: &TypeRegistry,
    mine: &str,
    theirs: &str,
    ty: Option<&Ty>,
) -> String {
    match ty {
        Some(ty) => compare(reg, mine, theirs, ty, 0),
        // A field the engine could not type is reported where it is declared;
        // here it is a value of unknown surface, which is the question
        // `derivedEquals` was written to answer.
        None => by_surface(mine, theirs),
    }
}

/// The comparison for a value whose surface only the runtime knows.
fn by_surface(mine: &str, theirs: &str) -> String {
    format!("if (!derivedEquals({}, {})) return false;", mine, theirs)
}

/// The same, told how deep inside a container it is, so that a loop inside a
/// loop names its own index and its own pair.
fn compare(reg: &TypeRegistry, mine: &str, theirs: &str, ty: &Ty, depth: usize) -> String {
    // A bare parameter, and a projection nothing settled, are both types the
    // emission has no name for.
    if matches!(ty, Ty::Param(_) | Ty::Assoc { .. } | Ty::Infer) {
        return by_surface(mine, theirs);
    }
    // Every width, `char` and `bool` are compared by identity. `char` is asked
    // here rather than through the shape table, which calls it `Plain` because
    // it is the one width the port writes as a string.
    if matches!(ty, Ty::Prim(_) | Ty::Str | Ty::Unit) {
        return format!("if ({} !== {}) return false;", mine, theirs);
    }
    match js_shape(reg, ty) {
        // `Box<T>` and `&T` are written as the `T` inside them.
        JsShape::SameAs(inner) => compare(reg, mine, theirs, &inner, depth),
        JsShape::Number | JsShape::BigInt | JsShape::Boolean | JsShape::Str | JsShape::Void => {
            format!("if ({} !== {}) return false;", mine, theirs)
        }
        // A nullable is two questions: are they both absent, and — where they
        // are not — are the values inside the same. Without the first, a
        // `T | null` field called `equals` on `null`.
        //
        // The first test proves the two agree, so inside the second both are
        // there; the inner comparison says so with `!` rather than relying on
        // TypeScript's narrowing, which does not follow an element access
        // through a loop variable — `this.slots[i]` stays `Tag | null` however
        // it was tested.
        JsShape::Nullable(inner) => format!(
            "{{ if (({m} == null) !== ({o} == null)) return false; if ({m} != null) {{ {c} }} }}",
            m = mine,
            o = theirs,
            c = compare(
                reg,
                &format!("{}!", mine),
                &format!("{}!", theirs),
                &inner,
                depth
            )
        ),
        JsShape::Bytes => {
            let i = index_name(depth);
            format!(
                "{{ if ({m}.length !== {o}.length) return false; for (let {i} = 0; {i} < {m}.length; {i}++) {{ if ({m}[{i}] !== {o}[{i}]) return false; }} }}",
                m = mine, o = theirs, i = i
            )
        }
        // Size, keys AND VALUES. Comparing size and keys alone answered `true`
        // for two maps that agree about which keys they hold and about nothing
        // else — proto's `data.ts` compared two `HashMap<PropertyName, Value>`
        // that way, and a derived `equals` that ignores half the map is a wrong
        // answer wherever the type is a HashMap key.
        JsShape::Map(_, value_ty) => {
            let (k, v, w) = (name("k", depth), name("v", depth), name("_w", depth));
            format!(
                "{{ if ({m}.size !== {o}.size) return false; for (const [{k}, {v}] of {m}) {{ if (!{o}.has({k})) return false; const {w} = {o}.get({k})!; {c} }} }}",
                m = mine, o = theirs, k = k, v = v, w = w,
                c = compare(reg, &v, &w, &value_ty, depth + 1)
            )
        }
        // Rust's `HashSet` equality is set equality: the same size, and every
        // member of one held by the other. The runtime's `HashSet` decides
        // membership by the element's own `hash`/`equals`, so `has` is the
        // element comparison and there is nothing to recurse into.
        JsShape::Set(_) => {
            let e = name("e", depth);
            format!(
                "{{ if ({m}.size !== {o}.size) return false; for (const {e} of {m}) {{ if (!{o}.has({e})) return false; }} }}",
                m = mine, o = theirs, e = e
            )
        }
        JsShape::Array(elem) => {
            let i = index_name(depth);
            format!(
                "{{ if ({m}.length !== {o}.length) return false; for (let {i} = 0; {i} < {m}.length; {i}++) {{ {c} }} }}",
                m = mine, o = theirs, i = i,
                c = compare(reg, &format!("{}[{}]", mine, i), &format!("{}[{}]", theirs, i), &elem, depth + 1)
            )
        }
        // A tuple is a JavaScript array and has no `equals` of its own, so each
        // position is compared by its own rule — the recursion `clone_at` has
        // had since the fourth pass, which this was written beside and did not
        // get. Live at `core/reactor/update.ts` on a
        // `[QueryId, MembershipChange][]` and at `storage-common/types.ts` on a
        // `[Value[], boolean] | null`, both of which the clone writer got right.
        JsShape::Tuple(parts) => {
            let each: Vec<String> = parts
                .iter()
                .enumerate()
                .map(|(at, part)| {
                    compare(
                        reg,
                        &format!("{}[{}]", mine, at),
                        &format!("{}[{}]", theirs, at),
                        part,
                        depth + 1,
                    )
                })
                .collect();
            format!("{{ {} }}", each.join(" "))
        }
        _ => format!("if (!{}.equals({})) return false;", mine, theirs),
    }
}

/// The index one loop uses. The outermost keeps the `i` it always had, so a
/// type with no container inside a container reads as it did.
fn index_name(depth: usize) -> String {
    name("i", depth)
}

fn name(stem: &str, depth: usize) -> String {
    if depth == 0 { stem.to_string() } else { format!("{}{}", stem, depth) }
}

#[cfg(test)]
mod tests {
    use super::field_eq_within;
    use crate::testing::Fixture;

    /// A crate whose types cover the shapes the rule turns on.
    const PRELUDE: &str = "\
pub struct Tag { pub n: u32 }\n\
pub struct Id(pub u32);\n\
pub struct Operation { pub n: u32 }\n\
";

    /// The comparison for one written Rust type, resolved the way a field of it
    /// would be.
    fn eq_of(rust_ty: &str) -> String {
        eq_within(rust_ty, &[])
    }

    /// The same, inside a type that declares parameters of its own.
    fn eq_within(rust_ty: &str, params: &[&str]) -> String {
        let f = Fixture::build(&[("lib.rs", PRELUDE)]);
        let ty = f.ty_in("lib.rs", rust_ty, params).expect("resolves");
        field_eq_within(&f.reg, "a", "b", Some(&ty))
    }

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 3): the rule this pins used to
    /// stop one level down — the doc comment said "a nested one is compared
    /// elementwise where it stands" and the code called `.equals()` on it. A
    /// `Uint8Array` has none, so proto's `StateBuffers` threw a TypeError on two
    /// maps of bytes, and `OperationSet`, whose values are `Operation[]`, threw
    /// one too.
    #[test]
    fn a_container_inside_a_container_is_compared_by_what_it_holds() {
        let bytes_in_a_map = eq_of("std::collections::HashMap<String, Vec<u8>>");
        assert!(bytes_in_a_map.contains("v.length !== _w.length"), "{}", bytes_in_a_map);
        assert!(bytes_in_a_map.contains("v[i1] !== _w[i1]"), "{}", bytes_in_a_map);
        assert!(!bytes_in_a_map.contains("v.equals"), "{}", bytes_in_a_map);

        let objects_in_a_map = eq_of("std::collections::HashMap<String, Vec<Operation>>");
        assert!(objects_in_a_map.contains("v[i1].equals(_w[i1])"), "{}", objects_in_a_map);

        // Three deep, and each loop names its own index.
        let deep = eq_of("Vec<Vec<Vec<u8>>>");
        assert!(deep.contains("let i = 0"), "{}", deep);
        assert!(deep.contains("let i1 = 0"), "{}", deep);
        assert!(deep.contains("let i2 = 0"), "{}", deep);
    }

    /// A `HashSet` carries no `equals` either, and Rust's set equality is the
    /// same size plus every member of one held by the other.
    #[test]
    fn a_set_is_compared_as_a_set() {
        let written = eq_of("std::collections::HashSet<Id>");
        assert!(written.contains("a.size !== b.size"), "{}", written);
        assert!(written.contains("if (!b.has(e)) return false;"), "{}", written);
        assert!(!written.contains(".equals("), "{}", written);
    }

    /// A nullable field is two questions, and the second is only asked where
    /// there is something to ask it of: `T | null` called `equals` on `null`.
    #[test]
    fn a_nullable_field_asks_whether_both_are_absent_first() {
        let written = eq_of("Option<Id>");
        assert!(written.contains("(a == null) !== (b == null)"), "{}", written);
        assert!(written.contains("if (a != null)"), "{}", written);
        assert!(written.contains("a!.equals(b!)"), "{}", written);
    }

    /// An array of NULLABLES is not a nullable array, and the difference was
    /// invisible in the rendered TypeScript the rule used to read: `(Token |
    /// null)[]` ends in `[]`, and stripping a ` | null` suffix off it stripped
    /// the ELEMENT's. `this.rows[i].equals(..)` then ran on a `null`.
    #[test]
    fn an_array_of_nullables_guards_each_element() {
        let written = eq_of("Vec<Option<Tag>>");
        assert!(written.contains("a.length !== b.length"), "{}", written);
        assert!(written.contains("(a[i] == null) !== (b[i] == null)"), "{}", written);
        assert!(written.contains("a[i]!.equals(b[i]!)"), "{}", written);
    }

    /// A field whose type is one of the declaring type's own PARAMETERS is one
    /// the emitter cannot compare — `T` is a number in `Holder<u32>` and a class
    /// in `Holder<Item>`, and `.equals()` on a number is a TypeError — so the
    /// decision is the value's own surface at run time.
    #[test]
    fn a_field_written_as_a_type_parameter_is_compared_at_run_time() {
        assert_eq!(
            eq_within("T", &["T", "E"]),
            "if (!derivedEquals(a, b)) return false;"
        );
        let in_a_list = eq_within("Vec<E>", &["T", "E"]);
        assert!(in_a_list.contains("derivedEquals(a[i], b[i])"), "{}", in_a_list);
        // A type that is NOT a parameter answers its own `equals`, as before.
        assert_eq!(eq_within("Id", &["T", "E"]), "if (!a.equals(b)) return false;");
    }

    /// A primitive has no `equals` either, at any depth — and `char`, which the
    /// port writes as a string, is one of them.
    #[test]
    fn a_primitive_is_compared_by_identity() {
        assert_eq!(eq_of("u32"), "if (a !== b) return false;");
        assert_eq!(eq_of("char"), "if (a !== b) return false;");
        let in_a_map = eq_of("std::collections::HashMap<String, u64>");
        assert!(in_a_map.contains("if (v !== _w) return false;"), "{}", in_a_map);
    }

    /// A type ALIAS is compared by what it stands for. Read off the rendered
    /// TypeScript, which keeps the alias's own name, `Bytes` answered an
    /// `equals` a `Uint8Array` does not have.
    #[test]
    fn an_alias_is_compared_by_what_it_stands_for() {
        let f = Fixture::build(&[("lib.rs", "pub type Bytes = Vec<u8>;\n")]);
        let ty = f.ty("lib.rs", "Bytes");
        let written = field_eq_within(&f.reg, "a", "b", Some(&ty));
        assert!(written.contains("a[i] !== b[i]"), "{}", written);
    }

    /// A field the engine could not type is a value of unknown surface, which
    /// is the question `derivedEquals` answers.
    #[test]
    fn a_field_with_no_resolved_type_is_compared_at_run_time() {
        let f = Fixture::build(&[("lib.rs", "pub struct Tag { pub n: u32 }\n")]);
        assert_eq!(
            field_eq_within(&f.reg, "a", "b", None),
            "if (!derivedEquals(a, b)) return false;"
        );
    }

    fn equals_of(src: &str, name: &str) -> String {
        let f = Fixture::build(&[("lib.rs", src)]);
        let e = f.files[0].file.enums.iter().find(|e| e.name == name).expect("enum");
        super::enum_equals(&f.reg, e, name)
    }

    /// An enum declares no fields of its own — they live on its variants — so
    /// the struct rule, handed the empty list, wrote `return true`: every
    /// `Literal` equalled every other `Literal`. Load-bearing, because the
    /// runtime's `HashMap` asks `equals` whether two keys are one.
    #[test]
    fn an_enum_compares_its_variant_first() {
        let ts = equals_of("pub enum Order { Less, Equal, Greater }", "Order");
        assert!(ts.contains("if (this.type !== other.type) return false;"), "{}", ts);
        assert!(!ts.contains("switch"), "there is no payload to compare:\n{}", ts);
    }

    #[test]
    fn an_enum_compares_the_payload_of_the_variant_both_carry() {
        let ts = equals_of(
            "pub struct Item { pub n: usize }\n\
             pub enum Slot { Empty, One(Item), Two { left: usize, right: String } }",
            "Slot",
        );
        assert!(ts.contains("case 'One'"), "{}", ts);
        assert!(
            ts.contains("if (!(this.value as any)._0.equals((other.value as any)._0)) return false;"),
            "{}",
            ts
        );
        assert!(ts.contains("case 'Two'"), "{}", ts);
        assert!(
            ts.contains("if ((this.value as any).left !== (other.value as any).left) return false;"),
            "a number is compared with `!==`:\n{}",
            ts
        );
        assert!(!ts.contains("case 'Empty'"), "a variant with no payload has nothing to compare:\n{}", ts);
    }
}
