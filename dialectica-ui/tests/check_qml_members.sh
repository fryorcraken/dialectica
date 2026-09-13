#!/usr/bin/env sh
# Every member a QML file reads off one of our own types must exist.
#
# Run: dialectica-ui/tests/check_qml_members.sh [qml-dir]
#      (qml-dir defaults to the `src/qml` beside this file)
#
# WHAT THIS CLOSES, and it is a gap three other gates were green over. With
# `color: DTheme.desk` in Main.qml rewritten as `DTheme.noSuchDesk` — a typo in
# the binding that paints the whole screen's ground — every existing gate
# passed:
#
#   * the QML suite          exit 0, 41 passed. No spec instantiates Main.qml,
#                            so `check_bindings` never sees it. The suite's
#                            claim to fail on any undefined binding is true
#                            only of components a spec instantiates, and
#                            Main.qml is not one.
#   * the static name gate   exit 0, 17 files. A D-PREFIXED typo contains no
#                            bare `Theme`; the gate checks names, not members.
#   * qmllint as configured  exit 0 — while PRINTING
#                            `Warning: Main.qml:36:23: Member "noSuchDesk" not
#                            found on type "DTheme" [missing-property]`.
#
# The third is the one worth staring at: qmllint already SAW the defect and was
# configured not to fail on it. `--missing-property error` escalates that one
# category, and it reaches every file in src/qml rather than only the ones a
# spec happens to construct. Measured both ways: exit 255 with the typo, exit 0
# on the unmutated tree.
#
# WHAT IT CANNOT SEE, stated here because a gate whose green is mistaken for a
# broader guarantee is worse than no gate — this repo has paid for that once.
#
#   * NOT the host collision. CI passes `-I <qml-dir>`, which puts OUR OWN
#     singleton on the import path, so qmllint resolves `DTheme` to the correct
#     file where every member genuinely exists. It is checking a DIFFERENT
#     RESOLUTION than the app performs. A green here says nothing whatsoever
#     about whether basecamp shadows a name — that remains the static name
#     gate's job, and ultimately a real launch's.
#   * NOT a wrong-but-defined value. `DTheme.ink` where `DTheme.inkSoft` was
#     meant is two valid members; nothing static can tell them apart.
#
# It closes the UNDEFINED-MEMBER class, which is the class the piece claimed to
# have closed and had closed only for instantiated components.
#
# WHY A SCRIPT AND NOT A FLAG IN ci.yml. A gate inline in a workflow `run:`
# block cannot be called, so it cannot be tested, so both defects in the
# previous name gate shipped through review. `tst_check_qml_members.sh` beside
# this file pins both directions.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
qml_dir=${1:-$here/../src/qml}

# Named candidates, never a bare `qmllint` first: a bare invocation may be the
# Qt5 binary, which rejects `--unqualified` outright ("Unknown option") and
# exits non-zero for a reason that has nothing to do with the code. The bare
# name stays last as a fallback.
linter=""
for cand in qmllint6 /usr/lib64/qt6/bin/qmllint \
            /usr/lib/qt6/bin/qmllint \
            /usr/lib/x86_64-linux-gnu/qt6/bin/qmllint qmllint; do
    if command -v "$cand" >/dev/null 2>&1 || [ -x "$cand" ]; then
        linter="$cand"
        break
    fi
done
[ -n "$linter" ] || { echo "::error::no qmllint found" >&2; exit 1; }

# Prove the chosen binary understands the flags before trusting a pass from it.
# A linter that exits non-zero on its own options would otherwise be
# indistinguishable from one reporting a real defect — and, worse, a linter that
# ignored an unknown flag would report a green that measured nothing.
#
# NAME THE REAL REQUIREMENT, not the first guess. This message used to say "a
# Qt5 qmllint?", and that sent a reader after the wrong thing: the case that
# actually fired was Qt **6.4.2**, the qmllint Ubuntu 24.04 ships, which is Qt6
# and still has no `--missing-property` command-line flag. Someone checking "is
# this Qt6?" gets `yes` and is no further forward. The requirement is a VERSION
# carrying that flag, so say which, and say how to check.
#
# Both flags go in one invocation, so a failure cannot say which was rejected —
# and the two are not equally likely. `--unqualified` has existed since well
# before 6.4; `--missing-property` arrived in **6.5** (checked in qtdeclarative:
# absent from the category list at tag v6.4.2, where the near-equivalent is
# named `--property`; present at v6.5.0 and at v6.8.3). So on a too-old Qt it is
# effectively always `--missing-property` that is missing, and the message says
# that rather than implying both are suspect.
if ! "$linter" --unqualified disable --missing-property error --help \
        >/dev/null 2>&1; then
    echo "::error::$linter does not accept --unqualified/--missing-property" >&2
    echo "         Almost certainly --missing-property: it arrived in Qt 6.5." >&2
    echo "         Being Qt6 is NOT enough. Qt 6.4.2 (Ubuntu 24.04's" >&2
    echo "         qt6-declarative-dev-tools) is Qt6 and still too old; it" >&2
    echo "         calls the near-equivalent category --property instead." >&2
    echo "         CI pins 6.8.3; this gate is also proven at 6.10.3." >&2
    echo "         Check with: $linter --version" >&2
    echo "         and:        $linter --help   (look for --missing-property)" >&2
    exit 1
fi

# The corpus, and the thing most worth asserting on. A glob that silently stops
# matching leaves this gate reporting clean over an unchecked tree, which is the
# failure mode the name gate beside it was found to have.
set -- "$qml_dir"/*.qml
if [ "$#" -eq 0 ] || [ ! -e "$1" ]; then
    echo "::error::no QML files in $qml_dir — this gate measured nothing" >&2
    exit 1
fi

# `--unqualified disable` is the one suppression and it names what it ignores:
# `logos` is injected into the QML context by the host, so it is by construction
# not declared in QML scope. `--missing-property error` is the escalation this
# gate exists for.
set +e
"$linter" --unqualified disable --missing-property error -I "$qml_dir" "$@"
rc=$?
set -e

if [ "$rc" -ne 0 ]; then
    echo "::error::a QML file reads a member that does not exist on the type" >&2
    echo "         (see the [missing-property] line above; exit $rc)" >&2
    exit 1
fi

echo "ok: $# QML file(s) checked, every member read off a known type exists"
