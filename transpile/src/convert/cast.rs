//! `x as T` between two numeric types.
//!
//! For: Rust's `as` is a value conversion — it truncates, wraps and changes
//! representation — and TypeScript's `as` is a type assertion that changes
//! nothing at all. Emitting one for the other wrote `n as bigint` where a
//! `number` stood, which is both the wrong value and a type error.
//!
//! Two representations are in play, because the port writes `u64` and `i64` as
//! `bigint` and every other integer and both floats as `number`. Crossing
//! between them is what JavaScript refuses to do implicitly, and it is the
//! crossing this file is mostly about.
//!
//! Widths are the wasm32 ones: `usize` and `isize` are 32 bits (spec 1a).

use crate::ty::Prim;

/// How the port represents a primitive.
#[derive(PartialEq, Clone, Copy)]
enum Repr {
    Number,
    BigInt,
    Other,
}

fn repr(prim: Prim) -> Repr {
    match prim {
        Prim::U64 | Prim::I64 | Prim::U128 | Prim::I128 => Repr::BigInt,
        Prim::U8
        | Prim::U16
        | Prim::U32
        | Prim::Usize
        | Prim::I8
        | Prim::I16
        | Prim::I32
        | Prim::Isize
        | Prim::F32
        | Prim::F64 => Repr::Number,
        Prim::Bool | Prim::Char => Repr::Other,
    }
}

/// The width in bits, and whether the type is signed. `None` for the floats and
/// for the two that are not numbers.
fn width(prim: Prim) -> Option<(u32, bool)> {
    Some(match prim {
        Prim::U8 => (8, false),
        Prim::U16 => (16, false),
        Prim::U32 | Prim::Usize => (32, false),
        Prim::U64 => (64, false),
        Prim::U128 => (128, false),
        Prim::I8 => (8, true),
        Prim::I16 => (16, true),
        Prim::I32 | Prim::Isize => (32, true),
        Prim::I64 => (64, true),
        Prim::I128 => (128, true),
        _ => return None,
    })
}

fn is_float(prim: Prim) -> bool {
    matches!(prim, Prim::F32 | Prim::F64)
}

/// Does every value of `from` fit in `to`, so that the cast keeps the value it
/// was given and nothing has to be written around it?
///
/// Signed to unsigned never fits, however narrow the source: `-1i8 as u32` is
/// 4294967295, and the whole point of the wrap is to produce it.
fn fits(from: Prim, to: Prim) -> bool {
    let (Some((from_bits, from_signed)), Some((to_bits, to_signed))) = (width(from), width(to))
    else {
        return false;
    };
    match (from_signed, to_signed) {
        (false, false) | (true, true) => from_bits <= to_bits,
        (false, true) => from_bits < to_bits,
        (true, false) => false,
    }
}

/// What the port writes for `value as to`, where `value` holds a `from`.
///
/// `None` means this pair is not one the port knows how to write, and the
/// caller reports it. `char` is the pair that reaches it: the port writes a
/// `char` as a string, and turning one into a number is a decision about text
/// encoding rather than about arithmetic.
pub fn numeric(from: Prim, to: Prim, value: &str) -> Option<String> {
    if from == to {
        return Some(value.to_string());
    }
    match (repr(from), repr(to)) {
        (Repr::Number, Repr::Number) => Some(narrow_number(from, to, value)),
        (Repr::Number, Repr::BigInt) => {
            // A float has to lose its fraction before it can be a `BigInt` at
            // all: `BigInt(1.5)` throws.
            let integral = if is_float(from) {
                format!("Math.trunc({})", value)
            } else {
                value.to_string()
            };
            let widened = format!("BigInt({})", integral);
            Some(if !is_float(from) && fits(from, to) {
                widened
            } else {
                wrap_bigint(to, &widened)
            })
        }
        (Repr::BigInt, Repr::Number) => {
            let (bits, signed) = width(to)?;
            if is_float(to) {
                return Some(format!("Number({})", value));
            }
            Some(format!(
                "Number(BigInt.as{}N({}, {}))",
                if signed { "Int" } else { "Uint" },
                bits,
                value
            ))
        }
        (Repr::BigInt, Repr::BigInt) if fits(from, to) => Some(value.to_string()),
        (Repr::BigInt, Repr::BigInt) => Some(wrap_bigint(to, value)),
        // `true as u8` is 1 in Rust and `Number(true)` in the port.
        _ if from == Prim::Bool => Some(match repr(to) {
            Repr::BigInt => format!("BigInt({})", value),
            _ => format!("Number({})", value),
        }),
        _ => None,
    }
}

/// `value as to` where both are `number`: Rust truncates towards zero and then
/// keeps the low bits of the target's width.
fn narrow_number(from: Prim, to: Prim, value: &str) -> String {
    if is_float(to) {
        // Every `number` is a double, so widening to `f64` is the value; `f32`
        // rounds to single precision, which is what `Math.fround` is for.
        return match to {
            Prim::F32 => format!("Math.fround({})", value),
            _ => value.to_string(),
        };
    }
    // A float becomes an integer by truncation before the width is applied;
    // an integer that already fits keeps every bit it has.
    let truncated = if is_float(from) {
        format!("Math.trunc({})", value)
    } else if fits(from, to) {
        return value.to_string();
    } else {
        value.to_string()
    };
    wrap(to, &truncated)
}

/// A value brought back inside a type's range, the way Rust's `as` and its
/// wrapping arithmetic leave it.
///
/// `!bits` on a `u32` is 4294967295 in Rust and -1 in JavaScript, because
/// JavaScript's bitwise operators produce a *signed* 32-bit number whatever
/// they were given.
pub fn wrap(to: Prim, value: &str) -> String {
    if repr(to) == Repr::BigInt {
        return wrap_bigint(to, value);
    }
    let Some((bits, signed)) = width(to) else {
        return value.to_string();
    };
    match (bits, signed) {
        (32, false) => format!("({} >>> 0)", value),
        (32, true) => format!("({} | 0)", value),
        (_, false) => format!("({} & 0x{:x})", value, (1u64 << bits) - 1),
        // JavaScript's shifts work on 32 bits, so a signed narrowing sign-extends
        // by shifting the sign bit up to bit 31 and back down again.
        (_, true) => format!("(({} << {}) >> {})", value, 32 - bits, 32 - bits),
    }
}

/// Keep the low bits of a 64- or 128-bit target, the way Rust's `as` does.
fn wrap_bigint(to: Prim, value: &str) -> String {
    let Some((bits, signed)) = width(to) else {
        return value.to_string();
    };
    format!(
        "BigInt.as{}N({}, {})",
        if signed { "Int" } else { "Uint" },
        bits,
        value
    )
}
