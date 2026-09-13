#!/usr/bin/env sh
# Fail if a branch deletes a file its pull request does not claim to delete.
#
#   check-claimed-deletions.sh <base-ref> <head-ref> <pr-body-file>
#
# RUN IT FROM INSIDE THE REPOSITORY TO BE MEASURED. There is a fourth input and
# it is the working directory: every `git` call here runs against the ambient
# cwd's repository, with no `-C` and no `--git-dir`. Both callers set it
# ambiently — the test harness `cd`s into each throwaway clone, the workflow
# relies on `actions/checkout` — so neither passes it and neither is checkable
# against the signature above.
#
# It is documented rather than parameterised, deliberately. A `git -C "$4"` on
# every call would be the tidier contract, but the guards below already make a
# wrong cwd LOUD: the refs fail to resolve and the gate exits 1 having said so.
# What it cannot do is exit 0 having measured the wrong repository. Given that,
# a fourth positional argument buys accuracy in the diagnosis and costs a change
# to both call sites; this line buys the same accuracy for a reader and costs
# nothing. If a third caller ever appears, parameterise it then — the diagnosis
# a wrong cwd produces today ("'origin/feature' does not resolve to a commit in
# this checkout") names the ref rather than the directory, which is the part
# that misleads.
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
#   * IT MEASURES AGAINST THE FORK POINT, NEVER AGAINST CURRENT `main`. A branch
#     that is BEHIND main has a clean three-dot diff and passes here, correctly —
#     the deletions are real but only visible from a vantage point this gate does
#     not have. Fired twice in one session, once on this change's own PR (#69),
#     which its spec-test reviewer caught and this author did not. NOT added as a
#     second arm: `gh api .../branches/main/protection` reports `"strict": true`,
#     so GitHub already refuses to merge a behind-branch, and an arm here would
#     go red on every open PR for a window after every merge — the ten-PR false
#     alarm this gate's own three-dot argument exists to avoid. It belongs in
#     closer.md step 2, where a human knows whether the merge is imminent.
#   * IT CATCHES A FILE VANISHING, NOT WORK VANISHING. That one line covers the
#     other two known blind spots:
#       - anything a branch ADDS. A textual merge that kept two copies of one
#         refactor is invisible here.
#       - a merge that silently REVERTS content. A staged resolution whose index
#         was computed against a base that has since moved records a tree
#         predating its own first parent; measured at 254 lines. No file
#         disappears, so `--diff-filter=D` returns nothing and this gate reports
#         `ok: 0 deleted path(s)`. Confirmed on a reconstruction, not assumed.
#         Nastier still, `merge-base --is-ancestor` on the reverted commit exits
#         0 — the commit stays reachable while its content is gone.
#     No guard is added for either: detecting "this merge reverted work" needs a
#     model of what the merge should have contained, which is a much harder
#     problem than this one. design.md §13 carries the working practice that
#     does catch it.
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
#
# THE FIRST ARGUMENT MUST BE ONE LINE, AND MUST CARRY THE FIX. A GitHub workflow
# command is newline-delimited: `::error::` consumes text up to the first `\n`
# and everything after it becomes ordinary log output. The annotation is what
# GitHub surfaces on the "Files changed" tab and in the check summary, and it is
# the only thing a reader who does not open the raw job log ever sees.
#
# This was a real defect, not a hypothetical. The shallow-clone message used to
# span four source lines, so the annotation truncated to
#
#   ...the checkout is a shallow clone, so the merge base of
#
# — breaking off mid-clause, with the sentence naming `fetch-depth: 0` on the
# next line and therefore never reaching the reader. The gate correctly refused
# to measure and then could not say how to fix it, which is this piece's own
# failure arriving one layer up: a red gate whose advice is truncated gets
# diagnosed wrongly, and a gate diagnosed wrongly gets disabled.
#
# So the shape enforces it rather than asking each caller to remember. `$1` is
# the headline and any newline in it is folded to a space, so a multi-line
# headline cannot silently lose its tail; the remaining arguments are printed
# as plain log lines beneath, where detail belongs. Every other `::error::` in
# ci.yml is single-line, so this is the file's convention as well as GitHub's.
cannot_measure() {
    headline=$1
    shift
    # Fold any newline and run of spaces into one space. A caller that wraps its
    # headline for source readability then still emits one annotation line.
    headline=$(printf '%s' "$headline" | tr '\n' ' ' | tr -s ' ')
    echo "::error::deletion gate cannot measure this branch — $headline"
    for detail in "$@"; do
        echo "  $detail"
    done
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
    # The headline names the fix, because the headline is the whole annotation.
    cannot_measure \
        "the checkout is a shallow clone — set 'fetch-depth: 0' on this job's actions/checkout step." \
        "The merge base of $base_ref and $head_ref cannot be resolved without history." \
        "Without that setting this check silently finds no deletions and passes," \
        "which is worse than not running at all."
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
    || cannot_measure \
        "'$base_ref' and '$head_ref' have no common ancestor in this checkout." \
        "There is nothing to diff against. The checkout is not shallow, so this" \
        "is a missing or unfetched ref rather than a truncated history."

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
# ONE DIRECTORY, ONE TRAP, SET ONCE. The scratch files were previously four
# separate `mktemp` calls registered by two `trap` statements, the second
# re-declaring the whole list — so adding a fifth file meant editing the second
# trap while the first remained a stale-looking prefix of it, and forgetting
# would leak silently with nothing to catch it. A directory makes the cleanup
# invariant hold by construction and a new scratch file inherit it for free,
# which is CLAUDE.md's "complexity in the data structure, not the logic" at
# five-line scale.
scratch=$(mktemp -d) || cannot_measure "mktemp -d failed."
trap 'rm -rf "$scratch"' EXIT
deleted_list="$scratch/deleted"
claimed_list="$scratch/claimed"
unclaimed_list="$scratch/unclaimed"
unfenced_body="$scratch/unfenced-body"

# ── Guard 3: the diff itself must succeed ─────────────────────────────────
#
# The last of the three could-not-look guards, and the one whose job is hardest
# to see, because the two git settings pinned below dominate the region. Guards
# 1 and 2 establish that the checkout has history and that both refs and their
# merge base resolve. This one catches everything that can still go wrong when
# all of that is true.
#
# It is NOT dead code, and that was measured rather than assumed. Removing a
# subtree object from the object store leaves `rev-parse --verify` and
# `merge-base` both exiting 0 — they peel and walk commits, never trees — while
# the diff exits 128. That is a partially-fetched or corrupted object store:
# every cheap check says the repository is healthy and only the diff disagrees.
#
# This guard survived a `|| true` mutation with the whole suite green before a
# test reached it, which is exactly why it is labelled now: the thing hardest to
# see in the source was the thing the suite was blindest to. design.md §2 and
# test 18 both call it "guard 3"; so does this banner, so the name resolves.
#
# TWO GIT SETTINGS ARE PINNED HERE RATHER THAN INHERITED, because each of them
# changes this gate's VERDICT and each is a default rather than a guarantee.
# `diff.renames` and `core.quotePath` can both be set in repo, global or system
# config, or through GIT_CONFIG_COUNT/GIT_CONFIG_KEY_0, none of which this
# workflow controls.
#
#   * `--find-renames` keeps rename DETECTION on, which is what makes a
#     `git mv` report as `R` and not as a `D` plus an `A`. Measured: the same
#     branch, the same command, `git mv big.txt moved.txt` — with detection on
#     the gate says "0 deleted paths" and exits 0; with `diff.renames false` in
#     the repo config it reports `big.txt` unclaimed and exits 1. A gate whose
#     answer depends on a developer's config is not a gate. Renames are also the
#     obvious false-positive class, and a false positive is what gets a check
#     disabled.
#
#     This is a WEAKER property than it looks and the comment must not overstate
#     it: a rename git scores below its similarity threshold still reports as a
#     deletion, correctly, because at that point the file really did go. What is
#     pinned is that the threshold is consulted at all.
#
#   * `core.quotePath=false` makes git print a non-ASCII path as itself rather
#     than C-quoted. At the default, deleting `café.txt` prints
#     `"caf\303\251.txt"`, so a correct PR claiming `Deletes: café.txt` is
#     REJECTED — and the note it prints tells the author their correct claim did
#     not match anything, which is advice pointing away from the problem. The
#     only body that satisfied the gate was one copying the escaped form back,
#     which no author would write. That is a false positive on a legitimate
#     change; the safe-direction failure is not a defence.
if ! git -c core.quotePath=false diff --find-renames --diff-filter=D \
        --name-only "$base_ref...$head_ref" > "$deleted_list" 2>/dev/null
then
    cannot_measure \
        "'git diff $base_ref...$head_ref' failed — the object store may be incomplete." \
        "The merge base resolved to $merge_base, so this is NOT the shallow-clone" \
        "case and not a missing ref: both of those were already checked. A tree or" \
        "subtree object the diff needs is unreadable."
fi

# ── Step 1: remove fenced code blocks from the body ───────────────────────
#
# FENCED CODE BLOCKS ARE REMOVED FIRST, and this is the anchoring rule's real
# teeth rather than a refinement of it.
#
# Anchoring (step 2 below) stops a claim mentioned mid-sentence. It does NOT
# stop one written on its own line inside a ``` fence — and a fenced example is
# exactly how a PR body documents this convention, which is the shape any PR
# introducing or amending this gate naturally contains. Measured before fixing:
# a body of "here is how it works", a fence, `Deletes: doomed.txt`, a closing
# fence, satisfied the gate for a genuine deletion of that file. To a human
# reading the rendered PR it is an example; to the gate it was an assertion.
#
# Toggling on every line whose first non-space characters are ``` or ~~~ (the
# two CommonMark fence characters), and dropping everything while inside. An
# unclosed fence swallows the rest of the body — deliberately the safe
# direction: claims go missing, so the gate FAILS a deletion rather than
# accepting one.
#
# Blockquote, bold `**Deletes:**`, lowercase, HTML comment, numbered list and
# nested bullet forms were all measured as already rejected by the anchor, so
# the fence was the one specific gap rather than general looseness.
#
# The variable says what the file HOLDS: a body with fences removed. It was
# briefly called `uncommented_body`, borrowed from the sibling adapter gate in
# ci.yml that genuinely strips `//` comments — but nothing in this script
# strips a comment, and a reader grepping for one would have been sent looking
# for code that does not exist.
awk '
    /^[[:space:]]*(```|~~~)/ { infence = !infence; next }
    !infence { print }
' "$body_file" > "$unfenced_body"

# ── Step 2: extract the claims from what survives ─────────────────────────
#
# Lines whose first non-space, non-list-marker content is `Deletes:`.
#
# Anchored at line start, with optional indentation and an optional Markdown
# list marker, because PR bodies are Markdown and authors write bullets. NOT a
# substring search: a body explaining this convention would otherwise
# acknowledge whatever path it used as an example.
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
    "$unfenced_body" > "$claimed_list"

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
