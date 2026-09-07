"""Two defect scans over the emitted output.

Committed here so a reviewer can rerun them (DD10): every slice's report cites
these two numbers, and until now the program that produced them lived in a
scratchpad nobody else had.

    python3 transpile/tests/scans/emitted_defects.py <emitted-dir>

`<emitted-dir>` is a directory of emitted crates — one produced by
`ankurah-transpile batch` for each crate — not the repository's `packages/`,
which carries hand-written files the emitter never wrote.

1. A receiver OR AN ARGUMENT read twice. Every form below takes each of its
   subjects once, so a plain name appearing both as one of its arguments and
   again elsewhere on the same line is a double evaluation. Arguments are split
   by balancing brackets rather than by matching text, so a ternary or an infix
   operator inside one does not split it.

2. An unparenthesised expression carrying a ternary or a binary operator
   standing under one of the suffixes this slice writes. `a ? b : c.drop()`
   drops the last operand, not the whole thing.

    python3 scan.py <emitted-dir>
"""
import re, sys, pathlib

ROOT = pathlib.Path(sys.argv[1])

# The calls the recent slices write, plus the ones their items REACH. A form
# added by a later slice belongs here: the scan says nothing about a call it
# does not know.
CALLS = [
    "new SeqCursor(", "countOwned(", "rangeContains(",
    "filterOwned(", "skipOwned(", "takeOwned(", "stepByOwned(",
    "iterLastOwned(", "iterFindOwned(", "iterFindMapOwned(",
    "dropOwned(", "dropUnbound(", "takeField(", "intoMatch(", "unsupported(",
    # Slice 12's own forms: the JSON halves' per-instantiation callbacks, and
    # the readers they are passed to.
    "toJSON(", "fromJson(", "jsonAll(",
]
SUFFIX = [
    ".drainRest()", ".takeRest()", ".next()", ".drop()", ".isMoved", ".isDropped",
    ".value", ".intoMatch(", ".takeField(", ".length",
    ".toJSON(", ".fromJson(",
]

BINARY = re.compile(r'[^=!<>]=[^=]|[+\-*/%<>]|&&|\|\||\?.*:')
NAME = re.compile(r'^[A-Za-z_$][\w$.]*$')


def balanced_back(text, at):
    depth = 0
    i = at - 1
    while i >= 0:
        c = text[i]
        if c in ')]}':
            depth += 1
        elif c in '([{':
            if depth == 0:
                return text[i + 1:at]
            depth -= 1
        elif depth == 0 and c in ',;=':
            return text[i + 1:at]
        i -= 1
    return text[:at]


def arguments(text, at):
    """Every argument of the call whose `(` is at `at`."""
    out, depth, start, i = [], 0, at + 1, at + 1
    while i < len(text):
        c = text[i]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            if depth == 0:
                out.append(text[start:i])
                return out
            depth -= 1
        elif c == ',' and depth == 0:
            out.append(text[start:i])
            start = i + 1
        i += 1
    out.append(text[start:])
    return out


repeated, unparenthesised = [], []
for path in sorted(ROOT.rglob('*.ts')):
    for number, line in enumerate(path.read_text().split('\n'), 1):
        for call in CALLS:
            at = 0
            while True:
                at = line.find(call, at)
                if at < 0:
                    break
                opens = at + len(call) - 1
                for arg in arguments(line, opens):
                    arg = arg.strip()
                    if not (NAME.match(arg) and len(arg) > 1):
                        continue
                    rest = line[:at] + line[opens:].replace(arg, '', 1)
                    if re.search(r'(?<![\w$.])' + re.escape(arg) + r'(?![\w$])', rest):
                        repeated.append(
                            f'{path.relative_to(ROOT)}:{number}: {call}… {arg} … — `{arg}` again on the line')
                at += len(call)
        for suffix in SUFFIX:
            at = 0
            while True:
                at = line.find(suffix, at)
                if at < 0:
                    break
                before = balanced_back(line, at).strip()
                if BINARY.search(before) and not before.startswith('('):
                    unparenthesised.append(f'{path.relative_to(ROOT)}:{number}: `{before}`{suffix}')
                at += len(suffix)

print(f'{ROOT}')
print(f'  receivers or arguments read twice: {len(repeated)}')
for row in repeated:
    print('   ', row)
print(f'  unparenthesised under a suffix: {len(unparenthesised)}')
for row in unparenthesised:
    print('   ', row)
