# Architecture findings — `op-clock`

Reviewed **architecture only**. Correctness, security and readability are held by
other instances; an unticked row for those still means nobody has done them.

Measurements were taken in `.claude/worktrees/review-clock-architecture`, which
was mutated and then removed. Baseline for every "N of M" below is
`cargo test -p dialectica-core` on `880bb2b` = **1002 passing** (972 lib + 30
integration).

---

- [x] **`dev-writer`** — `thread.rs:235` — `Placed`'s claimed structural
      guarantee is not structural; bypassing it is invisible to the compiler and
      to every test
      **What is claimed:** the doc says "A type that **cannot express a
      position** removes the possibility [...] The only way to get a
      `ThreadItem` is `Placed::at`, which takes one — so 'every item has a real
      position' is the compiler's to enforce rather than a loop's to remember."
      **What is built:** `Placed { item: ThreadItem }` wraps an *already fully
      constructed* `ThreadItem` whose `position` was set to `String::new()` at
      `thread.rs:791`. The type expresses a position perfectly well — it holds
      one, and it holds the empty one. `Placed::at` only overwrites it.
      **Scenario:** replace `read_thread`'s
      `.map(|(index, placed)| placed.at(index))` (`thread.rs:680`) with
      `.map(|(_index, placed)| placed.item)`. Every `ThreadItem` then ships
      `position: ""`, the wire emits `"position":""` for every item of every
      thread, and a view ordering by that field sees every item tied.
      **Measured:** it compiles (one `dead_code` warning on `at`, no error) and
      **1002 of 1002 tests pass**. The only signal is a warning that an
      `#[allow]` or one other caller would silence.
      **Why the tests cannot see it:** `thread.rs` contains **zero** assertions
      on `ThreadItem::position` — `grep -n position thread.rs` returns only doc
      comments, the field declaration, the assignment, and an unrelated
      `Vec::position` at line 3061. The wire tests at `wire.rs:6486`, `6504`,
      `6527` and `6615` assert the *key set*, so `"position"` being present is
      pinned and its *value* is not.
      **Severity: defect.** This is the one place in the change where the
      project's "put the complexity in the data structure" is claimed but not
      delivered, and the claim is what stops the next reader from checking.
      The shape that would deliver it: make `ThreadItem::position` private (or
      make `ThreadItem` constructible only through `Placed::at`) so `resolve_item`
      is unable to name it, which is the same move `OpEntry` makes correctly for
      `Arrival` (`arrival.rs:314`).

      **Fixed** in `05a6faf`, taking the second of the two shapes suggested —
      `ThreadItem` is now constructible only through `Placed::at`, in the sense
      that `Placed::at` is the only code in the module that names the
      `ThreadItem { .. }` literal. `Placed` holds the fields rather than a built
      item, so `resolve_item` has no `position` to set and no inner item to
      reach for; the measured bypass (`placed.item`) is now a compile error
      rather than a `dead_code` warning.

      Making `position` private was the alternative and was not taken:
      `ThreadItem` is the wire-facing struct with every other field `pub`, so
      one private field would have needed an accessor and a constructor, which
      is more surface for the same guarantee. The cost of the shape chosen is
      that a new `ThreadItem` field must be added in two places — the compiler
      names both, which is the trade CLAUDE.md asks for.

      The finding's second half — that the tests could not see it — is fixed
      separately in the same commit. `every_item_carries_its_index_in_the_whole_thread_as_its_position`
      is the module's first assertion on a `position` **value**; it reads across
      three pages, so a per-page index (indistinguishable on page 0) fails too.
      Proven to fail by restoring `String::new()` inside `at`, which reports
      `["", "", "", "", ""]` — the exact defect measured here.

