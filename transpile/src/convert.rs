//! Conversions: `?` across two error types, and the `into`/`from` family.
//!
//! For: Rust converts silently at these places and TypeScript does not. `?`
//! calls `From::from` on the error where the two error types differ, `.into()`
//! calls it on the value, and a port that hands the value on unchanged is
//! passing one type where another was declared — a wrong value at run time and
//! a type error at compile time.
//!
//! What each of these needs is the same three answers: what the value is now,
//! what the position wants it to be, and which impl gets from one to the other.
//! The first two come from the engine, the third from the impl table, and the
//! call is written by `emit_impls::conversion`, so the name matches the method
//! emission gave it.

use crate::body::{BodyTranslator, Lowered};
use crate::registry::convert::FROM_PATH;
use crate::registry::NoConversion;
use crate::ty::Ty;

pub(crate) mod cast;

#[cfg(test)]
mod tests;

impl BodyTranslator<'_> {
    /// The function `?` calls on the error, where the two error types differ.
    ///
    /// `None` means nothing is written around the error: either the types
    /// agree, or the conversion could not be resolved and the site has said so.
    pub(crate) fn try_conversion(
        &self,
        operand: Option<&Ty>,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let (operand, returns) = (operand?, self.fn_return.as_ref()?);
        let (from, to) = (error_of(operand)?, error_of(returns)?);
        if from == to || same_projection(&from, &to) {
            return None;
        }
        let written = self.conversion_callee(&from, &to, span, "`?` converts the error");
        if let Some(tc) = &self.types {
            let tc = tc.borrow();
            crate::trace::record_try(
                tc.registry,
                &tc.sink.file(),
                span,
                &from,
                &to,
                written.as_deref(),
            );
        }
        written
    }

    /// The function that converts `from` into `to`, or the reason the site is
    /// left as it stands.
    ///
    /// `what` opens the diagnostic and says which construct asked, so a reader
    /// of the run's output can tell a `?` from an `.into()` without the span.
    pub(crate) fn conversion_callee(
        &self,
        from: &Ty,
        to: &Ty,
        span: proc_macro2::Span,
        what: &str,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let reg = tc.registry;
        let describe = |ty: &Ty| reg.describe(ty);
        // A conversion whose source or target is still a type parameter is one
        // Rust decides per instantiation, from the bound the parameter carries.
        // A single emitted body stands for every instantiation and cannot pick
        // among them — the same fact the open-bound dispatch note records for
        // method calls — so the site says so rather than reporting a missing
        // impl that is not missing.
        if from.has_open_param() || to.has_open_param() {
            self.fallback(
                span,
                format!(
                    "{} from `{}` to `{}` through `From`, and the conversion is fixed by a \
                     bound on a type parameter, so which impl runs is decided per \
                     instantiation and one emitted body cannot say; the value is handed on \
                     unconverted",
                    what,
                    describe(from),
                    describe(to),
                ),
            );
            return None;
        }
        let found = match tc.probe().from_impl(from, to) {
            Ok(found) => found,
            Err(why) => {
                self.fallback(
                    span,
                    format!(
                        "{} from `{}` to `{}` through `From`, and {}; the value is handed on \
                         unconverted",
                        what,
                        describe(from),
                        describe(to),
                        match why {
                            NoConversion::NoTrait => "`From` is not declared".to_string(),
                            NoConversion::None => {
                                "no impl in the table performs it".to_string()
                            }
                            NoConversion::Ambiguous(ids) => format!(
                                "{} impls in the table perform it, which rustc would have \
                                 rejected",
                                ids.len()
                            ),
                        }
                    ),
                );
                return None;
            }
        };
        let call = match crate::emit_impls::conversion_call(reg, found.impl_id, to) {
            Ok(call) => call,
            Err(why) => {
                self.fallback(
                    span,
                    format!(
                        "{} from `{}` to `{}` through `From`, and {}; the value is handed on \
                         unconverted",
                        what,
                        describe(from),
                        describe(to),
                        why
                    ),
                );
                return None;
            }
        };
        // Emission hangs every `From` impl for one target on that target's
        // class and keeps the first of any two that would take the same name,
        // so a call to a name two impls share cannot be trusted to arrive at
        // the one the engine picked.
        let names = crate::emit_impls::conversion_names(reg, to, FROM_PATH);
        let method = call.callee.rsplit('.').next().unwrap_or(&call.callee);
        if names.iter().filter(|n| *n == method).count() > 1 {
            self.fallback(
                span,
                format!(
                    "{} from `{}` to `{}` through `From`, and more than one `From` impl for \
                     `{}` is emitted as `{}`, so the call could not say which; the value is \
                     handed on unconverted",
                    what,
                    describe(from),
                    describe(to),
                    describe(to),
                    method
                ),
            );
            return None;
        }
        Some(call.callee)
    }
}

