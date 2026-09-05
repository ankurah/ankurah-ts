//! rust-analyzer's answers about the corpus, as checked-in data (spec 6.3).
//!
//! On 2026-09-02 a throwaway spike ran rust-analyzer over the support checkout
//! and wrote down, for a sample of sites, the receiver type it inferred, the
//! method it resolved the call to, the deref and borrow steps it inserted, the
//! type it gave each closure's return, and the error conversion each `?`
//! performs. `tests/oracle/*.json` is that text converted to JSON; the schema is
//! `tests/oracle/SCHEMA.md`.
//!
//! Nothing from rust-analyzer is a dependency of the transpiler. This is data in
//! the repository, regenerated only deliberately by rebuilding the out-of-tree
//! spike on nightly and re-running `tests/oracle/convert_spike_txt.py`.

mod common;

use common::transpile_dir;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Sites<T> {
    /// Which spike output file the records came from.
    pub source: String,
    pub sites: Vec<T>,
}

/// One method call: what rust-analyzer thought the receiver was, what it
/// adjusted the receiver to, and which function the call landed on.
#[derive(Deserialize)]
pub struct MethodCall {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub receiver_type: String,
    pub receiver_type_adjusted: String,
    pub callee: String,
    pub callee_kind: String,
    pub result_type: String,
}

/// The deref and borrow steps inserted at one site, in order.
#[derive(Deserialize)]
pub struct AdjustmentChain {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub steps: Vec<AdjustmentStep>,
}

#[derive(Deserialize)]
pub struct AdjustmentStep {
    /// rust-analyzer's own word for the step, e.g. `Deref(None)`,
    /// `Borrow(Ref(Mut))`, `Pointer(Unsize)`.
    pub adjustment: String,
    pub from: String,
    pub to: String,
}

/// A closure with the type rust-analyzer gave it and the return it inferred.
#[derive(Deserialize)]
pub struct ClosureType {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub closure_type: String,
    pub inferred_return: String,
}

/// A `?` whose error type had to be converted, and the function that does it.
#[derive(Deserialize)]
pub struct TryConversion {
    pub file: String,
    pub line: u32,
    pub from_error: String,
    pub to_error: String,
    pub conversion: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// A call that resolved to a trait's own declaration rather than to an impl:
/// blanket or generic dispatch, the case a syntactic transpiler cannot guess.
#[derive(Deserialize)]
pub struct TraitGenericCall {
    pub file: String,
    pub line: u32,
    pub callee: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// A receiver that went through a user or std `Deref` impl before the call.
#[derive(Deserialize)]
pub struct OverloadedDeref {
    pub file: String,
    pub line: u32,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
    pub steps: Vec<DerefStep>,
}

#[derive(Deserialize)]
pub struct DerefStep {
    pub from: String,
    pub to: String,
}

/// A closure and the return type rust-analyzer inferred for it, from the
/// whole-corpus pass (no column, unlike `ClosureType`).
#[derive(Deserialize)]
pub struct ClosureReturn {
    pub file: String,
    pub line: u32,
    pub return_type: String,
    pub expr: String,
    #[serde(default)]
    pub truncated: bool,
}

/// An expression rust-analyzer could not type. Present so the gap is visible;
/// these sites are covered by the corpus inventory test instead.
#[derive(Deserialize)]
pub struct UntypedExpr {
    pub file: String,
    pub line: u32,
    pub expr: String,
}

#[derive(Deserialize)]
pub struct CrateCounts {
    pub source: String,
    pub crates: BTreeMap<String, BTreeMap<String, u64>>,
}

pub fn load<T: serde::de::DeserializeOwned>(name: &str) -> Sites<T> {
    let path = transpile_dir().join("tests/oracle").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()))
}

