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
        // A shift whose left operand is a bigint. Two things have to be
        // written: the count, which JavaScript needs as a bigint too (`1n << 63`
        // throws), and the result of a `<<`, which grows without bound where
        // Rust keeps the low bits of the type — `u64::MAX << 1` is
        // `0x1fffffffffffffffe` in JavaScript and `0xfffffffffffffffe` in Rust.
        // `&`, `|` and `^` between two values already inside the range answer
        // inside it, so only the shift is written around.
        if shift && is_bigint(left_ty) {
            let plain = compound_of(op.native).unwrap_or(op.native);
            let shifted = format!("({} {} {})", left, plain, bigint_count(right, right_ty));
            let value = if plain == "<<" {
                crate::convert::cast::wrap(left_ty, &shifted)
            } else {
                shifted
            };
            return Some(if op.native.ends_with('=') {
                self.place_assignment(left, &value, span)
            } else {
                value
            });
        }
        // R7: `+`, `-` and `*` on a fixed-width integer PANIC on overflow, as
        // the `debug_assertions = true` build this port mirrors does. JavaScript
        // wraps nothing and saturates nothing — it goes on counting in doubles,
        // silently losing precision above 2^53 — so a bare `a + b` was a third
        // answer, neither Rust's release wrap nor Rust's debug panic. Division
        // and remainder go through the same helpers, which panic on a zero
        // divisor as Rust does and truncate towards zero as Rust does.
        if left_ty.is_integer() {
            if let Some(name) = checked_helper(op.native) {
                let width = width_name(left_ty);
                // The helper is skipped only where the ANSWER is provably in
                // range, not where the operands are: `255 + 1` on a `u8` has
                // two operands that fit and a result that does not, and Rust
                // panics on it. Two literals are computed; a pair of array
                // lengths cannot overflow a 64-bit type, because a length is
                // below 2^32.
                if !answer_fits(op.native, left, right, left_ty) {
                    let call = format!("{}({}, {}, '{}')", name, left, right, width);
                    return Some(if op.native.ends_with('=') {
                        self.place_assignment(left, &call, span)
                    } else {
                        call
                    });
                }
            }
        }
        // Rust's integer division truncates towards zero. JavaScript's `/` on
        // two numbers is real division, so `7 / 2` is 3.5 where Rust says 3. A
        // bigint division already truncates.
        if left_ty.is_integer() && !is_bigint(left_ty) {
            if op.native == "/" {
                return Some(format!("Math.trunc({} / {})", left, right));
            }
            if op.native == "/=" {
                return Some(self.place_assignment(left, &format!("Math.trunc({} / {})", left, right), span));
            }
            // A shift by a literal at or past the left type's width is proof
            // the type is a guess and the guess is wrong: Rust rejects
            // `1u32 << 63` outright, so a source that compiles means a wider
            // type than the one the engine settled on. `core/src/collation.rs`
            // writes `f.to_bits() ^ (1 << 63)` where `f` comes out of ankql's
            // hardcoded AST and cannot be typed, and the engine's `i32` fallback
            // wrapped the shift into 32 bits and wrote a confident
            // `-2147483648`.
            if shift && !self.count_fits(left_ty, right, span) {
                return None;
            }
            if let Some(written) = bitwise(op, left_ty, left, right) {
                return Some(written);
            }
            // A compound bit operation is the operation and then the
            // assignment, and its result needs the same wrapping the
            // expression form gets: `value <<= 31` on a `u32` answered
            // -2147483648, and `value <<= 7` on a `u8` answered 256.
            if let Some(plain) = compound_of(op.native) {
                let expression = Operator { native: plain, ..op.clone() };
                if let Some(written) = bitwise(&expression, left_ty, left, right) {
                    return Some(self.place_assignment(left, &written, span));
                }
            }
        }
        None
    }

    /// Does a literal shift count fit inside the left operand's guessed width?
    /// A count that does not is reported, and nothing is written around the
    /// shift: wrapping it to a width the source disproves is a wrong answer
    /// stated confidently.
    fn count_fits(&self, left_ty: Prim, right: &str, span: proc_macro2::Span) -> bool {
        let text = right.trim().trim_start_matches('(').trim_end_matches(')');
        let Ok(count) = text.parse::<u32>() else { return true };
        let Some((bits, _)) = crate::convert::cast::width(left_ty) else { return true };
        if count < bits {
            return true;
        }
        self.fallback(
            span,
            format!(
                "this shifts by {count}, and the left operand is typed `{ty}`, which Rust would \
                 reject — so the type is the engine's guess and the guess is wrong; the shift is \
                 written as it stands rather than wrapped into {bits} bits",
                count = count,
                ty = format!("{:?}", left_ty).to_lowercase(),
                bits = bits
            ),
        );
        false
    }

    /// `place = value`, where the place is read as well as written.
    ///
    /// Rust evaluates a compound assignment's place ONCE. This writes it twice,
    /// which is the same answer for a name or a field chain and not for a place
    /// whose own evaluation does something — so that case says so.
    fn place_assignment(&self, place: &str, value: &str, span: proc_macro2::Span) -> String {
        if place.contains('(') {
            self.fallback(
                span,
                format!(
                    "`{}` is a compound assignment whose place is evaluated twice here, and Rust \
                     evaluates it once; whatever evaluating it does happens twice",
                    place
                ),
            );
        }
        format!("{} = {}", place, value)
    }
}

