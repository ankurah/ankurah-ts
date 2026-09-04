#!/usr/bin/env python3
"""Report which of the corpus's method calls the stub surface declares.

Two tiers, because two different questions are worth asking and only one of
them can be answered exactly.

Tier A is exact. The rust-analyzer oracle under
`transpile/tests/oracle/` records, for a sample of call sites, the
*resolved* callee: `HashMap<K, V, S, A>::len`, `Result<T, E>::unwrap`,
`Iterator::collect`. Those are (type, method) pairs an outside authority
already answered, so every one that names a std or extern type is checked
against the stubs by type and by method. A miss here is a real hole.

Tier B is a name-level sweep of the whole corpus. Nothing here resolves a
receiver — that is the engine's job, and the engine does not exist yet — so
the question it answers is narrower: is this method name declared *somewhere*,
either by ankurah's own source or by a stub? A name that is in neither is
unaccounted for, and is either a std method nobody declared or a crate this
surface does not cover. Tier B over-reports (a name ankurah defines masks a
std method of the same name) and cannot under-report.

Run:
    python3 transpile/std_surface/coverage.py [--corpus PATH] [--oracle PATH] [-v]
"""

import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TRANSPILE = os.path.dirname(HERE)
REPO = os.path.dirname(TRANSPILE)


def _first_dir(candidates):
    for c in candidates:
        if os.path.isdir(c):
            return os.path.normpath(c)
    return os.path.normpath(candidates[0])


# `transpile.toml` puts the Rust checkout at `../ankurah-ts-support` relative to
# the repo root. In a git worktree that resolves inside `.claude/worktrees/`, so
# the sibling of the *main* checkout is tried too. `ANKURAH_SUPPORT_PATH`, the
# variable the oracle converter already uses, overrides both.
DEFAULT_CORPUS = _first_dir([
    os.environ.get("ANKURAH_SUPPORT_PATH") or os.path.join(REPO, "..", "ankurah-ts-support"),
    os.path.join(REPO, "..", "ankurah-ts-support"),
    os.path.join(REPO, "..", "..", "..", "..", "ankurah-ts-support"),
])

# The oracle lands in `transpile/tests/oracle/` when the engine branch merges;
# until then it is in that branch's worktree.
DEFAULT_ORACLE = _first_dir([
    os.path.join(TRANSPILE, "tests", "oracle"),
    os.path.join(REPO, "..", "engine", "transpile", "tests", "oracle"),
])

# The crates in scope, per SYMBOL-TABLE-SPEC.md section 1a. `postgres`, `sled`,
# the tokio websocket client and server, `derive`, `tests-wasm` and `examples`
# are out of scope and are not scanned.
CORPUS_CRATES = [
    ("proto", "proto/src"),
    ("ankql", "ankql/src"),
    ("signals", "signals/src"),
    ("core", "core/src"),
    ("storage-common", "storage/common/src"),
    ("storage-sqlite", "storage/sqlite/src"),
    ("storage-indexeddb", "storage/indexeddb-wasm/src"),
    ("ws-client-wasm", "connectors/websocket-client-wasm/src"),
    ("local-process", "connectors/local-process/src"),
    ("ankurah", "ankurah/src"),
]

# The four crates the oracle and the corpus inventory both cover; the coverage
# target is zero unaccounted names on these.
CORE_FOUR = {"proto", "ankql", "signals", "core"}

# Files whose *calls* the engine never has to resolve: transpile.toml's
# `[excluded_files]` and `[hardcode]` lists, plus the modules the cfg evaluator
# drops under ankurah's wasm32 + `singlethread` configuration. Their `fn`
# definitions are still collected, because a name ankurah defines is accounted
# for wherever it is defined.
EXCLUDED = {
    "proto/src/postgres.rs",
    "signals/src/reactive_graph.rs",
    "signals/src/react.rs",
    "signals/src/react_native.rs",
    "signals/src/jsvalue.rs",
    "ankql/src/parser.rs",
    "ankql/src/grammar.rs",
    "proto/src/human_id.rs",
    "ankql/src/ast.rs",
    "ankql/src/selection/sql.rs",
    "ankql/src/lib.rs",
    "signals/src/lib.rs",
}