/// Are these two the same projection written two ways?
///
/// `D::Error` and `<D as Deserializer<'de>>::Error` name one type: the
/// unqualified form means "the one `Error` reachable on `D`", so it is whatever
/// the qualified one is. The engine models no lifetimes, so the two qualified
/// spellings `Deserializer<'_>` and `Deserializer<'de>` are one too. Reading
/// them as two made `serde`'s `deserialize` look like a `?` that converts.
fn same_projection(a: &Ty, b: &Ty) -> bool {
    let (
        Ty::Assoc { base: x, trait_: xt, name: xn },
        Ty::Assoc { base: y, trait_: yt, name: yn },
    ) = (a.peel_refs(), b.peel_refs())
    else {
        return false;
    };
    xn == yn && x == y && (xt.is_none() || yt.is_none() || xt == yt)
}

/// The `E` of a `Result<T, E>`, which is what `?` converts.
fn error_of(ty: &Ty) -> Option<Ty> {
    match ty.peel_refs() {
        Ty::Named { args, .. } if args.len() == 2 => Some(args[1].clone()),
        _ => None,
    }
}

/// The methods that name a conversion and take no arguments.
const CONVERSION_METHODS: [&str; 4] = ["into", "try_into", "to_string", "to_owned"];

impl BodyTranslator<'_> {
    /// Is this call one of the conversion family, and if so what does the port
    /// write for it?
    ///
    /// `x.into()` was emitted as a call to a method called `into`, which no
    /// emitted class and no runtime type has: 27 sites in core alone named a
    /// function that is not there. What the port writes instead is the impl
    /// Rust selects — a static on the target's class where the corpus wrote the
    /// impl, the value itself where the two types are one in TypeScript — or a
    /// diagnostic saying which of those it could not decide.
    pub(crate) fn conversion_method(
        &self,
        call: &syn::ExprMethodCall,
        receiver: &str,
        expected: Option<&Ty>,
    ) -> Option<String> {
        let method = call.method.to_string();
        if !CONVERSION_METHODS.contains(&method.as_str()) || !call.args.is_empty() {
            return None;
        }
        let span = syn::spanned::Spanned::span(call);
        let from = self.quietly(|| self.resolve_expr_type(&call.receiver)).ok();
        match method.as_str() {
            "to_string" | "to_owned" => self.owned_copy(&method, from.as_ref(), receiver, span),
            "into" => Some(self.into_call(from.as_ref(), expected, receiver, span)),
            // `try_into` hands back a `Result`, so a site the engine cannot
            // settle keeps the shape it had rather than being handed a value
            // where the source reads a wrapper.
            _ => self.try_into_call(from.as_ref(), expected, receiver, span),
        }
    }

    /// `to_string()` and `to_owned()`: a value of the receiver's own type,
    /// owned rather than borrowed.
    ///
    /// The port maps `String` and `&str` to one type, so `s.to_string()` on a
    /// string is the string — `'Alice'.toString()` was a call whose only effect
    /// was to be there. Everything else keeps a real copy: `to_string` through
    /// `Display`, `to_owned` through `Clone`.
    fn owned_copy(
        &self,
        method: &str,
        from: Option<&Ty>,
        receiver: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let Some(from) = from else {
            self.fallback(
                span,
                format!(
                    "`{}` is written on a receiver the engine could not type, so the copy is \
                     written by the method's name alone",
                    method
                ),
            );
            // `to_owned` on an untyped receiver keeps the `clone` it was
            // written as before this: a string is the one type it is wrong for,
            // and a string is what the engine names most reliably.
            return (method == "to_owned").then(|| format!("{}.clone()", receiver));
        };
        use crate::name_map::shape::{js_shape, JsShape};
        // A reference is erased in emission, so `&str` and `String` are one
        // question here and `&Vec<u8>` and `Vec<u8>` are another.
        match js_shape(tc.registry, from.peel_refs()) {
            JsShape::Str => Some(receiver.to_string()),
            // A number and a bigint each carry `toString`, and a class that
            // implements `Display` is emitted with one, so the ordinary method
            // call is what `to_string` wants.
            _ if method == "to_string" => None,
            // A number, a bigint and a boolean are copied by being read: there
            // is nothing to clone and no `clone` on them to call, and
            // `n.clone()` was a TypeError at run time.
            JsShape::Number | JsShape::BigInt | JsShape::Boolean => Some(receiver.to_string()),
            JsShape::Bytes => Some(format!("{}.slice()", receiver)),
            JsShape::Array(_) => Some(format!("[...{}]", receiver)),
            // `ToOwned` for everything else in the corpus is `Clone`, and the
            // emitted class carries `clone`.
            _ => Some(format!("{}.clone()", receiver)),
        }
    }

    /// `x.into()`, written as the conversion the port performs.
    fn into_call(
        &self,
        from: Option<&Ty>,
        expected: Option<&Ty>,
        receiver: &str,
        span: proc_macro2::Span,
    ) -> String {
        let (Some(from), Some(to)) = (from, expected) else {
            self.fallback(
                span,
                "`.into()` has no expected type here, so what it converts to is not said and \
                 the value stands as it is",
            );
            return receiver.to_string();
        };
        match self.conversion_text(from, to, receiver, span, "`.into()` converts") {
            Some(text) => text,
            None => receiver.to_string(),
        }
    }

    /// `x.try_into()`, whose value is a `Result` the caller then tests.
    fn try_into_call(
        &self,
        from: Option<&Ty>,
        expected: Option<&Ty>,
        receiver: &str,
        span: proc_macro2::Span,
    ) -> Option<String> {
        let to = expected.and_then(|ty| self.result_payload(ty))?;
        let from = from?;
        let tc = self.types.as_ref()?;
        let found = tc.borrow().probe().try_from_impl(from, &to).ok()?;
        let call = {
            let tc = tc.borrow();
            crate::emit_impls::conversion_call(tc.registry, found.impl_id, &to)
        };
        match call {
            Ok(call) => Some(format!("{}({})", call.callee, receiver)),
            Err(why) => {
                let described = {
                    let tc = tc.borrow();
                    (tc.registry.describe(from), tc.registry.describe(&to))
                };
                self.fallback(
                    span,
                    format!(
                        "`.try_into()` converts `{}` to `{}` through `TryFrom`, and {}; the \
                         call is written by the method's name alone",
                        described.0, described.1, why
                    ),
                );
                None
            }
        }
    }

    /// The `T` of a `Result<T, E>` an expectation names.
    pub(crate) fn result_payload(&self, ty: &Ty) -> Option<Ty> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        if !tc.is_result(ty) {
            return None;
        }
        match ty.peel_refs() {
            Ty::Named { args, .. } => args.first().cloned(),
            _ => None,
        }
    }

    /// The text a conversion from one type to another is written as, or `None`
    /// where the site has said why it could not be written.
    ///
    /// Three answers in order: the two types are one in TypeScript and nothing
    /// is written; the corpus wrote the impl and its emitted function is
    /// called; the declared surface wrote it, so the conversion is the
    /// runtime's own and the native-type table says how the port spells it.
    pub(crate) fn conversion_text(
        &self,
        from: &Ty,
        to: &Ty,
        value: &str,
        span: proc_macro2::Span,
        what: &str,
    ) -> Option<String> {
        if from.peel_refs() == to.peel_refs() {
            return Some(value.to_string());
        }
        let tc = self.types.as_ref()?;
        {
            let tc = tc.borrow();
            if let Some(text) =
                crate::native_types::conversion::between(tc.registry, from, to, value)
            {
                return Some(text);
            }
        }
        let callee = self.conversion_callee(from, to, span, what)?;
        Some(format!("{}({})", callee, value))
    }
}

