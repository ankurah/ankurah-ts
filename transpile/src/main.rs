use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod bincode_module;
mod body;
mod cfg;
mod codegen;
mod config;
mod control_flow;
mod diag;
mod emit;
mod emit_impls;
mod extract;
mod imports;
mod infer;
mod macros;
mod match_expr;
mod name_map;
mod native_types;
mod ownership;
mod registry;
#[cfg(test)]
mod testing;
mod trace;
mod ty;
mod types;

#[derive(Parser)]
#[command(name = "ankurah-transpile")]
#[command(about = "Transpile ankurah Rust source to TypeScript")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Generate TypeScript skeleton from a Rust source file
    Skeleton {
        /// Path to a Rust source file
        #[arg()]
        file: PathBuf,

        /// Rust crate path for MIRRORS annotation (e.g., "proto/src/id.rs")
        #[arg(long)]
        crate_path: Option<String>,
    },

    /// Report which function every method call in a crate resolves to.
    ///
    /// One tab-separated row per call: file, line, column, method, receiver,
    /// adjusted receiver, callee, result type, deref steps. Read by the oracle
    /// test, which compares the rows against rust-analyzer's answers for the
    /// same sites.
    Resolve {
        /// Rust crate source directory
        #[arg()]
        src_dir: PathBuf,

        /// Crate name, as `batch` takes it
        #[arg(long)]
        crate_name: String,
    },

    /// Batch-generate TS skeletons for an entire crate, writing to an output directory
    Batch {
        /// Rust crate source directory (e.g., ../ankurah-ts-support/proto/src)
        #[arg()]
        src_dir: PathBuf,

        /// Output directory for generated TS files
        #[arg()]
        out_dir: PathBuf,

        /// Crate name for MIRRORS annotation (e.g., "proto")
        #[arg(long)]
        crate_name: String,

        /// Where the declared std and extern surface lives. Defaults to
        /// `std_surface/` beside the transpiler's own crate.
        #[arg(long)]
        std_surface: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Skeleton { file, crate_path } => {
            let sink = diag::DiagSink::new();
            let mut parsed = vec![registry::ExtractedFile {
                path: file
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                file: extract::extract(&file)?,
                declarations_only: false,
            }];
            let mut surface = registry::Surface::default();
            let registry = registry::build_registry(&mut parsed, &mut surface, &[], &sink);
            let crate_path = crate_path.unwrap_or_else(|| file.display().to_string());
            let ts = codegen::generate_ts(&registry, &parsed[0].file, &crate_path);
            print!("{}", ts);
        }
        Command::Resolve {
            src_dir,
            crate_name,
        } => {
            let config_path = PathBuf::from("transpile.toml");
            let config = if config_path.exists() {
                Some(config::Config::load(&config_path)?)
            } else {
                None
            };
            trace::start();
            let out = tempdir_for_resolve()?;
            batch_generate(&src_dir, &out, &crate_name, config.as_ref(), None)?;
            std::fs::remove_dir_all(&out).ok();
            // `batch` prints the files it writes; the rows are tagged so the
            // reader can tell them apart without silencing that.
            for row in trace::rows() {
                println!("RESOLVED\t{}", row);
            }
            for row in trace::closure_rows() {
                println!("CLOSURE\t{}", row);
            }
        }
        Command::Batch {
            src_dir,
            out_dir,
            crate_name,
            std_surface,
        } => {
            // Load config if transpile.toml exists
            let config_path = PathBuf::from("transpile.toml");
            let config = if config_path.exists() {
                Some(config::Config::load(&config_path)?)
            } else {
                None
            };
            batch_generate(
                &src_dir,
                &out_dir,
                &crate_name,
                config.as_ref(),
                std_surface.as_deref(),
            )?;
        }
    }

    Ok(())
}

