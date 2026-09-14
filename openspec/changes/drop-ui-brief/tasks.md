# Tasks — drop the UI brief

## Stages
- [x] ~~spec~~ — **no spec delta.** `grep -rn "UI-BRIEF\|UI brief"` over
      `openspec/specs/` returns nothing: no live requirement cites the brief, so
      none is modified or deleted. See `.openspec.yaml`.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## 1. Survey

- [x] 1.1 Grep `UI-BRIEF\|UI brief\|ui-brief\|UI_BRIEF` across `docs/`,
      `dialectica/`, `dialectica-ui/`, `CLAUDE.md`, `.claude/` and
      `openspec/specs/`. Found **21 live citations**, seven more than the
      dispatch listed — see `design.md` §1 for the table of what was missed.
- [x] 1.2 Confirm no live spec cites the brief. `grep` over `openspec/specs/`
      returns **nothing**, which is what makes `skip_specs: true` correct rather
      than convenient.
- [x] 1.3 Establish that `openspec/changes/archive/` is out of scope. 23
      citations there, all left untouched.

## 2. Delete the document

- [x] 2.1 `git rm docs/UI-BRIEF.md`.

## 3. Repair the prose that cited it

- [x] 3.1 `CLAUDE.md` — remove the row from the "Where to look for what" table,
      and confirm the remaining table and its introductory note still read as
      coherent.
- [x] 3.2 `docs/PLAN.md` §5.5 (`:2446`) — the quoted "never auto-join" rule
      becomes PLAN.md's own sentence. **Claim stated.**
- [x] 3.3 `docs/PLAN.md` §5.5 (`:2451`) — "the brief describes only what does"
      becomes the real reason: no surface exists for these to be contracted
      against. **Claim restated.**
- [x] 3.4 `docs/PLAN.md` §5.5 (`:2484`) — the policy-layer claim is stated, and
      named by its three concrete instances rather than delegated. **Claim
      stated.**
- [x] 3.5 `docs/PLAN.md` §11 (`:3786`) — the "recorded in the brief too" aside
      becomes a statement that this is a gap to surface to whoever designs the
      feed. **Claim stated.**
- [x] 3.6 `docs/PLAN.md` §12 (`:4414`) — the publish prohibition, which PLAN.md
      had deliberately declined to state because the brief held it, is now
      stated in PLAN.md. **Claim stated**; this is the most consequential of the
      six.
- [x] 3.7 `docs/PLAN.md` (`:1368`) — the `agora` wordlist entry is **kept**, its
      reason re-grounded on the Rust test fixtures, which were verified with
      `grep -rn '"Agora"' dialectica/rust-lib/` rather than assumed.
- [x] 3.8 `docs/IDENTICON.md:15` — the sentence makes its own argument and
      cites `view-identity-onboarding`'s surviving requirement.

## 4. Repair the source and test comments

Each explains *why* the code is as it is, so each states its rule rather than
being deleted.

- [x] 4.1 `sanitise.rs` — the module doc's opening argument no longer attributes
      the assignment to the brief, and the block quote becomes a direct
      statement of the rule. See §5 for the `SPEC.md` finding this exposed.
- [x] 4.2 `wire.rs:512` — the slate-refresh flow is stated, not cited.
- [x] 4.3 `wire.rs:4973` — same, in the test comment.
- [x] 4.4 `wire.rs:11053` — points at `sanitise.rs`, which is where the
      obligation is actually discharged.
- [x] 4.5 `end_to_end.rs:1788` — "the UI brief's claim" becomes "the claim".
- [x] 4.6 `end_to_end.rs:2332-2347` — states the empty-versus-failed rule
      directly, and keeps the durable lesson (cite by heading, never by an
      ordinal) attached to the surviving `module-wire-contract` requirement.
- [x] 4.7 `FeedScreen.qml:9` — the storage-failure rule is stated.
- [x] 4.8 `FeedScreen.qml:397, :400` — the ordering sentence's justification no
      longer routes through the brief.
