//! Deterministic name mapping: Rust identifiers → TypeScript identifiers.
//!
//! This file converts what the source *wrote*. `emit_ty` converts what the
//! engine *resolved*, and `shape` holds the one table both of those and the
//! native-type translations read, so that what is emitted and what is
//! dispatched on cannot drift apart.

mod emit_ty;
pub mod rust_spelling;
pub mod shape;
pub mod system_shapes;

pub use emit_ty::map_ty;

/// Convert snake_case to camelCase
pub fn to_camel_case(s: &str) -> String {
    // A leading underscore is Rust's "deliberately unused", not a word break:
    // `_sub1` is one name, and folding the underscore away made it `Sub1`,
    // which reads as a type.
    let leading = s.len() - s.trim_start_matches('_').len();
    let (underscores, s) = s.split_at(leading);
    let mut result = String::from(underscores);
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

/// The words JavaScript will not accept as a variable, parameter or function
/// name. A Rust identifier can be any of them — `let new = Self::from_entity(..)`
/// is ordinary Rust and `const new = ..` is a syntax error — so a name that
/// lands in one of these positions is written with a trailing underscore.
///
/// Property names are not in this position: `obj.default` and a method called
/// `delete` are legal JavaScript, so nothing renames a field or a method.
/// The list is ECMAScript's reserved words, plus the strict-mode reserved
/// words (a module is always strict), plus the three names a module scope
/// cannot rebind: `arguments` and `eval` are unassignable in strict mode, and
/// `undefined` shadows the global every other line relies on.
const RESERVED: [&str; 51] = [
    "arguments", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
    "default", "delete", "do", "else", "enum", "eval", "export", "extends", "false", "finally",
    "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let",
    "new", "null", "package", "private", "protected", "public", "return", "static", "super",
    "switch", "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while",
    "with", "yield", "as", "of",
];

/// The identifier a bound name is written under.
///
/// A Rust identifier can be written `r#type`, which is the name `type` with the
/// raw-identifier marker in front of it; the marker is Rust syntax and not part
/// of the name, so it comes off before the word is looked up.
pub fn escape_reserved(name: &str) -> String {
    let name = name.strip_prefix("r#").unwrap_or(name);
    if RESERVED.contains(&name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
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
        // `Ord::cmp` and `PartialOrd::partial_cmp` answer the same number
        // here — the port has one ordering method, and an explicit `.cmp(..)`
        // used to keep its Rust name and reach nothing.
        "cmp" => Some("compareTo"),
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
        "i64" | "u64" => "bigint",
        // The atomics: an atomic IS the value it holds, and this list is the
        // one `native_types::number::ATOMICS` lowers the constructors of.
        "AtomicBool" => "boolean",
        "AtomicU32" | "AtomicU64" | "AtomicUsize" => "number",
        "Infallible" => "never",
        "Rule" => "string", // pest grammar::Rule → string in TS (no pest equivalent)
        _ => rust_name,
    }
}

/// The tokio channel module a written path goes through, where it goes through
/// one. `tokio::sync::mpsc::Receiver` and a bare `mpsc::Receiver` both answer
/// `mpsc`.
fn channel_module(path: &syn::Path) -> Option<&'static str> {
    let segments = &path.segments;
    if segments.len() < 2 {
        return None;
    }
    match segments[segments.len() - 2].ident.to_string().as_str() {
        "oneshot" => Some("oneshot"),
        "mpsc" => Some("mpsc"),
        // `watch` is not here: the declared surface no longer offers it,
        // because the browser target has nothing behind it, so nothing can name
        // a type inside it for this to qualify.
        _ => None,
    }
}

