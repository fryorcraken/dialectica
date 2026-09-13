#!/usr/bin/env sh
# Fail if a branch deletes a file its pull request does not claim to delete.
#
#   check-claimed-deletions.sh <base-ref> <head-ref> <pr-body-file>
#
# A claim is a line in the PR body reading `Deletes: <path>`, one per deleted
# path. Exits 0 if every deleted path is claimed, 1 otherwise — and 1, loudly,
# if the comparison could not be made at all.
#
# WHY THIS IS A FILE AND NOT AN INLINE `run:` BLOCK. Every other check in the
# `Lint` job is inline. This one must be provably able to FAIL, and its most
# important failure — a shallow clone — cannot be reached from a workflow
# without deliberately misconfiguring the workflow and pushing it. As a script
# it runs against throwaway repositories in
# .github/scripts/tests/test-check-claimed-deletions.sh, shallow clones
# included.
#
# WHAT THIS DOES NOT CHECK, stated because a gate trusted past its limits is
# how the false green it guards against happens in the first place:
#
#   * It does not check that a claimed deletion is CORRECT. `Deletes:` is an
#     assertion by whoever wrote the body. This converts a silent deletion into
#     a stated one, never into a reviewed one — closer.md step 2 is still the
#     reader.
#   * It does not notice anything a branch ADDS, which is the other half of the
#     stale-branch failure. A textual merge that kept two copies of one refactor
#     is invisible here.
#   * It runs on pull requests only. There is no PR body on a push to main or a
#     tag, so a green run on main is not evidence that this ran.
set -u

base_ref=${1:-}
head_ref=${2:-}
body_file=${3:-}

if [ -z "$base_ref" ] || [ -z "$head_ref" ] || [ -z "$body_file" ]; then
    echo "usage: $0 <base-ref> <head-ref> <pr-body-file>" >&2
    exit 1
fi

# `cannot measure` is in every could-not-look message, and the tests assert on
# that phrase. It is what separates "I looked and the branch is fine" from "I
# could not look" — the distinction this whole script exists to preserve. A
# reader who sees an unclaimed-deletions failure edits the PR body; a reader who
# sees this must change the checkout instead, and conflating them sends them to
# edit a body that was never the problem.
cannot_measure() {
    echo "::error::deletion gate cannot measure this branch — $1"
    exit 1
}

# ── Guard 1: the clone must be deep enough to hold the fork point ─────────
#
# `actions/checkout` defaults to `fetch-depth: 1`. In a clone that shallow,
# `git diff base...head` exits 128 with `fatal: no merge base` — it does NOT
# print an empty list. That sounds safe, and is not: three ordinary shell
# shapes turn that 128 into a pass, all three measured against a real shallow
# clone while writing this —
#
#   * a `run:` block with no `set -e`              -> exit 0, "no deletions"
#   * `set -eu`, piped into a `while read` loop    -> exit 0, pipe eats the 128
#   * `set -eu` with `|| true` on the substitution -> exit 0, "no deletions"
#
# So the shallow state is checked up front and named, rather than being left to
# surface as a `fatal:` line that mentions neither fetch-depth nor this gate.
#
# This guard is not redundant with guard 2. It names the cause the fix acts on,
# and it holds even in the case git does not currently produce: a graft boundary
# resolving as a plausible-but-wrong merge base. Probed at depths 1, 3, 10 and
# 25 against a 21-commit main with a branch forking at the root commit, today's
# git refuses outright at every depth that cannot see the fork point and is
# exact at every depth that can. The guard costs one command and does not
# depend on that staying true.
if [ "$(git rev-parse --is-shallow-repository 2>/dev/null)" = "true" ]; then
    cannot_measure "the checkout is a shallow clone, so the merge base of
  $base_ref and $head_ref cannot be resolved. Set 'fetch-depth: 0' on the
  actions/checkout step for this job. Without it this check silently finds no
  deletions and passes, which is worse than not running at all."
fi

# ── Guard 2: both refs must resolve, and so must their merge base ─────────
#
# Everything else that breaks the range: a base ref that was never fetched, an
# orphan branch with no common ancestor, a typo'd ref name. The shallow guard
# reports all of these as healthy, which is why both exist.
for ref in "$base_ref" "$head_ref"; do
    git rev-parse --verify --quiet "$ref^{commit}" > /dev/null 2>&1 \
        || cannot_measure "'$ref' does not resolve to a commit in this checkout."
done

merge_base=$(git merge-base "$base_ref" "$head_ref" 2>/dev/null) || merge_base=""
[ -n "$merge_base" ] \
    || cannot_measure "'$base_ref' and '$head_ref' have no common ancestor in
  this checkout, so there is nothing to diff against."