- [x] **`dev-writer`** — `authoring.rs:1768` —
      `every_publish_path_stamps_a_counter` is a hand-maintained sweep list over
      three named builders, and three more builders are already implied by the
      op kinds
      **Why this is the recorded trap:** the test enumerates `post`, `reply`,
      `vote` by name and its own comment says "a fourth that forgot the clock
      would publish an op no ordering could place [...] The compiler catches a
      MISSING field; it does not catch a field set to `None`". It then encodes
      exactly three. `OpKind` already has `Moderate`, `Revise` and
      `StoaMetadata` with **no publish builder yet** (`grep "publish(" authoring.rs`
      → three call sites, lines 298, 420, 476), so three more builders are
      coming, and each will be absent from this list with the suite green.
      **Scenario:** a later change adds `authoring::moderate` that builds its
      `Op` literal, writes `clock: None` (which the compiler demands and
      accepts), and calls `log.append` directly instead of `publish` — a
      plausible shortcut, since `moderate` needs no body cap. Every moderation
      op it publishes carries no counter, so `cmp_ops` sorts all of them below
      every counter-carrying op forever, and the `Hide`-wins legacy preference
      at `moderation.rs:459` silently governs them. Nothing fails.
      **The structural alternative already exists in this change:** `publish`
      (`authoring.rs:227`) is the single function that stamps, and
      `op.rs:1110`'s `one_of_each_kind` shows the right shape — it *derives* the
      version-2 half by mapping over the version-1 half rather than re-listing
      the kinds, so a new kind enters both halves for free. Assert the property
      that is total (`publish` is the only path from a locally-built `Op` to
      `append`), not the three names that happen to exist today.
      **Severity: defect** — this is the exact "hand-maintained sweep lists go
      stale silently" family, in a change that solved the same problem correctly
      two files away.
      **Measured, for contrast:** deleting the clock stamping inside `publish`
      fails **11 of 1002** tests, so the *current* three paths are well covered.
      The gap is only in what happens to a fourth.

      **Fixed** in `e8b1535`, in the `op.rs:1110` shape this finding names. The
      test is now `publish_stamps_a_counter_onto_an_op_of_any_kind` and derives
      its fixtures from `every_op_kind()` — the same list two other sweeps
      already count — rather than naming builders. `publish` overwrites `clock`
      with `..op` regardless of what the builder set, so the property really is
      total over kinds rather than over builders, which is what makes it
      assertable this way.

      Proven to fail with the finding's own scenario: making `publish` stamp
      only `Post` and `Vote` — a new kind whose path forgets the clock — fails
      this test and **only** this test, 1 of 44 in the module. The three-name
      version passed that mutation, which is the gap measured directly rather
      than argued.

      The ascending-counters half of the old test is kept as
      `the_counters_ascend_across_successive_publishes`, because it is a
      different claim: the first says a counter is stamped, the second says the
      value moves.

- [x] **`dev-writer`** — `feed.rs:6` and `feed.rs:67` — the feed's ordering
      changed behaviour and its module doc still describes the removed one,
      including the premise its ordering *name* was chosen on
      **What changed without being revisited:** `list_threads` reads
      `iter_stoa`, which returns `cmp_ops` order, and `cmp_ops` now leads with
      the op's own counter. So the feed **is** counter-ordered as of this change.
      The module header still reads: "*both are defined in terms of a Lamport
      timestamp that does not reach us — so both would today fall back to
      ascending op id, 'a hash [that] carries no recency whatever'*" (lines 6-8),
      and at line 67 "*before Lamport values arrive*".
      **Why this is architecture and not a typo:** that sentence is the entire
      justification for the module's central decision — shipping one ordering
      named `convergent` rather than §7.2's `new`/`active`, because neither could
      be honestly named. A Lamport value now does reach us, inside the signed
      preimage. The decision may well still be right (the counter is causal, not
      temporal, so `new` is still a claim the order cannot support) — but the
      *recorded reason* for it is now false, and this is the recorded
      "correcting prose can orphan a decision" trap running the other way: the
      cited reason moved and the decision was not re-examined.
      **Evidence it was never looked at:** `feed.rs` appears in the diff with 11
      changed lines, and `git diff origin/main...HEAD -- feed.rs` shows **all
      eleven are the compiler forcing `clock: None` into test fixtures**. No
      prose line was touched. `tasks.md:76-83` lists four files "carrying written
      arguments against exactly what this change builds" and `feed.rs` is not
      among them — while `transport.rs`, which *is* listed, turns out not to need
      one (its claim is scoped to "this layer", still true).
      **Severity: defect** — a live document describing behaviour the module no
      longer has, on the point the module exists to be careful about.

      **Fixed** in `8825ddc`, and this finding's reading is the one taken: the
      decision survives and the recorded reason did not, so the header now
      states the withdrawal explicitly rather than quietly editing the premise.
      The name is still `convergent`, re-argued from the narrower and still-true
      ground the finding itself identifies — a counter is **causal, not
      temporal**, so `new` remains a claim this order cannot support — plus an
      explicit note that the op's wall-clock is not the substitute either,
      because that is the reach a reader who learns "a time now exists" would
      make. The §8 reply-count bullet is separated the same way: the cost
      question is unchanged by a counter, and it now says so rather than
      resting on "before Lamport values arrive".

      This finding was one of three, each naming a *different* fifth site, so
      the list was rebuilt by grepping the claim's several phrasings rather than
      from any of them. That found sites in `wire.rs`, `revision.rs`,
      `thread.rs`, `authoring.rs`, plus more in `log/mod.rs` and `moderation.rs`
      than were reported — and one in `FeedScreen.qml` that is **rendered on
      screen**, with a test pinning it, fixed in `9ec147a`.

