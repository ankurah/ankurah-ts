//! A method's signature: what it takes, what it answers with, and how it takes
//! its receiver.
//!
//! Built from the extracted declaration, with the impl's own parameters in
//! scope. A method whose return type the engine cannot read stays out of the
//! table: "no answer" is the truth, and calling it `()` would not be.

use super::build::resolve_bounds;
use super::resolve_type;
use super::{ModuleId, TypeEnv, TypeRegistry};
use crate::diag::DiagSink;
use crate::types::{FnInfo, SelfKind};
use crate::ty::Ty;
use super::impls;

#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    /// How the receiver is taken. `None` is an associated function, which a
    /// method call never reaches.
    pub self_kind: Option<SelfKind>,
    /// The type this method accepts as its receiver, written in terms of the
    /// impl's own parameters — `&Arc<Inner<T>>` for a `&self` method on
    /// `Arc<Inner<T>>`. Method resolution matches this against the receiver it
    /// has, which is the question Rust's probe asks; comparing borrow kinds
    /// instead let a blanket impl win a step early.
    pub receiver: Option<Ty>,
    /// Parameters the method declares on top of its impl's.
    pub type_params: Vec<String>,
    /// What those parameters have to implement. `collect<B: FromIterator<Item>>`
    /// is how a turbofish written as `collect::<Vec<_>>()` gets its element
    /// type: the bound says which `FromIterator` impl `Vec<_>` has to be.
    pub bounds: Vec<impls::Bound>,
    /// Was it written `async fn`? Such a call is modelled as returning what it
    /// writes rather than a future (spec 4.10), so awaiting one is the
    /// identity, and awaiting anything else needs a `Future` the table reads.
    pub is_async: bool,
}

impl MethodSig {
    pub fn is_static(&self) -> bool {
        self.self_kind.is_none()
    }
}

/// A method's signature, or nothing when the engine could not name a type in
/// it. A method whose return type the engine cannot read stays out of the
/// table: "no answer" is the truth, and calling it `()` would not be.
pub(super) fn method_sig(
    reg: &TypeRegistry,
    module: ModuleId,
    params: &[String],
    self_ty: Option<&Ty>,
    method: &FnInfo,
    sink: &DiagSink,
) -> Option<MethodSig> {
    let env = TypeEnv::new(reg, module, sink)
        .with_params(params)
        .with_self(self_ty);

    // `self: Arc<Self>` and the other written receivers put the method on a
    // type the engine does not yet walk to. Reading one as by-value would say it
    // sits on `Self`; leaving it out says the truth, and the written type is
    // kept on the extracted function for the step that supports it.
    if method.self_kind == Some(crate::types::SelfKind::Arbitrary) {
        if let Some(written) = &method.self_receiver {
            sink.push(crate::diag::Diag::at(
                &sink.file(),
                syn::spanned::Spanned::span(written),
                format!(
                    "`self: {}` is a receiver the engine does not model; `{}` is left out of the method table",
                    quote::ToTokens::to_token_stream(written),
                    method.name
                ),
            ));
        }
        return None;
    }

    let receiver = self_ty.map(|ty| match method.self_kind {
        Some(crate::types::SelfKind::Ref) => Ty::Ref {
            mutable: false,
            inner: Box::new(ty.clone()),
        },
        Some(crate::types::SelfKind::RefMut) => Ty::Ref {
            mutable: true,
            inner: Box::new(ty.clone()),
        },
        _ => ty.clone(),
    });
    let receiver = method.self_kind.and(receiver);

    let mut resolved_params = Vec::new();
    for param in &method.params {
        let Some(rust_ty) = &param.rust_ty else {
            continue;
        };
        match resolve_type(rust_ty, &env) {
            Ok(ty) => resolved_params.push((param.name.clone(), ty)),
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        }
    }
    // A function with no written return type returns the unit type.
    let ret = match &method.rust_return {
        None => Ty::Unit,
        Some(rust_ty) => match resolve_type(rust_ty, &env) {
            Ok(ty) => ty,
            Err(diag) => {
                sink.push(diag);
                return None;
            }
        },
    };
    Some(MethodSig {
        params: resolved_params,
        ret,
        self_kind: method.self_kind,
        receiver,
        type_params: method.type_params.clone(),
        bounds: resolve_bounds(&method.syn_generics, &env, sink),
        is_async: method.is_async,
    })
}
