//! What a `for` loop owns, and when it lets go of it.
//!
//! `for entity in entities` hands the loop the whole sequence: Rust's
//! `IntoIterator` takes it by value, each turn takes one element out, and the
//! iterator drops whatever is left when the loop stops — which is what a
//! `break` or a `return` out of the body does. Emitting a plain `for … of`
//! released none of that: the element each turn bound was never dropped, and an
//! early exit abandoned the rest of the sequence.
//!
//! A loop over `&entities` owns nothing and needs none of this; the item type
//! says which of the two this is.

use crate::registry::Probe;
use crate::ty::Ty;
use crate::body::{indent, BodyTranslator};
use crate::ownership;

/// How a `for` loop over this sequence has to be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Iterate {
    /// `for … of`: the loop borrows, and the elements belong to somebody else.
    Borrowed,
    /// The sequence is a JavaScript array the loop owns, so it is walked by
    /// index and the tail it never reached is released at the end.
    OwnedArray,
    /// The loop owns a MAP, which the runtime writes as a keyed container
    /// rather than an array: `intoEntries()` empties it, marks it moved and
    /// hands the pairs over, and from there the loop is the array walk.
    OwnedMap,
    /// The loop owns the sequence, but the runtime does not write it as an
    /// array or a map — an iterator adapter, a set — so there is no way to name
    /// the elements the loop did not reach. Reported.
    OwnedOpaque,
}

/// The label a Rust loop carries, ready to stand before the emitted loop.
///
/// Rust's `break 'outer` names the loop it leaves. Dropping the name emitted a
/// bare `break`, which leaves the innermost loop instead — a different program,
/// and one whose `finally` blocks still balance, so nothing downstream sees
/// that the wrong elements were processed.
pub fn label_of(label: &Option<syn::Label>) -> String {
    match label {
        Some(label) => format!("{}: ", label.name.ident),
        None => String::new(),
    }
}

/// The loop a `break 'outer` or a `continue 'outer` names, as the suffix that
/// follows the keyword.
pub fn target_of(label: &Option<syn::Lifetime>) -> String {
    match label {
        Some(lifetime) => format!(" {}", lifetime.ident),
        None => String::new(),
    }
}

/// Which form this loop needs, from the sequence's type and the item's.
///
/// The item decides whether anything is owned at all: `IntoIterator for
/// Vec<T>` hands out a `T` and `IntoIterator for &Vec<T>` hands out a `&T`,
/// and the second owns nothing.
pub fn iterate(probe: &Probe, sequence: Option<&Ty>, item: Option<&Ty>) -> Iterate {
    let owns_items = item.is_some_and(|item| crate::ownership::drops_of(probe, item).is_droppable());
    if !owns_items {
        return Iterate::Borrowed;
    }
    match sequence {
        Some(ty) if is_array(probe, ty) => Iterate::OwnedArray,
        Some(ty) if is_map(probe, ty) => Iterate::OwnedMap,
        _ => Iterate::OwnedOpaque,
    }
}

/// Is this a sequence the runtime writes as a JavaScript array?
fn is_array(probe: &Probe, ty: &Ty) -> bool {
    match ty {
        Ty::Array { .. } | Ty::Slice(_) => true,
        Ty::Ref { .. } => false,
        Ty::Named { id, .. } => probe
            .reg
            .system_type("std::vec::Vec")
            .is_some_and(|vec| vec == *id),
        _ => false,
    }
}

/// Is this a container the runtime writes as a keyed MAP?
fn is_map(probe: &Probe, ty: &Ty) -> bool {
    matches!(
        crate::name_map::shape::js_shape(probe.reg, ty),
        crate::name_map::shape::JsShape::Map(..)
    )
}

/// The owned-array loop: one element out per turn, and the tail released when
/// the loop stops for any reason.
///
/// `at` is the index the next turn would read, so the `finally` releases
/// exactly what `next()` never handed out — which is what dropping Rust's
/// `IntoIter` does.
pub fn owned_array_loop(
    sequence: &str,
    at: &str,
    binding: &str,
    body: &str,
    label: &str,
) -> String {
    format!(
        "let {at} = 0;\n\
         try {{\n  \
           {label}while ({at} < {sequence}.length) {{\n    \
             const {binding} = {sequence}[{at}++];\n\
{body}  \
           }}\n\
         }} finally {{\n  \
           dropOwned({sequence}.slice({at}));\n\
         }}",
        at = at,
        sequence = sequence,
        binding = binding,
        label = label,
        body = crate::body::indent(&crate::body::indent(body)),
    )
}

