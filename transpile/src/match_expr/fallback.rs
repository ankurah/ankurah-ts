//! What one arm of the catch-all's EXPANSION writes around the shared body.
//!
//! For: the runtime's `.match({..})` dispatches on the variant name and has one
//! arm per variant, so a Rust `_` arm has to be written out once for every
//! variant the source left to it. Each of those copies needs the same three
//! things decided the same way — what it declares from the value it was handed,
//! what it releases on the way out, and whether it takes the payload at all —
//! and this is where they are decided, once, for the expansion and for a
//! chain's `else` alike.

use crate::body::{indent, BodyTranslator};

/// What one arm of the expansion — or one chain's `else` — writes around the
/// catch-all's body.
pub(super) struct Pieces {
    /// The drop flags the arm owes and the name it gives the whole value.
    pub(super) bindings: String,
    /// What the arm's `finally` says about the payload no name took.
    pub(super) release: String,
    /// Whether the arm is handed the payload at all.
    pub(super) takes_payload: bool,
    /// The whole value, as the arm can name it.
    pub(super) whole: String,
}

/// The catch-all's body, ready to stand where the catch-all would have run.
///
/// For: the expansion writes one arm per variant the source left to the `_`,
/// and a contested variant's arm CHAIN needs the same body as its last `else` —
/// so the two ask one thing for it. `lower` builds this once it has written the
/// catch-all's body, which is why the chain is written after the named arms
/// rather than with them.
pub(super) struct Fallback<'a> {
    pub(super) class: &'a str,
    pub(super) consuming: bool,
    pub(super) scrutinee: &'a str,
    pub(super) bound: Option<&'a str>,
    pub(super) declares: bool,
    pub(super) flags: &'a str,
    pub(super) body: &'a str,
    pub(super) owned: &'a [crate::ownership::Owned],
    pub(super) lifted: &'a [crate::ownership::Hoist],
    pub(super) produces: bool,
    /// Is `body` one EXPRESSION whose value an arm still owes (K2)?
    pub(super) value: bool,
    pub(super) is_async: bool,
    /// Whether a consuming arm that does not rebuild the value owes the payload
    /// a release.
    pub(super) owes_payload: bool,
    /// The local closure the body was hoisted into, where it was worth one.
    pub(super) hoisted: Option<&'a str>,
    /// The catch-all's body as it was written, for the names it declares.
    pub(super) rest_body: &'a syn::Expr,
}

impl<'a> Fallback<'a> {
    pub(super) fn pieces(&self, variant: &str, has_payload: bool, param: &str) -> Pieces {
        // A borrowing match leaves the enum whole, so the subject *is* the
        // value; a consuming one has only the payload, and the value is that
        // payload back under the variant this arm matched.
        let whole = if self.consuming {
            format!("new {}('{}', {})", self.class, variant, param)
        } else {
            self.scrutinee.to_string()
        };
        let bindings = match (self.bound, self.declares) {
            (Some(name), true) => format!("{}const {} = {};\n", self.flags, name, whole),
            _ => self.flags.to_string(),
        };
        // A unit variant's payload is empty, so there is nothing in it to own.
        let release = if has_payload && self.owes_payload {
            format!("dropUnbound({}, []);\n", param)
        } else {
            String::new()
        };
        let takes_payload = self.consuming && (self.declares || !release.is_empty());
        Pieces { bindings, release, takes_payload, whole }
    }

    /// The catch-all's body for this variant, as the statements a chain's last
    /// `else` holds.
    pub(super) fn statements(
        &self,
        variant: &str,
        has_payload: bool,
        param: &str,
        t: &BodyTranslator,
    ) -> String {
        let Pieces { bindings, release, whole, .. } = self.pieces(variant, has_payload, param);
        if let Some(hoisted) = self.hoisted {
            let call = match (self.bound, self.declares) {
                (Some(_), true) => format!("{}({})", hoisted, whole),
                _ => format!("{}()", hoisted),
            };
            let inner = if release.is_empty() {
                format!("return {};\n", call)
            } else {
                format!("try {{\n  return {};\n}} finally {{\n{}}}\n", call, indent(&release))
            };
            return format!("{}{}", self.flags, inner);
        }
        super::arms::arm_block(
            super::ArmParts {
                variant,
                bindings,
                param: None,
                body: self.body,
                owned: self.owned,
                lifted: self.lifted,
                produces: self.produces,
                value: self.value,
                is_async: self.is_async,
                release_rest: release,
            },
            t,
        )
    }
}

/// The TypeScript name of the subject, where the subject is a plain name.
///
/// A subject that is not one cannot be written twice — Rust evaluates it once —
/// and it is also a value no arm's body can name, because Rust has nothing to
/// call it either.
pub(super) fn subject_name(subject: &syn::Expr) -> Option<String> {
    let syn::Expr::Path(path) = subject else {
        return None;
    };
    let ident = path.path.get_ident()?;
    Some(crate::name_map::escape_reserved(&crate::name_map::to_camel_case(
        &ident.to_string(),
    )))
}

/// Does this arm's body read the subject itself?
///
/// A `_` arm moves nothing, so Rust lets its body use the value the match was
/// given — `_ => MutationError::RetrievalError(err)`. The emitted arm has to
/// have something to call `err`, and only an arm whose body says so needs it.
pub(super) fn mentions_subject(body: &syn::Expr, subject: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = subject else {
        return false;
    };
    let Some(ident) = path.path.get_ident() else {
        return false;
    };
    names_ident(&quote::ToTokens::to_token_stream(body), ident)
}

/// Is this identifier written anywhere in these tokens?
///
/// A name is a name whatever expression holds it, and the tokens are where
/// every one of them is. It answers yes for a field or method of the same
/// spelling too, which costs a binding the arm does not read and never costs
/// the arm a value it does.
fn names_ident(tokens: &proc_macro2::TokenStream, wanted: &proc_macro2::Ident) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == *wanted,
        proc_macro2::TokenTree::Group(group) => names_ident(&group.stream(), wanted),
        _ => false,
    })
}
