//! Naming one item of a Rust file, so `[[excluded_items]]` can point at it.
//!
//! An item is named the way the corpus writes it — `impl From<MutationError> for
//! JsValue`, `fn Context::js_node_id`, `mod ffi` — and matched by the LAST path
//! segment of every name in it, because `wasm_bindgen::JsValue` and `JsValue`
//! are the same type and the corpus writes both.
//!
//! Nothing here guesses: a selector that does not parse is a config error, and a
//! selector that matches nothing in a file that was read is reported, so the
//! config cannot go stale against the corpus in silence.

use anyhow::{Result, bail};

/// One item of a file, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemSelector {
    /// `impl <Trait> for <Type>` — the trait matched by its last segment and,
    /// where the selector writes them, its generic arguments by theirs.
    TraitImpl {
        trait_name: String,
        trait_args: Vec<String>,
        self_ty: String,
    },
    /// `impl <Type>` — an inherent impl block.
    InherentImpl { self_ty: String },
    /// `fn <name>` — a free function of the module.
    FreeFn { name: String },
    /// `fn <Type>::<name>` — a method of an impl on `<Type>`.
    Method { self_ty: String, name: String },
    /// `type <Trait>::<name>` — an associated type of a trait declaration.
    AssocType { trait_name: String, name: String },
    /// `mod <name>` — an inline module.
    Module { name: String },
    /// `const <name>` — a module-level `const` or `static`. Both are values
    /// declared at the top of a module, and the exclusion list names either.
    Const { name: String },
}

impl ItemSelector {
    pub fn parse(written: &str) -> Result<ItemSelector> {
        let s = written.trim();
        if let Some(rest) = s.strip_prefix("impl ") {
            let rest = rest.trim();
            return Ok(match rest.split_once(" for ") {
                Some((tr, self_ty)) => {
                    let (trait_name, trait_args) = split_generic(tr.trim());
                    ItemSelector::TraitImpl {
                        trait_name,
                        trait_args,
                        self_ty: last_segment(self_ty.trim()),
                    }
                }
                None => ItemSelector::InherentImpl {
                    self_ty: last_segment(rest),
                },
            });
        }
        if let Some(rest) = s.strip_prefix("fn ") {
            let rest = rest.trim();
            return Ok(match rest.rsplit_once("::") {
                Some((self_ty, name)) => ItemSelector::Method {
                    self_ty: last_segment(self_ty),
                    name: name.to_string(),
                },
                None => ItemSelector::FreeFn {
                    name: rest.to_string(),
                },
            });
        }
        if let Some(rest) = s.strip_prefix("type ") {
            let rest = rest.trim();
            let Some((trait_name, name)) = rest.rsplit_once("::") else {
                bail!("`type` names an associated type as `type <Trait>::<name>`");
            };
            return Ok(ItemSelector::AssocType {
                trait_name: last_segment(trait_name),
                name: name.to_string(),
            });
        }
        if let Some(rest) = s.strip_prefix("mod ") {
            return Ok(ItemSelector::Module {
                name: rest.trim().to_string(),
            });
        }
        if let Some(rest) = s.strip_prefix("const ") {
            return Ok(ItemSelector::Const {
                name: rest.trim().to_string(),
            });
        }
        bail!(
            "an item is named `impl <Trait> for <Type>`, `impl <Type>`, `fn <name>`, \
             `fn <Type>::<name>`, `type <Trait>::<name>`, `mod <name>` or `const <name>`"
        )
    }

    /// Does this selector name `item`?
    pub fn matches(&self, item: &syn::Item) -> bool {
        match (self, item) {
            (
                ItemSelector::TraitImpl {
                    trait_name,
                    trait_args,
                    self_ty,
                },
                syn::Item::Impl(i),
            ) => {
                let Some((_, path, _)) = &i.trait_ else {
                    return false;
                };
                let Some(seg) = path.segments.last() else {
                    return false;
                };
                if seg.ident != trait_name.as_str() {
                    return false;
                }
                if !trait_args.is_empty() && generic_args(seg) != *trait_args {
                    return false;
                }
                type_last_segment(&i.self_ty).as_deref() == Some(self_ty.as_str())
            }
            (ItemSelector::InherentImpl { self_ty }, syn::Item::Impl(i)) => {
                i.trait_.is_none()
                    && type_last_segment(&i.self_ty).as_deref() == Some(self_ty.as_str())
            }
            (ItemSelector::FreeFn { name }, syn::Item::Fn(f)) => f.sig.ident == name.as_str(),
            (ItemSelector::Module { name }, syn::Item::Mod(m)) => m.ident == name.as_str(),
            (ItemSelector::Const { name }, syn::Item::Const(c)) => c.ident == name.as_str(),
            (ItemSelector::Const { name }, syn::Item::Static(st)) => st.ident == name.as_str(),
            _ => false,
        }
    }

