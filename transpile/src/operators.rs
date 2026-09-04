//! Operators: what `==`, `<`, `+` and `!` mean once the operands are named.
//!
//! For: JavaScript's operators are not Rust's. `a == b` on two objects compares
//! identity where Rust compares values, `a < b` coerces both sides to strings
//! or numbers where Rust calls `PartialOrd`, integer division leaves a fraction
//! behind, and `!n` on a number is a boolean where Rust flips the bits. Each of
//! those is a wrong answer the emitted code gives silently.
//!
//! Two questions decide every site: are the operands primitives, and if not,
//! which impl does Rust's operator resolve to. The first is the engine's
//! answer about the types; the second is the impl table's, and the method it
//! lands on is named by the same function emission names it with.

use crate::body::BodyTranslator;
use crate::registry::NoConversion;
use crate::ty::{Prim, Ty};

mod primitives;

#[cfg(test)]
mod tests;

/// What an operand is, as far as an operator is concerned.
enum Operand {
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
struct Operator {
    /// Where the trait is declared, so the impl table can be asked for it.
    trait_path: String,
    trait_name: &'static str,
    rust_method: &'static str,
    ts_method: String,
    native: &'static str,
}

fn operator_of(op: &syn::BinOp) -> Option<Operator> {
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

impl BodyTranslator<'_> {
    /// `a OP b`, written the way the port means it.
    ///
    /// `None` where the site has nothing special to say and the operands stand
    /// either side of the JavaScript operator, which is what the caller writes.
    pub(crate) fn binary_operator(
        &self,
        bin: &syn::ExprBinary,
        left: &str,
        right: &str,
    ) -> Option<String> {
        let op = operator_of(&bin.op)?;
        let span = syn::spanned::Spanned::span(bin);
        let lhs = self.quietly(|| self.resolve_expr_type(&bin.left)).ok();
        // The other operand is what says how wide an unsuffixed literal is, so
        // the right one is asked under the left one's type — the same question
        // emission asks when it writes the operand.
        let rhs = self
            .quietly(|| {
                self.expecting(&bin.right, lhs.as_ref(), || {
                    self.resolve_expr_type(&bin.right)
                })
            })
            .ok();
        // An operand the engine could not name leaves the operator as written.
        // Nothing is filed for it here: what the engine could not type is a gap
        // recorded where the name was bound — `x is bound here but the engine
        // could not type it` and its kin — and filing it again at every
        // operator that reads the name would count one gap once per use and
        // make the coverage number a use count.
        let (Some(lhs), Some(rhs)) = (lhs, rhs) else {
            return None;
        };
        match (self.operand_kind(&lhs), self.operand_kind(&rhs)) {
            // Both numbers: the JavaScript operator, with the corrections
            // JavaScript's arithmetic needs.
            (Operand::Number(a), Operand::Number(b)) => {
                self.primitive_operator(&op, a, b, left, right, span)
            }
            // A string compares and concatenates natively, and `str` and
            // `String` are one type here. So does a boolean.
            (Operand::Native, Operand::Native)
            | (Operand::Native, Operand::Number(_))
            | (Operand::Number(_), Operand::Native) => None,
            _ => self.overloaded_operator(&op, lhs.peel_refs(), rhs.peel_refs(), left, right, span),
        }
    }

    /// Which side of the divide an operand falls: a value JavaScript's own
    /// operators work on, or an object whose operator is an impl.
    fn operand_kind(&self, ty: &Ty) -> Operand {
        if let Ty::Prim(prim) = ty.peel_refs() {
            if !matches!(prim, Prim::Bool | Prim::Char) {
                return Operand::Number(*prim);
            }
            return Operand::Native;
        }
        let Some(tc) = &self.types else {
            return Operand::Object;
        };
        use crate::name_map::shape::{js_shape, JsShape};
        match js_shape(tc.borrow().registry, ty.peel_refs()) {
            JsShape::Str | JsShape::Boolean => Operand::Native,
            JsShape::Number => Operand::Number(Prim::F64),
            JsShape::BigInt => Operand::Number(Prim::I64),
            _ => Operand::Object,
        }
    }

    /// An operator on operands that are not primitives: the impl's method.
    fn overloaded_operator(
        &self,
        op: &Operator,
        lhs: &Ty,
        rhs: &Ty,
        left: &str,
        right: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let describe = {
            let tc = tc.borrow();
            (tc.registry.describe(lhs), tc.registry.describe(rhs))
        };
        let say = |why: String| {
            self.fallback(
                span,
                format!(
                    "`{}` between `{}` and `{}` resolves through `{}`, and {}; the JavaScript \
                     operator is written, which compares references rather than values",
                    op.native, describe.0, describe.1, op.trait_name, why
                ),
            );
        };
        let found = {
            let tc = tc.borrow();
            tc.probe().operator_impl(&op.trait_path, lhs, rhs)
        };
        let found = match found {
            Ok(found) => found,
            Err(why) => {
                say(match why {
                    NoConversion::NoTrait => format!("`{}` is not declared", op.trait_name),
                    NoConversion::None => "no impl in the table performs it".to_string(),
                    NoConversion::Ambiguous(ids) => {
                        format!("{} impls in the table perform it", ids.len())
                    }
                });
                return None;
            }
        };
        let tc = tc.borrow();
        let def = tc.registry.impl_def(found.impl_id);
        // An impl the declared surface wrote is the runtime's own operator, and
        // the runtime writes nothing for it: `Vec<u8> == Vec<u8>` in Rust
        // compares element by element and `===` on two arrays compares
        // references.
        if tc.registry.modules().get(def.module).is_system {
            drop(tc);
            say("the impl is the declared surface's, so the comparison is the runtime's own \
                 and `@ankurah/base` supplies none"
                .to_string());
            return None;
        }
        if !crate::emit_impls::has_emitted_class(tc.registry, &def.self_ty) {
            drop(tc);
            say("the left operand has no class of its own for the method to be on".to_string());
            return None;
        }
        let args: Vec<String> = def
            .trait_ref
            .as_ref()
            .map(|t| {
                t.args
                    .iter()
                    .map(|ty| crate::name_map::map_ty(tc.registry, ty))
                    .collect()
            })
            .unwrap_or_default();
        let member =
            crate::emit::impl_method_name(op.trait_name, op.rust_method, &op.ts_method, &args);
        drop(tc);
        Some(match op.trait_name {
            "PartialEq" => {
                let call = format!("{}.{}({})", left, member, right);
                if op.native == "!==" {
                    format!("!{}", call)
                } else {
                    call
                }
            }
            // `partial_cmp` hands back an ordering, and the port writes it as a
            // number; the operator is the sign test Rust's own default methods
            // perform.
            "PartialOrd" => format!("{}.{}({}) {} 0", left, member, right, op.native),
            _ => format!("{}.{}({})", left, member, right),
        })
    }
}

/// `!x` and `-x`, where the operand is not a boolean or a signed number.
impl BodyTranslator<'_> {
    /// Rust's `!` is bitwise on an integer and logical on a boolean; JavaScript
    /// spells those `~` and `!` and means the boolean one by `!` whatever it is
    /// applied to, so `!bits` on a `u64` came out as `false`.
    pub(crate) fn unary_not(&self, unary: &syn::ExprUnary, written: &str) -> Option<String> {
        let ty = self.quietly(|| self.resolve_expr_type(&unary.expr)).ok()?;
        match ty.peel_refs() {
            Ty::Prim(Prim::Bool) => None,
            // JavaScript's `~` produces a signed 32-bit number whatever it was
            // given, so the complement of a `u32` came out negative and the
            // complement of a `u64` lost every bit above 32. Bringing it back
            // inside the type's range is what Rust's own `!` leaves.
            Ty::Prim(prim) if prim.is_integer() => {
                Some(crate::convert::cast::wrap(*prim, &format!("~{}", written)))
            }
            _ => {
                self.fallback(
                    syn::spanned::Spanned::span(unary),
                    "`!` here is neither a boolean negation nor an integer's complement, so it \
                     resolves through `Not`, which the engine does not write; the JavaScript \
                     `!` is emitted",
                );
                None
            }
        }
    }
}