# ── The body ──────────────────────────────────────────────────────────────
#
# Read from a file rather than an argument or an environment interpolation. A
# PR body is attacker-controlled text and this is the only step in the workflow
# that handles any; `${{ github.event.pull_request.body }}` expanded into a
# `run:` block is shell injection.
#
# A missing file fails rather than reading as "no claims made", because "no
# claims" is a legitimate state for most pull requests and would mask the
# workflow having stopped writing the file at all.
[ -f "$body_file" ] \
    || cannot_measure "the PR body file '$body_file' does not exist."

# ── The deletions ─────────────────────────────────────────────────────────
#
# Three dots. `base...head` diffs against the MERGE BASE — what this branch
# deletes relative to where it forked. The two-dot form reports what the base
# has gained since, which on a correctly merged branch produced a
# 6,871-deletion false alarm. A gate that cries wolf is a gate someone
# disables.
#
# Written to a file rather than held in a variable so that the read loop below
# is not a pipeline: a `git ... | while read` pipe would put the loop in a
# subshell, losing any variable it sets, and would swallow git's exit status —
# which is exactly shape two in the table above.
deleted_list=$(mktemp) || cannot_measure "mktemp failed."
claimed_list=$(mktemp) || cannot_measure "mktemp failed."
unclaimed_list=$(mktemp) || cannot_measure "mktemp failed."
trap 'rm -f "$deleted_list" "$claimed_list" "$unclaimed_list"' EXIT

if ! git diff --diff-filter=D --name-only "$base_ref...$head_ref" > "$deleted_list" 2>/dev/null
then
    cannot_measure "'git diff $base_ref...$head_ref' failed. The merge base
  resolved to $merge_base, so this is not the shallow-clone case."
fi

# Extract the claims: lines whose first non-space, non-list-marker content is
# `Deletes:`.
#
# Anchored at line start, with optional indentation and an optional Markdown
# list marker, because PR bodies are Markdown and authors write bullets. NOT a
# substring search: a body explaining this convention would otherwise
# acknowledge whatever path it used as an example. The adapter check in ci.yml
# carries the same scar and strips comments for it.
#
# sed rather than `grep -P` so this works on any POSIX sed.
#
# The trailing `s/[[:space:]]*$//` strips trailing whitespace and, with it, the
# carriage return of a CRLF body — GitHub delivers PR bodies with CRLF endings
# often enough to matter, and a trailing \r makes an exact comparison fail
# against a path that is otherwise identical. That is the direction that fails a
# CORRECT pull request: annoying rather than dangerous, but still wrong.
#
# One `s///p` whose pattern matches the whole line and whose replacement keeps
# only the path, rather than a substitution followed by `sed -i`: `-i` needs an
# argument on BSD and refuses one on GNU, and a portability fallback around a
# command that may already have rewritten the file is a worse shape than not
# needing one.
sed -n 's/^[[:space:]]*[-*+]\{0,1\}[[:space:]]*Deletes:[[:space:]]*\(.*[^[:space:]]\)[[:space:]]*$/\1/p' \
    "$body_file" > "$claimed_list"

unclaimed=0
deleted_count=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    deleted_count=$((deleted_count + 1))
    # -x exact whole line, -F literal: a path is not a pattern, and a path
    # containing `.` or `[` would otherwise match things it should not.
    if grep -qxF -- "$path" "$claimed_list"; then
        continue
    fi
    printf '%s\n' "$path" >> "$unclaimed_list"
    unclaimed=$((unclaimed + 1))
done < "$deleted_list"

# A claim with no matching deletion. NOT a failure: a stale claim left from an
# earlier push, or a path restored during review, is not a defect worth
# blocking a merge on, and failing here would fire during ordinary iteration on
# a PR whose body was written first. Reported so it is visible.
#
# NO SPEC: nothing specifies this direction; see design.md, Decisions §5, and
# the test named for it.
while IFS= read -r claim; do
    [ -n "$claim" ] || continue
    if ! grep -qxF -- "$claim" "$deleted_list"; then
        echo "note: the body claims 'Deletes: $claim', but the diff does not"
        echo "      delete that path. Not a failure — stale claims are harmless."
    fi
done < "$claimed_list"

if [ "$unclaimed" -eq 0 ]; then
    echo "ok: $deleted_count deleted path(s) against merge base $merge_base," \
         "all claimed in the PR body"
    exit 0
fi

echo "::error::this pull request deletes $unclaimed file(s) it does not claim to delete."
echo ""
echo "Deleted relative to the merge base ($merge_base) with no matching claim:"
while IFS= read -r path; do
    [ -n "$path" ] || continue
    echo "  $path"
done < "$unclaimed_list"
echo ""
echo "A branch cut before a change landed carries 'the file without that change'"
echo "as an intentional-looking deletion, and a squash merge applies it with no"
echo "conflict. If these deletions are NOT intended, rebase onto current main."
echo ""
echo "If they ARE intended, add one line per path to the pull request body:"
while IFS= read -r path; do
    [ -n "$path" ] || continue
    echo "  Deletes: $path"
done < "$unclaimed_list"
exit 1
