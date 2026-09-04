#!/usr/bin/env python3
"""Convert the 2026-09-02 rust-analyzer spike's text inventories to JSON.

Run once; the JSON under transpile/tests/oracle/ is the checked-in artefact.
"""
import json, os, re, sys

# usage: convert_spike_txt.py <out-dir> [<spike-dir>]
# The spike directory holds inventory4.txt and spike-run3.txt, the text output of
# the out-of-tree rust-analyzer spike. ANKURAH_SUPPORT_PATH names the Rust
# checkout the spike ran over, so its absolute paths can be made relative.
OUT = sys.argv[1]
SPIKE = sys.argv[2] if len(sys.argv) > 2 else os.path.expanduser(
    "~/.claude/handoffs/ankurah-ts/main/2026-09-02-1149-ra-spike"
)
SUPPORT = os.environ.get("ANKURAH_SUPPORT_PATH", "/Users/daniel/ak/ankurah-ts-support").rstrip("/") + "/"

LABELS = {
    "files": "files",
    "expressions": "expressions",
    "  typed by RA": "expressions_typed",
    "  untyped, in macro": "untyped_in_macro",
    "  untyped, real": "untyped_real",
    "method calls": "method_calls",
    "  resolved by RA": "method_calls_resolved",
    "  inherent impl": "callee_inherent_impl",
    "  concrete trait impl": "callee_concrete_trait_impl",
    "  generic/blanket trait": "callee_generic_or_blanket_trait",
    "  recv NOT syntactic": "receiver_not_syntactic",
    "  needed auto-deref": "receiver_needed_auto_deref",
    "  Deref-impl deref": "receiver_used_deref_impl",
    "  >=2 deref steps": "receiver_deref_steps_ge2",
    "  unsize coercion": "receiver_unsize_coercion",
    "  dyn receiver": "receiver_dyn",
    "? expressions": "try_expressions",
    "  err types identical": "try_err_identical",
    "  needs From conv": "try_needs_from_conversion",
    "closures": "closures",
    "  non-unit ret": "closures_nonunit_return",
    ".into()/.try_into()": "into_or_try_into_calls",
    "operator overloads": "operator_overloads",
    "index overloads": "index_overloads",
    "prefix overloads": "prefix_overloads",
    ".await": "await_expressions",
    "impl Trait params": "impl_trait_params",
}


def rel(path):
    return path.replace(SUPPORT, "")


def crate_counts(path):
    crates, cur = {}, None
    for line in open(path):
        line = line.rstrip("\n")
        if line.startswith("### "):
            cur = rel(line[4:].strip()).split("/")[0]
            if cur == "storage":
                cur = "storage-common"
            crates[cur] = {}
            continue
        if line.startswith("====") or not line.strip():
            continue
        if line.startswith("========= EXAMPLES") or "EXAMPLES" in line:
            break
        m = re.match(r"^(.*?)\s+(\d+)$", line)
        if not m or cur is None:
            continue
        label, n = m.group(1), int(m.group(2))
        key = LABELS.get(label)
        if key is None:
            raise SystemExit(f"unknown label {label!r} in {path}")
        crates[cur][key] = n
    return crates


EXPR = r"`(.*)`"


def examples(path):
    out = []
    started = False
    for line in open(path):
        line = line.rstrip("\n")
        if "EXAMPLES" in line:
            started = True
            continue
        if not started or not line.strip():
            continue
        tag, rest = line.split(" ", 1)
        m = re.match(r"^(\S+?):(\d+)\s+(.*)$", rest)
        if not m:
            raise SystemExit(f"cannot parse example line: {line!r}")
        file, lineno, tail = m.group(1), int(m.group(2)), m.group(3)
        rec = {"kind": tag, "file": file, "line": lineno}
        if tag == "UNTYPED":
            rec["expr"] = strip_expr(tail)
        elif tag == "TRAIT-GENERIC":
            callee, expr = tail.split("  `", 1)
            rec["callee"] = callee.strip()
            rec["expr"] = strip_expr("`" + expr)
        elif tag == "CLOSURE-RET":
            ret, expr = tail.split("  `", 1)
            rec["return_type"] = ret.strip().removeprefix("ret=")
            rec["expr"] = strip_expr("`" + expr)
        elif tag == "OVERLOADED-DEREF":
            m2 = re.match(r"^`(.*)`\s+\[(.*)\]$", tail)
            if not m2:
                raise SystemExit(f"cannot parse deref line: {line!r}")
            rec["expr"], rec["truncated"] = trunc(m2.group(1))
            rec["steps"] = [
                {"from": a.strip(), "to": b.strip()}
                for a, b in (s.split(" -> ", 1) for s in m2.group(2).split(" ; "))
            ]
        elif tag == "TRY-FROM":
            m2 = re.match(r"^(.*?) -> (.*?)  via (.*?)  `(.*)`$", tail)
            if not m2:
                raise SystemExit(f"cannot parse try line: {line!r}")
            rec["from_error"] = m2.group(1)
            rec["to_error"] = m2.group(2)
            rec["conversion"] = m2.group(3)
            rec["expr"], rec["truncated"] = trunc(m2.group(4))
        else:
            raise SystemExit(f"unknown example tag {tag!r}")
        out.append(rec)
    return out


