#!/usr/bin/env python3
"""Decide whether a sitometres run proved what its spec asks.

    adjudicate-ui-run.py <report.json> <spec.yaml>

THE EXIT CODE IS NOT THE GATE, which is the whole reason this exists.
sitometres exits 0 on INCONCLUSIVE by design, so `--strict` is passed at the
call site to close that — and even with it, the exit code cannot see a run that
started the application, executed nothing further, and reported a clean sheet.

So three conditions are read off the machine report, and all three must hold:

  1. the report's overall verdict is a pass;
  2. every step in the report is a pass;
  3. the number of steps in the report equals the number in the spec.

The THIRD is the one that carries the weight. Drop it and an empty run
satisfies the other two and reads as a pass — which is worse than a red run,
because it reads as evidence when it is the absence of evidence. Measured:
removing the `len(steps) != expected` branch below turns exactly five checks
in `tst_adjudicate_ui_run.py` red — both of the "stopped early" case's, both of
the "more steps than the spec" case's (the other direction of the same
inequality — a phantom or double-logged step, not merely a dropped one), and
"reports the count" in the every-condition case — and the stopped-early fixture
then prints `ok: all 2 steps passed`.

The report is the authority rather than the terminal summary because the
summary is styled for a human — ANSI-coloured, with no stable field to match
on — while the report is written from a `finally`, on every exit path.

EVERY failing condition is reported before exiting, not just the first: a run
that fails two of them should say so once rather than over two CI runs.
"""

import json
import os
import sys

import yaml


def main(argv):
    if len(argv) != 3:
        print(f"usage: {argv[0]} <report.json> <spec.yaml>", file=sys.stderr)
        return 2

    report_path, spec_path = argv[1], argv[2]

    # sitometres writes the report from a `finally`, so it is missing only when
    # the process never reached its exit at all — killed by a job timeout, or
    # OOM. Say THAT, rather than dying on a traceback about a missing file: the
    # difference between "nothing was proved" and "a file is absent" is the
    # difference between a diagnosis and a puzzle.
    if not os.path.exists(report_path):
        print(
            "::error::no JSON report — sitometres was killed before it could "
            "write one (job timeout?), so nothing was proved"
        )
        return 1

    with open(report_path) as fh:
        report = json.load(fh)
    with open(spec_path) as fh:
        spec = yaml.safe_load(fh)

    expected = len(spec["steps"])
    steps = report.get("steps", [])

    print(f"verdict: {report.get('verdict')}")
    for step in steps:
        print(f"  [{str(step.get('verdict')):>12}] {step.get('name')}")

    problems = []
    if report.get("verdict") != "pass":
        problems.append(f"verdict is {report.get('verdict')!r}, expected 'pass'")
    if len(steps) != expected:
        problems.append(
            f"report has {len(steps)} steps, spec has {expected} — "
            "the run did not execute the whole spec"
        )
    bad = [s.get("name") for s in steps if s.get("verdict") != "pass"]
    if bad:
        problems.append(f"steps that did not pass: {bad}")

    if problems:
        for problem in problems:
            print(f"::error::{problem}")
        return 1

    print(f"ok: all {len(steps)} steps passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
