#!/usr/bin/env sh
# Tests for `check_qml_members.sh`, the gate that fails on a QML file reading a
# member that does not exist on one of our own types.
#
# WHY THIS FILE EXISTS. The gate it tests was found because the equivalent check
# lived as a flag inside a workflow `run:` block, where nothing could call it —
# and both defects in this repo's previous name gate shipped through review for
# exactly that reason. A gate is part of the measurement. One narrowed until it
# matches nothing passes every run as quietly as a correct one, so BOTH bounds
# are pinned here: the gate must accept a sound tree and reject a broken one.
#
# WHAT THE REJECTION CASE IS, and why it is the shape it is. The defect that
# motivated the gate is a D-PREFIXED typo — `DTheme.noSuchDesk` where
# `DTheme.desk` was meant. That spelling contains no bare `Theme`, so the static
# name gate cannot see it; and it sits in `Main.qml`, which no spec
# instantiates, so `check_bindings` cannot see it either. Measured: with that
# one character sequence changed, the QML suite, the static name gate AND
# qmllint as previously configured were all green.
#
# Run directly: dialectica-ui/tests/tst_check_qml_members.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
gate="$here/check_qml_members.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fails=0

# The real DTheme is copied in rather than a stand-in written here, so the
# member names these cases assert on are the ones the view actually ships. A
# hand-written stub would keep passing after a token was renamed out of
# existence — the test would then be pinning its own fixture, not the view.
build() {
    dir=$work/$1
    mkdir -p "$dir"
    cp "$here/../src/qml/DTheme.qml" "$dir/"
    printf 'singleton DTheme 1.0 DTheme.qml\n' > "$dir/qmldir"
    printf 'import QtQuick\nRectangle { color: DTheme.%s }\n' "$2" \
        > "$dir/Probe.qml"
    printf '%s' "$dir"
}

# ---- MUST ACCEPT --------------------------------------------------------
#
# The half that carries the weight. The rejection case below is also satisfied
# by a gate that rejects everything; only this pins that it does not — and a
# gate red on correct input is one somebody turns off.
if "$gate" "$(build good desk)" >/dev/null 2>&1; then
    echo "ok: a member that exists on DTheme — accepted"
else
    echo "FAIL: the gate REJECTED a tree where every member exists" >&2
    fails=$((fails + 1))
fi

# The real shipped view. If the rule is one our own code cannot satisfy, that is
# a defect in the gate and it fails here rather than in CI after a Qt install.
#
# AND that the glob actually reached it, which exit 0 does not say. Narrowing
# the corpus to a single file leaves this case green — measured — because one
# harmless file has no missing member either. The floor is COUNTED FROM DISK
# here rather than read back out of the gate's own summary: asking the gate how
# many files it saw and agreeing with the answer is the implementation
# confirming itself, not a measurement.
on_disk=0
for f in "$here"/../src/qml/*.qml; do
    [ -e "$f" ] || break
    on_disk=$((on_disk + 1))
done
shipped=$("$gate" 2>&1) && rc=0 || rc=1
if [ "$rc" -ne 0 ]; then
    echo "FAIL: the gate REJECTED the shipped view" >&2
    printf '%s\n' "$shipped" >&2
    fails=$((fails + 1))
elif ! printf '%s' "$shipped" | grep -q "^ok: $on_disk QML file(s)"; then
    echo "FAIL: $on_disk .qml file(s) in src/qml but the gate did not report" >&2
    echo "      checking that many — the glob is narrower than the directory," >&2
    echo "      so most files went unlinted" >&2
    printf '%s\n' "$shipped" >&2
    fails=$((fails + 1))
else
    echo "ok: the shipped src/qml — all $on_disk file(s) reached"
fi

# ---- MUST REJECT --------------------------------------------------------
#
# The D-prefixed typo. Note what this case does NOT do: it never asks the gate
# what it found and agrees. The member name is chosen here, the expectation is
# written here, and a gate that stopped checking members fails this.
out=$("$gate" "$(build typo noSuchDeskAtAll)" 2>&1) && rc=0 || rc=1
if [ "$rc" -eq 0 ]; then
    echo "FAIL: the gate ACCEPTED a member that does not exist on DTheme" >&2
    fails=$((fails + 1))
elif ! printf '%s' "$out" | grep -q 'noSuchDeskAtAll'; then
    echo "FAIL: rejected, but the message never names the offending member —" >&2
    echo "      a red nobody can diagnose is a red somebody suppresses" >&2
    printf '%s\n' "$out" >&2
    fails=$((fails + 1))
else
    echo "ok: a member that does not exist on DTheme — rejected"
fi

# ---- MUST REJECT: an empty corpus ---------------------------------------
#
# THE HARDEST CASE, and the reason the three above are not enough. The gate
# runs over a glob; if the glob silently stops matching, every check passes
# vacuously and the gate reports `ok` — a green that measured nothing, which
# this repo rates as worse than no gate because it closes the question. So an
# empty corpus must produce a FAILURE, not a clean bill.
empty=$work/empty
mkdir -p "$empty"
if "$gate" "$empty" >/dev/null 2>&1; then
    echo "FAIL: the gate reported clean over a directory with no QML files" >&2
    fails=$((fails + 1))
else
    echo "ok: a directory with no QML files — rejected"
fi

if [ "$fails" -ne 0 ]; then
    echo "check_qml_members: $fails case(s) failed" >&2
    exit 1
fi
echo "check_qml_members: all cases passed"
