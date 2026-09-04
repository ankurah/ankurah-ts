//! Arithmetic and comparison on primitives, where JavaScript's operator does
//! not mean what Rust's does.
//!
//! Three differences matter. Integer division in Rust truncates and in
//! JavaScript leaves a fraction. `u64` and `i64` are `bigint` and every other
//! number is a `number`, and JavaScript refuses to mix them — `1n + 1` throws —
//! so an operator whose operands land on different sides of that line is a
//! run-time error rather than a wrong answer. And an arithmetic result that
//! leaves its type's range wraps in Rust's release profile and grows without
//! bound in JavaScript; that last one is a known gap, recorded in spec 7a and
//! not written here.

use crate::body::BodyTranslator;
use crate::ty::Prim;

use super::Operator;

/// Is this primitive written as a `bigint`?
fn is_bigint(prim: Prim) -> bool {
    matches!(prim, Prim::U64 | Prim::I64 | Prim::U128 | Prim::I128)
}

impl BodyTranslator<'_> {
    /// The operator between two primitives.
    ///
    /// `None` means the JavaScript operator stands as written, which is the
    /// answer for most of them.
    pub(super) fn primitive_operator(
        &self,
        op: &Operator,
        left_ty: Prim,
        right_ty: Prim,
        left: &str,
        right: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        // A shift's right operand has a type of its own and is never required
        // to be the same side of the bigint line as the left one; every other
        // operator takes two of one type.
        let shift = matches!(op.native, "<<" | ">>" | "<<=" | ">>=");
        // A shift's right operand is a `u32` in Rust and has to be a `bigint`
        // in JavaScript whenever the left one is: `1n << 63` throws.
        if shift && is_bigint(left_ty) && !is_bigint(right_ty) {
            return Some(format!("{} {} BigInt({})", left, op.native, right));
        }
        if !shift && is_bigint(left_ty) != is_bigint(right_ty) {
            self.fallback(
                span,
                format!(
                    "`{}` mixes a `{}` with a `{}`, which the port writes as a bigint and a \
                     number; JavaScript refuses to mix the two and throws at the operator",
                    op.native,
                    format!("{:?}", left_ty).to_lowercase(),
                    format!("{:?}", right_ty).to_lowercase(),
                ),
            );
            return None;
        }
        // Rust's integer division truncates towards zero. JavaScript's `/` on
        // two numbers is real division, so `7 / 2` is 3.5 where Rust says 3. A
        // bigint division already truncates.
        if left_ty.is_integer() && !is_bigint(left_ty) {
            if op.native == "/" {
                return Some(format!("Math.trunc({} / {})", left, right));
            }
            if op.native == "/=" {
                return Some(format!("{} = Math.trunc({} / {})", left, left, right));
            }
        }
        None
    }
}
