#!/usr/bin/env sh
# Tests for `check_every_test_ran`, the part of run-qml-tests.sh that fails a
# run in which a DECLARED test function never executed.
#
# WHY THIS FILE EXISTS. The check it guards exists because a QML suite can lose
# a test silently and stay green, and a check for that has the same failure mode
# as the thing it checks: narrowed until it matches nothing, it reports clean on
# every run. So both halves are pinned — what it MUST catch and what it MUST NOT
# — plus the end-to-end case, because the corpus it reads is written by the
# runner and a corpus that goes empty makes the check vacuous.
#
# THE DEFECT IT GUARDS, reproduced rather than described. QtTest treats
# `test_foo_data()` as the DATA PROVIDER for `test_foo()`, not as a test. So a
# pair like
#
#     function test_the_placeholder()      { verify(true) }
#     function test_the_placeholder_data() { /* assertions */ }
#
# takes BOTH functions out of the run — the `_data` body is called as a
# provider, returns undefined, and the other is then skipped for want of rows.
# Measured on Qt 6.10.3: `3 passed, 0 failed`, with the only trace a `WARNING:
# ... no data supplied` line, which is not a failure.
#
# Run directly: dialectica-ui/tests/tst_check_every_test_ran.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Extract the function by line range rather than sourcing the runner, which
# would execute the suite. Fails loudly if it is renamed or moved.
sed -n '/^check_every_test_ran() {$/,/^}$/p' "$here/run-qml-tests.sh" > "$work/fn.sh"
if ! grep -q '^check_every_test_ran() {$' "$work/fn.sh"; then
    echo "FAIL: could not extract check_every_test_ran from run-qml-tests.sh" >&2
    echo "      (was it renamed? this test guards it and must be updated with it)" >&2
    exit 1
fi
. "$work/fn.sh"

fails=0

assert_catches() {
    if check_every_test_ran "$2" "$3" >/dev/null 2>&1; then
        echo "FAIL: $1 — passed a run it must reject" >&2
        fails=$((fails + 1))
    else
        echo "ok: $1 — rejected"
    fi
}
assert_passes() {
    if check_every_test_ran "$2" "$3" >/dev/null 2>&1; then
        echo "ok: $1 — accepted"
    else
        echo "FAIL: $1 — rejected a run it must accept" >&2
        fails=$((fails + 1))
    fi
}

# ---- fixtures -----------------------------------------------------------
#
# Output lines are VERBATIM qmltestrunner format. Paraphrasing them would prove
# the regex matches the paraphrase and nothing about what Qt prints — in
# particular the `qmltestrunner::<Case>::` prefix, which contains colons and
# broke an earlier version of the name extraction.

cat > "$work/spec_ok.qml" <<'SPEC'
import QtQuick
import QtTest
TestCase {
    name: "Ok"
    function test_alpha() { verify(true) }
    function test_beta() { verify(true) }
}
SPEC

cat > "$work/out_ok" <<'OUT'
PASS   : qmltestrunner::Ok::initTestCase()
PASS   : qmltestrunner::Ok::test_alpha()
PASS   : qmltestrunner::Ok::test_beta()
PASS   : qmltestrunner::Ok::cleanupTestCase()
Totals: 4 passed, 0 failed, 0 skipped, 0 blacklisted, 2ms
OUT

assert_passes "every declared test ran" "$work/out_ok" "$work/spec_ok.qml"

# A FAILING test still RAN. The check must not confuse the two: it asks whether
# a test executed, never whether it passed, and a version that only accepted
# PASS lines would fire on every red suite — noise on exactly the runs someone
# is already reading.
cat > "$work/out_failed" <<'OUT'
PASS   : qmltestrunner::Ok::initTestCase()
FAIL!  : qmltestrunner::Ok::test_alpha() 'something' returned FALSE. ()
   Loc: [/x/spec_ok.qml(5)]
PASS   : qmltestrunner::Ok::test_beta()
PASS   : qmltestrunner::Ok::cleanupTestCase()
Totals: 3 passed, 1 failed, 0 skipped, 0 blacklisted, 2ms
OUT

assert_passes "a failing test counts as having run" "$work/out_failed" "$work/spec_ok.qml"

# ---- MUST CATCH ---------------------------------------------------------

cat > "$work/out_missing" <<'OUT'
PASS   : qmltestrunner::Ok::initTestCase()
PASS   : qmltestrunner::Ok::test_alpha()
PASS   : qmltestrunner::Ok::cleanupTestCase()
Totals: 3 passed, 0 failed, 0 skipped, 0 blacklisted, 2ms
OUT

assert_catches "a declared test absent from the run" "$work/out_missing" "$work/spec_ok.qml"

