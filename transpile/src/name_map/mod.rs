//! Deterministic name mapping: Rust identifiers → TypeScript identifiers.
//!
//! This file converts what the source *wrote*. `emit_ty` converts what the
//! engine *resolved*, and `shape` holds the one table both of those and the
//! native-type translations read, so that what is emitted and what is
//! dispatched on cannot drift apart.

mod emit_ty;
pub mod shape;

pub use emit_ty::map_ty;

/// Convert snake_case to camelCase
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(if i == 0 {
                c.to_lowercase().next().unwrap()
            } else {
                c
            });
        }
    }

    result
}

/// Static name overrides for special Rust → TS function names
pub fn fn_name_override(rust_name: &str) -> Option<&'static str> {
    match rust_name {
        "fmt" => Some("toString"),
        "serialize" => Some("encode"),
        "deserialize" => Some("decode"),
        "eq" => Some("equals"),
        "ne" => Some("notEquals"),
        "partial_cmp" => Some("compareTo"),
        "clone" => Some("clone"),
        "default" => Some("default"),
        "drop" => Some("drop"),
        "from" => Some("from"),
        "try_from" => Some("tryFrom"),
        "new" => Some("new"),
        "next" => Some("next"),
        "deref" => Some("deref"),
        _ => None,
    }
}

/// Map a Rust function name to TS
pub fn map_fn_name(rust_name: &str) -> String {
    if let Some(override_name) = fn_name_override(rust_name) {
        override_name.to_string()
    } else {
        to_camel_case(rust_name)
    }
}

/// Map a Rust type name to TS (types stay PascalCase, but some have TS equivalents)
pub fn map_type_name(rust_name: &str) -> &str {
    match rust_name {
        "String" | "str" => "string",
        "bool" => "boolean",
        "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "usize" | "f64" | "f32" => "number",
        "i64" | "u64" => "bigint | number",
        "AtomicBool" => "boolean",
        "AtomicU32" | "AtomicUsize" => "number",
        "Infallible" => "never",
        "Rule" => "string", // pest grammar::Rule → string in TS (no pest equivalent)
        _ => rust_name,
    }
}

/// Map a complex Rust type to TS type string
pub fn map_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                let mapped = map_type_name(&name);

                // Handle generic types
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    let inner_types: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                Some(map_type(inner_ty))
                            } else {
                                None
                            }
                        })
                        .collect();

                    match name.as_str() {
                        "Vec" if inner_types.len() == 1 => {
                            if inner_types[0] == "number" && is_u8_vec(ty) {
                                "Uint8Array".to_string()
                            } else {
                                format!("{}[]", inner_types[0])
                            }
                        }
                        "Option" if inner_types.len() == 1 => {
                            format!("{} | null", inner_types[0])
                        }
                        "Result" if inner_types.len() == 2 => {
                            // Result<T, E> stays as Result<T, E>
                            format!("Result<{}, {}>", inner_types[0], inner_types[1])
                        }
                        "Result" if inner_types.len() == 1 => {
                            format!("Result<{}, Error>", inner_types[0])
                        }
                        "HashMap" | "BTreeMap" if inner_types.len() == 2 => {
                            format!("Map<{}, {}>", inner_types[0], inner_types[1])
                        }
                        "HashSet" | "BTreeSet" if inner_types.len() == 1 => {
                            format!("Set<{}>", inner_types[0])
                        }
                        "Box" if inner_types.len() == 1 => {
                            // Box<dyn Trait> → Trait, Box<T> → T
                            inner_types[0].clone()
                        }
                        "Arc" | "Weak" | "Mutex" | "RwLock" | "RefCell" | "Borrow"
                        | "BorrowMut" => {
                            // These stay as-is (from @ankurah/base)
                            format!("{}<{}>", mapped, inner_types.join(", "))
                        }
                        _ => {
                            format!("{}<{}>", mapped, inner_types.join(", "))
                        }
                    }
                } else {
                    mapped.to_string()
                }
            } else {
                "unknown".to_string()
            }
        }
        syn::Type::Reference(type_ref) => {
            // &T → T, &[u8] → Uint8Array
            map_type(&type_ref.elem)
        }
        syn::Type::Tuple(tuple) if tuple.elems.is_empty() => "void".to_string(),
        syn::Type::Tuple(tuple) => {
            let types: Vec<String> = tuple.elems.iter().map(|t| map_type(t)).collect();
            format!("[{}]", types.join(", "))
        }
        syn::Type::Slice(slice) => {
            let inner = map_type(&slice.elem);
            if inner == "number" {
                "Uint8Array".to_string()
            } else {
                format!("{}[]", inner)
            }
        }
        syn::Type::ImplTrait(impl_trait) => {
            for bound in &impl_trait.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    if let Some(result) = map_trait_bound(trait_bound) {
                        return result;
                    }
                }
            }
            "unknown".to_string()
        }
        syn::Type::TraitObject(trait_obj) => {
            for bound in &trait_obj.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    if let Some(result) = map_trait_bound(trait_bound) {
                        return result;
                    }
                }
            }
            "unknown".to_string()
        }
        syn::Type::Array(arr) => {
            let inner = map_type(&arr.elem);
            if inner == "number" {
                // [u8; N] → Uint8Array
                "Uint8Array".to_string()
            } else {
                format!("{}[]", inner)
            }
        }
        syn::Type::Never(_) => "never".to_string(),
        syn::Type::Infer(_) => "unknown".to_string(),
        syn::Type::Paren(paren) => map_type(&paren.elem),
        _ => "unknown".to_string(),
    }
}

