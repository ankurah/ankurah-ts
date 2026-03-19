//! TS code emission — emit structs, enums, traits, functions as TS text

use std::collections::{HashMap, HashSet};

use crate::types::*;

// ── Top-level emitters ──────────────────────────────────────────────────

pub fn emit_struct(
    out: &mut String,
    s: &StructInfo,
    inherent_methods: &HashMap<String, Vec<&FnInfo>>,
    trait_impls: &HashMap<String, Vec<&str>>,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo)>>,
) {
    let export = if s.is_pub { "export " } else { "" };
    let traits = trait_impls.get(&s.name);
    let has_drop_impl = traits.map(|t| t.contains(&"Drop")).unwrap_or(false);
    let base = if has_drop_impl { " extends Drop" } else { " extends Struct" };
    let self_type = format!("{}{}", s.name, s.generics);
    let implements = format_implements(traits);

    out.push_str(&format!("{}class {}{}{}{} {{\n", export, s.name, s.generics, base, implements));

    // Fields — public fields are readonly, private fields are mutable
    for f in &s.fields {
        if let Some(name) = &f.name {
            if f.is_pub {
                out.push_str(&format!("  readonly {}: {};\n", name, f.ty));
            } else {
                out.push_str(&format!("  private {}: {};\n", name, f.ty));
            }
        }
    }

    // Constructor with field assignments
    if !s.fields.is_empty() {
        out.push('\n');
        let params: Vec<String> = s.fields.iter()
            .filter_map(|f| f.name.as_ref().map(|n| format!("{}: {}", n, f.ty)))
            .collect();
        out.push_str(&format!("  constructor({}) {{\n    super();\n", params.join(", ")));
        for f in &s.fields {
            if let Some(name) = &f.name {
                out.push_str(&format!("    this.{} = {};\n", name, name));
            }
        }
        out.push_str("  }\n");
    }

    // Methods
    let mut emitted = HashSet::new();
    emit_inherent_methods(out, &self_type, inherent_methods, &mut emitted);
    emit_trait_methods(out, &self_type, trait_methods, &mut emitted);
    emit_derive_methods(out, &s.name, &s.generics, &s.derives, &mut emitted, &s.fields);
    emit_struct_bincode(out, s, trait_impls);

    out.push_str("}\n\n");
}

pub fn emit_enum(
    out: &mut String,
    e: &EnumInfo,
    inherent_methods: &HashMap<String, Vec<&FnInfo>>,
    _trait_impls: &HashMap<String, Vec<&str>>,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo)>>,
) {
    let export = if e.is_pub { "export " } else { "" };

    // Variant type map
    out.push_str(&format!("{}type {}V = {{\n", export, e.name));
    for v in &e.variants {
        if v.fields.is_empty() {
            out.push_str(&format!("  {}: {{}};\n", v.name));
        } else {
            let fields: Vec<String> = v.fields.iter()
                .filter_map(|f| f.name.as_ref().map(|n| format!("{}: {}", n, f.ty)))
                .collect();
            out.push_str(&format!("  {}: {{ {} }};\n", v.name, fields.join("; ")));
        }
    }
    out.push_str("};\n\n");

    // Class
    out.push_str(&format!("{}class {}{} extends Enum<{}V> {{\n", export, e.name, e.generics, e.name));

    let self_type = format!("{}{}", e.name, e.generics);
    let mut emitted = HashSet::new();
    emit_inherent_methods(out, &self_type, inherent_methods, &mut emitted);
    emit_trait_methods(out, &self_type, trait_methods, &mut emitted);
    emit_derive_methods(out, &e.name, &e.generics, &e.derives, &mut emitted, &[]);

    if crate::bincode_module::has_serde_derive(&e.derives) {
        out.push('\n');
        out.push_str(&crate::bincode_module::generate_enum_codec(e));
    }

    out.push_str("}\n\n");
}

pub fn emit_trait(out: &mut String, t: &TraitInfo) {
    let export = if t.is_pub { "export " } else { "" };
    let keyword = if t.has_default_impls { "abstract class" } else { "interface" };

    out.push_str(&format!("{}{} {}{} {{\n", export, keyword, t.name, t.generics));

    for method in &t.methods {
        let async_kw = if method.is_async { "async " } else { "" };
        let params = format_params(&method.params);
        if t.has_default_impls {
            out.push_str(&format!("  {}{}({}): {} {{ throw new Error('TODO'); }}\n",
                async_kw, method.ts_name, params, method.return_type));
        } else {
            out.push_str(&format!("  {}{}({}): {};\n",
                async_kw, method.ts_name, params, method.return_type));
        }
    }

    out.push_str("}\n\n");
}

