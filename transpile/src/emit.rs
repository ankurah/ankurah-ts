//! TS code emission — emit structs, enums, traits, functions as TS text

use std::collections::{HashMap, HashSet};

use crate::registry::TypeRegistry;
use crate::types::*;

// ── Top-level emitters ──────────────────────────────────────────────────

pub fn emit_struct(
    out: &mut String,
    reg: &TypeRegistry,
    module: Option<crate::registry::ModuleId>,
    s: &StructInfo,
    inherent_methods: &HashMap<String, Vec<&FnInfo>>,
    trait_impls: &HashMap<String, Vec<(&str, &[String])>>,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo, &[String])>>,
    impl_bounds: Option<&HashMap<String, Vec<String>>>,
    assigned: &HashSet<String>,
) {
    let export = if s.is_pub { "export " } else { "" };
    let traits = trait_impls.get(&s.name);
    let has_drop_impl = traits.map(|t| t.iter().any(|(n, _)| *n == "Drop")).unwrap_or(false);
    let base = if has_drop_impl { " extends Drop" } else { " extends Struct" };
    // Merge impl bounds into class generics declaration
    let generics_decl = if let Some(bounds) = impl_bounds {
        merge_bounds_into_generics(&s.generics, bounds)
    } else {
        s.generics.clone()
    };
    let generics_usage = strip_generic_defaults(&generics_decl);
    let self_type = format!("{}{}", s.name, generics_usage);
    // The identity of the class being written, which is what the conversion
    // naming decision is keyed by. The leaf alone made two unrelated `Wrap`
    // classes in different modules contest each other's names.
    let self_id = module.and_then(|m| reg.module_type(m, &s.name));
    let implements = format_implements(reg, traits);

    out.push_str(&format!("{}class {}{}{}{} {{\n", export, s.name, generics_decl, base, implements));

    // Fields — Rust's "private" means module-private (same file), not class-private.
    // Since types within the same Rust module routinely access each other's fields,
    // we don't emit TS `private` — all fields are accessible (default public in TS classes).
    // A public Rust field is `readonly` for external consumers, unless one of
    // this type's own methods writes it: `fn drop(&mut self)` and every other
    // `&mut self` body assigns through the receiver, and TypeScript refuses
    // that on a `readonly` property.
    for f in &s.fields {
        if is_phantom_field(reg, f) { continue; }
        if let Some(name) = &f.name {
            if f.is_pub && !assigned.contains(name.as_str()) {
                out.push_str(&format!("  readonly {}: {};\n", name, f.ts_ty(reg)));
            } else {
                out.push_str(&format!("  {}: {};\n", name, f.ts_ty(reg)));
            }
        }
    }

    // Constructor with field assignments (skip PhantomData fields)
    let real_fields: Vec<&FieldInfo> = s.fields.iter().filter(|f| !is_phantom_field(reg, f)).collect();
    if !real_fields.is_empty() {
        out.push('\n');
        let params: Vec<String> = real_fields.iter()
            .filter_map(|f| f.name.as_ref().map(|n| format!("{}: {}", n, f.ts_ty(reg))))
            .collect();
        out.push_str(&format!("  constructor({}) {{\n    super();\n", params.join(", ")));
        for f in &real_fields {
            if let Some(name) = &f.name {
                out.push_str(&format!("    this.{} = {};\n", name, name));
            }
        }
        out.push_str("  }\n");
    }

    emit_owned_fields(out, reg, &real_fields);

    // Methods
    let mut emitted = HashSet::new();
    emit_inherent_methods(out, &self_type, inherent_methods, &mut emitted);
    emit_trait_methods(out, &self_type, self_id, trait_methods, &mut emitted);
    // The ordering a `PartialOrd`/`Ord` derive writes needs the whole
    // declaration, which `emit_derive_methods` is not handed.
    let ordering = crate::derives::ordering::struct_compare(
        reg,
        s,
        &format!("{}{}", s.name, strip_generic_defaults(&s.generics)),
    );
    emit_derive_methods(
        out, reg, &s.name, &s.generics, &s.derives, &mut emitted, &s.fields, Some(ordering),
        Some(crate::derives::hashing::struct_hash(reg, s)),
        None,
    );
    // The derives that write code rather than only proving an impl: Debug's
    // `debug()`, and thiserror's `toString`/`from`. `emitted` keeps a
    // hand-written `impl Display` ahead of a derived one, the way Rust's
    // coherence would.
    let (derived, gaps) = crate::derives::struct_members(reg, s, &mut emitted);
    out.push_str(&derived);
    crate::derives::report(gaps);

    // Deref delegation for wrapper types (tuple structs with a single field)
    let has_deref = traits.map(|t| t.iter().any(|(n, _)| *n == "Deref")).unwrap_or(false);
    if has_deref && s.fields.len() == 1 {
        if let Some(field_name) = s.fields[0].name.as_deref() {
            let inner_ty = s.fields[0].ts_ty(reg);
            if inner_ty.ends_with("[]") {
                out.push_str(&format!("\n  get length(): number {{\n    return this.{}.length;\n  }}\n", field_name));
                out.push_str(&format!("\n  [Symbol.iterator](): Iterator<any> {{\n    return this.{}[Symbol.iterator]();\n  }}\n", field_name));
            } else if inner_ty.starts_with("HashMap<") {
                out.push_str(&format!("\n  get size(): number {{\n    return this.{}.size;\n  }}\n", field_name));
                out.push_str(&format!("\n  [Symbol.iterator](): Iterator<any> {{\n    return this.{}[Symbol.iterator]();\n  }}\n", field_name));
                out.push_str(&format!("\n  entries(): IterableIterator<any> {{\n    return this.{}.entries();\n  }}\n", field_name));
                out.push_str(&format!("\n  get(key: any): any {{\n    return this.{}.get(key);\n  }}\n", field_name));
            }
        }
    }
    emit_struct_bincode(out, reg, module, s, trait_impls);

    out.push_str("}\n\n");
}

