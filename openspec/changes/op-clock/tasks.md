## Stages

- [x] spec — `spec-writer` — seven deltas, two of them withdrawing merged
      prohibitions (`op-ordering`'s own-clock ban, `op-format`'s
      no-ordering-field rule). Four were **not** in the original scope and are
      here because the change makes text they already carry false rather than
      merely incomplete: `moderation-resolution` (its degraded preference rested
      on "exactly two ops can ever exist", which free preimage bytes dissolve),
      `post-revision` and `thread-read` (both assert no Lamport value reaches
      us), and `composer-view` (which must now prevent a double-tapped submit,
      because core stops absorbing it). See `proposal.md` for the six answers and
      the "most recent first" verdict.
- [x] design + code — `dev-writer` — the counter and wall-clock enter the
      preimage at `VERSION_2`; `cmp_ops` stops being handed an `Arrival` at all,
      so "ordering does not consult the transport" holds by the comparator's
      type rather than by its body. The clock is a fold over the held ops,
      sorted ascending, which is what makes the advance bound a function of the
      op set rather than of arrival order. `LAYOUT_VERSION` 1 → 2. The composer
      disables its control while a publish is outstanding. `design.md` carries
      the six decisions and the constants' reasoning; the **NO SPEC** markers
      and one unimplementable-as-written finding are in the report.
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
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
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
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
- [ ] **`docs/PLAN.md` and `docs/UI-BRIEF.md`** — carried by the spec commit
      already on this branch; re-checked against the implemented behaviour.

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
