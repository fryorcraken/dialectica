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

# ── 17. `-F` is load-bearing: a claim is a literal, not a pattern ─────────
# REVIEW FINDING. Test 15 pinned `-x` and left `-F` unpinned, and 23 of 23 tests
# passed with `-F` removed. Without it the DELETED PATH becomes a basic regular
# expression, so `Deletes: sub/lib?rs` acknowledges a deletion of `sub/lib.rs`
# — attacker-supplied text reaching a pattern position, accepting a path the
# body never names.
#
# Two files are deleted and only one is absorbed by the crafted claim, so a pass
# here cannot be a coincidence of the gate failing for some other reason: under
# the mutation the output names only `sub/store.rs`.
clone_full "$work/c17"
cd "$work/origin" || exit 1
git checkout -q -b regex main
mkdir -p sub
echo l > sub/lib.rs
echo s > sub/store.rs
git add sub/lib.rs sub/store.rs
git commit -qm "add two rs files"
git checkout -q -b regex-del regex
git rm -q sub/lib.rs sub/store.rs
git commit -qm "delete both"
git checkout -q main
cd "$work/c17" || exit 1
git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
printf 'Deletes: sub/lib?rs\n' > "$work/body.txt"
out=$(sh "$script" origin/regex origin/regex-del "$work/body.txt" 2>&1)
st=$?
report "a regex metacharacter in a claim does not match a real path" 1 "$st" "$out"
# and specifically: lib.rs must still be reported unclaimed
case "$out" in
    *"sub/lib.rs"*)
        passes=$((passes + 1))
        echo "ok   sub/lib.rs still reported unclaimed under a regex-shaped claim" ;;
    *)
        failures=$((failures + 1))
        echo "FAIL sub/lib.rs was absorbed by the claim 'sub/lib?rs' — -F is not doing its job" ;;
esac

