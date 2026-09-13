# Readability findings — `deletion-gate`

Reviewed at `8ba7f23` in `.claude/worktrees/review-deletion-gate-read`. The suite
runs green as committed (**34 passed, 0 failed**), which matches design.md §10's
figure. Every entry below was produced by running the script and reading its real
output against a probe repository, not by reading the source and imagining it.

The organising question taken from the brief: can a reader meeting a red gate at
an inconvenient moment tell **in under a minute** whether it is a real defect or
a false positive, and what to do next?

For the common red — an unclaimed deletion — the answer is an emphatic yes, and
it is the best thing in the piece. Measured output:

```
::error::this pull request deletes 2 file(s) it does not claim to delete.

Deleted relative to the merge base (21351df...) with no matching claim:
  also-doomed.txt
  sub/doomed.txt
...
If they ARE intended, add one line per path to the pull request body:
  Deletes: also-doomed.txt
  Deletes: sub/doomed.txt
```

It names the paths, states both branches of the diagnosis (rebase vs. claim),
and hands over the exact lines to paste. Nothing below detracts from that.

## Defects

- [x] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:48`,
      `75-78`, `93-94`, `157-158` — every `cannot measure` message is multi-line,
      and GitHub keeps only the first line in the annotation, so the shallow
      case's `fetch-depth` fix is discarded before the reader sees it
      **Scenario:** a workflow command is newline-delimited — `::error::` consumes
      text up to the first `\n` and the remainder becomes ordinary log output
      rather than part of the annotation. `cannot_measure` emits
      `echo "::error::deletion gate cannot measure this branch — $1"`, and the
      shallow argument spans four source lines. The annotation GitHub surfaces on
      the PR "Files changed" tab and in the check summary is therefore exactly:

      `deletion gate cannot measure this branch — the checkout is a shallow clone, so the merge base of`

      It breaks off mid-clause. The sentence naming the fix — *"Set 'fetch-depth:
      0' on the actions/checkout step for this job"* — is on the next source line
      and never reaches the annotation. A reader who only opens the annotation (the
      normal path: GitHub links it from the PR, and it is what the check-summary
      surface shows) gets "cannot measure" plus a dangling "merge base of" and must
      go dig in the raw job log to find the actionable half.
      **Measured:** ran all four paths against a probe repo. The unclaimed-deletion
      failure at line 249 is correctly single-line and unaffected; all three
      `cannot_measure` calls and the guard-3 message truncate. Every one of the
      other 17 `::error::` annotations in `ci.yml` is single-line (`grep -n
      "::error::"`), so this script is the only place in the workflow that relies
      on a continuation line — the convention exists and this is the one departure
      from it.
      **Severity:** high for a readability review. The script's own comment at
      lines 41-46 states that this phrasing is what "separates 'I looked and the
      branch is fine' from 'I could not look'" and that the fix a reader must apply
      is *changing the checkout*. The message is written to route the reader
      correctly and the transport silently drops the routing. Test 5 asserts
      `fetch-depth` appears in the **combined stdout** (`grep -q 'fetch-depth'`), so
      the suite passes while the annotation a human actually reads does not carry
      it — the test cannot see this.

      **Fixed** in the commit carrying this tick. This is the best finding on the
      piece: it is my own thesis arriving one layer up, and my tests were
      structurally incapable of seeing it. I reproduced it before fixing — the
      annotation really did read `...so the merge base of`, breaking off exactly
      where you said.
      The fix is structural rather than per-message, because patching three
      strings would leave the next author free to reintroduce it. `cannot_measure`
      now takes a one-line headline plus optional detail lines and folds any
      newline in the headline to a space, so a caller who wraps for source
      readability still emits one annotation line. The shallow headline now leads
      with the fix: *"the checkout is a shallow clone — set 'fetch-depth: 0' on
      this job's actions/checkout step."*
      Test 22 asserts on `annotation_of()` — the first line of each `::error::`,
      which is what GitHub keeps — rather than on stdout, and checks three
      properties: the shallow annotation contains `fetch-depth`, every annotation
      is exactly one line, and (test 23) folding keeps cause and fix together.
      Verified it fails: restoring the old multi-line string turns tests 22 and 23
      red while tests 5 and 7 stay green, which is the blindness you identified,
      demonstrated.
      **What it still cannot check, per your own note:** it emulates the
      documented newline rule, it does not observe a rendered annotation panel. A
      live Actions run remains the only way to see the real thing. I did not close
      that gap with a weaker check and have said so in design.md §12.

- [x] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:154-159` —
      the third guard is the only one with no `── Guard N ──` banner, so a reader
      scanning the script finds two guards where five other documents say three
      **Scenario:** the script carries banner comments `── Guard 1: the clone must
      be deep enough ──` (line 52) and `── Guard 2: both refs must resolve ──`
      (line 81). The failing-`git diff` check at 154 sits under a banner reading
      `── The deletions ──` and is never called a guard anywhere in the file.
      Meanwhile `findings/correctness.md`, `findings/security.md`, `tasks.md` 4.0d,
      `design.md` §11 and test 18's own assertion string
      (`"guard 2 still passes, so the next failure is guard 3's"`) all refer to it
      as "guard 3". A reader who meets that name in any of those five places and
      opens the script to find it cannot: `grep -n "Guard 3"` over the script
      returns nothing.
      **Measured:** `grep -n "Guard 1\|Guard 2\|Guard 3"` over the script returns
      only lines 52, 67 and 81. This is precisely the brief's question — *"is that
      legible, or does it read as belt-and-braces?"* — and the answer is that it
      does not even read as a guard. The comment above it (126-153) is entirely
      about the two pinned git settings, not about what the `if !` is defending.
      Note this is exactly the guard that survived a `|| true` mutation at 23/23
      before test 18 existed; the thing hardest to see in the source is the thing
      the suite was blindest to.
      **Severity:** medium — no behavioural defect, but it defeats the design's
      own stated goal of making each guard's distinct job visible from the source.

      **Fixed** in the commit carrying this tick. Guard 3 now carries a
      `── Guard 3: the diff itself must succeed ──` banner whose body says what it
      defends (an unreadable tree in a repository every cheap check calls healthy)
      rather than leaving the two pinned git settings to dominate the region.
      `grep -n "Guard 3"` over the script now returns it, so the name five
      documents use resolves in the source.
      Your closing observation is the sharpest thing in this sheet and I have
      quoted it into design.md §2: *the thing hardest to see in the source was the
      thing the suite was blindest to.* That is not a coincidence and it is worth
      a reader knowing.

