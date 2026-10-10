# Design review: draft-belongs-to-target

No recorded decision is contradicted by the code, and none is partially
applied. Checked against `DComposer.qml`:

- Decision 1 (`heldDrafts`, field re-filled on key change, every edit held as
  made): `onTargetKeyChanged`, `showDraftOfTarget`, `field.onTextChanged`.
- Decision 2 (`JSON.stringify([kind, stoaAddress, replyParent])`): `targetKeyOf`
  and `targetKey` match.
- Decision 3 (`submit()` still sends `root.draft`): confirmed.
- Decision 4 (key read before the call, handed to `applyReply` then
  `clearDraftOf`): confirmed.
- Decision 5 (empty drops the entry): `holdDraft` does it.

The Context claims hold. `Main.qml` mounts one `FeedScreen` and one
`DThreadScreen` and re-points them through `enterOnly`. Both composers are gated
by `visible:`, not a `Loader`, so a gated composer stays mounted and keeps its
map. The seven owner answers in the issue's "Decisions" comment are each
reflected in the design or the spec delta. Nothing contradicts the issue's
scope. What follows is gaps and thin entries only.

Not verified by me: Decision 3's "sixteen tests go red" and Decision 4's "exactly
one test red". Checking either means editing source, which I do not do.
`tst_draft_targets.qml` holds 37 `test_` functions, so sixteen of them being the
cross-target ones is plausible.

- [ ] **`dev-writer`** — `design.md` Decision 2 describes a guard with no
      mutation evidence (a suggestion). The key is the thing that stops two
      targets sharing a draft, and the entry names three separate choices that
      each prevent a distinct failure: the parent rather than the thread, the
      Stoa as well as the parent, and JSON rather than a joined string.
      Decisions 3 and 4 say which tests go red when their guard is removed.
      Decision 2 says nothing of the sort, although the last commit
      (`d4aee312`, "pin the key's injectivity ...") added tests for exactly
      these. Add, per choice, what turns red when it is removed: keying on
      `parentOp` rather than `replyParent`, dropping `stoaAddress`, and joining
      with `":"`. If one of them turns nothing red, say so, because that makes it
      a decision pinned only by a later test.

- [ ] **`dev-writer`** — `design.md` omits the issue's "Left out on purpose"
      reasoning (a gap). The issue's Decisions comment records two things this
      change acted on: a draft is not tied to an identity because 0.0.1 ships one
      identity per user, and the outcome rule is unchanged, so a restored draft
      comes back without the outcome of the earlier visit's publish. The
      identity point is in `proposal.md:37` ("The view has one identity") and
      nowhere in `design.md`. The outcome point is in the spec delta and a code
      comment on `clearOutcome`, not in `design.md`. Both archive with the
      change, but the identity one is the "why not X" a later reader asks when a
      second identity appears: the key then needs an identity component, or a
      draft typed under one identity can be published under another, and that is
      the same signed-op disclosure the change exists to prevent. Record it in
      Decisions or Risks, with that cost and the condition that reopens it.

- [ ] **`dev-writer`** — `design.md` Decision 5 is thin (a suggestion). It names
      the chosen behaviour (drop an emptied draft) and one reason (the map holds
      only unsubmitted text). It names no alternative beyond "held as `""`" and
      says what that would cost only by implication. It does not say the choice
      is unobservable, and that is worth saying: both read back as an empty
      field, so no test can tell them apart, and the "no cap" decision is the only
      thing the drop bears on. One sentence saying nothing observable depends on
      it would stop a later reader treating it as a guard.
