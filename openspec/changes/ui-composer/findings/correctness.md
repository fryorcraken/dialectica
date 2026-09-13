# Correctness — `ui-composer`

Reviewed on `piece/ui-composer` at `bb3219c`, in a worktree of its own. The
whole QML suite was run before and after each mutation
(`dialectica-ui/tests/run-qml-tests.sh`): **107 tests across 8 spec files, all
green on the unmutated tree.** Every mutation below was verified to land — the
suite was re-run and the named test actually changed state — and every one was
reverted; `git status --porcelain` is empty.

## What is clean

The composer's state machine is the strong half of this piece and I could not
break it. `applyReply`'s fall-through **is total**: `Core.call()` already
rejects `null` and non-objects, so `reply.value` is always an object, and the
success branch is guarded on `typeof reply.value.opId === "string" &&
opId !== ""` followed by `wasNew === true` / `=== false` with an explicit
refusal underneath. I probed a JSON array reply (`typeof [] === "object"`,
`[].error === undefined`, so it reaches `applyReply` as `ok: true`) — `opId` is
`undefined`, so it refuses. A reply carrying `opId` but no `wasNew`, `wasNew`
as the string `"true"`, or `wasNew: null` all reach `refused`. There is no
reply shape I found that produces a claim it should not.

`utf8Length` is correct at every boundary I fed it, including the one it was
written for: `"a\ud83cb"` (lone high surrogate) and `"a\udfdbb"` (lone low
surrogate) both count 5 and neither throws, where the rejected
`encodeURIComponent` one-liner would have raised `URIError`. The astral case
(`"🏛"` → 4 bytes, 2 code units) is the one a `.length` composer gets wrong and
it is right here.

The byte cap agrees with core rather than drifting: `Composer.bodyByteLimit` is
`150 * 1024`, `op::MAX_FIELD_LEN` (op.rs:146) is `150 * 1024`, and
`authoring::MAX_BODY_LEN` (authoring.rs:206) is that constant. The comparison
senses match too — core refuses `body.len() > MAX_BODY_LEN`, the view sets
`overLimit` on `draftBytes > bodyByteLimit`, so at-cap is submittable on both
sides and there is no off-by-one band where the view permits what core refuses.

`isInvisible`'s eight ranges are character-for-character the eight arms of
`sanitise::is_invisible` (sanitise.rs:141-152). The `NO SPEC` is accurate: it
counts removals and not homoglyph marks, which is what the spec's lines 176-199
now explicitly scope it to.

`recordVote` builds a new object rather than mutating, which is the QML
property-change trap named correctly. `ownVotes` keyed by the post's op makes
cross-marking unrepresentable as claimed.

The `submitAgainst()` singleton-bridge helper holds everywhere it is needed:
`tst_composer_claims.qml` and `tst_composer.qml` both create-and-submit before
the next bridge assignment, and `tst_vote_and_gate.qml` / `tst_gate_affordance.qml`
assign the bridge inside `makeScreen()` immediately before `createObject`. I
found no site that builds two components against one bridge and then drives both.

## Findings

- [ ] **`dev-writer`** — `PublishOutcome.qml:78-79` — the deduplicated success
      carries no delivery denial, and the spec requires one on every success
      **Scenario:** submit a draft against a core replying
      `{"opId":"deadbeef","wasNew":false}`. The view renders "This post was
      already published." and "The identical content is already in this
      machine's log, under the same op id. Nothing new was written." Neither
      sentence says anything about delivery. The spec's requirement "A
      successful publish claims local storage and never delivery" (spec.md:265)
      opens "When a publish succeeds" and its promoted clause (spec.md:280-284)
      says the view **SHALL** state that whether any other peer has received the
      content is not something it can report, "with the success itself". A
      `wasNew: false` publish *is* a success — the piece's own design says so
      ("nothing failed, so a refusal is wrong"), and the code routes it to a
      non-refusal outcome with `isRefusal` false. So the denial is owed here and
      is absent. The `stored` branch (line 80) has it; this branch does not.
      **Measured:** the spec commit that promoted the denial (`75930e7`) states
      "Two code changes follow and are NOT made here". `git log piece/ui-composer
      -- dialectica-ui/src/qml/PublishOutcome.qml` returns one commit, `540e317`,
      which predates `75930e7` — so neither follow-up landed. **Severity: high.**
      This is the piece's stated substance: an author who resubmits after a
      network glitch sees "already published" and is given no reason to doubt it
      went somewhere, which is precisely the belief the promoted requirement
      exists to unsettle.

- [ ] **`tester`** — `tst_composer_claims.qml:545-548` — the pinned-sentence
      residue check **actively forbids** the fix to the finding above
      **Scenario:** this is not merely an untested gap; the test rejects
      compliance. `pinnedSentences()` lists exactly two sentences for the
      `existing` row, and `test_the_views_own_words_are_exactly_these_and_no_others`
      strips those two plus core's detail and asserts the whitespace-stripped
      residue is `""`. Adding the spec-required denial to line 79 therefore
      fails. **Measured:** I appended "Whether any other peer has received it is
      not something this software can tell you yet." to `PublishOutcome.qml:79`
      and re-ran the suite —
      `FAIL: test_the_views_own_words_are_exactly_these_and_no_others ... Residue:
      "Whetheranyotherpeerhasreceiveditisnotsomethingthissoftwarecantellyouyet."`,
      106 of 107 passing. The `existing` row needs the denial added to its
      `sentences` list in the same change that adds it to the component.
      **Severity: high** — it is the reason a dev-writer acting on the finding
      above will think they got it wrong.