- [x] **`dev-writer`** — `log/mod.rs:371` — `iter_target`'s contract still
      describes the pre-change ordering rule, and it is the doc every resolver
      author reads to learn what "first entry" means
      **The stale text:** "*`cmp_ops` leads with the highest Lamport timestamp
      only when the transport supplied one; otherwise — which is every op today —
      it falls back to ascending op id, an order carrying no recency whatever.
      Take the first entry because that is the position the ordering rule defines
      as current, not because this promises recency it cannot currently
      deliver.*"
      **Why it is wrong now:** `cmp_ops` cannot consult the transport at all —
      `OpEntry` (`arrival.rs:314`) carries only `Option<u64>` and `&OpId`, which
      is this change's best structural move. "which is every op today" is false
      for every op this build publishes.
      **Scenario:** the doc names Stoa metadata (§5.7) as "the case known to be
      coming" and says it "will reuse that resolver's authority check unchanged".
      The author of that resolver reads this comment, concludes first-entry
      carries no recency, and writes a resolver that re-sorts or adds its own
      tiebreak — reintroducing a second implementation of the ordering rule,
      which `feed.rs:39-46` calls out as the thing none of these modules may do.
      **Severity: defect.** Same family as the `feed.rs` entry but a separate
      fix in a separate file, so a separate box.

      **Fixed** in `8825ddc`. The doc now says the rule leads with the op's own
      counter, that a counter is causal rather than temporal, and that the
      op-id fallback applies among ops carrying none — so "first is not most
      recent" survives on accurate grounds rather than on a false premise.

      The scenario this finding describes is answered directly rather than only
      implied: the doc now tells that resolver author **not to re-sort or add a
      tiebreak**, and says why — a second implementation of the ordering rule is
      the one thing every module reading this log may not do, and two orders
      that disagree produce no error anywhere. The old text invited exactly that
      compensation, so removing the false premise without replacing the
      instruction would have left the trap open.