/// Map a complex Rust type to TS type string
pub fn map_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                let mapped = map_type_name(&name);
                // tokio's channel modules stay namespaces in the runtime, so
                // `mpsc::Receiver<T>` is `mpsc.Receiver<T>`. The bare leaf name
                // collides with the other channel's `Receiver` and with
                // anything else called that.
                let qualifier = channel_module(&type_path.path);
                let mapped: &str = &match qualifier {
                    Some(module) => format!("{}.{}", module, mapped),
                    None => mapped.to_string(),
                };

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

                    // Every argument was a lifetime, which the port erases:
                    // `Self::Iter<'_>` has an argument list in Rust and none
                    // here. Writing the brackets anyway produced `Iter<>`,
                    // which a JavaScript engine refuses to parse — the one
                    // emitted file in the port that would not load at all.
                    if inner_types.is_empty() {
                        return mapped.to_string();
                    }
                    match name.as_str() {
                        "Vec" if inner_types.len() == 1 => {
                            if inner_types[0] == "number" && is_u8_vec(ty) {
                                "Uint8Array".to_string()
                            } else {
                                format!("{}[]", crate::name_map::emit_ty::as_an_element(&inner_types[0]))
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
                            format!("HashMap<{}, {}>", inner_types[0], inner_types[1])
                        }
                        "HashSet" | "BTreeSet" if inner_types.len() == 1 => {
                            format!("HashSet<{}>", inner_types[0])
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
            // ONLY `[u8]`. A `Uint8Array` truncates what it is given to a byte,
            // so `[i16]` written as one turns `-1` into `255`, and its methods
            // are not an array's, so `[u32]` written as one has no `push`.
            if is_u8(&slice.elem) {
                "Uint8Array".to_string()
            } else {
                format!("{}[]", crate::name_map::emit_ty::as_an_element(&map_type(&slice.elem)))
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
            // `[u8; N]` and nothing else — see the slice arm above.
            if is_u8(&arr.elem) {
                "Uint8Array".to_string()
            } else {
                format!("{}[]", crate::name_map::emit_ty::as_an_element(&map_type(&arr.elem)))
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
                            return Some(format!("{}[]", crate::name_map::emit_ty::as_an_element(&map_type(&assoc.ty))));
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

/// Is this written type `u8`? The one element type the port writes as bytes.
fn is_u8(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "u8"))
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

    /// An argument list whose every argument is a LIFETIME is no argument list
    /// here: the port erases lifetimes. `Self::Iter<'_>` was written `Iter<>`,
    /// which a JavaScript engine refuses to parse — `core/util/iterable.ts` was
    /// the one emitted file in the whole port that would not load at all.
    #[test]
    fn an_argument_list_of_only_lifetimes_is_no_list() {
        for written in ["Iter<'_>", "Iter<'a>", "Cow<'a>"] {
            let ty: syn::Type = syn::parse_str(written).unwrap();
            let out = map_type(&ty);
            assert!(!out.contains('<'), "{} became {}", written, out);
        }
        // A type argument beside a lifetime still stands.
        let ty: syn::Type = syn::parse_str("Iter<'a, T>").unwrap();
        assert_eq!(map_type(&ty), "Iter<T>");
    }

    /// A `Uint8Array` truncates every value it is given to a byte and does not
    /// carry an array's methods, so only the element type Rust calls a byte is
    /// written as one. `entity_offsets: IVec<usize, 8>` was a `Uint8Array`,
    /// where an offset past 255 becomes a different number entirely.
    #[test]
    fn only_a_slice_of_bytes_is_written_as_bytes() {
        let bytes: syn::Type = syn::parse_str("&[u8]").unwrap();
        assert_eq!(map_type(&bytes), "Uint8Array");
        let array: syn::Type = syn::parse_str("[u8; 32]").unwrap();
        assert_eq!(map_type(&array), "Uint8Array");
        for written in ["&[usize]", "&[i16]", "&[u32]", "&[f64]", "[usize; 8]"] {
            let ty: syn::Type = syn::parse_str(written).unwrap();
            assert_eq!(map_type(&ty), "number[]", "{} is not bytes", written);
        }
    }

    #[test]
    fn test_fn_overrides() {
        assert_eq!(map_fn_name("fmt"), "toString");
        assert_eq!(map_fn_name("serialize"), "encode");
        assert_eq!(map_fn_name("eq"), "equals");
        assert_eq!(map_fn_name("fetch_from_peer"), "fetchFromPeer");
    }

    use super::*;

    #[test]
    fn a_name_javascript_reserves_is_written_with_a_suffix() {
        // The ones a Rust identifier can be and JavaScript cannot.
        for word in ["new", "default", "class", "yield", "with", "eval", "arguments", "of", "as"] {
            assert_eq!(escape_reserved(word), format!("{word}_"), "{word}");
        }
        // And the ones it can.
        for word in ["entity", "collection", "id", "drop", "value"] {
            assert_eq!(escape_reserved(word), word, "{word}");
        }
    }

    #[test]
    fn a_raw_identifier_loses_its_marker_before_the_word_is_looked_up() {
        assert_eq!(escape_reserved("r#type"), "type");
        assert_eq!(escape_reserved("r#new"), "new_");
    }

    #[test]
    fn a_leading_underscore_is_kept_rather_than_read_as_a_word_break() {
        assert_eq!(to_camel_case("_sub1"), "_sub1");
        assert_eq!(to_camel_case("_unused_value"), "_unusedValue");
        assert_eq!(to_camel_case("entity_id"), "entityId");
    }
}

/// Does this written type name a type alias the port emits under its own name?
///
/// A resolved type has no memory of the alias it was written as, so a signature
/// written from it turns `Listener` into the `Arc<dyn Fn(T)>` the alias stands
/// for. The question is asked of the SYNTAX, and recursively: `&Listener`,
/// `Vec<Listener>` and `Arc<Listener>` each name one as surely as a bare
/// `Listener` does.
///
/// A field carries no module of its own, so the alias is looked for by its leaf
/// name across the crate. Two modules declaring one alias name is a shape the
/// port has never had, and a false yes costs only the syntactic spelling, which
/// is what the source wrote.
pub fn names_an_alias(reg: &crate::registry::TypeRegistry, written: &syn::Type) -> bool {
    match written {
        syn::Type::Path(path) => {
            if let Some(last) = path.path.segments.last() {
                if reg.has_alias_named(&last.ident.to_string()) {
                    return true;
                }
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    return args.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(ty) => names_an_alias(reg, ty),
                        _ => false,
                    });
                }
            }
            false
        }
        syn::Type::Reference(r) => names_an_alias(reg, &r.elem),
        syn::Type::Paren(p) => names_an_alias(reg, &p.elem),
        syn::Type::Group(g) => names_an_alias(reg, &g.elem),
        syn::Type::Slice(s) => names_an_alias(reg, &s.elem),
        syn::Type::Array(a) => names_an_alias(reg, &a.elem),
        syn::Type::Tuple(t) => t.elems.iter().any(|e| names_an_alias(reg, e)),
        _ => false,
    }
}
