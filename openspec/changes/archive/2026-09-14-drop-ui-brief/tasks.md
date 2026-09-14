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
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [x] CI green, PR merged — `closer`

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
- [x] 3.6 `docs/PLAN.md` §12 (`:4414`) — the publish prohibition is
      **re-grounded on `composer-view`'s "A successful publish claims local
      storage and never delivery"**, which contracted it in prose throughout.
      PLAN.md keeps only the ordering decision. (An earlier pass *restated* the
      rule here on the false premise that nothing else held it — finding 2.)
- [x] 3.7 `docs/PLAN.md` (`:1368`) — **no such entry exists and nothing was
      done.** This task was written against a fabricated decision record; no
      wordlist, `names/` or spec file is in this diff. See `design.md` §2.3.
- [x] 3.8 `docs/IDENTICON.md:15` — the reconstructed obligation-6 paragraph is
      **removed**; what remains cites `generated-names`' requirement heading and
      `SPEC.md`'s address-on-screen rule, both of which stand on their own.

## 4. Repair the source and test comments

Each explains *why* the code is as it is, so each states its rule rather than
being deleted.

- [x] 4.1 `sanitise.rs` — the module doc now attributes the assignment and the
      block quote to **`SPEC.md`**, the designer's handoff, rather than to the
      brief. An earlier pass dissolved both into prose on the mistaken belief
      that `SPEC.md` was a phantom; reverted. See `design.md` §4.
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
- [x] 5.3 **Withdrawn: there is no `SPEC.md` defect.** The "points at a file that
      never existed" finding was wrong — `SPEC.md` is the designer's handoff at
      `tmp/ui-bundle-new/handoff/SPEC.md`, gitignored, which is why a history
      search could not see it. All **seven** citations resolve. The two rewrites
      are reverted and nothing is deferred. `design.md` §4.
- [x] 5.4 Flag the one rule left with no contract: the **extent-locality** rule,
      marked `NO SPEC:` at `tst_feed_extent_claim.qml`. `design.md` §5.1.

## 6. Gates

- [x] 6.1 `cargo test -p dialectica -p dialectica-core` — 925 + 30 passed, 0
      failed.
- [x] 6.2 `run-qml-tests.sh` — 13 spec files, 278 passed, 0 failed.
- [x] 6.3 `check_qml_names.py` and `check_qml_members.sh` — both ok.
- [x] 6.4 `cargo fmt --check` diffs shown to be **pre-existing**, by stashing
      the change and re-running for byte-identical output. Not introduced here.
      Note the workspace manifest reports clean; the diffs need
      `dialectica-core/Cargo.toml` directly, which is the known fmt gap.
- [x] 6.5 Re-run the survey grep (`git grep`, not `grep -rn`, which descends
      into `rust-lib/target/`). One deliberate hit remains: the `NO SPEC:`
      marker at `tst_feed_extent_claim.qml:21`.

## 7. Two citations arriving with the rebase

The rebase onto `6eec84f` brought in two citations that did not exist when §4
ran: PR #78 introduced `names.rs` entire and added the `feed.rs` line. Verified
against `3901e99`, where `feed.rs` holds no UI-BRIEF reference and `names.rs`
does not exist. Neither was a miss in the survey — they post-date it.

- [x] 7.1 `feed.rs` — the `author` doc comment's closing line. The earlier pass
      **reconstructed** obligation 6's argument here (pigeonhole, grinding);
      that is removed, since its only source was the brief. What remains is the
      bare citation to `generated-names`' requirement **by heading** — *"A name
      is never unique, never an identifier, and never numbered"*, verified at
      `openspec/specs/generated-names/spec.md:586`.
- [x] 7.2 `names.rs` — the module doc's deferred-gap paragraph. Points at this
      module's **own** *What a name is NOT* section and nothing else; the
      reworded restatement of what that section says is removed, because the
      section says it.
- [x] 7.3 Both survive issue #80, which deletes the author address and makes the
      public key the sole author identifier. Neither asserts that *the address*
      is the identity. `generated-names/spec.md:604` **does** say so in its body,
      so citing by heading rather than quoting the body was load-bearing.
      `IDENTICON.md` still carries the address claim on its own account — it is
      that file's subject — and is a sentence #80 will need to revisit.
- [x] 7.4 Gates: `cargo test -p dialectica -p dialectica-core` — 925 + 30
      passed, 0 failed, doctests clean. `cargo fmt --check` — **no output**,
      where §6.4 had to argue a diff was pre-existing; that diff is gone, so the
      argument is no longer needed.
