#!/usr/bin/env bash
# Guard for deep-dive/src/ub_zoo.rs: prove that the zoo really is UB.
#
# `cargo test` never runs the #[ignore]d `ub_*` tests, and the `deep-dive-miri`
# CI job only runs the non-ignored ones (the `fixed_*` twins), so neither would
# notice if a `ub_*` test quietly stopped committing its UB. This script runs
# each `ub_*` test alone under Miri, once per aliasing model, and checks the
# outcome against the `// miri-expect` line(s) written above the test:
#
#   // miri-expect: <ERE>       both models: Miri fails and its output matches
#   // miri-expect(sb): <ERE>   the same, Stacked Borrows only
#   // miri-expect(tb): <ERE>   the same, Tree Borrows only
#   an <ERE> of PASS            Miri reports nothing: the test passes
#
# It also fails when the compiled crate has no ignored `ub_*` test, when a
# `ub_*` test is not ignored or has no expectation for a model, or when an
# expectation is not attached to a `ub_*` test.
#
# CI installs the newest nightly that ships Miri, so a std or Miri change can
# reword a diagnostic (between nightly 2026-07-23 and 2026-09-26, std started
# running get_unchecked's precondition check under Miri). A failure prints
# Miri's actual error lines: if the test is still UB, widen its ERE, e.g.
# `old wording|new wording`, so both toolchains pass.
#
# Usage, from any directory:  bash deep-dive/scripts/check_ub_zoo.sh
# Needs a nightly toolchain with the miri component (override the toolchain
# name with MIRI_TOOLCHAIN). Both runs use -Zmiri-strict-provenance, like CI.
set -euo pipefail

cd "$(dirname "$0")/.."
src=src/ub_zoo.rs
toolchain=${MIRI_TOOLCHAIN:-nightly}
strict=-Zmiri-strict-provenance
sep=$'\037'

# 1. The ignored `ub_*` tests that the compiled crate really contains.
if ! list_out=$(MIRIFLAGS=$strict cargo "+$toolchain" miri test --quiet --lib \
  -- --list --ignored </dev/null 2>&1); then
  printf '%s\n' "$list_out"
  echo "error: could not list the ignored tests under Miri" >&2
  exit 1
fi
listed=$(printf '%s\n' "$list_out" |
  sed -n 's/^ub_zoo::tests::\(ub_[a-z0-9_]*\): test$/\1/p' | sort)
if [ -z "$listed" ]; then
  echo "error: found no ignored ub_zoo::tests::ub_* test to check" >&2
  exit 1
fi

# 2. The expectations in the source, one "name<US>sb<US>tb" line per test.
# The separator is the ASCII unit separator (\037), not a tab: `read` merges
# runs of IFS whitespace, so an empty field between two tabs would vanish.
expectations=$(awk '
  /^[ \t]*\/\/ miri-expect/ {
    line = $0
    sub(/^[ \t]*\/\/ miri-expect/, "", line)
    if (line ~ /^\(sb\):/) { model = "sb"; sub(/^\(sb\):/, "", line) }
    else if (line ~ /^\(tb\):/) { model = "tb"; sub(/^\(tb\):/, "", line) }
    else if (line ~ /^:/) { model = "both"; sub(/^:/, "", line) }
    else {
      print src ":" NR ": malformed miri-expect line" > "/dev/stderr"
      bad = 1
      next
    }
    sub(/^[ \t]+/, "", line)
    expect[model] = line
    pending = 1
    next
  }
  /^[ \t]*fn [A-Za-z0-9_]+[(<]/ {
    name = $0
    sub(/^[ \t]*fn /, "", name)
    sub(/[(<].*/, "", name)
    if (name ~ /^ub_/) {
      sb = ("sb" in expect) ? expect["sb"] : expect["both"]
      tb = ("tb" in expect) ? expect["tb"] : expect["both"]
      print name "\037" sb "\037" tb
    } else if (pending) {
      print src ":" NR ": miri-expect above fn " name \
        ", which is not a ub_* test" > "/dev/stderr"
      bad = 1
    }
    delete expect
    pending = 0
  }
  END { exit bad }
' src="$src" "$src")

parsed=$(printf '%s\n' "$expectations" | cut -d "$sep" -f1 | sort)
if [ "$listed" != "$parsed" ]; then
  echo "error: the ignored ub_* tests and the ub_* tests in $src differ." >&2
  echo "Every ub_* test must be #[ignore]d." >&2
  echo "(< only among the ignored tests, > only in the source)" >&2
  diff <(printf '%s\n' "$listed") <(printf '%s\n' "$parsed") >&2 || true
  exit 1
fi

# 3. Run each test alone, once per model, and compare with its expectation.
failures=0
checks=0
while IFS=$sep read -r name sb tb; do
  for model in sb tb; do
    if [ "$model" = sb ]; then
      expect=$sb
      flags=$strict
    else
      expect=$tb
      flags="-Zmiri-tree-borrows $strict"
    fi
    checks=$((checks + 1))
    if [ -z "$expect" ]; then
      echo "FAIL  $name [$model]: no miri-expect line covers this model"
      failures=$((failures + 1))
      continue
    fi
    rc=0
    out=$(MIRIFLAGS=$flags cargo "+$toolchain" miri test --lib -- --ignored \
      --exact --test-threads=1 "ub_zoo::tests::$name" </dev/null 2>&1) || rc=$?
    verdict=ok
    if ! grep -qF "test ub_zoo::tests::$name ... " <<<"$out"; then
      verdict="the test did not start"
    elif [ "$expect" = PASS ]; then
      if [ "$rc" -ne 0 ] ||
        ! grep -qF "test result: ok. 1 passed" <<<"$out"; then
        verdict="expected Miri to pass it, but the run failed"
      fi
    elif [ "$rc" -eq 0 ]; then
      verdict="expected Miri to reject it, but the run passed"
    elif ! grep -qE -- "$expect" <<<"$out"; then
      verdict="Miri failed without the expected diagnostic"
    fi
    if [ "$verdict" = ok ]; then
      echo "ok    $name [$model]: ${expect}"
    else
      echo "FAIL  $name [$model]: $verdict"
      echo "      expected: $expect"
      printf '%s\n' "$out" | grep -E '^(error|test |unsafe precondition)' |
        sed 's/^/      | /' || true
      failures=$((failures + 1))
    fi
  done
done <<<"$expectations"

echo
count=$(printf '%s\n' "$parsed" | wc -l | tr -d ' ')
echo "$count ub_* tests, $checks checks, $failures failed"
[ "$failures" -eq 0 ]
