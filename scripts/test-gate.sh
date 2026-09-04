#!/usr/bin/env bash
#
# Run bun's tests and the TypeScript typecheck one package at a time, and refuse
# to pass on a failure, an error, or a type error.
#
# Why this exists: bun abandons the rest of a test FILE at the first unhandled
# error — a throw at module scope, a fatal leak from the ownership runtime — and
# reports it on a separate " N error" line. The tests after it in that file never
# run and never appear in any count, so a run that says "0 fail" while reporting
# errors has not told you the tests passed; it has told you it stopped reading.
# `bun test` also exits 0 in some of those cases.
#
# The typecheck is here for the same reason. Until 2026-09-02 the repository root
# ran `tsc` over the whole tree including transpile/golden/, whose captured
# TypeScript does not parse; TypeScript reports only syntactic diagnostics when
# any exist and skips the semantic pass entirely, so two parse errors in a
# recording were hiding every type error in packages/. Both numbers are the same
# kind of truth, and a gate that shows one and hides the other is the thing that
# went wrong before.
#
# Usage:
#   scripts/test-gate.sh              # every workspace package
#   scripts/test-gate.sh proto core   # only those packages
#
# The transpiler's own captured output under transpile/ is deliberately out of
# scope: transpile/golden/ and transpile/tests/snapshots/ hold .test.ts files
# that are recordings of generated code, not tests of this repository. Running
# per package never reaches them; the root bunfig.toml pins bun's test root to
# packages/, and the root tsconfig.json excludes transpile/.

set -uo pipefail

cd "$(dirname "$0")/.." || exit 2
root=$(pwd)

if [ $# -gt 0 ]; then
  packages=("$@")
else
  packages=()
  for dir in packages/*/; do
    packages+=("$(basename "$dir")")
  done
fi

# Count lines like "  12 pass" from bun's summary; the last one wins, since a
# test's own stdout can print anything. Note the optional plural: bun writes
# " 1 error" but " 53 errors", and a gate that only matched the singular would
# miss precisely the runs it was built to catch.
count_of() {
  local word=$1 text=$2
  echo "$text" | grep -Eo "^[[:space:]]*[0-9]+ ${word}s?\$" | tail -1 | grep -Eo '[0-9]+' || echo 0
}

has_script() { grep -q "\"$2\"" "$1/package.json" 2>/dev/null; }

row_format='%-30s %6s %6s %6s %6s %6s %6s  %s\n'
rule() { printf '%.0s-' {1..104}; printf '\n'; }

# shellcheck disable=SC2059
printf "$row_format" PACKAGE PASS FAIL ERROR SKIP FILES TYPES STATUS
rule

total_pass=0 total_fail=0 total_error=0 total_skip=0
types_red=0 types_checked=0
bad=0

for pkg in "${packages[@]}"; do
  dir="$root/packages/$pkg"
  if [ ! -d "$dir" ]; then
    printf "$row_format" "$pkg" - - - - - - "NO SUCH PACKAGE"
    bad=1
    continue
  fi

  problems=""
  note=""

  # --- TypeScript ---------------------------------------------------------
  types=-
  types_output=""
  if has_script "$dir" typecheck; then
    types_output=$(cd "$dir" && bun run typecheck 2>&1)
    types_status=$?
    types=$(echo "$types_output" | grep -c 'error TS')
    types_checked=$((types_checked + 1))
    if [ "$types" -gt 0 ]; then
      problems="$problems TYPES"
      types_red=$((types_red + 1))
    elif [ "$types_status" -ne 0 ]; then
      problems="$problems TYPECHECK CRASHED"
    fi
  fi

  # --- bun test -----------------------------------------------------------
  pass=- fail=- error=- skip=- files=-
  output=""
  if ! has_script "$dir" test; then
    note="no test script"
  else
    # Run from the repository root, not from inside the package: bun reads
    # bunfig.toml from the current directory only, so a preload or any other
    # [test] setting declared at the root applies to every package only if the
    # root is where bun starts. The path argument is the filter.
    output=$(bun test "packages/$pkg" 2>&1)
    status=$?
    if echo "$output" | grep -qE "error: (No tests found|0 test files matching)|The following filters did not match any test files"; then
      note="no test files"
      pass=0 fail=0 error=0 skip=0 files=0
      output=""
    else
      pass=$(count_of pass "$output")
      fail=$(count_of fail "$output")
      error=$(count_of error "$output")
      skip=$(count_of skip "$output")
      files=$(echo "$output" | grep -Eo 'across [0-9]+ files?' | tail -1 | grep -Eo '[0-9]+' || echo 0)
      total_pass=$((total_pass + pass))
      total_fail=$((total_fail + fail))
      total_error=$((total_error + error))
      total_skip=$((total_skip + skip))

      if ! echo "$output" | grep -q "Ran [0-9]* tests\? across"; then
        problems="$problems CRASHED(exit $status, no summary)"
      else
        [ "$fail" -gt 0 ] && problems="$problems FAIL"
        [ "$error" -gt 0 ] && problems="$problems ERROR(files stopped early)"
      fi
    fi
  fi

  if [ -n "$problems" ]; then
    verdict="${problems# }"
    bad=1
  elif [ -n "$note" ]; then
    verdict="$note"
  else
    verdict=ok
  fi

  printf "$row_format" "$pkg" "$pass" "$fail" "$error" "$skip" "$files" "$types" "$verdict"

  # A package that failed prints its own output, so the table is a summary and
  # not a dead end.
  if [ -n "$problems" ]; then
    [ -n "$output" ] && echo "$output" | sed "s/^/    [$pkg test] /"
    [ "$types" != "-" ] && [ "$types" != "0" ] && echo "$types_output" | sed "s/^/    [$pkg types] /"
  fi
done

rule
# No TYPES total: a package typechecks its workspace dependencies' sources too,
# so core's errors are counted again in every package that imports core. Summing
# the column would produce exactly the kind of number this gate exists to refuse.
printf "$row_format" TOTAL "$total_pass" "$total_fail" "$total_error" "$total_skip" "" - ""
echo
echo "typecheck: $types_red of $types_checked packages red (per-package counts overlap; they do not sum)."

if [ "$bad" -ne 0 ]; then
  echo
  echo "GATE FAILED: a package reported failures, errors, type errors, or no summary."
  echo "An error count above zero means bun stopped reading a file partway, so the"
  echo "pass count on that row is a floor, not a total."
  exit 1
fi

echo
echo "GATE PASSED: no failures, no errors, no type errors."
