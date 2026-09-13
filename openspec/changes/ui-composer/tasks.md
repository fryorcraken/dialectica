# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

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

- **The `compose.apparatus` requirement collides with removing the apparatus
  column, and that collision is flagged rather than resolved.** The owner has
  said the right-hand `APPARATUS` column is design-bundle annotation shipped
  into the QML by mistake and is being removed from the screens. The spec
  (`composer-view`, "A closed gate shows the reason verbatim and offers a fix")
  still requires the view to state why no box is shown "using the bundle's
  `compose.apparatus` string", and `tst_vote_and_gate.qml`'s
  `test_the_apparatus_string_is_the_bundles_and_is_verbatim` asserts it is
  rendered verbatim. Hiding `ApparatusColumn` in `ScreenFrame.qml` fails exactly
  that one test across all eight spec files — measured, not predicted. The test
  is left standing with a long comment explaining the choice: deleting it would
  quietly drop a requirement the spec still makes. Either the spec stops
  requiring the string, or the sentence moves into the closed gate's own body.
  Both are spec-writer decisions.

- **Once the apparatus column goes, nothing asserts the interface positively
  DENIES delivery knowledge.** The denial ("whether any other peer receives it
  happens later and is not reported back here") lives only in the `ON PUBLISHING`
  MarginNote. The tester's delivery tests are absence sweeps and survive the
  removal unchanged, which also means they cannot notice that the honest
  disclaimer went with it.
