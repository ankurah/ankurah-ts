//! The conversion a CALLER chooses and a callee cannot: dictionary passing.
//!
//! For: `value.try_into()` on a `value: V` bounded by `V: TryInto<Expr>` is
//! decided by the caller's WRITTEN type. Rust compiles one body per
//! instantiation and picks the impl in each; the port compiles the body once
//! (spec 4.4a), and no runtime shape recovers the choice — `From<String>` and
//! `From<&str>` both take a JavaScript string. So the conversion travels as a
//! value: a function bounded by a conversion trait whose other side is a
//! concrete type grows a synthetic trailing parameter carrying it, a generic
//! body threads its own along, and a concrete call site synthesises one from
//! the type it inferred. A call site that cannot name that type is refused.
//! Spec §4.4b.
//!
//! The limit, and the reason for it: the parameter a dictionary is written for
//! must be one the signature carries a VALUE of — directly, or as the item of
//! something it takes. `populate<I, V, E>` carries a `V` (as `I`'s item) and
//! carries no `E` at all: `E` is whatever the `V` conversion failed with, so it
//! is decided by that conversion rather than by the caller, and a dictionary
//! for it would be a parameter no call site can name. `e.into()` on such a
//! parameter keeps the diagnostic it has.

use crate::registry::{Probe, TypeRegistry};
use crate::ty::{TraitRef, Ty};

/// Which side of the conversion the bounded parameter is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `V: Into<C>` / `V: TryInto<C>`: the dictionary takes a `V` and answers a `C`.
    Out,
    /// `V: From<C>` / `V: TryFrom<C>`: the dictionary takes a `C` and answers a `V`.
    In,
}

/// One conversion a function's bounds ask its callers to supply.
#[derive(Debug, Clone)]
pub struct Dictionary {
    /// The type parameter the bound constrains.
    pub param: String,
    pub direction: Direction,
    /// Does the trait answer a `Result`?
    pub fallible: bool,
    /// The concrete type on the other side of the conversion.
    pub concrete: Ty,
    /// The `Error = ..` the bound wrote, where it wrote one.
    pub error: Option<Ty>,
}

impl Dictionary {
    /// The synthetic parameter's name. One per type parameter, so a body that
    /// threads its own along writes the same name it declares.
    pub fn name(&self) -> String {
        format!("_conv{}", self.param)
    }

    /// `(v: V) => Result<Expr, E>` — what the synthetic parameter is declared
    /// as, in the caller's own spelling of the two types.
    pub fn ts_type(&self, reg: &TypeRegistry) -> String {
        let concrete = crate::name_map::map_ty(reg, &self.concrete);
        let (takes, answers) = match self.direction {
            Direction::Out => (self.param.clone(), concrete),
            Direction::In => (concrete, self.param.clone()),
        };
        let answers = match self.fallible {
            true => {
                let error = match &self.error {
                    Some(error) => crate::name_map::map_ty(reg, error),
                    None => "unknown".to_string(),
                };
                format!("Result<{}, {}>", answers, error)
            }
            false => answers,
        };
        format!("(value: {}) => {}", takes, answers)
    }

    /// Does this dictionary perform the same conversion as another, so that a
    /// caller's own may be threaded into it?
    pub fn same_conversion(&self, other: &Dictionary) -> bool {
        self.direction == other.direction
            && self.fallible == other.fallible
            && self.concrete == other.concrete
    }
}

/// The four conversion traits, and which side the bounded parameter is on.
fn direction_of(reg: &TypeRegistry, trait_ref: &TraitRef) -> Option<(Direction, bool)> {
    let named = |path: &str| reg.system_type(path) == Some(trait_ref.id);
    match () {
        _ if named("std::convert::Into") => Some((Direction::Out, false)),
        _ if named("std::convert::TryInto") => Some((Direction::Out, true)),
        _ if named("std::convert::From") => Some((Direction::In, false)),
        _ if named("std::convert::TryFrom") => Some((Direction::In, true)),
        _ => None,
    }
}

