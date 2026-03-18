mod rust_parser;
mod ts_parser;
mod name_map;
mod matcher;
mod attestation;
mod reporter;
mod transpiler;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Parity checker and skeleton transpiler for ankurah Rust→TS port
#[derive(Parser)]
#[command(name = "ankurah-checker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check parity between Rust source and TS port
    Check {
        /// Only check a specific package
        #[arg(long)]
        package: Option<String>,

        /// Show verbose output
        #[arg(long)]
        verbose: bool,

        /// Verify commit hash attestations
        #[arg(long)]
        verify_attestations: bool,
    },

    /// Generate a TS skeleton from a Rust source file
    Transpile {
        /// Rust source file (relative to rust root, e.g., ankql/src/ast.rs)
        #[arg(long)]
        file: String,

        /// Write output to file instead of stdout
        #[arg(long)]
        output: Option<String>,
    },

    /// Diff generated skeleton against existing TS file
    Diff {
        /// Rust source file (relative to rust root, e.g., ankql/src/ast.rs)
        #[arg(long)]
        file: Option<String>,

        /// Check all files in a package
        #[arg(long)]
        package: Option<String>,
    },
}

/// Configuration loaded from checker.toml
#[derive(Debug, serde::Deserialize)]
struct Config {
    paths: PathsConfig,
    crates: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
struct PathsConfig {
    rust_source: String,
    ts_source: String,
}

/// A MIRRORS annotation parsed from line 1 of a TS file
#[derive(Debug, Clone)]
struct MirrorsAnnotation {
    /// The Rust path from the annotation (e.g., "ankurah/ankql/src/ast.rs")
    rust_path: String,
    /// Optional suffix like "#[cfg(test)] mod tests"
    suffix: Option<String>,
}

/// A group of TS files that mirror the same Rust file
#[derive(Debug)]
struct MirrorGroup {
    rust_file: PathBuf,
    ts_files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            package,
            verbose,
            verify_attestations,
        } => {
            run_check(package.as_deref(), verbose, verify_attestations)?;
        }
        Commands::Transpile { file, output } => {
            run_transpile(&file, output.as_deref())?;
        }
        Commands::Diff { file, package } => {
            run_diff(file.as_deref(), package.as_deref())?;
        }
    }

    Ok(())
}

fn load_config(checker_dir: &Path) -> Result<Config> {
    let config_path = checker_dir.join("checker.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        Ok(toml::from_str(&content)?)
    } else {
        // Default config
        Ok(Config {
            paths: PathsConfig {
                rust_source: "../../ankurah-ts-support".to_string(),
                ts_source: "../../packages".to_string(),
            },
            crates: default_crate_map(),
        })
    }
}