- [ ] **`dev-writer`** — `FeedScreen.qml:568-647` — the closed gate's own body
      never states why there is no compose box; the statement lives only in the
      apparatus column the spec says to disregard
      **Scenario:** the spec requires the view to state, **"in the closed gate's
      own body"**, that no box is shown and why (spec.md:87-92), and spells out
      that the obligation "SHALL NOT be discharged by placing it anywhere a
      reader of the gate would not encounter it" (spec.md:100-101), with a
      scenario that disregards "every region of the screen given over to
      annotating the design" (spec.md:152-155). The only place that sentence
      appears is `FeedScreen.qml:657-661`, a `MarginNote` in the `apparatus:`
      list. The closed-gate `ColumnLayout` body carries the heading, core's
      reason, the fix button and the guidance text — none of which says a box is
      missing or why. **Measured:** I set `visible: false` on `ApparatusColumn`
      in `ScreenFrame.qml` and re-ran `tst_vote_and_gate.qml`: exactly one test
      failed (`test_the_apparatus_string_is_the_bundles_and_is_verbatim`,
      20 of 21 passing), and it fails because it asserts the *column's* string —
      so once the column goes, the requirement is unmet with nothing failing for
      the right reason. `design.md:211` records the opposite decision
      ("`compose.apparatus` moves into the apparatus column where it belongs"),
      which was correct against the pre-`75930e7` spec and is now stale.
      **Severity: medium.** The gate is honest today because the column is still
      rendered; the defect is that the obligation is pinned to something being
      removed.

- [ ] **`tester`** — `tst_composer_claims.qml:313-330` — the delivery-claim
      needles over-match into the honest denial, so an equivalent reword of the
      required sentence is reported as a forbidden claim
      **Scenario:** the sweep list contains `"received by"` and `"was received"`.
      The denial the spec requires is a statement *about* reception, so the
      current wording ("Whether any other peer **has received** it...") escapes
      only because the list happens to spell `"has been received"` rather than
      `"has received"`. I rewrote line 80 to "Whether it **was received by** any
      other peer is not something this software can tell you yet." — semantically
      identical, still a denial, claiming nothing. **Measured:** four tests fail
      across two files — `test_each_outcome_implies_what_it_must_and_denies_what_it_must_not`,
      `test_no_composer_state_claims_delivery`,
      `test_the_views_own_words_are_exactly_these_and_no_others` and
      `tst_composer.qml:352 test_a_success_names_local_storage_and_claims_no_delivery`
      — each reporting "must not claim 'was received'". This is the needle
      defect the brief names: the corpus now contains the required denial, so
      needles phrased as bare participles cannot separate a claim from its
      negation. The fix is to exclude the pinned denial sentence from the sweep
      corpus (as `test_no_gate_state_claims_delivery` already does at line 446
      for the apparatus note) rather than to keep the wording tiptoeing around
      the list. **Severity: medium** — nothing is wrong on screen today, but the
      sweep will fight the fix to the first finding above.

## Judgement on the absence sweeps

`deliveryClaims()` is a good list for what it is and the file says so honestly
in its own comment ("it is a filter, not a proof"). The needles are drawn from
the phrasings a reassuring reword actually reaches for, and the inclusion of the
imperative forms (`"send the "`, `"send this "`, `"send it"`) after a mutation
found the past-tense-only gap is the right kind of repair. The corpora are
right too: `test_no_composer_state_claims_delivery` sweeps seven states rather
than only the success, and `test_no_gate_state_claims_delivery` reveals
`showFix` first, with a precondition assertion proving the reveal took — that
precondition is what stops the test passing over collapsed guidance.

Two weaknesses, one recorded above as a finding and one not worth a box:

- The over-match into the denial (finding 4).
- `claimCases()`'s `mustNotSay` for `existing` includes `"could not"`, which is
  a two-word English fragment that would match honest copy such as "the core
  module could not be reached" if that ever reached this block. It cannot today
  (the `existing` branch renders no core text), so it is a latent over-match
  rather than a defect; noting it in prose rather than opening a box.

Nothing in the sweeps matches a comment rather than rendered text: I grepped the
QML for every needle stem and every hit was inside a `//` comment, which
`renderedText` never reaches.

## What I could not check

- **Anything about rendering.** QtTest drives properties and signals, never
  pixels, as the test file states. A message can satisfy every assertion and be
  4pt grey on grey.
- **The transport defect itself.** `transport.rs:1947`'s
  `a_body_at_the_authoring_cap_encodes_past_the_message_limit` reproduces the
  140-byte overshoot in core's suite, but nothing in this repo has a second
  peer, so no test here can observe a post being silently refused. The view's
  obligation is the only mitigation and that is what findings 1 and 3 are about.
- **`qmllint` clean.** I ran it with `--bare`, which broke module resolution and
  produced 177KB of import noise. CI's invocation (ci.yml:562) uses
  `-I dialectica-ui/src/qml` without `--bare` and is the authority. The
  `textFormat` counting gate (ci.yml:262-274) I did check by hand against
  `Composer.qml`: 3 `\bText \{` openings against 4 `textFormat:` assignments,
  so it passes — `TextEdit {` is not matched by the pattern and its own
  `textFormat: TextEdit.PlainText` is counted, which happens to make the file
  pass with margin rather than by exact accounting.