/// `resolve` runs the same pipeline `batch` does, because the answers it
/// reports are the ones body translation actually used. The TypeScript it
/// writes on the way is thrown away.
fn tempdir_for_resolve() -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ankurah-resolve-{}-{}", std::process::id(), nanos));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn batch_generate(
    src_dir: &Path,
    out_dir: &Path,
    crate_name: &str,
    config: Option<&config::Config>,
    std_surface_dir: Option<&Path>,
) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output dir {}", out_dir.display()))?;

    let sink = diag::DiagSink::new();

    // Phase 1: Parse all files and build type→file map
    let mut parsed_files: Vec<registry::ExtractedFile> = Vec::new();
    let mut type_to_file: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for entry in WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let rs_path = entry.path();
        let relative = rs_path.strip_prefix(src_dir).unwrap_or(rs_path);
        let rel_str = relative.display().to_string();

        // Skip excluded files
        if rel_str.contains("wasm") || rel_str.contains("uniffi") {
            continue;
        }
        // An excluded file is cfg-gated for a platform the port does not build,
        // so its types do not exist here at all. A hardcoded file is different:
        // its TypeScript is hand-written, so nothing may be emitted for it, but
        // the types it declares are part of the crate and other files resolve
        // through them.
        let mut declarations_only = false;
        if let Some(cfg) = config {
            let full_path = format!("{}/src/{}", crate_name, rel_str);
            if cfg.is_excluded_file(&full_path) {
                eprintln!("  SKIP {} (excluded)", rel_str);
                continue;
            }
            if cfg.is_hardcoded(&full_path) {
                eprintln!("  DECLARATIONS ONLY {} (hardcoded)", rel_str);
                declarations_only = true;
            }
        }

        let features = config.map(|c| &c.features);
        let mut rust_file = match extract::extract_with_features(rs_path, features) {
            Ok(f) => f,
            Err(e) => {
                // A file the parser cannot read is a hole in the crate, not a
                // file to pass over quietly.
                eprintln!("SKIP {}: {}", rel_str, e);
                sink.set_file(&rel_str);
                sink.push(diag::Diag {
                    file: rel_str.clone(),
                    line: 0,
                    col: 0,
                    message: format!("file could not be parsed: {:#}", e),
                });
                continue;
            }
        };

        // Extraction names the file by where it was read from; from here on it
        // is named by its place in the crate, which is the key the module tree
        // and the diagnostics use.
        rust_file.path = rel_str.clone();

        // Register all types defined in this file. A hardcoded file's types
        // are not added here: its TypeScript is hand-written and reached
        // through [cross_crate_types], not through a generated import.
        let ts_module = rs_to_ts_module(&rel_str);
        if declarations_only {
            parsed_files.push(registry::ExtractedFile {
                path: rel_str,
                file: rust_file,
                declarations_only,
            });
            continue;
        }
        for s in &rust_file.structs {
            type_to_file.insert(s.name.clone(), ts_module.clone());
        }
        for e in &rust_file.enums {
            type_to_file.insert(e.name.clone(), ts_module.clone());
        }
        for t in &rust_file.traits {
            type_to_file.insert(t.name.clone(), ts_module.clone());
        }
        // Register inline module symbols (types go in type_to_file, functions tracked separately)
        for (mod_name, sub_file) in &rust_file.inline_modules {
            let sub_module = format!("{}/{}", ts_module.trim_end_matches("/index"), mod_name);
            for s in &sub_file.structs {
                type_to_file.insert(s.name.clone(), sub_module.clone());
            }
            for e in &sub_file.enums {
                type_to_file.insert(e.name.clone(), sub_module.clone());
            }
            for t in &sub_file.traits {
                type_to_file.insert(t.name.clone(), sub_module.clone());
            }
        }

        parsed_files.push(registry::ExtractedFile {
            path: rel_str,
            file: rust_file,
            declarations_only,
        });
    }

    // Batch walks the filesystem, so the file order depends on the directory
    // listing. Sort it, so the registry and the diagnostics are the same on
    // every machine.
    parsed_files.sort_by(|a, b| a.path.cmp(&b.path));

    // Add cross-crate type mappings from config
    if let Some(cfg) = config {
        for (type_name, package) in &cfg.cross_crate_types {
            type_to_file.insert(type_name.clone(), package.clone());
        }
    }

    // Phase 2: Build the registry from the declared std surface and everything
    // this crate declares. The surface is parsed once per run and declared into
    // each registry the run builds.
    let crate_names = crate_names_for(crate_name, config);
    let surface_dir = registry::std_surface::default_dir(std_surface_dir);
    let registry = registry::std_surface::with_cached(&surface_dir, |surface| {
        if surface.is_empty() {
            eprintln!(
                "WARNING no std surface found at {}; every std type will be undeclared",
                surface_dir.display()
            );
        }
        registry::build_registry(&mut parsed_files, surface, &crate_names, &sink)
    });

    // Phase 3: Translate all deferred bodies with type context
    let total_bodies: usize = parsed_files
        .iter()
        .filter(|e| !e.declarations_only)
        .map(|e| {
            let f = &e.file;
            f.functions.iter().filter(|f| f.body_ast.is_some()).count()
                + f.impls
                    .iter()
                    .flat_map(|i| i.methods.iter())
                    .filter(|m| m.body_ast.is_some())
                    .count()
                + f.test_functions
                    .iter()
                    .filter(|f| f.body_ast.is_some())
                    .count()
        })
        .sum();
    eprintln!(
        "  Phase 3: translating {} bodies with registry",
        total_bodies
    );
    translate_all_bodies(&mut parsed_files, &registry, &sink);

    // An impl with no class of its own is emitted as module-level functions
    // (see `emit_impls`), and a module that calls one has to import it by name.
    // The map is built once the registry exists, because which impls need it is
    // a question about resolved self types.
    for entry in parsed_files.iter().filter(|e| !e.declarations_only) {
        let Some(module) = registry.modules().lookup_file(&entry.path) else {
            continue;
        };
        let ts_module = rs_to_ts_module(&entry.path);
        for f in emit_impls::free_functions(&registry, module, &entry.file) {
            type_to_file.insert(f.name, ts_module.clone());
        }
    }

    // Phase 4: Generate TS with resolved imports
    let mut file_count = 0;

    for entry in parsed_files.iter().filter(|e| !e.declarations_only) {
        let (rel_str, rust_file) = (&entry.path, &entry.file);
        let ts_relative = rs_to_ts_path(rel_str);
        let ts_path = out_dir.join(&ts_relative);

        if let Some(parent) = ts_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let crate_path = format!("{}/src/{}", crate_name, rel_str);
        let current_module = rs_to_ts_module(rel_str);
        let ts = codegen::generate_ts_with_imports_configured(
            &registry,
            rust_file,
            &crate_path,
            &type_to_file,
            &current_module,
            config,
        );

        std::fs::write(&ts_path, &ts)
            .with_context(|| format!("Failed to write {}", ts_path.display()))?;

        file_count += 1;
        println!("  {} → {}", rel_str, ts_relative);

        // Generate inline module files
        for (mod_name, sub_file) in &rust_file.inline_modules {
            let sub_module = format!("{}/{}", current_module.trim_end_matches("/index"), mod_name);
            let sub_crate_path = format!("{}/{}", crate_path.trim_end_matches(".rs"), mod_name);
            let sub_ts = codegen::generate_ts_with_imports_configured(
                &registry,
                sub_file,
                &sub_crate_path,
                &type_to_file,
                &sub_module,
                config,
            );
            let sub_relative = format!("{}/{}.ts", ts_relative.trim_end_matches(".ts"), mod_name);
            let sub_path = out_dir.join(&sub_relative);
            if let Some(parent) = sub_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&sub_path, &sub_ts)
                .with_context(|| format!("Failed to write {}", sub_path.display()))?;
            file_count += 1;
            println!("    {} (inline module)", sub_relative);
        }

        // Generate test file if there are test functions
        if let Some(test_ts) = codegen::generate_test_ts_with_imports(
            rust_file,
            &crate_path,
            &type_to_file,
            &current_module,
        ) {
            let test_relative = ts_relative.replace(".ts", ".test.ts");
            let test_path = out_dir.join(&test_relative);
            std::fs::write(&test_path, &test_ts)
                .with_context(|| format!("Failed to write {}", test_path.display()))?;
            file_count += 1;
            println!("  {} → {} (tests)", rel_str, test_relative);
        }
    }

    println!("\nGenerated {} files in {}", file_count, out_dir.display());
    eprintln!("  {} types with no declaration", registry.undeclared_reported());
    sink.print_summary();
    // A line the diagnostics-budget test parses. Everything above it is for a
    // person reading a run; this is for the harness.
    let (crate_diags, surface_diags) = sink.counts();
    eprintln!(
        "DIAGNOSTICS crate={} total={} undeclared={} surface={}",
        crate_name,
        crate_diags,
        registry.undeclared_reported(),
        surface_diags
    );
    Ok(())
}