- [x] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:161-209` —
      two unrelated comment blocks were merged without a separator, so 22 lines
      documenting the `sed` are read as documenting the `awk` that follows them
      **Scenario:** the block opens at 161 with *"Extract the claims: lines whose
      first non-space... content is `Deletes:`"* and spends lines 161-182 on the
      `sed` invocation — the anchor, `grep -P` portability, the CRLF trim, why not
      `sed -i`. Then at **line 183, with no blank comment line between**, it
      switches subject: *"FENCED CODE BLOCKS ARE REMOVED FIRST"*, and 183-203
      document the `awk`. The code that then appears at 206 is the `awk`; the `sed`
      the first 22 lines describe is at 211. Every other paragraph break in the
      block is a bare `#` (lines 163, 169, 171, 177, 185, 194, 200) — the
      separator is used consistently seven times and is missing at exactly the one
      boundary where the subject changes, which is the reading that makes it look
      like an append rather than a choice.
      A reader meeting a fence-related false positive opens this block, reads two
      paragraphs about `sed` portability and CRLF, and has to get to line 183
      before reaching anything about fences.
      **Severity:** medium — stylistic in isolation, but this is the densest
      comment block in the file and the one a reader arrives at when diagnosing the
      claim-parsing behaviour, which is where false positives live.

      **Fixed** in the commit carrying this tick, and you diagnosed the cause
      correctly — it was an append, not a choice. Rather than insert the missing
      `#`, I reordered so each block sits with the code it describes: `── Step 1:
      remove fenced code blocks ──` above the `awk`, `── Step 2: extract the
      claims ──` above the `sed`. A reader arriving from a fence-related false
      positive now meets the fence paragraph first, and the ordering of the
      comments matches the order of execution, so the drift cannot silently
      recur the same way.

