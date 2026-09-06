//! What ONE element's pattern takes out of the value that element holds.
//!
//! For: a variant's payload member and a tuple's element are the same question
//! asked twice — does this element still have an owner after the pattern has
//! run — and the two sides of the port used to answer it differently. The
//! `Result` side REFUSED an inner pattern that took a droppable name out of the
//! payload, because the port cannot release an object minus one field; the
//! plain enum arm merely left that member out of the `dropUnbound` list and
//! carried on, so what the pattern did not take LEAKED with no word said. And
//! neither looked through an `|`, so `Outer::W(Inner::X(n) | Inner::Y(n))` was
//! read as taking nothing at all and the `Inner` leaked.
//!
//! So the question is asked once here, per element, through the wrappers a
//! pattern may be written behind (`|`, parentheses, `&`), and the four answers
//! are what every caller acts on. K4, K5, K12, K15.

use crate::body::BodyTranslator;

/// What a pattern does to the element it is written for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Takes {
    /// A name for the WHOLE element: `E::V(token)`. The element has an owner,
    /// and the arm releases it through that name.
    Whole,
    /// Nothing at all — `_`, `..`, a literal, a path that only tests. The
    /// element is left where it is and the arm releases it with the rest.
    Nothing,
    /// The pattern reaches INSIDE the element and takes nothing droppable out:
    /// `Expr::Literal(Lit::Count(n))` binds a `u32` and leaves the `Lit` whole.
    /// The element still has no owner, so the arm releases it.
    Inside,
    /// The pattern reaches inside and takes something DROPPABLE out, or binds a
    /// name the engine cannot type. The element is partly moved, and the port
    /// has no way to release an object minus a field: R12 refuses it.
    Part,
}

/// The answer for one element, through `|`, parentheses and `&`.
///
/// Asked INSIDE the pattern's own scope, where the names it binds are typed.
pub(super) fn taken(pat: &syn::Pat, t: &BodyTranslator) -> Takes {
    match pat {
        // `&p` and `(p)` are the same pattern written differently.
        syn::Pat::Paren(paren) => taken(&paren.pat, t),
        syn::Pat::Reference(reference) => taken(&reference.pat, t),
        // Rust makes every alternative of an `|` bind the same names, so they
        // agree about what comes out; the strongest answer is taken so that an
        // alternative that reaches inside is not hidden by one that does not.
        // `Outer::W(Inner::X(n) | Inner::Y(n))` reaches inside on both sides,
        // and the old scan saw only the `|` and said the member was untouched.
        syn::Pat::Or(or) => or.cases.iter().map(|case| taken(case, t)).fold(Takes::Nothing, worst),
        // Asked FIRST of everything below: `None` is a `Pat::Ident` to syn,
        // which reads syntax and not what a name resolves to, and reading it as
        // a binding said `Step::Ready(None)` had an owner for its member.
        _ if BodyTranslator::binds_nothing(pat) => Takes::Nothing,
        // `token`, and `token @ Inner::X(n)`: either way the name owns the
        // whole element, so the arm releases it through that name.
        syn::Pat::Ident(_) => Takes::Whole,
        syn::Pat::TupleStruct(_) | syn::Pat::Struct(_) => {
            // `Some(x)` is the one pattern that reaches inside and leaves NO
            // wrapper: the port writes `Option<T>` as `T | null`, so `x` IS the
            // member and taking it takes all of it. Written as a partial move
            // it refused three live `Poll::Ready(Some(item))` arms in
            // storage-common. (The same shape `result_arms::inner_test` calls
            // `TestsAndTakesAll`.)
            if crate::ownership::arm_takes::takes_the_whole_nullable(pat, &|path| {
                t.names_option_variant(path)
            }) {
                return Takes::Whole;
            }
            if takes_something_droppable(pat, t) {
                Takes::Part
            } else {
                Takes::Inside
            }
        }
        // A tuple is an ARRAY in the port with no drop glue of its own, so the
        // names a tuple pattern binds are what the arm releases — as long as it
        // names EVERY element. `Holder::Pair((a, _))` names one of two, and the
        // emitter wrote `const [a, ] = v._0;` and left the second element with
        // no owner at all (H2). Whether that element is droppable is a question
        // about the member's own type, which the payload walk does not carry
        // here, so the partial tuple is refused: R12, loudly, rather than a
        // leak. A tuple SUBJECT is the same shape with the type in hand, and
        // `value_match` releases its unnamed positions by index.
        syn::Pat::Tuple(_) | syn::Pat::Slice(_)
            if crate::ownership::arm_takes::unowned_positions(pat, elements_in(pat))
                .is_none_or(|unowned| !unowned.is_empty()) =>
        {
            Takes::Part
        }
        // Anything else that binds takes the element whole, which is what the
        // emitter writes for it.
        _ => Takes::Whole,
    }
}

/// Does the arm release the element itself? True for everything the pattern
/// left standing there: what took nothing, and what reached inside without
/// taking anything droppable out.
pub(super) fn element_is_left_whole(pat: &syn::Pat, t: &BodyTranslator) -> bool {
    matches!(taken(pat, t), Takes::Nothing | Takes::Inside)
}

/// Does this pattern bind a name that owns something, or a name the engine
/// cannot type?
///
/// K12: a name with no type is answered as taking something, because the
/// engine cannot say it does not — and that answer now writes a refusal rather
/// than quietly excluding the element from the arm's release.
fn takes_something_droppable(pat: &syn::Pat, t: &BodyTranslator) -> bool {
    let Some(types) = t.types.as_ref() else { return true };
    crate::body::pattern_names(pat).iter().any(|name| {
        let borrowed = types.borrow();
        match borrowed.lookup(name) {
            None => true,
            Some(ty) => crate::ownership::drops_of(&borrowed.probe(), &ty).is_droppable(),
        }
    })
}

/// How many elements a tuple or slice pattern writes, which is how many the
/// value has: Rust makes a tuple pattern name every element unless it writes a
/// `..`, and `unowned_positions` answers `None` for a `..`.
fn elements_in(pat: &syn::Pat) -> usize {
    match pat {
        syn::Pat::Tuple(tuple) => tuple.elems.len(),
        syn::Pat::Slice(slice) => slice.elems.len(),
        _ => 0,
    }
}

/// The stronger of two answers, so that one alternative reaching inside is not
/// hidden by another that does not.
fn worst(a: Takes, b: Takes) -> Takes {
    let rank = |t: Takes| match t {
        Takes::Nothing => 0,
        Takes::Whole => 1,
        Takes::Inside => 2,
        Takes::Part => 3,
    };
    if rank(b) > rank(a) {
        b
    } else {
        a
    }
}
