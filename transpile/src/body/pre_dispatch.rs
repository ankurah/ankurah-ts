//! Methods the body translator answers before the native-type dispatch.
//!
//! For: what a call is written as usually follows from the RECEIVER's type, and
//! `native_types` is the table that says so. Four questions do not. A method
//! call's arguments are written for the type the CALLEE declares each of them
//! to be. `collect()` builds whatever its target names, and only the position
//! the call stands in says what that is. A consuming iterator terminal called
//! on a NAMED iterator is refused, because after it the port's array holds both
//! what the walk took and what it left. `unwrap_or` on a value the port writes
//! as a nullable is `??`, which is a fact about the port's spelling of `Option`
//! and not about the receiver's class. Each is settled here, where the call
//! expression and the position are both in hand, and never reaches the table.

use super::BodyTranslator;

impl<'a> BodyTranslator<'a> {
    /// What this call is written as WITHOUT asking the receiver's type table,
    /// or `None` where the table is what answers it.
    pub(crate) fn answered_before_dispatch(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
        args: &[String],
    ) -> Option<String> {
        if let Some(written) = self.collected_into(call, rust_method, receiver) {
            return Some(written);
        }
        if let Some(written) = self.iteration_of_a_parameter(call, rust_method, receiver) {
            return Some(written);
        }
        if let Some(written) = self.range_contains(call, args) {
            return Some(written);
        }
        // N10: `unwrap_or_default` answers the payload's `Default`, which the
        // port has to name from the resolved type — the method's own name says
        // nothing about it, and `unwrapOrDefault` is declared nowhere.
        if let Some(written) = self.unwrap_or_default(call, rust_method, receiver) {
            return Some(written);
        }
        // F1: a consuming terminal on a NAMED iterator leaves part of the
        // sequence behind, and the port's array cannot say which part.
        self.named_iterator_refusal(call)
            .map(|why| self.hole(syn::spanned::Spanned::span(call), why))
    }

