## Stages

- [x] spec — `spec-writer` — eight deltas, two of them withdrawing merged
      prohibitions (`op-ordering`'s own-clock ban, `op-format`'s
      no-ordering-field rule). Four were **not** in the original scope and are
      here because the change makes text they already carry false rather than
      merely incomplete: `moderation-resolution` (its degraded preference rested
      on "exactly two ops can ever exist", which free preimage bytes dissolve),
      `post-revision` and `thread-read` (both assert no Lamport value reaches
      us), and `composer-view` (which must now prevent a double-tapped submit,
      because core stops absorbing it). See `proposal.md` for the six answers and
      the "most recent first" verdict.

      **Second pass, after the code existed**, on the owner's ruling that the
      feed's ordering copy gets specified in this piece: a new `feed-view`
      capability now owns the ordering label, the author-assertion marking on a
      displayed time, and the out-of-chronological-order consequence. It closes
      the `NO SPEC:` marker at `FeedScreen.qml`'s ordering sentence. The label is
      **"newest first"**, and `feed-view` forbids "most recent first" by name;
      the design bundle's `copy.json` carries the forbidden phrase and is
      overruled, which `proposal.md` records.

      > **Third pass, settling the wording the `tester` routed back.** The
      > `tester`'s two findings were both re-verified against the files rather
      > than taken on report, and **both held**. The reason clause at
      > `FeedScreen.qml:433` discharges the author-assertion requirement
      > verbatim, and `tst_feed_copy.qml:197-204` pins it in both directions
      > including the `yet` prohibition — so only the label and the denial's
      > opening clause are in conflict, exactly as reported.
      >
      > **The label is "newest first"; the denial opens "Newest first means
      > latest in this forum's order, not latest by the clock."** and keeps its
      > remaining two sentences unchanged. `feed-view` gains a third direction
      > on the denial — it must deny the temporal *reading* and must not negate
      > the label — because the first two directions were jointly satisfiable by
      > a screen that contradicts itself, which is what shipped. `proposal.md`
      > carries the reasoning and records that neither label is in `copy.json`.
      >
      > **Routes on to a `dev-writer` then a `tester`**: `FeedScreen.qml` (the
      > label at 122, the denial at 433, and the three comment blocks at 108-120,
      > 402-408 and 412-431 that argue for the superseded wording — 412-417 is
      > the `NO SPEC:` marker this closes) and `tst_feed_copy.qml` (the
      > `"Not newest first"` anchors at 150, 181 and 210, and the `newest first`
      > prohibition at 213 which the label now legitimately trips).
      >
      > **Fourth pass, correcting my own false claim.** `proposal.md` said
      > "neither label is in `copy.json`". False: **three** handoff bundles
      > answer to that filename and they disagree. `tmp/ui-bundle-new/` has the
      > `["by relevance", "most recent first"]` the proposal quotes (that quote
      > stands) and no `orderingNote`; `tmp/ui-design/` **and**
      > `tmp/ui-bundle-old/` both carry `"same order for everyone"` plus an
      > `orderingNote` reading *"…Timestamps do not reach this machine yet…"*.
      > So the old label came from a bundle and the new one comes from none.
      > The proposal now names which bundle for every claim.
      >
      > **Dropping `FeedScreen.qml:410`'s `feed.orderingNote` citation was
      > right, but not for the reason given.** The instruction was that the key
      > does not exist; it exists in two bundles. It still goes, on grounds that
      > survive: its text is the "not yet" phrasing `feed-view` forbids by name
      > and `tst_feed_copy.qml:202` pins against, so citing it would aim the
      > reader at the claim this change falsifies — and all three bundles are
      > gitignored and absent from the tree, so the citation is unfollowable
      > regardless. The screen's comment naming no bundle is correct as written
      > and was left untouched.
      >
      > **Fifth pass, closing the four review boxes routed to this role.** No
      > code and no tests; two spec deltas and one design entry.
      >
      > - **`findings/security.md:118`** — `op-ordering`'s clock requirement now
      >   states the **intra-Stoa** reception oracle beside the cross-Stoa leak
      >   its scoping sentence already named, so that sentence reads as bounding
      >   the leak rather than enumerating it. A published counter states how many
      >   of a Stoa's ops its author had accepted as advances; the trade-off is
      >   stated with three bounds (a count and never *which* ops, nothing about
      >   unreadable Stoas, and only a peer that *publishes* answers), and the
      >   rejected alternative — suppressing or fuzzing the counter — is recorded
      >   with why it is not available.
      > - **`findings/spec-test.md:265`** — the unobservable clauses are gone.
      >   *"The clock survives a restart"* asserts only the surviving value; the
      >   replay scenario lost a clause that restated its own WHEN. The third site
      >   the finding named was **kept and rephrased** rather than removed — it was
      >   a weak phrasing of a real check, not a how-claim. The never-stored
      >   property stays a requirement and is now explicitly a constraint
      >   discharged structurally, with the reason no scenario can reach it.
      > - **`findings/spec-test.md:288`** — `thread-read` now contracts
      >   **`position` as an item's place in the whole thread and explicitly not
      >   a sort key** (stable across page sizes, does not restart per page,
      >   unique within a thread, no arithmetic, no adjacency, not comparable
      >   across threads), and the **asserted time as exactly three members a read
      >   hands out, with no fourth carrying the instant** — the shape `wire.rs`
      >   pins in a test, lifted into the contract. Scenarios added for each. The
      >   field *names* stay uncontracted on purpose, and that marker-stays
      >   decision is recorded in the box rather than left as silence.
      >
      >   **This is the second thing on this branch to be caught by re-reading a
      >   value instead of prose.** The draft contract said the position was
      >   *"comparable for sequence"*, which is the proposal's Q5 phrasing and
      >   sounds right. It is false: the position is `index.to_string()`,
      >   unpadded, so `"10" < "2"` and a caller comparing them as text gets a
      >   sequence that is not the returned one — invisible under the existing
      >   five-item fixture. Three documents carried the false promise and all
      >   three are corrected: the new requirement, `proposal.md`'s Q5 (which now
      >   records the withdrawal rather than quietly dropping it), and
      >   `thread-read`'s own pre-existing scenario *"Ordering the items requires
      >   only the position field"*, which asserted precisely the property the
      >   value lacks and is now *"Rendering the thread in order consults no
      >   time"*. `thread.rs:197`'s doc comment makes the same claim and is
      >   **routed to a `dev-writer`** — it is code, which this role does not
      >   touch.
      > - **`findings/architecture.md:242`** — `design.md`'s *"What this
      >   deliberately does not do"* now records the `FeedRow`/`ThreadItem`
      >   asymmetry as **two** decisions: the asserted time is **deferred** (scope,
      >   plus an unanswered product question about *which* of a thread's times a
      >   feed row should show), and a feed `position` is **declined for now**
      >   because no feed ordering exists for it to index. The residual
      >   source-independence cost is stated rather than implied, and
      >   `feed-view`'s time-marking requirement now points at the entry.
      >
      > **Re-read for self-contradiction across all seven deltas**, because
      > `openspec validate --strict` passes a spec that contradicts itself and
      > this piece has already produced two such contradictions. It produced a
      > third, and the sweep caught it before it landed — the position
      > comparability claim above, which contradicted both the value and a
      > scenario already in the same file. Also corrected from that sweep:
      > `design.md`'s reason for deferring a feed `position` said no feed
      > ordering exists for it to index, which `proposal.md`'s own spine bullet
      > (*"feed position"* resolves on the Lamport value) and `feed-view`'s
      > *"core returns the feed's rows already in the ordering rule's sequence"*
      > both falsify — the honest reason is scope, and the false one is recorded
      > beside it because it is the version that sounds right.
      >
      > *Original routing note follows.*
      >
      > `feed-view` makes **"newest first"** the label this interface uses. The
      > screen renders the ordering label **"same order for everyone"**
      > (`FeedScreen.qml:122`) and, beneath it, the denial **"Not newest first.
      > Posts carry a time their author claimed, which anyone could set, so the
      > feed is not ordered by it…"** (`FeedScreen.qml:433`). So the phrase the
      > spec makes the label is the phrase the screen negates.
      >
      > **What the spec and the screen already agree on**, so it is not relitigated:
      > the denial's *reason* clause discharges `feed-view`'s hardest requirement
      > verbatim — the displayed time is the author's own claim, not "no time is
      > available" — and `tst_feed_copy.qml:197-204` pins exactly that, including
      > an assertion that the word "yet" never returns. That half needs no change
      > in either document.
      >
      > **What conflicts is only the label and the first two words of the
      > denial**, and every way out is a copy decision:
      > - make the label "newest first" and reword the denial so it denies
      >   ordering *by the displayed time* without negating the label (the spec's
      >   own framing: the positional reading is permitted, the temporal one is
      >   not);
      > - or keep "same order for everyone" as the label, in which case
      >   `feed-view`'s *"'Newest first' … is the label this interface uses"* is a
      >   statement about a screen that does not say it.
      >
      > The `tester` does not choose between those: picking one writes the
      > interface's copy, and the brief routes wording back here. **`tst_feed_copy.qml`
      > is left exactly as it is** — it is green, and it is consistent with the
      > code it tests; what it is inconsistent with is the spec written after it.
      > Whoever settles the wording changes `FeedScreen.qml` and this test
      > together, and the `tester` re-proves the test against the new copy.