pub fn emit_enum(
    out: &mut String,
    reg: &TypeRegistry,
    module: Option<crate::registry::ModuleId>,
    e: &EnumInfo,
    inherent_methods: &HashMap<String, Vec<&FnInfo>>,
    _trait_impls: &HashMap<String, Vec<(&str, &[String])>>,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo, &[String])>>,
) {
    let export = if e.is_pub { "export " } else { "" };

    // Variant type map
    out.push_str(&format!("{}type {}V = {{\n", export, e.name));
    for v in &e.variants {
        if v.fields.is_empty() {
            out.push_str(&format!("  {}: {{}};\n", v.name));
        } else {
            let fields: Vec<String> = v.fields.iter()
                .filter_map(|f| f.name.as_ref().map(|n| format!("{}: {}", n, f.ts_ty(reg))))
                .collect();
            out.push_str(&format!("  {}: {{ {} }};\n", v.name, fields.join("; ")));
        }
    }
    out.push_str("};\n\n");

    // Class
    out.push_str(&format!("{}class {}{} extends Enum<{}V> {{\n", export, e.name, e.generics, e.name));

    let generics_usage = strip_generic_defaults(&e.generics);
    let self_type = format!("{}{}", e.name, generics_usage);
    let self_id = module.and_then(|m| reg.module_type(m, &e.name));
    let mut emitted = HashSet::new();
    emit_inherent_methods(out, &self_type, inherent_methods, &mut emitted);
    emit_trait_methods(out, &self_type, self_id, trait_methods, &mut emitted);

    // Enum-specific derive handling (clone needs variant-aware logic)
    if e.derives.iter().any(|d| d == "Clone") && emitted.insert("clone".to_string()) {
        // Clone via match — deep-clone each variant's fields
        let has_complex_fields = e.variants.iter().any(|v| v.fields.iter().any(|f| {
            let ty = f.ts_ty(reg);
            !is_primitive_ts_type(&ty) && ty != "Uint8Array"
        }));
        if has_complex_fields {
            out.push_str(&format!("\n  clone(): {} {{\n    return this.match({{\n", self_type));
            for v in &e.variants {
                if v.fields.is_empty() {
                    out.push_str(&format!("      {}: () => new {}('{}', {{}}),\n", v.name, e.name, v.name));
                } else {
                    let clone_fields: Vec<String> = v.fields.iter()
                        .filter_map(|f| {
                            let n = f.name.as_deref()?;
                            let ty = f.ts_ty(reg);
                            let base_ty = ty.trim_end_matches(" | null");
                            let nullable = ty.ends_with(" | null");
                            Some(if is_primitive_ts_type(base_ty) {
                                format!("{}: v.{}", n, n)
                            } else if base_ty == "Uint8Array" {
                                if nullable {
                                    format!("{}: v.{} != null ? new Uint8Array(v.{}) : null", n, n, n)
                                } else {
                                    format!("{}: new Uint8Array(v.{})", n, n)
                                }
                            } else if base_ty.ends_with("[]") {
                                let inner = &base_ty[..base_ty.len()-2];
                                if is_primitive_ts_type(inner) {
                                    format!("{}: [...v.{}]", n, n)
                                } else {
                                    format!("{}: v.{}.map(e => e.clone())", n, n)
                                }
                            } else if nullable {
                                format!("{}: v.{}?.clone() ?? null", n, n)
                            } else {
                                format!("{}: v.{}.clone()", n, n)
                            })
                        })
                        .collect();
                    out.push_str(&format!("      {}: (v) => new {}('{}', {{ {} }}),\n",
                        v.name, e.name, v.name, clone_fields.join(", ")));
                }
            }
            out.push_str("    });\n  }\n");
        } else {
            // Simple enum — shallow copy is sufficient
            out.push_str(&format!("\n  clone(): {} {{\n    return new {}(this.type, {{ ...this.value }});\n  }}\n",
                self_type, e.name));
        }
    }

    // Handle remaining derives (PartialEq, Default, etc.) — pass empty fields for enums
    let ordering = crate::derives::ordering::enum_compare(
        reg,
        e,
        &format!("{}{}", e.name, strip_generic_defaults(&e.generics)),
    );
    emit_derive_methods(
        out, reg, &e.name, &e.generics, &e.derives, &mut emitted, &[], Some(ordering),
        Some(crate::derives::hashing::enum_hash(reg, e)),
        Some(crate::derives::equality::enum_equals(reg, e, &format!("{}{}", e.name, strip_generic_defaults(&e.generics)))),
    );
    // `emitted` already holds every method a written impl put on the class, so a
    // hand-written `Display` keeps its `toString` and the derive does not write
    // a second one over it.
    let (derived, gaps) = crate::derives::enum_members(reg, self_id, e, &mut emitted);
    out.push_str(&derived);
    crate::derives::report(gaps);

    if crate::bincode_module::has_serde_derive(&e.derives) {
        out.push('\n');
        out.push_str(&crate::bincode_module::generate_enum_codec(reg, e));
        // The human-readable half of the same derive. A type whose fields have
        // no JSON spelling gets neither method rather than half a pair, and the
        // registry has already said why (`narrow_reads_json`) — asking again
        // here would file the same refusal a second time.
        if self_id.is_some_and(|id| reg.reads_json(id)) {
            match crate::json_module::enum_json(reg, e) {
                Ok(json) if emitted.insert("toJSON".to_string()) => {
                    out.push('\n');
                    out.push_str(&json);
                }
                Ok(_) => {}
                Err(reason) => {
                    crate::diag::pending::park_at(0, 0, format!("`{}`: {}", e.name, reason))
                }
            }
        }
    }

    out.push_str("}\n\n");
}

/// What a caller sees an `async fn` return.
///
/// Every `async fn` returns a promise, including one that returns nothing:
/// `async f(): void` is not TypeScript, `async f(): Promise<void>` is. The
/// unit case used to be left alone, so every async method returning `()` was
/// emitted with a type TypeScript rejects.
fn async_return(is_async: bool, ret: &str) -> String {
    if is_async {
        format!("Promise<{}>", ret)
    } else {
        ret.to_string()
    }
}

pub fn emit_trait(out: &mut String, t: &TraitInfo) {
    let export = if t.is_pub { "export " } else { "" };
    let keyword = if t.has_default_impls { "abstract class" } else { "interface" };

    out.push_str(&format!("{}{} {}{} {{\n", export, keyword, t.name, t.generics));

    for method in &t.methods {
        let params = format_params(&method.params);
        let ret = async_return(method.is_async, &method.return_type);
        // A method the trait wrote a body for is emitted with it; one it only
        // declared stays abstract, so an implementor that omits it is a
        // TypeScript error rather than a runtime throw.
        //
        // `async` is how a body is written, not part of what a caller may pass
        // or expect, and TypeScript rejects it on a declaration — on an
        // `abstract` member and on an interface member alike. The promise in the
        // return type is what the declaration has to say, and it says it.
        match (&method.body_ts, t.has_default_impls) {
            (Some(body), _) => {
                let async_kw = if method.is_async { "async " } else { "" };
                out.push_str(&format!(
                    "  {}{}({}): {} {{\n{}  }}\n",
                    async_kw,
                    method.ts_name,
                    params,
                    ret,
                    indent_body(body)
                ));
            }
            (None, true) => out.push_str(&format!(
                "  abstract {}({}): {};\n",
                method.ts_name, params, ret
            )),
            (None, false) => out.push_str(&format!(
                "  {}({}): {};\n",
                method.ts_name, params, ret
            )),
        }
    }

    out.push_str("}\n\n");
}