/// The plain operator a compound assignment performs before it assigns.
fn compound_of(native: &str) -> Option<&'static str> {
    match native {
        "&=" => Some("&"),
        "|=" => Some("|"),
        "^=" => Some("^"),
        "<<=" => Some("<<"),
        ">>=" => Some(">>"),
        _ => None,
    }
}

/// A shift count beside a bigint, which JavaScript needs as a bigint too.
///
/// The written text decides for a literal, whatever the engine made of its
/// type: `(a << 31, b << 4, c << 40)` typed `40` as a `u64` and the literal
/// emitter wrote it as the `number` `40`, so the two disagreed and the shift
/// threw `Cannot mix BigInt and other types`.
fn bigint_count(right: &str, right_ty: Prim) -> String {
    let text = right.trim();
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}n", text);
    }
    if is_bigint(right_ty) { text.to_string() } else { format!("BigInt({})", text) }
}

/// The bit operators, which JavaScript performs on a *signed* 32-bit number
/// whatever it was given.
///
/// `x >> 1` on a `u32` above 2^31 is negative in JavaScript and positive in
/// Rust, because JavaScript's `>>` keeps the sign bit; `>>>` is the one that
/// does what Rust's `>>` on an unsigned type does. And a result outside the
/// type's range — `1u32 << 31`, `!0u16 & 0xFFFF` — has to come back inside it,
/// which is what `cast::wrap` writes.
fn bitwise(op: &Operator, left_ty: Prim, left: &str, right: &str) -> Option<String> {
    let unsigned = matches!(left_ty, Prim::U8 | Prim::U16 | Prim::U32 | Prim::Usize);
    let native = match (op.native, unsigned) {
        // Rust's `>>` on an unsigned type shifts zeroes in; JavaScript's `>>`
        // shifts the sign bit in.
        (">>", true) => ">>>",
        ("&" | "|" | "^" | "<<" | ">>", _) => op.native,
        _ => return None,
    };
    // The operation is parenthesised before it is wrapped: `a & b >>> 0` is
    // `a & (b >>> 0)` in JavaScript, which masks the wrong side.
    Some(crate::convert::cast::wrap(
        left_ty,
        &format!("({} {} {})", left, native, right),
    ))
}

/// The base helper an arithmetic operator goes through, or `None` for an
/// operator Rust cannot overflow (the bit operations, the comparisons).
pub(crate) fn checked_helper(native: &str) -> Option<&'static str> {
    Some(match native {
        "+" | "+=" => "checkedAdd",
        "-" | "-=" => "checkedSub",
        "*" | "*=" => "checkedMul",
        "/" | "/=" => "checkedDiv",
        "%" | "%=" => "checkedRem",
        _ => return None,
    })
}

/// The name the runtime knows this width by.
pub(crate) fn width_name(prim: Prim) -> String {
    prim.rust_name()
}

/// Is the ANSWER provably inside the type's range, so that the helper can be
/// left out and the emitted expression stay what a reader of the port expects?
///
/// Two decimal literals: the answer is computed. Two array LENGTHS added or
/// subtracted in a 64-bit type: a JavaScript length is a non-negative integer
/// below 2^32, so their sum is below 2^33 and cannot leave the range. Anything
/// else is checked at run time, because a value that came from somewhere else
/// can be anything — `255 + 1` on a `u8` has two operands that fit and an
/// answer that does not, and Rust panics on it.
fn answer_fits(native: &str, left: &str, right: &str, prim: Prim) -> bool {
    let Some((bits, signed)) = crate::convert::cast::width(prim) else {
        return false;
    };
    if bits > 64 {
        return false;
    }
    let (low, high): (i128, i128) = if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    if let (Some(a), Some(b)) = (literal(left), literal(right)) {
        let answer = match native.trim_end_matches('=') {
            "+" => a.checked_add(b),
            "-" => a.checked_sub(b),
            "*" => a.checked_mul(b),
            "/" if b != 0 => a.checked_div(b),
            "%" if b != 0 => a.checked_rem(b),
            _ => None,
        };
        return answer.is_some_and(|answer| answer >= low && answer <= high);
    }
    // A length is below 2^32; two of them added or subtracted stay inside any
    // 64-bit type, and inside `u64` only when nothing is subtracted.
    let lengths = is_length(left) && is_length(right);
    let widening = bits >= 64 && matches!(native.trim_end_matches('='), "+" | "-");
    lengths && widening && (signed || native.trim_end_matches('=') == "+")
}

fn literal(written: &str) -> Option<i128> {
    let text = written.trim().trim_start_matches('(').trim_end_matches(')');
    text.trim_end_matches('n').parse::<i128>().ok()
}

/// Is this written operand a JavaScript array or string length?
fn is_length(written: &str) -> bool {
    let text = written.trim().trim_start_matches('(').trim_end_matches(')');
    text.ends_with(".length") || text.ends_with(".size")
}