# ── 18. Guard 3 is reachable, and is not dead code ────────────────────────
# REVIEW FINDING, and it asked the right question: replacing guard 3 with the
# `|| true` idiom design.md §2 names as dangerous left the whole suite green,
# because no fixture reached it. The answer is that guard 3 is NOT unreachable
# — it defends a case the fixtures did not model.
#
# THE FIRST VERSION OF THIS TEST PASSED FOR THE WRONG REASON and is the third
# instance of this piece's recurring defect. It removed the base commit's ROOT
# tree, which in a fresh clone also makes `rev-parse --verify <sha>^{commit}`
# fail — so the script exited 1 at GUARD 2, the assertion went green, and the
# `|| true` mutation still passed 33 of 33. Measured per guard before rewriting:
# with the root tree gone, guard 2 fired; the exit status was identical either
# way, which is precisely why the test could not tell the two apart.
#
# What actually reaches guard 3 is removing a SUBTREE object. Measured, all
# three guards individually:
#
#   guard 2a  rev-parse --verify base^{commit}  exit 0   (peels a commit only)
#   guard 2a  rev-parse --verify HEAD^{commit}  exit 0
#   guard 2b  merge-base base HEAD              exit 0   (walks commits)
#   the diff  ...                               exit 128 (must read the tree)
#
# That is a partially-fetched or corrupted object store — a real thing a CI
# checkout can produce, and the case where every cheap check says the repo is
# healthy and only the diff disagrees.
#
# (Removing the BLOB of the deleted file does NOT reach it: `--name-only` never
# reads file contents, so the diff succeeds. Noted because it is the obvious
# fixture to reach for and it proves nothing.)
clone_full "$work/c18"
cd "$work/c18" || exit 1
git checkout -q -b local origin/feature
base_sha=$(git merge-base origin/main origin/feature)
# Make every object loose so one can be removed. `unpack-objects` REFUSES to
# write an object that already exists in a pack, so the pack is moved aside
# first and fed in from outside the object store — unpacking with the pack still
# in place is a silent no-op, which broke an earlier version of this fixture.
mkdir -p "$work/packs-c18"
for pack in .git/objects/pack/*.pack; do
    [ -e "$pack" ] || break
    mv "$pack" "$work/packs-c18/"
done
rm -f .git/objects/pack/*.idx
for pack in "$work/packs-c18"/*.pack; do
    [ -e "$pack" ] || break
    git unpack-objects -q < "$pack" 2>/dev/null
done
# Assert the clone is HEALTHY before corrupting it. Without this the fixture can
# silently break and the test then passes on a repo that was broken for an
# unrelated reason — the failure mode this very test already had once.
if ! git rev-parse --verify --quiet "origin/main^{commit}" > /dev/null; then
    failures=$((failures + 1))
    echo "FAIL test 18 fixture is broken: origin/main does not resolve after unpack"
else
    subtree_sha=$(git rev-parse "$base_sha:sub")
    subtree_dir=$(printf '%s' "$subtree_sha" | cut -c1-2)
    subtree_file=$(printf '%s' "$subtree_sha" | cut -c3-)
    rm -f ".git/objects/$subtree_dir/$subtree_file"

    # Guard 2 must still PASS, or this test is measuring guard 2 again.
    if git rev-parse --verify --quiet "$base_sha^{commit}" > /dev/null \
       && git merge-base "$base_sha" HEAD > /dev/null 2>&1; then
        passes=$((passes + 1))
        echo "ok   guard 2 still passes, so the next failure is guard 3's"
    else
        failures=$((failures + 1))
        echo "FAIL guard 2 fires on this fixture — test 18 is measuring the wrong guard"
    fi

    printf 'Deletes: sub/doomed.txt\n' > "$work/body.txt"
    out=$(sh "$script" "$base_sha" HEAD "$work/body.txt" 2>&1)
    st=$?
    report "an unreadable subtree fails at guard 3 rather than passing" 1 "$st" "$out"
    case "$out" in
        *"cannot measure"*"git diff"*)
            passes=$((passes + 1))
            echo "ok   guard 3 names the failing diff, not a missing ref" ;;
        *)
            failures=$((failures + 1))
            echo "FAIL guard 3 did not report a failing 'git diff'" ;;
    esac
fi

# ── 19. A rename is not reported as a deletion, whatever the config ───────
# REVIEW FINDING. The verdict depended on `diff.renames`, which the script did
# not pin: `git mv big.txt moved.txt` reported 0 deletions with git's default
# and 1 unclaimed deletion with `diff.renames false` in the repo config. A gate
# whose answer a developer's config changes is not a gate, and a rename is the
# obvious false-positive class.
#
# The config is set IN THE CLONE the script runs against, so this fails unless
# the script pins `--find-renames` on its own invocation.
clone_full "$work/c19"
cd "$work/origin" || exit 1
git checkout -q -b renbase main
printf 'aaaa\nbbbb\ncccc\ndddd\neeee\nffff\n' > big.txt
git add big.txt
git commit -qm "add big.txt"
git checkout -q -b renamed renbase
git mv big.txt moved.txt
git commit -qm "rename big.txt"
git checkout -q main
cd "$work/c19" || exit 1
git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
git config diff.renames false
printf 'a rename, deliberately unclaimed' > "$work/body.txt"
out=$(sh "$script" origin/renbase origin/renamed "$work/body.txt" 2>&1)
st=$?
report "a rename is not a deletion even with diff.renames=false" 0 "$st" "$out"

# ── 20. A non-ASCII path can be claimed as itself ─────────────────────────
# REVIEW FINDING, and it refuted the author's own flagged weakness in its stated
# direction. A non-ASCII deletion does NOT pass silently — git C-quotes it, so
# the gate fails, which is safe. The defect is the mirror image: a CORRECT PR
# claiming `Deletes: café.txt` was REJECTED, and the note printed told the
# author their correct claim matched nothing. That is a false positive on a
# legitimate change, which is the failure mode that gets a gate disabled.
#
# Fixed by `-c core.quotePath=false` on the diff. This test fails without it.
clone_full "$work/c20"
cd "$work/origin" || exit 1
git checkout -q -b unibase main
echo x > "café.txt"
git add "café.txt"
git commit -qm "add a non-ascii path"
git checkout -q -b unidel unibase
git rm -q "café.txt"
git commit -qm "delete it"
git checkout -q main
cd "$work/c20" || exit 1
git fetch -q origin 'refs/heads/*:refs/remotes/origin/*'
printf 'Deletes: caf\303\251.txt\n' > "$work/body.txt"
out=$(sh "$script" origin/unibase origin/unidel "$work/body.txt" 2>&1)
st=$?
report "a non-ASCII path can be claimed as the author would write it" 0 "$st" "$out"
# and unclaimed, it must still be caught
printf 'unclaimed' > "$work/body.txt"
out=$(sh "$script" origin/unibase origin/unidel "$work/body.txt" 2>&1)
st=$?
report "an unclaimed non-ASCII deletion is still caught" 1 "$st" "$out"

# ── 21. A claim inside a code fence is not a claim ────────────────────────
# REVIEW FINDING. Anchoring stops a mid-sentence mention (test 8) but not a
# claim on its own line inside a ``` fence — which is exactly how a PR body
# documents this convention, and the shape any PR amending this gate contains.
# To a human reading the rendered PR it is an example; to the gate it was an
# assertion. Measured before the fix: exit 0, "all claimed in the PR body".
clone_full "$work/c21"
run_gate "Here is how the gate works:

\`\`\`
Deletes: sub/doomed.txt
\`\`\`

This PR does not actually claim that deletion."
st=$?
report "a claim inside a code fence does not count" 1 "$st" \
    "$(cat "$work/last-output.txt")"

# A tilde fence is the other CommonMark form and must behave the same.
clone_full "$work/c21b"
run_gate "Example:

~~~
Deletes: sub/doomed.txt
~~~
"
st=$?
report "a claim inside a tilde fence does not count" 1 "$st" \
    "$(cat "$work/last-output.txt")"

# The fence must not eat a REAL claim that follows it — otherwise the fix
# would trade a silent pass for a false positive, which is a worse trade.
clone_full "$work/c21c"
run_gate "The convention looks like this:

\`\`\`
Deletes: some/example.txt
\`\`\`

And this PR really does delete one:

Deletes: sub/doomed.txt
"
st=$?
report "a real claim after a fenced example still counts" 0 "$st" \
    "$(cat "$work/last-output.txt")"

# ── 22. Every annotation is one line, and carries its own fix ─────────────
# REVIEW FINDING, and the one my own tests were structurally unable to see.
#
# A GitHub workflow command is NEWLINE-DELIMITED: `::error::` consumes text up
# to the first \n, and the rest becomes ordinary log output. The annotation is
# what GitHub shows on the "Files changed" tab and in the check summary — the
# only surface a reader who does not open the raw job log ever sees.
#
# The shallow message used to span four source lines, so the annotation was:
#
#   ...the checkout is a shallow clone, so the merge base of
#
# truncated mid-clause, with `fetch-depth: 0` on the next line and therefore
# invisible. Test 5 greps the COMBINED STDOUT for `fetch-depth`, so it passed
# while the surface a human reads did not carry the fix. That is this piece's
# own failure one layer up: the gate refused to measure, then could not say how
# to fix it.
#
# `annotation_of` extracts what GitHub would keep — the first line of each
# ::error:: — so these assertions read the real surface rather than stdout.
#
# WHAT THIS STILL CANNOT CHECK, stated rather than papered over: it emulates
# GitHub's parsing from the documented newline rule, it does not observe a
# rendered annotation panel. A live Actions run remains the only way to see the
# real thing. What it does prove is the property that failed here — that the
# first line stands alone and carries the actionable word.
annotation_of() {
    # The first line of every ::error:: line, with the marker stripped.
    printf '%s\n' "$1" | awk '/^::error::/ { sub(/^::error::/, ""); print }'
}

clone_shallow "$work/c22" 1
run_gate "Deletes: sub/doomed.txt
"
ann=$(annotation_of "$(cat "$work/last-output.txt")")
case "$ann" in
    *fetch-depth*)
        passes=$((passes + 1))
        echo "ok   the shallow annotation itself names fetch-depth" ;;
    *)
        failures=$((failures + 1))
        echo "FAIL the shallow annotation loses fetch-depth to truncation:"
        printf '     >> %s\n' "$ann" ;;
esac

# And the annotation must be exactly one line — the general property, so a
# future message cannot reintroduce the defect in a different place.
ann_lines=$(printf '%s\n' "$ann" | grep -c .)
if [ "$ann_lines" -eq 1 ]; then
    passes=$((passes + 1)); echo "ok   the shallow annotation is a single line"
else
    failures=$((failures + 1))
    echo "FAIL the shallow annotation is $ann_lines lines; GitHub keeps only the first"
fi

# The same property on every other could-not-look path, so this is a rule about
# the script rather than a patch to one message.
clone_full "$work/c22b"
out=$(sh "$script" origin/no-such-branch origin/feature "$work/body.txt" 2>&1)
ann=$(annotation_of "$out")
ann_lines=$(printf '%s\n' "$ann" | grep -c .)
if [ "$ann_lines" -eq 1 ]; then
    passes=$((passes + 1)); echo "ok   the missing-ref annotation is a single line"
else
    failures=$((failures + 1))
    echo "FAIL the missing-ref annotation is $ann_lines lines"
fi

# The unclaimed-deletion red is the common case and must stay single-line too.
clone_full "$work/c22c"
run_gate "no claims here"
ann=$(annotation_of "$(cat "$work/last-output.txt")")
ann_lines=$(printf '%s\n' "$ann" | grep -c .)
if [ "$ann_lines" -eq 1 ]; then
    passes=$((passes + 1)); echo "ok   the unclaimed-deletion annotation is a single line"
else
    failures=$((failures + 1))
    echo "FAIL the unclaimed-deletion annotation is $ann_lines lines"
fi

# ── 23. A wrapped headline is folded, not truncated ───────────────────────
# The fix is structural: `cannot_measure` folds newlines in its headline, so a
# caller that wraps for source readability still emits one annotation line
# rather than silently losing its tail. Asserting the mechanism directly means a
# future author who wraps a message does not reintroduce the defect.
clone_shallow "$work/c23" 1
run_gate ""
ann=$(annotation_of "$(cat "$work/last-output.txt")")
case "$ann" in
    *"cannot measure"*"fetch-depth"*)
        passes=$((passes + 1))
        echo "ok   headline folding keeps cause and fix in one annotation" ;;
    *)
        failures=$((failures + 1))
        echo "FAIL cause and fix are not both in the annotation: $ann" ;;
esac

printf '\n%s passed, %s failed\n' "$passes" "$failures"
[ "$failures" -eq 0 ]
