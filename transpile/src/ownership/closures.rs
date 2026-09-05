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
///
/// `consumes` says whether the body hands one of them away — Rust's `FnOnce`.
/// The runtime needs it because `invoke` cannot read a body: a closure whose
/// body takes the captures is called with `callOnce`, and one that only reads
/// them is called and then dropped, which is where its captures' glue runs.
pub fn owned(captures: &[String], arrow: &str, consumes: bool) -> String {
    let marker = if consumes { ", undefined, true" } else { "" };
    format!("new OwnedClosure([{}], {}{})", captures.join(", "), arrow, marker)
}

/// What to say about a closure whose call sites the emitter cannot see.
///
/// R10 settles the emitted half: every emitted callee whose parameter carries a
/// callable bound invokes through base's `invoke`, and base's own
/// closure-taking methods accept either shape. What is left is a callee written
/// by hand, which the emitter cannot reach and cannot check.
pub fn loose_report(captures: &[String]) -> String {
    format!(
        "this closure owns {} and is written as an `OwnedClosure`, which is not a bare \
         callable; every emitted callee and `@ankurah/base` invoke one through `invoke`, but \
         the emitter cannot see THIS call site, so a hand-written callee has to do the same",
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
        // Rust's `|_, _|` says the closure ignores both arguments. TypeScript
        // has no such spelling: `_` there is a parameter *called* `_`, and two
        // of them are a duplicate parameter name a JavaScript engine refuses
        // (`core/src/property/backend/yjs.ts` was one). A run of underscores is
        // the port's spelling for the nth ignored parameter — a name nothing
        // emitted can collide with, and one a reader of TypeScript already
        // reads as "unused".
        let mut ignored = 0usize;
        let mut ignored_names: Vec<String> = Vec::new();
        let params: Vec<String> = closure
            .inputs
            .iter()
            .map(|pat| {
                if Self::binds_nothing(pat) {
                    ignored += 1;
                    let name = "_".repeat(ignored);
                    ignored_names.push(name.clone());
                    name
                } else {
                    Self::pat_static(pat)
                }
            })
            .collect();
        // A `move` closure captures everything it names; one written without
        // `move` captures by value only what its body hands away, which Rust
        // infers per capture. Both own what they took.
        let captures = self.owned_captures(closure);
        // An `OwnedClosure` is a VALUE, not an arrow in a call position, so
        // TypeScript has no call site to take the parameter types from:
        // `new OwnedClosure([t], (n) => n + 1)` infers `n: unknown` and every
        // use of `n` in the body is an error. It typechecked only while the
        // wrapper was written in an immediately-called position — which was
        // itself the `TypeError` R10 exists to stop. So a wrapped closure
        // writes the parameter types the engine already resolved.
        let params = match captures.is_empty() {
            true => params.join(", "),
            false => self.annotated_params(closure, expected, &params, &ignored_names),
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
        // `async |..| { .. }` and `async move |..| { .. }` are closures whose
        // body is a future. `ExprClosure::asyncness` was never read, so the
        // arrow came out without `async` and the `await` inside it was a parse
        // error — bun refuses the file.
        let is_async = closure.asyncness.is_some();
        let arrow_head = if is_async { "async " } else { "" };
        // A `return` inside a closure returns from the closure in Rust too, so
        // the sentinel an enclosing lifted body is collecting stops here.
        let (statements, arrow) = match &*closure.body {
            syn::Expr::Block(block) => {
                let body = self.inside_its_own_function(|| self.translate_block(&block.block));
                (
                    body.clone(),
                    format!("{}({}) => {{\n{}}}", arrow_head, params, indent(&body)),
                )
            }
            _ => {
                let (body, lifted) = self.with_own_hoists(|| {
                    self.inside_its_own_function(|| self.expr_value(&closure.body))
                });
                let inner = Self::arrow_body(&body, &lifted);
                let arrow = if !lifted.is_empty() {
                    format!("{}({}) => {{\n{}}}", arrow_head, params, indent(&inner))
                } else if body.starts_with("if ")
                    || body.starts_with("for ")
                    || body.starts_with("while ")
                    || body.starts_with('{')
                {
                    format!("{}({}) => {{\n  {}\n}}", arrow_head, params, body)
                } else {
                    format!("{}({}) => {}", arrow_head, params, body)
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
        let consumes = self.hands_a_capture_away(closure);
        match placement {
            Placement::Immediate => ownership::closures::immediate(&params, &statements, &owned),
            Placement::Bound => ownership::closures::owned(&names, &arrow, consumes),
            Placement::Loose => {
                self.fallback(
                    syn::spanned::Spanned::span(closure),
                    ownership::closures::loose_report(&names),
                );
                ownership::closures::owned(&names, &arrow, consumes)
            }
        }
    }

    /// A wrapped closure's parameter list, with each parameter's type written.
    ///
    /// The names come from the caller, which has already decided how an ignored
    /// parameter is spelled; the types come from the same signature
    /// `bind_closure_params` binds the body against, paired by POSITION because
    /// an ignored parameter's emitted name is not its Rust one. A parameter the
    /// engine could not type is written bare — `bind_closure_params` reports it.
    fn annotated_params(
        &self,
        closure: &syn::ExprClosure,
        expected: Option<&crate::ty::Ty>,
        names: &[String],
        ignored: &[String],
    ) -> String {
        let Some(tc) = &self.types else { return names.join(", ") };
        let tc = tc.borrow();
        let sig = tc.closure_signature(closure, expected);
        names
            .iter()
            .enumerate()
            .map(|(at, name)| {
                // A parameter the closure takes no name out of is read nowhere
                // in the body, so nothing needs its type — and writing one for
                // a Rust `()` would put `void` in a parameter position, which
                // says something narrower than the port means.
                if ignored.contains(name) {
                    return name.clone();
                }
                match sig.params.get(at).and_then(|(_, ty)| ty.as_ref()) {
                    Some(ty) => format!("{}: {}", name, crate::name_map::map_ty(tc.registry, ty)),
                    None => name.clone(),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
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
        // What the body hands away. Rust moves those into the body when the
        // closure runs, so the closure no longer has them to drop — and a
        // closure that does so IS an `FnOnce`, which the runtime now has a call
        // for: `callOnce` marks the closure moved and transfers the captures
        // before the body runs, so listing a consumed capture is right and the
        // second drop the note below feared cannot happen. Where the closure is
        // never called, its `drop()` releases them, which is Rust's own answer.
        let mut out: Vec<(String, ownership::Drops)> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let names = match closure.capture.is_some() {
            true => scan.captures(closure),
            false => scan.moved_captures(closure),
        };
        for site in names {
            // One name, one answer: a value handed away in a tail position is
            // recorded by both the walk and the tail scan, and each capture is
            // decided once whichever way it was reached.
            if seen.iter().any(|name| *name == site.name) {
                continue;
            }
            seen.push(site.name.clone());
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
