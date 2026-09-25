#!/usr/bin/env sh
# Tests for ui-tests.yml's "lgs left scaffold.toml's values alone" guard.
#
# design.md D4 records that both `lgs basecamp setup` and `lgs basecamp install`
# rewrite `scaffold.toml` (CLAUDE.md), and that the guard exists because a
# changed VALUE, as opposed to a stripped comment, would mean the run exercises
# a Basecamp or module set the repository does not declare. A check nobody has
# watched go red is not yet known to be able to.
#
# WHAT THIS FILE CAN AND CANNOT CLOSE. Whether `lgs basecamp setup`/`install`
# themselves ever rewrite a VALUE is a fact about `lgs`, observable only by
# running it, which this repo's owner has restricted to `lgs basecamp launch`
# locally and to CI otherwise (see this piece's tasks.md and design.md Risks).
# This file does not attempt that; it cannot prove `lgs` never trips the guard.
#
# What it closes is the other half: whether the guard's OWN mechanism,
# `tomlq -S . scaffold.toml` snapshotted before and diffed after, tells a
# changed value from the comment-and-whitespace rewrite `lgs` is known to
# perform. That is a property of `tomlq -S` and `diff`, checkable without
# `lgs`, Nix or a network, and it is the property the guard is trusted to have.
#
# EXTRACTED, NOT REIMPLEMENTED. The two `run:` blocks are read out of
# `ui-tests.yml` itself by step name, with `yq`, not retyped here: the same
# discipline `tst_check_bindings.sh` applies to `check_bindings`. Retyping
# would test this file's idea of the guard rather than the guard, and would
# stay green through a change to the real steps. Renaming either step fails
# this file loudly instead.
#
# Run: dialectica-ui/tests/tst_scaffold_values_unchanged.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
workflow=$here/../../.github/workflows/ui-tests.yml
. "$here/require-jq-yq.sh"
require_jq_yq

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

BEFORE_STEP="Read scaffold.toml before lgs touches it"
AFTER_STEP="lgs left scaffold.toml's values alone"

# extract_step <name> <file>: the literal `run:` text of the one step with that
# name in the `spec` job. Exits rather than writing an empty file on a miss: a
# renamed step must fail this file loudly, not report a false green over a body
# that was never read.
extract_step() {
    n=$(yq --arg name "$1" '[.jobs.spec.steps[] | select(.name == $name)] | length' "$workflow")
    if [ "$n" -ne 1 ]; then
        echo "FAIL: $n steps named '$1' in $workflow, expected 1. Has it been renamed or removed?"
        echo "      this test extracts it by name and must be updated with it"
        exit 1
    fi
    yq -r --arg name "$1" '.jobs.spec.steps[] | select(.name == $name) | .run // ""' "$workflow" > "$2"
    if [ -z "$(tr -d '[:space:]' < "$2")" ]; then
        echo "FAIL: step '$1' in $workflow has no run: body"
        exit 1
    fi
}

extract_step "$BEFORE_STEP" "$work/before.sh"
extract_step "$AFTER_STEP" "$work/after.sh"

# A realistic fixture: a `[repos.basecamp].pin` that satisfies the BEFORE
# step's own "full 40-char lowercase sha" check, plus a second table and a
# comment, so a comment-and-reorder rewrite has something to remove and
# reorder. Values are otherwise arbitrary.
GOOD_TOML='# a comment lgs is documented to strip
[repos.basecamp]
pin = "aa237766baf61404e12da86b7303cb41065464c9"
attr = "app"

[repos.lgpm]
pin = "d3af2972f51d9c542537d80d60ad8d20282ddcd1"
attr = "cli"
'

# Same values, no comment, tables and keys reordered, a different quoting style
# for one string: the shape of rewrite `lgs basecamp setup`/`install` are
# documented to perform (CLAUDE.md: "any `lgs basecamp` verb rewrites the
# file"). Not one value differs from GOOD_TOML.
COSMETIC_REWRITE_TOML='[repos.lgpm]
attr = '"'cli'"'
pin = "d3af2972f51d9c542537d80d60ad8d20282ddcd1"

[repos.basecamp]
attr = "app"
pin = "aa237766baf61404e12da86b7303cb41065464c9"
'

# GOOD_TOML with one hex digit of one pin changed, and nothing else: the case
# the guard exists to catch. Derived rather than retyped, so it cannot differ
# from GOOD_TOML in a second, unnoticed way.
VALUE_CHANGED_TOML=$(printf '%s' "$GOOD_TOML" | sed 's/41065464c9"/41065464c0"/')
# `$(…)` drops the trailing newline, so the comparison drops it from both.
if [ "$VALUE_CHANGED_TOML" = "$(printf '%s' "$GOOD_TOML")" ]; then
    echo "FAIL: the value-changed fixture is identical to the good one; the sed above no longer matches"
    exit 1
fi

# run_step <script> <dir>: runs a step body in <dir> with the shell and flags
# Actions uses for `shell: bash` (`bash --noprofile --norc -eo pipefail {0}`),
# and the two variables the steps read. Sets $code and $out.
run_step() {
    if out=$(cd "$2" && RUNNER_TEMP="$2/runner_temp" GITHUB_ENV="$2/github_env" \
        bash --noprofile --norc -eo pipefail "$1" 2>&1); then
        code=0
    else
        code=$?
    fi
}

# scenario <before-toml> <after-toml>: BOTH real steps back to back, in a fresh
# directory. `before` is what the BEFORE step snapshots; `after` replaces it
# before the AFTER step. A run where `lgs` changed nothing is
# `scenario x x`.
scenario() {
    dir=$work/case
    rm -rf "$dir"
    mkdir -p "$dir/runner_temp"
    : > "$dir/github_env"
    printf '%s' "$1" > "$dir/scaffold.toml"
    run_step "$work/before.sh" "$dir"
    if [ "$code" -ne 0 ]; then
        echo "FAIL: the BEFORE step itself failed on the fixture:"
        echo "$out"
        exit 1
    fi
    printf '%s' "$2" > "$dir/scaffold.toml"
    run_step "$work/after.sh" "$dir"
}

failures=0
ok() { echo "  ok: $1"; }
bad() {
    echo "  FAIL: $1 — $2"
    failures=$((failures + 1))
}
expect_exit() {
    if [ "$code" -eq "$1" ]; then ok "exits $1"; else bad "exits $1" "got $code: $out"; fi
}
expect_says() {
    case $out in
        *"$2"*) ok "$1" ;;
        *) bad "$1" "$out" ;;
    esac
}

echo "scaffold.toml genuinely unchanged"
scenario "$GOOD_TOML" "$GOOD_TOML"
expect_exit 0
expect_says "says so" "ok: scaffold.toml's values are unchanged"

echo "a comment-and-reorder rewrite with every value identical (what lgs does)"
# The pairing case. Without it, a guard that fires on ANY textual change, which
# is what `git diff` would have done and was #120's mistake, would pass the
# case above for the wrong reason (an accidental byte-for-byte match) and this
# file would not know the difference.
scenario "$GOOD_TOML" "$COSMETIC_REWRITE_TOML"
expect_exit 0
expect_says "still says unchanged" "ok: scaffold.toml's values are unchanged"

echo "one value actually changed (one hex digit of a pin)"
scenario "$GOOD_TOML" "$VALUE_CHANGED_TOML"
expect_exit 1
expect_says "names the cause" "changed a value in scaffold.toml"
expect_says "shows the changed pin in the diff" "41065464c0"

echo
if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "ok: the guard's diff distinguishes a changed value from lgs's own rewrite"
