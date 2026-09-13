#!/usr/bin/env sh
# Tests for check-claimed-deletions.sh.
#
# WHY THIS EXISTS AT ALL. A CI check is normally reachable only by pushing a
# branch, which makes "prove it can fail" cost a round trip per case — and makes
# the shallow-clone case unprovable without deliberately breaking the workflow
# and pushing that. Each test here builds a throwaway repository in a temp
# directory and runs the real script against it, so every branch of the gate is
# exercised in about a second.
#
# The shallow cases (5, 6) are the reason the script is a file rather than an
# inline `run:` block: they need a genuinely shallow clone, which
# `git clone --depth` produces and an inline block cannot be pointed at.
#
# Run: sh .github/scripts/tests/test-check-claimed-deletions.sh
set -u

here=$(cd "$(dirname "$0")" && pwd)
script="$here/../check-claimed-deletions.sh"
[ -x "$script" ] || { echo "FATAL: $script is not executable"; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

passes=0
failures=0

# report <name> <expected-status> <actual-status> <output>
report() {
    if [ "$2" = "$3" ]; then
        passes=$((passes + 1))
        printf 'ok   %s (exit %s)\n' "$1" "$3"
    else
        failures=$((failures + 1))
        printf 'FAIL %s: expected exit %s, got %s\n' "$1" "$2" "$3"
        printf '     output:\n'
        printf '%s\n' "$4" | sed 's/^/     | /'
    fi
}

# Build an origin repository with:
#   main:    c0 .. c3        (keep.txt grows)
#   feature: c0 + a commit deleting doomed.txt and adding added.txt
# so feature forks at c0 and main has moved on — the exact shape that makes the
# two-dot and three-dot ranges disagree.
build_origin() {
    origin="$work/origin"
    rm -rf "$origin"
    mkdir -p "$origin/sub"
    cd "$origin" || exit 1
    git init -q -b main .
    git config user.email test@example.invalid
    git config user.name "Test"
    echo a > keep.txt
    echo b > sub/doomed.txt
    echo c > also-doomed.txt
    git add keep.txt sub/doomed.txt also-doomed.txt
    git commit -qm c0
    for n in 1 2 3; do
        echo "main line $n" >> keep.txt
        git commit -qam "main c$n"
    done
    git checkout -q -b feature main~3
    git rm -q sub/doomed.txt
    echo new > added.txt
    git add added.txt
    git commit -qm "feature: delete sub/doomed.txt, add added.txt"
    git checkout -q main
}

# clone_full <dest> / clone_shallow <dest> <depth>
clone_full() {
    rm -rf "$1"
    git clone -q "file://$work/origin" "$1"
    cd "$1" || exit 1
    git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
}

clone_shallow() {
    rm -rf "$1"
    git clone -q --depth "$2" --no-single-branch "file://$work/origin" "$1"
    cd "$1" || exit 1
}

# run_gate <body-text> ; runs in the cwd, against origin/main...origin/feature
run_gate() {
    printf '%s' "$1" > "$work/body.txt"
    out=$(sh "$script" origin/main origin/feature "$work/body.txt" 2>&1)
    status=$?
    printf '%s' "$out" > "$work/last-output.txt"
    return $status
}

build_origin

# ── 1. Fails on an unclaimed deletion ─────────────────────────────────────
# The headline case: the branch deletes a file and the body never says so.
clone_full "$work/c1"
run_gate "Adds a thing.

No mention of any deletion here."
st=$?
report "unclaimed deletion fails" 1 "$st" "$(cat "$work/last-output.txt")"
# and it must name the path, not just fail
if grep -q 'sub/doomed.txt' "$work/last-output.txt"; then
    passes=$((passes + 1)); echo "ok   failure names the deleted path"
else
    failures=$((failures + 1)); echo "FAIL failure did not name sub/doomed.txt"
fi

# ── 2. Passes when the deletion is claimed ────────────────────────────────
clone_full "$work/c2"
run_gate "Removes the doomed file.

Deletes: sub/doomed.txt
"
st=$?
report "claimed deletion passes" 0 "$st" "$(cat "$work/last-output.txt")"

# ── 3. A clean branch passes ──────────────────────────────────────────────
# Guards against a gate that fails on everything, which would pass test 1 for
# the wrong reason.
clone_full "$work/c3"
cd "$work/c3" || exit 1
git checkout -q -b clean origin/main
echo x > brand-new.txt
git add brand-new.txt
git -c user.email=t@t -c user.name=t commit -qm "add only"
printf 'nothing deleted' > "$work/body.txt"
out=$(sh "$script" origin/main clean "$work/body.txt" 2>&1)
st=$?
report "a branch deleting nothing passes" 0 "$st" "$out"

# ── 4. Three dots, not two ────────────────────────────────────────────────
# THE REGRESSION TEST FOR THE 6,871-DELETION FALSE ALARM, and the mechanism is
# the opposite of the obvious one — this test was written the wrong way round
# first, passed under a deliberately two-dot implementation, and is recorded
# here so it is not rewritten back.
#
# `--diff-filter=D` on `git diff base head` asks "what does HEAD lack that BASE
# has". So the file that gets misattributed is one main *GAINED* after the
# fork: the branch predates it, therefore lacks it, therefore two-dot scores it
# a deletion BY THE BRANCH. A file main *deleted* after the fork does not
# discriminate at all — the branch still has it, which reads as an addition.
#
# That is precisely the real failure: main gained the agent files, three
# branches predated them, and the two-dot diff blamed each branch for ~700
# deletions it never made.
#
# Fixture: main gains `arrived-later.txt` after feature forks. A two-dot gate
# demands the branch claim it; a three-dot gate never mentions it.
clone_full "$work/c4"
cd "$work/origin" || exit 1
git checkout -q main
echo later > arrived-later.txt
git add arrived-later.txt
git commit -qm "main adds arrived-later.txt after the fork"
cd "$work/c4" || exit 1
git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
run_gate "Deletes: sub/doomed.txt
"
st=$?
report "a file main gained after the fork is not blamed on the branch" 0 "$st" \
    "$(cat "$work/last-output.txt")"
if grep -q 'arrived-later.txt' "$work/last-output.txt"; then
    failures=$((failures + 1))
    echo "FAIL two-dot leak: the gate blamed the branch for arrived-later.txt,"
    echo "     which main added after the fork — this is the false alarm itself"
else
    passes=$((passes + 1)); echo "ok   arrived-later.txt not attributed to the branch"
fi

# ── 5. A shallow clone FAILS rather than passing ──────────────────────────
# THE CORE SAFETY TEST. With fetch-depth 1 the merge base is unresolvable. A
# naive implementation reports zero deletions and exits 0 — measured: three
# separate shell shapes do exactly that. This asserts the opposite, and
# asserts the message names the cause someone can act on.
clone_shallow "$work/c5" 1
run_gate "Deletes: sub/doomed.txt
"
st=$?
report "a shallow clone fails loudly" 1 "$st" "$(cat "$work/last-output.txt")"
if grep -q 'fetch-depth' "$work/last-output.txt"; then
    passes=$((passes + 1)); echo "ok   shallow failure names fetch-depth"
else
    failures=$((failures + 1)); echo "FAIL shallow failure did not name fetch-depth"
fi
# It must NOT be mistakable for the unclaimed-deletion failure: a reader who
# sees "unclaimed deletions" goes and edits the PR body, which fixes nothing.
if grep -q 'cannot measure' "$work/last-output.txt"; then
    passes=$((passes + 1)); echo "ok   shallow failure is distinguishable from a real finding"
else
    failures=$((failures + 1)); echo "FAIL shallow failure not distinguishable from a finding"
fi

# ── 6. A shallow clone fails EVEN WHEN the branch is clean ────────────────
# The sharpest version of case 5: here there is genuinely nothing to find, so
# a gate that measured nothing and a gate that measured correctly agree on the
# verdict. The gate must still refuse, because it did not look.
clone_shallow "$work/c6" 1
cd "$work/c6" || exit 1
printf 'no deletions in this branch at all' > "$work/body.txt"
out=$(sh "$script" origin/main origin/main "$work/body.txt" 2>&1)
st=$?
report "shallow + nothing to find still fails" 1 "$st" "$out"

# ── 7. An unresolvable ref fails, and is not reported as shallow ──────────
# The other way the range breaks. The shallow guard alone would call this
# healthy, so this is what stops the two guards from collapsing into one.
clone_full "$work/c7"
run_gate "Deletes: sub/doomed.txt
"
out=$(sh "$script" origin/no-such-branch origin/feature "$work/body.txt" 2>&1)
st=$?
report "an unresolvable base ref fails" 1 "$st" "$out"
case "$out" in
    *fetch-depth*)
        failures=$((failures + 1))
        echo "FAIL a missing ref was misreported as a shallow-clone problem" ;;
    *)
        passes=$((passes + 1)); echo "ok   missing ref diagnosed as a missing ref" ;;
