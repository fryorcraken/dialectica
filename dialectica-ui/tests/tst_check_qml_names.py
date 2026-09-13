#!/usr/bin/env python3
"""Tests for `check_qml_names.py`, the static gate on QML type names.

Run directly: dialectica-ui/tests/tst_check_qml_names.py

WHY THIS FILE EXISTS. The gate is the only thing standing between this repo and
a repeat of the collision that stopped the screen rendering — and until this
change it was a heredoc inside `ci.yml`, which cannot be run except by pushing
a branch. A gate is part of the measurement: narrowed until it matches nothing
it passes exactly as quietly as a correct one. This repo has already been bitten
by precisely that, twice, and the second time was a filter pinned from both
sides whose corpus-builder was then mutated to return nothing with every test
still green. So this file pins three things, not one:

  1. BOTH BOUNDS. The gate must REJECT each bad tree and ACCEPT the good one.
     A gate that only rejects is satisfied by `sys.exit(1)`.
  2. THE REAL TREE. `test_the_shipped_module_passes` runs the gate over
     `dialectica-ui` itself, so a rule that cannot be satisfied by the code we
     actually ship fails here rather than in CI.
  3. THE CORPUS-BUILDERS, DIRECTLY. `qmldir_entries` and the file walk are what
     supply every other check its input, and a gate whose input is empty reports
     clean. The `*_measured_nothing` cases below hand the gate a tree with no
     QML files, a missing qmldir, an empty qmldir and a qmldir with no
     D-prefixed type, and require a FAILURE from each — refusing to measure is
     the only honest answer to an empty corpus.
"""

import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
GATE = HERE / "check_qml_names.py"
MODULE = HERE.parent

# A minimal module that satisfies every rule, built rather than copied so each
# case below can break exactly one thing. Every name is either D-prefixed or in
# the gate's GRANDFATHERED set.
GOOD_QMLDIR = """\
singleton DTheme 1.0 DTheme.qml
singleton Core 1.0 Core.qml
Identicon 1.0 Identicon.qml
"""

GOOD_FILES = {
    "DTheme.qml": 'pragma Singleton\nimport QtQuick\nQtObject { property color paper: "#efe9dc" }\n',
    "Core.qml": "pragma Singleton\nimport QtQuick\nQtObject { }\n",
    "Identicon.qml": "import QtQuick\nItem { property color c: DTheme.paper }\n",
}


def build_tree(root, qmldir=GOOD_QMLDIR, files=None, extra=None):
    """Write a module tree and return its root. `extra` adds files verbatim."""
    qml = root / "src" / "qml"
    qml.mkdir(parents=True, exist_ok=True)
    if qmldir is not None:
        (qml / "qmldir").write_text(qmldir)
    for name, body in (GOOD_FILES if files is None else files).items():
        (qml / name).write_text(body)
    for rel, body in (extra or {}).items():
        target = root / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body)
    return root