# Not method calls: `.await` is syntax, and the rest are macro-ish or numeric.
NOT_METHODS = {"await"}


# ── stripping Rust down to code ──────────────────────────────────────────────

def strip_noncode(src):
    """Blank out comments, string literals, char literals and lifetimes.

    Blanking rather than deleting keeps byte offsets, which keeps the later
    regexes honest about what is adjacent to what.
    """
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append(" " * (j - i))
            i = j
        elif c == "r" and i + 1 < n and src[i + 1] in "#\"":
            j = i + 1
            hashes = 0
            while j < n and src[j] == "#":
                hashes += 1
                j += 1
            if j < n and src[j] == '"':
                close = '"' + "#" * hashes
                k = src.find(close, j + 1)
                k = n if k < 0 else k + len(close)
                out.append(" " * (k - i))
                i = k
            else:
                out.append(c)
                i += 1
        elif c == '"':
            j = i + 1
            while j < n:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(" " * (j - i))
            i = j
        elif c == "'":
            # A lifetime (`'a`) or a char literal (`'x'`, `'\n'`). Both are
            # blanked; neither contributes a method call.
            m = re.match(r"'(?:\\.|[^\\'])'", src[i:])
            if m:
                out.append(" " * len(m.group(0)))
                i += len(m.group(0))
            else:
                out.append(" ")
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


# ── reading the stubs ────────────────────────────────────────────────────────

IMPL_RE = re.compile(r"\bimpl\b")
TRAIT_RE = re.compile(r"\b(?:pub\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)")
FN_RE = re.compile(r"\bfn\s+([a-z_][A-Za-z0-9_]*)")


def base_type_name(ty):
    """`HashMap<K, V, S>` -> `HashMap`; `[T]` -> `[T]`; `&'a str` -> `str`."""
    ty = ty.strip()
    ty = re.sub(r"^&(?:'[a-z_]+\s*)?(?:mut\s+)?", "", ty).strip()
    ty = re.sub(r"^dyn\s+", "", ty).strip()
    if ty.startswith("["):
        return "[T]"
    depth, cut = 0, len(ty)
    for i, ch in enumerate(ty):
        if ch == "<":
            if depth == 0:
                cut = i
            depth += 1
        elif ch == ">":
            depth -= 1
        elif ch in "+ " and depth == 0:
            cut = min(cut, i)
    ty = ty[:cut].strip()
    return ty.split("::")[-1]


def split_impl_header(header):
    """Return (trait_name_or_None, self_type) from an `impl ..` header."""
    body = header[len("impl"):].strip()
    if body.startswith("<"):
        depth, i = 0, 0
        while i < len(body):
            if body[i] == "<":
                depth += 1
            elif body[i] == ">":
                depth -= 1
                if depth == 0:
                    i += 1
                    break
            i += 1
        body = body[i:].strip()
    where = re.search(r"\bwhere\b", body)
    if where:
        body = body[: where.start()]
    m = re.search(r"\bfor\b", body)
    if m:
        # Careful: `impl<F: FnOnce() -> T> ..` has no bare `for`; the generics
        # were already consumed above, so any `for` left is the trait's.
        return body[: m.start()].strip(), body[m.end():].strip()
    return None, body.strip()


