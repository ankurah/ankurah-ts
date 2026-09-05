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
#[derive(Clone)]
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
            _ => self.overloaded_operator(&op, &lhs, &rhs, left, right, span),
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
            crate::emit::impl_method_name(op.trait_name, op.rust_method, &op.ts_method, &args, "");
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
        // operand is often a local the type context has not met yet. Guessing
        // `Rhs = Self` found no impl at all for `impl Add<Right> for Left`, so
        // nothing marked `left` moved, `add` consumed it, and the block
        // released it again. Where the right operand is unknown the impl is
        // looked up by the LEFT one alone, which answers whenever the trait has
        // one impl for that self type — and which side it takes on the right
        // cannot then change the answer.
        let Some(lhs) = lhs else {
            return (false, false);
        };
        if !matches!(
            (
                self.operand_kind(&lhs),
                rhs.as_ref().map(|ty| self.operand_kind(ty)).unwrap_or(Operand::Object)
            ),
            (Operand::Object, _) | (_, Operand::Object)
        ) {
            return (false, false);
        }
        // A reference is never moved out of, whatever the impl does with the
        // value behind it: `&a + &b` leaves both where they were.
        let left_is_ref = matches!(lhs, Ty::Ref { .. });
        let right_is_ref = rhs.as_ref().is_some_and(|ty| matches!(ty, Ty::Ref { .. }));
        let tc = tc.borrow();
        let candidates: Vec<crate::registry::Conversion> = match &rhs {
            Some(rhs) => tc.probe().operator_impl(&op.trait_path, &lhs, rhs).ok().into_iter().collect(),
            None => tc.probe().operator_impls_by_self(&op.trait_path, &lhs),
        };
        // Every candidate has to agree, or the answer is not known. Where the
        // right operand is a local the scan has not met, the left type may
        // carry several impls of the trait — `impl Add for Weight` beside
        // `impl Add<Right> for Weight` — and they agree about the operands in
        // every corpus shape, because Rust's operator traits take both by
        // value. Where they do not agree the site says so.
        let dispositions: Vec<(bool, bool)> = candidates
            .iter()
            .filter_map(|found| {
                let def = tc.registry.impl_def(found.impl_id);
                let method = def.methods.get(op.rust_method)?;
                let takes_self = matches!(method.self_kind, Some(crate::types::SelfKind::Value));
                let takes_rhs = method
                    .params
                    .first()
                    .is_some_and(|(_, ty)| !matches!(ty, Ty::Ref { .. }));
                Some((takes_self, takes_rhs))
            })
            .collect();
        let Some(first) = dispositions.first().copied() else {
            return (false, false);
        };
        if dispositions.iter().any(|d| *d != first) {
            drop(tc);
            self.fallback(
                syn::spanned::Spanned::span(bin),
                format!(
                    "`{}` here resolves to one of several `{}` impls written for the left \
                     operand, and they do not agree about which operands they consume, so \
                     whether the block still owns them is not decided; both are treated as \
                     moved, which leaks rather than releasing twice",
                    op.native, op.trait_name
                ),
            );
            return (!left_is_ref, !right_is_ref);
        }
        let (takes_self, takes_rhs) = first;
        (takes_self && !left_is_ref, takes_rhs && !right_is_ref)
    }
}

/// Rust's `^`, `&` and `|` on two booleans, which JavaScript spells otherwise.
impl BodyTranslator<'_> {
    /// For: `a ^ b` between booleans is "they differ", and JavaScript's `^`
    /// converts both to numbers and hands back `0` or `1` — a value that is
    /// neither `true` nor `false` and compares equal to neither.
    ///
    /// Rust evaluates both operands of `&` and `|`; `&&` and `||` do not. They
    /// agree in value and differ in what runs, so a right operand that logs,
    /// mutates or advances an iterator happened in Rust and did not happen
    /// here. A call evaluates both of its arguments, left to right, exactly
    /// once — so the two operators are calls on the runtime's `boolAnd` and
    /// `boolOr`, and there is nothing left to report. `^` is already eager and
    /// already a boolean, so it stays an expression.
    fn boolean_operator(
        &self,
        op: &Operator,
        lhs: &Ty,
        rhs: &Ty,
        left: &str,
        right: &str,
        _right_expr: &syn::Expr,
        _span: proc_macro2::Span,
    ) -> Option<String> {
        let both_bool = matches!(lhs.peel_refs(), Ty::Prim(Prim::Bool))
            && matches!(rhs.peel_refs(), Ty::Prim(Prim::Bool));
        if !both_bool {
            return None;
        }
        match op.native {
            "^" => Some(format!("{} !== {}", left, right)),
            "&" => Some(format!("boolAnd({}, {})", left, right)),
            "|" => Some(format!("boolOr({}, {})", left, right)),
            _ => None,
        }
    }
}