    /// `(a..b).contains(&x)` from the BOUNDS, not from a sequence.
    ///
    /// `Range::contains` is a comparison against the two ends and is the one
    /// method a range of a type the port cannot count still answers — a float
    /// range is not an iterator in Rust either, and neither is an unbounded
    /// one. Written through the materialised sequence,
    /// `(0.0f64..1.0f64).contains(&0.5)` came out as `range(0, 1).contains(
    /// 0.5)`: `range(0, 1)` is `[0]`, and an array has no `contains` (F3).
    ///
    /// O7: written inline as `start <= item && item < end` it evaluated the
    /// ITEM twice — `(0u32..n).contains(&side())` called `side()` twice — and
    /// it evaluated the end only when the first comparison held, where Rust
    /// builds the whole range before it looks at the item at all. One helper,
    /// with the arguments in Rust's order.
    ///
    /// O8: and every UNBOUNDED form is written here too. `a..`, `..b`, `..=b`
    /// and `..` each fell to the materialisation hole though their answers are
    /// `a <= x`, `x < b`, `x <= b` and `true`; a missing bound is `null`, which
    /// is Rust's `Unbounded`.
    fn range_contains(&self, call: &syn::ExprMethodCall, args: &[String]) -> Option<String> {
        let range = self.range_of_contains(call)?;
        let item = args.first()?;
        let bound = |e: Option<&Box<syn::Expr>>| match e {
            Some(e) => self.expr_value(e),
            None => "null".to_string(),
        };
        let inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));
        Some(format!(
            "rangeContains({}, {}, {}, {})",
            bound(range.start.as_ref()),
            bound(range.end.as_ref()),
            inclusive,
            item
        ))
    }

    /// Is this `range.contains(&x)`, and on which range? Asked by the lowering
    /// and by the receiver position, so the receiver is not materialised for a
    /// call that never reads the sequence.
    pub(crate) fn range_of_contains<'c>(
        &self,
        call: &'c syn::ExprMethodCall,
    ) -> Option<&'c syn::ExprRange> {
        if call.method != "contains" || call.args.len() != 1 {
            return None;
        }
        match crate::infer::calls::unparenthesise(&call.receiver) {
            syn::Expr::Range(range) => Some(range),
            _ => None,
        }
    }

    /// The same question, as the receiver position asks it.
    /// O8: every bound shape, because every one of them is written from the
    /// bounds now — an unbounded range has no sequence to materialise at all.
    pub(crate) fn contains_on_a_range(&self, call: &syn::ExprMethodCall) -> bool {
        self.range_of_contains(call).is_some()
    }

    /// `IntoIterator::into_iter` on a type PARAMETER, as the spread it is on
    /// every other receiver.
    ///
    /// The port materialises an iterator as a JavaScript array — that is what
    /// makes `map`, `filter`, `rev` and `contains` array operations here — so
    /// `into_iter` is the spread whatever the receiver is written as. The shape
    /// table answers it for a `Vec`, a map, a set and a receiver the engine
    /// could not name at all, and a bare type parameter fell between those:
    /// `js_shape` says `Plain` for it, and the call came out as the camelCase
    /// of its Rust name. `values.intoIter()` is `TypeError: values.intoIter is
    /// not a function`, and it is why all seven of ankql's
    /// `ast.test.ts` cases die — `Predicate::populate` takes an
    /// `I: IntoIterator<Item = V>` (G1).
    ///
    /// The RESOLUTION is what says this is `IntoIterator` and not some crate
    /// method of the same name; the parameter's own bound is what makes the
    /// value iterable at run time.
    fn iteration_of_a_parameter(
        &self,
        call: &syn::ExprMethodCall,
        rust_method: &str,
        receiver: &str,
    ) -> Option<String> {
        if rust_method != "into_iter" || !call.args.is_empty() {
            return None;
        }
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(&call.receiver, rust_method, call.turbofish.as_ref());
        let receiver_ty = found.as_ref().ok().map(|f| f.receiver_type().clone());
        tc.sink.rewind(mark);
        let found = found.ok()?;
        if !matches!(receiver_ty, Some(crate::ty::Ty::Param(_))) {
            return None;
        }
        // The parameter's own DECLARED bound is what makes the value iterable at
        // run time. A resolution that reached `IntoIterator` through the
        // blanket `impl<I: Iterator> IntoIterator for I` rests on an undecided
        // `T: Iterator`, and answering it here would both guess and silence the
        // report — `storage/common/sorting.rs`'s `mem::take(..).into_iter()` is
        // five of those, and they keep the diagnostic they had.
        if !found.obligations.is_empty() {
            return None;
        }
        let trait_id = tc.registry.method_trait(&found)?;
        if tc.registry.system_type("std::iter::IntoIterator") != Some(trait_id) {
            return None;
        }
        // The call resolved and this is the port's whole answer for it, so it
        // is recorded before the answer is written — the way `collect` is.
        drop(tc);
        self.record_resolution(call, rust_method);
        Some(format!("[...{}]", receiver))
    }

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
        // E10/J3: the statement's move flag stands after everything the call
        // evaluates, whatever the call's shape.
        let each = Self::each_argument(&call.args);
        self.lifted_above_the_flag(&syn::Expr::MethodCall(call.clone()), &each, args)
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
            JsShape::Array(_) => Some(receiver.to_string()),
            // A `Vec<u8>` is not: the port writes it as a `Uint8Array`, and a
            // chain hands back the array its adaptors built. Identity here put
            // a `number[]` behind three `Result<Uint8Array, IndexError>`
            // returns in `core/indexing/encoding.ts`, where every caller reads
            // it as bytes.
            JsShape::Bytes => Some(format!("Uint8Array.from({})", receiver)),
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

    /// Every position that names a target, one test each. The `let`
    /// annotation and the function's return type were already read; the three
    /// added here are the ones the corpus needed — a tuple-struct
    /// constructor's field (including through `Self`), an `Ok(..)` inside a
    /// match arm or an `if` branch, and a field of an enum-VARIANT literal.
    #[test]
    fn every_position_that_names_a_target_names_it() {
        let mut c = Fixture::build(&[(
            "lib.rs",
            "use std::collections::HashSet;\n\
             pub struct Clock(pub Vec<u32>);\n\
             pub enum Body { Get { ids: HashSet<u32> } }\n\
             impl From<Vec<u32>> for Clock {\n\
             fn from(ids: Vec<u32>) -> Self { Self(ids.into_iter().collect()) } }\n\
             pub fn build(ids: Vec<u32>) -> Clock { Clock(ids.into_iter().collect()) }\n\
             pub fn in_arm(ns: Vec<u32>, flag: bool) -> Result<Vec<u32>, ()> {\n\
             match flag { true => Ok(ns.into_iter().collect()), false => Err(()) } }\n\
             pub fn in_branch(ns: Vec<u32>, flag: bool) -> Result<Vec<u32>, ()> {\n\
             if flag { Err(()) } else { Ok(ns.into_iter().collect()) } }\n\
             pub fn variant_field(ids: Vec<u32>) -> Body { Body::Get { ids: ids.into_iter().collect() } }",
        )]);
        for f in ["from", "build", "in_arm", "in_branch", "variant_field"] {
            let ts = c.translated_method("lib.rs", f);
            assert!(!ts.contains("unsupported("), "`{}` still has no target:\n{}", f, ts);
        }
        assert!(
            c.translated_method("lib.rs", "variant_field").contains("HashSet.from("),
            "the variant's field type says a set is built"
        );
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

    /// A `Vec<u8>` target is a `Uint8Array`, and an array is not one.
    ///
    /// E1: the `Vec` arm answered the receiver for a byte target as well, so
    /// `bytes.into_iter().map(..).collect()` behind a
    /// `Result<Vec<u8>, IndexError>` handed back a `number[]` — three sites in
    /// `core/indexing/encoding.ts`, each read as bytes by its caller. The
    /// neighbouring `vec![..]` arm has always written `new Uint8Array([..])`.
    #[test]
    fn collect_into_a_byte_target_builds_a_uint8array() {
        let bytes = body(
            "pub fn f(bytes: Vec<u8>) -> Vec<u8> {              bytes.into_iter().map(|b| 0xFFu8.wrapping_sub(b)).collect() }",
            "f",
        );
        assert!(bytes.contains("Uint8Array.from("), "a byte target builds bytes:\n{}", bytes);
        assert!(!bytes.contains("unsupported("), "{}", bytes);

        // Through the `let`'s annotation, and through a turbofish, alike.
        let annotated = body(
            "pub fn f(bytes: Vec<u8>) -> usize {              let out: Vec<u8> = bytes.into_iter().map(|b| b + 1).collect(); out.len() }",
            "f",
        );
        assert!(annotated.contains("Uint8Array.from("), "{}", annotated);
        let turbofish = body(
            "pub fn f(bytes: Vec<u8>) -> usize {              bytes.into_iter().map(|b| b + 1).collect::<Vec<u8>>().len() }",
            "f",
        );
        assert!(turbofish.contains("Uint8Array.from("), "{}", turbofish);

        // A `Vec` of anything else is still the sequence itself, uncopied.
        let list = body(
            "pub fn f(ns: Vec<u32>) -> Vec<u32> { ns.into_iter().map(|n| n + 1).collect() }",
            "f",
        );
        assert!(!list.contains("Uint8Array"), "a u32 vector is an array:\n{}", list);
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