/// Does this pattern bind the whole element by reference — `ref item` or
/// `ref mut item`?
///
/// Only the whole one: a `ref` inside a destructuring binds part of an element
/// the loop already released field by field, and claiming the part as well
/// would release it twice.
fn binds_the_whole_element_by_reference(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(ident) => ident.by_ref.is_some() && ident.subpat.is_none(),
        syn::Pat::Paren(p) => binds_the_whole_element_by_reference(&p.pat),
        _ => false,
    }
}

impl<'a> BodyTranslator<'a> {
    /// `for x in seq { .. }`, with what the loop owns released where Rust
    /// releases it.
    ///
    /// Rust's `IntoIterator` takes the sequence by value, hands out one element
    /// per turn and drops the rest when the loop stops — so the binding is
    /// released at the end of each turn, and a `break` or a `return` releases
    /// everything the loop never reached. A loop over `&seq` owns none of that,
    /// and the item type is what tells the two apart.
    pub(crate) fn for_loop(&self, for_loop: &syn::ExprForLoop) -> String {
        use ownership::iteration::Iterate;
        let pat = Self::pat_static(&for_loop.pat);
        // A cursor iterated is a cursor consumed: the loop sees the elements
        // it has not handed out, and takes them out of it.
        let sequence = self.cursor_gives_up_its_rest(
            &for_loop.expr,
            self.expr(&for_loop.expr),
        );
        let item = self.iteration_item(&for_loop.expr);
        let sequence_ty = self.quietly(|| self.iterated_type(&for_loop.expr));
        let form = match &self.types {
            Some(tc) => ownership::iteration::iterate(
                &tc.borrow().probe(),
                sequence_ty.as_ref(),
                item.as_ref(),
            ),
            None => Iterate::Borrowed,
        };
        let _bindings = self.enter_pattern(&for_loop.pat, item.as_ref());
        let owned = match form {
            Iterate::Borrowed => Vec::new(),
            // Over an OWNED sequence the loop owns the element it was handed,
            // whatever the pattern binds of it. A `ref` binding's own type is a
            // `&T`, which owns nothing, so the ELEMENT's type decides — and the
            // release lands on the name the loop wrote, which is the element
            // itself. Without it `for ref item in owned_vec` leaked every
            // element: the tail release starts after the current index and
            // cannot reach what the turn already handed out.
            _ => {
                let names = crate::body::pattern_names(&for_loop.pat);
                let element = item.clone();
                let binds_by_reference = binds_the_whole_element_by_reference(&for_loop.pat);
                self.claim_bindings_as(
                    &names,
                    &|name| {
                        if binds_by_reference {
                            element.clone()
                        } else {
                            self.types.as_ref().and_then(|tc| tc.borrow().lookup(name))
                        }
                    },
                    crate::ownership::Drops::Unknown,
                    &for_loop.body.stmts,
                )
            }
        };
        // S2: the names this turn holds, and the ones the claim above released.
        // A statement inside the body that REFUSES hands nothing away, so a
        // binding the claim wrote off as moved is still the turn's — and the
        // tail release starts after the current index and cannot reach it.
        // X4/X7: a FRAME, pushed here and popped when the loop's body ends.
        // These three sets used to be body-global and keyed by spelling, so a
        // `let xs = replacement;` AFTER the loop still read `xs` as "some other
        // frame releases it" and nothing released the replacement, and a second
        // loop reusing an element name inherited the first loop's answer for it.
        // Only what THIS frame added comes out again, so a nested loop over the
        // same name leaves the outer frame's entry where it was.
        let mut frame = LoopFrame::default();
        if !matches!(form, Iterate::Borrowed) {
            // The sequence itself is the loop's from here: it is aliased into
            // `_seqN` and its tail is released by the loop's own `finally`.
            if let syn::Expr::Path(path) = &*for_loop.expr {
                if let Some(name) = ownership::moves::local_name(path) {
                    if self.own.released_elsewhere.borrow_mut().insert(name.clone()) {
                        frame.elsewhere.push(name);
                    }
                }
            }
            let claimed: Vec<String> = owned.iter().map(|o| o.name.clone()).collect();
            let mut all = self.own.loop_bindings.borrow_mut();
            let mut done = self.own.claimed_loop_bindings.borrow_mut();
            for name in crate::body::pattern_names(&for_loop.pat) {
                if all.insert(name.clone()) {
                    frame.bindings.push(name.clone());
                }
                if claimed.contains(&name) && done.insert(name.clone()) {
                    frame.claimed.push(name);
                }
            }
        }
        let body = crate::control_flow::sentinel::inside_a_loop(self, &for_loop.label, || {
            self.translate_loop_block(&for_loop.body)
        });
        frame.pop(self);
        drop(_bindings);
        let body = self.wrap_bindings(&owned, body);
        let label = ownership::iteration::label_of(&for_loop.label);
        match form {
            Iterate::Borrowed => {
                format!("{}for (const {} of {}) {{\n{}}}", label, pat, sequence, indent(&body))
            }
            Iterate::OwnedArray => {
                let held = self.fresh_hoist("_seq");
                let at = self.fresh_hoist("_at");
                let loop_ts =
                    ownership::iteration::owned_array_loop(&held, &at, &pat, &body, &label);
                format!("const {} = {};\n{}", held, sequence, loop_ts)
            }
            // `for (k, v) in map` moves the map into its `IntoIter`, which
            // hands out an owned pair each turn and drops what it never handed
            // out. `intoEntries()` is that move: it empties the map, marks it
            // dropped and hands the pairs over, and from there this is the
            // array walk — so the tail release covers the pairs the loop never
            // reached, and the container is nobody's to release afterwards.
            // Written as a plain `for … of`, the loop released every key and
            // value and left the container to the collector, which is the leak
            // `goldens/borrowed_iteration` recorded.
            Iterate::OwnedMap => {
                let held = self.fresh_hoist("_seq");
                let at = self.fresh_hoist("_at");
                let loop_ts =
                    ownership::iteration::owned_array_loop(&held, &at, &pat, &body, &label);
                format!("const {} = {}.intoEntries();\n{}", held, sequence, loop_ts)
            }
            Iterate::OwnedOpaque => {
                self.fallback(
                    syn::spanned::Spanned::span(&for_loop.expr),
                    "this loop takes the sequence by value, and the runtime does not write it \
                     as an array; the elements a `break` or a `return` leaves behind are not \
                     released, and neither is the container itself, which Rust drops when the \
                     iterator it was moved into ends",
                );
                format!("{}for (const {} of {}) {{\n{}}}", label, pat, sequence, indent(&body))
            }
        }
    }
}

