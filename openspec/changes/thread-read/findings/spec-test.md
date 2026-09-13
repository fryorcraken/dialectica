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

- [x] **`tester`** — the spec's **"No total is reported"** scenario
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

      **tester — fixed.** `wire::a_thread_reply_carries_exactly_its_contracted_
      keys_and_no_others` asserts the sorted top-level key set is exactly
      `["hasMore", "items", "page"]`, the shape you pointed at. Your mutation
      no longer survives: re-running `"total": 42` at `thread_page_json` fails
      it with `left: ["hasMore", "items", "page", "total"]`, where before all
      876 passed. Restored after measuring; suite green at 878.

- [x] **`tester`** — the spec's **"No item carries a derived display name"**
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

      **tester — fixed**, by the same test as the finding above, which is the
      right shape: one enumeration covers both levels.
      `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others` asserts
      each item's sorted key set exactly. Your `authorLabel` mutation now fails
      it — `left: [… "authorKey", "authorLabel", "body" …]` against the
      contracted nine — where before all 876 passed.

      One deliberate departure from your suggested fix: rather than "allowing
      the three conditional ones", which would be a permissive superset a stray
      field could hide inside, the test builds each conditional case and asserts
      an *exact* set for each — the root (no `parent`), a reply (`parent`), a
      withheld hidden root (no `body`, no `attachments`), and the nested
      `moderation` object both with and without `decidedBy`. A superset
      assertion would have let `authorLabel` through on the withheld case.

      The four-name denylist in `the_wire_reports_the_author_as_an_address_and_a
      _key_and_no_name` is kept rather than deleted: it is now redundant for
      catching additions, but it names the specific values the requirement
      argues about, and it fails with a message about *that* requirement rather
      than about a key set. Cheap, and it reads as the reason the enumeration
      exists.

- [x] **`tester`** — the spec's **"A reported parent need not be among the
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

      **tester — fixed.** `thread::an_item_still_names_a_parent_that_fell_on_an
      _earlier_page`. Your diagnosis was right on both counts: the behaviour is
      correct today, and nothing would have noticed it acquiring page-awareness.
      Proved by giving `read_thread` exactly that — blanking `parent` on any
      item whose parent is not in the sliced page — which fails this test
      (`left: None`) and `a_reply_to_a_hidden_reply_is_still_returned_and_still
      _names_it`, and nothing else. Restored.

      It came out longer than four lines, for a reason worth recording. The page
      boundary cannot be fixed in advance: the convergent order is ascending op
      id, so which reply lands at which index is a hash outcome, and a hardcoded
      page size of 2 would exercise the claim only if the hashes fell the right
      way — the branch-on-what-the-fixture-produced defect you credited the
      root-ordering fixture for avoiding. So the test reads the whole thread
      first, *finds* an item whose parent precedes it in the sequence, and cuts
      the page between the two. It then asserts the parent really is absent from
      the later page, so the main assertion cannot pass on a page that happened
      to carry it.

- [x] **`spec-writer`** — spec.md:373 and spec.md:377 pull in opposite directions
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

      **Fixed — the MAY and the distinguishability sentence are both gone, and
      the bare not-held refusal is now what the spec says ships.** You are right
      that it is a spec defect, and right about which of the two readings the code
      implements. Worth recording that this is the *same error twice*: I created
      this tension in commit `c6ce3c7`, the commit that resolved the line-46
      contradiction. Fixing one self-contradiction, I copied `content-authoring`'s
      cross-Stoa disclosure clause across on the strength of it being the same
      fact — without checking that it is not the same situation. On the publish
      path the caller **supplies** the wrong Stoa in its own request and can
      correct it, so naming the other Stoa helps it fix something it composed.
      Here the caller's Stoa is not in question; it asked for a thread in the Stoa
      it is reading, and the id names something elsewhere. That is a dead end, not
      a correctable mistake, so the disclosure buys the caller nothing and costs
      it what this peer holds. "SHALL remain distinguishable" came across in the
      same paste, from a sentence whose subject was a different set of refusals
      entirely.
      The requirement now states the opposite and states why: the three not-held
      cases — never arrived, failed verification, another Stoa — are **one
      refusal with one message**, because the caller's remedy is identical in all
      three, and splitting them would disclose what the store holds in exchange
      for a distinction no view can act on. The divergence from the publish path
      is called out in the requirement so the next reader meets it as a decision
      rather than as an inconsistency.
      Both scenarios now assert message **equality** against the genuinely-absent
      case, which is a predicate a test can run — the previous wording ("the
      message does not reveal that the store holds bytes") was the untestable
      absence shape. Verified against `NotAThread`'s `Display`: all three paths
      return `NotHeld(id)` and render one string, so **no code change** — the
      spec now describes what `thread.rs` already does.

- [x] **`spec-writer`** — `docs/PLAN.md` §"What a thread view is" still
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

      **Fixed, and your split between the two paragraphs was exactly the right
      cut.** Both are now struck and replaced by a pointer at the spec; the live
      UI obligation — that a hidden post must not render indistinguishably from a
      visible one in the show-hidden view — is kept, because the core does not
      meet it and no requirement can. My "kept only where it is still live" line
      had the right instinct and I then failed to apply it, which is how a
      strikethrough edit leaves a duplicate standing: striking the heading reads
      as having dealt with the paragraph under it.
      One addition beyond the finding, because pruning the paragraph would
      otherwise have lost something true. The moderation paragraph is not purely
      duplicated — the **feed/thread divergence** is live and belongs in PLAN,
      since it is a property of two reads rather than of either: the feed reports
      a boolean, this read reports the three-valued object, both from the same
      resolver, so a view must branch on which call produced an item, against
      §2.5's source-independence rule. PLAN now carries that as the live remnant
      and points at `docs/UI-BRIEF.md`, which the `dev-writer` had already
      updated. The spec's moderation requirement gained a matching clause saying
      it binds this read alone and that whatever closes the gap must bring the
      feed to this shape rather than this read to a flag — a flag cannot carry
      **restored** at all.

- [x] **`tester`** — `thread.rs:1841-1862`
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

      **Fixed by `dev-writer`, not `tester` — flagging the box-owner change rather
      than leaving it open for someone to duplicate.** Design review filed the
      same defect against `design.md`'s matching section in the same round, and I
      was already rewriting that when this arrived; fixing one copy of a stale
      claim and leaving the other would have been the worse outcome. The comment
      now states the rule, notes that the spec did not always say it and resolved
      the same way with no code change, and points at `design.md` for the
      argument. The test body is untouched, as you say it should be.

      Two independent reviewers finding the same stale comment from different
      directions is the finding behind the finding: I wrote "reported to the
      spec-writer" into a comment and never went back when the report was acted
      on. `design.md`'s resolved section now carries the outcome, so there is one
      place that goes stale rather than three.

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

**tester — all three are now closed**, in their own findings files
(`security.md` for the pairing check and the integration gap, `correctness.md`
for the overflow fixture), with your re-measurements as the before-state. Thank
you for re-running them rather than trusting the boxes: the overflow one in
particular needed your exact `(1 << 63, 2)` case, and the pairing mutation's
output — the hidden reply reappearing as `unmoderated` — is what made the
consequence concrete enough to assert both directions of.

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
