# thread-read — spec/test review

Read: `openspec/changes/thread-read/specs/thread-read/spec.md`, the test modules
of `dialectica-core/src/thread.rs` and `wire.rs`, and
`dialectica-core/tests/end_to_end.rs`. **The implementation was read only where a
mutation required it** — the lines mutated and their immediate context — which is
the one exception this role allows.

Baseline confirmed by command: **876 core and 26 integration tests pass** on
`piece/thread-read`. Four mutations were run; the tree was restored after each and
`git status --porcelain` is empty, with the full suite green again.

A gitignored `dialectica/logos-rust-sdk-src` symlink was absent in a fresh
worktree and had to be linked to the main checkout's `/nix/store` target before
cargo could resolve the manifest. Not a finding against the piece — noted because
the next reviewer will hit it.

## What is well covered

**The security property is genuinely pinned, at both layers.** Membership derived
from the parent chain rather than the claimed `thread` field is asserted in both
directions (`a_post_claiming_one_thread_is_placed_by_its_parent_in_another` checks
the claimed thread does *not* receive the post as well as that the real one does),
and duplicated at the wire in `the_wire_never_places_a_post_by_its_claimed_thread`
so a handler wired to the wrong function cannot leave it green. The fixtures
assert `verify()` on the intruder, so the attack is shown to need no forgery.

**The not-a-post rule over kinds is pinned as a generalisation, not an
enumeration.** I mutated the `_ =>` catch-all to special-case `OpKind::Revise` as
`NotHeld` — the one the spec says "invites the other answer". **Four tests
failed**: `every_non_post_kind_takes_the_same_refusal`,
`no_held_non_post_is_ever_reported_as_unheld`,
`reading_by_a_revisions_op_id_is_not_reading_the_thread`, and the wire's
`every_non_post_kind_takes_one_refusal_on_the_wire_and_never_the_unheld_one`. The
exhaustive match in `every_op_kind_is_either_a_post_or_in_the_non_post_table` has
no wildcard arm, so a fifth `OpKind` variant is a compile error rather than a
silently narrower sweep — which is the right answer to this repo's
hand-maintained-sweep-list defect.

**The refusal messages are asserted as a relation, not as pinned literals.**
`the_three_refusals_each_name_the_next_action_they_imply` requires each phrase of
exactly one refusal and forbids it of the other two, so moving "may not have
arrived yet" onto the not-a-post message fails even though the three strings stay
distinct. That is the fix for the "three distinct misinforming strings" defect,
and `no_held_non_post_is_ever_reported_as_unheld` builds its expectation from
`NotAThread::NotHeld(id)` on the *same* op id, so it cannot pass because two
different ids made two different strings. The weaker
`the_three_refusals_are_three_different_messages` is not load-bearing — it is
subsumed.

**The fixture searched rather than hoped.**
`a_thread_whose_root_does_not_sort_first` searches for an arrangement where a
reply's op id sorts below the root's and then *asserts* it found one, panicking on
exhaustion. That is the fix for the "branches on what the fixture happens to
produce" defect, and it is applied correctly.

**Positive halves are present throughout** — `a_long_legitimate_chain_still_
reaches_its_root` stops "always `None`" passing every termination test;
`a_revision_in_this_stoa_still_rewrites_the_post` stops "refuse every revision"
passing the cross-Stoa one; `an_unrevised_post_reports_the_two_ids_as_equal`
stops an unconditional `is_revised`. `a_post_becomes_placeable_when_its_missing_
parent_arrives` asserts both halves in one test.

The spec itself is self-consistent as it now stands. The revision refusal
contradiction the task mentioned was genuinely resolved in `c6ce3c7` — scenario
line 46 now names the not-a-post refusal, matching requirement line 393 — and I
found no second contradiction. `openspec validate --strict` reports only tasks.md
numbering warnings. PLAN.md's shedding is mostly done in the right shape:
strikethrough plus "see the `thread-read` spec", with the superseded `getThread`
sketch and the settled flat-vs-tree question both struck rather than deleted.

## Findings

- [ ] **`tester`** — the spec's **"No total is reported"** scenario
      (spec.md:543-547) has no test, and the absence is not merely unasserted —
      it is **measurably unenforced**.
      **Measured:** I added `"total": 42` to `thread_page_json`'s reply object
      (`wire.rs:1764`). **All 876 core tests passed.** No test enumerates the
      thread reply's top-level keys, so a count of what this peer holds can be
      added to the wire and nothing notices.
      **Why it matters beyond tidiness:** the requirement's own reason is that "a
      count of what this peer holds is not a count of what exists, and the two are
      indistinguishable once rendered" — the same
      indistinguishability argument the whole refusal triple rests on. The
      publish path already has this guard
      (`wire.rs:7025` sweeps `"total"`, `"count"`, `"tally"` … off a vote reply);
      the thread reply has no equivalent.
      **Severity: medium.** The fix is the shape already used at `wire.rs:7019`:
      assert the top-level key list is exactly `["items", "page", "hasMore"]`
      rather than spot-checking names.