    /// Does this selector name a method of an impl whose self type is `self_ty`?
    pub fn matches_method(&self, impl_self_ty: &syn::Type, method: &syn::ImplItemFn) -> bool {
        match self {
            ItemSelector::Method { self_ty, name } => {
                method.sig.ident == name.as_str()
                    && type_last_segment(impl_self_ty).as_deref() == Some(self_ty.as_str())
            }
            _ => false,
        }
    }

    /// Does this selector name an associated type of trait `trait_name`?
    pub fn matches_assoc_type(&self, in_trait: &str, assoc: &str) -> bool {
        match self {
            ItemSelector::AssocType { trait_name, name } => {
                trait_name == in_trait && name == assoc
            }
            _ => false,
        }
    }

    /// Whether this selector is one that only shows up inside another item, and
    /// so is never matched by `matches`.
    /// Read by this module's own tests.
    #[cfg(test)]
    pub fn is_nested(&self) -> bool {
        matches!(
            self,
            ItemSelector::Method { .. } | ItemSelector::AssocType { .. }
        )
    }
}

/// `From<MutationError>` → ("From", ["MutationError"]).
fn split_generic(s: &str) -> (String, Vec<String>) {
    match s.split_once('<') {
        Some((name, args)) => {
            let args = args.trim_end_matches('>');
            (
                last_segment(name),
                args.split(',')
                    .map(|a| last_segment(a.trim()))
                    .filter(|a| !a.is_empty())
                    .collect(),
            )
        }
        None => (last_segment(s), Vec::new()),
    }
}

fn last_segment(s: &str) -> String {
    s.rsplit("::").next().unwrap_or(s).trim().to_string()
}

fn generic_args(seg: &syn::PathSegment) -> Vec<String> {
    let syn::PathArguments::AngleBracketed(a) = &seg.arguments else {
        return Vec::new();
    };
    a.args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(t) => type_last_segment(t),
            _ => None,
        })
        .collect()
}

fn type_last_segment(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        syn::Type::Reference(r) => type_last_segment(&r.elem),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(src: &str) -> syn::Item {
        syn::parse_str(src).unwrap()
    }

    #[test]
    fn trait_impl_matches_through_a_qualified_path() {
        let sel = ItemSelector::parse("impl From<MutationError> for JsValue").unwrap();
        assert!(sel.matches(&item(
            "impl From<MutationError> for wasm_bindgen::JsValue { }"
        )));
        assert!(!sel.matches(&item("impl From<RetrievalError> for JsValue { }")));
        assert!(!sel.matches(&item("impl From<MutationError> for String { }")));
    }

    #[test]
    fn a_selector_without_arguments_matches_any() {
        let sel = ItemSelector::parse("impl WasmDescribe for Json").unwrap();
        assert!(sel.matches(&item(
            "impl wasm_bindgen::describe::WasmDescribe for Json { }"
        )));
    }

    #[test]
    fn free_fn_and_module() {
        assert!(
            ItemSelector::parse("fn wasm_prop")
                .unwrap()
                .matches(&item("pub fn wasm_prop<T>(x: T) -> T { x }"))
        );
        assert!(
            ItemSelector::parse("mod ffi")
                .unwrap()
                .matches(&item("pub mod ffi { }"))
        );
    }

    #[test]
    fn a_method_is_named_by_its_self_type() {
        let sel = ItemSelector::parse("fn Context::js_node_id").unwrap();
        assert!(sel.is_nested());
        let self_ty: syn::Type = syn::parse_str("Context").unwrap();
        let m: syn::ImplItemFn = syn::parse_quote! {
            pub fn js_node_id(&self) -> proto::EntityId { self.0.node_id() }
        };
        assert!(sel.matches_method(&self_ty, &m));
        let other: syn::Type = syn::parse_str("Transaction").unwrap();
        assert!(!sel.matches_method(&other, &m));
    }

    #[test]
    fn an_associated_type_is_named_by_its_trait() {
        let sel = ItemSelector::parse("type Model::RefWrapper").unwrap();
        assert!(sel.matches_assoc_type("Model", "RefWrapper"));
        assert!(!sel.matches_assoc_type("View", "RefWrapper"));
    }

    #[test]
    fn an_unreadable_selector_is_an_error() {
        assert!(ItemSelector::parse("the wasm one").is_err());
        assert!(ItemSelector::parse("type RefWrapper").is_err());
    }
}