impl BodyTranslator<'_> {
    /// `x as T`, written as the value conversion Rust means by it.
    ///
    /// TypeScript's `as` asserts a type and changes no value, so emitting one
    /// for the other left `n as bigint` where a `number` stood: the wrong value
    /// and a type error at once.
    pub(crate) fn cast(&self, cast: &syn::ExprCast) -> String {
        let value = self.expr(&cast.expr);
        let span = syn::spanned::Spanned::span(cast);
        let say = |why: String| {
            self.fallback(span, why);
            value.clone()
        };
        let Some(tc) = &self.types else {
            return say("`as` is written with no type context, so the conversion it \
                        performs is not known here"
                .to_string());
        };
        let to = match tc.borrow().resolve_written_type(&cast.ty) {
            Ok(ty) => ty,
            Err(diag) => return say(format!("{}; `as` writes the value unchanged", diag.message)),
        };
        let Ok(from) = self.quietly(|| self.resolve_expr_type(&cast.expr)) else {
            return say(
                "`as` is written on a value the engine could not type, so which conversion it \
                 performs is not known; the value is written unchanged"
                    .to_string(),
            );
        };
        let (Ty::Prim(from_prim), Ty::Prim(to_prim)) = (from.peel_refs(), to.peel_refs()) else {
            let described = {
                let tc = tc.borrow();
                (tc.registry.describe(&from), tc.registry.describe(&to))
            };
            return say(format!(
                "`as` converts `{}` to `{}`, which is not a conversion between two numbers; \
                 the value is written unchanged",
                described.0, described.1
            ));
        };
        match cast::numeric(*from_prim, *to_prim, &value) {
            Some(text) => text,
            None => {
                let described = {
                    let tc = tc.borrow();
                    (tc.registry.describe(&from), tc.registry.describe(&to))
                };
                say(format!(
                    "`as` converts `{}` to `{}`, which the port has no spelling for; the value \
                     is written unchanged",
                    described.0, described.1
                ))
            }
        }
    }
}