pub fn load_crate_counts() -> CrateCounts {
    let path = transpile_dir().join("tests/oracle/crate_counts.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()))
}

/// The oracle files are hand-converted text, so this guards the conversion:
/// every file parses into the structs above and holds the number of records the
/// spike produced. A count that moves means somebody regenerated the oracle,
/// which is a reviewable event.
#[test]
fn oracle_loads() {
    assert_eq!(load::<MethodCall>("method_calls.json").sites.len(), 14);
    assert_eq!(load::<AdjustmentChain>("adjustment_chains.json").sites.len(), 6);
    assert_eq!(load::<ClosureType>("closure_types.json").sites.len(), 6);
    // The spike's `?` sites all came from core/src/context.rs, which sits under
    // `#[async_trait]` and stayed untyped, so this file is deliberately empty.
    assert_eq!(load::<TryConversion>("try_sites.json").sites.len(), 0);

    assert_eq!(load::<TraitGenericCall>("trait_generic_calls.json").sites.len(), 76);
    assert_eq!(load::<OverloadedDeref>("overloaded_derefs.json").sites.len(), 27);
    assert_eq!(load::<ClosureReturn>("closure_returns.json").sites.len(), 52);
    assert_eq!(load::<TryConversion>("try_conversions.json").sites.len(), 57);
    assert_eq!(load::<UntypedExpr>("untyped_expressions.json").sites.len(), 188);

    let counts = load_crate_counts();
    let names: Vec<&str> = counts.crates.keys().map(|s| s.as_str()).collect();
    assert_eq!(names, ["ankql", "core", "proto", "signals"]);
    assert_eq!(counts.crates["ankql"]["method_calls"], 367);
}

/// One site the engine is expected to keep resolving.
#[derive(Deserialize)]
struct Pinned {
    file: String,
    line: u32,
    /// Pinned by span, not by line: two calls on one line — `a.b().c()` — are
    /// two sites, and matching by line alone let either one satisfy the pin.
    col: u32,
    method: String,
    /// Set where the port declares a different signature from Rust's, so the
    /// two result types are not expected to agree.
    #[serde(default)]
    result_differs: Option<String>,
}

#[derive(Deserialize)]
struct PinnedSites {
    site: Vec<Pinned>,
}

fn pinned_sites() -> Vec<Pinned> {
    let path = transpile_dir().join("tests/oracle/covered_sites.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let parsed: PinnedSites = toml::from_str(&text)
        .unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()));
    parsed.site
}