- [ ] **`tester`** — the spec's **"No item carries a derived display name"**
      scenario (spec.md:190-194) says "**every field of an item is enumerated**"
      and "no field holds a value derived from either by any further
      transformation". The test checks four *guessed* names instead, so the
      scenario's enumeration is not performed.
      **Measured:** I added `"authorLabel": item.author_key.chars().take(8)` to
      the item object (`wire.rs:1731`) — a field whose value is derived from
      `authorKey` by exactly the transformation the requirement forbids, and whose
      name is none of the four guessed. **All 876 core tests passed.**
      **What the test does:** `the_wire_reports_the_author_as_an_address_and_a_key
      _and_no_name` (`wire.rs:5911`) asserts absence of `name`, `displayName`,
      `generatedName`, `mark`. A denylist of four cannot discharge a scenario
      phrased as an enumeration — it fails on the names somebody thought of and
      passes on the one they did not, which is the same failure mode as the
      hand-maintained sweep list this repo has already been bitten by.
      **Severity: medium.** `the_thread_reply_is_the_ecosystems_pagination_shape`
      already lists the nine expected item fields at `wire.rs:5865`; asserting
      that list is the item's *complete* key set (allowing the three conditional
      ones) closes both this and the finding above.

- [ ] **`tester`** — the spec's **"A reported parent need not be among the
      items"** scenario (spec.md:242-246) has no test at any layer.
      **Scenario:** the requirement is explicit that a caller must be able to
      render an item whose parent it does not hold — "a parent may be missing from
      the page because it falls on another page" — and that "the read is not
      refused for the parent being off the page". `grep` for a test reading a
      multi-page thread and asserting a later page's item still reports a parent
      that fell on an earlier page returns nothing;
      `the_root_occupies_a_slot_and_is_not_repeated_on_a_later_page` reads page 1
      but asserts only that the root is absent from it, never that page 1's items
      still carry their parent ids.
      **Why it is not covered by the flat-sequence tests:** those all read the
      whole thread unpaged through the `read` helper, where every parent *is*
      present — so they cannot distinguish "reports the parent always" from
      "reports the parent when it is on the page".
      **Severity: low** — the implementation sets `parent` from the op and has no
      page-awareness, so it is very likely correct; the point is that nothing
      would notice if it acquired some. Four lines on the existing three-reply
      fixture at page size 2.

- [ ] **`spec-writer`** — spec.md:373 and spec.md:377 pull in opposite directions
      about the **cross-Stoa refusal**, and the tests pin only one of the two
      readings.
      **The tension:** line 373 says an op belonging to a different Stoa "SHALL
      therefore take the not-held refusal", which makes it byte-identical to the
      refusal for an op that never arrived. Line 377 then calls the cross-Stoa
      refusal "the exception, and it is deliberate", says it MAY name the Stoa the
      op actually belongs to — "which is what lets a view offer to read the thread
      where it really lives rather than merely reporting a dead end" — and closes
      with "the refusal SHALL remain distinguishable from the other refusals".
      **Measured against the code:** `a_thread_read_naming_the_wrong_stoa_is_
      refused` (`thread.rs:1536`) asserts `Err(NotAThread::NotHeld(root.op.id()))`
      — the bare not-held variant, naming no Stoa and indistinguishable from a
      genuinely absent op. That satisfies 373 and forecloses the affordance 377
      describes.
      **Why it is a spec defect rather than a test gap:** a reader cannot tell
      from the spec whether the implemented behaviour is correct-and-complete or
      correct-but-missing-the-MAY. The final sentence's condition ("Should this
      read become reachable by a caller that cannot read the store") reads as
      governing a future state, but "SHALL remain distinguishable" implies the
      refusal is distinguishable *now*, which it is not.
      **Severity: low.** Either drop the MAY and the distinguishability sentence,
      or say plainly that the affordance is deferred and the bare not-held refusal
      is what ships — so the next reader is not left reconciling them.