- [x] **`dev-writer`** — `log/mod.rs:436` and `log/mod.rs:46-52` — `MemoryOpLog`
      documents a sort key and a tie case that no longer exist
      **Line 436:** "*the sort key is `(Arrival, OpId)`, and `Arrival` is what
      the transport supplies*". It is now `(Option<u64> counter, OpId)`, taken
      from the op — the `sorted()` body 38 lines below (line 474) already says so
      correctly, so the file contradicts itself.
      **Lines 46-52:** the header explains the dedup-before-sort ordering by
      "*two entries sharing an `OpId` but carrying different `Arrival` metadata
      compare `Equal`*". `arrival.rs:378` states the strictly stronger position
      this change bought — "*Both clock fields are inside the preimage, so one op
      cannot arrive with two different counters — that would be two ops with two
      ids. This is a stronger position than the prior design*" — and `log/mod.rs`
      still argues the weaker one.
      **Severity: defect, lower stakes than the two above.** The dedup-by-map
      defence is still correct and still worth keeping; only its stated reason
      is out of date. Worth fixing in the same pass because a reader comparing
      `arrival.rs` and `log/mod.rs` currently gets two different accounts of the
      same property and no way to tell which is current.

      **Fixed** in `8825ddc`, both halves, and the finding's framing is right:
      the dedup-by-map defence is kept and only the reason is replaced.

      Lines 46-52 now state the tie from what is still true — the op id is the
      last resort in every branch, so two entries sharing one compare `Equal` —
      and then name the withdrawal explicitly, pointing at the stronger position
      `arrival.rs` holds: both clock fields are in the preimage, so one op
      cannot arrive with two counters. That removes the two-accounts problem by
      making `log/mod.rs` cite `arrival.rs` rather than paraphrase it.

      Line 436's sort key is now `(Option<u64> counter, OpId)`, and the
      map-not-sorted-structure argument is re-grounded: the order is not an
      insert-time fact, because an op carrying no counter sorts relative to a
      population that grows. The old argument ("would need re-keying if the
      metadata changed") was about a transport value that no longer enters.

      A third site in the same file was found while fixing these and is included:
      the module header's "`arrival::Arrival` says what orders it", which is the
      same claim one paragraph up from where anyone would look.

- [x] **`spec-writer`** — `feed.rs:106` vs `thread.rs:191`,`202` — `FeedRow` and
      `ThreadItem` now answer the same question with two different contracts, and
      no document records that as a decision
      **The asymmetry:** `ThreadItem` gained `position` and `asserted_time`;
      `FeedRow` gained neither (`grep -n "asserted\|position" feed.rs` finds no
      field). A view rendering a post in a feed and the same post in a thread now
      branches on where the data came from — which is the one thing CLAUDE.md
      names about this API: "*JSON shapes are source-independent, so a view
      renders without branching on where the data came from.*"
      **Why this needs a spec answer rather than a code fix:** it may be
      correct. A feed row is a thread *head* and the head's time may be the wrong
      thing to show beside a thread whose latest activity is elsewhere; and the
      `thread-read` delta is legitimately scoped to thread reads. But the
      decision is nowhere — `design.md` does not mention the feed except for the
      `ADVANCE_BOUND` reasoning at line 110, `proposal.md` scopes out "a feed
      ordering" (line 115) which is a different question from a feed *field*, and
      `specs/thread-read/spec.md` names no feed.
      **Severity: gap, not a defect.** What is needed is one recorded sentence
      saying whether a feed row is meant to carry an asserted time, so the next
      change adds it on purpose or declines on purpose. Flagged to
      `spec-writer` rather than `dev-writer` because the answer belongs in a
      spec or in `design.md`'s "what this deliberately does not do", not in code.

      **Fixed** in `design.md`'s *"What this deliberately does not do"*, which is
      the second of the two homes this box offers. It goes there rather than into
      a spec because what was missing is a *decision* — which alternative was
      chosen and what ruled the others out — and that is `design.md`'s section by
      this repo's own split. The spec side already carried the *fact*
      (`specs/feed-view/spec.md`'s time-marking requirement, which states that
      the feed's rows carry no time and that its positive half binds the moment
      one arrives); that sentence now points at the design entry so a reader
      meeting the fact can reach the reason.

      **The decision, and it is two decisions rather than one** — which is the
      part the box's framing did not anticipate and is worth stating plainly:

      - **The asserted time is DEFERRED, not declined.** A feed row is meant to
        carry one. Three reasons it is not in this change: the change's scope is
        the thread read and widening the feed's separate reply shape is a second
        interface change with its own tests; **which** time a feed row should
        show is genuinely unanswered — a feed row is a thread *head*, and a
        thread carries at least two candidate times (the head's, and its latest
        reply's), so shipping the head's by default would have settled a product
        question by accident, which is the box's own hypothesis promoted to the
        stated reason; and the thing that must not happen — the field arriving
        unmarked — is already closed by `feed-view` rather than deferred with the
        rest.
      - **The position is DECLINED for now**, and for a different reason. There
        is no feed ordering for a feed position to be the position *of*:
        `proposal.md` scopes a feed ordering out. Handing out a position before
        the order it indexes exists would be a field whose meaning is whatever
        the implementation happened to do — which is precisely the defect this
        piece spent its `thread-read` delta closing for the thread case. It
        follows the feed ordering whenever that lands.

      The entry states the residual cost in terms rather than leaving it
      implicit: until both land, a view rendering a post in the feed and in a
      thread branches on the source for the time, and that is a known violation
      of the source-independence convention this box cites, carried deliberately
      so the next change closes it on purpose instead of rediscovering it.

---

## What was clean

The central architectural devices of this change hold up under attack, and
several are better than the brief's questions assumed.

**The wall clock's display-only property is genuinely structural, not
disciplined.** I grepped every reference to `asserted_ms` across the crate:
outside tests there are exactly four — the field declaration (`op.rs:559`), the
encoder (`op.rs:736`), the decoder (`op.rs:834`), and **one** read,
`thread.rs:787`, which feeds it straight into `format_asserted`. No number
reaches the wire: `wire.rs:1866-1875` emits `{text, authorAsserted, clamped}`
with `text` a string and no millisecond field beside it, and `wire.rs:6606-6641`
pins that key set so adding one fails a test. A view genuinely cannot obtain the
instant. The `AssertedTime` type carrying `clamped` rather than letting a view
recompute it (`asserted_time.rs:90-95`) is the right call for the same reason —
recomputation would need the raw value.

**`cmp_ops` losing the ability to name an `Arrival` is the strongest move in the
change.** `OpEntry { counter: Option<u64>, id: &OpId }` makes "ordering does not
consult the transport" a property of the type rather than of the body, and
`OpEntry::of(op, id)` taking the whole `Op` rather than a caller-extracted
counter closes the pair-the-wrong-counter-with-the-wrong-id hole that no test
would have caught. This is exactly the reshape CLAUDE.md asks for, and the
design doc's reasoning for it (§2) is honest about what the alternative would
have been.

**The seam between counter and clock is carried in the types.** `OpClock` as one
struct behind one `Option` makes the encoding version and the presence of both
fields the same fact, so the three impossible states two loose `Option`s would
admit are unrepresentable. The two fields' unequal authority is then enforced by
*different mechanisms* rather than by naming: the counter is reachable as a
number and ordered on; the wall clock is reachable only through a formatter. They
could not be confused by a call site even in principle.

**The migration is a sort key, not a branch.** `sort_has_counter` leads the
SQLite `ORDER BY` and every index (`sqlite.rs:516-523`), and
`counter_sort_key`'s write-time reversal keeps it one ascending scan. `cmp_ops`
has one three-arm match over counter presence and the doc's transitivity
argument (two totally-ordered blocks, uniform rule between them) is correct.
There is no per-comparison branch and no fourth guard. The `check_layout` column
list moving with the schema — the defect design.md §9 records catching — is the
kind of coupling that is invisible in a schema diff, and it is now covered.

**The sweep-list trap was solved correctly in `op.rs`.** `one_of_each_kind`
(`op.rs:1110`) derives its version-2 half by *mapping over* the version-1 half,
with `one_of_each_kind_carries_both_versions` (`op.rs:2840`) pinning the split.
A new op kind added to the base list enters both halves automatically. This is
the shape the `authoring.rs` finding above asks for, which is why that finding
is a defect rather than a preference — the change demonstrably knows how.

**The core/UI split is respected.** `DComposer.qml` gained one boolean and two
bindings; nothing in it decides anything core decides. The `publishing` flag is
an affordance decision (whether a control is rendered), which is the view's job,
and clearing it above `applyReply` rather than at that function's five exits is
the shape that survives a sixth being added. The comment is unusually honest
that the duplicate is currently unreachable because `callModule` is synchronous,
and says which test would therefore be measuring the event loop rather than the
guard — that is the right way to record a guard whose value is prospective.

**The clock injection is layered correctly.** `dialectica-core` still reads no
clock; `now_ms` enters at `rust-lib/src/lib.rs`, is deliberately *not* behind
`cfg(logos_scaffold)` (so `cargo clippy` compiles it, at the cost of an
`#[allow(dead_code)]`), saturates rather than wrapping, and returns zero rather
than panicking on a pre-epoch clock. `ReadOptions` grouping four presentation
arguments instead of adding a fifth positional `u64` is "make the change easy,
then make the easy change" applied where the code pushed back.

**`moderation-resolution` fits its existing shape.** The `Hide`-wins preference
is narrowed to one gate — `if first.op.op.clock.is_some()` at
`moderation.rs:459` — rather than growing a parallel rule, and the falsified
"exactly two ops can ever exist" premise was rewritten rather than left standing.
`transport.rs`, listed in `tasks.md` as needing a retraction, turns out not to:
its claim is scoped to "no Lamport timestamp reaches *this layer*", which is
still true, since the counter arrives inside the op rather than from the
transport.

**CI's layout-derived gates still measure what they claim.** The `#[test]`-count
gate walks `dialectica/rust-lib` by rglob rather than a source-root list, so the
new `asserted_time.rs` (15 tests) is counted without a change; no file was moved
or renamed, so no gate is left pointing at a directory that no longer holds
tests.

## Notes on method

`cargo mutants` was started twice (99 mutants scoped to `asserted_time.rs`,
`--jobs 4`). The unmutated baseline passed in 44s but the run produced no
per-mutant output within the window the brief allows, and it was stopped. Nothing
above depends on it — the two load-bearing measurements are direct mutations run
to completion against the full suite, which is a stronger result than a survival
count anyway.

**No dependency was added by this change**, so there is nothing to assess for
maintenance or licence. `asserted_time.rs` hand-rolls `civil_from_days` with a
stated reason (one format, no locale, no timezone database, no parsing) — the
right call for this crate, and `docs`/tests cover the leap-year rules it exists
to get right.

The review worktree was mutated twice and both mutations were restored and
verified with `git status`; the tree was then removed rather than relied on.
