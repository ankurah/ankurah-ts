//! What a `move` closure owns, and who releases it.
//!
//! A Rust `move` closure takes its captures by value and drops them when the
//! closure itself is dropped — a listener holding an `Arc` keeps that `Arc`
//! alive for exactly as long as the listener lives. A JavaScript closure
//! captures the same values and the cascade cannot see any of them: it walks
//! own properties, and a capture is not a property. So every `move` closure
//! over a droppable value used to be a leak with nothing left that could
//! release it.
//!
//! `OwnedClosure(captures, fn)` is the runtime's answer: the captures become
//! ordinary owned fields, and dropping the closure cascades into them. It is
//! invoked as `closure.call(...)`, never as a bare call, so a call after the
//! drop is caught rather than reaching a body whose captures are gone.

use crate::ownership::Owned;
use crate::body::{indent, BodyTranslator};
use crate::ownership;

/// Where the closure's value goes, which decides who releases its captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// `(move || …)()` — created and finished in one expression, so the
    /// captures are released inside it and no runtime object is needed.
    Immediate,
    /// `let listener = move || …` — the local owns it, and the block that
    /// declared the local releases it.
    Bound,
    /// Anywhere else: an argument, a struct field, a return value. The closure
    /// is still an `OwnedClosure`, but the emitter cannot see who calls it, so
    /// the site is reported.
    Loose,
}

/// The immediately-invoked form: the body runs inside a scope that releases
/// what the closure captured, however the body is left.
///
/// This is what Rust does with a temporary closure — it is created, called, and
/// dropped in one expression, and dropping it drops the captures.
pub fn immediate(params: &str, body: &str, captures: &[Owned]) -> String {
    let mut inner = body.to_string();
    for capture in captures.iter().rev() {
        inner = crate::ownership::wrap(&inner, capture);
    }
    format!("({}) => {{\n{}}}", params, crate::body::indent(&inner))
}

/// The persistent form: the captures are listed beside the body that closes
/// over them, and from there they are the closure's own fields.
pub fn owned(captures: &[String], arrow: &str) -> String {
    format!("new OwnedClosure([{}], {})", captures.join(", "), arrow)
}

