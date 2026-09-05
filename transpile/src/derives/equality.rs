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

use crate::registry::TypeRegistry;
use crate::types::EnumInfo;

/// `equals` for an enum: the variant first, then the payload both values carry.
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
                let ts = field.ts_ty(reg);
                let base = ts.trim_end_matches(" | null");
                if ts.ends_with(" | null") {
                    out.push_str(&format!(
                        "        if ({m} === null || {o} === null) {{ if ({m} !== {o}) return false; }}\n        else {}\n",
                        field_eq_at(&mine, &theirs, base),
                        m = mine,
                        o = theirs
                    ));
                } else {
                    out.push_str(&format!(
                        "        {}\n",
                        field_eq_at(&mine, &theirs, base)
                    ));
                }
            }
            out.push_str("        break;\n      }\n");
        }
        out.push_str("    }\n");
    }
    out.push_str("    return true;\n  }\n");
    out
}

// ── How two values of one type are compared ─────────────────────────

/// One field compared for equality, by the two places it is read from.
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
pub(crate) fn field_eq_at(mine: &str, theirs: &str, ty: &str) -> String {
    compare(mine, theirs, ty, 0, &[])
}

/// The same, told the declaring type's own PARAMETERS.
///
/// A field written as `T` is one whose comparison emission cannot fix: `T` is a
/// number in `Holder<u32>` and a class in `Holder<Item>`, and `.equals()` on a
/// number is a TypeError. `keysEqual` decides at run time, by the value's own
/// surface — which is the same question `HashMap` already asks of a key.
pub(crate) fn field_eq_within(mine: &str, theirs: &str, ty: &str, params: &[String]) -> String {
    compare(mine, theirs, ty, 0, params)
}

/// The same, told how deep inside a container it is, so that a loop inside a
/// loop names its own index and its own pair.
fn compare(mine: &str, theirs: &str, ty: &str, depth: usize, params: &[String]) -> String {
    let ty = ty.trim();
    if params.iter().any(|p| p == ty) {
        return format!("if (!derivedEquals({}, {})) return false;", mine, theirs);
    }
    // A nullable is two questions: are they both absent, and — where they are
    // not — are the values inside the same. Without the first, a `T | null`
    // field called `equals` on `null`.
    if let Some(inner) = ty.strip_suffix("| null").map(str::trim) {
        return format!(
            "{{ if (({m} == null) !== ({o} == null)) return false; if ({m} != null) {{ {c} }} }}",
            m = mine,
            o = theirs,
            c = compare(mine, theirs, inner, depth, params)
        );
    }
    if crate::emit::is_primitive_ts_type(ty) {
        return format!("if ({} !== {}) return false;", mine, theirs);
    }
    if ty == "Uint8Array" {
        let i = index_name(depth);
        return format!(
            "{{ if ({m}.length !== {o}.length) return false; for (let {i} = 0; {i} < {m}.length; {i}++) {{ if ({m}[{i}] !== {o}[{i}]) return false; }} }}",
            m = mine, o = theirs, i = i
        );
    }
    if let Some(value_ty) = map_value_ty(ty) {
        // Size, keys AND VALUES. Comparing size and keys alone answered `true`
        // for two maps that agree about which keys they hold and about nothing
        // else — proto's `data.ts` compared two `HashMap<PropertyName, Value>`
        // that way, and a derived `equals` that ignores half the map is a wrong
        // answer wherever the type is a HashMap key.
        let (k, v, w) = (name("k", depth), name("v", depth), name("_w", depth));
        return format!(
            "{{ if ({m}.size !== {o}.size) return false; for (const [{k}, {v}] of {m}) {{ if (!{o}.has({k})) return false; const {w} = {o}.get({k})!; {c} }} }}",
            m = mine, o = theirs, k = k, v = v, w = w,
            c = compare(&v, &w, &value_ty, depth + 1, params)
        );
    }
    if let Some(_) = set_element_ty(ty) {
        // Rust's `HashSet` equality is set equality: the same size, and every
        // member of one held by the other. The runtime's `HashSet` decides
        // membership by the element's own `hash`/`equals`, so `has` is the
        // element comparison and there is nothing to recurse into.
        let e = name("e", depth);
        return format!(
            "{{ if ({m}.size !== {o}.size) return false; for (const {e} of {m}) {{ if (!{o}.has({e})) return false; }} }}",
            m = mine, o = theirs, e = e
        );
    }
    if let Some(inner) = ty.strip_suffix("[]") {
        let i = index_name(depth);
        return format!(
            "{{ if ({m}.length !== {o}.length) return false; for (let {i} = 0; {i} < {m}.length; {i}++) {{ {c} }} }}",
            m = mine, o = theirs, i = i,
            c = compare(&format!("{}[{}]", mine, i), &format!("{}[{}]", theirs, i), inner, depth + 1, params)
        );
    }
    format!("if (!{}.equals({})) return false;", mine, theirs)
}

/// The index one loop uses. The outermost keeps the `i` it always had, so a
/// type with no container inside a container reads as it did.
fn index_name(depth: usize) -> String {
    name("i", depth)
}