pub fn emit_function(out: &mut String, f: &FnInfo) {
    let export = if f.is_pub { "export " } else { "" };
    let async_kw = if f.is_async { "async " } else { "" };
    let params = format_params(&f.params);
    let ret = async_return(f.is_async, &f.return_type);

    let body = if let Some(body_ts) = &f.body_ts {
        body_ts.lines()
            .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
            .collect::<Vec<_>>()
            .join("\n") + "\n"
    } else {
        "  throw new Error('TODO');\n".to_string()
    };

    out.push_str(&format!("{}{}function {}{}({}): {} {{\n{}}}\n\n",
        export, async_kw, f.ts_name, f.generics, params, ret, body));
}

// ── Method emitters ─────────────────────────────────────────────────────

/// Every field name something in this file assigns.
///
/// Rust's `pub` means readable *and* writable by anyone holding a `&mut`, so a
/// field that anything assigns cannot carry TypeScript's `readonly` — and the
/// assignment may be in a free function or another type's method, not only in
/// the type's own. The question is asked of the Rust, because `*guard = v`
/// emits `guard.value = v` and would otherwise take `readonly` off every field
/// in the crate that happens to be called `value`.
pub fn assigned_fields(file: &crate::types::RustFile) -> HashSet<String> {
    struct Assigned {
        names: HashSet<String>,
    }
    impl Assigned {
        fn record(&mut self, place: &syn::Expr) {
            if let syn::Expr::Field(field) = place {
                if let syn::Member::Named(ident) = &field.member {
                    self.names
                        .insert(crate::name_map::to_camel_case(&ident.to_string()));
                }
            }
        }
    }
    impl syn::visit::Visit<'_> for Assigned {
        fn visit_expr(&mut self, expr: &syn::Expr) {
            match expr {
                syn::Expr::Assign(assign) => self.record(&assign.left),
                syn::Expr::Binary(bin) if crate::body::is_assign_op(&bin.op) => self.record(&bin.left),
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }
    let mut found = Assigned { names: HashSet::new() };
    for imp in &file.impls {
        for method in &imp.methods {
            if let Some(block) = &method.body_ast {
                syn::visit::Visit::visit_block(&mut found, block);
            }
        }
    }
    for f in file.functions.iter().chain(&file.test_functions) {
        if let Some(block) = &f.body_ast {
            syn::visit::Visit::visit_block(&mut found, block);
        }
    }
    let mut names = found.names;
    for (_, inner) in &file.inline_modules {
        names.extend(assigned_fields(inner));
    }
    names
}

/// Say what this type owns, where saying nothing would be wrong.
///
/// The drop cascade walks a value's own properties, and a field holding `&T`
/// looks exactly like a field holding `T`. Rust's drop of a borrow releases
/// nothing — `struct Ref<'a, T>(&'a Broadcast<T>)` does not release the
/// broadcast — so a type with a reference field lists what it really owns and
/// the cascade steps over the rest. Without this, dropping the borrow dropped
/// the borrowed value, and its real owner's drop was then a double drop.
///
/// The question is asked of the whole field type, not of its outermost layer:
/// a `Vec<&T>` is an array of borrows and a `HashMap<K, &V>` still owns its
/// keys.
fn emit_owned_fields(out: &mut String, reg: &crate::registry::TypeRegistry, fields: &[&FieldInfo]) {
    let borrows = |f: &&FieldInfo| {
        f.ty.as_ref()
            .is_some_and(|ty| crate::ownership::places::borrows_only(reg, ty))
    };
    if !fields.iter().any(borrows) {
        return;
    }
    let owned: Vec<String> = fields
        .iter()
        .filter(|f| !borrows(f))
        .filter_map(|f| f.name.as_ref().map(|n| format!("this.{}", n)))
        .collect();
    out.push_str(&format!(
        "\n  // A `&T` field is a borrow: dropping this releases the borrow and \
         nothing\n  // else, so the cascade must not walk it.\n  \
         protected override ownedFields(): unknown[] {{\n    return [{}];\n  }}\n",
        owned.join(", ")
    ));
}

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
    self_id: Option<crate::ty::TypeId>,
    trait_methods: &HashMap<String, Vec<(&str, &[String], &FnInfo, &[String])>>,
    emitted: &mut HashSet<String>,
) {
    let plain_name = self_type.split('<').next().unwrap_or(self_type);
    // What each name this pass wrote actually looks like, so a second impl
    // wanting the same name can be told apart: two impls whose TypeScript
    // signatures are identical — `From<&str>` and `From<String>` are both
    // `from(v: string)` — are ONE method here, and that is a merge rather than
    // a loss. Two that differ are a loss, and the site says so.
    let mut signatures: HashMap<String, String> = HashMap::new();
    if let Some(trait_fns) = trait_methods.get(plain_name) {
        for (trait_name, type_args, method, impl_params) in trait_fns {
            // Skip From<Infallible> — unreachable code
            if *trait_name == "From" && type_args.iter().any(|a| a == "never" || a == "Infallible") {
                continue;
            }
            let ret_override = trait_method_mapping(trait_name, &method.name).and_then(|m| m.1);
            let ts_name = trait_method_name(trait_name, type_args, method, self_type, self_id);
            // `Drop` declares `onDrop` protected, so the override has to say so.
            let modifiers = if *trait_name == "Drop" { "protected override " } else { "" };
            let signature = format!(
                "{}|{}",
                method.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>().join(","),
                method.return_type
            );
            // R9: a body that does nothing but call the very name it is being
            // emitted under is a CYCLE, not an implementation.
            // `impl PartialOrd for HeapItem` writes
            // `fn partial_cmp(&self, o) -> Option<Ordering> { Some(self.cmp(o)) }`,
            // and `cmp` and `partial_cmp` are one method here: whichever the
            // source wrote first took the name, and when that was the
            // forwarding one, `compareTo` called itself. Every TopK heap
            // comparison overflowed the stack, and `equals`, written as
            // `compareTo(o) === 0`, went down with it. The forwarding body is
            // not written; the one with something in it keeps the name.
            if forwards_to_itself(method, &ts_name) {
                crate::diag::pending::park(
                    method
                        .body_ast
                        .as_ref()
                        .map(syn::spanned::Spanned::span)
                        .unwrap_or_else(proc_macro2::Span::call_site),
                    format!(
                        "`{}::{}` for `{}` is written as a call to `{}`, which is the name it \
                         is emitted under here, so writing it would be a method that calls \
                         itself; the impl that has a body of its own keeps the name",
                        trait_name, method.name, self_type, ts_name
                    ),
                );
                continue;
            }
            if !emitted.contains(&ts_name) {
                emitted.insert(ts_name.clone());
                signatures.insert(ts_name.clone(), signature);
                let m = FnInfo {
                    name: method.name.clone(),
                    ts_name,
                    is_pub: method.is_pub,
                    vis: method.vis,
                    is_async: method.is_async,
                    is_static: method.is_static,
                    self_kind: method.self_kind,
                    self_receiver: method.self_receiver.clone(),
                    has_default_body: method.has_default_body,
                    params: method.params.clone(),
                    return_type: ret_override.map(|s| s.to_string())
                        .unwrap_or_else(|| method.return_type.clone()),
                    rust_return: method.rust_return.clone(),
                    // The impl's OWN parameters, where the method's
                    // signature names them and the class does not declare
                    // them: `impl<T: Into<Expr>> From<Vec<T>> for Expr` writes
                    // a static whose parameter is `T[]`, and a `T` nothing
                    // declares is not a type at all.
                    generics: with_impl_params(&method.generics, impl_params, method, self_type),
                    type_params: method.type_params.clone(),
                    syn_generics: method.syn_generics.clone(),
                    is_test: false,
                    body_ast: None,
                    body_ts: method.body_ts.clone(),
                };
                out.push('\n');
                emit_method_with(out, &m, self_type, modifiers);
            } else if signatures.get(&ts_name) != Some(&signature)
                || !matches!(trait_name, &"From" | &"TryFrom")
            {
                // Two written impls whose methods land on one name: the second
                // used to be dropped without a word, and every call to it went
                // to the first — `impl Add<Right> for Weight` beside
                // `impl Add for Weight` are both `add`, and `weight + right`
                // called the one that takes a `Weight`.
                //
                // An identical signature is not evidence that the two are one
                // method: R8 retracts that reading, because
                // `From<bincode::Error>` and `From<anyhow::Error>` are both
                // `from(e: Error)` and have different bodies. For a CONVERSION
                // the naming post-pass has already settled which impls share a
                // name and reported the contests it could not settle, so a
                // second word here would be about a decision, not a loss.
                crate::diag::pending::park(
                    method
                        .rust_return
                        .as_ref()
                        .map(syn::spanned::Spanned::span)
                        .or_else(|| method.body_ast.as_ref().map(syn::spanned::Spanned::span))
                        .unwrap_or_else(proc_macro2::Span::call_site),
                    format!(
                        "`{}` for `{}` emits a method called `{}`, and something on this class \
                         already has that name, so this one is not written and every call to it \
                         goes to the other",
                        trait_name, self_type, ts_name
                    ),
                );
            }
        }
    }
}

