#!/usr/bin/env sh
# Run the QML component tests.
#
# Ported from radicle-ui/tests/run-qml-tests.sh, which the `qml:` job in
# .github/workflows/ci.yml names as the machinery to copy. What is copied is the
# runner DISCOVERY and the smoke test, and neither is boilerplate:
#
# A `qmltestrunner` on PATH may well be the Qt5 one (Fedora's
# qt5-qtdeclarative-devel ships it as /usr/bin/qmltestrunner), and it fails
# against Qt6 QML with an empty error and a bare exit 1 — which reads exactly
# like "the tests are broken". So this prefers an explicitly Qt6 binary and
# falls back to a bare `qmltestrunner` only after PROVING it can run a trivial
# test. Without that proof a wrong-major-version runner is indistinguishable
# from a failing suite, and the natural response to the latter is to go looking
# in the tests.
#
# Set REQUIRE_QML_TESTS=1 (CI does) to fail rather than skip when no usable
# runner is found. Skipping quietly in CI is a false green: the job passes while
# testing nothing, which PLAN.md §10 calls worse than no gate at all.
set -eu

# Headless by default, set once here rather than prefixed onto every
# invocation: the tests never need a display, and a caller should not have to
# remember the variable.
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-offscreen}"

here=$(cd "$(dirname "$0")" && pwd)
qml_dir="$here/../src/qml"
require=${REQUIRE_QML_TESTS:-0}

find_runner() {
    # An explicit override wins, so a developer with a Qt6 build in an unusual
    # place does not have to edit this list.
    if [ -n "${QMLTESTRUNNER:-}" ]; then
        printf '%s\n' "$QMLTESTRUNNER"
        return 0
    fi
    # Qt6-specific names and locations first. The bare name is deliberately
    # NOT in this loop — see the fallback below.
    for cand in qmltestrunner6 \
                /usr/lib64/qt6/bin/qmltestrunner \
                /usr/lib/qt6/bin/qmltestrunner \
                /usr/lib/x86_64-linux-gnu/qt6/bin/qmltestrunner; do
        if command -v "$cand" >/dev/null 2>&1 || [ -x "$cand" ]; then
            printf '%s\n' "$cand"
            return 0
        fi
    done
    # Ambiguous fallback — verified by the smoke test below before use.
    if command -v qmltestrunner >/dev/null 2>&1; then
        printf '%s\n' qmltestrunner
        return 0
    fi
    return 1
}

# A QML binding that silently evaluates to `undefined` is NOT a test failure to
# qmltestrunner, and that is the blind spot this function closes.
#
# WHY IT IS NEEDED AT ALL. `qmltestrunner` reports a runtime error raised while
# instantiating a component as a QWARN and carries on: the spec still passes and
# the process still exits 0. Measured on Qt 6.10.3 — a stale `Theme.note` in
# MarginNote.qml (a component `tst_feed_states` instantiates and asserts nothing
# about) printed `ReferenceError: Theme is not defined` 35 times and the runner
# reported `12 passed, 0 failed`. Only an error raised inside a `compare()`
# fails a spec, which covers whatever the specs happen to read and nothing else.
#
# That is the exact defect class `piece/theme-unshadow` exists to prevent: the
# singleton was named `Theme`, basecamp registers that name, every token read
# `undefined`, and the screen rendered as unstyled defaults. The static gate in
# ci.yml catches the one spelling it knows (a bare `Theme`); this catches the
# runtime symptom whatever produced it.
#
# WHY NOT FAIL ON EVERY QWARN. Too blunt: an unrelated Qt deprecation notice or
# a platform warning would fail the suite for a reason that has nothing to do
# with the code under test, and a gate that cries wolf gets disabled. The two
# patterns below are pinned instead, each one meaning "a binding produced
# undefined" and nothing else.
#
# WHY BOTH PATTERNS, which is not obvious and was measured rather than assumed:
#
#   - `ReferenceError: X is not defined` — the name does not resolve at all.
#     This is the stale-singleton form (`Theme.note` after the rename) and the
#     typo'd-singleton form (`DThemeTypo.inkSoft`). Note the second is invisible
#     to ci.yml's gate, which looks for a bare `Theme`: a D-prefixed typo
#     contains none.
#   - `Unable to assign [undefined] to <T>` — the singleton resolves but the
#     property does not exist (`DTheme.noSuchToken`). This raises NO
#     ReferenceError whatsoever, so a check written only for the first pattern
#     passes a component whose colour is undefined at runtime. Measured: 12
#     passed, 0 failed, zero ReferenceErrors, and the binding is broken.
#
# The two families share NO common substring, which is why this is two patterns
# rather than one loose one: the first says "is not defined", the second says
# "[undefined]". Grepping the bare word `undefined` catches the second and
# MISSES the first — verified by mutation, not assumed — while also matching
# Qt's "undefined behaviour" warnings and several of this suite's own test
# names. Both failure directions are pinned in tst_check_bindings.sh.
#
# WHY NOT QT_FATAL_WARNINGS, which looks like the one-line answer. It aborts the
# process on the FIRST warning of any kind, so the run dies with a crash rather
# than a diagnosis, takes the remaining specs with it, and is blunt in exactly
# the way rejected above. qmltestrunner has no flag that escalates a warning to
# a failure — `-help` lists none — so inspecting its output is the mechanism
# available.
check_bindings() {
    out=$1
    spec=$2
    # `grep -c` counts matching LINES; 0 matches exits 1, which is why the
    # count is read rather than the exit status.
    n=$(grep -c -E 'ReferenceError: .* is not defined|Unable to assign \[undefined\]' "$out" || true)
    [ "$n" -eq 0 ] && return 0
    echo "qml tests: $spec — $n line(s) report a binding that evaluated to undefined." >&2
    echo "           These are QWARNs, so the spec above still says it passed." >&2
    echo "           A QML binding reading undefined renders as an unstyled default;" >&2
    echo "           this is the defect a singleton name collision produces." >&2
    grep -n -E 'ReferenceError: .* is not defined|Unable to assign \[undefined\]' "$out" >&2
    return 1
}