def scan_stub(path):
    """(declared_on, trait_methods, all_method_names) for one stub file."""
    src = strip_noncode(open(path, encoding="utf-8").read())
    declared_on = {}   # type name -> set of method names
    trait_methods = {} # trait name -> set of method names
    all_names = set()

    def block_end(s, open_brace):
        depth, i = 0, open_brace
        while i < len(s):
            if s[i] == "{":
                depth += 1
            elif s[i] == "}":
                depth -= 1
                if depth == 0:
                    return i
            i += 1
        return len(s)

    for m in re.finditer(r"\b(?:pub\s+)?(impl|trait)\b", src):
        kind = m.group(1)
        brace = src.find("{", m.start())
        if brace < 0:
            continue
        header = src[m.start():brace]
        # `trait X: Y { .. }` headers never contain `{`; `impl` ones can only
        # through a closure bound like `FnMut() -> R`, which has no braces
        # either, so the first `{` really is the block.
        end = block_end(src, brace)
        body = src[brace + 1:end]
        names = set(FN_RE.findall(body))
        all_names |= names
        if kind == "trait":
            tm = TRAIT_RE.search(header)
            if tm:
                trait_methods.setdefault(tm.group(1), set()).update(names)
        else:
            _trait, self_ty = split_impl_header(header.strip())
            key = base_type_name(self_ty)
            if key:
                declared_on.setdefault(key, set()).update(names)
    return declared_on, trait_methods, all_names


def load_stubs(root):
    declared_on, trait_methods, all_names = {}, {}, set()
    files = []
    for dirpath, _dirs, names in os.walk(root):
        for name in sorted(names):
            if name.endswith(".rs"):
                files.append(os.path.join(dirpath, name))
    for path in sorted(files):
        d, t, a = scan_stub(path)
        for k, v in d.items():
            declared_on.setdefault(k, set()).update(v)
        for k, v in t.items():
            trait_methods.setdefault(k, set()).update(v)
        all_names |= a
    return declared_on, trait_methods, all_names, sorted(files)


# ── reading the corpus ───────────────────────────────────────────────────────

CALL_RE = re.compile(r"\.\s*([a-z_][A-Za-z0-9_]*)\s*(?:::\s*<[^{};]*?>\s*)?\(")
DEF_RE = re.compile(r"\bfn\s+([a-z_][A-Za-z0-9_]*)")
TYPE_DEF_RE = re.compile(r"\b(?:struct|enum|trait|union|type)\s+([A-Z][A-Za-z0-9_]*)")


def scan_corpus(corpus_root):
    calls = {}         # crate -> {method name -> count}
    defined = set()    # every `fn` name ankurah declares anywhere
    corpus_types = set()  # every struct/enum/trait/type ankurah declares
    for crate, rel in CORPUS_CRATES:
        base = os.path.join(corpus_root, rel)
        if not os.path.isdir(base):
            continue
        for dirpath, _dirs, names in os.walk(base):
            for name in sorted(names):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, name)
                relpath = os.path.relpath(path, corpus_root)
                src = strip_noncode(open(path, encoding="utf-8", errors="replace").read())
                for dm in DEF_RE.finditer(src):
                    defined.add(dm.group(1))
                for tm in TYPE_DEF_RE.finditer(src):
                    corpus_types.add(tm.group(1))
                if relpath in EXCLUDED:
                    continue
                for mm in CALL_RE.finditer(src):
                    n = mm.group(1)
                    if n in NOT_METHODS:
                        continue
                    calls.setdefault(crate, {})
                    calls[crate][n] = calls[crate].get(n, 0) + 1
    return calls, defined, corpus_types


# ── the oracle ───────────────────────────────────────────────────────────────

CALLEE_RE = re.compile(r"^(?P<ty>.+)::(?P<method>[a-z_][A-Za-z0-9_]*)$")


