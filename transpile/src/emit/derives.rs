//! The methods a `derive` stands for, written out.
//!
//! For: Rust's derives are code the compiler writes, and the port has to write
//! the same code — a clone that copies every field, an equality that compares
//! them, a hash over them in declaration order — because the runtime dispatches
//! those by name and a missing one is a `TypeError` at the call.

use super::*;

pub(super) fn emit_derive_methods(
    out: &mut String,
    reg: &TypeRegistry,
    type_name: &str,
    generics: &str,
    derives: &[String],
    emitted: &mut HashSet<String>,
    fields: &[crate::types::FieldInfo],
    ordering_of: Option<(String, Vec<crate::derives::Gap>)>,
    hash_of: Option<String>,
    // An enum's `equals`, which has no fields of its own to compare: it
    // switches on the variant and compares the payload, the way its
    // `compareTo` already does. Handed in ready-made, because an enum's
    // variants are not this function's business.
    equality_of: Option<String>,
) {
    let full_type = format!("{}{}", type_name, strip_generic_defaults(generics));
    let field_names: Vec<&str> = fields.iter()
        .filter_map(|f| f.name.as_deref())
        .collect();

    // Emit in consistent order: equals, compareTo, clone, default
    // (matches hand-port convention)
    let derive_set: std::collections::HashSet<&str> = derives.iter().map(|s| s.as_str()).collect();

    if derive_set.contains("PartialEq") || derive_set.contains("Eq") {
        if emitted.insert("equals".to_string()) {
            if let Some(variants) = equality_of {
                out.push_str(&variants);
            } else if field_names.is_empty() {
                out.push_str(&format!("\n  equals(other: {}): boolean {{\n    return true;\n  }}\n", full_type));
            } else {
                // Generate field-by-field equality with null safety
                out.push_str(&format!("\n  equals(other: {}): boolean {{\n", full_type));
                // The nullable case is `compare`'s own, written once. Written a
                // second time here it was spelled `=== null`, which a field
                // holding `undefined` slips past where `compare`'s `== null`
                // does not — and it stripped ` | null` off the RENDERED type, so
                // `(Token | null)[]` lost its ELEMENT's nullability rather than
                // its own and compared two nulls with `equals`.
                for f in fields {
                    let n = match f.name.as_deref() {
                        Some(n) => n,
                        None => continue,
                    };
                    out.push_str(&format!("    {}\n", emit_field_eq(reg, n, f.ty.as_ref())));
                }
                out.push_str("    return true;\n  }\n");
            }
        }
    }

    // `#[derive(Hash)]` is what makes a type usable as a key: the runtime's
    // `HashMap` and `HashSet` file a key under its own `hash()` and refuse one
    // that declares none, because a container that silently answered nothing
    // for every key is worse than one that says so.
    if derive_set.contains("Hash") && emitted.insert("hash".to_string()) {
        out.push_str(&hash_of.unwrap_or_else(|| {
            format!("\n  hash(): string {{\n    return '{}';\n  }}\n", type_name)
        }));
    }

    if derive_set.contains("PartialOrd") || derive_set.contains("Ord") {
        if emitted.insert("compareTo".to_string()) {
            // The derive compares field by field in declaration order and stops
            // at the first pair that differs. Writing `throw new Error('TODO')`
            // compiled and then threw the moment anything ordered the type.
            match ordering_of {
                Some((written, gaps)) => {
                    out.push_str(&written);
                    crate::derives::report(gaps);
                }
                None => out.push_str(&format!(
                    "\n  compareTo(other: {}): number {{\n    return 0;\n  }}\n",
                    full_type
                )),
            }
        }
    }

    // clone and default are emitted below
    for derive in derives {
        match derive.as_str() {
            "Clone" => {
                if emitted.insert("clone".to_string()) {
                    if field_names.is_empty() {
                        out.push_str(&format!("\n  clone(): {} {{\n    return new {}();\n  }}\n", full_type, type_name));
                    } else {
                        let clone_fields: Vec<String> = fields.iter()
                            .filter_map(|f| {
                                let n = f.name.as_deref()?;
                                Some(crate::derives::cloning::clone_within(
                                    reg,
                                    &format!("this.{}", n),
                                    f.ty.as_ref(),
                                ))
                            })
                            .collect();
                        out.push_str(&format!("\n  clone(): {} {{\n    return new {}({});\n  }}\n",
                            full_type, type_name, clone_fields.join(", ")));
                    }
                }
            }
            // PartialEq/Eq and PartialOrd/Ord already emitted above in consistent order
            "PartialEq" | "Eq" | "PartialOrd" | "Ord" => {}
            "Default" => {
                if emitted.insert("default".to_string()) {
                    let static_generics = merge_class_type_params_for_static("", &full_type);
                    if field_names.is_empty() {
                        out.push_str(&format!("\n  static default{}(): {} {{\n    return new {}();\n  }}\n", static_generics, full_type, type_name));
                    } else {
                        // What each field's `Default::default()` is, read off
                        // the field's *type*. Reading its TypeScript spelling
                        // wrote `Arc<RwLock<Map<K, V>>>.default()`, which names
                        // a type where a value belongs.
                        let default_fields: Vec<String> = fields.iter()
                            .map(|f| match f.ty.as_ref() {
                                Some(ty) => crate::derives::default_value::default_value(reg, ty)
                                    .unwrap_or_else(|why| {
                                        crate::diag::pending::park(
                                            f.rust_ty_span(),
                                            format!(
                                                "`{}`'s derived `Default` has no value for this field, because {}",
                                                type_name, why
                                            ),
                                        );
                                        "undefined".to_string()
                                    }),
                                None => {
                                    crate::diag::pending::park(
                                        f.rust_ty_span(),
                                        format!(
                                            "`{}`'s derived `Default` has no value for this field, because the engine could not type it",
                                            type_name
                                        ),
                                    );
                                    "undefined".to_string()
                                }
                            })
                            .collect();
                        out.push_str(&format!("\n  static default{}(): {} {{\n    return new {}({});\n  }}\n",
                            static_generics, full_type, type_name, default_fields.join(", ")));
                    }
                }
            }
            _ => {}
        }
    }
}

fn emit_field_eq(
    reg: &crate::registry::TypeRegistry,
    name: &str,
    ty: Option<&crate::ty::Ty>,
) -> String {
    crate::derives::equality::field_eq_within(
        reg,
        &format!("this.{}", name),
        &format!("other.{}", name),
        ty,
    )
}
