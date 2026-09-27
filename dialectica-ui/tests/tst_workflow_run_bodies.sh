#!/usr/bin/env sh
# No workflow splices a `${{ … }}` expression into a `run:` body.
#
# Actions substitutes an expression into the script TEXT before the shell
# parses it, so a value carrying shell syntax would run as shell. Passed
# through an `env:` value instead, it reaches the shell as a variable and is
# one word however it is spelled. Every value these workflows splice is
# repo-controlled today (a hand-typed matrix, a pinned version), so this closes
# no present exploit. It keeps the safe shape total, so that a matrix later
# derived from the tree, or a new step, cannot bring back the unsafe one
# unnoticed. The e2e-suite-review change's design.md D8.
#
# HOW. Each workflow file is turned into JSON once with `yq`, and one jq
# program, PROBLEMS, prints one line per offending step and nothing for a clean
# file. Expressions in `env:`, `with:`, `if:` and step names are the sanctioned
# places and are not flagged; the committed workflows use all four, so their
# clean result is the pairing case for the failing ones below.
#
# Run: dialectica-ui/tests/tst_workflow_run_bodies.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
workflows=$here/../../.github/workflows
. "$here/require-jq-yq.sh"
require_jq_yq

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

PROBLEMS='
(.jobs // {}) | to_entries[] as {key: $job, value: $body}
| (($body // {}).steps // [])[]
| select((.run // "") | tostring | test("\\$\\{\\{"))
| "jobs.\($job) step \(.name // "(unnamed)" | tojson) splices an expression into its run: body. Pass it through env: instead"
'

# problems <workflow.json>: sets $found to the problem lines. A jq error is
# itself reported as a problem rather than ending this file.
problems() {
    if found=$(jq -r "$PROBLEMS" "$1" 2>&1); then
        :
    else
        found="jq exited $?: $found"
    fi
}

failures=0
ok() { echo "  ok: $1"; }
bad() {
    echo "  FAIL: $1 — $2"
    failures=$((failures + 1))
}

echo "the workflows as committed"
# Globbed rather than listed, so a third workflow is covered the day it lands.
# A glob that matches nothing is left as the literal pattern, which `-e`
# skips, and a count of zero fails: a check over no files cannot fail.
checked=0
for file in "$workflows"/*.yml "$workflows"/*.yaml; do
    [ -e "$file" ] || continue
    yq . "$file" > "$work/workflow.json"
    problems "$work/workflow.json"
    if [ -z "$found" ]; then
        ok "$(basename "$file") splices nothing into a run: body"
    else
        bad "$(basename "$file") splices nothing into a run: body" "$found"
    fi
    checked=$((checked + 1))
done
if [ "$checked" -eq 0 ]; then
    bad "found a workflow to check" "no *.yml or *.yaml under $workflows"
fi

yq . "$workflows/ui-tests.yml" > "$work/ui.json"

echo "a step splices the matrix value into its run: body"
jq '.jobs.spec.steps += [{"name": "a splice", "run": "echo ui-results/${{ matrix.spec }}.json"}]' \
    "$work/ui.json" > "$work/edited.json"
problems "$work/edited.json"
case $found in
    *'"a splice" splices an expression'*) ok "is reported, by step name" ;;
    *) bad "is reported, by step name" "got: $found" ;;
esac

echo "the same value passed through the step's env: instead"
# The one difference from the case above: the expression moved from the run:
# body into an env: value the body reads. That is the fix, and it must pass.
jq '.jobs.spec.steps += [{"name": "a splice", "env": {"RESULT": "ui-results/${{ matrix.spec }}.json"}, "run": "echo \"$RESULT\""}]' \
    "$work/ui.json" > "$work/edited.json"
problems "$work/edited.json"
if [ -z "$found" ]; then
    ok "is not reported"
else
    bad "is not reported" "got: $found"
fi

echo
if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "ok: no workflow splices an expression into a run: body"
