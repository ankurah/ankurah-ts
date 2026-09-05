//! Option<T> → T | null method translations
//!
//! Option maps to nullable (T | null) in TS, so "methods" on Option
//! become syntax-level operations, not method calls.

use super::MethodTranslation;

pub fn translate(receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let result = match method {
        // unwrap/expect/unwrap_or/unwrap_or_else handled in body.rs before dispatch.

        // Null checks
        "is_some" => format!("{} != null", receiver),
        "is_none" => format!("{} == null", receiver),

        // Map — apply function if non-null
        "map" if args.len() == 1 => format!("{} != null ? ({})({}!) : null", receiver, args[0], receiver),

        // A JavaScript value is neither borrowed nor owned, so the four
        // `as_` conversions between those states are the value itself. Written
        // through, they named methods no value has: 39 `asRef` and 11 `asMut`
        // calls across the emitted corpus, each a `TypeError`.
        "as_ref" | "as_mut" | "as_deref" | "as_deref_mut" if args.is_empty() => receiver.to_string(),

        // The combinators, each written as the test it is. The receiver is read
        // twice, as `map` above already reads it: what stands here is a place,
        // and a call with an effect in this position would be read twice by
        // `map` too.
        "and_then" if args.len() == 1 => {
            format!("{} != null ? ({})({}!) : null", receiver, args[0], receiver)
        }
        "filter" if args.len() == 1 => {
            format!("{} != null && ({})({}!) ? {} : null", receiver, args[0], receiver, receiver)
        }
        "is_some_and" if args.len() == 1 => {
            format!("{} != null && ({})({}!)", receiver, args[0], receiver)
        }
        "ok_or" if args.len() == 1 => {
            format!("{} != null ? Result.Ok({}!) : Result.Err({})", receiver, receiver, args[0])
        }
        "ok_or_else" if args.len() == 1 => {
            format!("{} != null ? Result.Ok({}!) : Result.Err(({})())", receiver, receiver, args[0])
        }
        "map_or" if args.len() == 2 => {
            format!("{} != null ? ({})({}!) : {}", receiver, args[1], receiver, args[0])
        }
        "map_or_else" if args.len() == 2 => {
            format!("{} != null ? ({})({}!) : ({})()", receiver, args[1], receiver, args[0])
        }

        // `Option<&T>::cloned` and `::copied` turn a borrow of the payload into
        // an owned one. A JavaScript value is neither, so the nullable is
        // already what they produce. (What `cloned` does to the payload itself
        // — an `Arc` refcount, say — is the same question `iter().cloned()`
        // raises and is answered the same way: it does not.)
        "cloned" | "copied" if args.is_empty() => receiver.to_string(),

        _ => return MethodTranslation::Passthrough,
    };
    MethodTranslation::Expr(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(receiver: &str, method: &str, args: &[&str]) -> String {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        match translate(receiver, method, &args) {
            MethodTranslation::Expr(ts) => ts,
            _ => panic!("`{method}` has no expression translation"),
        }
    }

    /// A JavaScript value is neither borrowed nor owned, so the conversions
    /// between those states are the value. Written through, `as_ref` named a
    /// method no value has: 39 `asRef` and 11 `asMut` calls across the emitted
    /// corpus, each a `TypeError` the moment it ran.
    #[test]
    fn the_borrow_conversions_are_the_value() {
        for method in ["as_ref", "as_mut", "as_deref", "as_deref_mut"] {
            assert_eq!(expr("slot", method, &[]), "slot");
        }
    }

    /// Each combinator is the test it stands for. `Option<T>` is `T | null`
    /// here, so "there is something" is `!= null` and "there is not" is `null`.
    #[test]
    fn the_combinators_are_written_as_the_tests_they_are() {
        assert_eq!(expr("slot", "and_then", &["f"]), "slot != null ? (f)(slot!) : null");
        assert_eq!(expr("slot", "filter", &["p"]), "slot != null && (p)(slot!) ? slot : null");
        assert_eq!(expr("slot", "is_some_and", &["p"]), "slot != null && (p)(slot!)");
        assert_eq!(
            expr("slot", "ok_or", &["e"]),
            "slot != null ? Result.Ok(slot!) : Result.Err(e)"
        );
        assert_eq!(
            expr("slot", "ok_or_else", &["mk"]),
            "slot != null ? Result.Ok(slot!) : Result.Err((mk)())"
        );
        assert_eq!(expr("slot", "map_or", &["0", "f"]), "slot != null ? (f)(slot!) : 0");
        assert_eq!(
            expr("slot", "map_or_else", &["mk", "f"]),
            "slot != null ? (f)(slot!) : (mk)()"
        );
    }

    /// A method the port has no translation for is passed through rather than
    /// guessed at: `take` and `replace` write to the place the option is in,
    /// and this table only knows how to read one.
    #[test]
    fn a_method_with_no_translation_is_passed_through() {
        assert!(matches!(translate("slot", "take", &[]), MethodTranslation::Passthrough));
        assert!(matches!(
            translate("slot", "replace", &["x".to_string()]),
            MethodTranslation::Passthrough
        ));
    }
}