impl BodyTranslator<'_> {
    /// `Target::from(x)` and `Target::try_from(x)`, written as the function the
    /// impl they name was emitted as.
    ///
    /// `Name::from(tag)` is `Name.fromTag(tag)` where two `From` impls for
    /// `Name` would otherwise share a name, and `Name.from(tag)` named a static
    /// the class does not declare.
    pub(crate) fn conversion_call(
        &self,
        callee: Option<&syn::Path>,
        call: &syn::ExprCall,
        args: &[String],
    ) -> Option<String> {
        let callee = callee?;
        let [value] = args else { return None };
        let [argument] = &call.args.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        let method = callee.segments.last()?.ident.to_string();
        let trait_path = match method.as_str() {
            "from" => FROM_PATH,
            "try_from" => crate::registry::convert::TRY_FROM_PATH,
            _ => return None,
        };
        let owner: Vec<String> = callee
            .segments
            .iter()
            .take(callee.segments.len() - 1)
            .map(|s| s.ident.to_string())
            .collect();
        if owner.is_empty() {
            return None;
        }
        let tc = self.types.as_ref()?;
        let to = {
            let tc = tc.borrow();
            let crate::registry::Def::Type(id) =
                tc.registry.lookup_type(tc.module, &owner).ok()??
            else {
                return None;
            };
            crate::ty::Ty::Named {
                id,
                args: Vec::new(),
            }
        };
        // The argument's own type is what says which impl this is; a `from`
        // whose argument the engine could not name is left as it was written.
        let from = self.quietly(|| self.resolve_expr_type(argument)).ok()?;
        let span = syn::spanned::Spanned::span(call);
        // `Wrapped::from(w)` where `w` is already a `Wrapped` is the value.
        // Rust's reflexive `impl<T> From<T> for T` is a real impl and there is
        // no method for it to land on.
        if from.peel_refs() == &to {
            return Some(value.to_string());
        }
        // A conversion the RUNTIME performs — `String::from(s)`, `u64::from(n)`
        // — has no emitted class to hang a static on. Looking only in the impl
        // table found the surface's impl, failed to write a call for it, and
        // fell through to `String.from(s)`, which is not a function.
        {
            let tc = tc.borrow();
            if let Some(written) = crate::native_types::conversion::between(tc.registry, &from, &to, value) {
                return Some(written);
            }
        }
        let describe = |ty: &crate::ty::Ty| self.types.as_ref().map(|tc| tc.borrow().registry.describe(ty)).unwrap_or_default();
        let found = {
            let tc = tc.borrow();
            tc.probe().conversion_impl(trait_path, &from, &to)
        };
        let found = match found {
            Ok(found) => found,
            Err(why) => {
                self.fallback(
                    span,
                    format!(
                        "`{}::{}` converts a `{}` to a `{}`, and {}; the value is written as it \
                         stands",
                        owner.join("::"),
                        method,
                        describe(&from),
                        describe(&to),
                        why
                    ),
                );
                return None;
            }
        };
        let call = {
            let tc = tc.borrow();
            crate::emit_impls::conversion_call(tc.registry, found.impl_id, &to)
        };
        match call {
            Ok(call) => Some(format!("{}({})", call.callee, value)),
            Err(why) => {
                self.fallback(
                    span,
                    format!(
                        "`{}::{}` converts a `{}` to a `{}`, and the impl that performs it {}; \
                         the value is written as it stands",
                        owner.join("::"),
                        method,
                        describe(&from),
                        describe(&to),
                        why
                    ),
                );
                None
            }
        }
    }
}

