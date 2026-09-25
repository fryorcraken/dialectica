#!/usr/bin/env python3
"""Tests for ui-tests.yml's "lgs left scaffold.toml's values alone" guard.

design.md D4 records that both `lgs basecamp setup` and `lgs basecamp install`
rewrite `scaffold.toml` (CLAUDE.md), and that the guard exists because a
changed VALUE — as opposed to a stripped comment — would mean the run
exercises a Basecamp or module set the repository does not declare. tasks.md
2.3 records the check has passed on every CI run so far, and that it "has not
been shown to fail: no run has had a verb change a value". That is a real gap:
a check nobody has watched go red is not yet known to be able to.

WHAT THIS FILE CAN AND CANNOT CLOSE. Whether `lgs basecamp setup`/`install`
themselves ever rewrite a VALUE is a fact about `lgs`, observable only by
running it — which this repo's owner has restricted to `lgs basecamp launch`
locally and to CI otherwise (see this piece's tasks.md and design.md Risks).
This file does not attempt that; it cannot prove `lgs` never trips the guard.

What it closes is the other half: whether the guard's OWN mechanism —
`tomlq -S . scaffold.toml`, snapshotted before and diffed after — actually
tells a changed value from the comment-and-whitespace rewrite `lgs` is known
to perform. That is a property of `tomlq -S` and `diff`, checkable without
`lgs`, Nix or a network, and it is the property the guard is trusted to have.

EXTRACTED, NOT REIMPLEMENTED. The two `run:` blocks are read out of
`ui-tests.yml` itself by step name, not retyped here — the same discipline
`tst_check_bindings.sh` applies to `check_bindings`. Retyping would test this
file's idea of the guard rather than the guard, and would stay green through a
change to the real steps. Renaming either step fails this file loudly instead.

Run: python3 dialectica-ui/tests/tst_scaffold_values_unchanged.py
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ui-tests.yml"

BEFORE_STEP = "Read scaffold.toml before lgs touches it"
AFTER_STEP = "lgs left scaffold.toml's values alone"

# A realistic fixture: a `[repos.basecamp].pin` that satisfies the BEFORE
# step's own "full 40-char lowercase sha" check, plus a second table and a
# comment, so a comment-and-reorder rewrite has something to remove and
# reorder. Values are otherwise arbitrary.
GOOD_TOML = """\
# a comment lgs is documented to strip
[repos.basecamp]
pin = "aa237766baf61404e12da86b7303cb41065464c9"
attr = "app"

[repos.lgpm]
pin = "d3af2972f51d9c542537d80d60ad8d20282ddcd1"
attr = "cli"
"""

# Same values, no comment, tables and keys reordered, different quoting style
# for one string — the shape of rewrite `lgs basecamp setup`/`install` are
# documented to perform (CLAUDE.md: "any `lgs basecamp` verb rewrites the
# file"). Not one value differs from GOOD_TOML.
COSMETIC_REWRITE_TOML = """\
[repos.lgpm]
attr = 'cli'
pin = "d3af2972f51d9c542537d80d60ad8d20282ddcd1"

[repos.basecamp]
attr = "app"
pin = "aa237766baf61404e12da86b7303cb41065464c9"
"""

# One character of one value changed (`pin` truncated), values otherwise the
# same — the case the guard exists to catch.
VALUE_CHANGED_TOML = GOOD_TOML.replace(
    "aa237766baf61404e12da86b7303cb41065464c9",
    "aa237766baf61404e12da86b7303cb41065464c0",
)


def extract_step(name):
    """The literal `run:` text of the named step in `spec`'s job.

    Raises rather than returning None on a miss: a step that has been renamed
    must fail this file loudly, not report a false green over a body that was
    never read.
    """
    with open(WORKFLOW) as fh:
        doc = yaml.safe_load(fh)
    for step in doc["jobs"]["spec"]["steps"]:
        if step.get("name") == name:
            run = step.get("run")
            if not run:
                raise AssertionError(f"step {name!r} in {WORKFLOW} has no 'run:' body")
            return run
    raise AssertionError(
        f"no step named {name!r} in {WORKFLOW} — has it been renamed or removed? "
        "this test extracts it by name and must be updated with it"
    )


def run_step(script, cwd, runner_temp, github_env):
    env = dict(os.environ)
    env["RUNNER_TEMP"] = str(runner_temp)
    env["GITHUB_ENV"] = str(github_env)
    proc = subprocess.run(
        ["bash", "-c", script],
        cwd=cwd, env=env, capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def scenario(before_toml, after_toml):
    """Run BOTH real steps back to back, in a fresh workdir.

    `before_toml` is written before the BEFORE step runs (its snapshot);
    `after_toml` replaces it before the AFTER step runs (its snapshot). A run
    where `lgs` changed nothing is `scenario(x, x)`.
    """
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        runner_temp = tmp / "runner_temp"
        runner_temp.mkdir()
        github_env = tmp / "github_env"
        github_env.write_text("")
        scaffold = tmp / "scaffold.toml"

        scaffold.write_text(before_toml)
        code, out = run_step(extract_step(BEFORE_STEP), tmp, runner_temp, github_env)
        if code != 0:
            raise AssertionError(f"the BEFORE step itself failed on the fixture:\n{out}")

        scaffold.write_text(after_toml)
        return run_step(extract_step(AFTER_STEP), tmp, runner_temp, github_env)


FAILURES = []


def check(description, condition, detail=""):
    if condition:
        print(f"  ok: {description}")
    else:
        print(f"  FAIL: {description} {detail}")
        FAILURES.append(description)


def main():
    print("scaffold.toml genuinely unchanged")
    code, out = scenario(GOOD_TOML, GOOD_TOML)
    check("exits 0", code == 0, f"(got {code}: {out})")
    check("says so", "ok: scaffold.toml's values are unchanged" in out, out)

    print("a comment-and-reorder rewrite with every value identical (what lgs does)")
    # This is the pairing case: without it, a guard that fires on ANY textual
    # change — which is what `git diff` would have done, and #120's mistake —
    # would pass the case above for the wrong reason (an accidental byte-for-byte
    # match) and this file would not know the difference.
    code, out = scenario(GOOD_TOML, COSMETIC_REWRITE_TOML)
    check("still exits 0", code == 0, f"(got {code}: {out})")
    check("still says unchanged", "ok: scaffold.toml's values are unchanged" in out, out)

    print("one value actually changed (one hex digit of a pin)")
    code, out = scenario(GOOD_TOML, VALUE_CHANGED_TOML)
    check("exits 1", code == 1, f"(got {code})")
    check("names the cause", "changed a value in scaffold.toml" in out, out)
    check("shows a diff", "pin" in out and "aa237766" in out, out)

    print()
    if FAILURES:
        print(f"{len(FAILURES)} check(s) failed")
        return 1
    print("ok: the guard's diff distinguishes a changed value from lgs's own rewrite")
    return 0


if __name__ == "__main__":
    sys.exit(main())
