- [x] **none** — no security findings

Dimension covered: security only.

What was checked, against `DComposer.qml` as it stands on the piece branch
(`git diff origin/main...HEAD`), and `FeedScreen.qml`, `DThreadScreen.qml` and
`Main.qml` as they mount it:

- **The harm the issue names (text signed into a Stoa or under a parent it was
  not written for).** `submit()` reads `root.draft` (the field) and
  `root.stoaAddress` / `root.replyParent` in the same synchronous turn.
  `onTargetKeyChanged` rewrites the field in the same turn as the retarget, and
  `onTextChanged` holds every edit under the key current at the time, so the
  field never holds another target's text while the target differs. Every
  intermediate target `Main.qml` passes through (address emptied, id withheld
  until address and genesis land) is handled like any other and cannot be
  submitted from, since the composer is not visible in those states.
- **Key injectivity with peer-supplied parts.** The key is
  `JSON.stringify([kind, stoaAddress, parent])`, so no op id or address
  character can make two targets share a key. The key always begins with `[`,
  so it cannot name `__proto__`, `constructor` or any other inherited property
  of the plain-object map. `heldDraft` also type-checks the value read back.
- **Stale answer clearing the wrong draft.** `submit()` captures the key
  before the call and `applyReply` / `clearDraftOf` act on that key, not on the
  field. Only a newly stored op clears; `existing` and `refused` keep the draft,
  as the spec requires.
- **Held text leaving the view.** `heldDrafts` is a QML property in the
  component's memory only; no write to storage, no core call other than the body
  of an explicit publish (the `tst_draft_targets.qml` test for this passes).
  Nothing peer-controlled adds entries: they are created only by the local
  user's edits, so the unbounded map is user-driven and not an input-driven
  allocation.
- **Undo reaching across targets.** Assigning `text` resets the document's undo
  history; the corresponding test passes.
- Ran `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_draft_targets.qml`:
  39 passed, 0 failed. No mutations were made to the tree.

Not a finding, noted for the record: drafts of Stoas the user has left remain in
view memory until the view ends. That is the spec's decision (a draft belongs to
its target and is not discarded on leaving), and it is not persisted.