fn name(stem: &str, depth: usize) -> String {
    if depth == 0 { stem.to_string() } else { format!("{}{}", stem, depth) }
}

/// `V` of a written `HashMap<K, V>` or `Map<K, V>`, at the top level of the
/// argument list.
fn map_value_ty(ty: &str) -> Option<String> {
    let inner = ty
        .strip_prefix("HashMap<")
        .or_else(|| ty.strip_prefix("BTreeMap<"))
        .or_else(|| ty.strip_prefix("Map<"))?
        .strip_suffix('>')?;
    Some(after_top_level_comma(inner)?.to_string())
}

/// `T` of a written `HashSet<T>` or `Set<T>`.
fn set_element_ty(ty: &str) -> Option<String> {
    let inner = ty
        .strip_prefix("HashSet<")
        .or_else(|| ty.strip_prefix("BTreeSet<"))
        .or_else(|| ty.strip_prefix("Set<"))?
        .strip_suffix('>')?;
    Some(inner.trim().to_string())
}

/// What stands after the first comma that is not inside a nested argument list.
fn after_top_level_comma(inner: &str) -> Option<&str> {
    let mut depth = 0usize;
    for (at, ch) in inner.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(inner[at + 1..].trim()),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{field_eq_at, field_eq_within};
    use crate::testing::Fixture;

    /// PREMISE CHANGED 2026-09-05 (fixpass4 item 3): the rule this pins used to
    /// stop one level down — the doc comment said "a nested one is compared
    /// elementwise where it stands" and the code called `.equals()` on it. A
    /// `Uint8Array` has none, so proto's `StateBuffers` threw a TypeError on two
    /// maps of bytes, and `OperationSet`, whose values are `Operation[]`, threw
    /// one too.
    #[test]
    fn a_container_inside_a_container_is_compared_by_what_it_holds() {
        let bytes_in_a_map = field_eq_at("a", "b", "HashMap<string, Uint8Array>");
        assert!(bytes_in_a_map.contains("v.length !== _w.length"), "{}", bytes_in_a_map);
        assert!(bytes_in_a_map.contains("v[i1] !== _w[i1]"), "{}", bytes_in_a_map);
        assert!(!bytes_in_a_map.contains("v.equals"), "{}", bytes_in_a_map);

        let objects_in_a_map = field_eq_at("a", "b", "HashMap<string, Operation[]>");
        assert!(objects_in_a_map.contains("v[i1].equals(_w[i1])"), "{}", objects_in_a_map);

        // Three deep, and each loop names its own index.
        let deep = field_eq_at("a", "b", "Uint8Array[][]");
        assert!(deep.contains("let i = 0"), "{}", deep);
        assert!(deep.contains("let i1 = 0"), "{}", deep);
        assert!(deep.contains("let i2 = 0"), "{}", deep);
    }

    /// A `HashSet` carries no `equals` either, and Rust's set equality is the
    /// same size plus every member of one held by the other.
    #[test]
    fn a_set_is_compared_as_a_set() {
        let written = field_eq_at("a", "b", "HashSet<Id>");
        assert!(written.contains("a.size !== b.size"), "{}", written);
        assert!(written.contains("if (!b.has(e)) return false;"), "{}", written);
        assert!(!written.contains(".equals("), "{}", written);
    }

    /// A nullable field is two questions, and the second is only asked where
    /// there is something to ask it of: `T | null` called `equals` on `null`.
    #[test]
    fn a_nullable_field_asks_whether_both_are_absent_first() {
        let written = field_eq_at("this.x", "other.x", "Id | null");
        assert!(written.contains("(this.x == null) !== (other.x == null)"), "{}", written);
        assert!(written.contains("if (this.x != null)"), "{}", written);
        assert!(written.contains("this.x.equals(other.x)"), "{}", written);
    }

    /// A field written as one of the type's own PARAMETERS is one the emitter
    /// cannot compare — `T` is a number in `Holder<u32>` and a class in
    /// `Holder<Item>`, and `.equals()` on a number is a TypeError — so the
    /// decision is the value's own surface at run time.
    #[test]
    fn a_field_written_as_a_type_parameter_is_compared_at_run_time() {
        let params = vec!["T".to_string(), "E".to_string()];
        assert_eq!(
            field_eq_within("this.x", "other.x", "T", &params),
            "if (!derivedEquals(this.x, other.x)) return false;"
        );
        let in_a_list = field_eq_within("this.rows", "other.rows", "E[]", &params);
        assert!(in_a_list.contains("derivedEquals(this.rows[i], other.rows[i])"), "{}", in_a_list);
        // A type that is NOT a parameter answers its own `equals`, as before.
        assert_eq!(
            field_eq_within("this.x", "other.x", "Id", &params),
            "if (!this.x.equals(other.x)) return false;"
        );
    }

    /// A primitive has no `equals` either, at any depth.
    #[test]
    fn a_primitive_is_compared_by_identity() {
        assert_eq!(field_eq_at("a", "b", "number"), "if (a !== b) return false;");
        let in_a_map = field_eq_at("a", "b", "HashMap<string, bigint>");
        assert!(in_a_map.contains("if (v !== _w) return false;"), "{}", in_a_map);
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
