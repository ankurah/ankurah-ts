//! `Vec<u8>` and `&[u8]`, which the port writes as a `Uint8Array`.
//!
//! A typed array is fixed-length: it has no `push`, no `pop`, no `splice`, and
//! its `length` cannot be assigned. Routing byte buffers through the ordinary
//! array translations produced calls that do not exist at runtime, silently.
//! The read-only half of `Vec<u8>` translates cleanly and is here; every call
//! that would grow or shrink the buffer is refused and reported, so the count
//! says how much of the corpus needs a growable byte type before one is chosen.

use super::MethodTranslation;

/// Methods that change a buffer's length. A `Uint8Array` cannot do any of them.
const MUTATING: [&str; 9] = [
    "push", "pop", "insert", "remove", "truncate", "clear", "resize", "drain", "retain",
];

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    if MUTATING.contains(&method) {
        return MethodTranslation::Refused {
            message: format!(
                "`{}` grows or shrinks a byte buffer, which a Uint8Array cannot do",
                method
            ),
            // What the array translations would have written. It does not run,
            // and the diagnostic above says so; keeping it means the shape of
            // the output does not change until a growable byte type is chosen.
            fallback: Box::new(super::array::translate(
                receiver,
                method,
                args,
                &super::array::Element::unknown(),
                super::iterator::Elements::Borrowed,
            )),
        };
    }

    let result = match method {
        "len" => format!("{}.length", receiver),
        "is_empty" => format!("{}.length === 0", receiver),

        // Reading the whole buffer, or a copy of it.
        "to_vec" | "to_owned" => format!("{}.slice()", receiver),
        "as_slice" | "as_bytes" | "as_ref" | "as_mut_slice" => receiver.to_string(),
        "iter" | "into_iter" => format!("[...{}]", receiver),
        // A byte is a number here, so a loop variable bound to one is a copy
        // and `*b = ..` through it writes nothing (F4/E12).
        "iter_mut" if args.is_empty() => {
            let message = "`iter_mut` on a byte buffer hands out `&mut u8`, and a byte is a \
                           JavaScript number: the loop would bind a COPY and every write \
                           through it would be lost"
                .to_string();
            return MethodTranslation::Refused {
                fallback: Box::new(MethodTranslation::Expr(crate::body::hole_text(&message))),
                message,
            };
        }

        // Reading one byte, or a run of them.
        // J1: `first`, `last` and `get` answer an `Option`, whose JavaScript
        // sentinel is `undefined`. The shared Option-adaptor table writes them.
        "contains" if args.len() == 1 => format!("{}.includes({})", receiver, args[0]),
        "starts_with" if args.len() == 1 => format!(
            "{}.slice(0, {}.length).every((b, i) => b === {}[i])",
            receiver, args[0], args[0]
        ),

        // Growing by copying: the only shape a fixed-length buffer allows, and
        // it only works where the receiver is somewhere that can be assigned to.
        "extend_from_slice" | "extend" if args.len() == 1 => format!(
            "{} = new Uint8Array([...{}, ...{}])",
            receiver, receiver, args[0]
        ),

        // Everything else a byte slice shares with any other sequence — the
        // Option-returning readers among them — is written by the shared table.
        _ => match super::iterator::translate(
            receiver,
            method,
            args,
            super::iterator::Receiver::Sequence,
            super::iterator::Elements::Borrowed,
        ) {
            Some(written) => written,
            None => return MethodTranslation::Passthrough,
        },
    };
    MethodTranslation::Expr(result)
}
