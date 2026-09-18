#!/usr/bin/env python3
"""`check_qml_reachable.py` catches what it claims, and passes what it must.

Run: dialectica-ui/tests/tst_check_qml_reachable.py

BOTH DIRECTIONS, because a gate that only rejects is satisfied by `sys.exit(1)`
and a gate narrowed until it matches nothing passes exactly as quietly as a
correct one. So every case below pins one of:

  * a tree the gate MUST reject, and the message must name the type — a red
    nobody can diagnose is barely better than a green;
  * a tree the gate MUST accept;
  * the shipped `dialectica-ui` module, which must pass;
  * the gate's own corpus-builders, which must FAIL rather than report clean
    when handed nothing to measure.

THE CASE THAT EARNS THIS FILE is `a_type_reachable_only_through_an_unreachable_type`.
It is the one that distinguishes a transitive walk from a mention-based grep, and
the mention-based version is the cheap implementation someone will reach for. On
that tree it reports clean and the defect ships. Deleting the transitive walk in
`reachable_from` turns exactly that case red.
"""

import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
GATE = HERE / "check_qml_reachable.py"
MODULE = HERE.parent

# A minimal module that passes: a root instantiating one component, which reads
# one singleton.
GOOD_QMLDIR = """\
singleton DTheme 1.0 DTheme.qml
DPanel 1.0 DPanel.qml
"""

GOOD_FILES = {
    "Main.qml": "import QtQuick\nItem { DPanel { } }\n",
    "DTheme.qml": 'pragma Singleton\nimport QtQuick\nQtObject { property color paper: "#efe9dc" }\n',
    "DPanel.qml": "import QtQuick\nRectangle { color: DTheme.paper }\n",
}


def build_tree(root, qmldir=GOOD_QMLDIR, files=None):
    qml = root / "src" / "qml"
    qml.mkdir(parents=True, exist_ok=True)
    if qmldir is not None:
        (qml / "qmldir").write_text(qmldir)
    for name, body in (GOOD_FILES if files is None else files).items():
        (qml / name).write_text(body)
    return root