/// Is there anything for a dictionary to CARRY — a class of the target's own
/// with conversions the port emits onto it, that a call site can name?
///
/// The scope's second half, and the reason for it, measured rather than
/// assumed. `Object::set<K: Into<JsValue>, V: TryInto<JsValue>>` is bounded
/// exactly as `populate<V: TryInto<Expr>>` is, and `JsValue` is the
/// declared-surface type the port writes every JavaScript value as: it has no
/// class of its own, so nothing could be handed over, and asking turned ONE
/// diagnostic in the callee's body into twenty-two refusals at its call sites.
fn the_port_emits_a_conversion(reg: &TypeRegistry, target: &Ty) -> bool {
    crate::emit_impls::has_emitted_class(reg, target)
        && [crate::registry::convert::FROM_PATH, crate::registry::convert::TRY_FROM_PATH]
            .iter()
            .any(|path| !crate::emit_impls::conversion_names(reg, target, path).is_empty())
}

/// Does this type mention any of the function's own parameters, so that it is
/// not the concrete other side a dictionary needs?
fn mentions_a_param(ty: &Ty, params: &[String]) -> bool {
    params.iter().any(|p| ty.mentions_param(p))
}

/// The type parameters the signature carries a VALUE of: those written in a
/// parameter or return type, and those an associated binding of such a
/// parameter's bound names (`I: Iterator<Item = V>` carries `V` once `I` is
/// carried).
pub fn carried(
    reg: &TypeRegistry,
    params: &[String],
    value_types: &[Ty],
    bounds: &[(String, TraitRef)],
) -> Vec<String> {
    let mut carried: Vec<String> = Vec::new();
    for param in params {
        if value_types.iter().any(|ty| ty.mentions_param(param)) {
            carried.push(param.clone());
        }
    }
    // A binding on a carried parameter's bound carries whatever it names, and
    // that one's bindings carry theirs: `I: Iterator<Item = V>` with a
    // `V: IntoIterator<Item = W>` above it carries `W` too.
    //
    // Not a CONVERSION bound's own bindings. `V: TryInto<Expr, Error = E>`
    // says what the conversion fails with, and that is decided by the
    // conversion — by the dictionary itself — rather than by the caller: a
    // dictionary for `E` would be a parameter no call site can name. `e.into()`
    // on such a parameter keeps the diagnostic it has.
    loop {
        let before = carried.len();
        for (subject, trait_ref) in bounds {
            if !carried.contains(subject) || direction_of(reg, trait_ref).is_some() {
                continue;
            }
            for (_, bound_to) in &trait_ref.bindings {
                for param in params {
                    if bound_to.mentions_param(param) && !carried.contains(param) {
                        carried.push(param.clone());
                    }
                }
            }
        }
        if carried.len() == before {
            return carried;
        }
    }
}

/// Every dictionary a signature asks its callers for, in the order its type
/// parameters are declared, so that the declaration and every call site agree
/// on which trailing argument is which.
pub fn wanted(
    reg: &TypeRegistry,
    params: &[String],
    bounds: &[(String, TraitRef)],
    carried: &[String],
) -> Vec<Dictionary> {
    let mut wanted: Vec<Dictionary> = Vec::new();
    for param in params {
        if !carried.contains(param) {
            continue;
        }
        for (subject, trait_ref) in bounds.iter().filter(|(s, _)| s == param) {
            let Some((direction, fallible)) = direction_of(reg, trait_ref) else { continue };
            // Only `V: TryInto<C>`, the one family the corpus needs and the
            // one the ruling's example names. `V: From<C>` and `V: TryFrom<C>`
            // put the parameter on the TARGET side, where which impl runs
            // depends on an instantiation the declaration cannot see. And a
            // plain `V: Into<C>` is the shape the wasm boundary is written in
            // — `Into<JsValue>`, `Into<EventNames>` — where the callee is a
            // hand-written file in the port and an extra argument at its call
            // sites would be an argument nothing declares. All three keep the
            // diagnostics they have.
            if direction != Direction::Out || !fallible {
                continue;
            }
            let Some(concrete) = trait_ref.args.first() else { continue };
            if mentions_a_param(concrete, params) || !the_port_emits_a_conversion(reg, concrete) {
                continue;
            }
            // One dictionary per parameter: a second conversion bound on the
            // same parameter would need a second name, and no corpus shape
            // asks for one. The first written wins and the rest keep the
            // diagnostics they have.
            if wanted.iter().any(|d| &d.param == subject) {
                continue;
            }
            wanted.push(Dictionary {
                param: subject.clone(),
                direction,
                fallible,
                concrete: concrete.clone(),
                error: trait_ref
                    .bindings
                    .iter()
                    .find(|(name, _)| name == "Error")
                    .map(|(_, ty)| ty.clone()),
            });
        }
    }
    wanted
}

