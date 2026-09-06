//! Methods the body translator answers before the native-type dispatch.
//!
//! For: what a call is written as usually follows from the RECEIVER's type, and
//! `native_types` is the table that says so. Three questions do not. A method
//! call's arguments are written for the type the CALLEE declares each of them
//! to be. `collect()` builds whatever its target names, and only the position
//! the call stands in says what that is. `unwrap_or` on a value the port writes
//! as a nullable is `??`, which is a fact about the port's spelling of `Option`
//! and not about the receiver's class. Each is settled here, where the call
//! expression and the position are both in hand, and never reaches the table.

use super::BodyTranslator;

impl<'a> BodyTranslator<'a> {
    /// A method call's arguments, each translated for the type the callee
    /// declares it to be.
    ///
    /// The callee's signature is what says what each argument has to be: a
    /// closure takes its parameter types from there, an `.into()` its target,
    /// and a literal its width. A closure written in one of these positions may
    /// also be one whose call site the emitter WRITES — `invoke(f, v)` for an
    /// `Option` combinator, `invokeRef($p, x)` for a `retain` — and
    /// `Placement::Loose`'s report, which says the emitter cannot see the call
    /// site, is false at exactly those.
    pub(crate) fn method_arguments(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
    ) -> Vec<String> {
        let want = self.argument_types(call);
        let invoked = self.own.argument_is_invoked.replace(
            crate::native_types::nullable::invokes_a_closure_argument(rust_method),
        );
        let args = call
            .args
            .iter()
            .enumerate()
            .map(|(index, a)| {
                let wants = want.get(index).and_then(|t| t.as_ref());
                // C1: a cell handed to a parameter that is itself a `&mut` to a
                // value goes over as the CELL. Rust reborrows a `&mut`
                // implicitly — `f(buffer)` where `buffer: &mut String` passes
                // the reference — and `buffer.value` would hand the callee a
                // copy of the string, which is the defect this is all about.
                if self.names_a_cell_param(a) {
                    return Self::path_static(match a {
                        syn::Expr::Path(path) => &path.path,
                        _ => unreachable!("names_a_cell answered for a path"),
                    });
                }
                self.expecting(a, wants, || self.moved_value(a))
            })
            .collect();
        self.own.argument_is_invoked.set(invoked);
        args
    }

    /// `unwrap_or` and `unwrap_or_else` on a value the port writes as a
    /// nullable, which is what `??` reads.
    ///
    /// `??` reads a *value* for null, and a `Result` is an object: `r ?? d`
    /// always takes `r`, whatever it holds. Only the nullable the port maps
    /// `Option` to can be written that way; a `Result` calls the runtime's own
    /// method, which consumes the receiver as Rust's does.
    pub(crate) fn nullable_unwrap_or(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
        args: &[String],
    ) -> Option<String> {
        if !matches!(rust_method, "unwrap_or" | "unwrap_or_else") || args.len() != 1 {
            return None;
        }
        let nullable = self
            .resolve_expr_type(&call.receiver)
            .ok()
            .is_some_and(|ty| self.is_nullable(&ty));
        if !nullable {
            return None;
        }
        if rust_method == "unwrap_or" {
            // Rust evaluates the default before it knows whether it is wanted,
            // and drops it where it is not. `??` alone left it to the leak
            // registry.
            if let Some(chosen) = self.nullable_default(receiver, &call.args[0], &args[0]) {
                return Some(chosen);
            }
        }
        Some(match rust_method {
            "unwrap_or" => format!("{} ?? {}", receiver, args[0]),
            _ => format!("{} ?? ({})()", receiver, args[0]),
        })
    }

