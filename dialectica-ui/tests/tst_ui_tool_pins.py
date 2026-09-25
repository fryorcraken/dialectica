#!/usr/bin/env python3
"""The tool versions the two UI workflows share must agree, and be exact.

ci.yml's `ui-specs` job validates every spec against sitometres' schema, and
ui-tests.yml's `spec` job runs the specs with sitometres. If the two name
different versions, the cheap job validates against a schema the expensive
job does not enforce, and both stay green. The same holds, at lower stakes,
for the lgs version ci.yml's `build` job and ui-tests.yml each install.

TWO LITERALS AND A CHECK, NOT ONE HOME. Each workflow keeps its own literal as
a job-level `env:` value, where someone reading that job sees it, and this
file keeps them equal. design.md D8 says why a single file loaded by both
workflows was rejected.

It also fails on a version written straight into a `run:` body. A literal
there bypasses the `env:` value this file compares, so the comparison would
pass while the job ran something else.

Every failing case below is the real pair of workflows with exactly one thing
changed, so a failure cannot come from a fixture broken in some other way.

Run:  python3 dialectica-ui/tests/tst_ui_tool_pins.py
"""

import copy
import re
import sys
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CI = REPO_ROOT / ".github" / "workflows" / "ci.yml"
UI = REPO_ROOT / ".github" / "workflows" / "ui-tests.yml"

# One row per shared tool: its env name, the job holding it in each workflow,
# the only shape its value may take (an exact version, never a range or
# `latest`), and what a version written straight into a `run:` body looks like.
SHARED_PINS = [
    {
        "tool": "sitometres",
        "env": "SITOMETRES",
        "ci_job": "ui-specs",
        "ui_job": "spec",
        "exact": r"@paradoxcomputer/sitometres@\d+\.\d+\.\d+",
        "literal_in_run": r"sitometres@",
    },
    {
        "tool": "lgs",
        "env": "LGS_VERSION",
        "ci_job": "build",
        "ui_job": "spec",
        "exact": r"\d+\.\d+\.\d+",
        "literal_in_run": r"logos-scaffold\s+--version\s+\d",
    },
]


def env_value(doc, workflow, job, name):
    """The job-level env value, or None and a problem naming what is missing."""
    value = ((doc.get("jobs") or {}).get(job) or {}).get("env", {}).get(name)
    if value is None:
        return None, f"{workflow} has no jobs.{job}.env.{name}"
    return str(value), None


def pin_problems(ci, ui):
    """Every way the shared pins fail to be one exact version, as messages."""
    problems = []
    for pin in SHARED_PINS:
        values = {}
        for workflow, doc, job in (("ci.yml", ci, pin["ci_job"]),
                                   ("ui-tests.yml", ui, pin["ui_job"])):
            value, missing = env_value(doc, workflow, job, pin["env"])
            if missing:
                problems.append(f"{pin['tool']}: {missing}")
                continue
            if not re.fullmatch(pin["exact"], value):
                problems.append(
                    f"{pin['tool']}: {workflow} jobs.{job}.env.{pin['env']} is "
                    f"{value!r}, which is not an exact version"
                )
            values[workflow] = value
        if len(set(values.values())) > 1:
            problems.append(
                f"{pin['tool']}: ci.yml pins {values['ci.yml']!r} but ui-tests.yml "
                f"pins {values['ui-tests.yml']!r}. They must be the same version"
            )
    return problems


def literal_problems(ci, ui):
    """Every `run:` body that names a shared tool's version itself."""
    problems = []
    for workflow, doc in (("ci.yml", ci), ("ui-tests.yml", ui)):
        for job_name, job in (doc.get("jobs") or {}).items():
            for step in (job or {}).get("steps") or []:
                body = step.get("run") or ""
                for pin in SHARED_PINS:
                    if re.search(pin["literal_in_run"], body):
                        problems.append(
                            f"{pin['tool']}: {workflow} jobs.{job_name} step "
                            f"{step.get('name')!r} writes a version into its run: "
                            f"body. Use ${pin['env']} so this check can see it"
                        )
    return problems


def problems(ci, ui):
    return pin_problems(ci, ui) + literal_problems(ci, ui)


def load(path):
    with open(path) as fh:
        return yaml.safe_load(fh)


FAILURES = []


def check(description, condition, detail=""):
    if condition:
        print(f"  ok: {description}")
    else:
        print(f"  FAIL: {description} {detail}")
        FAILURES.append(description)


def names(found, tool, phrase):
    """True when one reported problem is about `tool` and says `phrase`."""
    return any(p.startswith(f"{tool}:") and phrase in p for p in found)


def main():
    ci, ui = load(CI), load(UI)

    print("the workflows as committed")
    found = problems(ci, ui)
    check("pin every shared tool at one exact version", found == [], f"{found}")

    print("ui-tests.yml bumps sitometres and ci.yml does not")
    bumped = copy.deepcopy(ui)
    bumped["jobs"]["spec"]["env"]["SITOMETRES"] = "@paradoxcomputer/sitometres@9.9.9"
    found = problems(ci, bumped)
    check("is reported", names(found, "sitometres", "must be the same version"), f"{found}")

    print("ci.yml bumps lgs and ui-tests.yml does not")
    bumped = copy.deepcopy(ci)
    bumped["jobs"]["build"]["env"]["LGS_VERSION"] = "9.9.9"
    found = problems(bumped, ui)
    check("is reported", names(found, "lgs", "must be the same version"), f"{found}")

    print("both workflows agree on a range")
    # Agreement is not enough: two copies of `^0.1.2` agree and still move.
    ranged_ci, ranged_ui = copy.deepcopy(ci), copy.deepcopy(ui)
    ranged_ci["jobs"]["ui-specs"]["env"]["SITOMETRES"] = "@paradoxcomputer/sitometres@^0.1.2"
    ranged_ui["jobs"]["spec"]["env"]["SITOMETRES"] = "@paradoxcomputer/sitometres@^0.1.2"
    found = problems(ranged_ci, ranged_ui)
    check("is reported", names(found, "sitometres", "not an exact version"), f"{found}")

    print("a step writes sitometres' version into its run: body")
    literal = copy.deepcopy(ci)
    literal["jobs"]["ui-specs"]["steps"].append(
        {"name": "a bypass", "run": "npm install @paradoxcomputer/sitometres@0.1.2"}
    )
    found = problems(literal, ui)
    check("is reported", names(found, "sitometres", "writes a version"), f"{found}")

    print("a step writes lgs' version into its run: body")
    literal = copy.deepcopy(ui)
    literal["jobs"]["spec"]["steps"].append(
        {"name": "a bypass", "run": "cargo install logos-scaffold --version 0.3.1 --locked"}
    )
    found = problems(ci, literal)
    check("is reported", names(found, "lgs", "writes a version"), f"{found}")

    print("a pin removed from one workflow")
    removed = copy.deepcopy(ci)
    del removed["jobs"]["ui-specs"]["env"]["SITOMETRES"]
    try:
        found = problems(removed, ui)
    except Exception as err:  # the case exists to rule this out
        found = [f"raised {type(err).__name__}: {err}"]
    check("is reported by name, not raised", names(found, "sitometres", "has no jobs.ui-specs.env.SITOMETRES"), f"{found}")

    print()
    if FAILURES:
        print(f"{len(FAILURES)} check(s) failed")
        return 1
    print("ok: the UI workflows pin each shared tool at one exact version")
    return 0


if __name__ == "__main__":
    sys.exit(main())