/// What the callee's dictionary parameter was instantiated with at this call.
///
/// Two questions in order. The parameter may stand in a value position, where
/// unifying the declared types against the written arguments answers it. Or it
/// may be named only by a binding on another parameter's bound —
/// `I: Iterator<Item = V>` — where the answer is that other parameter's own
/// `Item`: read off the CALLER's bound where the argument is one of the
/// caller's parameters, and projected through the impl table where it is a
/// concrete type.
pub fn instantiation(
    probe: &Probe<'_>,
    dictionary: &Dictionary,
    sig_params: &[String],
    sig_value_types: &[Ty],
    sig_bounds: &[(String, TraitRef)],
    actual: &[Ty],
    caller_bounds: &[(String, TraitRef)],
) -> Option<Ty> {
    // The callee's parameters are renamed apart before they are matched
    // against what the caller wrote. `wrap(v)` inside `twice<V: Into<Held>>`
    // hands a `V` to a `V`, and unification's occurs check refuses to bind a
    // parameter to a type that mentions it — which is right for one scope and
    // wrong across two, where the two `V`s are different parameters that
    // happen to share a name.
    let apart: crate::ty::subst::Subst = sig_params
        .iter()
        .map(|p| (p.clone(), Ty::Param(renamed(p))))
        .collect();
    let mut subst = crate::ty::subst::Subst::default();
    let vars: Vec<String> = sig_params.iter().map(|p| renamed(p)).collect();
    // As written first, so that a parameter standing for `&'static str` is
    // bound to `&'static str` and reaches the impl written for it. Then again
    // with both sides peeled, because emission erases a reference and a
    // `&mut I` reborrowed at the call arrives here as the referent's own type:
    // a binding the first pass made is kept, since a second, conflicting bind
    // is refused rather than overwritten.
    for peeled in [false, true] {
        for (declared, actual) in sig_value_types.iter().zip(actual) {
            let declared = declared.substitute(&apart);
            let (declared, actual) = match peeled {
                false => (&declared, actual),
                true => (declared.peel_refs(), actual.peel_refs()),
            };
            let _ = crate::ty::unify(&vars, declared, actual, &mut subst);
        }
    }
    if let Some(found) = subst.get(&renamed(&dictionary.param)) {
        return Some(probe.normalize(found));
    }
    for (subject, trait_ref) in sig_bounds {
        let Some((name, _)) = trait_ref
            .bindings
            .iter()
            .find(|(_, ty)| matches!(ty, Ty::Param(p) if p == &dictionary.param))
        else {
            continue;
        };
        let Some(stood) = subst.get(&renamed(subject)) else { continue };
        // The caller wrote one of its OWN parameters there, or a projection
        // rooted at one. Rust reads the item off the bound the caller declared
        // for that parameter; there is no impl to project through, because the
        // type is not known yet.
        //
        // A projection is rooted rather than matched exactly:
        // `values.into_iter()` on an `I: IntoIterator<Item = V>` has the type
        // `<I as IntoIterator>::IntoIter`, and what its `Item` is comes from
        // the same bound that named `I`'s. The limit is that ONE binding of
        // that name has to be written on the root, which is what makes reading
        // it off the root the same answer as elaborating the chain.
        if let Some(open) = rooted_at_a_param(stood) {
            let mut written = caller_bounds
                .iter()
                .filter(|(s, _)| s == &open)
                .filter(|(_, t)| matches!(stood.peel_refs(), Ty::Param(_)) == (t.id == trait_ref.id))
                .filter_map(|(_, t)| {
                    t.bindings.iter().find(|(n, _)| n == name).map(|(_, ty)| ty.clone())
                });
            let found = written.next();
            if found.is_some() && written.next().is_none() {
                return found;
            }
            continue;
        }
        let projected = probe.normalize(&Ty::Assoc {
            base: Box::new(stood.peel_refs().clone()),
            trait_: Some(Box::new(trait_ref.clone())),
            name: name.clone(),
        });
        if !matches!(projected, Ty::Assoc { .. }) {
            return Some(projected);
        }
    }
    None
}

