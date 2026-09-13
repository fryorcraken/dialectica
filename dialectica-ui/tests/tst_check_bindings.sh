#!/usr/bin/env sh
# Tests for `check_bindings`, the part of run-qml-tests.sh that turns a QML
# binding evaluating to `undefined` from a QWARN into a failed run.
#
# WHY THIS FILE EXISTS. `check_bindings` is a helper, and a helper is part of
# the measurement: a check narrowed until it matches nothing passes every run
# just as quietly as a correct one, and the suite it guards would go on
# reporting green. So both halves are pinned here — what it MUST catch, and
# what it MUST NOT — and the corpus is real `qmltestrunner` output copied from
# runs recorded in openspec/changes/theme-unshadow/, not prose invented to fit
# the regex.
#
# WHY THE FIXTURES ARE VERBATIM. An assertion over a corpus is only as strong
# as the corpus. If these strings were paraphrased, the test would prove the
# regex matches the paraphrase and nothing about what Qt actually prints. Each
# fixture below names the mutation that produced it so it can be regenerated.
#
# Run directly: dialectica-ui/tests/tst_check_bindings.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Pull `check_bindings` in without running the suite. The runner is written so
# that sourcing it up to the first side effect defines the helpers; rather than
# depend on that, the function is extracted by line range, which fails loudly if
# the function moves or is renamed.
sed -n '/^check_bindings() {$/,/^}$/p' "$here/run-qml-tests.sh" > "$work/fn.sh"
if ! grep -q '^check_bindings() {$' "$work/fn.sh"; then
    echo "FAIL: could not extract check_bindings from run-qml-tests.sh" >&2
    echo "      (was it renamed? this test guards it and must be updated with it)" >&2
    exit 1
fi
. "$work/fn.sh"

fails=0

# assert_catches <name> <file>   — check_bindings must return non-zero
# assert_passes  <name> <file>   — check_bindings must return zero
assert_catches() {
    if check_bindings "$2" "fixture" >/dev/null 2>&1; then
        echo "FAIL: $1 — check_bindings passed output it must reject" >&2
        fails=$((fails + 1))
    else
        echo "ok: $1 — rejected"
    fi
}
assert_passes() {
    if check_bindings "$2" "fixture" >/dev/null 2>&1; then
        echo "ok: $1 — accepted"
    else
        echo "FAIL: $1 — check_bindings rejected output it must accept" >&2
        fails=$((fails + 1))
    fi
}

# ---- MUST CATCH ---------------------------------------------------------
#
# Case 1: a stale singleton reference. Produced by rewriting MarginNote.qml's
# `font: DTheme.note` as a two-line binding with `Theme.note` at column 0 —
# the mutation in findings/correctness.md. The real run printed this line 35
# times across tst_feed_states and still reported `12 passed, 0 failed`.
cat > "$work/stale.txt" <<'EOF'
PASS   : qmltestrunner::FeedStates::initTestCase()
QWARN  : qmltestrunner::FeedStates::test_the_two_states_are_distinguishable() file:///x/dialectica-ui/src/qml/MarginNote.qml:40: ReferenceError: Theme is not defined
PASS   : qmltestrunner::FeedStates::test_the_two_states_are_distinguishable()
Totals: 12 passed, 0 failed, 0 skipped, 0 blacklisted, 35ms
EOF
assert_catches "a stale Theme reference in an unasserted component" "$work/stale.txt"

# Case 2: a D-PREFIXED typo. `DThemeTypo.inkSoft` contains no bare `Theme`, so
# ci.yml's static gate cannot see it — measured, the gate exits 0 on this tree
# with the typo in place. The runtime symptom is identical, which is the whole
# argument for checking output as well as source.
cat > "$work/typo.txt" <<'EOF'
QWARN  : qmltestrunner::FeedStates::test_rows_are_taken_from_the_reply_in_order() file:///x/dialectica-ui/src/qml/MarginNote.qml:40: ReferenceError: DThemeTypo is not defined
PASS   : qmltestrunner::FeedStates::test_rows_are_taken_from_the_reply_in_order()
Totals: 12 passed, 0 failed, 0 skipped, 0 blacklisted, 85ms
EOF
assert_catches "a D-prefixed typo the static gate cannot see" "$work/typo.txt"