def load_oracle(oracle_root):
    out = []
    for fname in ("method_calls.json", "trait_generic_calls.json"):
        path = os.path.join(oracle_root, fname)
        if not os.path.isfile(path):
            continue
        data = json.load(open(path, encoding="utf-8"))
        for site in data.get("sites", []):
            callee = site.get("callee")
            if not callee or callee in ("<none>", "<UNRESOLVED>"):
                continue
            m = CALLEE_RE.match(callee.strip())
            if not m:
                continue
            out.append((base_type_name(m.group("ty")), m.group("method"), callee,
                        fname, site.get("file", "")))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", default=DEFAULT_CORPUS)
    ap.add_argument("--oracle", default=DEFAULT_ORACLE)
    ap.add_argument("--stubs", default=HERE)
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()

    declared_on, trait_methods, stub_names, stub_files = load_stubs(args.stubs)
    print(f"stubs: {len(stub_files)} files, "
          f"{len(declared_on)} types with inherent or trait impls, "
          f"{len(trait_methods)} traits, {len(stub_names)} distinct method names\n")

    calls, corpus_defined, corpus_types = scan_corpus(args.corpus)
    if not calls:
        print(f"no corpus found under {args.corpus}")
        return 1

    # ── Tier A ───────────────────────────────────────────────────────────────
    oracle = load_oracle(args.oracle)
    if not oracle:
        print(f"tier A: no oracle records found under {args.oracle}\n")
        a_missing = []
    else:
        seen, a_missing, a_ok, a_own, a_excluded = set(), [], [], [], []
        for ty, method, callee, src, site_file in oracle:
            if (ty, method) in seen:
                continue
            seen.add((ty, method))
            found = method in declared_on.get(ty, ()) or method in trait_methods.get(ty, ())
            if found:
                a_ok.append((ty, method, callee, src))
            elif ty in corpus_types:
                # ankurah's own type; the engine reads its signature from
                # ankurah's source, not from a stub.
                a_own.append((ty, method, callee, src))
            elif site_file in EXCLUDED:
                a_excluded.append((ty, method, callee, site_file))
            else:
                a_missing.append((ty, method, callee, src))
        print(f"tier A - oracle-resolved callees: {len(a_ok)} declared by a stub, "
              f"{len(a_own)} on an ankurah type, {len(a_excluded)} in an excluded file, "
              f"{len(a_missing)} missing, {len(seen)} distinct pairs")
        if args.verbose:
            for ty, method, callee, _src in sorted(a_ok):
                print(f"    ok      {callee}")
        if args.verbose:
            for ty, method, callee, _src in sorted(a_own):
                print(f"    ankurah {callee}")
        for ty, method, callee, site_file in sorted(a_excluded):
            print(f"    excluded {callee}   (only called from {site_file})")
        for ty, method, callee, src in sorted(a_missing):
            print(f"    MISSING {callee}   ({src})")
        print()

    # ── Tier B ───────────────────────────────────────────────────────────────
    print("tier B - method names called, by crate")
    print(f"    {'crate':<18} {'names':>6} {'calls':>7} {'ankurah':>8} {'stub':>6} {'unaccounted':>12}")
    total_unaccounted = {}
    core_unaccounted = {}
    for crate, _rel in CORPUS_CRATES:
        names = calls.get(crate)
        if not names:
            continue
        total_calls = sum(names.values())
        own = {n for n in names if n in corpus_defined}
        stub = {n for n in names if n not in own and n in stub_names}
        un = {n: names[n] for n in names if n not in own and n not in stub_names}
        for n, c in un.items():
            total_unaccounted[n] = total_unaccounted.get(n, 0) + c
            if crate in CORE_FOUR:
                core_unaccounted[n] = core_unaccounted.get(n, 0) + c
        print(f"    {crate:<18} {len(names):>6} {total_calls:>7} "
              f"{len(own):>8} {len(stub):>6} {len(un):>12}")

    print()
    print(f"unaccounted names, four core crates ({len(core_unaccounted)}):")
    for n, c in sorted(core_unaccounted.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"    {n} ({c})")
    others = {n: c for n, c in total_unaccounted.items() if n not in core_unaccounted}
    print()
    print(f"unaccounted names, storage and connector crates ({len(others)}):")
    for n, c in sorted(others.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"    {n} ({c})")

    return 1 if (a_missing or core_unaccounted) else 0


if __name__ == "__main__":
    sys.exit(main())