# The `_data` shape, with the warning line Qt actually emits. The warning names
# BOTH functions, which is why the check reads result lines only — counting any
# `::name()` would let the skipped half read as having run. Measured: with
# warnings counted, only `test_the_placeholder_data` was reported missing.
cat > "$work/spec_data.qml" <<'SPEC'
import QtQuick
import QtTest
TestCase {
    name: "Shadow"
    function test_the_placeholder() { verify(true) }
    function test_the_placeholder_data() { verify(true) }
    function test_zz_sentinel() { verify(true) }
}
SPEC

cat > "$work/out_data" <<'OUT'
PASS   : qmltestrunner::Shadow::initTestCase()
WARNING: qmltestrunner::Shadow::test_the_placeholder() no data supplied for test_the_placeholder() by test_the_placeholder_data()
   Loc: [qrc:/qt-project.org/imports/QtTest/TestSchedule.qml(24)]
PASS   : qmltestrunner::Shadow::test_zz_sentinel()
PASS   : qmltestrunner::Shadow::cleanupTestCase()
Totals: 3 passed, 0 failed, 0 skipped, 0 blacklisted, 2ms
OUT

assert_catches "the _data provider shape takes two tests out" "$work/out_data" "$work/spec_data.qml"

# Both names must be reported, not just one. A check that named only the `_data`
# half would send a reader looking at the wrong function — and it is the OTHER
# one that holds the assertions someone expected to run.
named=$(check_every_test_ran "$work/out_data" "$work/spec_data.qml" 2>&1 || true)
for expect in test_the_placeholder test_the_placeholder_data; do
    if printf '%s\n' "$named" | grep -q "$expect"; then
        echo "ok: the report names $expect"
    else
        echo "FAIL: the report does not name $expect" >&2
        fails=$((fails + 1))
    fi
done

# ---- the corpus-builder must be able to fail ----------------------------
#
# The check reads two derived lists, and either going empty makes it vacuous in
# a different direction. Both are exercised, because they fail opposite ways: an
# empty DECLARED list makes it accept everything, an empty RAN list makes it
# reject everything. A version broken either way is useless, and only one of the
# two looks broken from the outside.

cat > "$work/spec_empty.qml" <<'SPEC'
import QtQuick
import QtTest
TestCase { name: "Empty" }
SPEC

assert_passes "a spec declaring no tests" "$work/out_ok" "$work/spec_empty.qml"

cat > "$work/out_empty" <<'OUT'
********* Start testing of qmltestrunner *********
OUT

assert_catches "a run that produced no result lines at all" \
    "$work/out_empty" "$work/spec_ok.qml"

# ---- end to end, through the real runner --------------------------------
#
# The fixtures above prove the regexes. This proves the check is WIRED IN and
# reading the corpus the runner writes — the half a fixture cannot see, and the
# half that broke first: the path was passed as a basename and the grep could
# not open the file, so the check printed an error instead of a verdict.

e2e=$(mktemp -d)
trap 'rm -rf "$work" "$e2e"' EXIT

cat > "$e2e/tst_e2e_sound.qml" <<'SPEC'
import QtQuick
import QtTest
TestCase {
    name: "E2ESound"
    function test_one() { verify(true) }
    function test_two() { verify(true) }
}
SPEC

if "$here/run-qml-tests.sh" "$e2e/tst_e2e_sound.qml" >/dev/null 2>&1; then
    echo "ok: the real runner over a spec losing no test — accepted"
else
    echo "FAIL: the real runner rejected a sound spec — the end-to-end case" >&2
    echo "      cannot detect anything if it is red on correct input" >&2
    fails=$((fails + 1))
fi

cat > "$e2e/tst_e2e_vanish.qml" <<'SPEC'
import QtQuick
import QtTest
TestCase {
    name: "E2EVanish"
    function test_the_placeholder() { verify(true) }
    // Consumed as the data provider for the function above. Neither runs.
    function test_the_placeholder_data() { verify(true) }
}
SPEC

if "$here/run-qml-tests.sh" "$e2e/tst_e2e_vanish.qml" >/dev/null 2>&1; then
    echo "FAIL: the real runner EXITED 0 over a spec that lost two tests." >&2
    echo "      check_every_test_ran is not wired in, or is reading an empty" >&2
    echo "      corpus — look at how \$out and \$spec_path are passed in" >&2
    echo "      run-qml-tests.sh's run_spec." >&2
    fails=$((fails + 1))
else
    echo "ok: the real runner over a spec that lost two tests — rejected"
fi

if [ "$fails" -ne 0 ]; then
    echo "check_every_test_ran: $fails case(s) failed" >&2
    exit 1
fi
echo "check_every_test_ran: all cases passed"