/// The names this crate answers to in a written path: the TypeScript package
/// name the run was given, plus the Cargo and Rust spellings of the crate it
/// maps to, so `ankurah_proto::id::EntityId` written inside proto resolves.
fn crate_names_for(crate_name: &str, config: Option<&config::Config>) -> Vec<String> {
    // The TypeScript package name is not a Rust crate name, and `core` is a
    // real standard-library root: never let it capture `core::mem::swap`.
    let mut names = Vec::new();
    if !matches!(crate_name, "std" | "core" | "alloc") {
        names.push(crate_name.to_string());
    }
    if let Some(cfg) = config {
        for (cargo_name, package) in &cfg.crates {
            if package == crate_name {
                names.push(cargo_name.clone());
                names.push(cargo_name.replace('-', "_"));
            }
        }
    }
    names.sort();
    names.dedup();
    if names.is_empty() {
        names.push(crate_name.to_string());
    }
    names
}

/// Phase 3: Translate all deferred function bodies with type registry context.
/// This runs after all files are parsed and the registry is built, so every
/// function body has access to the full crate's type information.
fn translate_all_bodies(
    files: &mut [registry::ExtractedFile],
    registry: &registry::TypeRegistry,
    sink: &diag::DiagSink,
) {
    for entry in files.iter_mut().filter(|e| !e.declarations_only) {
        sink.set_file(&entry.path);
        let Some(module) = registry.modules().lookup_file(&entry.path) else {
            continue;
        };
        translate_module(&mut entry.file, registry, module, sink);
    }
}