pub fn emit_function(out: &mut String, f: &FnInfo) {
    let export = if f.is_pub { "export " } else { "" };
    let async_kw = if f.is_async { "async " } else { "" };
    let params = format_params(&f.params);
    let ret = if f.is_async && f.return_type != "void" {
        format!("Promise<{}>", f.return_type)
    } else {
        f.return_type.clone()
    };

    let body = if let Some(body_ts) = &f.body_ts {
        body_ts.lines()
            .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
            .collect::<Vec<_>>()
            .join("\n") + "\n"
    } else {
        "  throw new Error('TODO');\n".to_string()
    };

    out.push_str(&format!("{}{}function {}({}): {} {{\n{}}}\n\n",
        export, async_kw, f.ts_name, params, ret, body));
}

// ── Method emitters ─────────────────────────────────────────────────────

fn emit_inherent_methods(
    out: &mut String,
    self_type: &str,
    inherent_methods: &HashMap<String, Vec<&FnInfo>>,
    emitted: &mut HashSet<String>,
) {
    let plain_name = self_type.split('<').next().unwrap_or(self_type);
    if let Some(methods) = inherent_methods.get(plain_name) {
        for method in methods {
            if emitted.insert(method.ts_name.clone()) {
                out.push('\n');
                emit_method(out, method, self_type);
            }
        }
    }
}

fn emit_trait_methods(
    out: &mut String,
    self_type: &str,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo)>>,
    emitted: &mut HashSet<String>,
) {
    let plain_name = self_type.split('<').next().unwrap_or(self_type);
    if let Some(trait_fns) = trait_methods.get(plain_name) {
        for (trait_name, type_args, method) in trait_fns {
            if let Some((base_ts_name, ret_override)) = trait_method_mapping(trait_name, &method.name) {
                let ts_name = disambiguate_trait_method(base_ts_name, trait_name, type_args, plain_name);
                if emitted.insert(ts_name.clone()) {
                    let m = FnInfo {
                        name: method.name.clone(),
                        ts_name,
                        is_pub: method.is_pub,
                        is_async: method.is_async,
                        is_static: method.is_static,
                        params: method.params.clone(),
                        return_type: ret_override.map(|s| s.to_string())
                            .unwrap_or_else(|| method.return_type.clone()),
                        generics: method.generics.clone(),
                        is_test: false,
                        body_ts: method.body_ts.clone(),
                    };
                    out.push('\n');
                    emit_method(out, &m, self_type);
                }
            }
        }
    }
}

fn emit_derive_methods(
    out: &mut String,
    type_name: &str,
    generics: &str,
    derives: &[String],
    emitted: &mut HashSet<String>,
    fields: &[crate::types::FieldInfo],
) {
    let full_type = format!("{}{}", type_name, generics);
    let field_names: Vec<&str> = fields.iter()
        .filter_map(|f| f.name.as_deref())
        .collect();

    // Emit in consistent order: equals, compareTo, clone, default
    // (matches hand-port convention)
    let derive_set: std::collections::HashSet<&str> = derives.iter().map(|s| s.as_str()).collect();

    if derive_set.contains("PartialEq") || derive_set.contains("Eq") {
        if emitted.insert("equals".to_string()) {
            if field_names.is_empty() {
                out.push_str(&format!("\n  equals(other: {}): boolean {{\n    return true;\n  }}\n", full_type));
            } else {
                let checks: Vec<String> = fields.iter()
                    .filter_map(|f| {
                        let n = f.name.as_deref()?;
                        if is_primitive_ts_type(&f.ty) {
                            Some(format!("this.{} === other.{}", n, n))
                        } else {
                            Some(format!("this.{}.equals(other.{})", n, n))
                        }
                    })
                    .collect();
                out.push_str(&format!("\n  equals(other: {}): boolean {{\n    return {};\n  }}\n",
                    full_type, checks.join(" && ")));
            }
        }
    }

    if derive_set.contains("PartialOrd") || derive_set.contains("Ord") {
        if emitted.insert("compareTo".to_string()) {
            out.push_str(&format!("\n  compareTo(other: {}): number {{\n    throw new Error('TODO');\n  }}\n", full_type));
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
                            .filter_map(|f| f.name.as_deref())
                            .map(|n| {
                                // Primitives don't need .clone()
                                let field_type = fields.iter().find(|f| f.name.as_deref() == Some(n))
                                    .map(|f| f.ty.as_str()).unwrap_or("");
                                if is_primitive_ts_type(field_type) {
                                    format!("this.{}", n)
                                } else {
                                    format!("this.{}.clone()", n)
                                }
                            })
                            .collect();
                        out.push_str(&format!("\n  clone(): {} {{\n    return new {}({});\n  }}\n",
                            full_type, type_name, clone_fields.join(", ")));
                    }
                }
            }
            // PartialEq/Eq and PartialOrd/Ord already emitted above in consistent order
            "PartialEq" | "Eq" | "PartialOrd" | "Ord" => {}
            _ => {}
        }
    }
}

fn is_primitive_ts_type(ty: &str) -> bool {
    matches!(ty, "string" | "boolean" | "number" | "bigint | number")
}