# Run ONE spec and check its output. The single place a spec is executed and
# captured, used by both the named-spec path and the whole-suite loop below.
#
# WHY IT IS A FUNCTION rather than the same four lines written twice. The
# capture is the corpus `check_bindings` reads, and a corpus that silently goes
# empty makes the check report clean over a genuinely broken binding — the
# failure `tst_check_bindings.sh`'s end-to-end case exists to catch. That case
# can only drive ONE path, so with two capture sites a mutation to the other is
# invisible: measured, `: > "$out"` in the suite loop left every case green
# while `: > "$out"` in the named-spec path failed one. One site is one thing
# for the test to cover.
#
# Both streams go to the file, because the diagnostics checked below go to
# stderr while the PASS/FAIL lines go to stdout, and the check must read the
# same text a human does. The file is echoed straight back out, so the CI log is
# unchanged from before this check existed.
run_spec() {
    spec=$1
    out=$2
    rc=0
    echo "--- $(basename "$spec")"
    if [ -n "${extra_import:-}" ]; then
        "$runner" -input "$spec" -import "$qml_dir" -import "$extra_import" \
            >"$out" 2>&1 || rc=1
    else
        "$runner" -input "$spec" -import "$qml_dir" >"$out" 2>&1 || rc=1
    fi
    cat "$out"
    check_bindings "$out" "$(basename "$spec")" || rc=1
    return "$rc"
}

no_runner() {
    if [ "$require" = "1" ]; then
        echo "qml tests: $1" >&2
        echo "           REQUIRE_QML_TESTS=1, so this is a failure rather than a skip." >&2
        exit 1
    fi
    echo "qml tests: $1 — skipping" >&2
    exit 0
}

runner=$(find_runner) || no_runner "no qmltestrunner found"

# Prove the runner works at all before trusting a pass OR a failure from it.
# This is the step that turns "exit 1 with no output" from a mystery into a
# named diagnosis.
smoke=$(mktemp -d)
trap 'rm -rf "$smoke"' EXIT
cat > "$smoke/tst_smoke.qml" <<'SMOKE'
import QtQuick
import QtTest
TestCase { name: "Smoke"; function test_ok() { compare(1 + 1, 2); } }
SMOKE

if ! "$runner" -input "$smoke/tst_smoke.qml" >/dev/null 2>&1; then
    no_runner "$runner cannot run even a trivial test (wrong Qt major version?)"
fi

echo "qml tests: using $runner"

# A named spec, when arguments are given: `run-qml-tests.sh <file.qml> …`.
#
# This exists so nobody has to invoke `qmltestrunner` directly for a one-off
# probe. Doing so means hand-writing `QT_QPA_PLATFORM=offscreen`, and an
# environment-variable prefix is a shape this repo's permission checker cannot
# analyse — every such call costs an approval click. It also loses the runner
# discovery above, which is the difference between a real failure and a Qt5
# binary exiting 1 with no output.
#
# `check_bindings` runs over a named spec too: a probe is exactly where an
# undefined binding is easiest to write and hardest to notice.
#
# `--import <dir>` before the spec paths adds an extra import root, which a
# probe staging its own named module needs. It is an option rather than an
# environment variable for the same reason as QT_QPA_PLATFORM above.
extra_import=""
while [ "$#" -gt 0 ]; do
    case $1 in
        --import) extra_import=$2; shift 2 ;;
        *) break ;;
    esac
done

if [ "$#" -gt 0 ]; then
    for spec in "$@"; do
        [ -e "$spec" ] || { echo "qml tests: no such spec: $spec" >&2; exit 1; }
    done
    status=0
    count=0
    for spec in "$@"; do
        count=$((count + 1))
        run_spec "$spec" "$smoke/arg.$count" || status=1
    done
    exit "$status"
fi

# Every tst_*.qml in this directory, so a new spec file is picked up without
# editing this script.
#
# NOTE ON THE SPEC-COUNT ASSERTION the CI comment block discusses: it belongs
# in the CI job rather than here, and it is only safe now that real specs
# exist. A spec-count check over zero specs passes loudest when there is
# nothing to run.
status=0
count=0
for spec in "$here"/tst_*.qml; do
    # A literal-glob guard: with no matches, `$spec` is the pattern itself.
    # Without this the loop would "run" one nonexistent file and the runner's
    # complaint would look like a test failure.
    [ -e "$spec" ] || break
    count=$((count + 1))
    # The `--- <spec>` header is printed by run_spec, not here: CI's
    # `every QML spec file actually ran` step counts those lines, so printing
    # one in both places doubles the count and reddens that step.
    run_spec "$spec" "$smoke/out.$count" || status=1
done

if [ "$count" -eq 0 ]; then
    no_runner "no tst_*.qml specs found in $here"
fi

echo "qml tests: ran $count spec file(s)"
exit "$status"