fn emit_derive_methods(
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
                for f in fields {
                    let n = match f.name.as_deref() {
                        Some(n) => n,
                        None => continue,
                    };
                    let field_ty = f.ts_ty(reg);
                    let base_ty = field_ty.trim_end_matches(" | null");
                    let is_nullable = field_ty.ends_with(" | null");

                    if is_nullable {
                        out.push_str(&format!("    if (this.{} === null && other.{} === null) {{ /* both null, ok */ }}\n", n, n));
                        out.push_str(&format!("    else if (this.{} === null || other.{} === null) return false;\n", n, n));
                        out.push_str(&format!("    else {}\n", emit_field_eq(n, base_ty)));
                    } else {
                        out.push_str(&format!("    {}\n", emit_field_eq(n, base_ty)));
                    }
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
                                let ty = f.ts_ty(reg);
                                let base_ty = ty.trim_end_matches(" | null");
                                Some(if is_primitive_ts_type(base_ty) {
                                    format!("this.{}", n)
                                } else if base_ty == "Uint8Array" {
                                    if ty.ends_with(" | null") {
                                        format!("this.{} != null ? new Uint8Array(this.{}) : null", n, n)
                                    } else {
                                        format!("new Uint8Array(this.{})", n)
                                    }
                                } else if base_ty.ends_with("[]") {
                                    // Array — map clone
                                    let inner = &base_ty[..base_ty.len()-2];
                                    let clone_expr = if is_primitive_ts_type(inner) {
                                        format!("[...this.{}]", n)
                                    } else if inner.starts_with('[') && inner.contains(',') {
                                        // Array of tuples — clone each tuple element
                                        let tuple_inner = &inner[1..inner.len()-1];
                                        let parts: Vec<&str> = tuple_inner.split(", ").collect();
                                        let clones: Vec<String> = parts.iter().enumerate()
                                            .map(|(i, ty)| {
                                                if is_primitive_ts_type(ty.trim()) {
                                                    format!("e[{}]", i)
                                                } else {
                                                    format!("e[{}].clone()", i)
                                                }
                                            })
                                            .collect();
                                        format!("this.{}.map(e => [{}] as {})", n, clones.join(", "), inner)
                                    } else {
                                        format!("this.{}.map(e => e.clone())", n)
                                    };
                                    if ty.ends_with(" | null") {
                                        format!("this.{} != null ? {} : null", n, clone_expr)
                                    } else {
                                        clone_expr
                                    }
                                } else if base_ty.starts_with('[') && base_ty.ends_with(']') && base_ty.contains(',') {
                                    // Tuple — clone each element
                                    let inner = &base_ty[1..base_ty.len()-1];
                                    let parts: Vec<&str> = inner.split(", ").collect();
                                    let clones: Vec<String> = parts.iter().enumerate()
                                        .map(|(i, ty)| {
                                            if is_primitive_ts_type(ty.trim()) {
                                                format!("this.{}[{}]", n, i)
                                            } else {
                                                format!("this.{}[{}].clone()", n, i)
                                            }
                                        })
                                        .collect();
                                    format!("[{}] as {}", clones.join(", "), base_ty)
                                } else if base_ty.starts_with("HashMap<") || base_ty.starts_with("HashSet<") {
                                    // The runtime container's own clone, which
                                    // walks its keys and values by their Clone
                                    // shape. `new Map(...)` built a JavaScript
                                    // `Map` — identity-keyed, and shallow, so
                                    // both maps then owned one set of values.
                                    format!("this.{}.clone()", n)
                                } else if ty.ends_with(" | null") {
                                    format!("this.{}?.clone() ?? null", n)
                                } else {
                                    format!("this.{}.clone()", n)
                                })
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

fn is_primitive_ts_type(ty: &str) -> bool {
    matches!(ty, "string" | "boolean" | "number" | "bigint")
}

/// Generate an equality check expression for a field
/// One field compared for equality, by the two places it is read from.
///
/// `mine` and `theirs` are written expressions, not names, so the same rules
/// serve a struct's `this.x`/`other.x` and an enum payload's
/// `(this.value as any)._0`.
pub(crate) fn field_eq_at(mine: &str, theirs: &str, ty: &str) -> String {
    if is_primitive_ts_type(ty) {
        format!("if ({} !== {}) return false;", mine, theirs)
    } else if ty == "Uint8Array" {
        format!(
            "{{ if ({m}.length !== {o}.length) return false; for (let i = 0; i < {m}.length; i++) {{ if ({m}[i] !== {o}[i]) return false; }} }}",
            m = mine, o = theirs
        )
    } else if ty.starts_with("HashMap<") {
        // Size, keys AND VALUES. Comparing size and keys alone answered `true`
        // for two maps that agree about which keys they hold and about nothing
        // else — proto's `data.ts` compared two `HashMap<PropertyName, Value>`
        // that way, and a derived `equals` that ignores half the map is a wrong
        // answer wherever the type is a HashMap key.
        let value_ty = map_value_ty(ty);
        let compare = if is_primitive_ts_type(&value_ty) {
            "if (_w !== v) return false;".to_string()
        } else {
            format!("if (!{}) return false;", eq_call("v", "_w", &value_ty))
        };
        format!(
            "{{ if ({m}.size !== {o}.size) return false; for (const [k, v] of {m}) {{ if (!{o}.has(k)) return false; const _w = {o}.get(k)!; {c} }} }}",
            m = mine, o = theirs, c = compare
        )
    } else if ty.ends_with("[]") {
        let inner = &ty[..ty.len() - 2];
        let compare = if is_primitive_ts_type(inner) {
            format!("if ({}[i] !== {}[i]) return false;", mine, theirs)
        } else {
            format!("if (!{}) return false;", eq_call(&format!("{}[i]", mine), &format!("{}[i]", theirs), inner))
        };
        format!(
            "{{ if ({m}.length !== {o}.length) return false; for (let i = 0; i < {m}.length; i++) {{ {c} }} }}",
            m = mine, o = theirs, c = compare
        )
    } else {
        format!("if (!{}) return false;", eq_call(mine, theirs, ty))
    }
}

/// Two values of one type compared. A `Uint8Array` and an array have no
/// `equals`, so a nested one is compared elementwise where it stands.
fn eq_call(mine: &str, theirs: &str, ty: &str) -> String {
    if is_primitive_ts_type(ty) {
        format!("{} === {}", mine, theirs)
    } else {
        format!("{}.equals({})", mine, theirs)
    }
}

/// `V` of a written `HashMap<K, V>`, at the top level of the argument list.
fn map_value_ty(ty: &str) -> String {
    let Some(inner) = ty.strip_prefix("HashMap<").and_then(|r| r.strip_suffix('>')) else {
        return "unknown".to_string();
    };
    let mut depth = 0usize;
    for (at, ch) in inner.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return inner[at + 1..].trim().to_string(),
            _ => {}
        }
    }
    "unknown".to_string()
}