/// `?`, and the questions the constructs around it ask about a wrapper.
impl BodyTranslator<'_> {
    /// `e?` — take the value out, or leave with the error.
    ///
    /// The test and the early exit are statements, so they are lifted into the
    /// prelude and the expression becomes the name they left behind. That is
    /// what makes `f(g()?)` work: `g()` is asked once, its error leaves the
    /// function, and `f` is called with the value.
    ///
    /// The `Ok` wrapper is consumed by the `unwrap` that follows, and the `Err`
    /// wrapper by the `unwrapErr` that rebuilds it, so neither is left for the
    /// leak registry to find.
    pub(crate) fn lower_try(&self, try_expr: &syn::ExprTry) -> Lowered {
        let inner = self.expr(&try_expr.expr);
        let span = syn::spanned::Spanned::span(try_expr);
        let ty = self.resolve_expr_type(&try_expr.expr).ok();
        let temp = self.fresh_hoist("_r");

        // `?` on an `Option<T>` leaves with `None`, which this port writes as
        // null. The engine names the type; a receiver it could not name is a
        // `Result`, which is what all but a handful of `?` in the corpus are.
        let is_option = ty.as_ref().is_some_and(|ty| self.is_nullable(ty));
        if is_option {
            // Rust allows `?` on an `Option` only inside a function that
            // returns one. A function returning a `Result` here means the
            // engine named the operand wrongly, and `return null` would leave
            // through the wrong door.
            if self.fn_return.as_ref().is_some_and(|r| self.is_result(r)) {
                self.fallback(
                    span,
                    "`?` here tests an `Option` inside a function returning a `Result`, which \
                     Rust does not allow, so one of the two types is not what the engine read; \
                     the exit is written as `return null` all the same",
                );
            }
            return Lowered {
                declaration: format!(
                    "const {} = {};\nif ({} == null) {};\n",
                    temp,
                    inner,
                    temp,
                    self.leaving_with("null")
                ),
                value: temp,
                wrapper: None,
            };
        }

        if ty.is_none() {
            self.fallback(
                span,
                "`?` is lowered as a `Result` without the engine having named what it tests",
            );
        }
        let error = match self.try_conversion(ty.as_ref(), span) {
            Some(call) => format!("{}({}.unwrapErr())", call, temp),
            None => format!("{}.unwrapErr()", temp),
        };
        Lowered {
            declaration: format!(
                "const {} = {};\nif ({}.isErr()) {};\n",
                temp,
                inner,
                temp,
                self.leaving_with(&format!("Result.Err({})", error))
            ),
            value: format!("{}.unwrap()", temp),
            wrapper: Some(temp),
        }
    }

    /// The early exit a `?` performs, written for where it stands.
    ///
    /// In an ordinary body it is a `return`. Inside a body the emitter lifted
    /// into an arrow, a `return` would leave the arrow and the caller would
    /// read the error object as the expression's value — `Result.Err(..)` is
    /// truthy, so `if (applied)` took the success branch for an event that
    /// failed to apply. The exit travels out as a sentinel instead and the
    /// statement holding the lifted value performs the real return.
    pub(crate) fn leaving_with(&self, value: &str) -> String {
        if self.jump_as_value.get() {
            crate::body::return_sentinel(value)
        } else {
            format!("return {}", value)
        }
    }

    /// What a `?` operand has to be, given what the `?` itself has to produce.
    pub(crate) fn try_operand_expectation(
        &self,
        expected: Option<&crate::ty::Ty>,
    ) -> Option<crate::ty::Ty> {
        self.types
            .as_ref()?
            .borrow()
            .try_operand_expectation(expected)
    }

    /// Is this integer literal one the port writes as a `bigint`?
    ///
    /// The written suffix decides it where there is one; otherwise it is what
    /// the position wants, which is how `n + 1` beside a `u64` writes `1n`.
    pub(crate) fn is_bigint_literal(&self, lit: &syn::Lit, expected: Option<&crate::ty::Ty>) -> bool {
        let syn::Lit::Int(int) = lit else { return false };
        match int.suffix() {
            "u64" | "i64" | "u128" | "i128" => return true,
            "" => {}
            _ => return false,
        }
        matches!(
            expected.map(crate::ty::Ty::peel_refs),
            Some(crate::ty::Ty::Prim(
                crate::ty::Prim::U64
                    | crate::ty::Prim::I64
                    | crate::ty::Prim::U128
                    | crate::ty::Prim::I128
            ))
        )
    }

    /// Is this the `Result<T, E>` the port writes as the runtime's `Result`?
    pub(crate) fn is_result(&self, ty: &crate::ty::Ty) -> bool {
        match &self.types {
            Some(tc) => tc.borrow().is_result(ty),
            None => false,
        }
    }

    /// Is this the `Option<T>` the port writes as `T | null`?
    pub(crate) fn is_nullable(&self, ty: &crate::ty::Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let Some(id) = ty.peel_refs().id() else {
            return false;
        };
        matches!(
            tc.borrow().registry.shapes().form(id),
            Some(crate::name_map::system_shapes::Form::Nullable)
        )
    }
}