esac

# ── 8. The claim must be a line, not a substring ──────────────────────────
# A PR body is prose, and prose may quote this very check. If `Deletes:`
# matched anywhere in a line, a body explaining the convention would
# acknowledge whatever path it used as an example.
clone_full "$work/c8"
run_gate "This PR adds a gate. To acknowledge a removal, write a line reading Deletes: sub/doomed.txt in the body.

But this PR is not claiming that deletion."
st=$?
report "an inline mention does not count as a claim" 1 "$st" \
    "$(cat "$work/last-output.txt")"

# ── 9. A list-marker claim counts ─────────────────────────────────────────
# PR bodies are Markdown and authors write bullets. Refusing `- Deletes: x`
# would fail a body that says exactly the right thing.
clone_full "$work/c9"
run_gate "Summary.

- Deletes: sub/doomed.txt
"
st=$?
report "a markdown bullet claim counts" 0 "$st" "$(cat "$work/last-output.txt")"

# ── 10. Claimed-but-not-deleted is a note, not a failure ──────────────────
# NO SPEC: nothing specifies what happens when the body claims a path the diff
# does not delete. This accepts it (a stale claim from an earlier push is not a
# defect worth blocking a merge on) and says so in the output. Recorded in
# design.md, Decisions §5.
clone_full "$work/c10"
run_gate "Deletes: sub/doomed.txt
Deletes: never-existed.txt
"
st=$?
report "NO SPEC: a claim with no matching deletion does not fail" 0 "$st" \
    "$(cat "$work/last-output.txt")"