/// What to say about a closure whose call sites the emitter cannot see.
pub fn loose_report(captures: &[String]) -> String {
    format!(
        "this closure owns {} and is written as an `OwnedClosure`, so it is invoked as \
         `.call(...)` and released by whoever holds it; the emitter cannot see this call site \
         and has not rewritten it",
        captures
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl<'a> BodyTranslator<'a> {
    /// A closure, with what a `move` took by value released where Rust
    /// releases it.
    ///
    /// A closure that captures nothing droppable stays a plain arrow function:
    /// there is nothing for the cascade to find, and wrapping it would only add
    /// a drop the emitter then has to place. One that does capture something
    /// becomes an `OwnedClosure`, except where it is called on the spot — there
    /// the captures are released inside the closure's own body, because Rust
    /// creates, calls and drops it in the one expression.
    pub(crate) fn closure(
        &self,
        closure: &syn::ExprClosure,
        placement: ownership::closures::Placement,
        expected: Option<&crate::ty::Ty>,
    ) -> String {
        use ownership::closures::Placement;
        let params: Vec<String> = closure.inputs.iter().map(Self::pat_static).collect();
        let params = params.join(", ");
        let captures = if closure.capture.is_some() {
            self.owned_captures(closure)
        } else {
            Vec::new()
        };
        // The closure's own scope, with its parameters bound to whatever the
        // position it stands in says they are (spec 4.5). Without them the body
        // names values nothing has typed, and every call inside it falls back.
        //
        // It is a closure scope rather than a block because a `let` names a new
        // variable only within the scope it stands in: `|stack| { let stack =
        // stack.borrow_mut(); .. }` shadows the closure's own parameter, which
        // JavaScript refuses to declare twice, so the second one is emitted
        // under a fresh identifier. `redeclares` stops its walk at the nearest
        // function or closure, and a block does not stop it — pushed as one,
        // the `let` came out reading the binding it was declaring.
        self.push_closure_scope(Vec::new());
        self.bind_closure_params(closure, expected);
        // A block body carries its own braces; an expression body is the
        // closure's value, and a guard lifted out of it belongs inside the
        // arrow function, because its declaration names the closure's own
        // parameters and those do not exist outside.
        let (statements, arrow) = match &*closure.body {
            syn::Expr::Block(block) => {
                let body = self.translate_block(&block.block);
                (body.clone(), format!("({}) => {{\n{}}}", params, indent(&body)))
            }
            _ => {
                let (body, lifted) = self.with_own_hoists(|| self.expr_value(&closure.body));
                let inner = Self::arrow_body(&body, &lifted);
                let arrow = if !lifted.is_empty() {
                    format!("({}) => {{\n{}}}", params, indent(&inner))
                } else if body.starts_with("if ")
                    || body.starts_with("for ")
                    || body.starts_with("while ")
                    || body.starts_with('{')
                {
                    format!("({}) => {{\n  {}\n}}", params, body)
                } else {
                    format!("({}) => {}", params, body)
                };
                (inner, arrow)
            }
        };
        self.pop_scope();
        if captures.is_empty() {
            return arrow;
        }
        let owned: Vec<ownership::Owned> = captures
            .iter()
            .map(|(name, drops)| ownership::Owned {
                name: name.clone(),
                source: None,
                drops: *drops,
                flag: None,
                statement_scoped: false,
            })
            .collect();
        let names: Vec<String> = captures.iter().map(|(name, _)| name.clone()).collect();
        match placement {
            Placement::Immediate => ownership::closures::immediate(&params, &statements, &owned),
            Placement::Bound => ownership::closures::owned(&names, &arrow),
            Placement::Loose => {
                self.fallback(
                    syn::spanned::Spanned::span(closure),
                    ownership::closures::loose_report(&names),
                );
                ownership::closures::owned(&names, &arrow)
            }
        }
    }

    /// Bind a closure's parameters in the scope its body is translated in.
    ///
    /// A parameter the engine could type is bound to that type; one nothing
    /// typed is bound all the same, without one, so that the body reads it as a
    /// name that exists rather than as a name nobody declared — and the gap is
    /// reported once, here, instead of at every use inside the body.
    pub(crate) fn bind_closure_params(
        &self,
        closure: &syn::ExprClosure,
        expected: Option<&crate::ty::Ty>,
    ) {
        let Some(tc) = &self.types else { return };
        let sig = tc.borrow().closure_signature(closure, expected);
        for (name, ty) in &sig.params {
            match ty {
                Some(ty) => self.bind_var(name, ty.clone()),
                None => self.bind_untyped(name),
            }
        }
        let untyped = sig.untyped_params();
        if !untyped.is_empty() {
            self.fallback(
                syn::spanned::Spanned::span(closure),
                format!(
                    "this closure's parameter{} {} typed by nothing the engine can read: \
                     neither an annotation on the closure nor the position it stands in says \
                     what {} hold{}",
                    if untyped.len() == 1 { " is" } else { "s are" },
                    untyped
                        .iter()
                        .map(|n| format!("`{}`", n))
                        .collect::<Vec<_>>()
                        .join(", "),
                    if untyped.len() == 1 { "it" } else { "they" },
                    if untyped.len() == 1 { "s" } else { "" },
                ),
            );
        }
    }

    /// What releasing this body's receiver costs, where the closure being
    /// written is what took it.
    ///
    /// Only a method that took `self` by value has a receiver to hand over, and
    /// only one whose type has drop glue owes anything for it.
    fn receiver_the_closure_took(&self) -> Option<ownership::Drops> {
        if !self.owns_self {
            return None;
        }
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let ty = tc.self_ty.clone()?;
        let drops = ownership::drops_of(&tc.probe(), &ty);
        drops.is_droppable().then_some(drops)
    }

    /// The captures a `move` closure took by value that owe a release, with
    /// what each of them costs.
    pub(crate) fn owned_captures(&self, closure: &syn::ExprClosure) -> Vec<(String, ownership::Drops)> {
        let Some(tc) = &self.types else {
            return Vec::new();
        };
        let scan = ownership::Scan::new(self);
        let mut out: Vec<(String, ownership::Drops)> = Vec::new();
        for site in scan.captures(closure) {
            if out.iter().any(|(name, _)| *name == site.name) {
                continue;
            }
            // The receiver is a capture like any other where the method took it
            // by value: `fn callback(self) -> impl Fn()` hands the receiver to
            // the closure, and Rust drops it when the closure is dropped. A
            // plain arrow function has no field the cascade could reach it
            // through, so the receiver had nothing that could ever release it.
            // A `&self` method lends its receiver and hands over nothing.
            if site.name == self.self_name {
                if let Some(drops) = self.receiver_the_closure_took() {
                    out.push((self.self_name.to_string(), drops));
                }
                continue;
            }
            let Some(ty) = tc.borrow().lookup(&site.name) else {
                // A local the closure names and the engine could not type is
                // one nobody can say the closure owns. Listing the rest as
                // captures without it would say the closure owns exactly those,
                // which is a claim the emitter cannot make.
                if tc.borrow().is_bound(&site.name) {
                    self.fallback(
                        site.span,
                        format!(
                            "this closure takes `{}` by value and the engine could not type \
                             it, so it is left out of what the closure owns",
                            site.name
                        ),
                    );
                }
                continue;
            };
            let drops = ownership::drops_of(&tc.borrow().probe(), &ty);
            if drops.is_droppable() {
                // A shadow is emitted under a fresh identifier, and the capture
                // list has to name the value the body closes over rather than
                // the name Rust wrote for it.
                let emitted = self.emitted_name(&site.name).unwrap_or_else(|| site.name.clone());
                out.push((emitted, drops));
            }
        }
        out
    }
}