- [x] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:204`,
      `209`, `212` — the variable is named `uncommented_body` but nothing in this
      script strips comments; it holds a body with **code fences** removed
      **Scenario:** the `awk` at 206-209 toggles on ` ``` ` and `~~~` and drops
      lines inside a fence. There is no comment-stripping anywhere in the file. The
      name appears to have been carried over from the sibling adapter gate in
      `ci.yml` (line 454), which genuinely does `re.sub(r"^\s*//.*$", ...)` — and
      design.md §8b explicitly draws the analogy to that gate, which is the likely
      route by which the wrong noun arrived. A reader who greps this script for
      "comment" to find out what is stripped lands on lines 168 and 201, both of
      which discuss the *other* gate's comment stripping, and can reasonably
      conclude this script strips comments too. `unfenced_body` would say what the
      file holds.
      **Severity:** low — a misnomer, not a defect; recorded because the name is
      load-bearing for the one reader trying to work out why their fenced example
      did not count as a claim.

      **Fixed** — renamed to `unfenced_body`, and your reconstruction of how the
      wrong noun arrived is right: it came from the sibling adapter gate via the
      §8b analogy. The comment above it now says so explicitly, so a reader
      grepping this script for "comment" is told there is none to find rather
      than left inferring it from the two mentions of the *other* gate.

## On design.md's length — long because it accreted, and the seams are visible

The brief asks whether design.md (355 lines) is long because the subject is or
because it accreted. It is both, but the accretion is legible in the numbering
and worth one box, because the structure now actively misdirects:

- [x] **`dev-writer`** — `openspec/changes/deletion-gate/design.md:105`,
      `200`, `243` — the Decisions section's own numbering records the order the
      sections were written rather than the structure of the subject, and §2's
      title is now false
      **Scenario:** three concrete seams:
      **(a)** §2 is titled *"Two independent guards, because they catch different
      failures"* and argues at length that neither of two guards is redundant. The
      script has **three**. Guard 3's existence, reachability and test are
      documented in §11, 208 lines later. A reader who reads §2 — which is where
      the document tells them to understand the guards — comes away believing the
      script has two, and §2 contains nothing pointing forward to §11.
      **(b)** §7 is titled *"Three of these tests were written wrong first"* and
      then opens with a parenthetical saying the third is in §11, followed by
      *"**Both** are the defect family..."*. The section promises three and
      delivers two.
      **(c)** §8b is a letter-suffixed insertion between §8 and §9 — the standard
      tell of a section added after the numbering was fixed.
      Renumbering, retitling §2 to cover three guards, and folding §11's content
      into it would let a reader learn the guard structure in one place.
      **Severity:** medium — the subject genuinely warrants a long document (the
      measured shallow-clone table and the three wrong-first tests are exactly the
      "what a command cannot tell you" CLAUDE.md asks to be written down), so this
      is about ordering, not about cutting. But §2's title is a factual claim about
      the code that is now wrong, and per this repo's own rules a stale claim in a
      recorded decision is what the next reader trusts.

      **Fixed** in the commit carrying this tick, taking your suggested shape:
      **(a)** §2 is retitled *"Three independent guards"* and opens with a table
      of all three and what each catches that the others miss; §11's guard-3
      content is folded into it, so the guard structure is learned in one place.
      §11 is now the annotation finding from the top of this sheet.
      **(b)** §7 is retitled to *"Two ... and both"*, matching what it delivers,
      with the third pointed at §2 where it now lives beside its guard.
      **(c)** §8b is renumbered to §9, and 9/10/11 shift up; cross-references in
      the script and in design.md are updated. I left the findings sheets' `§8b`
      references alone — editing a reviewer's text to match my renumbering would
      be the wrong repair, so this note is the mapping.
      I did not renumber beyond that. The remaining order is still roughly
      chronological, but every section is now titled truthfully, which was the
      actual defect; a fuller restructure would churn the diff under the
      reviewers still to come for no gain a reader would feel.

