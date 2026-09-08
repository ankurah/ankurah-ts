//! Building a struct: the order its values are EVALUATED in, and the order
//! they are handed to the constructor.
//!
//! For: Rust evaluates a struct literal's fields in the order the LITERAL
//! writes them, and checks each value against the field it stands beside. A
//! ported struct is built through a constructor whose parameters are its fields
//! in DECLARATION order. The two orders are not the same, and a statement that
//! moves a value into one field and throws in another can tell them apart:
//! `Reordered { n: value.unwrap(), token }` evaluates the `unwrap` first in
//! Rust and moved `token` first here, so a `None` left the token owned by
//! nobody. So the values are evaluated into temporaries in SOURCE order and the
//! temporaries are handed over in declaration order.

use super::super::BodyTranslator;

impl BodyTranslator<'_> {
    /// `Rec { third: c, first: a }` as the call to `Rec`'s constructor.
    ///
    /// Rust checks each value against the FIELD it stands beside; a ported
    /// struct is built through a constructor whose parameters are its fields in
    /// DECLARATION order. Writing the values in the order the literal happened
    /// to name them handed `c` to `first` — silently, wherever the two fields
    /// have the same TypeScript type, which is exactly the case tsc cannot
    /// catch. `connectors/local-process/src/lib.rs:70` writes six fields in an
    /// order the constructor does not share, two of them `EntityId`.
    pub(crate) fn struct_literal(&self, s: &syn::ExprStruct, name: &str) -> String {
        // The declared fields, in the order the constructor takes them, and
        // the type of each where the engine has one. The two come from
        // different places on purpose: the ORDER covers every field, including
        // one whose type did not resolve, and a field left out of the call
        // would shift every argument after it.
        let typed = self.struct_field_types(s);
        let declared: Vec<(String, Option<crate::ty::Ty>)> = self
            .struct_field_order(s)
            .into_iter()
            .map(|name| {
                let ty = typed.iter().find(|(n, _)| *n == name).map(|(_, t)| t.clone());
                (name, ty)
            })
            .collect();
        // FF4: a field the declaration walks as a CURSOR is wrapped where the
        // value crosses into it, exactly as a cursor parameter is at a call.
        // Handed the sequence as it stood, `Holder { walk: t.into_iter() }`
        // emitted `new Holder([...tokens])` and `pull()`'s own
        // `this.walk.next()` named a method an array has not got.
        let walked = self.cursor_fields(s);
        let written = |f: &syn::FieldValue, ty: Option<&crate::ty::Ty>| {
            let value = self.expecting(&f.expr, ty, || self.moved_value(&f.expr));
            let member = crate::infer::member_name(&f.member);
            let wants = walked.iter().find(|(name, _)| *name == member).map(|(_, e)| *e);
            self.adapted_to_a_cursor(&f.expr, wants, value)
        };
        if declared.is_empty() {
            // The engine could not name the struct, so it cannot say what order
            // the constructor takes. Source order is what it has; the site says
            // the order is a guess.
            let values: Vec<String> = s
                .fields
                .iter()
                .map(|f| written(f, None))
                .collect();
            if s.fields.len() > 1 {
                self.fallback(
                    syn::spanned::Spanned::span(s),
                    format!(
                        "`{}` is built here and the engine could not name its declaration, \
                         so the values are handed to the constructor in the order the literal \
                         writes them rather than in the order the fields are declared",
                        name
                    ),
                );
            }
            return format!("new {}({})", name, values.join(", "));
        }
        // `..rest` fills every field the literal does not name from another
        // value of the same type. Nothing here reads it, so the fields it would
        // have filled would be `undefined`.
        if let Some(rest) = &s.rest {
            self.fallback(
                syn::spanned::Spanned::span(rest),
                format!(
                    "`..` fills the fields `{}` does not name from another value, and the port \
                     has no writing for it, so those fields are left undefined",
                    name
                ),
            );
        }
        // The values, EVALUATED in the order the LITERAL writes them, which is
        // the order Rust evaluates them in — each paired with the constructor
        // parameter it will be handed to. Translated in declaration order
        // instead, `Reordered { n: value.unwrap(), token }` wrote
        // `new Reordered(token, (value ?? throw))`: the token was handed over
        // before the `unwrap` Rust runs first, so a `None` left it owned by
        // nobody.
        let mut at: Vec<usize> = Vec::new();
        let mut evaluated: Vec<String> = Vec::new();
        // The expression behind each written value, in the same order, so the
        // move-flag placement can read it (E10).
        let mut behind: Vec<Option<&syn::Expr>> = Vec::new();
        for f in &s.fields {
            let member = crate::infer::member_name(&f.member);
            let Some(index) = declared.iter().position(|(name, _)| *name == member) else {
                continue;
            };
            at.push(index);
            evaluated.push(written(f, declared[index].1.as_ref()));
            behind.push(Some(&f.expr));
        }
        // A field the literal names and the declaration does not: the engine
        // resolved the literal to the wrong type, or the source does not
        // compile. Either way it must not vanish.
        for f in &s.fields {
            let member = crate::infer::member_name(&f.member);
            if !declared.iter().any(|(name, _)| *name == member) {
                self.fallback(
                    syn::spanned::Spanned::span(f),
                    format!(
                        "`{}` is not a field of `{}` as the engine read it, so the value \
                         written for it reaches no constructor parameter",
                        member, name
                    ),
                );
            }
        }
        // E10/J3: a constructor is a call, and the statement's move flag stands
        // after everything it evaluates. The plan is made in EVALUATION order,
        // so a lift lands where Rust ran the expression.
        let whole = syn::Expr::Struct(s.clone());
        let lifted = self.lifted_above_the_flag(&whole, &behind, evaluated);
        // Named by neither the literal nor anything else: only `..rest` can
        // produce this, and the line above says so.
        let mut values: Vec<String> = vec!["undefined".to_string(); declared.len()];
        for (index, text) in at.into_iter().zip(lifted) {
            values[index] = text;
        }
        format!("new {}({})", name, values.join(", "))
    }
}
