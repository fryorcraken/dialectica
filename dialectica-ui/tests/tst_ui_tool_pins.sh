#!/usr/bin/env sh
# The tool versions the two UI workflows share must agree, and be exact.
#
# ci.yml's `ui-specs` job validates every spec against sitometres' schema, and
# ui-tests.yml's `spec` job runs the specs with sitometres. If the two name
# different versions, the cheap job validates against a schema the expensive
# job does not enforce, and both stay green. The same holds, at lower stakes,
# for the lgs version ci.yml's `build` job and ui-tests.yml each install.
#
# TWO LITERALS AND A CHECK, NOT ONE HOME. Each workflow keeps its own literal as
# a job-level `env:` value, where someone reading that job sees it, and this
# file keeps them equal. The e2e-ui-suite change's design.md D8 says why a
# single file loaded by both
# workflows was rejected.
#
# It also fails on a version written straight into a `run:` body. A literal
# there bypasses the `env:` value this file compares, so the comparison would
# pass while the job ran something else.
#
# HOW. `yq` turns each workflow into JSON once; everything after that is jq.
# The check is one jq program, PROBLEMS below, which prints one line per
# problem and nothing for a clean pair. Every failing case is the real pair of
# workflows with exactly one thing changed by a jq edit, so a failure cannot
# come from a fixture broken in some other way.
#
# Run: dialectica-ui/tests/tst_ui_tool_pins.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
workflows=$here/../../.github/workflows
. "$here/require-jq-yq.sh"
require_jq_yq

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

yq . "$workflows/ci.yml" > "$work/ci.json"
yq . "$workflows/ui-tests.yml" > "$work/ui.json"

# One row per shared tool: its env name, the job holding it in each workflow,
# the only shape its value may take (an exact version, never a range or
# `latest`), and what a version written straight into a `run:` body looks like.
PROBLEMS='
def pins: [
  { tool: "sitometres", env: "SITOMETRES", ci_job: "ui-specs", ui_job: "spec",
    exact: "^@paradoxcomputer/sitometres@[0-9]+\\.[0-9]+\\.[0-9]+$",
    literal_in_run: "sitometres@" },
  { tool: "lgs", env: "LGS_VERSION", ci_job: "build", ui_job: "spec",
    exact: "^[0-9]+\\.[0-9]+\\.[0-9]+$",
    literal_in_run: "logos-scaffold\\s+--version\\s+[0-9]" }
];

# The job-level env value as a string, or null when it is absent.
def env_value($doc; $job; $name):
  $doc.jobs[$job].env[$name] | if . == null then null else tostring end;

def pin_problems($ci; $ui):
  pins[] as $p
  | env_value($ci; $p.ci_job; $p.env) as $c
  | env_value($ui; $p.ui_job; $p.env) as $u
  | ( ( ["ci.yml", $p.ci_job, $c], ["ui-tests.yml", $p.ui_job, $u] )
      | . as [$workflow, $job, $value]
      | if $value == null then
          "\($p.tool): \($workflow) has no jobs.\($job).env.\($p.env)"
        elif ($value | test($p.exact) | not) then
          "\($p.tool): \($workflow) jobs.\($job).env.\($p.env) is \($value | tojson), which is not an exact version"
        else empty end ),
    ( if $c != null and $u != null and $c != $u then
        "\($p.tool): ci.yml pins \($c | tojson) but ui-tests.yml pins \($u | tojson). They must be the same version"
      else empty end );

def literal_problems($ci; $ui):
  ( ["ci.yml", $ci], ["ui-tests.yml", $ui] ) as [$workflow, $doc]
  | ($doc.jobs // {}) | to_entries[] as {key: $job, value: $body}
  | (($body // {}).steps // [])[] as $step
  | pins[] as $p
  | select(($step.run // "") | tostring | test($p.literal_in_run))
  | "\($p.tool): \($workflow) jobs.\($job) step \($step.name | tojson) writes a version into its run: body. Use $\($p.env) so this check can see it";

pin_problems($ci[0]; $ui[0]), literal_problems($ci[0]; $ui[0])
'

# problems <ci.json> <ui.json>: sets $found to the problem lines. A jq error is
# itself reported as a problem rather than ending this file, so a case that
# exists to rule out a crash can see one.
problems() {
    if found=$(jq -n -r --slurpfile ci "$1" --slurpfile ui "$2" "$PROBLEMS" 2>&1); then
        :
    else
        found="jq exited $?: $found"
    fi
}

# edit <in.json> <jq filter>: writes the edited copy to $work/edited.json.
edit() {
    jq "$2" "$1" > "$work/edited.json"
}

failures=0
# reported <description> <tool> <phrase>: one problem line is about <tool> and
# says <phrase>.
reported() {
    if printf '%s\n' "$found" | grep "^$2: " | grep -qF "$3"; then
        echo "  ok: $1"
    else
        echo "  FAIL: $1 — got: $found"
        failures=$((failures + 1))
    fi
}

echo "the workflows as committed"
problems "$work/ci.json" "$work/ui.json"
if [ -z "$found" ]; then
    echo "  ok: pin every shared tool at one exact version"
else
    echo "  FAIL: pin every shared tool at one exact version — got: $found"
    failures=$((failures + 1))
fi

echo "ui-tests.yml bumps sitometres and ci.yml does not"
edit "$work/ui.json" '.jobs.spec.env.SITOMETRES = "@paradoxcomputer/sitometres@9.9.9"'
problems "$work/ci.json" "$work/edited.json"
reported "is reported" sitometres "must be the same version"

echo "ci.yml bumps lgs and ui-tests.yml does not"
edit "$work/ci.json" '.jobs.build.env.LGS_VERSION = "9.9.9"'
problems "$work/edited.json" "$work/ui.json"
reported "is reported" lgs "must be the same version"

echo "both workflows agree on a range"
# Agreement is not enough: two copies of `^0.1.2` agree and still move.
jq '.jobs["ui-specs"].env.SITOMETRES = "@paradoxcomputer/sitometres@^0.1.2"' \
    "$work/ci.json" > "$work/ranged-ci.json"
jq '.jobs.spec.env.SITOMETRES = "@paradoxcomputer/sitometres@^0.1.2"' \
    "$work/ui.json" > "$work/ranged-ui.json"
problems "$work/ranged-ci.json" "$work/ranged-ui.json"
reported "is reported" sitometres "not an exact version"

echo "a step writes sitometres' version into its run: body"
edit "$work/ci.json" '.jobs["ui-specs"].steps += [{"name": "a bypass", "run": "npm install @paradoxcomputer/sitometres@0.1.2"}]'
problems "$work/edited.json" "$work/ui.json"
reported "is reported" sitometres "writes a version"

echo "a step writes lgs' version into its run: body"
edit "$work/ui.json" '.jobs.spec.steps += [{"name": "a bypass", "run": "cargo install logos-scaffold --version 0.3.1 --locked"}]'
problems "$work/ci.json" "$work/edited.json"
reported "is reported" lgs "writes a version"

echo "a pin removed from one workflow"
edit "$work/ci.json" 'del(.jobs["ui-specs"].env.SITOMETRES)'
problems "$work/edited.json" "$work/ui.json"
reported "is reported by name, not by a jq error" sitometres "has no jobs.ui-specs.env.SITOMETRES"

echo
if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "ok: the UI workflows pin each shared tool at one exact version"
