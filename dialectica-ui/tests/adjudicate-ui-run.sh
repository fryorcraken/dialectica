#!/usr/bin/env sh
# Decide whether a sitometres run proved what its spec asks.
#
#     dialectica-ui/tests/adjudicate-ui-run.sh <report.json> <spec.yaml>
#
# THE EXIT CODE IS NOT THE GATE, which is the whole reason this exists.
# sitometres exits 0 on INCONCLUSIVE by design, so `--strict` is passed at the
# call site to close that. Even with it, the exit code cannot see a run that
# started the application, executed nothing further, and reported a clean sheet.
#
# So three conditions are read off the machine report, and all three must hold:
#
#   1. the report's overall verdict is a pass;
#   2. every step in the report is a pass;
#   3. the number of steps in the report equals the number in the spec.
#
# The THIRD carries the weight. Drop it and an empty run satisfies the other
# two and reads as a pass, which is worse than a red run: it reads as evidence
# when it is the absence of evidence. The e2e-ui-suite change's design.md D1
# records which checks in tst_adjudicate_ui_run.sh go red without it, measured.
#
# The report is the authority rather than the terminal summary because the
# summary is styled for a human, ANSI-coloured with no stable field to match
# on, while the report is written from a `finally`, on every exit path.
#
# The report is JSON and is read with `jq`. The spec is YAML and is read with
# `yq`, the jq wrapper, so both are queried in one language. The e2e-ui-suite
# change's design.md D12.
#
# EVERY failing condition is reported before exiting, not just the first: a run
# that fails two of them should say so once rather than over two CI runs.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
. "$here/require-jq-yq.sh"

if [ $# -ne 2 ]; then
    echo "usage: $0 <report.json> <spec.yaml>" >&2
    exit 2
fi
report=$1
spec=$2

# sitometres writes the report from a `finally`, so it is missing when the
# process never reached its exit (killed by a job timeout, or OOM) AND when it
# never started: ui-tests.yml runs this on `always()`, so an earlier step that
# stopped the job lands here too. This script cannot tell the two apart, so it
# names both rather than guessing, and points at the step log that can. Either
# way it says so rather than passing on jq's error about a missing file:
# "nothing was proved" is a diagnosis; "a file is absent" is a puzzle.
if [ ! -e "$report" ]; then
    echo "::error::no JSON report, so nothing was proved — sitometres either never started (an earlier step stopped the job: see its error above) or was killed before it could write one (a job timeout, or OOM)"
    exit 1
fi

require_jq_yq || exit 1

problems=""
problem() {
    problems="$problems::error::$1
"
}

# A spec whose `steps:` is absent or not a list is refused rather than counted:
# jq's `null | length` is 0, which an empty run would match, and a number's
# `length` is its absolute value. A spec yq cannot parse is refused the same
# way, with yq's own error left on stderr above. All three are a problem like
# any other, so the conditions below are still checked and reported; under
# `set -e` a bare `expected=$(…)` would end the script on yq's exit code instead.
if ! expected=$(yq '.steps | if type == "array" then length else null end' "$spec"); then
    expected=null
fi
count=$(jq '.steps // [] | length' "$report")

echo "verdict: $(jq -r '.verdict' "$report")"
jq -r '(.steps // [])[] | (.verdict | tostring) as $v
       | "  [\((" " * (12 - ($v | length))) // "")\($v)] \(.name)"' "$report"

if [ "$(jq '.verdict == "pass"' "$report")" != true ]; then
    problem "verdict is '$(jq -r '.verdict' "$report")', expected 'pass'"
fi
if [ "$expected" = null ]; then
    problem "$spec has no steps: list that yq could read, so the report cannot be counted against it"
else
    # Written as "equal, or else a problem" so that it fails CLOSED: `[`
    # returns 2, not 1, when either side is not an integer, and `-ne` in an
    # `if` would read that error as "not unequal" and pass the run. Measured,
    # with the yq guard above disabled and a YAML-answering yq on PATH: `-ne`
    # printed `ok: all 2 steps passed`.
    [ "$count" -eq "$expected" ] ||
        problem "report has $count steps, spec has $expected — the run did not execute the whole spec"
fi
failed=$(jq -r '[(.steps // [])[] | select(.verdict != "pass") | .name | tojson] | join(", ")' "$report")
if [ -n "$failed" ]; then
    problem "steps that did not pass: $failed"
fi

if [ -n "$problems" ]; then
    printf '%s' "$problems"
    exit 1
fi
echo "ok: all $count steps passed"
