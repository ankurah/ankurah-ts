//! What an operator, an index and a range answer with.
//!
//! Each is a position that says something about the value written in it: an
//! unsuffixed literal takes the width of what it is added to, an index takes
//! the element of what it reaches into, and a range takes the type of its ends.

use super::context::TypeContext;
use crate::diag::Diag;
use crate::ty::{Prim, TraitRef, Ty};

impl TypeContext<'_> {
    /// A binary operator's result. Comparison and logical operators are `bool`
    /// whatever they are applied to; arithmetic on primitives is the primitive.
    /// An operator on anything else resolves through its trait's `Output`, which
    /// is the operators step.
    pub(super) fn binary_type(&self, bin: &syn::ExprBinary, expected: Option<&Ty>) -> Result<Ty, Diag> {
        use syn::BinOp::*;
        match bin.op {
            // A comparison answers `bool` whatever it compares, but the two
            // sides are one type, which is what gives an unsuffixed literal its
            // width: `n < 5` where `n: u64` compares two 64-bit values.
            Eq(_) | Ne(_) | Lt(_) | Le(_) | Gt(_) | Ge(_) => {
                if let (Ok(left), Ok(right)) =
                    (self.resolve_expr(&bin.left), self.resolve_expr(&bin.right))
                {
                    self.constrain_here(syn::spanned::Spanned::span(&bin.right), &left, &right);
                }
                return Ok(Ty::Prim(Prim::Bool));
            }
            // `&&` and `||` take `bool` on both sides and nothing else.
            And(_) | Or(_) => {
                for side in [&bin.left, &bin.right] {
                    if let Ok(found) = self.resolve_expr(side) {
                        self.constrain_here(
                            syn::spanned::Spanned::span(side),
                            &Ty::Prim(Prim::Bool),
                            &found,
                        );
                    }
                }
                return Ok(Ty::Prim(Prim::Bool));
            }
            AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
            | BitXorAssign(_) | BitAndAssign(_) | BitOrAssign(_) | ShlAssign(_)
            | ShrAssign(_) => return Ok(Ty::Unit),
            _ => {}
        }
        // An unsuffixed integer literal takes the type of whatever it is written
        // against — `n + 1` where `n: usize` is `usize` arithmetic, not a type
        // mismatch between `usize` and the literal's default `i32`.
        // A shift is the exception: its right operand has a type of its own —
        // `1u64 << 63i32` is a `u64` — so reading the whole expression off the
        // shift amount answered `i32` for what the position said was 64-bit.
        if !matches!(bin.op, Shl(_) | Shr(_)) {
            if let Some(ty) = self.literal_against(&bin.left, &bin.right)? {
                return Ok(ty);
            }
        }
        // Where both operands are literals the whole expression takes its type
        // from the position, which is how `bits ^ (1 << 63)` beside a `u64` is
        // 64-bit arithmetic rather than the literal's default `i32`.
        let left = self.resolve_expr_expecting(&bin.left, expected)?;
        let Ty::Prim(prim) = left.peel_refs() else {
            // An operator between ported types is a call, and its impl says
            // what the call answers.
            if let Some(output) = self.overloaded_result(bin, &left) {
                return Ok(output);
            }
            return Err(self.refuse(
                syn::spanned::Spanned::span(bin),
                format!(
                    "`{}` overloads this operator; which impl it takes is the operators step",
                    self.registry.describe(&left)
                ),
            ));
        };
        // A shift's right operand has its own type; every other arithmetic
        // operator on primitives takes two of the same and gives that back.
        if matches!(bin.op, Shl(_) | Shr(_)) {
            return Ok(Ty::Prim(*prim));
        }
        let right = self.resolve_expr(&bin.right)?;
        if right.peel_refs() == &Ty::Prim(*prim) {
            Ok(Ty::Prim(*prim))
        } else {
            Err(self.refuse(
                syn::spanned::Spanned::span(bin),
                "the two sides of this operator are different types",
            ))
        }
    }

    /// What an overloaded operator answers: its impl's `Output`.
    ///
    /// Only an impl with no parameters of its own is read. A generic one —
    /// `impl<T> Add for Wrapper<T>` — writes its `Output` in terms of those
    /// parameters, and the impl table hands back the impl it matched without
    /// the substitution that matched it, so there is nothing here to put in
    /// their place. Asking is not translating: what the right operand could not
    /// say is reported where the operator is written.
    pub(super) fn overloaded_result(&self, bin: &syn::ExprBinary, left: &Ty) -> Option<Ty> {
        let trait_path = crate::operators::operator_trait(&bin.op)?;
        let mark = self.sink.mark();
        let right = self.resolve_expr(&bin.right);
        self.sink.rewind(mark);
        // Rust's operator traits default `Rhs` to `Self`, which every operator
        // impl in the corpus takes.
        let right = right.unwrap_or_else(|_| left.clone());
        let found = self.probe().operator_impl(&trait_path, left, &right).ok()?;
        let def = self.registry.impl_def(found.impl_id);
        // A generic impl says what it answers in terms of its own parameters —
        // `impl<T> Add for Generic<T> { type Output = Generic<T>; }` — and the
        // match that selected it is what says which `T` this site has. Refusing
        // every generic impl left the local a `+` was bound to untyped, so
        // nothing released what it held.
        Some(def.assoc_types.get("Output")?.substitute(&found.args))
    }

    /// The type of an arithmetic operator where one side is an unsuffixed
    /// integer literal: the other side's, since that is what Rust infers.
    pub(super) fn literal_against(
        &self,
        left: &syn::Expr,
        right: &syn::Expr,
    ) -> Result<Option<Ty>, Diag> {
        // Not only a bare literal: `bits ^ (1 << 63)` writes an operand built
        // entirely out of unsuffixed literals, and Rust gives the whole of it
        // the other side's type. Reading `1 << 63` as the literal's default
        // `i32` made it a `number` beside a `bigint`, which JavaScript refuses
        // to combine at all.
        fn unsuffixed(e: &syn::Expr) -> bool {
            match e {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => {
                    i.suffix().is_empty()
                }
                syn::Expr::Paren(p) => unsuffixed(&p.expr),
                syn::Expr::Group(g) => unsuffixed(&g.expr),
                syn::Expr::Unary(u) => {
                    matches!(u.op, syn::UnOp::Neg(_) | syn::UnOp::Not(_)) && unsuffixed(&u.expr)
                }
                // A shift's right operand has a type of its own, so only the
                // left one carries the whole expression's.
                syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Shl(_) | syn::BinOp::Shr(_)) => {
                    unsuffixed(&b.left)
                }
                syn::Expr::Binary(b) => unsuffixed(&b.left) && unsuffixed(&b.right),
                _ => false,
            }
        }
        let other = if unsuffixed(left) {
            right
        } else if unsuffixed(right) {
            left
        } else {
            return Ok(None);
        };
        let ty = self.resolve_expr(other)?;
        Ok(match ty.peel_refs() {
            Ty::Prim(prim) if prim.is_integer() => Some(Ty::Prim(*prim)),
            _ => None,
        })
    }

    /// What `base[index]` hands back: `<base as Index<I>>::Output`, where `I` is
    /// the type of the index expression.
    ///
    /// The std surface declares the whole family — `Index<I> for Vec<T>` and
    /// `for [T]` through `SliceIndex`, `Index<&Q> for HashMap<K, V>` through
    /// `Borrow` — so which of them applies, and whether the answer is an element
    /// or a slice, follows from the index's type rather than from a list of
    /// container names.
    #[cfg(test)]
    pub fn index_of(&self, base: &Ty, index_src: &str) -> Option<Ty> {
        let index: syn::Expr = syn::parse_str(index_src).expect("parses as an expression");
        self.index_result(base, &index)
    }

    pub(super) fn index_result(&self, base: &Ty, index: &syn::Expr) -> Option<Ty> {
        let index_ty = self.index_type(index)?;
        let trait_id = self.registry.system_type("std::ops::Index")?;
        // `HashMap` is indexed by a borrowed key, and the deref chain is what
        // walks a `Vec` down to the `[T]` its `Index` is written for.
        for candidate in std::iter::once(base.clone())
            .chain(self.probe().deref_chain(base).ok()?.into_iter().map(|s| s.to))
        {
            let found = self.project_with(
                &candidate,
                TraitRef {
                    id: trait_id,
                    args: vec![index_ty.clone()],
                    bindings: Vec::new(),
                },
                "Output",
            );
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// The type of the expression between the brackets.
    ///
    /// An unsuffixed integer literal is read as `usize` here. That is not a
    /// guess about inference: `SliceIndex` is implemented for `usize` and for
    /// ranges of `usize` and for nothing else numeric, so `usize` is the only
    /// type such a literal can have in this position. Where the index is
    /// anything else the ordinary rules answer, and a `HashMap` indexed by
    /// `&key` goes through `Borrow` like any other lookup.
    pub(super) fn index_type(&self, index: &syn::Expr) -> Option<Ty> {
        if super::shapes::is_unsuffixed_int(index) {
            return Some(Ty::Prim(Prim::Usize));
        }
        if let syn::Expr::Range(range) = index {
            // A range BETWEEN THE BRACKETS is a range of `usize`, for the same
            // reason a bare literal there is.
            return self.range_shape(range, &|e| self.index_type(e));
        }
        let found = self.resolve_expr(index).ok()?;
        // So is a `{integer}` that arrived from somewhere else, such as the
        // element of `for i in (0..16).rev()`: the position is what decides it,
        // and the constraint is what carries that back to where it was written.
        let Ty::Var(id) = &found else { return Some(found) };
        let integral = self.vars.borrow().kind_of(*id) == Some(crate::ty::VarKind::Integral);
        if !integral {
            return Some(found);
        }
        let usize_ty = Ty::Prim(Prim::Usize);
        self.constrain_here(syn::spanned::Spanned::span(index), &usize_ty, &found);
        Some(usize_ty)
    }

    /// `a..b` is a `Range<A>`, `a..` a `RangeFrom<A>`, `..b` a `RangeTo<A>`,
    /// `..` a `RangeFull` and `a..=b` a `RangeInclusive<A>`, each declared in
    /// `std::ops`.
    ///
    /// The endpoints are read by the ordinary rules, so an unsuffixed literal
    /// there is Rust's `{integer}` and the other endpoint's suffix decides it:
    /// reading the START as `usize` made `for i in 0..4u8` an iteration over
    /// `usize` and reported the `u8` the body then met as a contradiction.
    pub(super) fn range_type(&self, range: &syn::ExprRange) -> Option<Ty> {
        let endpoint = |e: &syn::Expr| self.resolve_expr(e).ok();
        // Both ends are one type, which is how the end's suffix settles the
        // start's literal and the other way round.
        if let (Some(start), Some(end)) = (&range.start, &range.end) {
            if let (Some(a), Some(b)) = (endpoint(start), endpoint(end)) {
                self.constrain_here(syn::spanned::Spanned::span(end), &a, &b);
            }
        }
        self.range_shape(range, &endpoint)
    }

    /// The `std::ops` range type a written range has, with each end read by
    /// `endpoint`.
    fn range_shape(
        &self,
        range: &syn::ExprRange,
        endpoint: &dyn Fn(&syn::Expr) -> Option<Ty>,
    ) -> Option<Ty> {
        let closed = matches!(range.limits, syn::RangeLimits::Closed(_));
        let (path, arg) = match (&range.start, &range.end) {
            (Some(start), Some(end)) if closed => (
                "std::ops::RangeInclusive",
                endpoint(start).or_else(|| endpoint(end)),
            ),
            (Some(start), Some(end)) => {
                ("std::ops::Range", endpoint(start).or_else(|| endpoint(end)))
            }
            (Some(start), None) => ("std::ops::RangeFrom", endpoint(start)),
            (None, Some(end)) if closed => ("std::ops::RangeToInclusive", endpoint(end)),
            (None, Some(end)) => ("std::ops::RangeTo", endpoint(end)),
            (None, None) => ("std::ops::RangeFull", None),
        };
        let id = self.registry.system_type(path)?;
        Some(Ty::Named {
            id,
            args: arg.into_iter().collect(),
        })
    }
}
