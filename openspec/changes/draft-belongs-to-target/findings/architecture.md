# Architecture review: draft-belongs-to-target

Dimension: architecture only. All three entries are low severity and are
shape or naming preferences, not defects: nothing below produces a wrong
publish.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DComposer.qml:332,356,390-391` —
      the new `published` local and `applyReply` parameter share a name with the
      component's own `published()` signal (line 151), and the two meanings sit on
      adjacent lines: `root.clearDraftOf(published)` then `root.published()`.
      **Scenario:** a reader of `applyReply` sees `published` as the key of the
      target and, one line down, `root.published()` as the signal; a later edit
      that drops the `root.` qualifier on the signal call, or reaches for
      `published` meaning the signal inside `applyReply`, silently resolves to the
      string parameter and throws "not a function" only on the stored path.
      Rename to what it holds (for example `publishedKey`). **Stylistic**, low;
      no behaviour changes.

      **Fixed** (`dev-writer`), in the commit that ticks this box: the local in
      `submit()` and the parameter of `applyReply` are both `publishedKey`. No
      behaviour changes, so no test fails without it; `tst_composer.qml` and
      the whole suite pass after the rename.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DComposer.qml:88-102` —
      `heldDrafts` is declared `property var` but is deliberately mutated in place
      so that it never notifies; the only protection against someone binding to
      it is a comment (and `design.md` Risks: "A future binding over it would
      silently never update"). **Scenario:** a later change adds
      `visible: root.heldDrafts[someKey] !== undefined` (for example to mark a
      Stoa that has a draft); it evaluates once and never again, with no error.
      The shape puts the complexity in a caution rather than the data: either
      make `holdDraft` replace the object (copy-on-write; the map holds one entry
      per target with unsubmitted text, so the copy is cheap) so the property
      really notifies, or keep the map out of the property system. **Design
      preference**, low; the current form is documented and works as stated.

      **Rejected** (`dev-writer`). Three reasons. Copy-on-write copies every
      key on each keystroke, and the owner settled that the map has no cap, so
      the per-keystroke cost would grow with a number nothing bounds, to serve
      a binding that does not exist. The scenario's own example, marking a Stoa
      that has a draft, is an indicator that a draft was kept, which the
      delta's "A restored draft is not announced" forbids, so the nearest
      reader of such a binding is one the spec rules out. And the second
      option, state outside the property system, means moving the map into an
      imported script: a second file, and a second lifetime to get right, for
      one variable. I did not try it. A change that does need to read the map
      reactively should add a signal or a counter then, with its reader in
      view. The argument is recorded in `design.md`, Risks, beside the entry
      this finding quotes, in the commit that ticks this box.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DComposer.qml:75-80` —
      `targetKeyOf(kind, stoaAddress, parent)` is a public function of the
      component with exactly one caller, the `targetKey` binding on the next line,
      which passes the component's own three properties. No screen and no test
      calls it (`git grep targetKeyOf dialectica-ui` hits only this file), so it
      widens the component's scanned surface without a second consumer. Its third
      parameter is also named `parent`, which shadows `Item.parent` inside the
      function. **Scenario:** none breaks; it is surface a reader must ask "who
      else calls this?" about. Inline the `JSON.stringify` into the binding, or
      rename the parameter to `replyParent` if the function is kept for a
      future caller. **Stylistic**, low.

      **Fixed** (`dev-writer`), in the commit that ticks this box: the function
      is gone and `targetKey` is the `JSON.stringify` binding itself, so the
      shadowing parameter went with it. `git grep targetKeyOf` over
      `dialectica-ui`, `design.md` and `tasks.md` now returns nothing. The key
      is unchanged, and the whole suite passes.

## Clean

- **Where the state lives.** The per-target map sits in `DComposer`, the one
  component that both reads and writes it, with the lifetime the spec requires
  (a composer lives exactly as long as its screen and the view). Nothing is
  threaded through `Main.qml` or the two screens: `FeedScreen.qml` and
  `DThreadScreen.qml` are untouched, and the diff to the navigator is empty.
  The alternatives in `design.md` Decision 1 (singleton, store on `Main.qml`,
  recreating the composer) each cost more structure than the map inside the
  component, and the argument holds against the code.
- **The core/UI split.** Held drafts reach nothing but the two publish calls in
  `submit()`; no new core method and no change to `Core.qml`.
- **Key construction.** `JSON.stringify([kind, stoaAddress, replyParent])` is
  injective, built from `replyParent` (what reaches core) and not `parentOp`,
  and `heldDraft` returns only a `string`, so a peer-supplied op id cannot
  select an inherited property.
- **One handler for the safety rule.** `onTargetKeyChanged` re-fills the field
  on the change itself, and every edit is held under the current key from the
  field's own `onTextChanged`. The rejected alternative (publish reads the map)
  does give the body two sources; keeping the field as the single source for
  `submittable`, the byte count and the publish is the right call. The
  `showDraftOfTarget` write re-enters `holdDraft` with the same key and text,
  which is idempotent.
- **Stale-answer guard.** Taking `targetKey` before the call and clearing under
  that key in `clearDraftOf` is a small, separable job, and the `key ===
  root.targetKey` branch keeps the field untouched when the composer has moved
  on. `clearDraft()` had no remaining caller and was removed; no stale
  reference to it is left in the sources (the one mention in `tst_composer.qml`
  is a historical account of a mutation, not a call).
- **Visit versus target.** `clearOutcome`/`beginVisit` are untouched and the
  new comment correctly separates the two lifetimes: the outcome belongs to a
  visit, the draft to a target. `Main.qml`'s `enterOnly` clears the other
  states before setting one, so a Stoa-to-Stoa move passes through `list` and
  begins a visit; the outcome cannot outlive the target it describes.
- **Dependencies.** None added. No file moved or renamed, so no CI gate is left
  measuring a directory that no longer holds what it checks.
- **Test helpers.** `tst_draft_targets.qml` copies `visibleNodes` and
  `visibleNamed` from `tst_publish_outcome_visits.qml` and says so in a
  comment. The repo has no shared test-helper convention (`visibleNamed` already
  has four copies across the suite), so that follows the existing pattern and is
  not raised as a finding against this piece.
- **Not run.** I did not mutate or run the suite: this is a shape review, and
  the `spec-test` and `correctness` reviewers hold the measurement.
