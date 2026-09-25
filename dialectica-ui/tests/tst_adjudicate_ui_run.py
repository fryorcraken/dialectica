#!/usr/bin/env python3
"""Pin what the adjudicator must catch AND what it must not.

A check narrowed to nothing passes as quietly as a correct one, so every case
below that expects a FAILURE is paired with the passing case it is derived
from — each fixture differs from the green one in exactly the property under
test, so a pass here cannot come from the fixture being broken in some other
way.

Run:  python3 dialectica-ui/tests/tst_adjudicate_ui_run.py
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJUDICATOR = HERE / "adjudicate-ui-run.py"


def step(name, verdict="pass"):
    return {"name": name, "verdict": verdict}


def run(report, spec_steps, write_report=True):
    """Run the adjudicator over a fixture pair, returning (exit code, output)."""
    with tempfile.TemporaryDirectory() as tmp:
        report_path = Path(tmp) / "report.json"
        spec_path = Path(tmp) / "spec.yaml"

        if write_report:
            report_path.write_text(json.dumps(report))

        # Written as real YAML rather than assembled from a template, so the
        # step count the adjudicator reads is the count of a document that
        # actually parses.
        lines = ["steps:"]
        for i in range(spec_steps):
            lines.append(f"  - name: step {i}")
            lines.append("    click: something")
        spec_path.write_text("\n".join(lines) + "\n")

        proc = subprocess.run(
            [sys.executable, str(ADJUDICATOR), str(report_path), str(spec_path)],
            capture_output=True,
            text=True,
        )
        return proc.returncode, proc.stdout + proc.stderr


FAILURES = []


def check(description, condition, detail=""):
    if condition:
        print(f"  ok: {description}")
    else:
        print(f"  FAIL: {description} {detail}")
        FAILURES.append(description)


def main():
    print("a report that satisfies all three conditions passes")
    code, out = run({"verdict": "pass", "steps": [step("a"), step("b")]}, 2)
    check("exit 0", code == 0, f"(got {code}: {out})")
    check("says so", "all 2 steps passed" in out)

    print("a run that stopped early fails, even with nothing failing")
    # The ONLY difference from the green fixture: the spec declares three steps
    # and the report carries two. Verdict is a pass and no step failed, so the
    # first two conditions hold — this is the case condition 3 exists for.
    code, out = run({"verdict": "pass", "steps": [step("a"), step("b")]}, 3)
    check("exit 1", code == 1, f"(got {code})")
    check("names the cause", "did not execute the whole spec" in out, out)

    print("a failed step inside a passing report fails, and is named")
    code, out = run({"verdict": "pass", "steps": [step("a"), step("b", "fail")]}, 2)
    check("exit 1", code == 1, f"(got {code})")
    check("names the step", "'b'" in out, out)

    print("an inconclusive verdict fails")
    code, out = run({"verdict": "inconclusive", "steps": [step("a"), step("b")]}, 2)
    check("exit 1", code == 1, f"(got {code})")
    check("names the verdict", "inconclusive" in out, out)

    print("every failing condition is reported, not just the first")
    # Three conditions broken at once: verdict, a failed step, and a short count.
    code, out = run({"verdict": "fail", "steps": [step("a", "fail")]}, 2)
    check("exit 1", code == 1, f"(got {code})")
    check("reports the verdict", "expected 'pass'" in out, out)
    check("reports the count", "did not execute the whole spec" in out, out)
    check("reports the step", "steps that did not pass" in out, out)

    print("a missing report says nothing was proved, not that a file is absent")
    code, out = run({}, 2, write_report=False)
    check("exit 1", code == 1, f"(got {code})")
    check("names the real cause", "nothing was proved" in out, out)
    check("is not a traceback", "Traceback" not in out, out)

    print()
    if FAILURES:
        print(f"{len(FAILURES)} check(s) failed")
        return 1
    print("ok: the adjudicator catches each condition and passes a clean run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