/// The members every ported type inherits from `AkObject`.
///
/// A Rust field with one of these names becomes a class property that shadows
/// the runtime's own member, and the ownership machinery then reads the field
/// where it meant to read itself. `label` is the one that bites: it is a
/// `protected get`, so the emitted constructor assigns through a getter with no
/// setter and throws.
const RUNTIME_MEMBERS: [&str; 9] = [
    "label",
    "drop",
    "isDropped",
    "isMoved",
    "onDrop",
    "ownedFields",
    "takeField",
    "assertNotDropped",
    "markMoved",
];

/// Say so where a declared field's name is one the runtime already uses.
fn report_member_collisions(file: &types::RustFile, sink: &diag::DiagSink) {
    let mut say = |owner: &str, field: &Option<String>, at: &syn::Type| {
        let Some(name) = field else { return };
        if !RUNTIME_MEMBERS.contains(&name.as_str()) {
            return;
        }
        let start = syn::spanned::Spanned::span(at).start();
        sink.push(diag::Diag {
            file: sink.file(),
            line: start.line,
            col: start.column + 1,
            message: format!(
                "`{}.{}` has the name of a member every ported type inherits from `AkObject`, \
                 so the field shadows the runtime's own and the ownership checks read the \
                 wrong one",
                owner, name
            ),
        });
    };
    for s in &file.structs {
        for f in &s.fields {
            say(&s.name, &f.name, &f.rust_ty);
        }
    }
    for e in &file.enums {
        for v in &e.variants {
            for f in &v.fields {
                say(&e.name, &f.name, &f.rust_ty);
            }
        }
    }
}