    /// What `collect()` builds, from the type the call's position gives it.
    ///
    /// Written as the identity it was right for a `Vec` and wrong for
    /// everything else: an `entity_map: HashSet<_>` came out an array and was
    /// then asked `has` and `add`, which an array does not have —
    /// `core/util/expand_states.ts:10`.
    ///
    /// Rust's `collect` is `FromIterator`: the TARGET decides what is built, and
    /// the port has a different construction for each — a `Vec` is the sequence
    /// itself, a `HashSet` and a `HashMap` are runtime containers built from it,
    /// a `String` is a join. Written as the identity, `let m: HashSet<_> = ..
    /// .collect()` handed back an array and every `contains`/`insert` on it read
    /// `undefined`.
    ///
    /// A target the engine cannot name, and one whose `FromIterator` the port
    /// has no construction for — `Result<Vec<_>, E>`, which short-circuits —
    /// is a hole (R12) rather than a guess.
    pub(crate) fn collected_into(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
    ) -> Option<String> {
        if rust_method != "collect" || !call.args.is_empty() {
            return None;
        }
        // The call resolved — this is `Iterator::collect` and nothing else —
        // so it is recorded before the port's own answer is written, the way
        // `unwrap` on a `LockResult` is.
        self.record_resolution(call, rust_method);
        // The POSITION is what names the target: `collect`'s own return type
        // is its `B`, the `FromIterator` parameter, and only the `let`'s
        // annotation or the function's return type says what `B` is. Asked of
        // the call alone, `let m: HashSet<_> = .. .collect()` answered `B`.
        let whole = syn::Expr::MethodCall(call.clone());
        // A turbofish says the target outright — `collect::<Vec<_>>()` — and it
        // is the only thing that does where the value is consumed on the spot.
        let written = call.turbofish.as_ref().and_then(|args| match args.args.first() {
            Some(syn::GenericArgument::Type(ty)) => self
                .types
                .as_ref()
                .and_then(|tc| self.quietly(|| tc.borrow().resolve_written_type(ty).ok())),
            _ => None,
        });
        let target = written
            .or_else(|| self.expectation_for(&whole))
            .or_else(|| self.quietly(|| self.resolve_expr_type(&whole)).ok());
        let Some(target) = target.filter(|ty| !matches!(ty, crate::ty::Ty::Param(_))) else {
            return Some(self.hole(
                syn::spanned::Spanned::span(call),
                "`collect` builds whatever its target type names, and the engine could not name \
                 the type this one is collected into",
            ));
        };
        let reg = self.registry()?;
        use crate::name_map::shape::{js_shape, JsShape};
        match js_shape(reg, &target) {
            // A `Vec`, an array or a slice IS the sequence.
            JsShape::Array(_) | JsShape::Bytes => Some(receiver.to_string()),
            JsShape::Set(_) => Some(format!("HashSet.from({})", receiver)),
            JsShape::Map(..) => Some(format!("HashMap.from({})", receiver)),
            JsShape::Str => Some(format!("{}.join('')", receiver)),
            _ => Some(self.hole(
                syn::spanned::Spanned::span(call),
                format!(
                    "`collect` into `{}` is a `FromIterator` the port has no construction for",
                    crate::name_map::map_ty(reg, &target)
                ),
            )),
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    const PRELUDE: &str = "\
use std::collections::{HashMap, HashSet};\n\
pub struct Item { pub id: u32, pub name: String }\n\
";

    fn body(rust: &str, method: &str) -> String {
        let mut f = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
        f.translated_method("lib.rs", method)
    }

    /// `collect` builds whatever its TARGET names, and the target comes from
    /// the position: the `let`'s annotation, the function's return type, or a
    /// turbofish. Written as the identity it handed back an array for every one
    /// of them — `core/util/expand_states.ts:10` then asked that array `has`
    /// and `add`.
    #[test]
    fn collect_builds_what_its_target_names() {
        let set = body(
            "pub fn f(items: &Vec<Item>) -> HashSet<u32> { \
             let s: HashSet<_> = items.iter().map(|i| i.id).collect(); s }",
            "f",
        );
        assert!(set.contains("HashSet.from("), "{}", set);

        let map = body(
            "pub fn f(items: &Vec<Item>) -> HashMap<u32, String> { \
             items.iter().map(|i| (i.id, i.name.clone())).collect() }",
            "f",
        );
        assert!(map.contains("HashMap.from("), "{}", map);

        let text = body(
            "pub fn f(items: &Vec<Item>) -> String { \
             items.iter().map(|i| i.name.clone()).collect() }",
            "f",
        );
        assert!(text.contains(".join('')"), "{}", text);

        // A `Vec` IS the sequence, and a turbofish names one as readily as an
        // annotation does.
        let list = body(
            "pub fn f(items: &Vec<Item>) -> String { \
             items.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(\",\") }",
            "f",
        );
        assert!(!list.contains("unsupported("), "a Vec target is the sequence:\n{}", list);
    }

    /// A target the engine cannot name is a hole (R12), not the array that
    /// happens to be right for a `Vec`.
    #[test]
    fn collect_into_a_target_nothing_names_is_a_hole() {
        let ts = body(
            "pub fn f(items: &Vec<Item>) -> u32 { \
             let s = items.iter().map(|i| i.id).collect(); 0 }",
            "f",
        );
        assert!(ts.contains("unsupported("), "{}", ts);
        assert!(ts.contains("could not name the type this one is collected into"), "{}", ts);
    }
}