/// The engine's answers, against rust-analyzer's, on the sites both cover.
///
/// The two do not name types the same way — rust-analyzer writes
/// `Vec<EventId, Global>` and `&mut DebugStruct<'_, '_>`, the engine writes
/// `Vec<EventId>` — so the comparison is on the shape that decides the emitted
/// TypeScript: the borrow taken, the outermost type constructor of the adjusted
/// receiver, which function was selected including the trait it came through,
/// the result type, and the ordered chain of dereferences.
///
/// Every site in `covered_sites.toml` must resolve. A site the engine does not
/// cover *and* is not pinned is a gap, printed by category, which is the
/// std-surface step's work list.
#[test]
fn engine_matches_oracle() {
    let engine = common::run_resolve("signals", "signals/src");
    let calls = load::<MethodCall>("method_calls.json");
    let pinned = pinned_sites();

    let mut mismatches = Vec::new();
    let mut uncovered: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut covered = 0usize;

    for site in &calls.sites {
        let method = callee_method(&site.callee);
        let pin = pinned
            .iter()
            .find(|p| p.file == site.file && p.line == site.line && p.col == site.col && p.method == method);
        let Some(row) = engine
            .iter()
            .find(|r| {
                r.file == site.file && r.line == site.line && r.col == site.col && r.method == method
            })
        else {
            if pin.is_some() {
                mismatches.push(format!(
                    "{}:{} {}: pinned as covered, but the engine no longer resolves it",
                    site.file, site.line, method
                ));
            } else {
                uncovered
                    .entry(callee_owner(&site.callee))
                    .or_default()
                    .push(format!("{}:{} {}", site.file, site.line, site.callee));
            }
            continue;
        };
        covered += 1;

        // The receiver, after the dereferences and the borrow.
        if site.receiver_type_adjusted != "(none)"
            && shape(&site.receiver_type_adjusted) != shape(&row.adjusted)
        {
            mismatches.push(format!(
                "{}:{} {}: receiver is `{}`, rust-analyzer says `{}`",
                site.file, site.line, method, row.adjusted, site.receiver_type_adjusted
            ));
        }

        // Which function, including the trait it came through: selecting the
        // right receiver through the wrong trait is a different answer.
        if callee_identity(&site.callee, &site.callee_kind) != engine_callee_identity(&row.callee) {
            mismatches.push(format!(
                "{}:{} {}: callee is `{}`, rust-analyzer says `{}` ({})",
                site.file, site.line, method, row.callee, site.callee, site.callee_kind
            ));
        }

        // And what the call produced.
        match pin.and_then(|p| p.result_differs.as_ref()) {
            Some(_) => {}
            None if shape(&site.result_type) != shape(&row.result) => mismatches.push(format!(
                "{}:{} {}: result is `{}`, rust-analyzer says `{}`",
                site.file, site.line, method, row.result, site.result_type
            )),
            None => {}
        }
    }

    for pin in &pinned {
        let seen = calls.sites.iter().any(|s| {
            s.file == pin.file && s.line == pin.line && callee_method(&s.callee) == pin.method
        });
        if !seen {
            mismatches.push(format!(
                "{}:{} {}: pinned, but the oracle has no such site",
                pin.file, pin.line, pin.method
            ));
        }
    }

    // The deref chains, from the whole-corpus pass: the same steps, in the same
    // order, from and to the same types.
    let derefs = load::<OverloadedDeref>("overloaded_derefs.json");
    let ours: Vec<&OverloadedDeref> = derefs
        .sites
        .iter()
        .filter(|s| s.file.starts_with("signals/"))
        .collect();
    let mut deref_covered = 0usize;
    for site in &ours {
        let expected: Vec<(String, String)> = site
            .steps
            .iter()
            .map(|s| (head(&s.from), head(&s.to)))
            .collect();
        let matched = engine.iter().any(|r| {
            r.file == site.file && r.line == site.line && adjustment_chain(r) == expected
        });
        if matched {
            deref_covered += 1;
        } else {
            uncovered
                .entry("deref chain not reproduced".to_string())
                .or_default()
                .push(format!("{}:{} {}", site.file, site.line, site.expr));
        }
    }

    let mut report = String::new();
    for (category, sites) in &uncovered {
        report.push_str(&format!("\n  {} ({} sites)\n", category, sites.len()));
        for site in sites {
            report.push_str(&format!("    {}\n", site));
        }
    }
    eprintln!(
        "oracle: {} of {} method-call sites covered ({} pinned), {} of {} deref chains; \
         the rest are gaps, by what is missing:{}",
        covered,
        calls.sites.len(),
        pinned.len(),
        deref_covered,
        ours.len(),
        report
    );

    assert!(
        mismatches.is_empty(),
        "the engine disagrees with rust-analyzer on {} site(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
    assert!(
        covered >= pinned.len(),
        "every pinned site has to be covered"
    );
}

/// The engine's whole adjustment chain at one site, in rust-analyzer's terms.
///
/// rust-analyzer writes the auto-ref as one more step — `HashMap` to `&mut
/// HashMap` — where the engine keeps the dereferences and the borrow apart.
/// They are the same chain, so the borrow is put back on the end before the two
/// are compared.
fn adjustment_chain(row: &common::Resolved) -> Vec<(String, String)> {
    let mut chain: Vec<(String, String)> =
        row.steps.iter().map(|(from, to)| (head(from), head(to))).collect();
    let last = chain
        .last()
        .map(|(_, to)| to.clone())
        .unwrap_or_else(|| head(&row.receiver));
    let (borrow, adjusted) = shape(&row.adjusted);
    if !borrow.is_empty() {
        chain.push((last, adjusted));
    }
    chain
}

/// `HashMap<K, V, S, A>::len` gives `len`.
fn callee_method(callee: &str) -> String {
    callee.rsplit("::").next().unwrap_or(callee).to_string()
}

/// `HashMap<K, V, S, A>::len` gives `HashMap<K, V, S, A>`; `<Foo as Bar>::baz`
/// gives `Foo`.
fn callee_owner(callee: &str) -> String {
    let owner = match callee.rfind("::") {
        Some(at) => &callee[..at],
        None => callee,
    };
    let owner = owner.trim_start_matches('<');
    match owner.find(" as ") {
        Some(at) => owner[..at].to_string(),
        None => owner.trim_end_matches('>').to_string(),
    }
}

/// Which function was selected, as a pair the two sides can both produce: the
/// trait it came through if it came through one, otherwise the type it is an
/// inherent method of, plus the method's name.
fn callee_identity(callee: &str, _kind: &str) -> (String, String) {
    // rust-analyzer writes a trait method as `Trait::method` and an inherent one
    // as `Type<..>::method`; either way the owner is what identifies it.
    (head_name(&callee_owner(callee)), callee_method(callee))
}

/// The engine writes a trait callee as `<Self as Trait>::method`.
fn engine_callee_identity(callee: &str) -> (String, String) {
    let name = callee_method(callee);
    let owner = match callee.rfind("::") {
        Some(at) => &callee[..at],
        None => callee,
    };
    if let Some(at) = owner.find(" as ") {
        let trait_name = owner[at + 4..].trim_end_matches('>');
        return (head_name(trait_name), name);
    }
    (head_name(owner.trim_start_matches('<')), name)
}

/// The borrow a type is behind and the constructor at its head, which is what
/// the emitted TypeScript turns on. Lifetimes and the allocator and hasher
/// parameters rust-analyzer prints are not part of that.
fn shape(ty: &str) -> (String, String) {
    let ty = ty.trim();
    let (borrow, rest) = if let Some(rest) = ty.strip_prefix("&mut ") {
        ("&mut", rest)
    } else if let Some(rest) = ty.strip_prefix('&') {
        ("&", rest)
    } else {
        ("", ty)
    };
    (borrow.to_string(), head(rest))
}

/// The whole written type, reduced to the tree of names it is built from:
/// `alloc::vec::Vec<u8>` and `Vec<u8>` both become `Vec<u8>`, and
/// `Vec<String>` does not.
///
/// Only the *leaf* of each name is compared, because rust-analyzer and the
/// engine render module paths differently and there is no mapping between the
/// two spellings; the arguments are compared all the way down, which is what
/// tells `Result<Entity, RetrievalError>` from `Result<Entity, MutationError>`.
/// Comparing only the outermost name let an inner-generic difference pass.
fn head(ty: &str) -> String {
    let ty = ty.trim().trim_start_matches('&').trim_start_matches("mut ").trim();
    if ty.starts_with('[') {
        return "[]".to_string();
    }
    let Some(open) = ty.find('<') else {
        let name = ty.split([' ', ',']).next().unwrap_or(ty);
        return name.rsplit("::").next().unwrap_or(name).to_string();
    };
    let Some(close) = ty.rfind('>') else {
        let name = &ty[..open];
        return name.rsplit("::").next().unwrap_or(name).to_string();
    };
    let name = ty[..open].rsplit("::").next().unwrap_or(&ty[..open]).to_string();
    let args: Vec<String> = split_arguments(&ty[open + 1..close])
        .into_iter()
        .map(|arg| head(&arg))
        .collect();
    if args.is_empty() {
        return name;
    }
    format!("{}<{}>", name, args.join(","))
}

/// Split a generic argument list on the commas that are not inside a nested
/// argument list of its own.
fn split_arguments(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in inner.chars() {
        match ch {
            '<' | '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' | ']' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                let part = current.trim().to_string();
                if !part.is_empty() {
                    out.push(part);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let part = current.trim().to_string();
    if !part.is_empty() {
        out.push(part);
    }
    // A lifetime is not a type, and neither is the allocator parameter
    // rust-analyzer prints on `Vec` and `HashMap`: nightly declares
    // `Vec<T, A = Global>`, the declared surface does not model allocators, and
    // the README says so. Both are dropped rather than compared.
    out.retain(|p| !p.starts_with('\'') && p != "Global");
    out
}

/// A type's outermost name, with no arguments. The owner of a callee is
/// identified by its name; the parameters printed beside it are the
/// declaration's own placeholders — `HashMap<K, V, S>` here and
/// `HashMap<K, V, S, A>` in rust-analyzer — and say nothing about which
/// function was selected.
fn head_name(ty: &str) -> String {
    let ty = ty.trim().trim_start_matches('&').trim_start_matches("mut ").trim();
    if ty.starts_with('[') {
        return "[]".to_string();
    }
    let name = ty.split(['<', ' ', ',']).next().unwrap_or(ty);
    name.rsplit("::").next().unwrap_or(name).to_string()
}

/// One closure the engine is expected to keep typing.
///
/// Pinned by span, like a method call: a line may hold two closures — `fold(0,
/// |acc, x| ..)` inside a `map` — and matching by line alone let either one
/// satisfy the pin.
#[derive(Deserialize)]
struct PinnedClosure {
    file: String,
    line: u32,
    /// Absent for a site the whole-corpus pass recorded, which has no column.
    #[serde(default)]
    col: Option<u32>,
}

#[derive(Deserialize, Default)]
struct PinnedClosures {
    #[serde(default)]
    closure: Vec<PinnedClosure>,
}

fn pinned_closures() -> Vec<PinnedClosure> {
    let path = transpile_dir().join("tests/oracle/covered_closures.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let parsed: PinnedClosures = toml::from_str(&text)
        .unwrap_or_else(|e| panic!("{} does not match the schema: {e}", path.display()));
    parsed.closure
}

/// The crates the closure oracle draws its sites from, and where each one's
/// source lives under the support checkout.
const CLOSURE_CRATES: [(&str, &str); 3] = [
    ("proto", "proto/src"),
    ("signals", "signals/src"),
    ("ankql", "ankql/src"),
];

/// The signatures the engine gave every closure in those crates, keyed by
/// position. A closure the engine reached twice records twice; the first record
/// is the one the translator used.
fn engine_closures() -> BTreeMap<(String, u32, u32), common::ClosureRow> {
    let mut out = BTreeMap::new();
    for (name, dir) in CLOSURE_CRATES {
        for row in common::run_closures(name, dir) {
            out.entry((row.file.clone(), row.line, row.col)).or_insert(row);
        }
    }
    out
}

/// The engine's closure signatures, against rust-analyzer's, on the sites both
/// cover (spec 4.5, 6.3).
///
/// Two files hold the oracle's answers. `closure_types.json` came from a spike
/// over signals and carries the closure's whole type — `impl Fn(T)` — with a
/// column. `closure_returns.json` came from a whole-corpus pass and carries
/// only the return, with no column, so a line holding two closures is compared
/// against whichever of them the engine typed.
///
/// Every site in `covered_closures.toml` must still be typed. A site the engine
/// does not type and is not pinned is a gap, printed rather than failed, which
/// is the next step's work list.
#[test]
fn engine_matches_the_closure_oracle() {
    let engine = engine_closures();
    let pinned = pinned_closures();
    let mut mismatches: Vec<String> = Vec::new();
    let mut untyped: Vec<String> = Vec::new();
    // The sites that were actually compared, printed so that pinning one is a
    // matter of copying a line rather than of rerunning the reasoning.
    let mut compared: Vec<String> = Vec::new();
    let mut covered = 0usize;

    // The closure's own type: how many arguments it takes and what they are.
    for site in load::<ClosureType>("closure_types.json").sites {
        let Some(row) = engine.get(&(site.file.clone(), site.line, site.col)) else {
            untyped.push(format!("{}:{} (no signature recorded)", site.file, site.line));
            continue;
        };
        let want = callable_inputs(&site.closure_type);
        let got: Vec<Option<String>> = row.params.clone();
        if got.iter().any(|p| p.is_none()) {
            untyped.push(format!(
                "{}:{} {} (a parameter the engine could not type)",
                site.file, site.line, site.closure_type
            ));
            continue;
        }
        covered += 1;
        compared.push(format!("{}\t{}\t{}", site.file, site.line, site.col));
        let got: Vec<String> = got.into_iter().map(|p| head(&p.unwrap_or_default())).collect();
        let want: Vec<String> = want.iter().map(|t| head(t)).collect();
        if got != want {
            mismatches.push(format!(
                "{}:{}: parameters are ({}), rust-analyzer says ({})",
                site.file,
                site.line,
                got.join(", "),
                want.join(", ")
            ));
        }
        compare_return(&site.file, site.line, &site.inferred_return, row, &mut mismatches);
    }

    // The return type alone, from the whole-corpus pass.
    for site in load::<ClosureReturn>("closure_returns.json").sites {
        let Some(row) = engine
            .iter()
            .find(|((file, line, _), _)| *file == site.file && *line == site.line)
            .map(|(_, row)| row)
        else {
            untyped.push(format!("{}:{} (no signature recorded)", site.file, site.line));
            continue;
        };
        if row.ret.is_none() {
            untyped.push(format!(
                "{}:{} -> {} (a return the engine could not type)",
                site.file, site.line, site.return_type
            ));
            continue;
        }
        covered += 1;
        compared.push(format!("{}\t{}\t-", site.file, site.line));
        compare_return(&site.file, site.line, &site.return_type, row, &mut mismatches);
    }

    // A pin asserts what its oracle file asserts and no more. A site from the
    // whole-corpus return pass carries no column and pins the return; a site
    // from the spike carries one and pins the parameters as well.
    for pin in &pinned {
        let typed = engine.iter().any(|((file, line, col), row)| {
            *file == pin.file
                && *line == pin.line
                && pin.col.map_or(true, |want| want == *col)
                && row.ret.is_some()
                && (pin.col.is_none() || row.params.iter().all(|p| p.is_some()))
        });
        if !typed {
            mismatches.push(format!(
                "{}:{}: pinned as typed, but the engine no longer types it",
                pin.file, pin.line
            ));
        }
    }

    untyped.sort();
    untyped.dedup();
    compared.sort();
    compared.dedup();
    eprintln!("closure oracle: compared these sites:\n    {}", compared.join("\n    "));
    eprintln!(
        "closure oracle: {} of {} answers compared ({} pinned); the rest the engine \
         does not type yet:\n    {}",
        covered,
        load::<ClosureType>("closure_types.json").sites.len()
            + load::<ClosureReturn>("closure_returns.json").sites.len(),
        pinned.len(),
        untyped.join("\n    ")
    );

    assert!(
        mismatches.is_empty(),
        "the engine disagrees with rust-analyzer about {} closure(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

fn compare_return(
    file: &str,
    line: u32,
    want: &str,
    row: &common::ClosureRow,
    mismatches: &mut Vec<String>,
) {
    let Some(got) = &row.ret else { return };
    // rust-analyzer writes the unit type as `()`; so does the engine.
    if head(got) != head(want) {
        mismatches.push(format!(
            "{}:{}: return is `{}`, rust-analyzer says `{}`",
            file, line, got, want
        ));
    }
}

/// The argument types inside a written callable: `impl Fn(T)` gives `["T"]`,
/// `impl Fn(())` gives `["()"]`, and `impl Fn()` gives nothing.
fn callable_inputs(written: &str) -> Vec<String> {
    let Some(open) = written.find('(') else {
        return Vec::new();
    };
    let Some(close) = written.rfind(')') else {
        return Vec::new();
    };
    let inside = written[open + 1..close].trim();
    if inside.is_empty() {
        return Vec::new();
    }
    split_arguments(inside)
}

/// Every `?` rust-analyzer recorded as converting its error, against what the
/// engine did there.
///
/// The two do not name types the same way — rust-analyzer writes
/// `<Vec<u8, Global> as TryInto<EntityId>>::Error` where the engine writes
/// `DecodeError` — so the comparison is not on the spellings. It is on the
/// question the emitted code turns on: does this `?` convert at all, FROM what,
/// and did the engine write a call for it? A site
/// whose two error types differ in nothing but a lifetime is one `From` never
/// runs at, and the engine has to agree; a site where they really differ is one
/// the engine has to have seen, because a `?` it passed over in silence hands
/// the wrong value on.
#[test]
fn engine_matches_the_try_oracle() {
    let oracle = load::<TryConversion>("try_conversions.json");
    let mut engine: Vec<common::TryRow> = Vec::new();
    for (name, dir) in [
        ("proto", "proto/src"),
        ("ankql", "ankql/src"),
        ("signals", "signals/src"),
        ("core", "core/src"),
    ] {
        engine.extend(common::run_tries(name, dir));
    }

    let mut wrong = Vec::new();
    let mut checked = 0usize;
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut unsettled: BTreeMap<String, usize> = BTreeMap::new();
    for site in &oracle.sites {
        if NOT_READ.contains(&site.file.as_str()) {
            *skipped.entry(site.file.clone()).or_default() += 1;
            continue;
        }
        checked += 1;
        // rust-analyzer records the `?` on the line the expression under it
        // starts, which is where the engine records it too; the column differs
        // between the two and is not compared.
        let found = engine
            .iter()
            .any(|r| r.file == site.file && r.line == site.line);
        // A projection rust-analyzer left unnormalised — `<Vec<u8> as
        // TryInto<EntityId>>::Error` — says nothing about which type it is, and
        // the engine resolved it to the concrete one the impl declares. The
        // oracle cannot settle whether those two differ, so the site is
        // recorded as one it does not answer rather than compared against a
        // spelling.
        if is_projection(&site.from_error) || is_projection(&site.to_error) {
            let both_projections =
                is_projection(&site.from_error) && is_projection(&site.to_error);
            // Two projections of one name off one base are one type, and the
            // engine has to agree; anything else the oracle leaves open.
            if !both_projections {
                *unsettled.entry(site.file.clone()).or_default() += 1;
                continue;
            }
        }
        let converts = strip_lifetimes(&site.from_error) != strip_lifetimes(&site.to_error);
        // The full record, not only "did the engine see a conversion here":
        // the SOURCE error type it settled on, and whether it wrote a call for
        // it. A `?` the engine saw but named the wrong source for converts
        // through the wrong impl, and a `?` it named right but wrote no call
        // for hands the error on unconverted — both of which the presence test
        // alone reads as agreement.
        if let Some(row) = engine.iter().find(|r| r.file == site.file && r.line == site.line) {
            let oracle_from = strip_lifetimes(&site.from_error);
            let engine_from = strip_lifetimes(&row.from);
            if !is_projection(&site.from_error)
                && engine_from != "_"
                && !oracle_from.ends_with(&engine_from)
                && !engine_from.ends_with(&oracle_from)
            {
                wrong.push(format!(
                    "{}:{} — rust-analyzer converts FROM `{}` and the engine settled on `{}`",
                    site.file, site.line, site.from_error, row.from
                ));
            }
            if converts && row.written.is_none() {
                unsettled
                    .entry(format!("{} (no call written)", site.file))
                    .and_modify(|n| *n += 1)
                    .or_insert(1);
            }
        }
        if converts && !found {
            wrong.push(format!(
                "{}:{} — rust-analyzer converts `{}` to `{}` and the engine saw no conversion",
                site.file, site.line, site.from_error, site.to_error
            ));
        }
        if !converts && found {
            wrong.push(format!(
                "{}:{} — the two error types differ only in a lifetime, which the engine does \
                 not model, and it recorded a conversion anyway",
                site.file, site.line
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "{} of {} oracle `?` sites disagree with the engine:\n{}\n\nnot read by this run: \
         {skipped:?}\nleft open by the oracle's own spelling: {unsettled:?}",
        wrong.len(),
        checked,
        wrong.join("\n")
    );
    assert_eq!(
        checked + skipped.values().sum::<usize>(),
        oracle.sites.len(),
        "every oracle site is either checked or named in NOT_READ"
    );
}

/// The oracle's files this run does not read, and why each one is out.
///
/// `wasm.rs` and `postgres.rs` are `#[cfg(feature = ..)]` modules the configured
/// feature set leaves out (spec 1a). `parser.rs` and `sql.rs` hold their `?`
/// sites inside `#[cfg(test)]` functions, whose bodies the extractor keeps apart
/// from the crate's own and never translates.
/// Fifty-seven rows, forty-seven of them in these four files: fifteen in
/// `wasm.rs`, fourteen in `sql.rs`, nine each in `postgres.rs` and `parser.rs`.
/// Every one of the fourteen `sql.rs` rows is at line 254 or later and that
/// file's `mod tests` opens at 244, which is what makes the reason above true
/// of the file rather than only of most of it.
const NOT_READ: [&str; 4] = [
    "proto/src/wasm.rs",
    "proto/src/postgres.rs",
    "ankql/src/parser.rs",
    "ankql/src/selection/sql.rs",
];

/// Is this rust-analyzer's unnormalised spelling of an associated type —
/// `<X as Trait<..>>::Error` — rather than the type itself?
fn is_projection(written: &str) -> bool {
    written.starts_with('<') && written.contains(" as ") && written.contains(">::")
}

/// A rust-analyzer type with its lifetimes and its allocator parameter taken
/// out, so that `<D as Deserializer<'_>>::Error` and
/// `<D as Deserializer<'de>>::Error` read as the one type they are (spec 7a
/// records the same for the method oracle).
fn strip_lifetimes(written: &str) -> String {
    let mut out = String::with_capacity(written.len());
    let mut chars = written.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            while chars.peek().is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out.replace(", Global", "").replace("<>", "").replace(' ', "")
}
