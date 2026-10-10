# Findings: correctness

Dimension covered: correctness only.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DComposer.qml:356,390` — `applyReply(reply, published)` silently stops clearing the draft when `published` is omitted
      **Scenario:** `applyReply({ok: true, value: {opId: "x", wasNew: true}})` (the pre-change one-argument call shape; `applyReply` is a public, non-underscored function and the only thing keeping a store and a cleared draft together) sets `outcome = "stored"` and emits `published()`, then calls `clearDraftOf(undefined)`. That deletes `heldDrafts["undefined"]` and skips `field.text = ""`, because `undefined === root.targetKey` is false. The text stays in the field and in `heldDrafts`, so the user sees "stored" with the same text still offered for a second Publish. An op is permanent and a second publish is a second signed op, so this failure mode is not cosmetic.
      **Severity:** low. Nothing in the tree calls `applyReply` other than `submit()`, which passes the key, so no current path reaches it. This is a hardening gap in a changed signature, not a live defect. A default of `root.targetKey` for the second parameter, or a refusal on a non-string key, would remove it. Whichever is chosen belongs with a test, because nothing exercises the one-argument form (`git grep` finds `applyReply` named only in comments under `dialectica-ui/tests/`).
      **Measured:** the one-argument call was reasoned from the code, not run. The two-argument path was run and is green.

      **Fixed** (`dev-writer`), in the commit that ticks this box: the key parameter defaults to `root.targetKey`, which is what the one-argument form meant before this change. The refusal was not taken because it would report "refused" for an op that was stored. The test is `test_a_stored_reply_applied_without_a_key_clears_the_draft_shown` in `tst_composer.qml`. I ran the scenario first, against the unchanged composer: it failed at "a store never leaves its text in the field", with the text still in the field, so the reasoning in this entry holds when run. It passes with the default. `design.md` Decision 4 records the choice.

## Areas checked and found clean

- **Target key.** `JSON.stringify([kind, stoaAddress, replyParent])` is injective for any peer-supplied op id, lone surrogates and quote or backslash characters included. Every key begins with `[`, so no key can name an `Object.prototype` member, and `heldDraft` returns only a `string`.
- **Re-fill on a change of target.** The key is a readonly binding, so its value is updated before `onTargetKeyChanged` runs. The `onTextChanged` write that the re-fill itself triggers therefore lands under the new key and writes back the same text. Intermediate keys, such as a new Stoa with the old parent while `Main.qml` writes the thread screen's address then its id, show only an empty entry in practice. Even when one held text, the next change replaces it before any control can act.
- **Entry removal.** `holdDraft` deletes an entry only when the current key's text became empty, so a transient key cannot delete another target's draft.
- **Clearing on store.** The key is read before the call in `submit()`, and `clearDraftOf` clears the field only when the composer still points at that key. I removed the `holdDraft(key, "")` line from `clearDraftOf` to check. `test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named` failed, so the ordering is pinned. I reverted the edit with `git checkout`.
- **Other outcomes.** `existing` and `refused` keep the draft, as before.
- **Mounting.** Only `FeedScreen` and `DThreadScreen` mount `DComposer`. The post composer ignores `parentOp` through `replyParent`.
- **Text normalisation.** `TextEdit`'s plain-text getter already folds NBSP and U+2028 on the publish path, and the held text is that getter's value, so a restore is idempotent and publishes what was shown. This was not changed by the piece.
- **Suite.** `tst_draft_targets.qml` runs 39 passed, and the whole QML suite (29 spec files) exits 0 with no failure or undefined-binding line.