impl crate::body::BodyTranslator<'_> {
    /// A resolved method call's arguments, with the dictionaries its callee
    /// asks for appended (spec 4.4b).
    pub(crate) fn with_dictionaries(
        &self,
        passthrough: bool,
        args: Vec<String>,
        found: &crate::registry::MethodResolution,
        call: &syn::ExprMethodCall,
    ) -> Vec<String> {
        if !passthrough {
            return args;
        }
        let mut args = args;
        let actual = self.written_argument_types(call.args.iter());
        args.extend(self.dictionary_arguments(
            found,
            &actual,
            syn::spanned::Spanned::span(call),
        ));
        args
    }

    /// The same for a `f(..)`.
    pub(crate) fn with_call_dictionaries(
        &self,
        args: Vec<String>,
        call: &syn::ExprCall,
        expected: Option<&Ty>,
    ) -> Vec<String> {
        let mut args = args;
        let actual = self.written_argument_types(call.args.iter());
        args.extend(self.call_dictionary_arguments(
            call,
            expected,
            &actual,
            syn::spanned::Spanned::span(call),
        ));
        args
    }

    /// What each written argument's type resolves to, for matching against what
    /// the callee declared. An argument the engine cannot type is left out, and
    /// the parameters it would have bound stay open.
    fn written_argument_types<'e>(
        &self,
        args: impl Iterator<Item = &'e syn::Expr>,
    ) -> Vec<Ty> {
        args.filter_map(|a| self.quietly(|| self.resolve_expr_type(a)).ok()).collect()
    }

    /// The conversion the caller handed in, where this receiver is a type
    /// parameter one of the enclosing function's bounds converts (spec 4.4b).
    ///
    /// `into` reads an infallible dictionary and `try_into` a fallible one:
    /// what the emitted call answers has to be what the Rust method answers,
    /// so a `try_into` written against an `Into` bound is left to the impl
    /// table and its diagnostic rather than handed a value where the source
    /// reads a wrapper.
    pub(crate) fn through_a_dictionary(
        &self,
        from: Option<&Ty>,
        receiver: &str,
        method: &str,
    ) -> Option<String> {
        let Ty::Param(param) = from?.peel_refs() else { return None };
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let found = tc.dictionaries.iter().find(|d| {
            &d.param == param
                && d.direction == crate::convert::dictionary::Direction::Out
                && d.fallible == (method == "try_into")
        })?;
        Some(format!("{}({})", found.name(), receiver))
    }

    /// The trailing arguments a call owes for the dictionaries the callee's own
    /// bounds ask for (spec 4.4b).
    ///
    /// Three answers per dictionary, in order. The callee's parameter stands
    /// for one of THIS body's own parameters, and this body was handed a
    /// dictionary for it, so that one goes over. Or it stands for a concrete
    /// type, and the conversion is written here, where the type is known. Or
    /// the engine cannot name it, which is a refusal: an emitted call missing
    /// the conversion its callee reads would be a `TypeError` three frames
    /// down, where nothing says what went wrong.
    pub(crate) fn dictionary_arguments(
        &self,
        found: &crate::registry::MethodResolution,
        actual: &[Ty],
        span: proc_macro2::Span,
    ) -> Vec<String> {
        let Some(tc) = &self.types else { return Vec::new() };
        let sig = tc.borrow().registry.method_sig(found);
        match sig {
            Some(sig) => self.dictionaries_for(&sig, actual, span),
            None => Vec::new(),
        }
    }

    /// The same for a `f(..)` whose callee the path resolved to a signature.
    pub(crate) fn call_dictionary_arguments(
        &self,
        call: &syn::ExprCall,
        expected: Option<&Ty>,
        actual: &[Ty],
        span: proc_macro2::Span,
    ) -> Vec<String> {
        let Some(tc) = &self.types else { return Vec::new() };
        let sig = tc.borrow().call_sig(call, expected).map(|(sig, _)| sig);
        match sig {
            Some(sig) => self.dictionaries_for(&sig, actual, span),
            None => Vec::new(),
        }
    }

    fn dictionaries_for(
        &self,
        sig: &crate::registry::MethodSig,
        actual: &[Ty],
        span: proc_macro2::Span,
    ) -> Vec<String> {
        let Some(tc) = &self.types else { return Vec::new() };
        let asked = {
            let tc = tc.borrow();
            let bounds = crate::registry::method::param_bounds_of(&sig.bounds);
            let declared: Vec<Ty> = sig.params.iter().map(|(_, ty)| ty.clone()).collect();
            let mut value_types = declared.clone();
            value_types.push(sig.ret.clone());
            let carried = carried(tc.registry, &sig.type_params, &value_types, &bounds);
            let wanted = wanted(tc.registry, &sig.type_params, &bounds, &carried);
            wanted
                .into_iter()
                .map(|one| {
                    let stood = instantiation(
                        &tc.probe(),
                        &one,
                        &sig.type_params,
                        &declared,
                        &bounds,
                        actual,
                        &tc.param_bounds,
                    );
                    let threaded = match &stood {
                        Some(Ty::Param(open)) => tc
                            .dictionaries
                            .iter()
                            .find(|mine| &mine.param == open && mine.same_conversion(&one))
                            .map(|mine| mine.name()),
                        _ => None,
                    };
                    (one, stood, threaded)
                })
                .collect::<Vec<_>>()
        };
        asked
            .into_iter()
            .map(|(one, stood, threaded)| match (threaded, stood) {
                (Some(name), _) => name,
                (None, Some(concrete)) if !matches!(concrete, Ty::Param(_)) => {
                    self.written_conversion(&one, &concrete, span)
                }
                (None, stood) => {
                    let named = match stood {
                        Some(ty) => match &self.types {
                            Some(tc) => format!("`{}`", tc.borrow().registry.describe(&ty)),
                            None => "a type with no context to name it in".to_string(),
                        },
                        None => "nothing the engine can name".to_string(),
                    };
                    self.fallback(
                        span,
                        format!(
                            "this call has to hand `{}` the conversion its `{}` bound needs, \
                             and the type standing at that parameter is {}",
                            one.name(),
                            one.param,
                            named
                        ),
                    );
                    crate::body::hole_text(&format!(
                        "the conversion for `{}` cannot be named here",
                        one.param
                    ))
                }
            })
            .collect()
    }

    /// The arrow a concrete instantiation's conversion is written as.
    fn written_conversion(
        &self,
        one: &Dictionary,
        concrete: &Ty,
        span: proc_macro2::Span,
    ) -> String {
        let (from, to) = match one.direction {
            Direction::Out => (concrete.clone(), one.concrete.clone()),
            Direction::In => (one.concrete.clone(), concrete.clone()),
        };
        let what = format!("the conversion `{}`'s bound asks for", one.param);
        let Some(text) = self.conversion_text(&from, &to, "value", span, &what) else {
            return format!(
                "(value) => {}",
                crate::body::hole_text(&format!(
                    "no impl converts what stands at `{}` here",
                    one.param
                ))
            );
        };
        // The parameter carries the type this site inferred. Written bare, the
        // arrow's `value` was `unknown` to TypeScript — the callee's `V` is
        // named only through `I extends Iterable<V>`, which an array literal
        // does not pin — and every synthesised dictionary was a type error.
        let takes = match &self.types {
            Some(tc) => format!("value: {}", crate::name_map::map_ty(tc.borrow().registry, &from)),
            None => "value".to_string(),
        };
        // A fallible bound answers a `Result`, and every conversion the impl
        // table writes here is an infallible `From`: a genuine `TryFrom` whose
        // call already answers a wrapper is not found by `conversion_text` at
        // all, and takes the refusal above rather than being wrapped twice.
        match one.fallible {
            true => format!("({}) => Result.Ok({})", takes, text),
            false => format!("({}) => {}", takes, text),
        }
    }
}

/// A callee's type parameter, named apart from the caller's own. `$` cannot
/// appear in a Rust identifier, so a renamed parameter can never collide with
/// one the source wrote.
fn renamed(param: &str) -> String {
    format!("callee${}", param)
}

/// The type parameter a projection chain is rooted at, where it is one.
/// `<<I as IntoIterator>::IntoIter as Iterator>::Item` is rooted at `I`.
fn rooted_at_a_param(ty: &Ty) -> Option<String> {
    match ty.peel_refs() {
        Ty::Param(name) => Some(name.clone()),
        Ty::Assoc { base, .. } => rooted_at_a_param(base),
        _ => None,
    }
}
