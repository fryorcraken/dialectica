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
# when it is the absence of evidence. design.md D1 records which checks in
# tst_adjudicate_ui_run.sh go red without it, measured.
#
# The report is the authority rather than the terminal summary because the
# summary is styled for a human, ANSI-coloured with no stable field to match
# on, while the report is written from a `finally`, on every exit path.
#
# The report is JSON and is read with `jq`. The spec is YAML and is read with
# `yq`, the jq wrapper, so both are queried in one language. design.md D12.
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

# sitometres writes the report from a `finally`, so it is missing only when the
# process never reached its exit at all: killed by a job timeout, or OOM. Say
# THAT, rather than passing on jq's error about a missing file. "Nothing was
# proved" is a diagnosis; "a file is absent" is a puzzle.
if [ ! -e "$report" ]; then
    echo "::error::no JSON report — sitometres was killed before it could write one (job timeout?), so nothing was proved"
    exit 1
fi

require_jq_yq || exit 1

# A spec with no `steps:` list is refused rather than counted: jq's
# `null | length` is 0, which an empty run would match.
expected=$(yq '.steps | if type == "array" then length else error("the spec has no steps: list") end' "$spec")
count=$(jq '.steps // [] | length' "$report")

echo "verdict: $(jq -r '.verdict' "$report")"
jq -r '(.steps // [])[] | (.verdict | tostring) as $v
       | "  [\((" " * (12 - ($v | length))) // "")\($v)] \(.name)"' "$report"

problems=""
problem() {
    problems="$problems::error::$1
"
}

if [ "$(jq '.verdict == "pass"' "$report")" != true ]; then
    problem "verdict is '$(jq -r '.verdict' "$report")', expected 'pass'"
fi
# Written as "equal, or else a problem" so that it fails CLOSED: `[` returns 2,
# not 1, when either side is not an integer, and `-ne` in an `if` would read
# that error as "not unequal" and pass the run. Measured, with the yq guard
# above disabled and a YAML-answering yq on PATH: `-ne` printed `ok: all 2
# steps passed`.
[ "$count" -eq "$expected" ] ||
    problem "report has $count steps, spec has $expected — the run did not execute the whole spec"
failed=$(jq -r '[(.steps // [])[] | select(.verdict != "pass") | .name | tojson] | join(", ")' "$report")
if [ -n "$failed" ]; then
    problem "steps that did not pass: $failed"
fi

if [ -n "$problems" ]; then
    printf '%s' "$problems"
    exit 1
fi
echo "ok: all $count steps passed"
