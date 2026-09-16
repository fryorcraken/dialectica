# Correctness review — `op-clock`

Reviewed at `880bb2b`, against `origin/main` at `9357417` (three-dot).
Dimension: **correctness only**. Security, readability and architecture are
held by other instances.

Baseline measured in a scratch worktree: **972 unit + 30 integration tests
pass**. Every measurement below was taken by mutating that tree and re-running;
the tree was restored to clean afterwards and the worktree removed.

---

## Findings

- [x] **`tester`** — `moderation.rs:1317` — `the_order_is_by_lamport_and_not_by_op_id` measures nothing
      **Scenario:** both ops come from `a_moderation` (`moderation.rs:666-679`),
      which builds `clock: None`. Position is expressed only through
      `Arrival::ordered(2, …)` / `Arrival::ordered(9, …)` — and after this change
      nothing reads an `Arrival`: `OpEntry` (`arrival.rs:313-317`) carries only
      `counter` and `id`. Both ops land in the `(None, None)` arm, and `resolve`
      then takes the Hide-preference branch, which ignores position entirely.
      **Measured, twice:** the test passes with the two arrival values
      **inverted** (`9`/`2` instead of `2`/`9`), and passes again with both
      arrivals replaced by `Arrival::unordered()` — i.e. with every scrap of
      ordering metadata deleted. 1 of 1 passes under each mutation.
      The name is the strongest claim in the file and it is false; the comment at
      `1355` ("The HIGH op id gets the HIGH Lamport value, so it decides under
      §5.7") is false on both halves. **Severity: high** — this is the
      security-relevant resolver, and the test asserts coverage the suite does
      not have.

      **Fixed.** Renamed to `the_op_counter_decides_and_not_the_op_id` — the old
      name was the false claim, and "Lamport" now means the counter in the op
      rather than the transport's value, so the name had to move with the body.
      The ops now come from `a_moderation_at` with real counters, the arrivals
      say nothing, and the pair is **searched** (the shape
      `an_unhide_reverses_a_hide_whatever_the_two_op_ids_are` already uses in this
      file) so the unhide carries both the higher counter and the higher op id —
      which puts the counter rule, ascending-op-id order and the degraded
      branch's `Hide` preference in three-way disagreement. The search is guarded
      at the point of use.
      **Mutation proving it fails:** `moderation.rs`'s `deciding_index` changed to
      `if false && first.op.op.clock.is_some()`, so a counter-carrying leader is
      demoted to the degraded branch. It **fails** on
      `deciding_op() == Some(unhide_id)`, left `Some(the hide)`, alongside six
      other moderation tests. Restored.

- [x] **`tester`** — `moderation.rs:1288` — `a_later_hide_reverses_an_earlier_unhide` does not establish "later"
      **Scenario:** same shape. Both ops are `a_moderation` (`clock: None`);
      "later" is expressed only as `Arrival::ordered(2, …)` vs
      `Arrival::ordered(3, …)`, which orders nothing. The assertion
      (`is_hidden()` + `deciding_op() == hide_id`) is exactly what the
      Hide-preference branch yields for *any* op set containing a binding Hide,
      whatever the order.
      **Measured:** passes with the two arrival values inverted — 1 of 1.
      Note its sibling `a_later_unhide_reverses_an_earlier_hide`
      (`moderation.rs:1082`) **was** re-aimed onto `a_moderation_at` with real
      counters and carries an explicit note about this exact trap. The fix was
      applied to the Unhide direction and not to the Hide direction, which is why
      this one survived. **Severity: high.**

      **Fixed**, by the same re-aim its sibling got: `a_moderation_at` with
      counters 2 and 3 in the ops, arrivals `unordered()`.
      **Mutations, and they answer differently — recorded because the difference
      is the finding underneath the finding:**
      - `cmp_ops`'s both-carry-a-counter arm inverted to ascending —
        `a_counter.cmp(&b_counter)` — makes the unhide lead, the counter branch
        decides it, and this test **FAILS** on `assert!(resolved.is_hidden())`.
        So the counter genuinely drives the answer now.
      - Deleting the counter branch (`if false && …`) leaves it **PASSING**,
        because the `Hide` preference the read then falls back to reaches the
        same op.
      That second explanation cannot be removed from any Hide/Unhide pair where
      the hide wins, so it is stated in the test's own comment rather than
      papered over, with a pointer to `a_later_unhide_reverses_an_earlier_hide`,
      which the preference fails outright. A finding-fix that claimed the
      preference had been excluded here would be the same defect one layer up.

- [x] **`tester`** — `log/contract.rs:1510` — `a_resolver_can_be_written_against_the_trait_alone` asserts a false ordering claim
      **Scenario:** the hide and unhide are built inline with `clock: None`
      (`contract.rs:1541`, `contract.rs:1551`) and appended with
      `Arrival::ordered(2, …)` / `Arrival::ordered(3, …)`. Both fall into the
      `(None, None)` arm, so the assertion at `1567-1570` actually rests on
      **ascending op id** — it passes only because the unhide happens to hash
      lower. The comment at `1566` says "Last write wins by Lamport order (§5.7):
      the unhide is current", which is not what is being measured.
      The test's stated *subject* — that a resolver can be written against the
      trait alone — is real and still holds, so this is narrower than the two
      above: the fix is to give the two ops counters (or to reword the assertion
      and comment as a degraded-order claim). **Severity: medium.**

      **Fixed**, and the fix found a second thing worth recording. The hide and
      unhide now carry counters in the ops and arrive `unordered()`. The obvious
      first attempt — counters 2 and 3 — **failed its own new fixture guard**: at
      those values the unhide's op id sorts BELOW the hide's, so the counter rule
      and ascending op id would have agreed and the assertion would still have
      distinguished neither. So the pair is now **searched**, and guarded at the
      point of use.
      **Mutations proving both variants fail** — and they need two different ones,
      which is itself the point of a contract suite run against both stores:
      - `cmp_ops`'s counter arm led by `a.id.cmp(b.id)` — the `_in_memory`
        variant **fails** (`Some(Hide)`, wanted `Some(Unhide)`); the `_in_sqlite`
        variant **passes**, because SQLite orders by stored columns and never
        calls `cmp_ops`.
      - `sqlite.rs`'s `ORDER BY` reordered to `op_id ASC, sort_has_counter ASC,
        sort_counter ASC` — the `_in_sqlite` variant **fails** the same way.
      Both restored. A single mutation would have "proved" one variant and left
      the other exactly as unmeasured as it was.

- [x] **`tester`** — `revision.rs:925` — `the_highest_counter_revision_is_current` has no fixture guard
      **Scenario:** `v2`/`v3`/`v4` carry counters 2/3/4 via `a_revision_at`, but
      unlike every neighbouring counter test in the file the fixture never
      controls or checks the op-id relation. Its immediate neighbour
      `the_op_counter_decides_currency_against_op_id_order` (`revision.rs:960`)
      asserts `higher_counter.op.id() > lower_counter.op.id()` at `977-980`;
      `a_revision_carrying_a_counter_beats_one_carrying_none_whatever_its_op_id`
      (`1057`) searches for the disagreeing candidate.
      **Measured, and it is currently sound:** inverting the counters (2/3/4 →
      4/3/2) flips the winner from `v4` to `v2` — the test fails, so the counter
      rule genuinely decides today. This is therefore a **latent** gap rather
      than a present vacuity: nothing pins the disagreement, so a future reword
      of a revision body — which changes the hash — can silently restore the
      coincidence with the suite green. `versions_naming_the_original_all_compete_directly`
      (`revision.rs:1506`) has the same gap with counters 2/3/4/5.
      **Severity: low** — a fixture-guard gap, by the file's own convention.

      **Fixed in both**, and the sibling was checked precisely because this
      finding named it — which is the "check the sibling you did not look at"
      rule doing its job. Each now asserts that the winner does not hold the
      LOWEST op id among its competitors, that being exactly the condition under
      which ascending-op-id order would name it too, with a failure message
      saying to re-roll a body rather than to delete the guard.
      **Mutation proving the assertions they protect can fail:** `cmp_ops`'s
      counter arm led by `a.id.cmp(b.id)`. `the_highest_counter_revision_is_
      current` **fails** with `"v2"` where `"v4"` was wanted, and
      `versions_naming_the_original_all_compete_directly` **fails** with `"v2"`
      where `"v5"` was wanted — while the new guards pass, which is what confirms
      the fixtures genuinely disagree today rather than the guards masking it.
      Four other revision tests fail under the same mutation. Restored.

---

## What was measured clean

**The wall clock does not reach any ordering.** Traced every comparison, sort
and resolution. `OpEntry` (`arrival.rs:313-317`) carries `counter` and `id` and
cannot name an `Arrival` or an `asserted_ms`; `cmp_ops` (`arrival.rs:429-450`)
reads only those two. The instant reaches the surface **only** through
`format_asserted` (`thread.rs:787`), which returns text plus a bool; the JSON
emitter (`wire.rs:1866-1875`) writes `text`/`authorAsserted`/`clamped` and no
number. `grep` for `asserted_ms` across `thread.rs`, `wire.rs`, `feed.rs`,
`revision.rs` and `moderation.rs` finds only fixture constructions and that one
call. The author's claim that sorting would require parsing a string **holds**.
`ThreadItem::position` is a decimal index string, not the counter.

**The counter-advance bound is measured against the held ops, as the spec
requires.** `clock_from_counters` (`arrival.rs:252-276`) sorts ascending and
folds from zero, so the result is a function of the op *set* and not of arrival
order. `OpLog::clock` (`log/mod.rs:400-406`) scopes it per-Stoa and derives it
from `iter_stoa` with no stored high-water mark. The subtlety the spec records
as a mid-draft fix is genuinely implemented. Near-overflow is handled:
`counter - clock` is written that way round rather than `clock + ADVANCE_BOUND`
(which would panic in debug), and `next_counter` saturates. `u64::MAX` refuses
the advance and the peer still publishes.

**The store-layout defect is real, was fixed, and is now caught by the suite.**
I re-introduced it — restored the four deleted sort column names
(`sort_ordered, sort_lamport, sort_msg, sort_op_id`) in `check_layout` — and
**8 of 63 sqlite tests fail**, with `a_store_this_build_wrote_passes_its_own_layout_check`
as the dedicated guard. The column list now matches `CREATE TABLE`. I found no
second instance of this shape.

**The leap-day literal is correct, and so is the whole calendar.** All three
date literals verify against `date -u` (1709164800 → 2024-02-29, 4107542400 →
2100-03-01, 951782400 → 2000-02-29). I then differential-fuzzed
`civil_from_days` against an independent incremental calendar walk over
**800,000 consecutive days** (1970 → ~4160, covering the 2000/2100/2200/2300/2400
leap-rule boundaries): **zero disagreements**. No further defects of that shape.

**Op identity and the downgrade attack.** The version discriminant is the first
preimage byte (`op.rs:718-740`), so a version-1 op's bytes and id are unchanged
by construction — pinned by `an_op_of_the_earlier_version_keeps_the_id_it_always_had`.
`a_downgrade_to_the_earlier_version_does_not_verify` (`op.rs:3030`) closes the
strip attack, and `an_op_of_this_version_carrying_no_counter_is_refused`
(`op.rs:3059`) confirms a version-2 op with the clock bytes removed is refused
rather than read as version 1. The decoder validates no clock *value*, which is
correct — refusing an op for its wall-clock would be a censorship vector. The
migration rule (no counter sorts below every counter, counter-less ops keep
ascending op id among themselves) is stated in `cmp_ops` and covered.

**The author's least-confident item is not vacuous.**
`the_recorded_arrival_does_not_reach_the_sort_key_at_all` (`sqlite.rs:1605`)
reads the **stored** `sort_has_counter`/`sort_counter` columns back out of
SQLite rather than re-calling `SortKey::of`, so it covers the write path, not
just the type signature. I mutated `append` to write
`entry.arrival.lamport()` into the sort column: **it is the only test in all 972
that fails.** It earns its place.

**`cargo mutants` was attempted and abandoned**, per the brief. 120 mutants
across `arrival.rs` and `asserted_time.rs`; the unmutated baseline alone is 14s
build + 49s test, so mutants began timing out at the 120s cap and the run would
have taken hours. The four TIMEOUT lines are infrastructure, not findings. Note
it could not have seen `ADVANCE_BOUND`, `FUTURE_ALLOWANCE_MS` or `FLOOR_MS`
anyway — it does not mutate a `const` — which is why those carry hardcoded
pinning assertions.

## One observation, not a finding

`log/mod.rs:370-376` still describes the pre-change behaviour ("leads with the
highest Lamport timestamp only when the transport supplied one; otherwise —
which is every op today"). That is now false. It is a stale doc comment rather
than a correctness defect, and belongs to the readability reviewer; recorded
here only because I read it while tracing the ordering path, and no box is
opened for it.