def trunc(expr):
    if expr.endswith("…"):
        return expr[:-1], True
    return expr, False


def strip_expr(tail):
    m = re.match(r"^`(.*)`$", tail.strip())
    if not m:
        raise SystemExit(f"cannot parse expr {tail!r}")
    return trunc(m.group(1))[0]


def sites(path):
    """Parse the (b)/(c)/(d)/(e) records of a spike run."""
    calls, adjusts, closures, tries = [], [], [], []
    skipped = 0
    cur = None
    for line in open(path):
        line = line.rstrip("\n")
        m = re.match(r"^\((\w)\) (\w+) (\S+?):(\d+):(\d+)\s+`(.*)`$", line)
        if m:
            expr, truncated = trunc(m.group(6))
            cur = {
                "tag": m.group(2),
                "file": rel(m.group(3)),
                "line": int(m.group(4)),
                "col": int(m.group(5)),
                "expr": expr,
            }
            if truncated:
                cur["truncated"] = True
            {"CALL": calls, "ADJUST": adjusts, "CLOSURE": closures, "TRY": tries}[m.group(2)].append(cur)
            continue
        if line.startswith("      ") and cur is not None:
            body = line.strip()
            if cur["tag"] == "ADJUST":
                kind, types = body.split(": ", 1)
                a, b = types.split(" -> ", 1)
                cur.setdefault("steps", []).append({"adjustment": kind, "from": a, "to": b})
            else:
                key, val = body.split("=", 1)
                cur[key.strip().replace(" ", "_")] = val.strip()
            continue
        cur = None

    def resolved(rec):
        return not any(v in ("<none>", "<UNRESOLVED>") for v in rec.values() if isinstance(v, str))

    RENAME = {
        "recv_orig": "receiver_type",
        "recv_adj": "receiver_type_adjusted",
        "result": "result_type",
        "inferred_ret": "inferred_return",
        "conversion_fn": "conversion",
    }

    def tidy(rec):
        out = {}
        for k, v in rec.items():
            if k == "tag":
                continue
            k = RENAME.get(k, k)
            if k == "callee":
                m = re.match(r"^(.*) \[(.*)\]$", v)
                if m:
                    out["callee"] = m.group(1)
                    out["callee_kind"] = m.group(2)
                    continue
            out[k] = v
        return out

    keep = {}
    for name, recs in (("calls", calls), ("adjustments", adjusts), ("closures", closures), ("tries", tries)):
        good = [r for r in recs if resolved(r)]
        keep[name] = [tidy(r) for r in good]
        globals()["_skipped"] = globals().get("_skipped", 0) + len(recs) - len(good)
    return keep


def write(name, obj):
    path = os.path.join(OUT, name)
    with open(path, "w") as f:
        json.dump(obj, f, indent=1, sort_keys=False)
        f.write("\n")
    print(f"{name}: {os.path.getsize(path)} bytes")


os.makedirs(OUT, exist_ok=True)
counts = crate_counts(f"{SPIKE}/inventory4.txt")
write("crate_counts.json", {"source": "inventory4.txt", "crates": counts})

ex = examples(f"{SPIKE}/inventory4.txt")
by_kind = {}
for r in ex:
    by_kind.setdefault(r["kind"], []).append({k: v for k, v in r.items() if k != "kind"})
write("untyped_expressions.json", {"source": "inventory4.txt", "sites": by_kind.get("UNTYPED", [])})
write("trait_generic_calls.json", {"source": "inventory4.txt", "sites": by_kind.get("TRAIT-GENERIC", [])})
write("closure_returns.json", {"source": "inventory4.txt", "sites": by_kind.get("CLOSURE-RET", [])})
write("overloaded_derefs.json", {"source": "inventory4.txt", "sites": by_kind.get("OVERLOADED-DEREF", [])})
write("try_conversions.json", {"source": "inventory4.txt", "sites": by_kind.get("TRY-FROM", [])})

s = sites(f"{SPIKE}/spike-run3.txt")
write("method_calls.json", {"source": "spike-run3.txt", "sites": s["calls"]})
write("adjustment_chains.json", {"source": "spike-run3.txt", "sites": s["adjustments"]})
write("closure_types.json", {"source": "spike-run3.txt", "sites": s["closures"]})
write("try_sites.json", {"source": "spike-run3.txt", "sites": s["tries"]})
print("dropped unresolved records:", globals().get("_skipped", 0))
for k, v in by_kind.items():
    print(f"  {k}: {len(v)}")
