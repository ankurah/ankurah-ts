use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use clap::Parser;
use walkdir::WalkDir;

mod bincode_module;
mod body;
mod codegen;
mod config;
mod control_flow;
mod drop_analysis;
mod emit;
mod extract;
mod imports;
mod macros;
mod match_expr;
mod name_map;
mod ownership;
mod resolve;
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
    /// Analyze which types transitively contain Drop types
    DropAnalysis {
        /// Path to Rust source directory
        #[arg()]
        path: PathBuf,
    },

    /// Generate TypeScript skeleton from a Rust source file
    Skeleton {
        /// Path to a Rust source file
        #[arg()]
        file: PathBuf,

        /// Rust crate path for MIRRORS annotation (e.g., "proto/src/id.rs")
        #[arg(long)]
        crate_path: Option<String>,
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::DropAnalysis { path } => {
            drop_analysis::analyze(&path)?;
        }
        Command::Skeleton { file, crate_path } => {
            let rust_file = extract::extract(&file)?;
            let crate_path = crate_path.unwrap_or_else(|| file.display().to_string());
            let ts = codegen::generate_ts(&rust_file, &crate_path);
            print!("{}", ts);
        }
        Command::Batch { src_dir, out_dir, crate_name } => {
            // Load config if transpile.toml exists
            let config_path = PathBuf::from("transpile.toml");
            let config = if config_path.exists() {
                Some(config::Config::load(&config_path)?)
            } else {
                None
            };
            batch_generate(&src_dir, &out_dir, &crate_name, config.as_ref())?;
        }
    }

    Ok(())
}

fn batch_generate(src_dir: &Path, out_dir: &Path, crate_name: &str, config: Option<&config::Config>) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output dir {}", out_dir.display()))?;

    // Phase 1: Parse all files and build type→file map
    let mut parsed_files: Vec<(String, types::RustFile)> = Vec::new();
    let mut type_to_file: std::collections::HashMap<String, String> = std::collections::HashMap::new();

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
        if let Some(cfg) = config {
            let full_path = format!("{}/src/{}", crate_name, rel_str);
            if cfg.is_excluded_file(&full_path) || cfg.is_hardcoded(&full_path) {
                eprintln!("  SKIP {} (config)", rel_str);
                continue;
            }
        }

        let rust_file = match extract::extract(rs_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("SKIP {}: {}", rel_str, e);
                continue;
            }
        };

        // Register all types defined in this file
        let ts_module = rs_to_ts_module(&rel_str);
        for s in &rust_file.structs {
            type_to_file.insert(s.name.clone(), ts_module.clone());
        }
        for e in &rust_file.enums {
            type_to_file.insert(e.name.clone(), ts_module.clone());
        }
        for t in &rust_file.traits {
            type_to_file.insert(t.name.clone(), ts_module.clone());
        }

        parsed_files.push((rel_str, rust_file));
    }

    // Add cross-crate type mappings from config
    if let Some(cfg) = config {
        for (type_name, package) in &cfg.cross_crate_types {
            type_to_file.insert(type_name.clone(), package.clone());
        }
    }

    // Phase 2: Build type registry from all extracted signatures + system types
    let system_types = config.map(|c| c.system_types.as_slice()).unwrap_or(&[]);
    let registry = resolve::build_registry(&parsed_files, system_types);

    // Phase 3: Translate all deferred bodies with type context
    let total_bodies: usize = parsed_files.iter().map(|(_, f)| {
        f.functions.iter().filter(|f| f.body_ast.is_some()).count()
        + f.impls.iter().flat_map(|i| i.methods.iter()).filter(|m| m.body_ast.is_some()).count()
        + f.test_functions.iter().filter(|f| f.body_ast.is_some()).count()
    }).sum();
    eprintln!("  Phase 3: translating {} bodies with registry", total_bodies);
    translate_all_bodies(&mut parsed_files, &registry);

    // Phase 4: Generate TS with resolved imports
    let mut file_count = 0;

    for (rel_str, rust_file) in &parsed_files {
        let ts_relative = rs_to_ts_path(rel_str);
        let ts_path = out_dir.join(&ts_relative);

        if let Some(parent) = ts_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let crate_path = format!("{}/src/{}", crate_name, rel_str);
        let current_module = rs_to_ts_module(rel_str);
        let ts = codegen::generate_ts_with_imports_configured(
            rust_file, &crate_path, &type_to_file, &current_module, config);

        std::fs::write(&ts_path, &ts)
            .with_context(|| format!("Failed to write {}", ts_path.display()))?;

        file_count += 1;
        println!("  {} → {}", rel_str, ts_relative);

        // Generate test file if there are test functions
        if let Some(test_ts) = codegen::generate_test_ts(rust_file, &crate_path) {
            let test_relative = ts_relative.replace(".ts", ".test.ts");
            let test_path = out_dir.join(&test_relative);
            std::fs::write(&test_path, &test_ts)
                .with_context(|| format!("Failed to write {}", test_path.display()))?;
            file_count += 1;
            println!("  {} → {} (tests)", rel_str, test_relative);
        }
    }

    println!("\nGenerated {} files in {}", file_count, out_dir.display());
    Ok(())
}

/// Phase 3: Translate all deferred function bodies with type registry context.
/// This runs after all files are parsed and the registry is built, so every
/// function body has access to the full crate's type information.
fn translate_all_bodies(
    files: &mut [(String, types::RustFile)],
    registry: &resolve::TypeRegistry,
) {
    for (path, file) in files.iter_mut() {
        let module = path.trim_end_matches(".rs")
            .replace("mod", "index")
            .replace("lib", "index");

        // Translate free functions
        for func in &mut file.functions {
            translate_fn_body(func, "Self", registry, &module);
        }

        // Translate impl methods
        for imp in &mut file.impls {
            let self_type = imp.target_type.clone();
            for method in &mut imp.methods {
                translate_fn_body(method, &self_type, registry, &module);
            }
        }

        // Translate test functions
        for func in &mut file.test_functions {
            translate_fn_body(func, "Self", registry, &module);
        }

        // Translate trait default methods
        for tr in &mut file.traits {
            for method in &mut tr.methods {
                translate_fn_body(method, "Self", registry, &module);
            }
        }
    }
}

/// Translate a single function's body_ast → body_ts with type-aware context.
fn translate_fn_body(
    func: &mut types::FnInfo,
    self_type: &str,
    registry: &resolve::TypeRegistry,
    module: &str,
) {
    if let Some(ref block) = func.body_ast {
        let translator = body::BodyTranslator::with_registry(self_type, registry, module);
        func.body_ts = Some(translator.translate_block(block));
    }
    // Free the AST memory after translation
    if func.body_ts.is_some() {
        func.body_ast = None;
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