fn translate_module(
    file: &mut types::RustFile,
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    sink: &diag::DiagSink,
) {
    let inline_mod_names: Vec<String> = file
        .inline_modules
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let consts = resolve_module_consts(registry, module, &file.consts, sink);
    report_member_collisions(file, sink);
    // Read while the bodies are still ASTs: translation drops them below.
    file.assigned_fields = emit::assigned_fields(file);

    for func in &mut file.functions {
        translate_fn_body(
            func,
            "Self",
            "this",
            None,
            &[],
            &[],
            registry,
            module,
            &inline_mod_names,
            &consts,
            sink,
        );
    }

    for (mod_name, sub_file) in &mut file.inline_modules {
        let Some(child) = registry
            .modules()
            .get(module)
            .children
            .get(mod_name)
            .copied()
        else {
            continue;
        };
        translate_module(sub_file, registry, child, sink);
    }

    for imp in &mut file.impls {
        let self_type = imp.target_type.clone();
        let self_ty = impl_self_ty(registry, module, imp, sink);
        // An impl written for a type with no class of its own is emitted as
        // module-level functions, and there `self` is an ordinary parameter.
        let self_name = match &self_ty {
            Some(ty) if !emit_impls::has_emitted_class(registry, ty) => "self",
            _ => "this",
        };
        // What the impl's own parameters are known to implement, so that a call
        // on one of them — `self.0.read().unwrap().clone()` under
        // `impl<T: Clone> ValueCell<T>` — reaches the trait's declaration.
        let env = registry::TypeEnv::new(registry, module, sink)
            .with_params(&imp.type_params)
            .with_self(self_ty.as_ref());
        let bounds = registry::method::param_bounds_of(&registry::resolve_bounds(
            &imp.generics,
            &env,
            sink,
        ));
        for method in &mut imp.methods {
            translate_fn_body(
                method,
                &self_type,
                self_name,
                self_ty.clone(),
                &imp.type_params,
                &bounds,
                registry,
                module,
                &inline_mod_names,
                &consts,
                sink,
            );
        }
    }

    for func in &mut file.test_functions {
        translate_fn_body(
            func,
            "Self",
            "this",
            None,
            &[],
            &[],
            registry,
            module,
            &[],
            &consts,
            sink,
        );
    }

    // Inside a trait's own default body `Self` is whatever implements it: a
    // parameter carrying that one bound, which is what a call on `self`
    // dispatches through.
    for tr in &mut file.traits {
        let Some(trait_id) = registry.module_type(module, &tr.name) else {
            continue;
        };
        let self_ty = ty::Ty::Param("Self".to_string());
        let bounds = vec![(
            "Self".to_string(),
            ty::TraitRef {
                id: trait_id,
                args: tr.type_params.iter().cloned().map(ty::Ty::Param).collect(),
                bindings: Vec::new(),
            },
        )];
        for method in &mut tr.methods {
            translate_fn_body(
                method,
                &tr.name,
                "this",
                Some(self_ty.clone()),
                &tr.type_params,
                &bounds,
                registry,
                module,
                &[],
                &[],
                sink,
            );
        }
    }
}

/// The type an impl block is written for, which is what `self` means inside it.
fn impl_self_ty(
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    imp: &types::ImplInfo,
    sink: &diag::DiagSink,
) -> Option<ty::Ty> {
    // `impl Collatable for &str` has no path to name a target class by, but it
    // still has a self type, and `self` inside it is a `&str`.
    let syn_ty = imp.self_ty.as_ref()?;
    let env = registry::TypeEnv::new(registry, module, sink).with_params(&imp.type_params);
    match registry::resolve_type(syn_ty, &env) {
        Ok(resolved) => Some(resolved),
        Err(diag) => {
            sink.push(diag);
            None
        }
    }
}

fn resolve_module_consts(
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    consts: &[types::ConstInfo],
    sink: &diag::DiagSink,
) -> Vec<(String, ty::Ty)> {
    let env = registry::TypeEnv::new(registry, module, sink);
    let mut out = Vec::new();
    for c in consts {
        let Some(syn_ty) = &c.rust_ty else { continue };
        match registry::resolve_type(syn_ty, &env) {
            Ok(resolved) => out.push((c.name.clone(), resolved)),
            Err(diag) => sink.push(diag),
        }
    }
    out
}

