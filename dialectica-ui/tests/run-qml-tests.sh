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
    echo "--- $(basename "$spec")"
    "$runner" -input "$spec" -import "$qml_dir" || status=1
done

if [ "$count" -eq 0 ]; then
    no_runner "no tst_*.qml specs found in $here"
fi

echo "qml tests: ran $count spec file(s)"
exit "$status"
