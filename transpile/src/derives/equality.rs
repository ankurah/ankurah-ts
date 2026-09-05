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
                        crate::emit::field_eq_at(&mine, &theirs, base),
                        m = mine,
                        o = theirs
                    ));
                } else {
                    out.push_str(&format!(
                        "        {}\n",
                        crate::emit::field_eq_at(&mine, &theirs, base)
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

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

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
