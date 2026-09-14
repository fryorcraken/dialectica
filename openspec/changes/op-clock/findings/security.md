# Security review — `op-clock`

Dimension: **security only**. Correctness, readability and architecture are held
by other instances of this agent; nothing below is a judgement about them.

Reviewed at `880bb2b` on `piece/op-clock`, against `origin/main` at `9357417`,
in a throwaway worktree. Baseline suite before any mutation: **1002 tests pass**
(972 unit + 30 integration).

The four threat-model cases the brief named were worked directly, with probes
rather than by reading. Three of them are **closed**, and the measurements are
recorded in the prose at the end so the next reviewer does not redo them. The
findings below are the two places the review did not come out clean, plus one
recorded trade-off.

---

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/log/mod.rs:400`
      — `OpLog::clock` decodes every op in the Stoa on every publish, turning a
      peer-controlled row count into publish latency

      `clock()` is implemented once, as a trait default that calls `iter_stoa`,
      and `SqliteOpLog` does **not** override it (`grep -rn "fn clock"` over
      `src/log/` returns exactly one hit, the default at `mod.rs:400`). Every
      publish therefore reads every row of the Stoa through `ordered_read`, which
      runs `SignedOp::from_bytes` on each `op_bytes` blob (`sqlite.rs:578-605`,
      `decode_entry` at `sqlite.rs:639`) — full bodies and attachment lists
      included — only to discard everything but `clock.counter`.

      **Scenario:** an attacker floods a Stoa with ops whose bodies sit just
      under `MAX_FIELD_LEN`. Every honest peer that receives them then pays a
      full decode of all of them each time it publishes anything. The attacker
      chooses N; the victim pays O(N × body).

      **Measured** (release build, `authoring::post` into a pre-filled
      `SqliteOpLog`, 140 KiB bodies):

      | ops in Stoa | one publish |
      |---|---|
      | 100 | 45.5 ms |
      | 500 | 156.8 ms |
      | 1000 | 437.8 ms |

      Linear, and ~140 MB decoded per publish at N=1000. Extrapolating the
      measured slope, ~45,000 max-size ops puts a single `publish_post` past the
      SDK's **20-second default call timeout** (PHASE0-FINDINGS §183), which the
      caller then sees as `timeout` and, per §261-268, cannot distinguish from a
      dead module.

      **This cost is new in this piece.** On `origin/main`, `authoring::publish`
      was `sign` then `append` and read nothing
      (`git show origin/main:.../authoring.rs`, lines 187-196) — O(1). The
      `log.clock(&op.stoa)?` at `authoring.rs:230` is what introduces the scan.

      **`guarded` does not help here.** It converts a panic into an error JSON
      (`wire.rs:63`); it cannot shorten a call that is merely slow, so this
      degrades to the timeout path rather than to an error.

      **A cheap derivation already exists and is unused.** `score_epoch`
      (`sqlite.rs:697`) stores exactly `clock.counter` per row. Measured on the
      same 1000×140 KiB store: the shipped `OpLog::clock` took **99.8 ms**; a
      plain `SELECT score_epoch FROM ops WHERE stoa = ?1 AND score_epoch IS NOT
      NULL` fed to `clock_from_counters` took **35.4 ms** and returned the
      **identical value** — and that is without an index on
      `(stoa, score_epoch)`, which would make it an index-only walk. The trait's
      own doc comment at `mod.rs:395-399` anticipates exactly this
      (*"an implementor overriding it for speed is answering the same question a
      faster way"*); the override was simply not written for the production
      store.

      **Severity: medium.** Remote, attacker-amplified, no memory-safety
      consequence, degrades a peer's ability to publish rather than silencing it.
      It is a defect rather than a preference: the spec requires the clock be
      *derived* (`op-ordering` spec lines 7-9), and a `score_epoch` read satisfies
      that derivation requirement identically while not reading bodies.

---

- [ ] **`spec-writer`** — `openspec/changes/op-clock/specs/op-ordering/spec.md:13`
      — the counter is an intra-Stoa reception oracle, and the spec names only
      the cross-Stoa leak

      Line 13 scopes the clock per Stoa specifically to stop *cross*-Stoa
      inference (*"letting a reader in a quiet Stoa infer that the peer is busy
      elsewhere"*). The *within*-Stoa leak is not named anywhere in the change:
      a published counter states how many of that Stoa's ops its author had
      **received** at publish time, and an observer who controls that number can
      probe it.

      **Scenario:** an attacker publishes 5 ops into a Stoa and relays only the
      first 2 to one target. Both targets then publish identical content. The
      attacker reads the counters off the wire and learns which target received
      what.

      **Measured** (`MemoryOpLog`, `authoring::post`, same body on both sides):
      the peer holding all five published at **counter 6**; the peer holding two
      published at **counter 3**. The two ops also carry different op ids, so the
      same content is distinguishable by the author's reception state alone.

      In a censorship-resistant forum this is the property an adversary wants:
      it turns "did my suppression of ops 3-5 reach this peer?" into a value read
      directly off that peer's next post. It is **inherent to a Lamport counter**
      and not a coding defect — the fix is not to change the code but to say so,
      so that a later reader does not assume line 13 enumerates the leaks.

      **Severity: low, and it is a documentation gap rather than a code defect.**
      What is being asked for is a stated trade-off in the `op-ordering`
      requirement beside line 13, in the register the rest of that spec already
      uses.

---

## What was clean, and the measurements behind saying so

Recorded as prose rather than as boxes, because none of it needs action.

**Threat 1 — the far-future asserted time decides nothing.** Traced every reach
of the value. `asserted_ms` is read in exactly one non-test place outside its own
module: `thread.rs:787`, `.map(|c| format_asserted(c.asserted_ms, now_ms))`.
`format_asserted` (`asserted_time.rs:108`) returns `AssertedTime { text: String,
clamped: bool }` with no numeric field, and `wire.rs:1866-1874` emits exactly
`{"text", "authorAsserted", "clamped"}` — no millisecond field anywhere in the
JSON. The ordering position is carried separately as `position`
(`wire.rs:1826`), assigned from the sequence index at `thread.rs:677-681`.
`cmp_ops` (`arrival.rs:429-450`) reads only `counter` and `id`, and structurally
**cannot** read the time: `OpEntry` (`arrival.rs:313-317`) carries no clock and
no `Arrival`. The SQL order (`sqlite.rs:587`) sorts on
`sort_has_counter, sort_counter, op_id` and has no wall-clock column at all.
Appendix A's failure is not reproducible here.

`format_asserted` is also **total**. Swept in a **debug** build (overflow checks
on) across 100,000 points spanning the whole `u64` range, each against both a
normal and a hostile reader clock, plus dense neighbourhoods at both ends
(`u64::MAX-10000 ..= u64::MAX`, `0..10000`) and the era-arithmetic corners
(`doe` at 0, 146_096, 146_097). No panic. The `saturating_add` at
`asserted_time.rs:113-114` is load-bearing and is pinned by the suite's own
`a_maximal_reader_clock_does_not_overflow_the_allowance`.

**Threat 2 — the ceiling attack is closed.** `clock_from_counters`
(`arrival.rs:252-276`) sorts ascending and folds from zero, and the subtraction
is written `counter - clock` after `counter > clock` is established, so nothing
there can overflow. Measured: `clock_from_counters([1,2,3,u64::MAX])` returns
**3**, and `next_counter(3)` is 4 — the victim can still publish. The fold is
order-independent (forward, reversed and rotated inputs all returned the same
value over a 52-element set containing both `u64::MAX` and `u64::MAX - 1`), and
duplicates do not stack (1000 copies of `ADVANCE_BOUND` yield `ADVANCE_BOUND`).
The ladder is real but priced as the design claims: 10^6 ladder ops reach
10^12, which is **5.4 × 10⁻⁸ of `u64::MAX`**; reaching the ceiling needs
`u64::MAX / ADVANCE_BOUND` ≈ 1.8 × 10^13 published, delivered and stored ops.
`next_counter(u64::MAX)` saturates rather than wrapping.

**Threat 3 — no path refuses or rewrites an op for its clock.** `transport::receive`
(`transport.rs:454-499`) runs channel lookup → size check → decode → verify →
Stoa match → append, and touches no clock value; there is no `ADVANCE_BOUND`
reference anywhere in `transport.rs`. `Op::decode` (`op.rs:831-838`) reads the
sixteen clock bytes and validates nothing about them, which is what the spec
requires. Nothing rewrites a stored op: `append` is `INSERT OR IGNORE`
(`sqlite.rs:684`).

The version-2 decode path is also panic-free under fuzzing: every one of the
171 prefixes of a valid version-2 `SignedOp`, all 1360 single-byte mutations of
it, and a lying `153_600`-element list count written at every 4-byte offset,
each refused without panic and without any call exceeding 200 ms — run in a
debug build.

**Threat 4 — no new amplification path.** Op identity is still content-derived,
so an exact byte replay still dedups (`Appended::AlreadyPresent`, verified). The
free preimage bytes do let one author mint unlimited distinct ops at one counter
— verified that two ops differing **only** in `asserted_ms` have different ids
and both store — but an attacker could already do that by varying the body, and
there is no rate limit or dedup-based flood control anywhere for this to have
defeated. It compounds the first finding above (more rows to decode) rather than
opening a path of its own.

**Threat 5 — moderation.** `resolve` (`moderation.rs:430-530`) holds no bounded
-candidate-set assumption: it collects an unbounded `Vec`, uses
`iter().position()`, and reaches the chosen entry with `.nth()` rather than
indexing, so a broken index renders as `Unmoderated` instead of panicking. The
author's report is accurate — the code never held the two-op premise, only the
prose did. The degraded `Hide` preference is keyed on `first.op.op.clock.is_some()`
(`moderation.rs:459`), and since `cmp_ops` places every counter-carrying op ahead
of every op without one, that branch is reachable **only** when every binding
moderation predates the clock fields. An attacker cannot force it. The
`the_hide_bias_searches_only_ops_that_already_bind` test pins the filter that
stops a forged `Hide` from winning the tie-break.

**Threat 6 — arithmetic.** Every clock-path arithmetic site uses a checked or
saturating form: `saturating_add` at `asserted_time.rs:113`, `arrival.rs:293`;
the deliberately-ordered subtraction at `arrival.rs:268`; `saturating_mul`/`min`
paging at `thread.rs:690-691`; `u128 → u64` via `try_into().unwrap_or(u64::MAX)`
in the host clock at `rust-lib/src/lib.rs`. `counter_sort_key`
(`sqlite.rs:114`) is a bijection with a `wrapping_add` that is intentional and
tested at both ends. The `score_epoch` in-band-sentinel collision (`u64::MAX as
i64 == -1`) was found and closed before this branch, with
`an_op_with_no_counter_and_one_at_the_maximal_counter_store_different_score_epochs`
pinning it.

**Non-security-relevant:** the `keystore.rs` diff is entirely `cargo fmt`
reflow — no behavioural change to the AAD construction, the permission checks or
the Argon2 bounds. `cargo fmt --check` passes on the branch.

**`cargo mutants` was started and abandoned, so it contributes nothing here.**
`--file "**/asserted_time.rs"` enumerated 99 mutants and `--file` over both clock
modules enumerated 120; each mutant rebuilds the crate, and neither run got past
the unmutated baseline within the window the brief allows. It is left undone
rather than reported as clean — **no mutation result backs any statement above**,
and the measurements quoted are direct probes instead. Anyone picking it up
should expect a long run and should note what it structurally cannot see here:
`ADVANCE_BOUND`, `FUTURE_ALLOWANCE_MS` and `FLOOR_MS` are `const`s, which
`cargo mutants` does not mutate. Each already carries a hardcoded assertion for
that reason (`arrival.rs:207-210`, `asserted_time.rs:59-61`,
`the_constants_are_pinned_to_known_answers`), which is the right answer and is
in place.

## Tree state

Every mutation and probe was made in a throwaway worktree
(`.claude/worktrees/review-clock-security`), which is removed. Nine scratch
integration-test files were created there and deleted before this file was
written — note that the CI gate at `ci.yml:1173-1198` counts `#[test]`
attributes across the whole `dialectica/rust-lib` tree, so a scratch test left
behind would have failed the run. No file outside this findings file and its
`tasks.md` row is touched by this review.