- [x] design + code — `dev-writer` — the counter and wall-clock enter the
      preimage at `VERSION_2`; `cmp_ops` stops being handed an `Arrival` at all,
      so "ordering does not consult the transport" holds by the comparator's
      type rather than by its body. The clock is a fold over the held ops,
      sorted ascending, which is what makes the advance bound a function of the
      op set rather than of arrival order. `LAYOUT_VERSION` 1 → 2. The composer
      disables its control while a publish is outstanding. `design.md` carries
      the decisions (`grep -n "^### " design.md` counts them) and the constants'
      reasoning; the **NO SPEC** markers and one unimplementable-as-written
      finding are in the report.
- [x] tests — `tester` — fifteen `tester` boxes across three review files, every
      change proved able to fail by a mutation that was then restored. `git diff`
      touches test code only, verified hunk by hunk against each file's
      `#[cfg(test)]` boundary rather than from memory.

      **The clamp is pinned end to end**, which is the box that mattered: both
      surviving review mutations (`"clamped": false` at the emitter, and the
      reader's clock disconnected in `thread.rs`) now fail two new `wire.rs`
      tests. Expected times are hardcoded from `date -u`, never read back out of
      the crate — and **three of my four first constants were wrong**, caught by
      that check before the test ran, which is the independence rule paying for
      itself in one measurement.

      **The cause was the fixtures, and they are what changed.** `thread.rs`
      gains `a_reply_at` (counter and asserted time as separate parameters, the
      single `A_TIME` standing for both being the whole defect); `wire.rs` gains
      `read_a_thread_asserting` (author's claim and reader's clock separate);
      `authoring.rs` gains `ANOTHER_ROOT`, so the two tests named for *receiving*
      an op have a second identity — a clock scoped to the peer's own authorship
      now fails exactly those two out of 44 and nothing else in the suite.

      Two vacuous `moderation.rs` tests re-aimed onto real counters, one renamed
      (`the_op_counter_decides_and_not_the_op_id`) with a searched
      three-way-disagreeing fixture; two `arrival.rs` bound tests merged into one
      four-row table that now reaches strictly inside the range; `sqlite.rs`'s
      inert loop driven through a real read so its five counters are live again;
      `contract.rs`'s resolver given counters plus a searched fixture; fixture
      guards added to two `revision.rs` tests and two `authoring.rs` ones; three
      misleading names or comments corrected.

      **One thing left open, reported rather than closed.** `wasNew: false` has
      no wire test (the `readability` box says why).

      **`tst_feed_copy.qml` is now re-aimed at the settled copy** (second pass,
      after the spec-writer chose the wording and the `dev-writer` applied it).
      The file went from 5 passed / 2 failed to 9 passed / 0 failed, and the two
      previously-failing tests were replaced rather than repointed: pinning the
      new copy needs different assertions, not the old ones with a new needle.

      **The ordering coverage is now three functions where it was one**, and the
      split is the finding rather than tidying. `oneStringContaining` uses
      `compare`, which ABORTS its caller — so the cross-screen recency sweep that
      sat below it had never executed against the current copy in any run, and
      neither had the `yet` prohibition. Both are now proved able to fail
      (mutations: `"…not ordered by it yet"`, and `"…ordered by date"` added to a
      second string). The sweep is its own function, so no anchor failure can
      hide it again.

      **A defect in my own first draft, caught by mutation and worth recording**:
      the sweep exempted the denial by its anchor BEFORE checking the
      negates-the-label prohibition, so a denial opening `"Not newest first"` —
      the exact defect `feed-view` gained a direction to stop — passed the sweep.
      The exemption is now scoped to the temporal list alone; with it misordered
      that mutation failed 1 test, with it correct it fails 2.

      Nine mutations run in all, each restored; `git diff --stat` shows
      `tst_feed_copy.qml` alone, `FeedScreen.qml` byte-identical.
- [x] review: correctness — `code-reviewer` — four findings, all for `tester`.
      The implementation is sound: the wall clock reaches no comparison (traced
      every sort and resolver; `OpEntry` cannot name an `Arrival` or an
      `asserted_ms`, and the instant leaves core only as text), the advance bound
      is folded over sorted held counters so it is a function of the op set and
      not of arrival order, and the downgrade attack is closed by the version
      byte living in the preimage. Both self-reported defects re-measured: the
      `check_layout` column list now matches `CREATE TABLE` and re-introducing
      the stale names fails 8 of 63 sqlite tests, and the calendar agrees with an
      independent walk over 800,000 consecutive days with zero disagreements.
      What is wrong is all in the tests. Two `moderation.rs` ordering tests are
      fully vacuous — `the_order_is_by_lamport_and_not_by_op_id` passes with its
      arrival values inverted AND with every arrival deleted, satisfied by the
      Hide bias rather than by any order; `a_later_hide_reverses_an_earlier_unhide`
      is the same shape, and is the direction the author's own re-aiming of its
      sibling missed. A `contract.rs` assertion claims Lamport order while resting
      on op-id hash luck. One `revision.rs` fixture lacks the disagreement guard
      its neighbours all carry — measured sound today, latent tomorrow.
      `cargo mutants` attempted and abandoned: the unmutated baseline alone is
      49s, so 120 mutants timed out at the cap.
- [x] review: security — `code-reviewer` — two findings in `findings/security.md`,
      one for `dev-writer` and one for `spec-writer`. The piece's three
      load-bearing defences hold under attack and the measurements are recorded
      so they are not redone: the asserted time reaches no comparison anywhere
      (`OpEntry` cannot name it, no numeric field leaves core, no SQL column
      carries it) and `format_asserted` is total over a debug sweep of the `u64`
      range; the ceiling attack is priced at ~1.8e13 ops and a `u64::MAX` op
      leaves the victim publishing at its own unchanged clock; no path refuses or
      rewrites an op for its clock, and the version-2 decoder survived every
      prefix, all 1360 single-byte mutations and a lying list count without
      panicking. What is wrong is a cost: the publish path now calls
      `OpLog::clock`, whose only implementation is a trait default that decodes
      every body in the Stoa to read one `u64` from each — 438 ms per publish
      over 1000 ops of 140 KiB, linear, against an O(1) publish on `main`, with
      the same answer available from the `score_epoch` column in a third of the
      time. The second is a spec gap: a published counter is an intra-Stoa
      reception oracle an observer can probe, and `op-ordering` names only the
      cross-Stoa leak. `cargo mutants` attempted and abandoned — 99 mutants for
      `asserted_time.rs` alone, each rebuilding the crate — so no mutation result
      backs any claim here.
- [x] review: readability — `code-reviewer` — twelve findings, seven for
      `dev-writer` and five for `tester`. The central distinction is stated at the
      field, the sort, the wire and the schema, and reaches the designer brief;
      `asserted_time.rs` and `arrival.rs` argue the shape rather than the rule.
      What is wrong is prose that outlived the change: `feed.rs` still argues no
      Lamport value reaches us (the retraction sweep listed four files and missed
      it), `moderation.rs` still cites recorded arrivals in its convergence
      argument, `Placed` claims a compiler guarantee privacy actually provides,
      `design.md` cites a spec sentence that does not exist, `tasks.md` says six
      decisions where there are nine, `check_layout` does not record the near-miss
      that would have bricked every store, and `now_ms` names both clocks. On the
      tests: two `arrival.rs` bound tests carry one identical assertion under two
      names, `sqlite.rs`'s sentinel-regression loop is inert under the new
      two-column shape while instructing the reader to preserve it, and a
      `moderation.rs` test calls its fixtures forged while asserting they verify.
- [x] review: architecture — `code-reviewer` — six findings in
      `findings/architecture.md`. Two structural claims that measurement
      contradicts (`Placed`'s position guarantee survives being bypassed with
      1002/1002 green; `every_publish_path_stamps_a_counter` is a three-name
      sweep list with three more builders implied), three stale contracts in
      modules the change reached without revisiting (`feed.rs`'s ordering name
      rests on the removed premise, `log/mod.rs`'s `iter_target` and sort-key
      docs), and one unrecorded `FeedRow`/`ThreadItem` shape asymmetry for the
      `spec-writer`. The central devices — `OpEntry` unable to name an
      `Arrival`, `OpClock` behind one `Option`, the wall clock reachable only as
      text, the migration as a sort key — all hold under attack.
- [x] review: spec-test — `spec-test-reviewer` — eight findings, six for
      `tester` and two for `spec-writer`; implementation not read. **Two
      surviving mutations**, both on `thread-read`'s clamping scenario:
      hardcoding `"clamped": false` at `wire.rs:1872`, and disconnecting the
      reader's clock at `thread.rs:787` so clamping can never fire, each pass
      all 972 + 30 tests — the second with `rustc` warning `now_ms` unused into
      a green run. Nothing asserts `clamped == true` anywhere; every thread and
      wire fixture carries one asserted-time value, so *"the sequence does not
      follow the asserted times"* cannot be tested non-vacuously either. What is
      strong: the ordering rule fails **16 tests** when the counter arm is
      replaced by ascending op id, migration fails **5** when the
      counter/no-counter arms invert (with a counter-zero fixture), and the
      advance bound fails **4** when `<=` becomes `<`. `arrival.rs`'s searched
      disagreement fixture is the right answer to the recorded defect family;
      two `authoring.rs` ordering assertions lack that guard and pass on digest
      luck. The composer guard is honestly scoped — deleting it fails exactly
      one test, not the four its comment claims.
      `the_recorded_arrival_does_not_reach_the_sort_key_at_all` measures more
      than the author credited: it drives five arrival shapes through `append`
      and reads the stored sort columns back. Spec is self-consistent,
      `validate --strict` passes, both `REMOVED` requirements are genuinely
      subsumed, and PLAN.md has shed what the spec now carries.
- [x] review: design — `design-reviewer` — six findings, all gaps rather than
      contradictions: the code takes every decision `design.md` records, and both
      withdrawn prohibitions are explicitly retired in the deltas that own them.
      One live falsified premise survives in `authoring.rs`'s `Published` doc and
      one in PLAN §7.2 rule 5; the tiebreak, accept-and-clamp, and the end of
      content-dedup are decisions taken but not recorded under Decisions.
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## Implementation

- [x] **`op-format`** — `Op` gains `clock: Option<OpClock>`; `VERSION_2` carries
      sixteen fixed bytes at a fixed offset with no presence tag; `VERSION_1`
      encodes exactly as before, so **a pre-existing op's id is unchanged** and
      the pinned `the_op_id_constant_is_pinned_to_a_known_answer` still passes
      against its independently-derived literal.
- [x] **`op-ordering` — the comparator.** `OpEntry` carries `Option<u64>` and an
      `OpId` and **cannot name an `Arrival`**; `cmp_tiebreak` and the message-id
      tiebreak are deleted. `ADVANCE_BOUND = 1_000_000`, pinned.
- [x] **`op-ordering` — the clock.** `clock_from_counters` folds the held ops'
      counters **in ascending order**, so the bound is measured against a value
      computed from the op set. `OpLog::clock(stoa)` is a default method over
      `iter_stoa`, derived on demand and never stored.
- [x] **`op-ordering` — the wall-clock.** `asserted_time::format_asserted`
      returns `{ text: String, clamped: bool }` and no number.
      `FUTURE_ALLOWANCE_MS` = 24h, `FLOOR_MS` = 2010-01-01, both pinned.
- [x] **Storage.** `LAYOUT_VERSION` 1 → 2; `sort_has_counter` / `sort_counter`
      replace four columns; `score_epoch` comes from the op's counter. The
      `check_layout` column list moved with them — missing that would have made
      every store this build wrote permanently unopenable.
- [x] **`content-authoring`.** `publish` stamps `next_counter(log.clock(stoa))`
      and the caller's `now_ms`. Authoring one body twice publishes two ops;
      re-publishing an op the peer holds publishes one.
- [x] **`thread-read`.** `ThreadItem` gains `position` (an opaque index string)
      and `asserted_time` (`Option<AssertedTime>`, absent for a pre-clock op).
      `Placed::at` makes "every item has a real position" structural.
- [x] **`moderation-resolution`.** The `Hide` preference keys on whether the
      leading candidate carries a counter. The falsified two-op premise is
      rewritten, and the unbounded candidate set is tested at 50 ops.
- [x] **`post-revision`.** The module doc asserted no Lamport value reaches us;
      corrected.
- [x] **`composer-view`.** `DComposer` disables its submit control from
      submission until an outcome, on every outcome including a refusal.
- [x] **`docs/PLAN.md`** — carried by the spec commit already on this branch,
      re-checked against the implemented behaviour, and §7.2 rule 5 re-aimed in
      the findings pass: its conclusion (the stored epoch is the op's counter)
      survives and is what decision 8 implements, so only the premise changed,
      with the wall clock named as explicitly **not** the new age input.
      ~~and `docs/UI-BRIEF.md`~~ — **struck.** That file was deleted by #83
      under an owner ruling that it was this codebase's own output being read
      back as input. There is no brief to correct, and the obligations it would
      have carried live in the `composer-view`, `thread-read` and
      `moderation-resolution` deltas instead. See `proposal.md`'s Impact.

## Notes for the `dev-writer`, which are not tasks

Things the spec deliberately does not settle, and things the survey of the
current code found that will bite. Neither is a checklist; both are here so they
are met before they are discovered.

**Four files currently carry written arguments against exactly what this change
builds.** Each needs a retraction or a citation of why an author-asserted clock
inside the signed preimage is a different thing from a receiver-asserted one
outside it — leaving them standing is how the next reader concludes the code is
wrong. They are `op.rs`'s module header ("No wall-clock timestamp [...] a forum
that ordered by it would be ordering by a field its adversary sets"),
`arrival.rs`'s header ("Why a second Lamport clock is the one thing not to
build"), `transport.rs`, and `log/sqlite.rs`'s `score_epoch` comment.

> **`dev-writer`, findings pass: the count was four and the answer is at least
> twelve.** Three reviewers each found a *different* fifth site and each
> believed it was the one the list missed, which is what prompted rebuilding
> the list by grepping the claim's several phrasings instead of working from
> any of them. Beyond the four above: `feed.rs` (the header and the §8 bullet),
> `log/mod.rs` (three — `iter_target`'s contract, the `(Arrival, OpId)` sort
> key, and the dedup rationale), `moderation.rs` (three — `resolve`'s
> convergence claim and two in the module header), `authoring.rs`'s `Published`
> doc, `wire.rs`'s feed method, `revision.rs` (two), `thread.rs`'s header, and
> **`FeedScreen.qml`**, where the claim is *rendered on screen* with a test
> pinning it in place. `transport.rs`, on the list, turned out **not** to need
> a retraction: its claim is scoped to "this layer" and is still true, since
> the counter arrives inside the op rather than from the transport.
>
> The durable lesson is about the shape of the note rather than its arithmetic:
> an enumeration of sites is the hand-maintained-sweep trap applied to prose. A
> grep for the *claim* is the thing to hand the next `dev-writer`, and the
> phrasings are several — "does not reach us", "the transport supplied one",
> "which is every op today", "carries no recency", "no timestamp and no nonce",
> "before Lamport values arrive".

**`moderation.rs`'s `Hide`-wins preference rests on a premise this change
falsifies.** Its doc states that a `Moderate` op has "no nonce, no timestamp and
no free byte", so "exactly two ops can ever exist" for one Stoa, moderator and
target. Adding the two fields makes that false, and the spec delta says so. The
preference itself stays correct — it is already keyed on whether the *leading*
candidate is ordered — but its justification and the tests built on the two-op
premise both need revisiting.

**`op.rs`'s `one_of_each_kind()` is a hand-maintained fixture list read by ten
tests**, and is the recorded stale-sweep shape. Every op it builds needs the new
fields; a kind omitted from it loses coverage in all ten silently.

**`log/mod.rs`'s `Entry::target` is deliberately exhaustive with no wildcard** and
is the one enumeration the compiler enforces. The sort keys the SQLite projection
materialises are derived from the `Arrival` at write time and must now come from
the op — that is a `LAYOUT_VERSION` bump, and the existing version check refuses
an unrecognised layout rather than migrating it.

**The constants the spec names but does not fix**: the counter advance bound, the
wall-clock future allowance, and the wall-clock floor. Each is required to be a
named constant, identical on every peer, and pinned against silent drift the way
`MAX_FIELD_LEN` and the channel prefix are. Choosing the values is the
`dev-writer`'s, and `design.md` is where the reasoning for each belongs.

**PLAN.md pruning this change owes**, since the spec now states the behaviour:
§13's Lamport entry and its `createdAt` entry are both answered and should be
struck through and pointed at the spec, with the reasoning moving to `design.md`
rather than being restated; §9.1 §8's feed-label bullet is resolved by the field
landing; §5.7's "no Lamport value reaches us today" parenthesis and §6's
suspended-reversibility note become false on merge.