fn default_crate_map() -> HashMap<String, String> {
    [
        ("ankql", "ankql"),
        ("ankurah-proto", "proto"),
        ("ankurah-signals", "signals"),
        ("ankurah-core", "core"),
        ("ankurah-storage-common", "storage-common"),
        ("ankurah-storage-sqlite", "storage-sqlite"),
        ("ankurah-storage-postgres", "storage-postgres"),
        ("ankurah-storage-indexeddb-wasm", "storage-indexeddb"),
        ("ankurah-websocket-client", "connector-websocket"),
        ("ankurah-websocket-server", "connector-websocket-server"),
        ("ankurah-connector-local-process", "connector-local"),
        ("ankurah", "ankurah"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

// --- Check command (Phase 1) ---

fn run_check(
    package_filter: Option<&str>,
    verbose: bool,
    verify_attestations: bool,
) -> Result<()> {
    let checker_dir = std::env::current_dir()?;
    let config = load_config(&checker_dir)?;

    let rust_root = checker_dir.join(&config.paths.rust_source);
    let ts_root = checker_dir.join(&config.paths.ts_source);

    if !rust_root.exists() {
        anyhow::bail!("Rust source not found at: {}", rust_root.display());
    }
    if !ts_root.exists() {
        anyhow::bail!("TS source not found at: {}", ts_root.display());
    }

    let mirror_groups = discover_mirrors(&ts_root, &rust_root, package_filter)?;

    if mirror_groups.is_empty() {
        println!("No mirrored files found.");
        return Ok(());
    }

    let mut total_stats = reporter::Stats::default();

    for group in &mirror_groups {
        let rust_path = &group.rust_file;

        let rust_items = match rust_parser::parse_rust_file(rust_path) {
            Ok(items) => items,
            Err(e) => {
                eprintln!("  Error parsing {}: {}", rust_path.display(), e);
                continue;
            }
        };

        let mut ts_items = Vec::new();
        for ts_path in &group.ts_files {
            match ts_parser::parse_ts_file(ts_path) {
                Ok(items) => ts_items.extend(items),
                Err(e) => {
                    eprintln!("  Error parsing {}: {}", ts_path.display(), e);
                }
            }
        }

        let rust_hash = if verify_attestations {
            attestation::get_file_hash(rust_path).ok()
        } else {
            None
        };

        let match_results = matcher::match_items(&rust_items, &ts_items);

        let file_stats = reporter::report_file(
            &group.ts_files,
            rust_path,
            rust_hash.as_deref(),
            &match_results,
            verbose,
        );

        total_stats = total_stats.merge(&file_stats);
    }

    println!();
    reporter::report_summary(&total_stats);

    Ok(())
}

// --- Transpile command (Phase 2) ---

fn run_transpile(file: &str, output: Option<&str>) -> Result<()> {
    let checker_dir = std::env::current_dir()?;
    let config = load_config(&checker_dir)?;
    let rust_root = checker_dir.join(&config.paths.rust_source);

    let rust_path = rust_root.join(file);
    if !rust_path.exists() {
        anyhow::bail!("Rust file not found: {}", rust_path.display());
    }

    let skeleton = transpiler::skeleton::generate_skeleton(&rust_path, file)?;
    let rendered = skeleton.render();

    if let Some(out_path) = output {
        std::fs::write(out_path, &rendered)?;
        println!("Written to {}", out_path);
    } else {
        print!("{}", rendered);
    }

    Ok(())
}

// --- Diff command (Phase 2) ---

fn run_diff(file: Option<&str>, package: Option<&str>) -> Result<()> {
    let checker_dir = std::env::current_dir()?;
    let config = load_config(&checker_dir)?;
    let rust_root = checker_dir.join(&config.paths.rust_source);
    let ts_root = checker_dir.join(&config.paths.ts_source);

    if let Some(file) = file {
        diff_single_file(file, &rust_root, &ts_root)?;
    } else if let Some(pkg) = package {
        diff_package(pkg, &rust_root, &ts_root)?;
    } else {
        anyhow::bail!("Must specify --file or --package");
    }

    Ok(())
}

fn diff_single_file(rel_path: &str, rust_root: &Path, ts_root: &Path) -> Result<()> {
    use colored::Colorize;

    let rust_path = rust_root.join(rel_path);
    if !rust_path.exists() {
        anyhow::bail!("Rust file not found: {}", rust_path.display());
    }

    let skeleton = transpiler::skeleton::generate_skeleton(&rust_path, rel_path)?;

    // Find the corresponding TS file(s) via MIRRORS annotations
    let mirror_groups = discover_mirrors(ts_root, rust_root, None)?;
    let group = mirror_groups
        .iter()
        .find(|g| g.rust_file == rust_path);

    match group {
        Some(group) => {
            // Parse the existing TS files
            let mut existing_ts_items = Vec::new();
            for ts_path in &group.ts_files {
                match ts_parser::parse_ts_file(ts_path) {
                    Ok(items) => existing_ts_items.extend(items),
                    Err(e) => {
                        eprintln!("  Error parsing {}: {}", ts_path.display(), e);
                    }
                }
            }

            // Parse the generated skeleton to extract its items
            let rendered = skeleton.render();
            let tmp_path = PathBuf::from("generated.ts");
            let generated_items = ts_parser::parse_ts_source(&rendered, &tmp_path)?;

            // Compare
            println!(
                "\n{} ({})",
                format!("=== {} ===", rel_path).bold(),
                format!("{} TS file(s)", group.ts_files.len()).dimmed()
            );

            let mut missing_in_ts = 0;
            let mut found = 0;

            for gen_item in &generated_items {
                let matched = existing_ts_items.iter().any(|ts_item| {
                    ts_item.name == gen_item.name && ts_item.kind == gen_item.kind
                        || (ts_item.kind == gen_item.kind
                            && ts_item.parent_class == gen_item.parent_class
                            && ts_item.name == gen_item.name)
                });

                if matched {
                    found += 1;
                    println!(
                        "  {} {} {}",
                        "+".green(),
                        gen_item.kind,
                        gen_item.name,
                    );
                } else {
                    missing_in_ts += 1;
                    println!(
                        "  {} {} {} — {} (generated from Rust, not found in TS)",
                        "x".red(),
                        gen_item.kind,
                        gen_item.name,
                        "MISSING".red(),
                    );
                }
            }

            // Check for extra items in TS that weren't generated
            let mut extra_in_ts = 0;
            for ts_item in &existing_ts_items {
                let in_generated = generated_items.iter().any(|gen| {
                    gen.name == ts_item.name && gen.kind == ts_item.kind
                        || (gen.kind == ts_item.kind
                            && gen.parent_class == ts_item.parent_class
                            && gen.name == ts_item.name)
                });
                if !in_generated {
                    extra_in_ts += 1;
                    println!(
                        "  {} {} {} — {} (in TS only, not in Rust)",
                        "?".yellow(),
                        ts_item.kind,
                        ts_item.name,
                        "EXTRA".yellow(),
                    );
                }
            }

            println!(
                "\n  Generated: {} | Found in TS: {} | Missing: {} | Extra in TS: {}",
                generated_items.len(),
                found.to_string().green(),
                if missing_in_ts > 0 {
                    missing_in_ts.to_string().red().to_string()
                } else {
                    "0".to_string()
                },
                if extra_in_ts > 0 {
                    extra_in_ts.to_string().yellow().to_string()
                } else {
                    "0".to_string()
                },
            );
        }
        None => {
            println!(
                "{}",
                format!(
                    "No TS file found with MIRRORS annotation for {}. Showing generated skeleton:",
                    rel_path
                )
                .yellow()
            );
            println!();
            print!("{}", skeleton.render());
        }
    }

    Ok(())
}

fn diff_package(package: &str, rust_root: &Path, ts_root: &Path) -> Result<()> {
    use colored::Colorize;

    // Walk the rust crate directory to find all .rs files
    let pkg_dir = rust_root.join(package);
    if !pkg_dir.exists() {
        anyhow::bail!("Rust package directory not found: {}", pkg_dir.display());
    }

    let mut rs_files = Vec::new();
    for entry in WalkDir::new(&pkg_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let rel = entry
            .path()
            .strip_prefix(rust_root)
            .unwrap_or(entry.path());
        rs_files.push(rel.to_string_lossy().to_string());
    }

    rs_files.sort();

    println!(
        "{}\n",
        format!("Diffing package '{}' ({} Rust files)", package, rs_files.len()).bold()
    );

    for rel_path in &rs_files {
        if let Err(e) = diff_single_file(rel_path, rust_root, ts_root) {
            eprintln!("  Error processing {}: {}", rel_path, e);
        }
    }

    Ok(())
}

// --- Mirror discovery ---

/// Parse a MIRRORS annotation from the first line of a TS file.
fn parse_mirrors_annotation(line: &str) -> Option<MirrorsAnnotation> {
    let trimmed = line.trim();
    if !trimmed.starts_with("// MIRRORS:") {
        return None;
    }

    let after_prefix = trimmed.strip_prefix("// MIRRORS:")?.trim();

    if let Some(rs_end) = after_prefix.find(".rs") {
        let path_end = rs_end + 3;
        let rust_path = after_prefix[..path_end].trim().to_string();
        let suffix = {
            let rest = after_prefix[path_end..].trim();
            if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            }
        };
        Some(MirrorsAnnotation { rust_path, suffix })
    } else {
        None
    }
}

/// Discover all TS files with MIRRORS annotations and group them by Rust file.
fn discover_mirrors(
    ts_root: &Path,
    rust_root: &Path,
    package_filter: Option<&str>,
) -> Result<Vec<MirrorGroup>> {
    let mut groups: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(ts_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "ts")
                && !e.path().to_string_lossy().contains("node_modules")
                && !e.path().to_string_lossy().contains("/dist/")
        })
    {
        let ts_path = entry.path().to_path_buf();

        if let Some(pkg) = package_filter {
            let rel = ts_path.strip_prefix(ts_root).unwrap_or(&ts_path);
            let first_component = rel
                .components()
                .next()
                .map(|c| c.as_os_str().to_string_lossy().to_string());
            if first_component.as_deref() != Some(pkg) {
                continue;
            }
        }

        let content = match std::fs::read_to_string(&ts_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let first_line = content.lines().next().unwrap_or("");

        if let Some(annotation) = parse_mirrors_annotation(first_line) {
            let rust_rel = if annotation.rust_path.starts_with("ankurah/") {
                annotation.rust_path.strip_prefix("ankurah/").unwrap()
            } else {
                &annotation.rust_path
            };
            let rust_file = rust_root.join(rust_rel);

            if rust_file.exists() {
                groups.entry(rust_file).or_default().push(ts_path);
            } else if !annotation.rust_path.contains('(') {
                eprintln!(
                    "Warning: Rust file not found: {} (referenced by {})",
                    rust_file.display(),
                    ts_path.display()
                );
            }
        }
    }

    let mut result: Vec<MirrorGroup> = groups
        .into_iter()
        .map(|(rust_file, ts_files)| MirrorGroup { rust_file, ts_files })
        .collect();
    result.sort_by(|a, b| a.rust_file.cmp(&b.rust_file));

    Ok(result)
}