## Figures — checked against commands, and one is stale

Per the brief I re-derived every quantity. The design.md table is **sound**:
`piece/authoring` 26 files two-dot / 0 three-dot, `piece/generated-names` 13/0,
and `piece/publish-envelope` `18 files changed, 2065 insertions(+), 5067
deletions(-)` two-dot against `11 files changed, 2028 insertions(+), 130
deletions(-)` three-dot — the 5,067/130 pair the brief said was corrected today
is correct. The six files it names are the ones it lists. The `core-e2e` archive
claim in `docs/OPENSPEC-ARCHIVE.md` re-derives exactly: `git log --diff-filter=A
-- openspec/changes/archive/2026-09-13-core-e2e/` returns `b85111d`.

Two figure defects, both in the same family — a number that moved when a branch
tip did:

- [x] **`dev-writer`** — `openspec/changes/deletion-gate/design.md:40` — the
      stale `5,034` survives here, one section after the corrected `5,067`
      **Scenario:** line 24 of the same file (§"The near miss") correctly reads
      *"5,067 lines deleted two-dot against 130 three-dot"*. Line 40, in the very
      next section, argues *"'5,034 deletions' reads exactly the same whether the
      branch rotted or `main` grew"* — quoting the superseded figure. The brief
      records that `5,034/97` was corrected to `5,067/130` because branch tips
      moved; that correction reached line 24 and missed line 40.
      **Measured:** `git diff --shortstat origin/main origin/piece/publish-envelope`
      returns `5067 deletions(-)` today; no measurement in the repository produces
      5,034. Since the sentence's whole rhetorical move is *this exact number is
      ambiguous*, quoting a number that no longer arises from any command makes the
      argument unreproducible at precisely the point it asks the reader to trust a
      quantity.
      **Severity:** medium — one of two numbers in a two-number document
      disagreeing with the other is the shape this repo says it fabricates.

      **Fixed**, and not by substituting 5,067. The sentence's whole move is
      *this quantity is ambiguous*, so no specific figure was ever load-bearing
      there and any figure could only rot — it now reads "a five-thousand-line
      deletion count", with a parenthetical recording that it once quoted 5,034
      and why a number was the wrong thing to pin. The reproducible pair stays in
      the measured table two sections above, which is where a reader who wants a
      number should get one.

- [x] **`dev-writer`** — `openspec/changes/deletion-gate/proposal.md:32-40` —
      the document's headline "run these two commands before believing anything
      below" no longer runs: the branch it names has been deleted
      **Scenario:** the proposal presents a fenced pair of commands against
      `origin/piece/op-transport` and instructs the reader *"Run those two commands
      before believing anything below; they are the whole argument in six lines."*
      That branch no longer exists. Both commands now fail:
      `fatal: ambiguous argument 'origin/piece/op-transport': unknown revision or
      path not in the working tree` — after `git fetch origin --prune`, it is
      absent from `git branch -r` (the change having merged).
      The 7,101-vs-37 figures are therefore no longer re-derivable, which is the
      exact fate the same section already documents for its *predecessor* figure:
      *"That instance can no longer be measured... the historical 7,071 figure is
      not reproducible from the repository today."* The document anticipated this
      failure mode for one number and then reintroduced it for the replacement.
      `origin/piece/publish-envelope` is live today and gives 5,067-vs-130, which
      design.md already uses — but per this repo's self-invalidating rule, a
      substitute branch tip will rot the same way, so the durable fix is to phrase
      the instruction against whatever branch is open rather than to pin another.
      **Severity:** medium — the proposal's stated evidentiary basis. Also note
      `proposal.md:70` still cites a third figure, the **6,871**-deletion false
      alarm, for which no reproducing command is given anywhere.

      **Fixed**, taking your durable option rather than pinning a third branch.
      Confirmed the breakage first: `git rev-parse --verify origin/piece/op-transport`
      exits 1. The fenced block is now a loop over `git branch -r --list
      'origin/piece/*'` printing both ranges for whatever is open, so it names no
      ref that can be pruned. I ran the replacement before shipping it — it works,
      and `piece/authoring` currently shows 27,075 deletions two-dot against 168
      three-dot, which is a louder demonstration than the figure it replaces.
      The paragraph now says outright that the section documented this rot for one
      figure and then reintroduced it, since that is the lesson rather than the
      figures. It also states what a reader should conclude if every branch has
      been rebased and the two forms agree — the healthy state, not a failed
      reproduction — so an empty result is not read as the argument collapsing.
      Note this box duplicates one on the architecture sheet addressed to
      `spec-writer`; `proposal.md` is that role's file. I fixed the text because it
      was factually broken in my tree and I was editing alongside it. **The
      `spec-writer` box stays open** — if that role would rather phrase it
      differently, this is theirs to overrule.
      The **6,871** figure you flag at line 70 is a real remaining gap. I did not
      touch it: it is a session observation from before this piece, no command
      reproduces it, and inventing a citation would be worse than leaving it
      visibly unsourced. Flagging it rather than closing it.