/// Translate a single function's body_ast → body_ts with type-aware context.
#[allow(clippy::too_many_arguments)]
fn translate_fn_body(
    func: &mut types::FnInfo,
    self_type: &str,
    // The identifier Rust's `self` is emitted as: `this` for a method on an
    // emitted class, and the function's first parameter for a method whose
    // impl has no class of its own.
    self_name: &'static str,
    self_ty: Option<ty::Ty>,
    impl_params: &[String],
    impl_bounds: &[(String, ty::TraitRef)],
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    inline_module_names: &[String],
    consts: &[(String, ty::Ty)],
    sink: &diag::DiagSink,
) {
    if let Some(ref block) = func.body_ast {
        let mut params = impl_params.to_vec();
        params.extend(func.type_params.iter().cloned());

        let mut tc = infer::TypeContext::new(registry, module, self_ty.clone(), params, sink);
        // The bounds in scope: the impl block's, plus this function's own.
        let mut bounds = impl_bounds.to_vec();
        {
            let env = registry::TypeEnv::new(registry, module, sink)
                .with_params(&tc.params)
                .with_self(self_ty.as_ref());
            bounds.extend(registry::method::param_bounds_of(&registry::resolve_bounds(
                &func.syn_generics,
                &env,
                sink,
            )));
        }
        tc.param_bounds = bounds;
        for (name, ty) in consts {
            tc.bind(name, ty.clone());
        }

        let typed_params: Vec<(String, ty::Ty)> = func
            .params
            .iter()
            .filter(|p| !p.is_self)
            .filter_map(|p| {
                let syn_ty = p.rust_ty.as_ref()?;
                match tc.resolve_written_type(syn_ty) {
                    Ok(ty) => Some((p.name.clone(), ty)),
                    Err(diag) => {
                        sink.push(diag);
                        None
                    }
                }
            })
            .collect();
        tc.push_fn(typed_params);

        // A return type written as `Self::Target` has no TypeScript name of its
        // own: the syntactic mapping renders it as the bare associated name,
        // which is not a type. The impl that supplies it is in the table, so ask.
        if let Some(written) = func.rust_return.as_ref().filter(|t| projects_through_self(t)) {
            if let Ok(resolved) = tc.resolve_written_type(written) {
                let normalized = tc.probe().normalize(&resolved);
                if normalized != resolved {
                    func.return_type = name_map::map_ty(registry, &normalized);
                }
            }
        }

        // What this function returns, so that `?` can say whether the error it
        // hands on needs a `From` conversion Rust would have called.
        let returns = func
            .rust_return
            .as_ref()
            .and_then(|written| tc.resolve_written_type(written).ok());

        // Rust drops a by-value parameter at the end of the function body, so
        // the body's block owns it exactly as it owns its own locals. A `&self`
        // or `&T` parameter is a borrow and owns nothing.
        let mut owned_params: Vec<(String, ty::Ty)> = func
            .params
            .iter()
            .filter(|p| !p.is_self)
            .filter(|p| !matches!(p.rust_ty, Some(syn::Type::Reference(_))))
            .filter_map(|p| {
                let syn_ty = p.rust_ty.as_ref()?;
                Some((p.name.clone(), tc.resolve_written_type(syn_ty).ok()?))
            })
            .collect();
        // `fn into_inner(self)` takes the receiver by value, so the body owns it
        // like any other by-value parameter: the caller stops owning it at the
        // call, and if the body does not hand it on, the body releases it.
        // Leaving it out made every self-taking method a leak.
        if func.self_kind == Some(types::SelfKind::Value) {
            if let Some(ty) = self_ty.clone() {
                owned_params.insert(0, (self_name.to_string(), ty));
            }
        }

        let mut translator = body::BodyTranslator::with_context(self_type, tc);
        translator.self_name = self_name;
        translator.inline_module_names = inline_module_names.to_vec();
        translator.fn_return = returns;
        translator.owns_self = func.self_kind == Some(types::SelfKind::Value);
        func.body_ts = Some(translator.translate_fn_block(block, &owned_params));
        translator.pop_scope();
        // Fallbacks taken on translation paths that carry no sink of their own.
        diag::pending::drain(sink);
    }
    if func.body_ts.is_some() {
        func.body_ast = None;
    }
}

/// Does this written type project through `Self` — `Self::Target`, `&Self::Item`?
/// Those are the ones whose TypeScript name the syntactic mapping cannot write.
fn projects_through_self(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path) => {
            path.path.segments.len() > 1 && path.path.segments[0].ident == "Self"
        }
        syn::Type::Reference(r) => projects_through_self(&r.elem),
        syn::Type::Paren(p) => projects_through_self(&p.elem),
        syn::Type::Group(g) => projects_through_self(&g.elem),
        _ => false,
    }
}

/// Convert Rust file path to TS module name (for import resolution)
/// e.g., "id.rs" → "./id", "lib.rs" → ".", "foo/mod.rs" → "./foo"
fn rs_to_ts_module(rs_path: &str) -> String {
    let base = rs_path.replace(".rs", "");
    let base = base.replace("mod", "index").replace("lib", "index");
    if base == "index" {
        ".".to_string()
    } else {
        format!("./{}", base)
    }
}

/// Convert Rust file path to TS file path
fn rs_to_ts_path(rs_path: &str) -> String {
    let mut ts = rs_path.replace(".rs", ".ts");

    // mod.rs → index.ts, lib.rs → index.ts
    ts = ts.replace("mod.ts", "index.ts");
    ts = ts.replace("lib.ts", "index.ts");

    // yrs → yjs (E5)
    ts = ts.replace("yrs.ts", "yjs.ts");

    ts
}