- [ ] **`spec-writer`** — `docs/PLAN.md` §"What a thread view is" still
      **duplicates** behaviour the spec now states, rather than pointing at it,
      in the two paragraphs the strikethrough deliberately left standing.
      **Where:** PLAN.md:3427-3432 restates the three-state moderation enum and
      the two things a boolean loses, and PLAN.md:3434-3440 restates that a hidden
      post is absent from the default view and marked in the show-hidden view.
      Both are now requirements — spec.md:248-254 and spec.md:328-336 — stated
      there at greater length and with the deciding-op rule the PLAN text omits.
      **Why it is not covered by the edit that landed:** PLAN.md:3426 says "the
      argument below is kept only where it is still live", which is the right
      instinct, but the moderation-state paragraph is not a live open question —
      it is the spec's requirement written twice. The hidden-post paragraph's last
      sentence ("the view must not render a hidden post indistinguishably … in the
      show-hidden view") *is* still live, because it is a UI obligation the core
      does not meet; the first two sentences are not.
      **Severity: low**, and it is exactly the drift this repo's
      two-copies-of-one-rule rule exists to prevent: a later edit to the spec's
      hidden-reply requirement leaves the PLAN copy silently stale.
      **Fix:** strike the duplicated sentences and keep the UI obligation, the way
      the `getThread` sketch above it was handled.

- [ ] **`tester`** — `thread.rs:1841-1862`
      `reading_by_a_revisions_op_id_is_not_reading_the_thread` carries a **stale
      comment describing a spec conflict that no longer exists**, and describes
      spec text that was removed.
      **What it says:** "SPEC CONFLICT, resolved toward the requirement rather
      than the scenario, and flagged because the two genuinely disagree … Reported
      to the spec-writer." It then quotes the scenario as saying the refusal "is
      the one for a thread this peer does not hold".
      **What the spec says now:** commit `c6ce3c7` rewrote that scenario;
      spec.md:46 reads "the refusal is the one for an op the peer holds that is
      not a post, a revision being exactly that". There is no conflict, and the
      quoted sentence is not in the file.
      **Severity: low**, but it is this repo's persuasive-citation failure mode in
      miniature — a reader who trusts the comment goes looking for a contradiction
      that was fixed, or worse, re-opens it. The test body is correct and should
      stay; only the comment needs rewriting to say the scenario and the
      requirement now agree and this pins the agreement.

## Confirmed still open, not re-reported as new

Both are already unticked boxes owned by `tester` in the earlier findings files.
I re-measured them because an open box is a claim like any other, and both
**reproduce**:

- **`security.md`'s genesis/Stoa pairing gap.** Replacing `if genesis_address !=
  stoa` with `if false && genesis_address != stoa` (`wire.rs:1593`) leaves **876
  of 876 core and 26 of 26 integration tests green.** The check that stops one
  Stoa's moderator set being applied to another Stoa's posts is still entirely
  unpinned.
- **`correctness.md`'s pagination overflow fixture.** Replacing
  `page.saturating_mul(per_page)` with `page.wrapping_mul(per_page)`
  (`thread.rs:530`) leaves **876 of 876 and 26 of 26 green**, in release as well
  as debug. `an_enormous_page_index_does_not_overflow`'s `usize::MAX, 20` fixture
  is one where wrapping and saturating agree, exactly as reported.

The third known gap — **no integration test drives `read_thread` against a real
`SqliteOpLog`** — also reproduces: `grep -n "read_thread"` over
`tests/end_to_end.rs` returns nothing, and the branch's diffstat shows that file
untouched.

## Scenarios that cannot be tested

**None.** I checked every scenario for testability and found no scenario asserting
something no test could honestly discharge. Two are worth naming as tested by
proxy rather than untestable:

- "A cycle among parents does not prevent an answer" is discharged against the
  `CyclicLog` fake, because a real op id is the hash of bytes including `parent`,
  so a cycle is unmintable against SHA-256. The fake is justified in its own doc
  comment by a real threat — a corrupted or hand-edited SQLite file — and the
  security review agreed. A cycle test that hangs rather than fails is worth
  having.
- "Two peers holding the same ops return the same sequence" is discharged by two
  `MemoryOpLog`s filled in opposite orders rather than by two processes, which is
  the honest reading of "two peers" for a pure function of the op set.

## What I could not check

- **The `readThread` adapter in `rust-lib/src/lib.rs` is behind
  `cfg(logos_scaffold)` and is not compiled by `cargo test`.** Nothing in the
  suite I ran touches it; only CI's Build LGX does. This is the same gap both
  earlier reviewers recorded, and my mutations could not reach it.
- **`sqlite-projection` owns proving a real SQLite file yields a key-disagreeing
  row.** `thread.rs`'s two guards against such a row are tested against the
  `CyclicLog` fake, which is the right division; I did not try to produce one from
  a real file.
- I did not mutate `wire.rs` exhaustively — it is ~11k lines. The four mutations
  above were chosen for the consensus-critical and security-property lines the
  task named, plus the two wire-shape clauses I suspected by reading and could not
  convict without running.
