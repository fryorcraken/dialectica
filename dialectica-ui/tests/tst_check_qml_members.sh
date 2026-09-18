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

# ---- MUST REJECT: a linter whose LEVEL the gate cannot use --------------
#
# THE CASE THAT SHIPPED BROKEN, and the reason the preflight is no longer a
# `--help` probe. The gate used `--missing-property error`, and `error` is a
# LEVEL VALUE that arrived AFTER the category did: Qt 6.8.3 — the version CI
# pins — accepts `--missing-property` and rejects `error`, offering only
# disable/info/warning. So the gate was unrunnable on the pinned Qt while
# passing at 6.10.3, where a fourth level exists.
#
# The old preflight could not see that, and the reason is worth keeping: it
# appended `--help`, which makes Qt's argument parser print usage and exit 0
# BEFORE any level value is validated. Measured — `--missing-property
# totalGibberish --help` exits 0 on a linter that rejects `totalGibberish`
# outright when asked to lint a file. A probe that proves a FLAG NAME is
# accepted proves nothing about the VALUE, and the value is what broke.
#
# So the probe must lint something, and this case pins THAT SPECIFICALLY — not
# merely that the gate goes red, which is the trap it took two attempts to get
# out of. A stub rejecting the level fails the gate twice over: once in the
# preflight and again in the real lint run, because both pass the same level. A
# case asserting only on the exit code cannot tell those apart, so it passes
# identically with the broken `--help` preflight restored — measured, and it is
# exactly this repo's "two explanations, same answer" defect family.
#
# The discriminator is therefore the MESSAGE. A gate whose preflight caught it
# says so before linting anything; a gate that sailed past the preflight reports
# a missing member instead, which on a tree where every member exists is a lie.
# The stub is written against the SHAPE of the defect rather than the token
# `error` — the gate no longer passes `error`, and a stub pinned to that token
# would go quietly inert — so it rejects EVERY level except `disable`, and, like
# the real binary, exits 0 for `--help` without validating anything.
stub_dir=$work/stub
mkdir -p "$stub_dir"
cat > "$stub_dir/qmllint" <<'STUB'
#!/usr/bin/env sh
for a in "$@"; do
    [ "$a" = "--help" ] && { echo "Usage: qmllint [options] files"; exit 0; }
done
prev=""
for a in "$@"; do
    case $prev in
        --missing-property|-W|--max-warnings)
            if [ "$a" != "disable" ]; then
                echo "Invalid logging level \"$a\" provided for \"$prev\"" >&2
                echo "Usage: qmllint [options] files" >&2
                exit 255
            fi
            ;;
    esac
    prev="$a"
done
exit 0
STUB
chmod +x "$stub_dir/qmllint"

# The stub is selected via `QMLLINT`, not by PATH order. PATH alone does not
# work: the gate tries absolute candidates BEFORE the bare name, so on any
# machine with a real qmllint installed this case would measure that one and
# pass for the wrong reason — a gate the defect satisfies.
#
# Prove the stub models the defect before drawing any conclusion from it: it
# must REJECT the level when linting a file, and ACCEPT `--help`. A stub broken
# some other way (not executable, bad shebang) would fail the gate too, and the
# case would look green while measuring nothing.
stub_models_it=1
if "$stub_dir/qmllint" --missing-property warning "$work/anything.qml" \
        >/dev/null 2>&1; then
    echo "FAIL: the stub ACCEPTED an escalation level while linting, so it" >&2
    echo "      does not stand in for a Qt that rejects one" >&2
    stub_models_it=0
fi
if ! "$stub_dir/qmllint" --missing-property warning --help >/dev/null 2>&1; then
    echo "FAIL: the stub rejects --help, so it does not model the" >&2
    echo "      short-circuit that let the defect through — a --help" >&2
    echo "      preflight would 'catch' it for the wrong reason" >&2
    stub_models_it=0
fi

if [ "$stub_models_it" -eq 0 ]; then
    fails=$((fails + 1))
else
    # `build good desk` is a tree where every member EXISTS. So "a QML file
    # reads a member that does not exist" is necessarily false here, and a gate
    # emitting it has skipped its preflight and misattributed the stub's refusal
    # to the code under test. That is the assertion; the exit code is not.
    out=$(QMLLINT="$stub_dir/qmllint" "$gate" "$(build good desk)" 2>&1) \
        && rc=0 || rc=1
    if [ "$rc" -eq 0 ]; then
        echo "FAIL: a linter that rejects the escalation LEVEL was treated" >&2
        echo "      as usable — this is how the 6.8.3 break shipped" >&2
        fails=$((fails + 1))
    elif printf '%s' "$out" | grep -q 'does not exist on the type'; then
        echo "FAIL: the gate blamed a MISSING MEMBER on a tree where every" >&2
        echo "      member exists — it ran the linter without proving the" >&2
        echo "      linter could run, so the preflight is not probing the" >&2
        echo "      level (a --help probe exits 0 without validating one)" >&2
        printf '%s\n' "$out" >&2
        fails=$((fails + 1))
    else
        echo "ok: a linter rejecting the escalation level — rejected upfront"
    fi
fi

# ---- MUST REJECT: an empty corpus ---------------------------------------
#
# THE HARDEST CASE, and the reason the three above are not enough. The gate
# runs over a glob; if the glob silently stops matching, every check passes
# vacuously and the gate reports `ok` — a green that measured nothing. PLAN.md
# §10 (CI), under "Anti-false-green", takes from Radicle's CI the observation
# that a green gate which cannot see the thing it claims to check is worse than
# no gate, and says to copy the habit. So an empty corpus must produce a
# FAILURE, not a clean bill.
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