def run_gate(root):
    """Return (exit code, combined output)."""
    proc = subprocess.run(
        [sys.executable, str(GATE), str(root)],
        capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


fails = []


def expect_reject(name, root, must_mention=None):
    code, out = run_gate(root)
    if code == 0:
        fails.append(f"{name}: gate ACCEPTED a tree it must reject\n{out}")
        return
    if must_mention and must_mention not in out:
        fails.append(f"{name}: rejected, but the message never names "
                     f"{must_mention!r} — a red nobody can diagnose\n{out}")
        return
    print(f"ok: {name} — rejected")


def expect_accept(name, root):
    code, out = run_gate(root)
    if code != 0:
        fails.append(f"{name}: gate REJECTED a tree it must accept\n{out}")
        return
    print(f"ok: {name} — accepted")


with tempfile.TemporaryDirectory() as tmp:
    tmp = pathlib.Path(tmp)

    # ---- MUST ACCEPT ----------------------------------------------------
    #
    # The half that carries the weight. Every rejection case below is also
    # passed by a gate that rejects everything; only this pins that it does not.
    expect_accept("a module where every rule holds", build_tree(tmp / "good"))

    # The real shipped module. If the convention the gate enforces is one our
    # own code cannot satisfy, that is a defect in the gate and it fails here,
    # in a second, rather than in CI after a Qt install.
    expect_accept("the shipped dialectica-ui module", MODULE)

    # AND that the walk actually REACHED that module, which exit 0 does not say.
    #
    # WHY THIS IS SEPARATE FROM THE CASE ABOVE. Narrowing the walker to
    # `rglob("Core.qml")` fails 5 of the constructed cases — but the shipped
    # module still passed, because the real module happens to contain a
    # `Core.qml`. A gate reading one harmless file reports `ok` and the case
    # above is satisfied. The `if not qml_files` guard catches only a walk that
    # goes to ZERO; a walk narrowed to one file sails past it.
    #
    # The floor is COUNTED FROM DISK here rather than read out of the gate's own
    # message. Asking the gate how many files it saw and agreeing with the
    # answer is not a measurement — it is the implementation confirming itself.
    # Two independent walks of the same tree must agree.
    on_disk = len(list(MODULE.rglob("*.qml")))
    code, out = run_gate(MODULE)
    if code != 0:
        fails.append(f"the shipped module's file count: gate failed\n{out}")
    elif f"{on_disk} QML file(s)" not in out:
        fails.append(
            f"the shipped module's file count: {on_disk} .qml file(s) on disk "
            f"but the gate did not report checking that many — the walk is "
            f"narrower than the tree, so most files went unchecked\n{out}")
    else:
        print(f"ok: the shipped module — all {on_disk} QML file(s) reached")

    # ---- MUST REJECT: the prefix rule over EVERY entry -------------------
    #
    # THE ARCHITECTURE FINDING, pinned. `Theme 1.0 Identicon.qml` is a plain
    # COMPONENT entry declaring the exact name this piece exists to remove. The
    # pre-fix gate read `parts[0] == "singleton"` and skipped it: measured exit
    # 0, `ok: 17 QML file(s) checked`, over a qmldir declaring a type called
    # `Theme`. A non-singleton is shadowable in the same way a singleton is —
    # the host's own launch log registers `LogosButton.qml`, a component.
    expect_reject(
        "a non-singleton qmldir entry named after a host type",
        build_tree(tmp / "component_theme",
                   qmldir=GOOD_QMLDIR + "Theme 1.0 Identicon.qml\n"),
        must_mention="qmldir declares 'Theme'",
    )

    # The singleton form, which the pre-fix gate did catch. Kept so a later
    # rewrite cannot lose the case it already covered while fixing the one it
    # did not.
    expect_reject(
        "a singleton qmldir entry that is not D-prefixed",
        build_tree(tmp / "singleton_theme",
                   qmldir="singleton Theme 1.0 DTheme.qml\n",
                   files={"DTheme.qml": GOOD_FILES["DTheme.qml"]}),
        must_mention="not D-prefixed",
    )

    # `internal` entries are checked too — for a reason that was measured after
    # the first version of this comment asserted the wrong one. `internal` does
    # NOT export the name (a named-module import gets `ProbeInternal is not a
    # type`), but the name is live INSIDE the directory, resolved by filename
    # whether or not the qmldir declares it — and the directory is where
    # basecamp loads the plugin. See `qmldir_entries`' docstring in the gate.
    expect_reject(
        "an internal qmldir entry that is not D-prefixed",
        build_tree(tmp / "internal_theme",
                   qmldir=GOOD_QMLDIR + "internal Theme Identicon.qml\n"),
        must_mention="qmldir declares 'Theme'",
    )

    # ---- MUST ACCEPT: qmldir COMMAND lines are not type declarations -----
    #
    # The other bound on the widened walk. Having made it read every line, a
    # walk that then treats `module Foo` or `depends QtQuick 2.0` as a type
    # named `module` would be red on every correct qmldir — which is the shape
    # someone fixes by narrowing the walk back to `singleton`, undoing the
    # finding. Pinned so that regression fails here instead.
    expect_accept(
        "a qmldir carrying module/depends/import command lines",
        build_tree(tmp / "commands",
                   qmldir="module Dialectica\ndepends QtQuick 2.0\n"
                          "import QtQuick auto\n" + GOOD_QMLDIR),
    )

    # ---- MUST REJECT: a declared file that is not there -------------------
    expect_reject(
        "a qmldir naming a file that does not exist",
        build_tree(tmp / "missing_file",
                   qmldir=GOOD_QMLDIR + "DGhost 1.0 DGhost.qml\n"),
        must_mention="which does not exist",
    )

    # ---- MUST REJECT: a stale reference, DERIVED not hardcoded ------------
    #
    # The second architecture finding. The pre-fix arm searched for the literal
    # word `Theme`, so it caught the collision that already happened and no
    # other. Here the singleton is `DCore` and the body still says `Core.call` —
    # a stale reference from a rename that has nothing to do with `Theme`.
    # Measured against the pre-fix gate on the real tree: 33 such references
    # after renaming `Core` to `DCore`, exit 0.
    expect_reject(
        "a stale reference to a renamed singleton other than Theme",
        build_tree(tmp / "stale_core",
                   qmldir="singleton DCore 1.0 DCore.qml\n",
                   files={"DCore.qml": "pragma Singleton\nimport QtQuick\nQtObject { }\n",
                          "Uses.qml": "import QtQuick\nItem { property var x: Core.call() }\n"}),
        must_mention="references a bare 'Core'",
    )

    # The column-0 form, which is why the lookbehind is `(?<![A-Za-z])` and not
    # `[^A-Za-z]`. A binding split across two lines put `Theme.note` at column
    # 0, and the old regex — needing a character before it — was blind to a
    # genuinely broken binding that passed the gate, qmllint and the suite.
    expect_reject(
        "a stale reference at column 0, which needs no preceding character",
        build_tree(tmp / "column0",
                   extra={"src/qml/Split.qml":
                          "import QtQuick\nItem {\n  font:\nTheme.note\n}\n"}),
        must_mention="references a bare 'Theme'",
    )

    # A tree MID-RENAME — which is what `main` is — must be told about the
    # bodies, not only the qmldir line. The stale set is seeded from the names
    # arm 1 just rejected as well as from the D-prefixed ones, because a type
    # declared without the `D` is a name we are asking to be renamed and every
    # reference to it must change with it. Without that seeding the gate run
    # against `main` printed one error and fell silent about 116 references.
    mid = build_tree(tmp / "mid_rename",
                     qmldir="singleton Theme 1.0 Theme.qml\n",
                     files={"Theme.qml": "pragma Singleton\nimport QtQuick\nQtObject { }\n",
                            "Uses.qml": "import QtQuick\nItem { property color c: Theme.paper }\n"})
    code, out = run_gate(mid)
    if code == 0:
        fails.append("a tree mid-rename: gate ACCEPTED it")
    elif "references a bare 'Theme'" not in out:
        fails.append("a tree mid-rename: the qmldir line is reported but the "
                     f"body references are not — a partial red\n{out}")
    else:
        print("ok: a tree mid-rename — both the declaration and its references reported")

    # ---- MUST REJECT: files OUTSIDE src/qml ------------------------------
    #
    # The walk is an rglob over the module, not a glob of `src/qml/*.qml`. The
    # old glob left `dialectica-ui/tests/` outside the gate entirely.
    expect_reject(
        "a stale reference in tests/, outside src/qml",
        build_tree(tmp / "outside",
                   extra={"tests/tst_x.qml":
                          "import QtQuick\nItem { property var c: Theme.paper }\n"}),
        must_mention="tst_x.qml",
    )

    # ---- MUST ACCEPT: comments are not code ------------------------------
    #
    # The other bound on comment stripping. A gate that fires on its own
    # explanation is a gate someone weakens rather than obeys — and `DTheme.qml`
    # quotes `Theme.x` in its header on purpose, explaining the collision.
    expect_accept(
        "a bare Theme inside //, /* */ and trailing comments",
        build_tree(tmp / "comments",
                   extra={"src/qml/Commented.qml":
                          "import QtQuick\n"
                          "// Theme.paper was the old name\n"
                          "/* and Theme.ink\n   across lines */\n"
                          "Item { property color c: DTheme.paper } // not Theme.paper\n"}),
    )

    # ---- MUST REJECT: an empty corpus ------------------------------------
    #
    # THE HARDEST CASE, and the reason the three above are not enough. Every
    # check in the gate runs over a corpus something else builds: the file walk
    # and `qmldir_entries`. If either silently returns nothing, every check
    # passes vacuously and the gate reports `ok` — a green that measured
    # nothing, which this repo rates as worse than no gate, because it closes
    # the question. So breaking each corpus-builder must produce a FAILURE.
    empty = tmp / "no_qml"
    (empty / "src" / "qml").mkdir(parents=True)
    expect_reject("a module with no QML files at all", empty,
                  must_mention="measured nothing")

    no_qmldir = tmp / "no_qmldir"
    build_tree(no_qmldir, qmldir=None)
    expect_reject("a module with no qmldir", no_qmldir,
                  must_mention="measured nothing")

    expect_reject(
        "a qmldir declaring no types",
        build_tree(tmp / "empty_qmldir",
                   qmldir="# every line a comment\nmodule Dialectica\n"),
        must_mention="measured nothing",
    )

    # And the reference check's own corpus: it looks for the unprefixed form of
    # each D-prefixed type, so a qmldir with no D-prefixed type leaves it with
    # nothing to search for. That is indistinguishable from a clean tree unless
    # the gate says so.
    expect_reject(
        "a qmldir with no D-prefixed type, leaving nothing to search for",
        build_tree(tmp / "nothing_to_find",
                   qmldir="singleton Core 1.0 Core.qml\n",
                   files={"Core.qml": GOOD_FILES["Core.qml"]}),
        must_mention="nothing to look for",
    )

if fails:
    print()
    for f in fails:
        print(f"FAIL: {f}", file=sys.stderr)
    print(f"check_qml_names: {len(fails)} case(s) failed", file=sys.stderr)
    sys.exit(1)
print("check_qml_names: all cases passed")
