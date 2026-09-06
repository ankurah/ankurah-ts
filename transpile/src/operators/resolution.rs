//! Which operator a written symbol IS: the trait it resolves through, the
//! method that trait declares, and what stands between two primitives.
//!
//! Split out from the emission beside it because it is a table and two
//! questions asked of it, and neither reads the body being translated.

use crate::ty::Prim;

/// What an operand is, as far as an operator is concerned.
pub(super) enum Operand {
    /// A number, and which one: the width decides truncation and whether the
    /// port writes it as a `bigint`.
    Number(Prim),
    /// A string or a boolean, which JavaScript's own operators compare by
    /// value.
    Native,
    /// Anything else, whose operator is an impl.
    Object,
}

/// One operator: the trait it resolves through, that trait's method, and the
/// text that stands between the two operands when both are primitives.
#[derive(Clone)]
pub(crate) struct Operator {
    /// Where the trait is declared, so the impl table can be asked for it.
    pub(crate) trait_path: String,
    pub(crate) trait_name: &'static str,
    pub(crate) rust_method: &'static str,
    pub(crate) ts_method: String,
    pub(crate) native: &'static str,
}

pub(super) fn operator_of(op: &syn::BinOp) -> Option<Operator> {
    use syn::BinOp::*;
    let (trait_name, rust_method, native) = match op {
        Add(_) => ("Add", "add", "+"),
        Sub(_) => ("Sub", "sub", "-"),
        Mul(_) => ("Mul", "mul", "*"),
        Div(_) => ("Div", "div", "/"),
        Rem(_) => ("Rem", "rem", "%"),
        BitXor(_) => ("BitXor", "bitxor", "^"),
        BitAnd(_) => ("BitAnd", "bitand", "&"),
        BitOr(_) => ("BitOr", "bitor", "|"),
        Shl(_) => ("Shl", "shl", "<<"),
        Shr(_) => ("Shr", "shr", ">>"),
        Eq(_) => ("PartialEq", "eq", "==="),
        Ne(_) => ("PartialEq", "eq", "!=="),
        Lt(_) => ("PartialOrd", "partial_cmp", "<"),
        Le(_) => ("PartialOrd", "partial_cmp", "<="),
        Gt(_) => ("PartialOrd", "partial_cmp", ">"),
        Ge(_) => ("PartialOrd", "partial_cmp", ">="),
        AddAssign(_) => ("AddAssign", "add_assign", "+="),
        SubAssign(_) => ("SubAssign", "sub_assign", "-="),
        MulAssign(_) => ("MulAssign", "mul_assign", "*="),
        DivAssign(_) => ("DivAssign", "div_assign", "/="),
        RemAssign(_) => ("RemAssign", "rem_assign", "%="),
        BitXorAssign(_) => ("BitXorAssign", "bitxor_assign", "^="),
        BitAndAssign(_) => ("BitAndAssign", "bitand_assign", "&="),
        BitOrAssign(_) => ("BitOrAssign", "bitor_assign", "|="),
        ShlAssign(_) => ("ShlAssign", "shl_assign", "<<="),
        ShrAssign(_) => ("ShrAssign", "shr_assign", ">>="),
        _ => return None,
    };
    let trait_path = match trait_name {
        "PartialEq" => "std::cmp::PartialEq".to_string(),
        "PartialOrd" => "std::cmp::PartialOrd".to_string(),
        other => format!("std::ops::{}", other),
    };
    Some(Operator {
        trait_path,
        trait_name,
        rust_method,
        ts_method: crate::name_map::to_camel_case(rust_method),
        native,
    })
}

/// `==` and `!=` performed by the RUNTIME rather than by an impl.
///
/// I8: these are the two operators the runtime can perform on any pair of
/// values, and every route to an impl that fails used to leave `===` standing —
/// identity where Rust compares contents, so the branch could never be taken.
/// `valueEquals` is the comparison: `===` for a primitive, element by element
/// for a sequence (bytes included), the value's own `equals()` for anything
/// that declares one, and a loud refusal for two objects that declare none,
/// because Rust's `==` would not have compiled without a `PartialEq` impl.
///
/// `None` for every other operator: `<` between two objects needs an ordering
/// the runtime cannot invent, and its diagnostic still stands.
pub(super) fn by_value_comparison(op: &Operator, left: &str, right: &str) -> Option<String> {
    (op.trait_name == "PartialEq").then(|| {
        let helper = if op.native == "!==" { "valueNotEquals" } else { "valueEquals" };
        format!("{}({}, {})", helper, left, right)
    })
}

/// The trait an operator resolves through, as the impl table knows it.
///
/// The type engine asks this to find what an overloaded operator *answers*:
/// `impl Add for Tag { type Output = Tag; }` is the only place that is written
/// down, and without it the local a `+` was bound to had no type and nothing
/// released what it held.
pub(crate) fn operator_trait(op: &syn::BinOp) -> Option<String> {
    operator_of(op).map(|found| found.trait_path)
}