fn emit_struct_bincode(
    out: &mut String,
    s: &StructInfo,
    trait_impls: &HashMap<String, Vec<&str>>,
) {
    let has_custom_serde = trait_impls.get(&s.name)
        .map(|t| t.contains(&"Serialize"))
        .unwrap_or(false)
        && !s.derives.iter().any(|d| d == "Serialize");

    if !has_custom_serde && crate::bincode_module::has_serde_derive(&s.derives) {
        out.push('\n');
        if s.fields.iter().all(|f| f.name.is_some()) {
            out.push_str(&crate::bincode_module::generate_struct_codec(s));
        } else {
            out.push_str(&crate::bincode_module::generate_tuple_struct_codec(s));
        }
    }
}

fn emit_method(out: &mut String, method: &FnInfo, self_type: &str) {
    let static_kw = if method.is_static { "static " } else { "" };
    let async_kw = if method.is_async { "async " } else { "" };
    let generics = &method.generics;
    let params = format_params_filtered(&method.params);
    let ret = resolve_self_type(&method.return_type, self_type);
    let ret = if method.is_async && ret != "void" {
        format!("Promise<{}>", ret)
    } else {
        ret
    };

    let body = if let Some(body_ts) = &method.body_ts {
        indent_body(body_ts)
    } else {
        "    throw new Error('TODO');\n".to_string()
    };

    out.push_str(&format!("  {}{}{}{}({}): {} {{\n{}  }}\n",
        static_kw, async_kw, method.ts_name, generics, params, ret, body));
}

/// Indent body lines by 4 spaces (2 for class, 2 for method body)
fn indent_body(body: &str) -> String {
    body.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("    {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn resolve_self_type(ty: &str, self_type: &str) -> String {
    if ty == "Self" { self_type.to_string() } else { ty.replace("Self", self_type) }
}

fn format_params(params: &[ParamInfo]) -> String {
    params.iter()
        .filter(|p| !p.is_self)
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_params_filtered(params: &[ParamInfo]) -> String {
    params.iter()
        .filter(|p| !p.is_self && !is_rust_only_type(&p.ty))
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_implements(traits: Option<&Vec<&str>>) -> String {
    if let Some(traits) = traits {
        let ifaces: Vec<&str> = traits.iter()
            .filter(|t| !is_skipped_trait(t))
            .copied()
            .collect();
        if ifaces.is_empty() { String::new() } else { format!(" implements {}", ifaces.join(", ")) }
    } else {
        String::new()
    }
}

fn is_rust_only_type(ty: &str) -> bool {
    ty.contains("Formatter") || ty.contains("Serializer") || ty.contains("Deserializer")
        || ty == "S" || ty == "D"
}

fn is_skipped_trait(trait_name: &str) -> bool {
    matches!(trait_name,
        "Display" | "Debug" | "FromStr" | "TryFrom" | "From" |
        "Serialize" | "Deserialize" | "Clone" | "PartialEq" | "Eq" |
        "PartialOrd" | "Ord" | "Hash" | "Default" | "Send" | "Sync" |
        "Deref" | "DerefMut" | "Into" | "TryInto" | "IntoIterator"
    )
}

fn disambiguate_trait_method(base_name: &str, trait_name: &str, type_args: &[String], _self_type: &str) -> String {
    if !matches!(trait_name, "From" | "TryFrom" | "TryInto" | "Into") || type_args.is_empty() {
        return base_name.to_string();
    }
    let source = &type_args[0];

    if source.ends_with("[]") || source.starts_with("Map<") || source.starts_with("Set<")
        || source.contains("Uint8Array")
        || matches!(source.as_str(), "string" | "boolean" | "number" | "bigint | number")
    {
        return base_name.to_string();
    }

    if source.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !source.contains('<') && !source.contains(',') && !source.contains(' ')
    {
        return format!("from{}", source);
    }

    base_name.to_string()
}

fn trait_method_mapping(trait_name: &str, rust_method_name: &str) -> Option<(&'static str, Option<&'static str>)> {
    match (trait_name, rust_method_name) {
        ("Display", "fmt") => Some(("toString", Some("string"))),
        ("Debug", "fmt") => None,
        ("Default", "default") => Some(("default", None)),
        ("Clone", "clone") => Some(("clone", None)),
        ("PartialEq", "eq") => Some(("equals", Some("boolean"))),
        ("PartialOrd", "partial_cmp") => Some(("compareTo", Some("number"))),
        ("From", "from") => Some(("from", None)),
        ("TryFrom", "try_from") => Some(("tryFrom", None)),
        ("TryInto", "try_into") => Some(("from", None)),
        ("Into", "into") => Some(("from", None)),
        ("FromStr", "from_str") => Some(("fromStr", None)),
        ("IntoIterator", "into_iter") => Some(("iter", None)),
        _ => None,
    }
}