## What was clean

**The failure messages route correctly, which is the piece's strongest property
and it is the one I most tried to break.** All four paths were run against a real
probe repository. The unclaimed-deletion red names every path, explains both
possible causes in the reader's own terms ("a branch cut before a change landed
carries 'the file without that change' as an intentional-looking deletion"), and
prints the `Deletes:` lines ready to paste. The shallow red names `fetch-depth: 0`
and the specific checkout step. The unresolvable-ref red names the ref that did
not resolve and — verified by test 7's negative assertion — does *not* mention
`fetch-depth`, so the two cannot-look causes are genuinely distinguished from each
other. The `cannot measure` phrase is present on every could-not-look path. The
only thing wrong with any of this is the annotation truncation filed above; the
message text itself is well judged throughout.

**The stale-claim note reads correctly and does not look like a failure.** Probed:
a body claiming a real deletion plus `Deletes: never-was.txt` prints
`note: ... Not a failure — stale claims are harmless.` and then the `ok:` line and
exit 0. A reader cannot mistake it for the red, which is the risk with a warning
printed immediately before a pass.

**The three guards' distinct jobs are individually well argued** where they are
argued at all — guard 1's comment (52-73) explains why it is not redundant with
guard 2 and names the hypothetical it covers, and guard 2's (81-85) lists the
three concrete cases it catches that guard 1 reports as healthy. The problem filed
above is guard 3's labelling and the design's stale §2 title, not the reasoning.

**The documentation split holds, and I checked it specifically for leakage.**
`closer.md` gains exactly four lines: the marker, the absent prompt, the flag not
being needed, the `schema:` requirement, and a pointer to the page. Every piece of
*reasoning* — why the `--strict` message misleads, the two-symptoms-one-cause
diagnosis, the `core-e2e` precedent and its limits — lives only in
`docs/OPENSPEC-ARCHIVE.md`. Nothing in the pointer contradicts the page, and the
page carries nothing that would need updating in two files. The brief's question
("has reasoning leaked into the pointer?") is answered no. The dispatched-vs-
discretionary rule in `closer.md` is likewise reasoned in `proposal.md` and stated
as a rule in the agent file, with the same split.

**The test script is more readable than the script it tests.** Each numbered case
carries a comment saying what would break if it were absent, and the three
wrong-first fixtures (4, 15, 18) each record the mechanism that made the earlier
version prove nothing — which is exactly what stops the next reader "simplifying"
them back. Test 18's `guard 2 must still pass` pre-assertion is the sharpest
readable thing in the piece.

**`ci.yml`'s new comments earn their place.** The `fetch-depth: 0` block explains
why not a finite depth, and the gate-on-the-gate block records that the decision
was reversed and why. Both say something a reader could not get from the code.

## What I could not check

The rendered GitHub annotation itself. The truncation finding above rests on the
documented newline-delimited behaviour of workflow commands and on the observed
fact that all 17 other `::error::` lines in `ci.yml` are single-line; I could not
open a live Actions run to photograph the annotation panel. If a `dev-writer`
disputes it, the cheap confirmation is one deliberately-shallow PR. The finding
does not depend on it for the narrower claim that this script is the file's only
multi-line annotation.
