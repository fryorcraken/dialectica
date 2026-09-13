# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
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

**Not done, and named rather than left to be found:**

- The reply composer is not instantiated anywhere. `Composer.kind: "reply"` is
  built and tested, but this feed lists thread heads and a reply box under one
  would be a thread-view affordance on a screen that is not one. It arrives with
  the thread screen.
- The invisible-character warning counts only what core's sanitiser REMOVES, not
  what it MARKS. The spec asks for both; the marked half is a homoglyph
  judgement that a second QML implementation would get differently from core.
  Marked `NO SPEC` in `Composer.qml` and argued in `design.md`.