/// The unary and indexing operators, which resolve through an impl exactly as
/// the binary ones do.
///
/// For: `-a`, `!a`, `a[i]` and `*a` are method calls in Rust whenever the
/// operand is not a primitive — `impl Neg for Weight` is `Weight::neg` — and
/// the port wrote the JavaScript operator instead. `-object` is `NaN`,
/// `object[0]` is `undefined`, and neither said anything.
impl BodyTranslator<'_> {
    /// `-a` where `a` is not a number: the `Neg` impl's method.
    pub(crate) fn unary_neg(&self, unary: &syn::ExprUnary, written: &str) -> Option<String> {
        let ty = self.quietly(|| self.resolve_expr_type(&unary.expr)).ok()?;
        if matches!(self.operand_kind(&ty), Operand::Number(_)) {
            return None;
        }
        self.unary_through("std::ops::Neg", "Neg", "neg", &ty, written, "-", syn::spanned::Spanned::span(unary))
    }

    /// `!a` where `a` is neither a boolean nor an integer: the `Not` impl's.
    pub(crate) fn unary_not_impl(&self, unary: &syn::ExprUnary, written: &str) -> Option<String> {
        let ty = self.quietly(|| self.resolve_expr_type(&unary.expr)).ok()?;
        self.unary_through("std::ops::Not", "Not", "not", &ty, written, "!", syn::spanned::Spanned::span(unary))
    }

    /// `a[i]` where `a` is not a JavaScript sequence: the `Index` impl's.
    pub(crate) fn index_through_impl(&self, base: &syn::Expr, base_ts: &str, index: &str) -> Option<String> {
        let ty = self.quietly(|| self.resolve_expr_type(base)).ok()?;
        // An array, a `Uint8Array`, a `Map` and a string are indexed the way
        // JavaScript indexes them; the impl table is for everything else.
        use crate::name_map::shape::{js_shape, JsShape};
        let tc = self.types.as_ref()?;
        let shape = js_shape(tc.borrow().registry, ty.peel_refs());
        if !matches!(shape, JsShape::Plain) {
            return None;
        }
        let found = {
            let tc = tc.borrow();
            tc.probe().operator_impls_by_self("std::ops::Index", ty.peel_refs())
        };
        let Some(found) = found.first() else {
            self.fallback(
                syn::spanned::Spanned::span(base),
                format!(
                    "`[..]` on `{}` resolves through `Index`, and no impl in the table performs \
                     it; the JavaScript index is written, which reads a property that is not there",
                    self.describe(&ty)
                ),
            );
            return None;
        };
        let member = {
            let tc = tc.borrow();
            let def = tc.registry.impl_def(found.impl_id);
            let args: Vec<String> = def
                .trait_ref
                .as_ref()
                .map(|t| t.args.iter().map(|ty| crate::name_map::map_ty(tc.registry, ty)).collect())
                .unwrap_or_default();
            crate::emit::impl_method_name("Index", "index", "index", &args, "")
        };
        Some(format!("{}.{}({})", base_ts, member, index))
    }

    /// One unary operator through its trait, or the reason there is none.
    #[allow(clippy::too_many_arguments)]
    fn unary_through(
        &self,
        trait_path: &str,
        trait_name: &str,
        rust_method: &str,
        ty: &Ty,
        written: &str,
        native: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        if !matches!(self.operand_kind(ty), Operand::Object) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let found = {
            let tc = tc.borrow();
            tc.probe().operator_impls_by_self(trait_path, ty.peel_refs())
        };
        let Some(found) = found.first() else {
            self.fallback(
                span,
                format!(
                    "`{}` on `{}` resolves through `{}`, and no impl in the table performs it; \
                     the JavaScript operator is written, which does something else entirely",
                    native,
                    self.describe(ty),
                    trait_name
                ),
            );
            return None;
        };
        let tc = tc.borrow();
        let def = tc.registry.impl_def(found.impl_id);
        if !crate::emit_impls::has_emitted_class(tc.registry, &def.self_ty) {
            drop(tc);
            self.fallback(
                span,
                format!(
                    "`{}` on `{}` resolves through `{}`, and the operand has no class of its own \
                     for the method to be on",
                    native,
                    self.describe(ty),
                    trait_name
                ),
            );
            return None;
        }
        let member = crate::name_map::to_camel_case(rust_method);
        Some(format!("{}.{}()", written, member))
    }

    /// How a diagnostic names a type.
    fn describe(&self, ty: &Ty) -> String {
        match &self.types {
            Some(tc) => tc.borrow().registry.describe(ty),
            None => "the operand".to_string(),
        }
    }
}

/// Which unary operators take their operand away from the block that held it.
impl BodyTranslator<'_> {
    /// Does the impl behind `-a`, `!a` or `*a` consume `a`?
    ///
    /// Rust's `Neg`, `Not` and the assignment-free unary traits take `self` by
    /// value, so the call releases the operand and the block must not release
    /// it again. `Deref` takes `&self` and consumes nothing.
    pub(crate) fn unary_takes(&self, unary: &syn::ExprUnary) -> bool {
        let trait_path = match unary.op {
            syn::UnOp::Neg(_) => "std::ops::Neg",
            syn::UnOp::Not(_) => "std::ops::Not",
            _ => return false,
        };
        let Some(tc) = &self.types else { return false };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(&unary.expr)) else {
            return false;
        };
        // A reference is never moved out of, and a primitive has no impl.
        if matches!(ty, Ty::Ref { .. }) || !matches!(self.operand_kind(&ty), Operand::Object) {
            return false;
        }
        let tc = tc.borrow();
        let found = tc.probe().operator_impls_by_self(trait_path, ty.peel_refs());
        let Some(found) = found.first() else { return false };
        let def = tc.registry.impl_def(found.impl_id);
        let method = match unary.op {
            syn::UnOp::Neg(_) => def.methods.get("neg"),
            _ => def.methods.get("not"),
        };
        method.is_some_and(|m| matches!(m.self_kind, Some(crate::types::SelfKind::Value)))
    }
}
