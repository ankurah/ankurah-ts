//! What a closure's parameters and result are (spec 4.5).
//!
//! For: a closure in Rust is written without types because the position it
//! stands in supplies them — `broadcast.listen(|value| ..)` types `value` from
//! `listen`'s `L: IntoBroadcastListener<T>`, and `let f: Box<dyn Fn(u32)> = |x|
//! ..` types `x` from the annotation. Until the parameters have types the body
//! cannot be typed either, so every call inside a closure fell back, which is
//! most of "bound here but the engine could not type it".
//!
//! Three sources, in the order Rust reads them: the annotation the closure
//! writes for itself, the callable the position expects, and — for the result
//! only — the body's own tail.

use super::expected;
use super::TypeContext;
use crate::ty::Ty;

/// A closure's signature, as far as those three sources settle it.
///
/// A parameter the engine could not type stays in the list under its name: the
/// body still binds the name, and binding it untyped is what keeps the
/// translator from reading it as a name nobody declared.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureSig {
    pub params: Vec<(String, Option<Ty>)>,
    pub ret: Option<Ty>,
}

impl ClosureSig {
    /// The parameters nothing typed, named so a diagnostic can say which.
    pub fn untyped_params(&self) -> Vec<String> {
        self.params
            .iter()
            .filter(|(_, ty)| ty.is_none())
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl TypeContext<'_> {
    /// A closure's signature at a position that expects `expected`.
    ///
    /// The annotation wins where the source wrote one, because it is what the
    /// source said. Where it did not, the expected callable's parameter at the
    /// same position answers. The result comes from the closure's own `->`
    /// first, then the expected callable's `Output`, and last from typing the
    /// body with the parameters bound.
    pub fn closure_signature(
        &self,
        closure: &syn::ExprClosure,
        expected: Option<&Ty>,
    ) -> ClosureSig {
        let probe = self.probe();
        let shape = expected.and_then(|ty| {
            expected::fn_shape_through_impls(&probe, self.registry, ty, &self.param_bounds)
        });

        // A closure written with more or fewer parameters than the position
        // calls it with is one rustc rejects, so the two disagreeing means the
        // engine read one of them wrongly and neither can be trusted to type
        // the body.
        if let Some(shape) = &shape {
            if shape.inputs.len() != closure.inputs.len() {
                self.sink.report(
                    syn::spanned::Spanned::span(closure),
                    format!(
                        "this closure is written with {} parameter(s) and the position it \
                         stands in calls it with {}, which Rust does not allow, so one of the \
                         two is not what the engine read",
                        closure.inputs.len(),
                        shape.inputs.len()
                    ),
                );
            }
        }

        let mut params: Vec<(String, Option<Ty>)> = Vec::new();
        for (index, pat) in closure.inputs.iter().enumerate() {
            let name = crate::body::BodyTranslator::pat_static(pat);
            let annotated = match pat {
                syn::Pat::Type(typed) => self.resolve_written_type(&typed.ty).ok(),
                _ => None,
            };
            let from_position = shape.as_ref().and_then(|s| s.inputs.get(index)).cloned();
            let ty = match (annotated, from_position) {
                (Some(written), Some(want)) => {
                    // The annotation is what the source said and it stands; but
                    // an annotation the position contradicts is one rustc would
                    // have rejected, so the disagreement is a fact about the
                    // engine's reading and is said out loud.
                    let filled = expected::fill_infer(&written, &want);
                    if filled.peel_refs() != want.peel_refs()
                        && !expected::has_infer(&written)
                        && expected::is_settled(&want, &self.params)
                    {
                        self.sink.report(
                            syn::spanned::Spanned::span(pat),
                            format!(
                                "this closure annotates `{}` as `{}` and the position it \
                                 stands in wants a `{}`, which Rust does not allow; the \
                                 annotation is what the body is typed with",
                                name,
                                self.registry.describe(&filled),
                                self.registry.describe(&want)
                            ),
                        );
                    }
                    Some(filled)
                }
                (Some(written), None) => Some(written),
                (None, Some(want)) => self.through_pattern(pat, &want),
                (None, None) => None,
            };
            params.push((name, ty));
        }

        let ret = match &closure.output {
            syn::ReturnType::Type(_, ty) => self.resolve_written_type(ty).ok(),
            // The expected callable's `Output` answers only where the callee
            // settled it. `Iterator::map` declares `F: FnMut(Self::Item) -> B`
            // and leaves `B` to whatever the closure returns, so reading `B`
            // off the bound would answer the question with the question.
            syn::ReturnType::Default => shape
                .as_ref()
                .map(|s| s.output.clone())
                .filter(|ty| *ty != Ty::Unit && expected::is_settled(ty, &self.params))
                .or_else(|| self.closure_body_type(closure, &params)),
        };

        let sig = ClosureSig { params, ret };
        crate::trace::record_closure(
            self.registry,
            &self.sink.file(),
            syn::spanned::Spanned::span(closure),
            &sig,
        );
        sig
    }

    /// The type the name a closure parameter binds actually holds, once the
    /// pattern between it and the value is taken into account.
    ///
    /// `|&x|` applied to a `&u8` binds `x` to a `u8`; `|(a, b)|` binds two
    /// names and one type belongs to neither. Only a plain name — the common
    /// case — takes the position's type whole.
    fn through_pattern(&self, pat: &syn::Pat, want: &Ty) -> Option<Ty> {
        match pat {
            syn::Pat::Ident(_) | syn::Pat::Wild(_) => Some(want.clone()),
            syn::Pat::Reference(inner) => self.through_pattern(&inner.pat, want.peel_refs()),
            syn::Pat::Type(typed) => self.through_pattern(&typed.pat, want),
            syn::Pat::Paren(paren) => self.through_pattern(&paren.pat, want),
            // A tuple or a struct pattern binds several names at once, and the
            // translator binds those through the pattern machinery instead.
            _ => None,
        }
    }

    /// What the closure's body produces, asked with its parameters visible.
    ///
    /// A `?` and a `return` inside a closure both leave through the closure's
    /// own result rather than the enclosing function's, so the tail is read
    /// with the closure's result standing as the function's for the length of
    /// the question.
    fn closure_body_type(
        &self,
        closure: &syn::ExprClosure,
        params: &[(String, Option<Ty>)],
    ) -> Option<Ty> {
        self.with_closure_params(params, || match &*closure.body {
            syn::Expr::Block(block) => self.block_tail_type(&block.block).ok(),
            other => self.resolve_expr(other).ok(),
        })
        // `|s| s.try_into()` is a `Result<U, U::Error>` until something says
        // what `U` is, and here that something is a turbofish two calls further
        // on. Propagation is one level deep (spec 4.6), so the engine says it
        // cannot tell rather than handing on somebody else's open parameter.
        .filter(|ty| expected::is_settled(ty, &self.params))
    }
}