/// Map a trait bound to a TS type
fn map_trait_bound(trait_bound: &syn::TraitBound) -> Option<String> {
    let seg = trait_bound.path.segments.last()?;
    let name = seg.ident.to_string();

    match name.as_str() {
        // Fn(&T) -> R → (arg: T) => R
        "Fn" | "FnMut" | "FnOnce" => {
            if let syn::PathArguments::Parenthesized(args) = &seg.arguments {
                let params: Vec<String> = args
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| format!("arg{}: {}", i, map_type(ty)))
                    .collect();
                let ret = match &args.output {
                    syn::ReturnType::Default => "void".to_string(),
                    syn::ReturnType::Type(_, ty) => map_type(ty),
                };
                return Some(format!("({}) => {}", params.join(", "), ret));
            }
            None
        }
        // impl Into<T> → T
        "Into" | "AsRef" => {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(map_type(inner));
                }
            }
            None
        }
        // impl Iterator<Item = T> → T[]
        "Iterator" | "IntoIterator" => {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let syn::GenericArgument::AssocType(assoc) = arg {
                        if assoc.ident == "Item" {
                            return Some(format!("{}[]", map_type(&assoc.ty)));
                        }
                    }
                }
            }
            None
        }
        // impl Future<Output = T> → Promise<T>
        "Future" => {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let syn::GenericArgument::AssocType(assoc) = arg {
                        if assoc.ident == "Output" {
                            return Some(format!("Promise<{}>", map_type(&assoc.ty)));
                        }
                    }
                }
            }
            Some("Promise<void>".to_string())
        }
        // Other traits — just use the trait name as an interface
        _ => Some(name),
    }
}

/// Check if a Vec<T> is Vec<u8> (should map to Uint8Array)
fn is_u8_vec(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(inner))) =
                        args.args.first()
                    {
                        if let Some(inner_seg) = inner.path.segments.last() {
                            return inner_seg.ident == "u8";
                        }
                    }
                }
            }
        }
    }
    false
}

/// Map Rust crate name to TS package name
pub fn map_crate_to_package(crate_name: &str) -> Option<&'static str> {
    match crate_name {
        "ankurah_proto" => Some("@ankurah/proto"),
        "ankurah_core" => Some("@ankurah/core"),
        "ankurah_signals" => Some("@ankurah/signals"),
        "ankql" => Some("@ankurah/ankql"),
        "ankurah_storage_common" => Some("@ankurah/storage-common"),
        "ankurah_storage_sqlite" => Some("@ankurah/storage-sqlite"),
        "ankurah_storage_postgres" => Some("@ankurah/storage-postgres"),
        "ankurah_websocket_client" => Some("@ankurah/connector-websocket"),
        "ankurah_websocket_server" => Some("@ankurah/connector-websocket-server"),
        "ankurah_connector_local_process" => Some("@ankurah/connector-local"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case() {
        assert_eq!(to_camel_case("snake_case"), "snakeCase");
        assert_eq!(to_camel_case("fetch_from_peer"), "fetchFromPeer");
        assert_eq!(to_camel_case("id"), "id");
        assert_eq!(to_camel_case("next_entity_id"), "nextEntityId");
    }

    #[test]
    fn test_fn_overrides() {
        assert_eq!(map_fn_name("fmt"), "toString");
        assert_eq!(map_fn_name("serialize"), "encode");
        assert_eq!(map_fn_name("eq"), "equals");
        assert_eq!(map_fn_name("fetch_from_peer"), "fetchFromPeer");
    }
}