/// What one loop's turn added to the body's ownership state, so that the state
/// can be put back when the turn's body has been written.
///
/// X4/X7: three body-global `HashSet<String>`s answered questions about a name
/// long after the loop that put it there had ended — for a `let` of the same
/// name below the loop, and for the next loop that reused an element's
/// spelling. A frame is the scope the answers really have.
#[derive(Default)]
struct LoopFrame {
    elsewhere: Vec<String>,
    bindings: Vec<String>,
    claimed: Vec<String>,
}

impl LoopFrame {
    fn pop(&mut self, t: &crate::body::BodyTranslator) {
        for name in self.elsewhere.drain(..) {
            t.own.released_elsewhere.borrow_mut().remove(&name);
        }
        for name in self.bindings.drain(..) {
            t.own.loop_bindings.borrow_mut().remove(&name);
        }
        for name in self.claimed.drain(..) {
            t.own.claimed_loop_bindings.borrow_mut().remove(&name);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// X4: the three sets a consuming loop writes are a FRAME, not a body-global
    /// record keyed by spelling. `for item in xs { .. }` says "some other frame
    /// releases `xs`", which is true of the sequence the loop took and not of a
    /// `let xs = ..` written below it.
    #[test]
    fn a_shadow_below_a_loop_is_not_the_sequence_the_loop_took() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub fn look(t: &Token) -> u32 { t.n }\n\
             pub fn shadowed(xs: Vec<Token>, replacement: Vec<Token>) -> u32 {\n\
               let xs = xs;\n\
               let mut total = 0;\n\
               for item in xs { total += look(&item); }\n\
               let xs = replacement;\n\
               let built: std::collections::BinaryHeap<u32> = \
                 xs.into_iter().map(|t| t.n).collect();\n\
               total + built.len() as u32\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "shadowed");
        assert!(ts.contains("unsupported("), "the collect was expected to refuse:\n{}", ts);
        assert!(
            ts.contains("dropOwned(xs_2);"),
            "the replacement is the block's, and the loop above it took a different \
             sequence:\n{}",
            ts
        );
    }

    /// And the frame really is popped rather than the name simply being fresh:
    /// two loops over the same spelling each get their own answer.
    #[test]
    fn two_loops_over_one_spelling_each_answer_for_themselves() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub fn look(t: &Token) -> u32 { t.n }\n\
             pub fn twice(a: Vec<Token>, b: Vec<Token>) -> u32 {\n\
               let mut total = 0;\n\
               for rest in a { total += look(&rest); }\n\
               for rest in b { total += look(&rest); }\n\
               total\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "twice");
        assert_eq!(
            ts.matches("rest.drop();").count(),
            2,
            "each turn of each loop releases its own element:\n{}",
            ts
        );
    }
}