def run_gate(root):
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

    expect_accept("a module where every registration is reached",
                  build_tree(tmp / "good"))

    # ---- the case the whole gate turns on -------------------------------
    #
    # `DLeaf` is instantiated — but only by `DOrphan`, which nothing
    # instantiates. A mention-based check ("does the name appear anywhere?")
    # reports BOTH reachable and ships the defect. The transitive walk reports
    # both, because neither is reached from the root.
    #
    # This is the shape the real defect had: `DTip` was instantiated by
    # `DStatusBar` and `DVouchStamp`, and both of those were unmounted.
    expect_reject(
        "a type reachable only through an unreachable type",
        build_tree(
            tmp / "island",
            qmldir="singleton DTheme 1.0 DTheme.qml\n"
                   "DPanel 1.0 DPanel.qml\n"
                   "DOrphan 1.0 DOrphan.qml\n"
                   "DLeaf 1.0 DLeaf.qml\n",
            files=dict(
                GOOD_FILES,
                **{
                    "DOrphan.qml": "import QtQuick\nItem { DLeaf { } }\n",
                    "DLeaf.qml": "import QtQuick\nRectangle { color: DTheme.paper }\n",
                },
            ),
        ),
        must_mention="DLeaf",
    )

    # And the orphan itself is named too, not only its leaf.
    expect_reject(
        "the unreachable type at the head of the island",
        build_tree(
            tmp / "island2",
            qmldir="singleton DTheme 1.0 DTheme.qml\n"
                   "DPanel 1.0 DPanel.qml\n"
                   "DOrphan 1.0 DOrphan.qml\n"
                   "DLeaf 1.0 DLeaf.qml\n",
            files=dict(
                GOOD_FILES,
                **{
                    "DOrphan.qml": "import QtQuick\nItem { DLeaf { } }\n",
                    "DLeaf.qml": "import QtQuick\nRectangle { color: DTheme.paper }\n",
                },
            ),
        ),
        must_mention="DOrphan",
    )

    # ---- a plain unreached registration ---------------------------------
    expect_reject(
        "a registered type nothing instantiates",
        build_tree(
            tmp / "unreached",
            qmldir=GOOD_QMLDIR + "DGhost 1.0 DGhost.qml\n",
            files=dict(GOOD_FILES,
                       **{"DGhost.qml": "import QtQuick\nItem { }\n"}),
        ),
        must_mention="DGhost",
    )

    # ---- the record, and what it must and must not excuse ----------------
    expect_accept(
        "an unreached type carrying a reason",
        build_tree(
            tmp / "recorded",
            qmldir=GOOD_QMLDIR
                   + "\n# UNINSTANTIATED: held back by a scope decision\n"
                     "DGhost 1.0 DGhost.qml\n",
            files=dict(GOOD_FILES,
                       **{"DGhost.qml": "import QtQuick\nItem { }\n"}),
        ),
    )

    # A record with no reason after the colon is not a record. "Intended" with
    # nothing said is indistinguishable from an oversight waved through, which
    # is the difference the record exists to make visible.
    expect_reject(
        "a record stating no reason",
        build_tree(
            tmp / "no_reason",
            qmldir=GOOD_QMLDIR + "\n# UNINSTANTIATED:\nDGhost 1.0 DGhost.qml\n",
            files=dict(GOOD_FILES,
                       **{"DGhost.qml": "import QtQuick\nItem { }\n"}),
        ),
        must_mention="DGhost",
    )

    # A record on a type that IS reached is stale, and a stale record excuses a
    # type nobody is holding back. The next reader cannot tell it from a live
    # one, so the gate refuses it rather than ignoring it.
    expect_reject(
        "a stale record on a type the root does reach",
        build_tree(
            tmp / "stale",
            qmldir="singleton DTheme 1.0 DTheme.qml\n"
                   "\n# UNINSTANTIATED: this reason is no longer true\n"
                   "DPanel 1.0 DPanel.qml\n",
        ),
        must_mention="DPanel",
    )

    # A blank line breaks the association, so one record cannot drift onto a
    # type added below it by someone who never read the comment.
    expect_reject(
        "a record separated from its registration by a blank line",
        build_tree(
            tmp / "detached",
            qmldir=GOOD_QMLDIR
                   + "\n# UNINSTANTIATED: a reason for something else\n\n"
                     "DGhost 1.0 DGhost.qml\n",
            files=dict(GOOD_FILES,
                       **{"DGhost.qml": "import QtQuick\nItem { }\n"}),
        ),
        must_mention="DGhost",
    )

    # ---- a name in a comment is not an instantiation ---------------------
    #
    # Not hypothetical: `Core.qml` names `DOnboardingScreen` in prose while
    # instantiating nothing, and the real defect was live while that comment
    # existed.
    expect_reject(
        "a type named only in a comment",
        build_tree(
            tmp / "comment_only",
            qmldir=GOOD_QMLDIR + "DGhost 1.0 DGhost.qml\n",
            files=dict(
                GOOD_FILES,
                **{
                    "Main.qml": "import QtQuick\n// see DGhost { } for why\nItem { DPanel { } }\n",
                    "DGhost.qml": "import QtQuick\nItem { }\n",
                },
            ),
        ),
        must_mention="DGhost",
    )

    # ---- a singleton is reached by a member read, not an instantiation ---
    #
    # The first version of this gate asked whether anything wrote `DTheme { }`
    # and reported all three of the module's singletons as dead code. A
    # singleton nothing READS is still caught.
    expect_reject(
        "a singleton nothing reads",
        build_tree(
            tmp / "dead_singleton",
            qmldir=GOOD_QMLDIR + "singleton DUnused 1.0 DUnused.qml\n",
            files=dict(GOOD_FILES,
                       **{"DUnused.qml": "pragma Singleton\nimport QtQuick\nQtObject { }\n"}),
        ),
        must_mention="DUnused",
    )

    # ---- the corpus-builders, which must fail rather than report clean ---
    #
    # A gate handed nothing to measure must say so. Reporting "ok" over an empty
    # walk is the failure this repo has already paid for elsewhere.
    empty = tmp / "no_qmldir"
    (empty / "src" / "qml").mkdir(parents=True)
    expect_reject("a tree with no qmldir", empty, must_mention="measured nothing")

    no_types = build_tree(tmp / "no_types", qmldir="# only a comment\n")
    expect_reject("a qmldir declaring no types", no_types,
                  must_mention="measured nothing")

    no_root = build_tree(
        tmp / "no_root",
        files={k: v for k, v in GOOD_FILES.items() if k != "Main.qml"},
    )
    expect_reject("a tree with no Main.qml", no_root,
                  must_mention="measured nothing")

    # A walk that reaches nothing beyond the root means the instantiation scan
    # stopped matching, not that the view is empty — silent narrowing is the
    # failure a gate cannot report on itself.
    narrowed = build_tree(
        tmp / "narrowed",
        qmldir="DPanel 1.0 DPanel.qml\n",
        files={
            "Main.qml": "import QtQuick\nItem { }\n",
            "DPanel.qml": "import QtQuick\nRectangle { }\n",
        },
    )
    expect_reject("a root reaching nothing at all", narrowed,
                  must_mention="measured nothing")

# ---- and the tree that actually ships ------------------------------------
expect_accept("the shipped dialectica-ui module", MODULE)

# Two independent walks of the same tree must agree. Counting the registrations
# from disk rather than trusting the gate's own summary is what catches a walk
# narrower than the file.
declared_on_disk = 0
for line in (MODULE / "src/qml/qmldir").read_text().splitlines():
    parts = line.strip().split()
    if not parts or parts[0].startswith("#"):
        continue
    if parts[0] in ("module", "depends", "import", "optional", "prefer",
                    "typeinfo", "classname", "plugin", "designersupported"):
        continue
    declared_on_disk += 1

code, out = run_gate(MODULE)
if code != 0:
    fails.append(f"the shipped module's registration count: gate failed\n{out}")
elif f"{declared_on_disk} registered type(s)" not in out:
    fails.append(
        f"the shipped module's registration count: {declared_on_disk} "
        f"registration(s) in qmldir but the gate did not report checking that "
        f"many — the walk is narrower than the file\n{out}")
else:
    print(f"ok: the shipped module — all {declared_on_disk} registration(s) checked")

if fails:
    print()
    for f in fails:
        print(f"FAIL: {f}", file=sys.stderr)
    print(f"check_qml_reachable: {len(fails)} case(s) failed", file=sys.stderr)
    sys.exit(1)
print("check_qml_reachable: all cases passed")
