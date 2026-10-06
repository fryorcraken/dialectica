# Design review: reply-caption-no-edit

The recorded decisions are in good shape. Read against the tree: the caption is
`"A reply is a signed record."` with `objectName: "replyCaption"` inside
`replyComposerOpen`; no editing was built; the claim matcher, the scope pin, the
walk anchors and the `NO SPEC:` caption test all exist as Decisions 1 and 3
describe; and the grep Decision 1 cites (`revise` over `lib.rs`, `Core.qml`,
`content-authoring`) returns nothing, as stated. `tst_thread_reply.qml` runs 27
passed, 0 failed on this tree. No code contradicts a decision, and nothing
contradicts issue #177: its two options were "drop the sentence to what is true
today" or build editing, and the owner's comment picks the first. Both
below are gaps or suggestions, not code defects.

- [ ] **`dev-writer`** — `design.md` Decision 3 — a test-side choice with a real
      alternative is recorded only in a test comment, not in Decisions (gap).
      `stringsUnder` walks every descendant's `text` **ignoring `visible`**
      (`tst_thread_reply.qml`, comment above `stringsUnder`: "stricter than
      'rendered'"). The alternative, walking only visible items, would let a claim
      that is hidden and shown later pass. Decision 3 covers the scope of the walk
      and the placeholder and popup limits, but not this, and the entry's own
      guard list asserts nothing that would catch a switch to a visible-only walk.
      Add one sentence under Decision 3 naming the choice and what it rules out.

- [ ] **`dev-writer`** — `DThreadScreen.qml:732-733` — pointer to a document that
      is about to move (suggestion). The caption's comment says "The
      reply-caption-no-edit change's design.md says why the first survived the
      cut." After archive that file is at
      `openspec/changes/archive/<date>-reply-caption-no-edit/design.md`. The
      change name still greps, so the pointer is not dead, but the next toucher of
      the caption is told to look in a place that is not a live path. Either name
      the reasoning in the comment in one clause (the cut dropped the weaker false
      promise and kept the stronger one; `design.md` Decision 2 already says so),
      or accept the grep-by-name and say so in the comment.