# Case 3: a missing token on a correctly-named singleton. `DTheme.noSuchToken`
# raises NO ReferenceError at all — Qt reports the failed assignment instead.
# A check written only for ReferenceError passes this, which is why the second
# pattern exists. Copied from a `-tap` run of the real mutation.
cat > "$work/undef.txt" <<'EOF'
ok 8 - FeedStates::test_rows_are_taken_from_the_reply_in_order()
  ---
  extensions:
    messages:
    - severity: warning
      message: file:///x/dialectica-ui/src/qml/MarginNote.qml:40:13: Unable to assign [undefined] to QColor
  ...
# tests 12
# pass 12
# fail 0
EOF
assert_catches "a missing token, which raises no ReferenceError" "$work/undef.txt"

# ---- MUST NOT CATCH -----------------------------------------------------
#
# Case 4: the real clean run. If this were rejected the check would be useless
# in the other direction — every suite run would fail and the gate would be
# turned off. This is the actual output of the suite on this tree.
cat > "$work/clean.txt" <<'EOF'
qml tests: using /usr/lib64/qt6/bin/qmltestrunner
--- tst_feed_states.qml
********* Start testing of qmltestrunner *********
Config: Using QtTest library 6.10.3, Qt 6.10.3 (x86_64-little_endian-lp64 shared (dynamic) release build; by GCC 15.2.1), fedora 43
PASS   : qmltestrunner::FeedStates::initTestCase()
PASS   : qmltestrunner::FeedStates::test_the_two_states_are_distinguishable()
PASS   : qmltestrunner::FeedStates::cleanupTestCase()
Totals: 12 passed, 0 failed, 0 skipped, 0 blacklisted, 38ms
********* Finished testing of qmltestrunner *********
qml tests: ran 4 spec file(s)
EOF
assert_passes "a clean run of the real suite" "$work/clean.txt"

# Case 5: an UNRELATED warning must not fail the run. This is the "fail on any
# QWARN is too blunt" decision, pinned so a later widening of the pattern
# breaks this test rather than silently making the gate cry wolf. If a genuine
# need to catch this class arises, that is a deliberate change to make here.
cat > "$work/unrelated.txt" <<'EOF'
QWARN  : qmltestrunner::FeedStates::test_rows_are_taken_from_the_reply_in_order() Detected anchors on an item that is managed by a layout. This is undefined behaviour; use Layout.alignment instead.
PASS   : qmltestrunner::FeedStates::test_rows_are_taken_from_the_reply_in_order()
Totals: 12 passed, 0 failed, 0 skipped, 0 blacklisted, 38ms
EOF
assert_passes "an unrelated Qt warning" "$work/unrelated.txt"

# Case 6: the WORD undefined in ordinary passing output. Several specs in this
# suite have test names containing "undefined" — for instance
# `test_has_more_defaults_to_false_rather_than_undefined`. A pattern matching
# the bare word would fail the suite on a spec whose name mentions it, which is
# a false positive waiting in the existing tests rather than a hypothetical.
cat > "$work/wordy.txt" <<'EOF'
PASS   : qmltestrunner::FeedStates::test_has_more_defaults_to_false_rather_than_undefined()
PASS   : qmltestrunner::SanitisedText::test_a_missing_value_renders_empty_rather_than_undefined()
Totals: 12 passed, 0 failed, 0 skipped, 0 blacklisted, 38ms
EOF
assert_passes "test names containing the word undefined" "$work/wordy.txt"

if [ "$fails" -ne 0 ]; then
    echo "check_bindings: $fails case(s) failed" >&2
    exit 1
fi
echo "check_bindings: all cases passed"