fn emit_field_eq(name: &str, ty: &str) -> String {
    field_eq_at(&format!("this.{}", name), &format!("other.{}", name), ty)
}

fn emit_struct_bincode(
    out: &mut String,
    reg: &TypeRegistry,
    module: Option<crate::registry::ModuleId>,
    s: &StructInfo,
    trait_impls: &HashMap<String, Vec<(&str, &[String])>>,
) {
    let has_custom_serde = trait_impls.get(&s.name)
        .map(|t| t.iter().any(|(n, _)| *n == "Serialize"))
        .unwrap_or(false)
        && !s.derives.iter().any(|d| d == "Serialize");

    if !has_custom_serde && crate::bincode_module::has_serde_derive(&s.derives) {
        out.push('\n');
        if s.fields.iter().all(|f| f.name.is_some()) {
            out.push_str(&crate::bincode_module::generate_struct_codec(reg, s));
        } else {
            out.push_str(&crate::bincode_module::generate_tuple_struct_codec(reg, s));
        }
        // The registry already decided whether this type has a JSON half, and
        // said why where it does not (`narrow_reads_json`). Asking again here
        // would file the same refusal a second time under a different wording.
        if module.and_then(|m| reg.module_type(m, &s.name)).is_some_and(|id| reg.reads_json(id)) {
            match crate::json_module::struct_json(reg, s) {
                Ok(json) => {
                    out.push('\n');
                    out.push_str(&json);
                }
                Err(reason) => {
                    crate::diag::pending::park_at(0, 0, format!("`{}`: {}", s.name, reason))
                }
            }
        }
    }
}

/// A method's type parameters, plus the impl's own where the signature names
/// them and nothing else declares them.
fn with_impl_params(generics: &str, impl_params: &[String], method: &FnInfo, self_type: &str) -> String {
    let signature = format!(
        "{} {}",
        method.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>().join(" "),
        method.return_type
    );
    let declared = format!("{} {}", generics, self_type);
    let missing: Vec<&String> = impl_params
        .iter()
        .filter(|p| names_type_param(&signature, p) && !names_type_param(&declared, p))
        .collect();
    if missing.is_empty() {
        return generics.to_string();
    }
    let mut params: Vec<String> = missing.into_iter().cloned().collect();
    let inner = generics.trim_start_matches('<').trim_end_matches('>').trim();
    if !inner.is_empty() {
        params.extend(inner.split(',').map(|p| p.trim().to_string()));
    }
    format!("<{}>", params.join(", "))
}

/// Is this type parameter named in this text as a name of its own, rather than
/// as part of a longer identifier?
fn names_type_param(text: &str, param: &str) -> bool {
    let bytes = text.as_bytes();
    text.match_indices(param).any(|(at, _)| {
        let before = at == 0 || !is_ident_byte(bytes[at - 1]);
        let after_at = at + param.len();
        let after = after_at == bytes.len() || !is_ident_byte(bytes[after_at]);
        before && after
    })
}

fn is_ident_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' }

fn emit_method(out: &mut String, method: &FnInfo, self_type: &str) {
    emit_method_with(out, method, self_type, "")
}

