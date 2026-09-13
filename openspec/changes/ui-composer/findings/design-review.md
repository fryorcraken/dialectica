# Design review — `ui-composer`

Reviewed on `piece/ui-composer` at `bba5864`, in a worktree of its own. Suite
green at **124 tests across 8 spec files** (`run-qml-tests.sh`), matching the
spec-test reviewer's baseline. The other five findings files were read first;
nothing below repeats one.

`design.md` was read against the code, `docs/PLAN.md` was read from
**`origin/main`** (not the branch's copy), and every figure and source citation
in `design.md` was checked against the file it names.

## The record is in good shape, and this is worth saying plainly

Six of the eight Decisions entries are the shape the role exists to demand:
what was chosen, the constraint that forced it, the alternatives with what ruled
each out, and the cost. Three are unusually good.

**The `PublishOutcome` one-value entry matches what shipped.** `state` is
computed once at `PublishOutcome.qml:70-74`, total, with `"refused"` as the
fall-through, and all five elements key on it or on `isRefusal` derived from it
— I checked each of the five. The entry records that one classification was
chosen over an `isSuccess`/`isRefusal` pair *because two derived booleans can
still disagree*, which is the alternative-and-why-not part that usually rots
first. The totality is pinned by tests rather than present by accident: the
spec-test reviewer's mutation G (reverting `state` to a bare alias) fails two
tests, so this was not made by luck.

**The delivery denial is a positive `SHALL` and is built as one.**
`PublishOutcome.qml:161` is its own `Text` keyed on `!root.isRefusal`, not a
clause in either arm of the qualifier's ternary — so it is hung on the condition
that defines who is owed it. The wording lives in one helper
(`spec.deliveryDenial()`), so the test pin and the sweep exclusion cannot drift;
`test_the_pinned_denial_is_spelled_the_same_way_in_both_files` pins the second
copy. The entry records why silence is not compliance here, which is the part a
prohibition-shaped reading would have lost.

**`showScore` defaults false, with three alternatives and why each lost** —
including the one that reads as though it works and does not (`score: -1`,
`Math.max(0, -1) === 0`). `VoteControl.qml:31` is `property bool showScore:
false`, `FeedScreen.qml` sets it nowhere, and `score` survives unbound at line
59 with its floor. Code and record agree.

**Every figure and citation checks out.** `authoring.rs:206` is
`MAX_BODY_LEN = op::MAX_FIELD_LEN`; `op.rs:146` is `150 * 1024`, matching
`Composer.bodyByteLimit`. `sanitise.rs:141` is `is_invisible`, and its eight
match arms are character-for-character `Composer.isInvisible`'s eight ranges.
`lib.rs:257,272,285` are exactly `publish_post`, `publish_reply`,
`publish_vote`. The 153,740-byte figure traces to the archived `op-transport`
change's `design.md:213`, which is where it was measured. The "fourteen tests"
claim at `design.md:281` is the architecture reviewer's own measurement, stated
as theirs. `FeedScreen.qml:42`'s PLAN.md §9.1 quote is **verbatim in
`origin/main`'s PLAN.md at line 3477** — I checked the branch-independent copy
specifically, since a sibling piece was caught citing a section for the opposite
of what it said.

**PLAN.md migration is done correctly.** §7.4's "the UI half is not settled"
reasoning is struck rather than left to contradict the change, and the score
reasoning moved to the change — I confirmed the pointer resolves, since PLAN.md
sends the reader to `proposal.md` and `proposal.md:104` does carry it. §9.1
gains a summary of what is now contracted and keeps no duplicated reasoning.
Nothing this change acted on is left explaining itself in two places.

The reply-composer scope boundary is recorded in three places and reads as a
decision rather than an oversight: `design.md`'s own section, `tasks.md`'s
"Not done, and named rather than left to be found", and `Main.qml:13-21`. The
`capabilityFrom()` guard deletion is recorded as a decision at `design.md:270`,
with the measurement that the deleted guard defended the *open* arm — and
crucially with the reason a reader following the old comment would have deleted
it for the wrong reason. `replyParent` is recorded at `design.md` and argued in
`Composer.qml:25-42`, correctly distinguishing "unreachable" from "ignored".

The findings below are gaps in the record, not contradictions between record and
code. I found **no** case of the code contradicting a recorded decision.

## Findings

- [ ] **`dev-writer`** — `design.md` has **no Decisions entry for the
      draft-clearing asymmetry**, the one decision in this piece the spec calls
      "the decision rather than an inconsistency"
      The three-way rule — cleared on `wasNew:true`, kept on `wasNew:false`,
      kept on every refusal — is a whole spec requirement (spec.md:336-372, two
      scenarios) and `grep -n "clearDraft\|cleared" design.md` returns
      **nothing**. Its entire argument lives in a code comment at
      `Composer.qml:231-240`, and that comment is marked **`NO SPEC`**, which
      says the opposite of the truth: the spec now contracts it in full. So the
      record currently tells the next reader that an unmade decision sits there
      for them to change, when it is made, contracted, and argued.
      The comment's argument is a *good* Decisions entry hiding in the wrong
      file — it has the choice, the constraint, both alternatives and the named
      cost ("a user who wanted to post a near-identical follow-up has lost their
      starting point"). It needs lifting into Decisions, and the `NO SPEC`
      marker removing. **This compounds with the spec-test reviewer's open box:**
      their mutations A and B inverted *both halves* of this asymmetry with
      124/124 green, so the decision is currently neither recorded in `design.md`
      nor pinned by a test — which is the definition of a decision made by
      accident. The spec-test reviewer already opened the test half
      (`findings/spec-test.md:27` and `:106`); this box is the record half, and
      the two should be closed together.
      **Verified:** grep returns no hit in `design.md`; the suite is green at
      124 with the asymmetry inverted either way, per that reviewer's measured
      mutations.

- [ ] **`dev-writer`** — `FeedScreen.qml:207-210` — the vote path **deliberately
      does not consult `wasNew`** where the composer treats a missing `wasNew`
      as a refusal, and `design.md` records only the composer's half
      Two publish paths in one change take **opposite policies on the same
      field**, and only one is in the record. `design.md:101-116` ("`wasNew` is
      read as `=== false`, not as falsy") argues at length that a reply carrying
      an `opId` and no `wasNew` "is not core's success shape for this call" and
      must refuse, and calls the strictness "the point". `voteOn` then accepts
      exactly that shape: `if (reply.ok && typeof reply.value.opId === "string"
      && reply.value.opId !== "")` records the vote, `wasNew` unread.
      The choice is defensible — the code comment gives the reason ("a vote
      published twice is the same vote, and both answers mean the viewer's vote
      is on record") — and that is precisely why it belongs in Decisions rather
      than only in a comment. A reader who finds the `wasNew === false` entry and
      then reads `voteOn` sees a rule followed at one call site and missed at
      another, which is the shape CLAUDE.md warns about; without the entry they
      cannot tell a deliberate divergence from a missed one. The entry should say
      what makes a vote different from a post: a vote has no third outcome to
      distinguish, so the field carries no information the view would render.
      **Verified:** `grep -n "wasNew" spec.md` returns nothing — the spec does
      not contract this either, so `design.md` is the only place it can live.

- [ ] **`dev-writer`** — `FeedScreen.qml:529-537` — the **undo press publishes
      nothing**, a choice with a real alternative, recorded nowhere and pinned by
      no test
      `VoteControl` emits `voted(0)` when the viewer presses the arrow they
      already voted (`VoteControl.qml:48,75`). `FeedScreen` drops it:
      `if (direction !== 0) screen.voteOn(...)`. The comment argues it at
      paragraph length — "Core has no vote retraction, so publishing something
      for it would be publishing an op that does not mean what the press meant"
      — and by the "if it needed a paragraph, it was a decision" test that makes
      it a Decisions entry. The alternative is real and someone will reach for
      it: map 0 onto the opposite direction, or onto a fresh publish of the same
      direction. Both are wrong for the recorded reason, and the reason is in a
      file the next person changing `voteOn` may not open.
      It also leaves a **user-visible dead affordance undocumented**: the arrow
      stays pressable and the press does nothing at all — not even a message.
      That is arguably right (nothing honest can be published), but it is a cost
      the record should name, the way the `showScore` entry names its costs.
      **Verified:** `grep -rn "direction !== 0\|voted(0)\|retract"` across
      `tests/` returns **nothing**, and `grep -n "retraction\|undo"` across
      `spec.md` and `proposal.md` returns nothing. Unrecorded, unspecified,
      untested — the three together.
      This connects to the standing note that moderation reversibility is
      suspended for want of a Lamport value; vote retraction is the same gap in a
      second place, and saying so in one line would let a reader find both.

- [ ] **`dev-writer`** — `design.md:221-268` records why the old "the key *is*
      the post" claim was **false**, but never records the **axis the spec
      contracted** — who bears the cost of one malformed row
      The entry is honest and thorough about the correction, and the code matches
      the spec on the axis: `voteTarget()` returns `""`, the row still renders
      (`FeedScreen.qml:540-588` is outside the vote control), the control goes
      non-interactive rather than absent, and no call is reached. All three
      spec-required behaviours hold, and the rule is applied about **rows
      generally** — `voteTarget` guards the row object itself for `null` and
      `undefined` before reaching any field, so it is not written as a
      `currentVersion` special case. Good.
      What is missing is the **alternatives**. The spec spends four paragraphs
      (spec.md:598-613) establishing that the choice between *inert*, *drop the
      row* and *fail the read* is decided by who bears the cost — dropping hides
      peer content on a censorship-resistant forum and hides it silently; failing
      the read lets any peer blank a feed for free and collides with "empty and
      unreadable must never look alike". `design.md` carries none of that. An
      entry with no alternatives reads as though there was no choice, and this
      one had three with a stated tie-breaker. The next person who finds a second
      malformed field will re-litigate it from scratch, and "drop the row" is the
      one that looks tidiest from inside the code.
      **Verified:** `grep -n "malformed\|inert"` in `design.md` returns only the
      two lines of the `currentVersion` narrative; neither alternative is named.

- [ ] **`dev-writer`** — `design.md:337-352` claims the reply path "is tested in
      both modes, so the reply path is exercised rather than merely written" —
      true, but it is the **one claim in this file I could only partly confirm**,
      and it needs a number or a name rather than an adjective
      The architecture reviewer states "nine tests across two files" drive
      `kind: "reply"`. I confirmed `tst_composer.qml` carries
      `test_a_reply_still_carries_its_parent`,
      `test_a_post_and_a_reply_call_different_methods_with_the_right_fields` and
      `test_a_post_given_a_parent_does_not_carry_it_anywhere`, and
      `tst_composer_claims.qml` carries
      `test_the_two_reply_refusals_are_rendered_identically_but_for_cores_text`
      and `test_a_post_and_a_reply_refusal_differ_only_in_the_subject_word` — so
      the claim is **true**, and the reply mode is genuinely exercised.
      The finding is that `design.md` asserts it as prose where the count is the
      load-bearing part: an uninstantiated component's defence is *exactly* how
      much of it runs, and "is tested in both modes" survives a future change
      deleting four of those five tests. Name them, or name the count as the
      architecture file does. **Severity: low** — this is a suggestion about an
      entry that is otherwise correct, not a defect.

## On the two rewritten justifications

Both rewrites this piece has been through landed correctly, and I checked each
against the code rather than against the prose that describes the fix.

**The vote-key claim** ("by construction … the key *is* the post") was found
equally wrong about any other row field, and the rewrite at `design.md:232-248`
says so in as many words — it names the general shape ("a property of
peer-supplied data, not of the code") rather than only patching the instance.
The spec generalised in the same direction at spec.md:614-619. No stale residue:
`grep` finds no surviving copy of the original claim anywhere in the change.

**The missing-box decision at the old `design.md:211`** ("moves into the
apparatus column where it belongs") went stale against a spec change and is now
corrected at `design.md:301-335`, which states the old position, why it was
right against the spec as it stood, and why it is now wrong twice over. The code
matches: `FeedScreen.qml:734` renders the statement as a `Text` in the closed
gate's own `ColumnLayout`, and `test_the_missing_box_statement_is_in_the_gates_own_body`
asserts **placement** via a walker that skips the column's subtree — so the
requirement now fails for the right reason if the sentence moves back. The
apparatus list at `FeedScreen.qml:780-789` carries a comment saying nothing
load-bearing may live there. This is a stale claim caught and fully repaired.

## What I could not check

- **That `compose.apparatus`, `compose.fix` and `compose.blockedTitle` are the
  bundle's strings.** `copy.json` **is not in this repository and never has
  been** (`git log --all -- "**/copy.json"` is empty), and `docs/UI-BRIEF.md`
  does not mention it. So every "copy.json `x`, verbatim" claim — in
  `design.md`, in `FeedScreen.qml:721`, and in the spec's **SHALL** at
  spec.md:90 — is unverifiable from inside this repo, and the test at
  `tst_vote_and_gate.qml:780` pins a **hardcoded literal**, not the bundle. A
  pin against a literal fails on reword, never on divergence from the bundle.
  This is a **pre-existing condition rather than this piece's defect** —
  `SanitisedText.qml` carries the same convention and predates the change — so I
  have not opened a box. It is worth a spec-writer's eye that this change newly
  makes a spec `SHALL` depend on a file no gate here can read.
- **Anything about rendering.** QtTest drives properties and signals, never
  pixels. `design.md`'s own "What the tests can and cannot see" says this and is
  accurate.
- **The three staged, uncommitted test files** in the piece worktree. My
  worktree is cut from the pushed branch, so I reviewed what is on the branch.
  Where a gap I name might be closed by that work, I have said so: the
  draft-clearing box above is the record half regardless — an uncommitted test
  would close the spec-test reviewer's box, not this one, since `design.md`
  carries no entry either way.
- **The merge with `piece/drop-apparatus` (#70).** The architecture reviewer's
  open box addressed to me concerns two `MarginNote`s this piece adds to a column
  that branch deletes, and the `otherKnownDenials()` entry that excludes one from
  a sweep without pinning it. I agree with their reading and their recommended
  resolution — drop both notes with the column and delete the dead exclusion in
  the same change — and I confirmed the asymmetry they describe: the exclusion is
  acceptable only because the sentence is leaving the tree, which makes it a
  coordination item for whoever resolves that merge rather than a defect here.
  I did not run the merge.
