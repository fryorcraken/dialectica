#!/usr/bin/env sh
# Pins BOTH directions of check_probe_twins.sh: what it must catch, and what it
# must not complain about.
#
# A check narrowed to nothing passes as quietly as a correct one, so the cases
# below include a broken-corpus case — the gate must FAIL rather than report
# clean when it cannot read what it is meant to compare.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
gate="$here/check_probe_twins.sh"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0

# Run the gate over a staged pair and assert its exit status.
#
# `want` is "ok" or "reject". The gate's OUTPUT is captured and echoed only on
# an unexpected result, so a passing run stays quiet and a failing one shows
# what the gate actually said.
check() {
    name=$1
    want=$2
    rc=0
    out=$("$gate" "$work/healthy.qml" "$work/shadowed.qml" 2>&1) || rc=$?
    if [ "$want" = "ok" ] && [ "$rc" -eq 0 ]; then
        pass=$((pass + 1)); return 0
    fi
    if [ "$want" = "reject" ] && [ "$rc" -ne 0 ]; then
        pass=$((pass + 1)); return 0
    fi
    fail=$((fail + 1))
    echo "FAIL: $name — wanted $want, gate exited $rc" >&2
    echo "$out" >&2
    return 0
}

write_healthy() {
    cat > "$work/healthy.qml" <<'EOF'
import QtQuick

// a comment that only the healthy twin carries
Item {
    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
EOF
}

# ---- the case that must pass -------------------------------------------

write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick

// a DIFFERENT comment, and more of them, which must not count as a difference
// because each twin explains its own half of the demonstration
Item {
    property var data

    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
EOF
check "a correct pair differing by the shadow line alone" ok

# Indentation must not count as a difference either.
write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick
Item {
        property var data
        Rectangle {
                anchors.fill: parent
                color: "#1e3a5f"
        }
        Rectangle {
                x: 10
                y: 10
                width: 30
                height: 30
                color: "#f0c674"
        }
}
EOF
check "a reindented shadowed twin is still the same code" ok

# ---- the cases that must be rejected ------------------------------------

# THE ONE THAT MATTERS: a second difference. The pair would still render two
# verdicts, but the colour change is a rival explanation for why.
write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick
Item {
    property var data

    Rectangle {
        anchors.fill: parent
        color: "#990000"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
EOF
check "a shadowed twin whose colours also differ" reject

# The defect removed: the twins are identical, so the demonstration shows
# nothing at all. The probe's failing test would go green.
write_healthy
cp "$work/healthy.qml" "$work/shadowed.qml"
check "an identical pair, the defect deleted" reject

# The shadow present but a line also ADDED alongside it.
write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick
Item {
    property var data
    property int extra: 7

    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
EOF
check "two added lines rather than one" reject

# A different one-line difference that is NOT the default-property shadow. The
# count is right and the verdicts might even differ; the mechanism would not be
# the one the demonstration claims.
write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick
Item {
    visible: false

    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
EOF
check "a one-line difference that is not the shadow" reject

# The healthy twin carrying code the shadowed one lacks — the difference in the
# other direction.
write_healthy
cat > "$work/shadowed.qml" <<'EOF'
import QtQuick
Item {
    property var data

    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
}
EOF
check "the shadowed twin missing code the healthy one has" reject

# BROKEN CORPUS. An empty file compared against an empty file differs by nothing
# — a gate that reported that as clean would be measuring nothing at all.
: > "$work/healthy.qml"
: > "$work/shadowed.qml"
check "two empty files are a broken corpus, not a clean result" reject

# One side empty.
write_healthy
: > "$work/shadowed.qml"
check "an empty shadowed twin is a broken corpus" reject

# ---- and the real files, as shipped -------------------------------------

rc=0
out=$("$gate" 2>&1) || rc=$?
if [ "$rc" -eq 0 ]; then
    pass=$((pass + 1))
else
    fail=$((fail + 1))
    echo "FAIL: the twins actually shipped in this repo do not satisfy the gate" >&2
    echo "$out" >&2
fi

echo "check_probe_twins: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