/// The same, with the modifiers TypeScript needs in front of the name — which
/// today is only `protected override` on the `onDrop` an `impl Drop` becomes.
fn emit_method_with(out: &mut String, method: &FnInfo, self_type: &str, modifiers: &str) {
    let static_kw = if method.is_static { "static " } else { "" };
    let async_kw = if method.is_async { "async " } else { "" };
    let generics = if method.is_static {
        merge_class_type_params_for_static(&method.generics, self_type)
    } else {
        method.generics.clone()
    };
    let params = format_params_filtered(&method.params, self_type);
    let ret = async_return(
        method.is_async,
        &resolve_self_type(&method.return_type, self_type),
    );

    let body = if let Some(body_ts) = &method.body_ts {
        let ts = body_ts.clone();
        // The `_result` accumulator a formatter composes into used to be spliced
        // in here, by searching the finished text for `_result +=` and
        // rewriting whichever lines happened to read `return Result.Ok(..)`.
        // It found the statement forms and not the tail, so a `Display` ending
        // in `write!(f, "b")` answered `"b"` rather than everything it had
        // written; and it knew nothing of a `write!` with no `?`. The body
        // translator opens the accumulator, appends every write and returns it,
        // from the Rust rather than from the text.
        let ts = if method.ts_name == "toString" && ts.contains("fromParts(") {
            // Monomorphized generic calls (Attested::<Event>::from_parts) can't be translated
            format!("return `[{}]`;\n", self_type)
        } else {
            ts
        };
        indent_body(&ts)
    } else {
        "    throw new Error('TODO');\n".to_string()
    };

    out.push_str(&format!("  {}{}{}{}{}({}): {} {{\n{}  }}\n",
        modifiers, static_kw, async_kw, method.ts_name, generics, params, ret, body));
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

/// Substitute the impl's own type for `Self` in a written TypeScript type.
///
/// Anchored on identifier boundaries: an unanchored replace also rewrote the
/// `Self` inside `SelfDescribing` or `MySelf`.
fn resolve_self_type(ty: &str, self_type: &str) -> String {
    if ty == "Self" {
        return self_type.to_string();
    }
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = String::with_capacity(ty.len());
    let bytes = ty.as_bytes();
    let mut i = 0;
    while i < ty.len() {
        if ty[i..].starts_with("Self") {
            let before_ok = i == 0 || !is_ident(bytes[i - 1] as char);
            let after = i + 4;
            let after_ok = after >= bytes.len() || !is_ident(bytes[after] as char);
            if before_ok && after_ok {
                out.push_str(self_type);
                i = after;
                continue;
            }
        }
        let ch = ty[i..].chars().next().expect("in bounds");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn format_params(params: &[ParamInfo]) -> String {
    params.iter()
        .map(|p| format!("{}: {}", p.name, param_spelling(p)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_params_filtered(params: &[ParamInfo], self_type: &str) -> String {
    params.iter()
        .filter(|p| !is_rust_only_type(&p.ty))
        .map(|p| format!("{}: {}", p.name, resolve_self_type(&param_spelling(p), self_type)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `implements` clause: the traits this class implements that the port has
/// a TypeScript name for.
///
/// Only a trait the crate declares is emitted as an interface. A trait from the
/// declared surface has none, and naming one here named something that does not
/// exist — `class Weight extends Struct implements Add`, `implements Clone` —
/// which is where a good part of signals' unresolved-name errors came from.
fn format_implements(reg: &TypeRegistry, traits: Option<&Vec<(&str, &[String])>>) -> String {
    if let Some(traits) = traits {
        let ifaces: Vec<String> = traits.iter()
            .filter(|(t, _)| reg.emits_interface(t))
            .map(|(name, type_args)| {
                if type_args.is_empty() {
                    name.to_string()
                } else {
                    format!("{}<{}>", name, type_args.join(", "))
                }
            })
            .collect();
        if ifaces.is_empty() { String::new() } else { format!(" implements {}", ifaces.join(", ")) }
    } else {
        String::new()
    }
}

/// Merge impl block generic bounds into a class's generic declaration.
/// E.g., `<Upstream, Input, Output, Transform>` with bounds
/// `{Upstream: [Signal, With<Input>, Clone], Transform: [Clone]}` becomes
/// `<Upstream extends Signal & With<Input> & Clone, Input, Output, Transform extends Clone>`
/// The parameters written inside a generic list.
///
/// Two things have to be read the way TypeScript reads them. The list ends at
/// ONE `>`, however many the last parameter's own type ends with — taking every
/// trailing `>` off took the list's terminator with them, and the class then
/// read `class Reactor<E extends .., Ev extends Clone = Attested<Event> extends
/// Struct {`, which swallowed the rest of the file. And a comma inside a type
/// argument belongs to that argument: `<A, B<C, D>>` declares two parameters.
fn generic_params(generics: &str) -> Vec<String> {
    let inner = generics
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .unwrap_or(generics);
    let mut params = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                params.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        params.push(current);
    }
    params
}

fn merge_bounds_into_generics(generics: &str, bounds: &HashMap<String, Vec<String>>) -> String {
    if generics.is_empty() || bounds.is_empty() { return generics.to_string(); }
    let params = generic_params(generics);
    let merged: Vec<String> = params.iter().map(|p| {
        let p = p.trim();
        // Extract existing param name (before any `extends` or `=`)
        let param_name = p.split_whitespace().next().unwrap_or(p);
        // Check if there are impl bounds for this param
        if let Some(impl_bounds) = bounds.get(param_name) {
            // Check if param already has `extends` constraints
            if p.contains(" extends ") {
                // Extract existing bounds and merge
                let extends_pos = p.find(" extends ").unwrap();
                let existing_part = &p[extends_pos + 9..]; // after " extends "
                // Split on default " = " if present
                let (existing_bounds_str, default_part) = if let Some(eq_pos) = existing_part.find(" = ") {
                    (&existing_part[..eq_pos], &existing_part[eq_pos..])
                } else {
                    (existing_part, "")
                };
                let existing_bounds: Vec<&str> = existing_bounds_str.split(" & ").map(|s| s.trim()).collect();
                let mut all_bounds: Vec<String> = existing_bounds.iter().map(|s| s.to_string()).collect();
                for b in impl_bounds {
                    if !all_bounds.iter().any(|eb| eb == b) {
                        all_bounds.push(b.clone());
                    }
                }
                format!("{} extends {}{}", param_name, all_bounds.join(" & "), default_part)
            } else {
                // No existing extends — check for default
                let (base, default_part) = if let Some(eq_pos) = p.find(" = ") {
                    (&p[..eq_pos], &p[eq_pos..])
                } else {
                    (p, "")
                };
                let _ = base; // unused, param_name is what we need
                format!("{} extends {}{}", param_name, impl_bounds.join(" & "), default_part)
            }
        } else {
            p.to_string()
        }
    }).collect();
    format!("<{}>", merged.join(", "))
}

/// Strip bounds and defaults from generic params for use in type references.
/// `<T = void>` → `<T>`, `<T extends Foo = void>` → `<T>`, `<T extends Signal & Clone>` → `<T>`
fn strip_generic_defaults(generics: &str) -> String {
    if generics.is_empty() { return generics.to_string(); }
    let params = generic_params(generics);
    let stripped: Vec<String> = params.iter().map(|p| {
        let p = p.trim();
        // Extract just the param name (before any `extends` or `=`)
        p.split_whitespace().next().unwrap_or(p).to_string()
    }).collect();
    format!("<{}>", stripped.join(", "))
}

/// The TypeScript a parameter is declared with.
///
/// C1: a `&mut T` whose `T` the port writes as a JavaScript VALUE is a
/// `BorrowMut<T>` — a cell the callee writes through and the caller reads back.
/// JavaScript passes a number, a string and a boolean by value, so a plain
/// parameter carried the callee's writes nowhere.
pub(crate) fn param_spelling(param: &crate::types::ParamInfo) -> String {
    if crate::is_boxed_mut(param) {
        return format!("BorrowMut<{}>", param.ty);
    }
    param.ty.clone()
}

fn is_rust_only_type(ty: &str) -> bool {
    ty.contains("Formatter") || ty.contains("Serializer") || ty.contains("Deserializer")
        || ty.contains("PhantomData")
}

/// Check if a field should be skipped in TS emission (zero-sized Rust types)
fn is_phantom_field(reg: &TypeRegistry, f: &FieldInfo) -> bool {
    f.ts_ty(reg).contains("PhantomData")
}


pub(crate) fn disambiguate_trait_method(
    base_name: &str,
    trait_name: &str,
    type_args: &[String],
    _self_type: &str,
    self_id: Option<crate::ty::TypeId>,
) -> String {
    // The one decision, settled over the whole impl table before anything is
    // written. Where the asker knows which type is being built, this is the
    // answer; the rules below are what remains for a caller that does not.
    if matches!(trait_name, "From" | "TryFrom") {
        if let (Some(self_id), Some(source)) = (self_id, type_args.first()) {
            if let Some(name) = crate::emit_impls::conversion_name(self_id, source) {
                return name;
            }
        }
    }
    if type_args.is_empty() {
        return base_name.to_string();
    }

    // PartialEq<T> where T != Self — disambiguate as equalsT
    if trait_name == "PartialEq" {
        let target = &type_args[0];
        // Only disambiguate when comparing against a different type (str, string, etc.)
        // The argument arrives as Rust wrote it, so `String` and `str` are
        // the two spellings of the one TypeScript `string`.
        if matches!(target.as_str(), "string" | "str" | "String") {
            return format!("{}Str", base_name);
        } else if target.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return format!("{}{}", base_name, target);
        }
        return base_name.to_string();
    }

    if !matches!(trait_name, "From" | "TryFrom" | "TryInto" | "Into") {
        return base_name.to_string();
    }
    // The argument arrives as the path the source wrote — `bincode::Error`,
    // `String`, `crate::property::PropertyError` — because that is what tells
    // two conversions of one type apart. The primitive question is about the
    // TYPE, so it is asked of the leaf's TypeScript spelling.
    let source = type_args[0].trim_start_matches('&');
    let leaf = source.rsplit("::").next().unwrap_or(source);

    // Nothing here settles a contest: that is `emit_impls::naming`'s answer,
    // asked above by every caller that knows which type is being built. What
    // remains is the plain reading, for a caller that does not — an operator's
    // method name, a free function's symbol.
    if source_reads_as_plain(source) {
        return base_name.to_string();
    }
    if leaf.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !source.contains('<')
        && !source.contains(',')
        && !source.contains(' ')
    {
        return format!("{}{}", base_name, leaf);
    }
    base_name.to_string()
}

/// Does this conversion source name a type whose TypeScript spelling says
/// nothing about which impl it is?
///
/// `String`, `u64` and `Vec<u8>` all become a built-in TypeScript type, so the
/// leaf reads as noise beside `from` where only one impl wants that name. Both
/// halves of the naming — the class's method and the registry post-pass that
/// finds out which names are contested — ask this one question.
pub(crate) fn source_reads_as_plain(source: &str) -> bool {
    // The written path keeps the `&` of a reference argument, and the question
    // here is about the TYPE.
    let source = source.trim_start_matches('&');
    let leaf = source.rsplit("::").next().unwrap_or(source);
    let as_ts = crate::name_map::map_type_name(leaf);
    as_ts.ends_with("[]")
        || as_ts.starts_with("HashMap<")
        || as_ts.starts_with("HashSet<")
        || as_ts.contains("Uint8Array")
        || matches!(&*as_ts, "string" | "boolean" | "number" | "bigint")
}

/// A type's spelling as a fragment of a method name: identifier characters
/// only, first letter capitalised. `T[]` is `T` and `Uint8Array` is itself; a
/// spelling with nothing left in it has no fragment to give.
pub(crate) fn name_fragment(spelling: &str) -> Option<String> {
    let kept: String = spelling.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut chars = kept.chars();
    let first = chars.next()?;
    if first.is_ascii_digit() {
        return None;
    }
    Some(first.to_uppercase().chain(chars).collect())
}

/// The source type's name as a conversion static spells it.
///
/// A leaf alone is not enough where one type converts from several types that
/// share it: `RetrievalError` has `From<bincode::Error>`,
/// `From<crate::selection::filter::Error>` and `From<anyhow::Error>`, all three
/// of which named `fromError`, and emission kept the first and dropped two.
/// The segment in front of the leaf tells them apart — `fromBincodeError`,
/// `fromFilterError`, `fromAnyhowError` — and it is dropped where it says
/// nothing the leaf does not already say (`crate::property::PropertyError`
/// stays `fromPropertyError`) or where it is only a position in the crate
/// (`crate`, `self`, `super`).
///
/// One rule, read by both halves: the class's static is written from the impl's
/// written path and so is every call site's name for it.
pub(crate) fn qualified_source(written: &str) -> String {
    let mut segments = written.split("::").filter(|s| !s.is_empty());
    let leaf = written.rsplit("::").next().unwrap_or(written).to_string();
    let qualifier = {
        let all: Vec<&str> = segments.by_ref().collect();
        if all.len() < 2 {
            None
        } else {
            all.get(all.len() - 2).copied()
        }
    };
    let Some(qualifier) = qualifier else {
        return leaf;
    };
    if matches!(qualifier, "crate" | "self" | "super" | "std" | "core" | "alloc") {
        return leaf;
    }
    let lower_leaf = leaf.to_lowercase();
    if lower_leaf.contains(&qualifier.to_lowercase()) {
        return leaf;
    }
    // A Rust module is snake_case and a TypeScript name is not:
    // `serde_json::Error` is `fromSerdeJsonError`, not `fromSerde_jsonError`.
    let mut out = crate::name_map::to_camel_case(qualifier);
    let mut chars = out.chars();
    out = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    };
    out.push_str(&leaf);
    out
}

/// For static methods, merge class-level type params into the method's own generics.
/// TypeScript static methods cannot reference enclosing class type parameters, so
/// `impl<T> Foo<T> { fn new() -> Self }` must emit `static new<T>(): Foo<T>`.
fn merge_class_type_params_for_static(method_generics: &str, self_type: &str) -> String {
    // Extract class type params from self_type, e.g. "Foo<T, U>" → ["T", "U"]
    let class_params = if let Some(start) = self_type.find('<') {
        let inner = &self_type[start + 1..self_type.len() - 1];
        // Split on commas, but respect nested angle brackets
        let mut params = Vec::new();
        let mut depth = 0;
        let mut current = String::new();
        for ch in inner.chars() {
            match ch {
                '<' => { depth += 1; current.push(ch); }
                '>' => { depth -= 1; current.push(ch); }
                ',' if depth == 0 => {
                    let p = current.trim().to_string();
                    if !p.is_empty() { params.push(p); }
                    current.clear();
                }
                _ => { current.push(ch); }
            }
        }
        let p = current.trim().to_string();
        if !p.is_empty() { params.push(p); }
        params
    } else {
        return method_generics.to_string();
    };

    if class_params.is_empty() {
        return method_generics.to_string();
    }

    // Extract existing method type param names (just the bare names, before any "extends")
    let method_param_names: HashSet<String> = if method_generics.is_empty() {
        HashSet::new()
    } else {
        let inner = &method_generics[1..method_generics.len() - 1];
        inner.split(',')
            .map(|p| p.trim().split_whitespace().next().unwrap_or("").to_string())
            .collect()
    };

    // Add class params that aren't already declared on the method
    let mut new_params: Vec<String> = class_params.into_iter()
        .filter(|p| {
            let name = p.split_whitespace().next().unwrap_or(p);
            !method_param_names.contains(name)
        })
        .collect();

    if new_params.is_empty() {
        return method_generics.to_string();
    }

    // Merge: class params first, then method params
    if !method_generics.is_empty() {
        let inner = &method_generics[1..method_generics.len() - 1];
        new_params.push(inner.to_string());
    }
    format!("<{}>", new_params.join(", "))
}

/// What one trait method is emitted as on the class.
///
/// One function, because the name is computed twice: here, where the method is
/// written, and in the check that looks for a name the runtime already uses. A
/// second rule would drift from this one and the check would then be about a
/// name nobody emits.
pub(crate) fn trait_method_name(
    trait_name: &str,
    type_args: &[String],
    method: &FnInfo,
    self_type: &str,
    self_id: Option<crate::ty::TypeId>,
) -> String {
    impl_method_name(
        trait_name,
        &method.name,
        &method.ts_name,
        type_args,
        self_type,
        self_id,
    )
}

/// The same, for a caller that has the trait's method by name rather than an
/// extracted declaration to read it off: an operator, whose method the trait
/// declares and the impl need not write out.
pub(crate) fn impl_method_name(
    trait_name: &str,
    rust_method: &str,
    ts_method: &str,
    type_args: &[String],
    self_type: &str,
    self_id: Option<crate::ty::TypeId>,
) -> String {
    // For known Rust traits (Display, Clone, etc.), apply name mapping. For
    // unknown or domain traits, the method's own TypeScript name stands.
    let base = match trait_method_mapping(trait_name, rust_method) {
        Some((mapped, _)) => mapped.to_string(),
        None => ts_method.to_string(),
    };
    disambiguate_trait_method(&base, trait_name, type_args, self_type, self_id)
}


/// Does this method's whole body call the name it is being emitted under?
///
/// `fn partial_cmp(&self, o) -> Option<Ordering> { Some(self.cmp(o)) }` and
/// `fn cmp(&self, o) -> Ordering` are one method in TypeScript, so the
/// forwarding one reads `return this.compareTo(other)` — a method that calls
/// itself and never returns. The question is asked of the TRANSLATED body,
/// because that is where the two names have become one.
fn forwards_to_itself(method: &FnInfo, ts_name: &str) -> bool {
    let Some(body) = method.body_ts.as_deref() else { return false };
    let statements: Vec<&str> = body.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let [only] = statements[..] else { return false };
    only.contains(&format!("this.{}(", ts_name))
}

fn trait_method_mapping(trait_name: &str, rust_method_name: &str) -> Option<(&'static str, Option<&'static str>)> {
    match (trait_name, rust_method_name) {
        // `impl Drop for T` is the type's own cleanup, and `AkObject.drop()` is
        // the template that calls it: mark, unregister, run this, then drop the
        // fields. Overriding `drop()` instead would put the cleanup after the
        // cascade, which is the wrong order and hands the body dead fields.
        ("Drop", "drop") => Some(("onDrop", Some("void"))),
        ("Display", "fmt") => Some(("toString", Some("string"))),
        ("Debug", "fmt") => None,
        ("Default", "default") => Some(("default", None)),
        ("Clone", "clone") => Some(("clone", None)),
        ("PartialEq", "eq") => Some(("equals", Some("boolean"))),
        ("PartialOrd", "partial_cmp") => Some(("compareTo", Some("number"))),
        ("Ord", "cmp") => Some(("compareTo", Some("number"))),
        ("From", "from") => Some(("from", None)),
        ("TryFrom", "try_from") => Some(("tryFrom", None)),
        ("TryInto", "try_into") => Some(("from", None)),
        ("Into", "into") => Some(("from", None)),
        ("FromStr", "from_str") => Some(("fromStr", None)),
        ("IntoIterator", "into_iter") => Some(("iter", None)),
        _ => None,
    }
}

#[cfg(test)]
mod cycle_tests {
    use super::*;

    fn method(body: &str) -> FnInfo {
        FnInfo {
            name: "partial_cmp".to_string(),
            ts_name: "compareTo".to_string(),
            is_pub: true,
            vis: crate::types::VisInfo::Public,
            is_async: false,
            is_static: false,
            self_kind: None,
            self_receiver: None,
            has_default_body: false,
            params: Vec::new(),
            return_type: "number".to_string(),
            rust_return: None,
            generics: String::new(),
            type_params: Vec::new(),
            syn_generics: syn::Generics::default(),
            is_test: false,
            body_ast: None,
            body_ts: Some(body.to_string()),
        }
    }

    /// `Ord::cmp` and `PartialOrd::partial_cmp` are ONE method here, so an impl
    /// written `Some(self.cmp(other))` becomes `return this.compareTo(other)` —
    /// a method that calls itself. Whichever the source wrote first took the
    /// name, and when that was the forwarding one every comparison overflowed
    /// the stack: storage-common's `HeapItem` is written exactly that way, so
    /// every TopK heap comparison did, and `equals` — `compareTo(o) === 0` —
    /// went down with it.
    #[test]
    fn a_body_that_is_only_a_call_to_its_own_name_is_a_cycle() {
        assert!(forwards_to_itself(&method("return this.compareTo(other);"), "compareTo"));
        assert!(forwards_to_itself(&method("  return this.compareTo(other);  "), "compareTo"));
    }

    #[test]
    fn a_body_with_something_in_it_is_not_a_cycle() {
        // The Ord body itself: it calls a FIELD's comparison, not its own name.
        assert!(!forwards_to_itself(&method("return this.n.compareTo(other.n);"), "compareTo"));
        // More than one statement is a body, whatever it calls.
        assert!(!forwards_to_itself(
            &method("const a = this.key();\nreturn this.compareTo(other);"),
            "compareTo"
        ));
        // A different name is a different method.
        assert!(!forwards_to_itself(&method("return this.partialCompareTo(other);"), "compareTo"));
        assert!(!forwards_to_itself(&method(""), "compareTo"));
    }
}
