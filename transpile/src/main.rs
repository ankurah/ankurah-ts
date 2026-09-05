use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod bincode_module;
mod body;
mod calls;
mod cfg;
mod codegen;
mod config;
mod control_flow;
mod convert;
mod derives;
mod diag;
mod emit;
mod emit_impls;
mod extract;
mod imports;
mod infer;
mod json_module;
mod macros;
mod match_expr;
mod name_map;
mod native_types;
mod operators;
mod ownership;
mod registry;
mod siblings;
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
                hand_written: false,
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
            for row in trace::try_rows() {
                println!("TRYCONV\t{}", row);
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
    // Every path in transpile.toml is written the way the corpus lays the crate
    // out — `storage/sqlite/src/connection.rs` — so the file being read is named
    // that way too, not by the TypeScript package it becomes.
    let corpus_prefix = corpus_prefix_for(src_dir, crate_name, config);

    // The port's crate scope is `[crates]`, and a crate of the corpus outside it
    // is a hard error: a silent skip is how a whole crate falls out of the port
    // with nothing said. A source tree that is not the corpus — a golden, a unit
    // fixture — is not a crate of the port, and the scope has nothing to say
    // about it; the run reports that it stood aside rather than passing quietly.
    if let Some(cfg) = config {
        let from_corpus = corpus_prefix != format!("{crate_name}/src");
        if from_corpus && !cfg.is_in_scope(crate_name) {
            bail!(
                "`{}` is a crate of the corpus and is not in the port's crate scope. \
                 transpile.toml's [crates] table has: {}. Add it there with the TypeScript \
                 package it becomes, or transpile one of those.",
                crate_name,
                cfg.packages_in_scope().join(", ")
            );
        }
        if !from_corpus && !cfg.is_in_scope(crate_name) {
            eprintln!(
                "  scope: `{}` is not under {} and is not in [crates]; the crate-scope check \
                 does not apply",
                crate_name,
                cfg.paths.rust_source.display()
            );
        }
    }
    let features = config.map(|c| c.features_for_package(crate_name));

    // Phase 1: Parse all files and build type→file map
    // Which `[[provided]]` and `[[extra_exports]]` entries named a file the
    // walk actually saw. An entry naming a file that does not exist was ignored
    // in silence, and that failure has already bitten once: every one of the
    // fifteen entries was matching nothing because `[paths] rust_source`
    // resolved wrong from a worktree, and it was found by accident.
    let mut provided_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
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

        // An excluded file is not in the port at all, so its types do not exist
        // here. A provided module is different: its TypeScript is hand-written,
        // so nothing may be emitted over it, but the types it declares are part
        // of the crate and other files resolve through them.
        let mut declarations_only = false;
        let mut excluded_here: Vec<&config::ExcludedItem> = Vec::new();
        let full_path = format!("{}/{}", corpus_prefix, rel_str);
        if let Some(cfg) = config {
            if cfg.is_excluded_file(&full_path) {
                eprintln!("  SKIP {} (excluded)", rel_str);
                continue;
            }
            if let Some(provided) = cfg.provided_module(&full_path) {
                eprintln!(
                    "  PROVIDED {} → {} ({})",
                    rel_str,
                    provided.module,
                    first_line(&provided.reason)
                );
                provided_seen.insert(provided.file.clone());
                declarations_only = true;
            }
            for extra in cfg.extra_exports_in(&full_path) {
                provided_seen.insert(extra.file.clone());
            }
            excluded_here = cfg.excluded_items_in(&full_path);
        }

        let extract_cfg = extract::ExtractCfg {
            features: features.as_ref(),
            excluded: &excluded_here,
        };
        let mut rust_file = match extract::extract_with_cfg(rs_path, extract_cfg) {
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

        // A `#[cfg]` nothing decided, and an `[[excluded_items]]` entry that
        // named nothing in the file it points at, are both reported here, where
        // the file that raised them is still in hand.
        sink.set_file(&rel_str);
        diag::pending::drain(&sink);
        let hit = extract::take_exclusions_hit();
        for entry in &excluded_here {
            if !hit.contains(&entry.written) {
                sink.push(diag::Diag {
                    file: rel_str.clone(),
                    line: 0,
                    col: 0,
                    message: format!(
                        "transpile.toml excludes `{}` from this file and there is no such item; \
                         the config has gone stale against the corpus",
                        entry.written
                    ),
                });
            }
        }

        // Register all types defined in this file. A provided module's types go
        // in too, named by the hand-written TypeScript module they live in, so
        // an import of one resolves the same way an emitted type's does.
        let ts_module = rs_to_ts_module(&rel_str);
        if declarations_only {
            if let Some(provided) = config.and_then(|c| c.provided_module(&full_path)) {
                let module = format!("./{}", provided.module);
                for name in rust_file
                    .structs
                    .iter()
                    .map(|s| s.name.clone())
                    .chain(rust_file.enums.iter().map(|e| e.name.clone()))
                    .chain(rust_file.traits.iter().map(|t| t.name.clone()))
                    // A provided module's public functions are part of what it
                    // offers: ankql's parser is one, and `parse_selection` is
                    // what every caller of it names.
                    .chain(
                        rust_file
                            .functions
                            .iter()
                            .filter(|f| f.is_pub)
                            .map(|f| f.ts_name.clone()),
                    )
                {
                    type_to_file.insert(name, module.clone());
                }
            }
            parsed_files.push(registry::ExtractedFile {
                path: rel_str,
                file: rust_file,
                declarations_only,
                // Everything read this way in THIS crate is a `[[provided]]`
                // module: its members are whatever the person who wrote the
                // file wrote.
                hand_written: true,
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
        // A module-level `pub fn` is a name every caller imports, exactly as a
        // type is. Without it a call to one resolved to nothing at run time —
        // `generateSelectionSql is not defined` in ankql's own test suite.
        for f in rust_file.functions.iter().filter(|f| f.is_pub) {
            type_to_file
                .entry(f.ts_name.clone())
                .or_insert_with(|| ts_module.clone());
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
            hand_written: declarations_only,
        });
    }

    // Batch walks the filesystem, so the file order depends on the directory
    // listing. Sort it, so the registry and the diagnostics are the same on
    // every machine.
    parsed_files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut sibling_idents: Vec<String> = Vec::new();
    // Phase 1b: the in-family crates this one depends on, read for their
    // declarations. Their types then have real ids here rather than foreign
    // ones, and the import map sends them to their own package.
    if let Some(cfg) = config {
        if let Some(cargo_name) = cfg.cargo_crate_for_package(crate_name) {
            let root = std::fs::canonicalize(&cfg.paths.rust_source)
                .unwrap_or_else(|_| cfg.paths.rust_source.clone());
            let located = siblings::locate(cfg);
            for sibling in siblings::dependencies_of(cfg, &located, &cargo_name) {
                sibling_idents.push(sibling.ident.clone());
                let loaded = siblings::declarations(&sibling, cfg, &root)?;
                eprintln!(
                    "  sibling {} ({} files) → {}",
                    sibling.ident,
                    loaded.files.len(),
                    sibling.package
                );
                // A sibling file the parser refused is a hole in what THIS
                // crate can resolve: every type it declared is a foreign name
                // here. It was passed over in silence, so a crate could lose a
                // dependency's declarations and emit against nothing.
                for failure in &loaded.failures {
                    eprintln!("  SIBLING PARSE FAILURE {}", failure);
                    sink.set_file("");
                    sink.push(diag::Diag {
                        file: String::new(),
                        line: 0,
                        col: 0,
                        message: format!(
                            "the in-family crate `{}` has a file this run could not read, so the \
                             types it declares resolve to nothing here: {}",
                            sibling.ident, failure
                        ),
                    });
                }
                for entry in &loaded.files {
                    for name in entry
                        .file
                        .structs
                        .iter()
                        .map(|s| s.name.clone())
                        .chain(entry.file.enums.iter().map(|e| e.name.clone()))
                        .chain(entry.file.traits.iter().map(|t| t.name.clone()))
                    {
                        // This crate's own declarations win: a name it declares
                        // is its own, whatever a sibling calls the same thing.
                        type_to_file
                            .entry(name)
                            .or_insert_with(|| sibling.package.clone());
                    }
                }
                parsed_files.extend(loaded.files);
            }
            parsed_files.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }

    // A `pub mod x;` whose file the port leaves out declares a module that is
    // not there. The declaration is still in the parent — `#[cfg(feature =
    // "wasm")] pub mod wasm;` is live under the port's feature set even though
    // wasm.rs is excluded by item — so the parent's re-export has to go with the
    // file, or `index.ts` names a module nothing writes.
    if let Some(cfg) = config {
        for entry in parsed_files.iter_mut() {
            let parent = entry.path.clone();
            entry.file.mod_decls.retain(|(name, _)| {
                let child = child_file_of(&parent, name);
                !cfg.is_excluded_file(&format!("{}/{}", corpus_prefix, child))
            });
        }
    }

    // Add cross-crate type mappings from config
    if let Some(cfg) = config {
        for (type_name, package) in &cfg.cross_crate_types {
            type_to_file.insert(type_name.clone(), package.clone());
        }
    }

    // Phase 2: Build the registry from the declared std surface and everything
    // this crate declares. The surface is parsed once per run and declared into
    // each registry the run builds.
    // A `[[provided]]` or `[[extra_exports]]` entry naming a file the walk
    // never saw. An `[[excluded_items]]` entry that matched nothing has been
    // reported since step 8; these two were ignored, so a provided module whose
    // Rust file is renamed silently starts being transpiled over.
    if let Some(cfg) = config {
        let mut stale: Vec<&str> = Vec::new();
        for entry in cfg.provided_modules.iter().chain(cfg.extra_exports.iter()) {
            if entry.file.starts_with(&format!("{}/", corpus_prefix))
                && !provided_seen.contains(&entry.file)
            {
                stale.push(&entry.file);
            }
        }
        stale.sort();
        for file in stale {
            sink.push(diag::Diag {
                file: file.to_string(),
                line: 0,
                col: 0,
                message: format!(
                    "transpile.toml names `{}` as hand-written and this crate's source walk \
                     never saw it; the config has gone stale against the corpus",
                    file
                ),
            });
        }
    }

    let crate_names = crate_names_for(crate_name, config);
    let surface_dir = registry::std_surface::default_dir(std_surface_dir);
    let mut registry = registry::std_surface::with_cached(&surface_dir, |surface| {
        if surface.is_empty() {
            eprintln!(
                "WARNING no std surface found at {}; every std type will be undeclared",
                surface_dir.display()
            );
        }
        registry::build_registry_with_siblings(
            &mut parsed_files,
            surface,
            &crate_names,
            &sibling_idents,
            &sink,
        )
    });
    mark_hand_written_types(&mut registry, &parsed_files, config);
    // A type whose JSON half is refused has no `fromJson`, and neither does
    // anything that holds one. Asked AFTER the hand-written marking, because a
    // type whose TypeScript somebody wrote carries its own pair.
    let ours: std::collections::HashSet<registry::ModuleId> = parsed_files
        .iter()
        .filter(|f| !f.declarations_only)
        .filter_map(|f| registry.modules().lookup_file(&f.path))
        .collect();
    registry::narrow_reads_json(&mut registry, &ours);
    let registry = registry;

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
            // The import map is keyed by name alone, so two files each emitting
            // a function of one name would have every call to either import
            // whichever file was read last.
            if let Some(other) = type_to_file.get(&f.name) {
                if *other != ts_module {
                    sink.set_file(&entry.path);
                    sink.push(diag::Diag {
                        file: entry.path.clone(),
                        line: 0,
                        col: 0,
                        message: format!(
                            "`{}` is emitted as a module-level function here and in `{}`, and \
                             the import map is keyed by the name alone, so every call to \
                             either reaches one of them",
                            f.name, other
                        ),
                    });
                }
            }
            type_to_file.insert(f.name, ts_module.clone());
        }
        for d in emit_impls::dispatchers(&registry, module, &entry.file) {
            type_to_file.insert(d.name, ts_module.clone());
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

        // The corpus path, which is what `transpile.toml` writes and what the
        // MIRRORS header should name: `storage/sqlite/src/engine.rs`, not the
        // TypeScript package's `storage-sqlite/src/engine.rs`, which is not a
        // path in the Rust repository at all.
        let crate_path = format!("{}/{}", corpus_prefix, rel_str);
        let current_module = rs_to_ts_module(rel_str);
        // Emission is the last place that asks the engine a question — a derive
        // hook wanting a field's `Debug`, a format string wanting a type — so
        // what it could not answer is filed against this file, the way a
        // fallback taken during translation is.
        sink.set_file(rel_str);
        let ts = codegen::generate_ts_with_imports_configured(
            &registry,
            rust_file,
            &crate_path,
            &type_to_file,
            &current_module,
            config,
        );

        diag::pending::drain(&sink);

        std::fs::write(&ts_path, &ts)
            .with_context(|| format!("Failed to write {}", ts_path.display()))?;

        file_count += 1;
        println!("  {} → {}", rel_str, ts_relative);

        // Generate inline module files. The test module is NOT one: its
        // declarations belong in the `.test.ts` beside the tests that name
        // them, and a `tests.ts` of its own would be a production module the
        // package index re-exports.
        for (mod_name, sub_file) in rust_file
            .inline_modules
            .iter()
            .filter(|(name, _)| Some(name) != rust_file.test_module.as_ref())
        {
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
            &registry,
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
    report_cfg_decisions(crate_name);
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

/// Where the crate being transpiled sits under the corpus, so a path in
/// `transpile.toml` — `storage/sqlite/src/connection.rs` — names the same file
/// the walk is reading. `batch` is handed the `src` directory, so the answer is
/// that path relative to `[paths] rust_source`; when the two share no prefix
/// (a test corpus, a scratch tree) the crate's own name stands in.
fn corpus_prefix_for(src_dir: &Path, crate_name: &str, config: Option<&config::Config>) -> String {
    let fallback = format!("{crate_name}/src");
    let Some(cfg) = config else { return fallback };
    let root = std::fs::canonicalize(&cfg.paths.rust_source)
        .unwrap_or_else(|_| cfg.paths.rust_source.clone());
    let here = std::fs::canonicalize(src_dir).unwrap_or_else(|_| src_dir.to_path_buf());
    match here.strip_prefix(&root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.display().to_string(),
        // The source directory is not under the corpus, so where this crate
        // sits there cannot be read off it and the crate's own name stands in.
        // Every `[[provided]]`, `[[excluded_items]]` and `[excluded_files]`
        // entry is matched against that spelling, so a crate whose directory is
        // not its package name — `storage/sqlite` for `storage-sqlite` — would
        // match none of them, which is worth saying out loud.
        _ => {
            eprintln!(
                "  corpus: {} is not under {}, so this crate is named `{}` for the purpose of \
                 transpile.toml's per-file entries",
                here.display(),
                root.display(),
                fallback
            );
            fallback
        }
    }
}

/// A `&mut` parameter whose type JavaScript copies.
///
/// `fn render(buffer: &mut String, ..)` grows the caller's string in Rust. A
/// JavaScript string, number, boolean or bigint is passed by value, so what the
/// body assigns to the parameter the caller never sees, and the function
/// silently produces nothing — ankql's SQL renderer threads one through six
/// recursive helpers and answered `''` for every query.
///
/// There is no shape in the port that fixes this: the parameter would have to
/// become a return value or a holder object, and either is a change to what the
/// function means. So it is reported at the signature.
/// Where `mod x;` written in `parent` puts x's file, crate-relative. A crate
/// root or a `mod.rs` keeps its children beside it; any other module keeps them
/// in a directory named after itself.
fn child_file_of(parent: &str, child: &str) -> String {
    let dir = parent.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let stem = parent.rsplit('/').next().unwrap_or(parent).trim_end_matches(".rs");
    match (dir, stem) {
        ("", "lib") | ("", "mod") => format!("{child}.rs"),
        ("", other) => format!("{other}/{child}.rs"),
        (dir, "lib") | (dir, "mod") => format!("{dir}/{child}.rs"),
        (dir, other) => format!("{dir}/{other}/{child}.rs"),
    }
}

/// The first line of a multi-line reason, for a one-line progress message.
fn first_line(reason: &str) -> &str {
    reason.trim().lines().next().unwrap_or("").trim()
}

/// What the run decided every `#[cfg]` predicate it met to be. The crate
/// inventory's claim is that every predicate the corpus writes is decided; this
/// is the line a run answers it with.
fn report_cfg_decisions(crate_name: &str) {
    let rows = cfg::decisions();
    if rows.is_empty() {
        return;
    }
    let undecided = rows.iter().filter(|(_, a, _)| a.is_none()).count();
    eprintln!(
        "  cfg: {} predicates decided, {} undecided",
        rows.len() - undecided,
        undecided
    );
    for (predicate, answer, sites) in &rows {
        let verdict = match answer {
            Some(true) => "true",
            Some(false) => "false",
            None => "UNDECIDED",
        };
        eprintln!("CFG\t{}\t{}\t{}\t{}", crate_name, predicate, verdict, sites);
    }
}

/// The names this crate answers to in a written path: the TypeScript package
/// name the run was given, plus the Cargo and Rust spellings of the crate it
/// maps to, so `ankurah_proto::id::EntityId` written inside proto resolves.
/// Record which types the emitter will not write TypeScript for.
///
/// Two kinds: a `[provided_impls]` entry, whose TypeScript is a `.provided.ts`
/// file, and everything declared in a `[hardcode]` file, whose TypeScript is
/// kept as it stands. Both are still declared — their fields have types and
/// their derives register impls — but their *members* are whatever the person
/// who wrote the file wrote, so a hook must not call a method it did not emit.
fn mark_hand_written_types(
    registry: &mut registry::TypeRegistry,
    files: &[registry::ExtractedFile],
    config: Option<&config::Config>,
) {
    let mut ids = Vec::new();
    for entry in files.iter().filter(|e| e.hand_written) {
        let Some(module) = registry.modules().lookup_file(&entry.path) else {
            continue;
        };
        let names = entry
            .file
            .structs
            .iter()
            .map(|s| s.name.clone())
            .chain(entry.file.enums.iter().map(|e| e.name.clone()));
        for name in names {
            if let Some(id) = registry.module_type(module, &name) {
                ids.push(id);
            }
        }
    }
    if let Some(cfg) = config {
        for fqn in cfg.provided_impls.keys() {
            // `ankurah_proto::id::EntityId` — the crate name, then the module
            // path the registry knows the type by. Read WITHOUT the crate name
            // this is the path inside the crate being transpiled, which is the
            // only crate whose impls this run emits. A SIBLING's provided type
            // is deliberately not marked here: "hand-written" stops an impl on
            // the type being emitted at all, and an impl THIS crate writes for a
            // sibling's type is this crate's own code — core's
            // `impl OrderedCollation for EntityId` has to be emitted, as the
            // module-level functions an impl away from its class becomes.
            let segments: Vec<String> = fqn.split("::").skip(1).map(|s| s.to_string()).collect();
            if segments.is_empty() {
                continue;
            }
            if let Ok(Some(registry::Def::Type(id))) =
                registry.lookup_type(registry.crate_root(), &segments)
            {
                ids.push(id);
            }
        }
    }
    for id in ids {
        registry.mark_hand_written(id);
    }
}

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

/// The members every ported type inherits from `AkObject`, and — for an enum —
/// from `Enum`.
///
/// A Rust field with one of these names becomes a class property that shadows
/// the runtime's own member, and the ownership machinery then reads the field
/// where it meant to read itself.
///
/// These are the runtime's *contract*: what the emitted code calls on a ported
/// value. Its conveniences are not here. The diagnostic accused `label` in
/// eight proto and core fields until the runtime renamed its own accessor to
/// `$label` — the ruling being that the runtime moves and the port does not,
/// because renaming a field changes the wire protocol. A name that is only
/// spelled inside the runtime cannot collide with anything the port emits, so
/// listing one here reports a collision that does not exist.
const RUNTIME_MEMBERS: [&str; 8] = [
    "drop",
    "isDropped",
    "isMoved",
    "onDrop",
    "ownedFields",
    "takeField",
    "assertNotDropped",
    "markMoved",
];

/// The members an emitted enum inherits on top of those, from `Enum`.
///
/// `type` and `value` are the variant tag and its payload, `match` and
/// `intoMatch` are how every emitted arm reads them, and `is` is the narrowing
/// test. A *method* of any of those names replaces the machinery that reads the
/// variant. A variant's *fields* do not: they are keys of the payload object
/// the class holds, not members of the class, so a field called `value` is
/// `this.value.value` and collides with nothing.
const ENUM_MEMBERS: [&str; 5] = ["type", "value", "match", "intoMatch", "is"];

/// Say so where a declared name is one the runtime already uses.
fn report_member_collisions(
    file: &types::RustFile,
    reg: &registry::TypeRegistry,
    module: registry::ModuleId,
    sink: &diag::DiagSink,
) {
    let self_id = |name: &str| reg.module_type(module, name);
    for s in &file.structs {
        for f in &s.fields {
            let Some(name) = &f.name else { continue };
            if !RUNTIME_MEMBERS.contains(&name.as_str()) {
                continue;
            }
            let start = syn::spanned::Spanned::span(&f.rust_ty).start();
            sink.push(diag::Diag {
                file: sink.file(),
                line: start.line,
                col: start.column + 1,
                message: format!(
                    "`{}.{}` has the name of a member every ported type inherits from \
                     `AkObject`, so the field shadows the runtime's own and the ownership \
                     checks read the wrong one",
                    s.name, name
                ),
            });
        }
    }
    // A method takes a class member where a field takes a property, so the same
    // names are at stake — and an enum's methods are at stake against `Enum`'s
    // as well. The name checked is the one emission writes, not the one the
    // source wrote: `impl Drop for T { fn drop }` is emitted as `onDrop`, which
    // is the hook the runtime declares for exactly that.
    for imp in &file.impls {
        let on_an_enum = file.enums.iter().any(|e| e.name == imp.target_type);
        let trait_name = imp.trait_name();
        let type_args = imp.trait_type_args();
        // `impl Drop` is *meant* to land on `onDrop`: that is the hook the
        // runtime declares for it, called between the mark and the cascade.
        if trait_name.as_deref() == Some("Drop") {
            continue;
        }
        for m in &imp.methods {
            let emitted = match &trait_name {
                Some(trait_name) => emit::trait_method_name(
                    trait_name,
                    &type_args,
                    m,
                    &imp.target_type,
                    self_id(&imp.target_type),
                ),
                None => m.ts_name.clone(),
            };
            let taken = RUNTIME_MEMBERS.contains(&emitted.as_str())
                || (on_an_enum && ENUM_MEMBERS.contains(&emitted.as_str()));
            if !taken {
                continue;
            }
            sink.push(diag::Diag {
                file: sink.file(),
                line: 0,
                col: 0,
                message: format!(
                    "`{}::{}` is emitted as `{}`, which is a member every ported type inherits \
                     from the runtime, so the method replaces the runtime's own",
                    imp.target_type, m.name, emitted
                ),
            });
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
    translate_module_consts(file, registry, module, &consts, sink);
    report_member_collisions(file, registry, module, sink);
    // The module-level functions this file's impls become, asked for once with
    // the run's sink so that a name two impls would take is reported here and
    // nowhere else.
    let _ = emit_impls::free_functions_reporting(registry, module, file, sink);
    // Read while the bodies are still ASTs: translation drops them below.
    file.assigned_fields = emit::assigned_fields(file);

    // A test module's functions are DECLARED here and translated as the
    // parent's `test_functions`; translating them twice would count every gap
    // in them twice.
    for func in file.functions.iter_mut().filter(|_| !file.is_test_module) {
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
            Some(ty)
                if emit_impls::emits_as_free_function(
                    registry,
                    ty,
                    &imp.type_params,
                    module,
                ) =>
            {
                "self"
            }
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

    // A test body is written INSIDE `mod tests`, so it resolves in that
    // module: `TestEntity::new(..)` names a struct the test module declares,
    // and asking the parent's scope for it answered "does not name a function
    // here" at every fixture in the corpus.
    let test_scope = file
        .test_module
        .as_ref()
        .and_then(|name| registry.modules().get(module).children.get(name).copied())
        .unwrap_or(module);
    for func in file.test_helpers.iter_mut().chain(file.test_functions.iter_mut()) {
        translate_fn_body(
            func,
            "Self",
            "this",
            None,
            &[],
            &[],
            registry,
            test_scope,
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

/// What each module-level `const` and `static` IS, written as TypeScript.
///
/// The initialiser goes through the ordinary expression path with the const's
/// own declared type as the expectation, so `const TAG: u8 = 0x00` writes `0`
/// and `const WORDLIST: &[&str; 256] = &[..]` writes the array. A const the
/// translator cannot write keeps `undefined` and says so AT THE CONST, where
/// before the stub was emitted in silence and the failure surfaced as an
/// `undefined` index somewhere else entirely.
fn translate_module_consts(
    file: &mut types::RustFile,
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    consts: &[(String, ty::Ty)],
    sink: &diag::DiagSink,
) {
    // A `thread_local!` writes its own declaration; this list is only what
    // codegen emits as a `const`.
    let declared: Vec<String> = file.module_decls.clone();
    for c in file.consts.iter_mut() {
        if declared.iter().any(|d| d.contains(&c.name)) {
            continue;
        }
        let Some(init) = c.init.clone() else { continue };
        let mut tc = infer::TypeContext::new(registry, module, None, Vec::new(), sink);
        for (name, ty) in consts {
            tc.bind(name, ty.clone());
        }
        let want = c
            .rust_ty
            .as_ref()
            .and_then(|written| tc.resolve_written_type(written).ok());
        let translator = body::BodyTranslator::with_context("Self", tc);
        let written = match &want {
            Some(ty) => translator.expecting(&init, Some(ty), || translator.expr_value(&init)),
            None => translator.expr_value(&init),
        };
        translator.pop_scope();
        diag::pending::drain(sink);
        c.init_ts = Some(written);
        c.init = None;
    }
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

        // Every parameter and the return type, written from the type the
        // engine resolved rather than from the syntax.
        //
        // The syntactic mapping cannot see what a name means: `Self` came out
        // as the word `Self`, `Self::Target` as the bare associated name, and a
        // crate type sharing a leaf with a std one as the std one's spelling.
        // `map_ty` reproduces that mapping case for case where the two agree,
        // so a signature only moves where the syntax was wrong. A type the
        // engine could not name keeps what it had, which is where it stood
        // before.
        for param in func.params.iter_mut() {
            let Some(written) = param.rust_ty.as_ref() else {
                continue;
            };
            if names_an_alias(registry, module, written) {
                continue;
            }
            if let Ok(resolved) = quiet_type(&tc, written) {
                param.ty = name_map::map_ty(registry, &resolved);
            }
        }
        if let Some(written) = func.rust_return.as_ref() {
            if !names_an_alias(registry, module, written) {
                if let Ok(resolved) = quiet_type(&tc, written) {
                    func.return_type = name_map::map_ty(registry, &resolved);
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
        // C1: a `&mut T` parameter whose `T` the port writes as a JavaScript
        // VALUE is a `BorrowMut<T>`, and the body reads and writes it through
        // `.value`. Without the cell the callee's writes went nowhere.
        let cell_params: Vec<String> = func
            .params
            .iter()
            .filter(|p| is_boxed_mut(p))
            .map(|p| p.name.clone())
            .collect();
        *translator.boxed.borrow_mut() = cell_params.clone();
        *translator.cell_params.borrow_mut() = cell_params;
        translator.self_name = self_name;
        translator.inline_module_names = inline_module_names.to_vec();
        translator.fn_return = returns;
        translator.owns_self = func.self_kind == Some(types::SelfKind::Value);
        // A `fmt` taking a `Formatter` is a formatter body: its `write!` calls
        // compose one string, and the `Ok(())` it ends with is that string.
        translator.formatter = func.name == "fmt"
            && !body::writes_once_at_the_tail(block)
            && func.params.iter().any(|p| {
                p.rust_ty.as_ref().is_some_and(|ty| {
                    let written = quote::ToTokens::to_token_stream(ty).to_string();
                    written.contains("Formatter")
                })
            });
        func.body_ts = Some(translator.translate_fn_block(block, &owned_params));
        translator.pop_scope();
        // Fallbacks taken on translation paths that carry no sink of their own.
        diag::pending::drain(sink);
    }
    if func.body_ts.is_some() {
        func.body_ast = None;
    }
}

/// Is this parameter a `&mut` to something the port writes as a JavaScript
/// VALUE, so that a write through it needs a runtime cell?
///
/// A `&mut` to a class is already a reference in JavaScript and needs nothing:
/// `fn fill(v: &mut Vec<u8>)` writes into the array the caller passed. A number,
/// a string, a boolean and a bigint are copied at the call, and so is a
/// nullable of one.
pub(crate) fn is_boxed_mut(param: &types::ParamInfo) -> bool {
    let Some(syn::Type::Reference(reference)) = &param.rust_ty else {
        return false;
    };
    if reference.mutability.is_none() {
        return false;
    }
    is_value_spelling(&param.ty)
}

/// Is this TypeScript spelling a value JavaScript copies?
pub(crate) fn is_value_spelling(ty: &str) -> bool {
    let bare = ty.strip_suffix(" | null").unwrap_or(ty);
    matches!(bare, "string" | "number" | "boolean" | "bigint")
}

/// Does this written type name a type alias?
///
/// A resolved type has no memory of the alias it was written as, so writing the
/// signature from it turns `Listener` into the `Arc<dyn Fn(T)>` the alias
/// stands for. The port emits the alias, and the alias is what the source said,
/// so the syntactic spelling stays where one is named.
fn names_an_alias(
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    written: &syn::Type,
) -> bool {
    match written {
        syn::Type::Path(path) => {
            let segments: Vec<String> =
                path.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if matches!(
                registry.lookup_type(module, &segments),
                Ok(Some(registry::Def::Alias(_)))
            ) {
                return true;
            }
            // An alias UNDER a wrapper is still an alias the port emits:
            // `Arc<Listener>` and `Vec<Listener>` name one as surely as a bare
            // `Listener` does, and reading only the outermost name expanded
            // them into the `Arc<dyn Fn(T)>` the alias stands for.
            path.path
                .segments
                .last()
                .into_iter()
                .filter_map(|segment| match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => Some(args),
                    _ => None,
                })
                .flat_map(|args| args.args.iter())
                .any(|arg| match arg {
                    syn::GenericArgument::Type(ty) => names_an_alias(registry, module, ty),
                    _ => false,
                })
        }
        // A reference is erased in emission, so what it points at decides.
        syn::Type::Reference(r) => names_an_alias(registry, module, &r.elem),
        syn::Type::Paren(p) => names_an_alias(registry, module, &p.elem),
        syn::Type::Group(g) => names_an_alias(registry, module, &g.elem),
        syn::Type::Slice(s) => names_an_alias(registry, module, &s.elem),
        syn::Type::Array(a) => names_an_alias(registry, module, &a.elem),
        syn::Type::Tuple(t) => t.elems.iter().any(|e| names_an_alias(registry, module, e)),
        _ => false,
    }
}

/// A written type resolved and read through the impl table, with no diagnostic
/// filed for it.
///
/// The body translation asks the same questions and reports what it could not
/// answer; asking again here to write the signature would count each gap twice.
fn quiet_type(tc: &infer::TypeContext<'_>, written: &syn::Type) -> Result<ty::Ty, diag::Diag> {
    let mark = tc.sink.mark();
    let resolved = tc.resolve_written_type(written);
    tc.sink.rewind(mark);
    Ok(tc.probe().normalize(&resolved?))
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
