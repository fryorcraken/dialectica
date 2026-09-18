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
# configured not to fail on it. `--missing-property warning -W 0` escalates that
# warning into a failure, and it reaches every file in src/qml rather than only
# the ones a spec happens to construct. Measured both ways: exit 255 with the
# typo, exit 0 on the unmutated tree.
#
# WHAT IT CANNOT SEE, stated here because a gate whose green is mistaken for a
# broader guarantee is the anti-false-green problem PLAN.md §10 (CI) describes
# — and this repo has paid for that once.
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
#
# `QMLLINT` overrides the search, and it exists so the PREFLIGHT ITSELF can be
# tested. Without it, `tst_check_qml_members.sh` cannot hand this gate a linter
# that rejects the escalation: putting a stub first on PATH does not work,
# because the absolute candidates below are tried before the bare name, so the
# case would silently measure the machine's real qmllint and pass for the wrong
# reason — a gate the defect satisfies. It is not a production knob; CI sets it
# nowhere and the search below is what runs there.
linter=""
if [ -n "${QMLLINT:-}" ]; then
    linter=$QMLLINT
else
    for cand in qmllint6 /usr/lib64/qt6/bin/qmllint \
                /usr/lib/qt6/bin/qmllint \
                /usr/lib/x86_64-linux-gnu/qt6/bin/qmllint qmllint; do
        if command -v "$cand" >/dev/null 2>&1 || [ -x "$cand" ]; then
            linter="$cand"
            break
        fi
    done
fi
[ -n "$linter" ] || { echo "::error::no qmllint found" >&2; exit 1; }

# Prove the chosen binary can actually RUN the invocation below before trusting
# a pass from it. A linter that exits non-zero on its own options would
# otherwise be indistinguishable from one reporting a real defect — and, worse,
# a linter that ignored an unknown flag would report a green that measured
# nothing.
#
# THE PROBE MUST LINT A FILE. It used to append `--help`, and that is how a gate
# unrunnable on CI's Qt shipped green from a developer machine: `--help` makes
# Qt's argument parser print usage and exit 0 BEFORE it validates any level
# VALUE. Measured — `--missing-property totalGibberish --help` exits 0 on a
# linter that rejects `totalGibberish` outright when asked to lint a file. A
# probe of the flag NAME says nothing about the VALUE.
#
# NAME THE REAL REQUIREMENT, not the first guess. This message used to say "a
# Qt5 qmllint?", and that sent a reader after the wrong thing: the case that
# actually fired was Qt **6.4.2**, the qmllint Ubuntu 24.04 ships, which is Qt6
# and still has no `--missing-property` command-line flag. Someone checking "is
# this Qt6?" gets `yes` and is no further forward.
#
# Both flags go in one invocation, so a failure cannot say which was rejected —
# and the two are not equally likely. `--unqualified` has existed since well
# before 6.4; `--missing-property` arrived in **6.5** (checked in qtdeclarative:
# absent from the category list at tag v6.4.2, where the near-equivalent is
# named `--property`; present at v6.5.0 and at v6.8.3). So on a too-old Qt it is
# effectively always `--missing-property` that is missing.

# The probe file is a minimal VALID QML document, written fresh rather than
# borrowed from the tree. Pointing the probe at a real view file would make a
# genuine defect in that file look like an unusable linter; pointing it at this
# script would fail on every Qt, since a shell script is not QML (measured:
# exit 255, `Unexpected token` — a permanently red gate).
probe_dir=$(mktemp -d)
trap 'rm -rf "$probe_dir"' EXIT INT TERM
printf 'import QtQuick\nItem {}\n' > "$probe_dir/Probe.qml"

if ! "$linter" --unqualified disable --missing-property warning -W 0 \
        "$probe_dir/Probe.qml" >/dev/null 2>&1; then
    echo "::error::$linter cannot run this gate's qmllint invocation" >&2
    echo "         Probed by LINTING A FILE, not by --help, because --help" >&2
    echo "         exits 0 without validating a level value." >&2
    echo "         Most likely --missing-property: it arrived in Qt 6.5." >&2
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
# not declared in QML scope.
#
# THE ESCALATION IS `-W 0`, NOT THE LEVEL `error`, and the difference is a red
# CI run. `--missing-property error` works at 6.10.3 and is REJECTED at 6.8.3,
# which CI pins: the level `error` arrived after the category did, and 6.8.3
# offers only disable/info/warning. The gate was therefore green on a developer
# machine and unrunnable on CI.
#
# `-W 0` — fail if more than zero warnings — reaches the same place through a
# flag both versions have, so there is ONE code path rather than a
# version-conditional one. That matters more than it looks: a fallback branch
# taken only on CI is the branch nobody runs locally, which is the arrangement
# most likely to rot.
#
# WHAT THIS WIDENS, said plainly because the gate's name no longer covers it.
# `-W 0` fails on ANY warning qmllint emits, not only `missing-property`. That
# is a deliberate trade, on two grounds: the shipped tree is measured clean of
# every other category, so it costs nothing today; and the alternative — naming
# every other category `disable` — is a hand-maintained sweep list, correct only
# until qmllint adds a category, with nothing able to notice. A gate STRICTER
# than its name fails loudly and gets fixed; one looser than its name is the
# failure this whole file exists to prevent. If an unrelated category ever does
# fire here, addressing it or disabling that one category by name keeps the
# guarantee; raising the `-W` ceiling instead would restore the silent-pass
# defect this gate was written to close, so weigh that cost before doing it.
set +e
"$linter" --unqualified disable --missing-property warning -W 0 \
    -I "$qml_dir" "$@"
rc=$?
set -e

if [ "$rc" -ne 0 ]; then
    echo "::error::a QML file reads a member that does not exist on the type" >&2
    echo "         (see the [missing-property] line above; exit $rc)" >&2
    echo "         If the line above names a DIFFERENT category, that is this" >&2
    echo "         gate's -W 0 firing wider than its name — read the category" >&2
    echo "         and fix that, rather than assuming a missing member." >&2
    exit 1
fi

echo "ok: $# QML file(s) checked, every member read off a known type exists"