# ── 11. An empty body with deletions fails ────────────────────────────────
# The PR-body env var is empty when a PR has no description at all, and an
# empty string is the value most likely to be mishandled.
clone_full "$work/c11"
run_gate ""
st=$?
report "an empty PR body does not excuse a deletion" 1 "$st" \
    "$(cat "$work/last-output.txt")"

# ── 12. A path with a space is matched exactly ────────────────────────────
# `git diff --name-only` prints such a path bare (it only quotes on control or
# non-ASCII characters), and any word-splitting in the script would break the
# comparison in the direction that PASSES.
clone_full "$work/c12"
cd "$work/origin" || exit 1
git checkout -q -b spaced main
echo s > "a file with spaces.txt"
git add "a file with spaces.txt"
git commit -qm "add spaced file"
git checkout -q -b spaced-del spaced
git rm -q "a file with spaces.txt"
git commit -qm "delete spaced file"
git checkout -q main
cd "$work/c12" || exit 1
git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
printf 'unclaimed' > "$work/body.txt"
out=$(sh "$script" origin/spaced origin/spaced-del "$work/body.txt" 2>&1)
st=$?
report "a deleted path containing spaces is caught" 1 "$st" "$out"
printf 'Deletes: a file with spaces.txt\n' > "$work/body.txt"
out=$(sh "$script" origin/spaced origin/spaced-del "$work/body.txt" 2>&1)
st=$?
report "a deleted path containing spaces can be claimed" 0 "$st" "$out"

# ── 13. A body that cannot be read fails ─────────────────────────────────
# If the workflow ever stops writing the body file, "no claims" and "no body"
# would otherwise be the same state — and the first is a legitimate PR.
clone_full "$work/c13"
out=$(sh "$script" origin/main origin/feature "$work/definitely-not-here.txt" 2>&1)
st=$?
report "a missing body file fails rather than reading as no claims" 1 "$st" "$out"

# ── 14. A CRLF body still matches ─────────────────────────────────────────
# GitHub delivers PR bodies with CRLF line endings. A trailing \r survives into
# the extracted claim and makes the exact comparison fail against a path that
# is otherwise identical — failing a CORRECT pull request. The trim that
# prevents this rides on one regex in the script, and nothing else exercises it.
clone_full "$work/c14"
printf 'Removes the file.\r\n\r\nDeletes: sub/doomed.txt\r\n' > "$work/body.txt"
out=$(sh "$script" origin/main origin/feature "$work/body.txt" 2>&1)
st=$?
report "a CRLF body's claim still matches" 0 "$st" "$out"

# ── 15. A claim is matched whole-line, not as a substring ─────────────────
# GET THE DIRECTION RIGHT. The lookup is `grep -qxF "$deleted_path"
# "$claims_file"` — the deleted path is the PATTERN and the claims are the
# HAYSTACK. So dropping `-x` makes a claim that CONTAINS the path satisfy it,
# not the other way round: `Deletes: sub/doomed.txt.bak` would acknowledge a
# deletion of `sub/doomed.txt`.
#
# A first version of this test asserted the reverse (a shorter claim matching a
# longer path) and stayed green under exactly the mutation it named — the
# defect family this repo catalogues, where two explanations give the same
# answer. Measured: with `-x` removed, the body below passes the gate.
clone_full "$work/c15"
run_gate "Deletes: sub/doomed.txt.bak
"
st=$?
report "a claim containing the path does not acknowledge it" 1 "$st" \
    "$(cat "$work/last-output.txt")"

# ── 16. The invocation the workflow actually uses ─────────────────────────
# Every test above passes symbolic refs. The workflow passes a raw base SHA
# (from github.event.pull_request.base.sha) and the literal `HEAD`. That is a
# different code path through `git rev-parse --verify "$ref^{commit}"` and
# through the `...` range, and testing only the symbolic form would leave the
# form that actually runs in CI unexercised.
clone_full "$work/c16"
cd "$work/c16" || exit 1
git checkout -q -b local origin/feature
base_sha=$(git merge-base origin/main origin/feature)
printf 'no claims' > "$work/body.txt"
out=$(sh "$script" "$base_sha" HEAD "$work/body.txt" 2>&1)
st=$?
report "a raw base SHA with HEAD is caught" 1 "$st" "$out"
printf 'Deletes: sub/doomed.txt\n' > "$work/body.txt"
out=$(sh "$script" "$base_sha" HEAD "$work/body.txt" 2>&1)
st=$?
report "a raw base SHA with HEAD can be claimed" 0 "$st" "$out"

printf '\n%s passed, %s failed\n' "$passes" "$failures"
[ "$failures" -eq 0 ]