- [x] 4.9 `FeedScreen.qml:654` and its tail — the extent-claim rule and its
      explicit non-obligation are both stated.
- [x] 4.10 `Core.qml:45` — states why the two shapes must stay disjoint.
- [x] 4.11 `tst_feed_states.qml:8` — states the rule.
- [x] 4.12 `tst_feed_extent_claim.qml:1` and `:13` — states the rule and the
      non-obligation.
- [x] 4.13 `tst_feed_copy.qml:12` and `:285` — states the global-claim
      prohibition and the empty-versus-failed distinction.
- [x] 4.14 `tst_screen_frame_geometry.qml:186` — assertion **message** prose
      only; the assertion itself is untouched.

## 5. The `ScreenFrame` contract, and a defect found on the way

- [x] 5.1 Move the implementer-facing `ScreenFrame` contract into
      `ScreenFrame.qml`'s own header — the one piece of the brief addressed to
      code authors rather than designers. `design.md` §3 records why this moves
      rather than being deleted, and why it is not a new document.
- [x] 5.2 `ScreenFrame.qml:47` — points at the contract at the top of its own
      file.
- [x] 5.3 Record the pre-existing **`SPEC.md` citations point at a file that
      never existed** finding. Two fixed because they were inside the block
      being repaired and one quoted the deleted file; four left alone and
      reported. `design.md` §4.

## 6. Gates

- [x] 6.1 `cargo test -p dialectica -p dialectica-core` — 889 + 30 passed, 0
      failed.
- [x] 6.2 `run-qml-tests.sh` — 13 spec files, 274 passed, 0 failed.
- [x] 6.3 `check_qml_names.py` and `check_qml_members.sh` — both ok.
- [x] 6.4 `cargo fmt --check` diffs shown to be **pre-existing**, by stashing
      the change and re-running for byte-identical output. Not introduced here.
- [x] 6.5 Re-run the survey grep and confirm it returns nothing outside
      `openspec/changes/archive/`.

## 7. Two citations arriving with the rebase

The rebase onto `6eec84f` brought in two citations that did not exist when §4
ran: PR #78 introduced `names.rs` entire and added the `feed.rs` line. Verified
against `3901e99`, where `feed.rs` holds no UI-BRIEF reference and `names.rs`
does not exist. Neither was a miss in the survey — they post-date it.

- [x] 7.1 `feed.rs` — the `author` doc comment's closing line. Replaced with the
      obligation's own argument: a name is a short derivation of a key, so
      distinct keys collide by pigeonhole and an attacker who regenerates freely
      can grind for one resembling a target's. Cites `generated-names`'
      requirement **by heading** — *"A name is never unique, never an
      identifier, and never numbered"*, verified at
      `openspec/specs/generated-names/spec.md:586` — which is the practice
      `end_to_end.rs` (§4.6) records as the durable lesson.
- [x] 7.2 `names.rs` — the module doc's deferred-gap paragraph. Points at this
      module's **own** *What a name is NOT* section rather than any external
      document: the argument was already here, so the citation had no reason to
      leave the file.
- [x] 7.3 Both written to survive issue #80, which deletes the author address
      and makes the public key the sole author identifier. Neither replacement
      asserts that *the address* is the identity or must be on screen — they
      state only the name-side claim, that a name is not an identifier. Note
      that `generated-names/spec.md:604` **does** say "The address is the
      identity", so citing that requirement by heading rather than quoting its
      body was load-bearing, not stylistic. §3.8's `IDENTICON.md` repair does
      carry the address claim and is a sentence #80 will need to correct.
- [x] 7.4 Gates: `cargo test -p dialectica -p dialectica-core` — 925 + 30
      passed, 0 failed, doctests clean. `cargo fmt --check` — **no output**,
      where §6.4 had to argue a diff was pre-existing; that diff is gone, so the
      argument is no longer needed.
