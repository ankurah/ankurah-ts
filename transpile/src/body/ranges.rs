//! `a..b` as a VALUE.
//!
//! Rust's range is a value with methods on it — `for i in 0..n` iterates one and
//! `(0..n).rev()` calls a method on one — and the port has no `Range` type.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// A range is a value in Rust — `for i in 0..n` iterates one and
    /// `(0..n).rev()` calls a method on one — and the port has no
    /// `Range` type. It used to be written `undefined`, so
    /// `for (const attempt of undefined)` raised `undefined is not
    /// iterable` the first time the loop was reached; `Entity::commit`'s
    /// retry loop is one of those. A BOUNDED range is the sequence of
    /// its values, which is what makes every adaptor on it work — `rev`,
    /// `map`, `filter` and `contains` are all array operations here.
    ///
    /// An unbounded one — `..n`, `a..`, `..` — has no sequence to build,
    /// and in the one position where Rust means a SLICE by it the index
    /// lowering has already answered. So it is a hole (R12).
    pub(crate) fn range_value(&self, range: &syn::ExprRange) -> String {
        let ends = (range.start.as_ref(), range.end.as_ref());
        let (Some(start), Some(end)) = ends else {
            return self.hole(
                syn::spanned::Spanned::span(range),
                "an unbounded range is not a sequence the port can build, and this one \
                 does not stand where a slice is taken",
            );
        };
        // Only the widths `range`/`rangeIncl` really count. The check used to
        // name the two it could NOT do — a `bigint` width — and let everything
        // else through, so a float range came out as `range(0, 1)`, which is
        // `[0]` and answers `contains` about the wrong values, and a `char`
        // range came out as `rangeIncl('a', 'c')`, which is `["a"]` because
        // `'a' + 1` is `"a1"`. Neither said anything. (F3.)
        if let Some(why) = self.range_endpoint_refusal(start, end) {
            return self.hole(syn::spanned::Spanned::span(range), why);
        }
        let from = self.expr_value(start);
        let to = self.expr_value(end);
        let helper = match range.limits {
            syn::RangeLimits::Closed(_) => "rangeIncl",
            syn::RangeLimits::HalfOpen(_) => "range",
        };
        format!("{}({}, {})", helper, from, to)
    }
    /// Why this range's endpoints cannot be counted, or `None` where they can.
    ///
    /// `range`/`rangeIncl` count with `n++` on a JavaScript number, so the
    /// widths they implement are the discrete ones a number holds exactly:
    /// `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`. A `u64` or
    /// `i64` is a `bigint` here and `n++` is not the same operation; a float
    /// range is not an iterator in Rust either, and only its `contains` is
    /// meaningful, which is lowered from the BOUNDS and never reaches this; a
    /// `char` range is a sequence of code points and the port has no helper for
    /// it. An endpoint the engine could not TYPE is left alone: that is the
    /// engine's own gap and is reported where the name is. J2: the case that
    /// used to be behind that door — `for attempt in 0..MAX_RETRIES` over a
    /// function-local `const` — is not one any more, because an annotated
    /// body-level `const` types its name; so a width the engine CAN name is now
    /// refused wherever it appears, including behind such a const.
    fn range_endpoint_refusal(&self, start: &syn::Expr, end: &syn::Expr) -> Option<String> {
        use crate::ty::{Prim, Ty};
        for e in [start, end] {
            // An endpoint the engine could not type is its own gap and is
            // reported where the name is. What is refused here is a width the
            // engine CAN name and `n++` cannot step — which now includes an
            // annotated body-level `const` (J2).
            let Ok(ty) = self.quietly(|| self.resolve_expr_type(e)) else { continue };
            match ty.peel_refs() {
                Ty::Prim(
                    Prim::U8
                    | Prim::U16
                    | Prim::U32
                    | Prim::Usize
                    | Prim::I8
                    | Prim::I16
                    | Prim::I32
                    | Prim::Isize,
                ) => {}
                Ty::Prim(Prim::U64 | Prim::I64 | Prim::U128 | Prim::I128) => {
                    return Some(
                        "a range over a width the port holds in a `bigint` is not a sequence \
                         the port builds"
                            .to_string(),
                    )
                }
                Ty::Prim(Prim::F32 | Prim::F64) => {
                    return Some(
                        "a float range is not an iterator in Rust either; only `contains` is \
                         meaningful on one, and that is lowered from the bounds rather \
                         than from a sequence"
                            .to_string(),
                    )
                }
                Ty::Prim(Prim::Char) => {
                    return Some(
                        "a `char` range is the sequence of its code points, and the port \
                         writes a `char` as a one-character string, which `n++` does \
                         not step"
                            .to_string(),
                    )
                }
                other => {
                    let named = self
                        .types
                        .as_ref()
                        .map(|tc| tc.borrow().registry.describe(other))
                        .unwrap_or_else(|| "this type".to_string());
                    return Some(format!(
                        "a range over `{named}` is not a sequence the port builds; only a \
                         discrete integer width is"
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    fn body(rust: &str, method: &str) -> String {
        let mut f = Fixture::build(&[("lib.rs", rust)]);
        f.translated_method("lib.rs", method)
    }

    /// F3: only the widths `range`/`rangeIncl` really count are built. The
    /// check named the one width it could NOT do and let everything else
    /// through, so `('a'..='c')` came out as `rangeIncl('a', 'c')` — which is
    /// `["a"]`, because `'a' + 1` is the string `"a1"`.
    #[test]
    fn only_a_discrete_integer_width_is_materialised() {
        let chars = body(
            "pub fn letters() -> Vec<char> { ('a'..='c').collect::<Vec<char>>() }",
            "letters",
        );
        assert!(chars.contains("unsupported("), "{}", chars);
        assert!(chars.contains("code points"), "{}", chars);

        let floats = body(
            "pub fn spread() -> Vec<f64> { (0.0f64..1.0f64).collect::<Vec<f64>>() }",
            "spread",
        );
        assert!(floats.contains("unsupported("), "{}", floats);
        assert!(floats.contains("not an iterator in Rust either"), "{}", floats);

        let ints = body("pub fn ns() -> Vec<u32> { (0u32..4u32).collect() }", "ns");
        assert!(ints.contains("range(0, 4)"), "{}", ints);
    }

    /// An endpoint the engine could not TYPE is its own gap, reported where the
    /// name is. `for attempt in 0..MAX_RETRIES` over a function-local `const`
    /// 3.12, revisited by J2: an ANNOTATED body-level `const` is typed by its
    /// annotation, so `for attempt in 0..MAX_RETRIES` no longer reaches the
    /// rule with an endpoint nothing can name — and a width the engine can name
    /// is refused wherever it stands, including behind such a const.
    #[test]
    fn an_annotated_body_const_gives_its_endpoint_a_width() {
        let whitelisted = body(
            "pub fn retries() -> usize {\n\
               const MAX_RETRIES: usize = 3;\n\
               let mut n = 0usize;\n\
               for _a in 0..MAX_RETRIES { n += 1; }\n\
               n\n\
             }",
            "retries",
        );
        assert!(whitelisted.contains("range(0, MAX_RETRIES)"), "{}", whitelisted);
        assert!(!whitelisted.contains("unsupported("), "{}", whitelisted);

        // The same loop over a width `n++` does not step is refused now, where
        // before the const had no type and the endpoint was let through.
        let wide = body(
            "pub fn wide() -> usize {\n\
               const LIMIT: u64 = 3;\n\
               let mut n = 0usize;\n\
               for _a in 0..LIMIT { n += 1; }\n\
               n\n\
             }",
            "wide",
        );
        assert!(wide.contains("unsupported("), "{}", wide);
        assert!(wide.contains("bigint"), "{}", wide);
    }

    /// `Range::contains` is a comparison against the two ends, and is the one
    /// method a range the port cannot count still answers. Written through the
    /// materialised sequence, `(0.0..1.0).contains(&0.5)` was
    /// `range(0, 1).contains(0.5)` — an array has no `contains`, and nothing
    /// said so.
    ///
    /// O7: it is ONE helper now, with the arguments in Rust's order. Written
    /// inline as `start <= item && item < end` it evaluated the item twice and
    /// the end only where the first comparison held, where Rust builds the
    /// whole range before it looks at the item at all.
    #[test]
    fn contains_is_written_from_the_bounds() {
        let float = body(
            "pub fn within(x: f64) -> bool { (0.0f64..1.0f64).contains(&x) }",
            "within",
        );
        assert_eq!(
            float.lines().find(|l| l.contains("return")).unwrap().trim(),
            "return rangeContains(0.0, 1.0, false, x);"
        );
        assert!(!float.contains("unsupported("), "the sequence is never built:\n{}", float);

        let closed = body(
            "pub fn upto(x: u32) -> bool { (0u32..=16u32).contains(&x) }",
            "upto",
        );
        assert!(closed.contains("rangeContains(0, 16, true, x)"), "{}", closed);
    }

    /// O7: the item is written ONCE. `(0u32..n).contains(&side())` called
    /// `side()` twice, because the inline form names the item in both halves of
    /// its `&&`.
    #[test]
    fn the_item_is_evaluated_once() {
        let ts = body(
            "pub fn side() -> u32 { 3 }\n\
             pub fn within(n: u32) -> bool { (0u32..n).contains(&side()) }",
            "within",
        );
        assert_eq!(ts.matches("side()").count(), 1, "{}", ts);
    }

    /// O8: every UNBOUNDED form is written from the bounds too. `a..`, `..b`,
    /// `..=b` and `..` each fell to the materialisation hole though their
    /// answers are `a <= x`, `x < b`, `x <= b` and `true`; a missing bound is
    /// `null`, which is Rust's `Unbounded`.
    #[test]
    fn an_unbounded_range_answers_contains_too() {
        for (rust, want) in [
            ("pub fn f(x: u32) -> bool { (5u32..).contains(&x) }", "rangeContains(5, null, false, x)"),
            ("pub fn f(x: u32) -> bool { (..5u32).contains(&x) }", "rangeContains(null, 5, false, x)"),
            ("pub fn f(x: u32) -> bool { (..=5u32).contains(&x) }", "rangeContains(null, 5, true, x)"),
            ("pub fn f(x: u32) -> bool { (..).contains(&x) }", "rangeContains(null, null, false, x)"),
        ] {
            let ts = body(rust, "f");
            assert!(ts.contains(want), "`{}` for `{}`:\n{}", want, rust, ts);
            assert!(!ts.contains("unsupported("), "no sequence is built:\n{}", ts);
        }
    }

    /// E7: `step_by` had no lowering and came out as `(range(0, 10)).stepBy(2)`,
    /// a method no array declares, with no diagnostic beside it.
    #[test]
    fn step_by_is_every_nth_value_of_the_sequence() {
        let ts = body("pub fn evens() -> Vec<u32> { (0u32..10u32).step_by(2).collect() }", "evens");
        assert!(ts.contains("stepBy((range(0, 10)), 2)"), "{}", ts);
    }
}
