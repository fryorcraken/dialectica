#!/usr/bin/env sh
# The render probe's demonstration pair must differ by EXACTLY ONE LINE of code.
#
# WHY THIS GATE EXISTS. `tst_render_probe.qml` proves it can fail by rendering
# two subjects — `RenderProbeHealthy.qml` and `RenderProbeShadowed.qml` — and
# showing the first paints while the second is flat. The spec requires those two
# subjects to differ ONLY in whether their declared children reach the scene
# graph, because a pair differing in anything else (different content, size or
# colours) leaves two explanations for the difference in verdict and establishes
# neither.
#
# Nothing in the QML suite can check that. The two tests would still show two
# verdicts after someone edited one twin's colours, and would still pass — while
# no longer demonstrating WHY the verdicts differ. That is the silent decay this
# gate prevents.
#
# WHY NOT IN THE QML SPEC, which is where it was first written. QML blocks
# XMLHttpRequest against a local file unless QML_XHR_ALLOW_FILE_READ=1 is set,
# and an environment-variable prefix is a shape this repo's permission checker
# cannot statically analyse — CLAUDE.md's "what Bash costs" covers it. The
# property is static anyway, so a shell gate is the right layer.
#
# COMMENTS ARE EXCLUDED, deliberately. The twins carry different explanatory
# comments and SHOULD — each explains its own half of the demonstration. The
# requirement is one line of CODE.
#
# `tst_check_probe_twins.sh` beside this file pins both directions.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
healthy=${1:-"$here/RenderProbeHealthy.qml"}
shadowed=${2:-"$here/RenderProbeShadowed.qml"}

fail() {
    echo "probe twins: $1" >&2
    exit 1
}

for f in "$healthy" "$shadowed"; do
    [ -f "$f" ] || fail "no such file: $f"
done

# Code lines only: blanks and whole-line comments dropped, indentation
# normalised so a reindent is not read as a difference.
code_lines() {
    sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' "$1" \
        | grep -v '^$' \
        | grep -v '^//'
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

code_lines "$healthy" > "$tmp/healthy"
code_lines "$shadowed" > "$tmp/shadowed"

# A comparison over two empty files would pass while comparing nothing — the
# exact false green this repo keeps finding. Both sides must be non-trivial.
h_count=$(grep -c '' "$tmp/healthy" || true)
s_count=$(grep -c '' "$tmp/shadowed" || true)
[ "$h_count" -ge 5 ] || fail "healthy twin has only $h_count code lines; the corpus is broken, not clean"
[ "$s_count" -ge 5 ] || fail "shadowed twin has only $s_count code lines; the corpus is broken, not clean"

# Lines present in the shadowed twin but not the healthy one, and vice versa.
only_shadowed=$(diff "$tmp/healthy" "$tmp/shadowed" | grep '^>' | sed 's/^> //' || true)
only_healthy=$(diff "$tmp/healthy" "$tmp/shadowed" | grep '^<' | sed 's/^< //' || true)

if [ -n "$only_healthy" ]; then
    echo "probe twins: the healthy twin has code the shadowed one lacks." >&2
    echo "             The pair must differ ONLY by the shadowing line, or the" >&2
    echo "             demonstration has two explanations instead of one." >&2
    echo "$only_healthy" >&2
    exit 1
fi

n=$(printf '%s\n' "$only_shadowed" | grep -c '' || true)
if [ "$only_shadowed" = "" ]; then
    fail "the twins are identical — the shadowed one is missing its defect, so the probe's failure demonstration proves nothing"
fi
if [ "$n" -ne 1 ]; then
    echo "probe twins: the shadowed twin adds $n lines; it must add exactly 1." >&2
    echo "$only_shadowed" >&2
    exit 1
fi
if [ "$only_shadowed" != "property var data" ]; then
    fail "the one added line must be 'property var data' (Item's default property, whose shadowing is the defect being demonstrated), got: $only_shadowed"
fi

echo "probe twins: ok — the pair differs by exactly 'property var data'"
