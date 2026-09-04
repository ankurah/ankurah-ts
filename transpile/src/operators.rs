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

/// The trait an operator resolves through, as the impl table knows it.
///
/// The type engine asks this to find what an overloaded operator *answers*:
/// `impl Add for Tag { type Output = Tag; }` is the only place that is written
/// down, and without it the local a `+` was bound to had no type and nothing
/// released what it held.
pub(crate) fn operator_trait(op: &syn::BinOp) -> Option<String> {
    operator_of(op).map(|found| found.trait_path)
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
        // The position types an unsuffixed literal: `bits ^ (1 << 63)` beside a
        // `u64` is 64-bit arithmetic, and asking about `1` on its own answered
        // `i32` — which then had the whole shift wrapped back into 32 bits.
        let want = self.expectation_at(span);
        let lhs = self
            .quietly(|| {
                self.expecting(&bin.left, want.as_ref(), || self.resolve_expr_type(&bin.left))
            })
            .ok();
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
            // `String` are one type here. So does a boolean — except for the
            // three operators Rust also gives booleans, which JavaScript reads
            // as bit arithmetic on numbers: `a ^ b` on two booleans is `0` or
            // `1`, not `true` or `false`.
            (Operand::Native, Operand::Native)
            | (Operand::Native, Operand::Number(_))
            | (Operand::Number(_), Operand::Native) => {
                self.boolean_operator(&op, &lhs, &rhs, left, right, &bin.right, span)
            }
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

/// Which operands an operator's own impl takes away from the block that held
/// them.
///
/// For: `a + b` between two ported types is a method call, and Rust's operator
/// traits take `self` and `rhs` by value — `fn add(self, rhs: Rhs)`. The call
/// releases both, so a block that releases them again releases values their new
/// owner has already dropped. The move analysis reads the syntax, and the
/// syntax of an operator says nothing about that; the impl table does.
impl BodyTranslator<'_> {
    /// `(the left operand is moved, the right operand is moved)`.
    ///
    /// Both are false where the operator is JavaScript's own: a number, a
    /// string and a boolean have no drop glue, and there is no call to move
    /// anything into.
    pub(crate) fn operator_takes(&self, bin: &syn::ExprBinary) -> (bool, bool) {
        let Some(op) = operator_of(&bin.op) else {
            return (false, false);
        };
        let Some(tc) = &self.types else {
            return (false, false);
        };
        // Asking is not translating: this runs during the move scan, before the
        // operator is written, and what it cannot resolve is reported there.
        let lhs = self.quietly(|| self.resolve_expr_type(&bin.left)).ok();
        let rhs = self.quietly(|| {
            self.expecting(&bin.right, lhs.as_ref(), || self.resolve_expr_type(&bin.right))
        })
        .ok();
        // This runs while the block's parameters are being claimed, which is
        // before the block's own `let`s have been translated — so the right
        // operand is often a local the type context has not met yet. Rust's
        // operator traits default `Rhs` to `Self`, and every operator impl in
        // the corpus takes that default, so the left operand's type stands in
        // for it. Where that guess finds no impl nothing is marked, and where
        // it finds the wrong one the answer is "moved", which the memo's rule
        // already prefers: a value moved and dropped anyway corrupts the
        // program, and a value kept and not dropped is a leak the registry
        // reports.
        let Some(lhs) = lhs else {
            return (false, false);
        };
        let rhs = rhs.unwrap_or_else(|| lhs.clone());
        if !matches!(
            (self.operand_kind(&lhs), self.operand_kind(&rhs)),
            (Operand::Object, _) | (_, Operand::Object)
        ) {
            return (false, false);
        }
        // A reference is never moved out of, whatever the impl does with the
        // value behind it: `&a + &b` leaves both where they were.
        let left_is_ref = matches!(lhs, Ty::Ref { .. });
        let right_is_ref = matches!(rhs, Ty::Ref { .. });
        let tc = tc.borrow();
        let Ok(found) = tc.probe().operator_impl(&op.trait_path, lhs.peel_refs(), rhs.peel_refs())
        else {
            return (false, false);
        };
        let def = tc.registry.impl_def(found.impl_id);
        let Some(method) = def.methods.get(op.rust_method) else {
            return (false, false);
        };
        let takes_self = matches!(method.self_kind, Some(crate::types::SelfKind::Value));
        let takes_rhs = method
            .params
            .first()
            .is_some_and(|(_, ty)| !matches!(ty, Ty::Ref { .. }));
        (takes_self && !left_is_ref, takes_rhs && !right_is_ref)
    }
}

/// Rust's `^`, `&` and `|` on two booleans, which JavaScript spells otherwise.
impl BodyTranslator<'_> {
    /// For: `a ^ b` between booleans is "they differ", and JavaScript's `^`
    /// converts both to numbers and hands back `0` or `1` — a value that is
    /// neither `true` nor `false` and compares equal to neither.
    ///
    /// Rust evaluates both operands of `&` and `|`; `&&` and `||` do not. The
    /// two agree in value, and differ where the right operand does something on
    /// its own, so a right operand that is not a place is reported.
    fn boolean_operator(
        &self,
        op: &Operator,
        lhs: &Ty,
        rhs: &Ty,
        left: &str,
        right: &str,
        right_expr: &syn::Expr,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let both_bool = matches!(lhs.peel_refs(), Ty::Prim(Prim::Bool))
            && matches!(rhs.peel_refs(), Ty::Prim(Prim::Bool));
        if !both_bool {
            return None;
        }
        let written = match op.native {
            "^" => format!("{} !== {}", left, right),
            "&" => format!("{} && {}", left, right),
            "|" => format!("{} || {}", left, right),
            _ => return None,
        };
        if matches!(op.native, "&" | "|") && !crate::body::is_place(right_expr) {
            self.fallback(
                span,
                format!(
                    "`{}` between booleans evaluates both sides in Rust, and the port writes it \
                     as `{}`, which does not evaluate the right one when the left has already \
                     decided; what the right side does on its own does not happen",
                    op.native,
                    if op.native == "&" { "&&" } else { "||" }
                ),
            );
        }
        Some(written)
    }
}
