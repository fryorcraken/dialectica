# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — closer
- [x] `openspec validate --strict`, then `archive` — closer

## Implementation

- [x] `Core.qml` gains `publishPost`, `publishReply`, `publishVote` on the
      existing `call()`, so every publish goes through the one error branch.
- [x] `Composer.qml`: the open-gate composer, one component for post and reply.
      Holds the draft, the UTF-8 byte count, the invisible-character warning, and
      one `outcome` string covering stored / existing / refused.
- [x] `PublishOutcome.qml`: renders the three outcomes from that one string, so
      no two can be on screen at once.
- [x] `VoteControl.qml` gains `showScore`, defaulting false, suppressing the
      number. `score` and its floor stay, unbound.
- [x] `FeedScreen.qml`: the open branch of the gate, the corrected closed branch
      (heading naming both affordances, `compose.fix`, no delivery claim), a
      vote control per row reading `ownVotes`, and a re-read after a publish.
- [x] `qmldir` registers `Composer` and `PublishOutcome`.
- [x] `tst_composer.qml` and `tst_vote_and_gate.qml`, including tests that fail
      on the bundle's false copy so a future paste is caught. Seven mutations of
      the implementation were each caught; they are listed in the handover.
- [x] `docs/UI-BRIEF.md` corrected where this change makes it wrong: the
      deduplicated-publish section offered "scroll to and highlight the existing
      post", which the view cannot do for a reply.

- [x] `tst_composer_claims.qml` and `tst_gate_affordance.qml` — the tester's two
      files. The first pins what each outcome must and must not IMPLY (the
      sibling `thread-read` piece proved distinctness cannot see meaning); the
      second probes the gate by writing into every candidate rather than by
      recognising a property name. Ten mutations, each verified to land and each
      caught; five of them left the pre-existing 88 tests entirely green.

**Not done, and named rather than left to be found:**

- The reply composer is not instantiated anywhere. `Composer.kind: "reply"` is
  built and tested, but this feed lists thread heads and a reply box under one
  would be a thread-view affordance on a screen that is not one. It arrives with
  the thread screen.
- The invisible-character warning counts only what core's sanitiser REMOVES, not
  what it MARKS. The spec asks for both; the marked half is a homoglyph
  judgement that a second QML implementation would get differently from core.
  Marked `NO SPEC` in `Composer.qml` and argued in `design.md`.

- **The `compose.apparatus` collision is resolved, and the resolution was the
  second of the two readings offered.** The spec-writer chose "the sentence moves
  into the closed gate's own body" over "the spec stops requiring the string", so
  the requirement is now on the statement being present *where the gate is
  rendered* — explicitly not discharged by placing it where a reader of the gate
  would not meet it. `test_the_missing_box_statement_is_in_the_gates_own_body`
  asserts placement rather than presence, via `renderedTextOutsideApparatus`, so
  it survives the column's removal on `piece/drop-apparatus` (#70) rather than
  failing with it.

- **This piece adds nothing to the apparatus column, deliberately.** It grew two
  MarginNotes during development — `ON PUBLISHING` and `ON THE ARROWS` — and both
  were removed before merge. Neither was required (the spec says the score's
  absence "does not oblige the view to carry prose about why no number is
  there"), so keeping them would have handed #70's merge a decision about
  sentences nothing requires. `ON PUBLISHING` was the worse of the two: a second
  delivery denial that `tst_composer_claims.qml` excluded from its sweep and
  pinned nowhere, which made it the one place a delivery claim could be reworded
  in with no test failing. The exclusion is deleted with the note, and a new
  assertion in `test_the_sweep_filter_drops_only_the_pinned_denial` fails if
  anyone re-adds it.

- **The interface positively DENIES delivery knowledge, and that does not depend
  on the apparatus column.** The denial is rendered by `PublishOutcome` beside the
  success it qualifies and pinned character-for-character by
  `test_the_views_own_words_are_exactly_these_and_no_others`. The absence sweeps
  remain absence sweeps — they can never prove something required was said — but
  the positive half is carried by the pins, which is a different instrument doing
  a different job.
