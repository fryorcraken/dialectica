#!/usr/bin/env sh
# Pin what the adjudicator must catch AND what it must not.
#
# A check narrowed to nothing passes as quietly as a correct one, so every case
# below that expects a FAILURE is paired with the passing case it is derived
# from. Each fixture differs from the green one in exactly the property under
# test, so a pass here cannot come from the fixture being broken in some other
# way.
#
# The REAL adjudicator is run, as a separate process, over files on disk: the
# same shape ui-tests.yml invokes it in.
#
# Run: dialectica-ui/tests/tst_adjudicate_ui_run.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
adjudicator=$here/adjudicate-ui-run.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

failures=0
ok() { echo "  ok: $1"; }
bad() {
    echo "  FAIL: $1 — $2"
    failures=$((failures + 1))
}

# write_spec <n | yaml>: a spec with n steps, or, when the argument is not a
# number, that text verbatim. Written as real YAML rather than a count, so the
# number the adjudicator reads is the length of a document that actually
# parses.
write_spec() {
    case $1 in
        '' | *[!0-9]*)
            printf '%s\n' "$1" > "$work/spec.yaml"
            return
            ;;
    esac
    printf 'steps:\n' > "$work/spec.yaml"
    i=0
    while [ "$i" -lt "$1" ]; do
        printf '  - name: step %s\n    click: something\n' "$i" >> "$work/spec.yaml"
        i=$((i + 1))
    done
}

# adjudicate <report-json | none> <spec-steps | spec-yaml> [PATH]: runs the
# adjudicator and sets $code and $out. `none` writes no report at all.
adjudicate() {
    write_spec "$2"
    rm -f "$work/report.json"
    if [ "$1" != none ]; then
        printf '%s\n' "$1" > "$work/report.json"
    fi
    if out=$(PATH=${3:-$PATH} sh "$adjudicator" "$work/report.json" "$work/spec.yaml" 2>&1); then
        code=0
    else
        code=$?
    fi
}

expect_exit() {
    if [ "$code" -eq "$1" ]; then ok "exit $1"; else bad "exit $1" "got $code: $out"; fi
}
expect_says() {
    case $out in
        *"$2"*) ok "$1" ;;
        *) bad "$1" "$out" ;;
    esac
}
expect_not_says() {
    case $out in
        *"$2"*) bad "$1" "$out" ;;
        *) ok "$1" ;;
    esac
}

echo "a report that satisfies all three conditions passes"
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"}]}' 2
expect_exit 0
expect_says "says so" "all 2 steps passed"

echo "a run that stopped early fails, even with nothing failing"
# The ONLY difference from the green fixture: the spec declares three steps and
# the report carries two. The verdict is a pass and no step failed, so the first
# two conditions hold. This is the case condition 3 exists for.
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"}]}' 3
expect_exit 1
expect_says "names the cause" "did not execute the whole spec"

echo "a report with MORE steps than the spec fails too, not just fewer"
# The comparison is `-eq`, not `-le`: a phantom or double-logged step is the
# same "the report and the spec disagree on what ran" problem as a run that
# stopped early. Spec declares two, report carries three.
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"},{"name":"c","verdict":"pass"}]}' 2
expect_exit 1
expect_says "names the cause" "did not execute the whole spec"

echo "a failed step inside a passing report fails, and is named"
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"fail"}]}' 2
expect_exit 1
expect_says "names the step" 'steps that did not pass: "b"'

echo "an inconclusive verdict fails"
adjudicate '{"verdict":"inconclusive","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"}]}' 2
expect_exit 1
expect_says "names the verdict" "verdict is 'inconclusive'"

echo "every failing condition is reported, not just the first"
# Three conditions broken at once: the verdict, a failed step, and a short count.
adjudicate '{"verdict":"fail","steps":[{"name":"a","verdict":"fail"}]}' 2
expect_exit 1
expect_says "reports the verdict" "expected 'pass'"
expect_says "reports the count" "did not execute the whole spec"
expect_says "reports the step" "steps that did not pass"

# NO SPEC: none of the three adjudication conditions covers a report that does
# not exist. This pins the chosen behaviour: exit 1 with a message saying
# nothing was proved, rather than jq's own missing-file error, and naming BOTH
# ways a report goes missing rather than guessing one. ui-tests.yml runs this
# step on `always()`, so it also runs when an earlier step stopped the job and
# sitometres never started. The e2e-ui-suite change's design.md D1 chose the
# message; the e2e-suite-review change's design.md D6 corrected its cause.
echo "a missing report says nothing was proved, not that a file is absent"
adjudicate none 2
expect_exit 1
expect_says "names the real cause" "nothing was proved"
expect_not_says "is not jq's missing-file error" "Could not open"
expect_says "names a run that never started" "never started"
expect_says "names a run that was killed" "was killed"

# NO SPEC: the three conditions presume the spec has a steps: list to count
# the report against. A spec without one is a problem reported alongside the
# others, not an abort on jq's own error and exit code. The e2e-suite-review
# change's design.md D7.
echo "a spec with no steps: list is reported, alongside every other problem"
adjudicate '{"verdict":"fail","steps":[{"name":"a","verdict":"fail"}]}' 'app: dialectica_ui'
expect_exit 1
expect_says "names the cause" "no steps: list"
expect_says "still reports the verdict" "expected 'pass'"
expect_says "still reports the step" "steps that did not pass"

echo "a steps: value that is not a list is refused, not counted"
# jq's `length` of a number is its absolute value, so without the type check
# `steps: 2` would be counted as two and match this two-step green report.
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"}]}' 'steps: 2'
expect_exit 1
expect_says "names the cause" "no steps: list"
expect_not_says "does not pass" "steps passed"

echo "a spec that does not parse is reported, alongside every other problem"
adjudicate '{"verdict":"fail","steps":[{"name":"a","verdict":"fail"}]}' 'steps: ['
expect_exit 1
expect_says "names the cause" "no steps: list"
expect_says "still reports the verdict" "expected 'pass'"

# The other half of this pair is the first case above, which runs the real
# `yq` and passes. Here a `yq` that answers in YAML, as the Go yq on GitHub's
# runner image does, is put first on PATH. The e2e-ui-suite change's design.md
# D12.
echo "a yq that is not the jq wrapper is refused by name"
mkdir -p "$work/other-yq"
cat > "$work/other-yq/yq" <<'EOF'
#!/bin/sh
# Stands in for mikefarah/yq: the same command name, answering in YAML.
printf 'steps:\n  - name: a\n'
EOF
chmod +x "$work/other-yq/yq"
adjudicate '{"verdict":"pass","steps":[{"name":"a","verdict":"pass"},{"name":"b","verdict":"pass"}]}' 2 "$work/other-yq:$PATH"
expect_exit 1
expect_says "names the cause" "is not the jq-wrapper yq"
expect_not_says "does not pass" "steps passed"

echo
if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "ok: the adjudicator catches each condition and passes a clean run"
